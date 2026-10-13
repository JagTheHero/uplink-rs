//! Offline renderer: `render_uni <in.uni> <out.wav> [seconds] [volume 0..128]`.
use std::io::Write;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: render_uni <in.uni> <out.wav> [seconds] [volume0..128]");
        std::process::exit(2);
    }
    let bytes = std::fs::read(&args[1]).expect("read input");
    let seconds: Option<f64> = args.get(3).and_then(|s| s.parse().ok());
    let volume: i32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(128);
    let mut song = uplink_engine::mikmod::Song::new(&bytes, volume).expect("load song");
    let limit = seconds.map(|s| (s * uplink_engine::mikmod::SAMPLE_RATE as f64) as usize);
    let mut pcm: Vec<i16> = Vec::new();
    let mut block = vec![0i16; 8192 * 2];
    while song.active() && limit.is_none_or(|l| pcm.len() / 2 < l) {
        song.render(&mut block);
        pcm.extend_from_slice(&block);
    }
    if let Some(l) = limit {
        pcm.truncate(l * 2);
    }
    let mut f = std::io::BufWriter::new(std::fs::File::create(&args[2]).expect("create output"));
    let data_len = (pcm.len() * 2) as u32;
    let rate = uplink_engine::mikmod::SAMPLE_RATE;
    f.write_all(b"RIFF").unwrap();
    f.write_all(&(36 + data_len).to_le_bytes()).unwrap();
    f.write_all(b"WAVEfmt ").unwrap();
    f.write_all(&16u32.to_le_bytes()).unwrap();
    f.write_all(&1u16.to_le_bytes()).unwrap();
    f.write_all(&2u16.to_le_bytes()).unwrap();
    f.write_all(&rate.to_le_bytes()).unwrap();
    f.write_all(&(rate * 4).to_le_bytes()).unwrap();
    f.write_all(&4u16.to_le_bytes()).unwrap();
    f.write_all(&16u16.to_le_bytes()).unwrap();
    f.write_all(b"data").unwrap();
    f.write_all(&data_len.to_le_bytes()).unwrap();
    for s in &pcm {
        f.write_all(&s.to_le_bytes()).unwrap();
    }
}
