use perf_event::events::Hardware;
use perf_event::{Builder, Group};
use tokio::{fs::File, io::AsyncReadExt};

async fn cat(path: &str) {
    let mut file = File::open(path).await.expect("openat");
    let mut contents = [0; 10000];
    file.read_exact(&mut contents).await.expect("read");
    let contents = String::from_utf8(contents.into()).expect("string to utf8");
    println!("{}", contents);
}

#[tokio::main]
async fn main() {
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
    cat("/etc/dictionaries-common/words").await;
    group.disable().expect("group disable");

    let counts = group.read().expect("group read");
    println!(
        "instructions / cycles: {} / {} ({:.2} ipc)",
        counts[&insns],
        counts[&cycles],
        (counts[&insns] as f64 / counts[&cycles] as f64)
    );
}
