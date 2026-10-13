//! Plays a loose `music/<name>.uni` through `music::play`: `play_music <data dir> <name> [seconds] [volume0..128]`.
use std::time::{Duration, Instant};

use uplink_engine::{archive, music};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: play_music <data dir containing music/NAME.uni> <name> [seconds] [volume0..128]");
        std::process::exit(2);
    }
    archive::init(std::path::Path::new(&args[1]));
    let seconds: f64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(f64::INFINITY);
    let volume: i32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(20);
    music::play(&format!("music/{}.uni", args[2]), volume);
    let start = Instant::now();
    while !music::finished() && start.elapsed().as_secs_f64() < seconds {
        std::thread::sleep(Duration::from_millis(50));
    }
    println!("finished={} after {:.1}s", music::finished(), start.elapsed().as_secs_f64());
    music::stop();
    println!("after stop: finished={}", music::finished());
    std::thread::sleep(Duration::from_millis(300));
}
