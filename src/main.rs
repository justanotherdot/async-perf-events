use std::fmt;

use perf_event::events::Hardware;
use perf_event::{Builder, Counter, Group};
use tokio::{fs::File, io::AsyncReadExt};
use tracing::{event, instrument, Level};
use tracing_subscriber;

// TODO: don't force perf to be passed along.
#[instrument]
async fn cat<'a>(path: &'a str, perf: Perf<'a>) {
    let mut file = File::open(path).await.expect("openat");
    let mut contents = [0; 10000];
    file.read_exact(&mut contents).await.expect("read");
    let contents = String::from_utf8(contents.into()).expect("string to utf8");
    println!("{}", contents);
    emit_ipc(perf);
}

struct Perf<'a> {
    insns: &'a Counter,
    cycles: &'a Counter,
    group: &'a mut Group,
}

impl<'a> fmt::Debug for Perf<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        f.write_str("perf")
    }
}

fn emit_ipc(perf: Perf) {
    let Perf {
        insns,
        cycles,
        group,
    } = perf;
    let counts = group.read().expect("group read");
    // TODO: make structured event.
    event!(
        Level::INFO,
        "instructions / cycles: {insns} / {cycles} ({ipc:.2} ipc)",
        insns = counts[&insns],
        cycles = counts[&cycles],
        ipc = (counts[&insns] as f64 / counts[&cycles] as f64)
    );
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

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
    let perf = Perf {
        insns: &insns,
        cycles: &cycles,
        group: &mut group,
    };
    cat("/etc/dictionaries-common/words", perf).await;
    group.disable().expect("group disable");
}
