//! Software mixer: a port of MikMod's low quality mixer (`virtch.c`, `VC1_*`) as SDL_mixer
//! 1.2.5 configures it for the game: 16 bit stereo, 44100 Hz, no interpolation, no reverb,
//! no surround. All arithmetic is integer, so the output matches the original sample for
//! sample.

use super::loader::{SF_BIDI, SF_LOOP, SF_REVERSE};

pub const MIXFREQ: i64 = 44100;
const FRACBITS: u32 = 11;
/// Output gain shift (`BITSHIFT`): the mix is `>> 9` into 16 bits.
const BITSHIFT: u32 = 9;
/// Frames mixed per `AddChannel` pass (`TICKLSIZE / 2` in stereo).
const SAMPLES_THAT_FIT: i64 = 4096;
const PAN_LEFT: i32 = 0;
const PAN_RIGHT: i32 = 255;

/// `md_volume`, `md_musicvolume` as set by SDL_mixer's `music.c`.
const MD_VOLUME: u32 = 96;
const MD_MUSICVOLUME: u32 = 128;

#[derive(Clone, Default)]
pub struct Vinf {
    kick: bool,
    active: bool,
    flags: u16,
    handle: usize,
    start: u32,
    size: u32,
    reppos: u32,
    repend: u32,
    frq: u32,
    vol: i32,
    pan: i32,
    lvolsel: i32,
    rvolsel: i32,
    current: i64,
    increment: i64,
}

pub struct Mixer {
    voices: Vec<Vinf>,
    /// Sample data per sample index (empty for samples without data).
    samples: Vec<Vec<i16>>,
    tickleft: i64,
    /// `md_bpm`: tempo used to size the next tick.
    pub bpm: u16,
    tickbuf: Vec<i32>,
}

impl Mixer {
    pub fn new(numvoices: usize, samples: Vec<Vec<i16>>) -> Mixer {
        let voices = (0..numvoices)
            .map(|t| Vinf { frq: 10000, pan: if t & 1 != 0 { PAN_LEFT } else { PAN_RIGHT }, ..Default::default() })
            .collect();
        Mixer { voices, samples, tickleft: 0, bpm: 125, tickbuf: vec![0; SAMPLES_THAT_FIT as usize * 2] }
    }

    // ---- Voice_* (mdriver.c) + VC1_Voice* (virtch_common.c) ----

    /// `Voice_Play_internal`.
    pub fn voice_play(&mut self, voice: usize, handle: usize, flags: u16, length: u32, loopstart: u32, loopend: u32, start: u32) {
        let mut repend = loopend;
        if flags & SF_LOOP != 0 && repend > length {
            repend = length;
        }
        if let Some(v) = self.voices.get_mut(voice) {
            v.flags = flags;
            v.handle = handle;
            v.start = start;
            v.size = length;
            v.reppos = loopstart;
            v.repend = repend;
            v.kick = true;
        }
    }

    pub fn voice_stop(&mut self, voice: usize) {
        if let Some(v) = self.voices.get_mut(voice) {
            v.active = false;
        }
    }

    /// `Voice_SetVolume_internal` (the UWORD parameter truncation included).
    pub fn voice_set_volume(&mut self, voice: usize, vol: u32) {
        if let Some(v) = self.voices.get_mut(voice) {
            let tmp = (vol as u16 as u32) * MD_VOLUME * MD_MUSICVOLUME;
            v.vol = (tmp / 16384) as u16 as i32;
        }
    }

    /// `Voice_SetPanning_internal` with `md_pansep == 128` (the identity for 0..255).
    pub fn voice_set_panning(&mut self, voice: usize, pan: i32) {
        if let Some(v) = self.voices.get_mut(voice) {
            v.pan = pan;
        }
    }

    pub fn voice_set_frequency(&mut self, voice: usize, frq: u32) {
        if let Some(v) = self.voices.get_mut(voice) {
            v.frq = frq;
        }
    }

    // ---- mixing ----

    /// Number of frames left until the next player tick is due (0 means "run the player now").
    pub fn tickleft(&self) -> i64 {
        self.tickleft
    }

    pub fn consume_ticks(&mut self, n: i64) {
        self.tickleft -= n;
    }

    /// Called after the player ran: `tickleft = (md_mixfreq*125)/(md_bpm*50)`.
    pub fn start_tick(&mut self) {
        self.tickleft = (MIXFREQ * 125) / (self.bpm as i64 * 50);
    }

