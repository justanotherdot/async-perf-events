use std::fmt;

use lazy_static::lazy_static;
use parking_lot::Mutex;
use perf_event::events::Hardware;
use perf_event::{Builder, Counter, Group};
use tokio::{fs::File, io::AsyncReadExt};
use tracing::{event, instrument, Level};
use tracing_subscriber;

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
    println!("{}", contents);
    emit_ipc();
}

fn emit_ipc() {
    let mut group = PERF.group.lock();
    let counts = group.read().expect("group read");
    event!(
        Level::INFO,
        "{{ instructions: {insns}, cycles: {cycles}, ipc: {ipc:.2} }}",
        insns = counts[&PERF.insns],
        cycles = counts[&PERF.cycles],
        ipc = (counts[&PERF.insns] as f64 / counts[&PERF.cycles] as f64)
    );
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
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
