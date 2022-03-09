use lazy_static::lazy_static;
use parking_lot::Mutex;
use perf_event::events::Hardware;
use perf_event::{Builder, Counter, Group};
use tokio::{fs::File, io::AsyncReadExt};
use tracing::instrument;
use tracing_subscriber;
use tracing_subscriber::{fmt, prelude::*, registry::Registry};

use tracing::span;
use tracing::Subscriber;
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

struct PerfLayer;

struct Perf {
    insns: Counter,
    cycles: Counter,
    group: Mutex<Group>,
}

lazy_static! {
    static ref PERF: Perf = {
        let mut group = Group::new().expect("group");
        let cycles = Builder::new()
            .group(&mut group)
            .kind(Hardware::CPU_CYCLES)
            .build()
            .expect("cycles counter");
        let insns = Builder::new()
            .group(&mut group)
            .kind(Hardware::INSTRUCTIONS)
            .build()
            .expect("instructions counter");
        group.enable().expect("group enable");
        Perf {
            insns: insns,
            cycles: cycles,
            group: Mutex::new(group),
        }
    };
}

#[instrument]
async fn cat(path: &str) {
    let mut file = File::open(path).await.expect("openat");
    let mut contents = [0; 10000];
    file.read_exact(&mut contents).await.expect("read");
    let contents = String::from_utf8(contents.into()).expect("string to utf8");
    //println!("{}", contents);
    contents.chars().into_iter().for_each(|_x| ());
}

impl PerfLayer {
    pub fn emit_ipc(&self) {
        let mut group = PERF.group.lock();
        let counts = group.read().expect("group read");
        println!(
            "{{ instructions: {insns}, cycles: {cycles}, ipc: {ipc:.2} }}",
            insns = counts[&PERF.insns],
            cycles = counts[&PERF.cycles],
            ipc = (counts[&PERF.insns] as f64 / counts[&PERF.cycles] as f64)
        );
    }
}

impl<S> Layer<S> for PerfLayer
where
    S: Subscriber + for<'span> LookupSpan<'span> + std::fmt::Debug,
{
    fn on_enter(&self, _id: &span::Id, _ctx: Context<'_, S>) {
        let mut group = PERF.group.lock();
        group.reset().expect("failed to reset perf event");
    }

    fn on_exit(&self, _id: &span::Id, _ctx: Context<'_, S>) {
        self.emit_ipc();
    }
}

//fn setup_global_subscriber() -> impl Drop {
fn setup_global_subscriber() {
    let fmt_layer = fmt::Layer::default();

    //let (flame_layer, _guard) = FlameLayer::with_file("./tracing.folded").unwrap();
    let perf_layer = PerfLayer;

    let subscriber = Registry::default().with(fmt_layer).with(perf_layer);

    tracing::subscriber::set_global_default(subscriber).expect("Could not set global default");
    //_guard
}

#[tokio::main]
async fn main() {
    let _guard = setup_global_subscriber();
    {
        let mut group = PERF.group.lock();
        group.enable().expect("group enable");
    }
    cat("/etc/dictionaries-common/words").await;
    {
        let mut group = PERF.group.lock();
        group.disable().expect("group disable");
    }
}