    /// Mix `portion` frames (at most [`SAMPLES_THAT_FIT`]) of all voices into `out` (interleaved stereo).
    pub fn mix(&mut self, out: &mut [i16], portion: usize) {
        let n = portion * 2;
        for v in self.tickbuf[..n].iter_mut() {
            *v = 0;
        }
        for t in 0..self.voices.len() {
            let vnf = &mut self.voices[t];
            if vnf.kick {
                vnf.current = (vnf.start as i64) << FRACBITS;
                vnf.kick = false;
                vnf.active = true;
            }
            if vnf.frq == 0 {
                vnf.active = false;
            }
            if vnf.active {
                vnf.increment = (((vnf.frq as u64) << FRACBITS) as i64) / MIXFREQ;
                if vnf.flags & SF_REVERSE != 0 {
                    vnf.increment = -vnf.increment;
                }
                let (vol, pan) = (vnf.vol, vnf.pan);
                vnf.lvolsel = (vol * (PAN_RIGHT - pan)) >> 8;
                vnf.rvolsel = (vol * pan) >> 8;
                let idxsize = if vnf.size != 0 { ((vnf.size as i64) << FRACBITS) - 1 } else { 0 };
                let idxlend = if vnf.repend != 0 { ((vnf.repend as i64) << FRACBITS) - 1 } else { 0 };
                let idxlpos = (vnf.reppos as i64) << FRACBITS;
                add_channel(vnf, &self.samples, &mut self.tickbuf, portion as i64, idxsize, idxlpos, idxlend);
            }
        }
        for (o, &x) in out[..n].iter_mut().zip(self.tickbuf[..n].iter()) {
            // EXTRACT_SAMPLE / CHECK_SAMPLE (16 bit)
            *o = (x >> BITSHIFT).clamp(-32768, 32767) as i16;
        }
    }
}

/// `AddChannel`: advance one voice by `todo` frames, adding it into `buf`.
fn add_channel(vnf: &mut Vinf, samples: &[Vec<i16>], buf: &mut [i32], mut todo: i64, idxsize: i64, idxlpos: i64, idxlend: i64) {
    let s = match samples.get(vnf.handle) {
        Some(s) if !s.is_empty() => s,
        _ => {
            vnf.current = 0;
            vnf.active = false;
            return;
        }
    };
    let mut ptr = 0usize;
    while todo > 0 {
        if vnf.flags & SF_REVERSE != 0 {
            if vnf.flags & SF_LOOP != 0 && vnf.current < idxlpos {
                if vnf.flags & SF_BIDI != 0 {
                    vnf.current = idxlpos + (idxlpos - vnf.current);
                    vnf.flags &= !SF_REVERSE;
                    vnf.increment = -vnf.increment;
                } else {
                    vnf.current = idxlend - (idxlpos - vnf.current);
                }
            } else if vnf.current < 0 {
                vnf.current = 0;
                vnf.active = false;
                break;
            }
        } else if vnf.flags & SF_LOOP != 0 && vnf.current >= idxlend {
            if vnf.flags & SF_BIDI != 0 {
                vnf.flags |= SF_REVERSE;
                vnf.increment = -vnf.increment;
                vnf.current = idxlend - (vnf.current - idxlend);
            } else {
                vnf.current = idxlpos + (vnf.current - idxlend);
            }
        } else if vnf.current >= idxsize {
            vnf.current = 0;
            vnf.active = false;
            break;
        }

        let end = if vnf.flags & SF_REVERSE != 0 {
            if vnf.flags & SF_LOOP != 0 {
                idxlpos
            } else {
                0
            }
        } else if vnf.flags & SF_LOOP != 0 {
            idxlend
        } else {
            idxsize
        };

        let done = if end == vnf.current || vnf.increment == 0 { 0 } else { ((end - vnf.current) / vnf.increment + 1).min(todo).max(0) };
        if done == 0 {
            vnf.active = false;
            break;
        }

        let endpos = vnf.current + done * vnf.increment;
        if vnf.vol != 0 {
            // MixStereoNormal
            let mut index = vnf.current;
            for _ in 0..done {
                let sample = s.get((index >> FRACBITS) as usize).copied().unwrap_or(0) as i32;
                index += vnf.increment;
                buf[ptr] = buf[ptr].wrapping_add(vnf.lvolsel.wrapping_mul(sample));
                buf[ptr + 1] = buf[ptr + 1].wrapping_add(vnf.rvolsel.wrapping_mul(sample));
                ptr += 2;
            }
            vnf.current = index;
        } else {
            vnf.current = endpos;
            ptr += done as usize * 2;
        }
        todo -= done;
    }
}
