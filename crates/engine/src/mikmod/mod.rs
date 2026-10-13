//! Pure Rust port of the slice of MikMod (as bundled with SDL_mixer 1.2.5) that Uplink's
//! `.uni` songs need: the UniMod loader, the Protracker-style module player and the low quality
//! software mixer. [`Song::render`] produces 44.1 kHz stereo 16 bit frames.

mod loader;
mod mixer;
mod player;

use mixer::Mixer;
use player::Player;

pub const SAMPLE_RATE: u32 = 44100;

pub struct Song {
    player: Player,
    mixer: Mixer,
}

impl Song {
    /// Parse a `.uni` file. `volume` is SDL_mixer's 0..=128.
    pub fn new(bytes: &[u8], volume: i32) -> Result<Song, String> {
        let mut module = loader::load(bytes)?;
        let samples = module.samples.iter_mut().map(|s| std::mem::take(&mut s.data)).collect();
        let mixer = Mixer::new(module.numchn, samples);
        let mut player = Player::new(module)?;
        player.set_volume(volume);
        Ok(Song { player, mixer })
    }

    pub fn set_volume(&mut self, volume: i32) {
        self.player.set_volume(volume);
    }

    /// `Player_Active`: false once the song has run past its last position.
    pub fn active(&self) -> bool {
        self.player.active()
    }

    /// Fill `out` (interleaved stereo, an even number of values) like `VC1_WriteSamples`.
    pub fn render(&mut self, out: &mut [i16]) {
        let mut done = 0usize;
        let total = out.len() / 2;
        while done < total {
            if self.mixer.tickleft() <= 0 {
                self.player.handle_tick(&mut self.mixer);
                self.mixer.start_tick();
            }
            let left = (self.mixer.tickleft() as usize).min(total - done);
            self.mixer.consume_ticks(left as i64);
            let mut todo = left;
            while todo > 0 {
                let portion = todo.min(4096);
                self.mixer.mix(&mut out[done * 2..], portion);
                done += portion;
                todo -= portion;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fnv(pcm: &[i16]) -> u64 {
        pcm.iter().fold(0xcbf29ce484222325u64, |h, s| (h ^ (*s as u16 as u64)).wrapping_mul(0x100000001b3))
    }

    #[test]
    fn garbage_is_rejected() {
        assert!(Song::new(b"not a module", 128).is_err());
        assert!(Song::new(&[], 128).is_err());
    }

    /// Needs the original songs: set `UPLINK_MUSIC_DIR` to a folder with the six `.uni` files.
    /// The hashes are of the first 30 s at volume 128 and were checked to be sample-identical to
    /// SDL_mixer 1.2.5's bundled MikMod (the reference the original game used).
    #[test]
    fn matches_reference_render() {
        let Ok(dir) = std::env::var("UPLINK_MUSIC_DIR") else { return };
        let golden: [(&str, u64); 6] = [
            ("a94final", 0x6bd162aefbff75b8),
            ("bluevalley", 0xdd8d294450af24ae),
            ("myst2", 0x8e1482cca33bcf95),
            ("mystique", 0xa034df29f0df632b),
            ("serenity", 0x03b6298e0b5432bc),
            ("symphonic", 0x0c952dc423cc95d9),
        ];
        for (name, want) in golden {
            let bytes = std::fs::read(format!("{dir}/{name}.uni")).unwrap();
            let mut song = Song::new(&bytes, 128).unwrap();
            let mut pcm = vec![0i16; 30 * SAMPLE_RATE as usize * 2];
            for block in pcm.chunks_mut(8192 * 2) {
                song.render(block);
            }
            assert!(pcm.iter().any(|&s| s != 0), "{name} is silent");
            assert_eq!(fnv(&pcm), want, "{name}");
        }
    }

    /// Not a test: `UPLINK_MUSIC_DIR=dir UPLINK_DUMP_SONG=name UPLINK_DUMP_VOLUME=20 UPLINK_DUMP_OUT=file
    /// cargo test -p uplink-engine dump_song -- --ignored` writes the whole song as raw S16LE stereo
    /// (for comparing against an audio capture of the game).
    #[test]
    #[ignore]
    fn dump_song() {
        let (Ok(dir), Ok(name), Ok(out)) = (std::env::var("UPLINK_MUSIC_DIR"), std::env::var("UPLINK_DUMP_SONG"), std::env::var("UPLINK_DUMP_OUT")) else { return };
        let volume = std::env::var("UPLINK_DUMP_VOLUME").ok().and_then(|v| v.parse().ok()).unwrap_or(20);
        let bytes = std::fs::read(format!("{dir}/{name}.uni")).unwrap();
        let mut song = Song::new(&bytes, volume).unwrap();
        let mut all = Vec::new();
        let mut block = vec![0i16; 4096 * 2];
        while song.active() {
            song.render(&mut block);
            all.extend(block.iter().flat_map(|s| s.to_le_bytes()));
        }
        std::fs::write(out, all).unwrap();
    }
}
