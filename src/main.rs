use parking_lot::Mutex;
use perf_event::events::Hardware;
use perf_event::{Builder, Counter, Group};
use std::collections::HashMap;
use std::default::Default;
use tokio::{fs::File, io::AsyncReadExt};
use tracing::instrument;
use tracing::span;
use tracing::Subscriber;
use tracing_subscriber;
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;
use tracing_subscriber::{fmt, prelude::*, registry::Registry};

// Ref. https://docs.rs/tracing-flame/latest/src/tracing_flame/lib.rs.html#1-514

struct PerfLayer {
    group: Mutex<Group>,
    counters: HashMap<String, Counter>,
}

#[instrument]
async fn cat(path: &str) {
    let mut file = File::open(path).await.expect("openat");
    let mut contents = Vec::with_capacity(4096);
    file.read_to_end(&mut contents).await.expect("read");
    let contents = String::from_utf8(contents.into()).expect("string to utf8");
    contents.chars().into_iter().for_each(|_x| ());
}

impl<S> Layer<S> for PerfLayer
where
    S: Subscriber + for<'span> LookupSpan<'span> + std::fmt::Debug,
{
    fn on_new_span(&self, _attrs: &span::Attributes<'_>, _id: &span::Id, _ctx: Context<'_, S>) {
        // TODO: collect identifier for constructed span.
        let mut group = self.group.lock();
        group.reset().expect("failed to reset perf event");
    }

    fn on_close(&self, _id: span::Id, _ctx: Context<'_, S>) {
        // TODO: dump out identifier for current span.
        self.emit_ipc();
    }
}

// TODO: needs to take
//   * the file information to point to
//   * any other configuration information
//   * the shared buffer for the contents to write?.
struct PerfLayerGuard;

impl Drop for PerfLayerGuard {
    fn drop(&mut self) {
        // TODO: flush to disk.
        println!("PerfLayerGuard drop");
    }
}

impl PerfLayer {
    pub fn emit_ipc(&self) {
        let mut group = self.group.lock();
        let counts = group.read().expect("group read");
        let insns = self.counters.get("insns").expect("insns get");
        let cycles = self.counters.get("cycles").expect("cycles get");
        println!(
            "{{ instructions: {insns}, cycles: {cycles}, ipc: {ipc:.2} }}",
            insns = counts[&insns],
            cycles = counts[&cycles],
            ipc = (counts[&insns] as f64 / counts[&cycles] as f64),
        );
    }

    /// Configure the layer to write to a file when the layer is dropped.
    pub fn with_file(_file: &str) -> (PerfLayer, PerfLayerGuard) {
        (
            PerfLayer {
                group: Mutex::new(Group::new().expect("group")),
                counters: Default::default(),
            },
            PerfLayerGuard,
        )
    }

    /// Incrementally add a perf counter.
    pub fn with_perf_event(mut self, name: &str, counter: perf_event::Builder) -> PerfLayer {
        {
            let mut group = self.group.lock();
            let counter = counter
                .group(&mut *group)
                .build()
                .expect("perf event counter");
            self.counters.insert(name.to_string(), counter);
        }
        self
    }
}

pub fn setup_global_subscriber() -> impl Drop {
    let fmt_layer = fmt::Layer::default();
    let (perf_layer, guard) = PerfLayer::with_file("./perf.folded");
    let perf_layer = perf_layer
        .with_perf_event("cycles", Builder::new().kind(Hardware::CPU_CYCLES))
        .with_perf_event("insns", Builder::new().kind(Hardware::INSTRUCTIONS));
    {
        let mut group = perf_layer.group.lock();
        group.enable().expect("group enable");
    }
    let subscriber = Registry::default().with(fmt_layer).with(perf_layer);
    tracing::subscriber::set_global_default(subscriber).expect("Could not set global default");
    guard
}

#[tokio::main]
async fn main() {
    let _guard = setup_global_subscriber();
    //{
    //    let mut group = layer.group.lock();
    //    group.enable().expect("group enable");
    //}
    cat("/etc/dictionaries-common/words").await;
    cat("/proc/interrupts").await;
    cat("/sys/fs/cgroup/memory.pressure").await;
    //{
    //    let mut group = layer.group.lock();
    //    group.disable().expect("group disable");
    //}
}
