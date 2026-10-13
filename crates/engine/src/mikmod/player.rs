//! Module player: a port of the parts of MikMod's `mplayer.c` that Uplink's `.uni` songs use
//! (Protracker style effects, a few Scream Tracker effects, no instruments/envelopes/NNA).
//!
//! The C integer widths are kept on purpose (`UWORD` periods wrap, `SBYTE` vibrato positions,
//! and so on) because the original relies on them. Unknown opcodes are logged once and skipped.

use super::loader::{unioperands, Module, SF_BIDI, SF_LOOP, UF_XMPERIODS};
use super::mixer::Mixer;

// Opcode numbers (the `UNI_*` enum).
const NOTE: u8 = 1;
const INSTRUMENT: u8 = 2;
const PT0: u8 = 3;
const PT1: u8 = 4;
const PT2: u8 = 5;
const PT3: u8 = 6;
const PT4: u8 = 7;
const PT5: u8 = 8;
const PT6: u8 = 9;
const PT7: u8 = 10;
const PT8: u8 = 11;
const PT9: u8 = 12;
const PTA: u8 = 13;
const PTB: u8 = 14;
const PTC: u8 = 15;
const PTD: u8 = 16;
const PTE: u8 = 17;
const PTF: u8 = 18;
const S3M_A: u8 = 19;
const S3M_D: u8 = 20;
const S3M_E: u8 = 21;
const S3M_F: u8 = 22;
const S3M_I: u8 = 23;
const S3M_Q: u8 = 24;
const S3M_R: u8 = 25;
const S3M_T: u8 = 26;
const S3M_U: u8 = 27;
const KEYOFF: u8 = 28;
const XM_4: u8 = 31;

const KICK_ABSENT: u8 = 0;
const KICK_NOTE: u8 = 1;
const KICK_KEYOFF: u8 = 2;
const KICK_ENV: u8 = 4;

const KEY_KICK: u8 = 0;
const KEY_OFF: u8 = 1;
const KEY_KILL: u8 = 3;

const POS_NONE: i16 = -2;

const OLDPERIODS: [u16; 24] = [
    1712 * 16, 1664 * 16, 1616 * 16, 1570 * 16, 1524 * 16, 1480 * 16, 1438 * 16, 1396 * 16, 1356 * 16, 1318 * 16, 1280 * 16, 1244 * 16, 1208 * 16,
    1174 * 16, 1140 * 16, 1108 * 16, 1076 * 16, 1046 * 16, 1016 * 16, 988 * 16, 960 * 16, 932 * 16, 906 * 16, 880 * 16,
];
const VIBRATO_TABLE: [u8; 32] = [
    0, 24, 49, 74, 97, 120, 141, 161, 180, 197, 212, 224, 235, 244, 250, 253, 255, 253, 250, 244, 235, 224, 212, 197, 180, 161, 141, 120, 97, 74, 49, 24,
];
const AVIBTAB: [u8; 128] = [
    0, 1, 3, 4, 6, 7, 9, 10, 12, 14, 15, 17, 18, 20, 21, 23, 24, 25, 27, 28, 30, 31, 32, 34, 35, 36, 38, 39, 40, 41, 42, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53,
    54, 54, 55, 56, 57, 57, 58, 59, 59, 60, 60, 61, 61, 62, 62, 62, 63, 63, 63, 63, 63, 63, 63, 64, 63, 63, 63, 63, 63, 63, 63, 62, 62, 62, 61, 61, 60, 60, 59,
    59, 58, 57, 57, 56, 55, 54, 54, 53, 52, 51, 50, 49, 48, 47, 46, 45, 44, 42, 41, 40, 39, 38, 36, 35, 34, 32, 31, 30, 28, 27, 25, 24, 23, 21, 20, 18, 17, 15,
    14, 12, 10, 9, 7, 6, 4, 3, 1,
];
const FINETUNE: [u16; 16] = [8363, 8413, 8463, 8529, 8581, 8651, 8723, 8757, 7895, 7941, 7985, 8046, 8107, 8169, 8232, 8280];

/// `MP_CONTROL`: per module channel effect state.
#[derive(Default)]
struct Ctl {
    s: Option<usize>,
    sample: u8,
    note: u8,
    outvolume: i16,
    chanvol: i8,
    fadevol: u16,
    panning: i16,
    kick: u8,
    period: u16,
    keyoff: u8,
    notedelay: u8,
    start: i32,
    /// `slave != NULL` (without NNA the voice with the same index as the channel).
    slave: bool,
    anote: u8,
    ownper: i16,
    ownvol: i16,
    row: Option<(usize, usize)>,
    retrig: i8,
    speed: u32,
    volume: i16,
    tmpvolume: i16,
    tmpperiod: u16,
    wantedperiod: u16,
    arpmem: u8,
    slidespeed: u16,
    portspeed: u16,
    s3mtremor: u8,
    s3mtronof: u8,
    s3mvolslide: u8,
    s3mrtgspeed: u8,
    s3mrtgslide: u8,
    glissando: u8,
    wavecontrol: u8,
    vibpos: i8,
    vibspd: u8,
    vibdepth: u8,
    trmpos: i8,
    trmspd: u8,
    trmdepth: u8,
    hioffset: u32,
    soffset: u16,
    newsamp: u16,
    pat_reppos: i16,
    pat_repcnt: u16,
}

/// `MP_VOICE`: what the mixer is told each tick.
#[derive(Default)]
struct Vc {
    s: Option<usize>,
    volume: i16,
    chanvol: i8,
    fadevol: u16,
    panning: i16,
    kick: u8,
    period: u16,
    keyoff: u8,
    start: i32,
    aswppos: u16,
}

/// The playback fields of `MODULE`.
struct State {
    bpm: u16,
    sngspd: u16,
    vbtick: u16,
    patpos: u16,
    sngpos: i16,
    volume: i16,
    initvolume: i16,
    patdly: u8,
    patdly2: u8,
    patbrk: u16,
    posjmp: i16,
    pat_repcrazy: u8,
    numrow: u16,
    panning: [u16; 64],
    avibpos: Vec<u16>,
    // SDL_mixer's music.c: extspd=1, panflag=1, wrap=0, loop=0, fadeout=0
    extspd: bool,
    panflag: bool,
    wrap: bool,
    looping: bool,
}

pub struct Player {
    m: Module,
    st: State,
    control: Vec<Ctl>,
    voice: Vec<Vc>,
    logged: [bool; 64],
    rng: u32,
}

/// `UniSetRow` / `UniGetByte` over one row of a track.
struct Cursor<'a> {
    t: &'a [u8],
    pc: usize,
    end: usize,
}

impl<'a> Cursor<'a> {
    fn new(t: &'a [u8], start: usize) -> Cursor<'a> {
        let len = (t.get(start).copied().unwrap_or(0) & 0x1f) as usize;
        Cursor { t, pc: start + 1, end: start + len }
    }
    fn byte(&mut self) -> u8 {
        if self.pc < self.end {
            let b = self.t.get(self.pc).copied().unwrap_or(0);
            self.pc += 1;
            b
        } else {
            0
        }
    }
    fn skip(&mut self, op: u8) {
        for _ in 0..unioperands(op) {
            self.byte();
        }
    }
}

/// `UniFindRow`: offset of the (repeat-expanded) row `row` in a track.
fn find_row(t: &[u8], mut row: u16) -> Option<usize> {
    let mut p = 0usize;
    loop {
        let c = *t.get(p)?;
        if c == 0 {
            return None;
        }
        let l = (c >> 5) as u16 + 1;
        if l > row {
            return Some(p);
        }
        row -= l;
        p += (c & 0x1f) as usize;
    }
}

fn get_period(note: u16, speed: u32) -> u16 {
    if speed == 0 {
        return 4242;
    }
    let n = (note % 24) as usize;
    let o = note / 24;
    ((8363u32.wrapping_mul(OLDPERIODS[n] as u32) >> o) / speed) as u16
}

fn get_frequency(period: u16) -> u32 {
    (8363 * 1712) / (if period != 0 { period as u32 } else { 1 })
}

fn rand_below(rng: &mut u32, ceil: u32) -> u16 {
    *rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
    ((*rng >> 16) & (ceil - 1)) as u16
}

/// Shared by vibrato, tremolo and fine vibrato: the waveform lookup.
fn waveform(wave: u8, pos: i8, rng: &mut u32) -> u16 {
    let mut q = ((pos >> 2) as u8) & 0x1f;
    match wave & 3 {
        0 => VIBRATO_TABLE[q as usize] as u16,
        1 => {
            q <<= 3;
            if pos < 0 {
                q = 255 - q;
            }
            q as u16
        }
        2 => 255,
        _ => rand_below(rng, 256),
    }
}

fn do_vibrato(a: &mut Ctl, st: &State, rng: &mut u32) {
    let mut temp = waveform(a.wavecontrol, a.vibpos, rng);
    temp = temp.wrapping_mul(a.vibdepth as u16);
    temp >>= 7;
    temp <<= 2;
    a.period = if a.vibpos >= 0 { a.tmpperiod.wrapping_add(temp) } else { a.tmpperiod.wrapping_sub(temp) };
    if st.vbtick != 0 {
        a.vibpos = a.vibpos.wrapping_add(a.vibspd as i8);
    }
}

fn do_tremolo(a: &mut Ctl, st: &State, rng: &mut u32) {
    let mut temp = waveform(a.wavecontrol >> 4, a.trmpos, rng);
    temp = temp.wrapping_mul(a.trmdepth as u16);
    temp >>= 6;
    if a.trmpos >= 0 {
        a.volume = a.tmpvolume.wrapping_add(temp as i16);
        if a.volume > 64 {
            a.volume = 64;
        }
    } else {
        a.volume = a.tmpvolume.wrapping_sub(temp as i16);
        if a.volume < 0 {
            a.volume = 0;
        }
    }
    if st.vbtick != 0 {
        a.trmpos = a.trmpos.wrapping_add(a.trmspd as i8);
    }
}

fn do_vol_slide(a: &mut Ctl, st: &State, dat: u8) {
    if st.vbtick == 0 {
        return;
    }
    if dat & 0xf != 0 {
        a.tmpvolume -= (dat & 0x0f) as i16;
        if a.tmpvolume < 0 {
            a.tmpvolume = 0;
        }
    } else {
        a.tmpvolume += (dat >> 4) as i16;
        if a.tmpvolume > 64 {
            a.tmpvolume = 64;
        }
    }
}

fn do_tone_slide(a: &mut Ctl, st: &State) {
    if st.vbtick != 0 {
        let dist = a.period as i32 - a.wantedperiod as i32;
        if dist == 0 || a.portspeed as i32 > dist.abs() {
            a.period = a.wantedperiod;
            a.tmpperiod = a.wantedperiod;
        } else if dist > 0 {
            a.tmpperiod = a.tmpperiod.wrapping_sub(a.portspeed);
            a.period = a.period.wrapping_sub(a.portspeed);
        } else {
            a.tmpperiod = a.tmpperiod.wrapping_add(a.portspeed);
            a.period = a.period.wrapping_add(a.portspeed);
        }
    } else {
        a.tmpperiod = a.period;
    }
}

fn tone_slide_kick(a: &mut Ctl) {
    if a.fadevol == 0 {
        a.kick = if a.kick == KICK_NOTE { KICK_NOTE } else { KICK_KEYOFF };
    } else {
        a.kick = if a.kick == KICK_NOTE { KICK_ENV } else { KICK_ABSENT };
    }
}

fn do_arpeggio(a: &mut Ctl, st: &State, dat: u8) {
    let mut note = a.note;
    if dat != 0 {
        match st.vbtick % 3 {
            1 => note = note.wrapping_add(dat >> 4),
            2 => note = note.wrapping_add(dat & 0xf),
            _ => {}
        }
        a.period = get_period((note as u16) << 1, a.speed);
        a.ownper = 1;
    }
}

fn do_e_effects(a: &mut Ctl, st: &mut State, ch: usize, dat: u8) {
    let mut nib = dat & 0xf;
    match dat >> 4 {
        0x1 => {
            if a.period != 0 && st.vbtick == 0 {
                a.tmpperiod = a.tmpperiod.wrapping_sub((nib << 2) as u16);
            }
        }
        0x2 => {
            if a.period != 0 && st.vbtick == 0 {
                a.tmpperiod = a.tmpperiod.wrapping_add((nib << 2) as u16);
            }
        }
        0x3 => a.glissando = nib,
        0x4 => {
            a.wavecontrol &= 0xf0;
            a.wavecontrol |= nib;
        }
        0x5 => {
            if a.period != 0 {
                a.speed = FINETUNE[nib as usize] as u32;
                a.tmpperiod = get_period((a.note as u16) << 1, a.speed);
            }
        }
        0x6 => {
            if st.vbtick != 0 {
                return;
            }
            if nib != 0 {
                if a.pat_repcnt != 0 {
                    a.pat_repcnt -= 1;
                } else {
                    a.pat_repcnt = nib as u16;
                }
                if a.pat_repcnt != 0 {
                    if a.pat_reppos == POS_NONE {
                        a.pat_reppos = (st.patpos as i32 - 1) as i16;
                    }
                    if a.pat_reppos == -1 {
                        st.pat_repcrazy = 1;
                        st.patpos = 0;
                    } else {
                        st.patpos = a.pat_reppos as u16;
                    }
                } else {
                    a.pat_reppos = POS_NONE;
                }
            } else {
                a.pat_reppos = (st.patpos as i32 - 1) as i16;
            }
        }
        0x7 => {
            a.wavecontrol &= 0x0f;
            a.wavecontrol |= nib << 4;
        }
        0x8 => {
            if st.panflag {
                nib = if nib <= 8 { nib << 4 } else { nib * 17 };
                a.panning = nib as i16;
                st.panning[ch] = nib as u16;
            }
        }
        0x9 => {
            if nib != 0 {
                if a.retrig == 0 {
                    if a.period != 0 {
                        a.kick = KICK_NOTE;
                    }
                    a.retrig = nib as i8;
                }
                a.retrig = a.retrig.wrapping_sub(1);
            }
        }
        0xa => {
            if st.vbtick == 0 {
                a.tmpvolume += nib as i16;
                if a.tmpvolume > 64 {
                    a.tmpvolume = 64;
                }
            }
        }
        0xb => {
            if st.vbtick == 0 {
                a.tmpvolume -= nib as i16;
                if a.tmpvolume < 0 {
                    a.tmpvolume = 0;
                }
            }
        }
        0xc => {
            if st.vbtick >= nib as u16 {
                a.tmpvolume = 0;
            }
        }
        0xd => {
            if st.vbtick == 0 {
                a.notedelay = nib;
            } else if a.notedelay != 0 {
                a.notedelay -= 1;
            }
        }
        0xe => {
            if st.vbtick == 0 && st.patdly2 == 0 {
                st.patdly = nib + 1;
            }
        }
        _ => {} // 0x0 filter, 0xf invert loop: not supported by MikMod either
    }
}

fn do_s3m_vol_slide(a: &mut Ctl, st: &State, inf: u8) {
    let inf = if inf != 0 {
        a.s3mvolslide = inf;
        inf
    } else {
        a.s3mvolslide
    };
    let lo = (inf & 0xf) as i16;
    let hi = (inf >> 4) as i16;
    if lo == 0 {
        if st.vbtick != 0 {
            a.tmpvolume += hi;
        }
    } else if hi == 0 {
        if st.vbtick != 0 {
            a.tmpvolume -= lo;
        }
    } else if lo == 0xf {
        if st.vbtick == 0 {
            a.tmpvolume += if hi != 0 { hi } else { 0xf };
        }
    } else if hi == 0xf {
        if st.vbtick == 0 {
            a.tmpvolume -= if lo != 0 { lo } else { 0xf };
        }
    } else {
        return;
    }
    a.tmpvolume = a.tmpvolume.clamp(0, 64);
}

fn do_s3m_slide(a: &mut Ctl, st: &State, inf: u8, up: bool) {
    let inf = if inf != 0 {
        a.slidespeed = inf as u16;
        inf
    } else {
        a.slidespeed as u8
    };
    let hi = inf >> 4;
    let lo = inf & 0xf;
    let amount = if hi == 0xf {
        if st.vbtick != 0 {
            return;
        }
        (lo as u16) << 2
    } else if hi == 0xe {
        if st.vbtick != 0 {
            return;
        }
        lo as u16
    } else {
        if st.vbtick == 0 {
            return;
        }
        (inf as u16) << 2
    };
    a.tmpperiod = if up { a.tmpperiod.wrapping_sub(amount) } else { a.tmpperiod.wrapping_add(amount) };
}

fn do_s3m_tremor(a: &mut Ctl, st: &State, inf: u8) {
    let inf = if inf != 0 {
        a.s3mtronof = inf;
        inf
    } else {
        if a.s3mtronof == 0 {
            return;
        }
        a.s3mtronof
    };
    if st.vbtick == 0 {
        return;
    }
    let on = (inf >> 4) + 1;
    let off = (inf & 0xf) + 1;
    a.s3mtremor %= on.wrapping_add(off);
    a.volume = if a.s3mtremor < on { a.tmpvolume } else { 0 };
    a.s3mtremor = a.s3mtremor.wrapping_add(1);
}

fn do_s3m_retrig(a: &mut Ctl, st: &State, inf: u8) {
    if inf != 0 {
        a.s3mrtgslide = inf >> 4;
        a.s3mrtgspeed = inf & 0xf;
    }
    if a.s3mrtgspeed > 0 {
        if a.retrig == 0 {
            if a.kick != KICK_NOTE {
                a.kick = KICK_KEYOFF;
            }
            a.retrig = a.s3mrtgspeed as i8;
            if st.vbtick != 0 {
                match a.s3mrtgslide {
                    1..=5 => a.tmpvolume -= 1 << (a.s3mrtgslide - 1),
                    6 => a.tmpvolume = (2 * a.tmpvolume) / 3,
                    7 => a.tmpvolume >>= 1,
                    9..=0xd => a.tmpvolume += 1 << (a.s3mrtgslide - 9),
                    0xe => a.tmpvolume = (3 * a.tmpvolume) >> 1,
                    0xf => a.tmpvolume <<= 1,
                    _ => {}
                }
                a.tmpvolume = a.tmpvolume.clamp(0, 64);
            }
        }
        a.retrig = a.retrig.wrapping_sub(1);
    }
}

fn do_s3m_fine_vibrato(a: &mut Ctl, rng: &mut u32) {
    let mut temp = waveform(a.wavecontrol, a.vibpos, rng);
    temp = temp.wrapping_mul(a.vibdepth as u16);
    temp >>= 8;
    a.period = if a.vibpos >= 0 { a.tmpperiod.wrapping_add(temp) } else { a.tmpperiod.wrapping_sub(temp) };
    a.vibpos = a.vibpos.wrapping_add(a.vibspd as i8);
}

fn do_s3m_tremolo(a: &mut Ctl, st: &State, rng: &mut u32) {
    let mut temp = waveform(a.wavecontrol >> 4, a.trmpos, rng);
    temp = temp.wrapping_mul(a.trmdepth as u16);
    temp >>= 7;
    if a.trmpos >= 0 {
        a.volume = a.tmpvolume.wrapping_add(temp as i16);
        if a.volume > 64 {
            a.volume = 64;
        }
    } else {
        a.volume = a.tmpvolume.wrapping_sub(temp as i16);
        if a.volume < 0 {
            a.volume = 0;
        }
    }
    if st.vbtick != 0 {
        a.trmpos = a.trmpos.wrapping_add(a.trmspd as i8);
    }
}

/// Effect `4` / `7` / `U` parameter memory shared by the three.
fn vib_params(a: &mut Ctl, dat: u8) {
    if dat & 0x0f != 0 {
        a.vibdepth = dat & 0xf;
    }
    if dat & 0xf0 != 0 {
        a.vibspd = (dat & 0xf0) >> 2;
    }
}

fn trm_params(a: &mut Ctl, dat: u8) {
    if dat & 0x0f != 0 {
        a.trmdepth = dat & 0xf;
    }
    if dat & 0xf0 != 0 {
        a.trmspd = (dat & 0xf0) >> 2;
    }
}

impl Player {
    pub fn new(m: Module) -> Result<Player, String> {
        if m.flags & UF_XMPERIODS != 0 {
            return Err("XM/linear period modules are not supported".into());
        }
        let numchn = m.numchn;
        let mut control: Vec<Ctl> = (0..numchn).map(|_| Ctl::default()).collect();
        for (t, c) in control.iter_mut().enumerate() {
            c.chanvol = m.chanvol[t] as i8;
            c.panning = m.panning[t] as i16;
        }
        let initspeed = m.initspeed;
        let sngspd = if initspeed != 0 { (initspeed as u16).min(32) } else { 6 };
        let volume = (m.initvolume as i16).min(128);
        let st = State {
            bpm: if m.inittempo < 32 { 32 } else { m.inittempo as u16 },
            sngspd,
            vbtick: sngspd,
            patpos: 0,
            sngpos: 0,
            volume,
            initvolume: m.initvolume as i16,
            patdly: 0,
            patdly2: 0,
            patbrk: 0,
            posjmp: 2, // make sure the player fetches the first note
            pat_repcrazy: 0,
            numrow: u16::MAX,
            panning: m.panning,
            avibpos: vec![0; m.samples.len()],
            extspd: true,
            panflag: true,
            wrap: false,
            looping: false,
        };
        Ok(Player { m, st, control, voice: (0..numchn).map(|_| Vc::default()).collect(), logged: [false; 64], rng: 1 })
    }

    /// `Player_SetVolume`: 0..=128.
    pub fn set_volume(&mut self, volume: i32) {
        let v = volume.clamp(0, 128) as i16;
        self.st.volume = v;
        self.st.initvolume = v;
    }

    /// `Player_Active`
    pub fn active(&self) -> bool {
        self.st.sngpos < self.m.numpos as i16
    }

    /// `Player_HandleTick`
    pub fn handle_tick(&mut self, mx: &mut Mixer) {
        if !self.active() {
            return;
        }
        let st = &mut self.st;
        st.vbtick = st.vbtick.wrapping_add(1);
        if st.vbtick >= st.sngspd {
            if st.pat_repcrazy != 0 {
                st.pat_repcrazy = 0; // play 2 times row 0
            } else {
                st.patpos = st.patpos.wrapping_add(1);
            }
            st.vbtick = 0;

            // pattern delay: patdly2 is the counter, patdly the command memory
            if st.patdly != 0 {
                st.patdly2 = st.patdly;
                st.patdly = 0;
            }
            if st.patdly2 != 0 {
                st.patdly2 -= 1;
                if st.patdly2 != 0 && st.patpos != 0 {
                    st.patpos -= 1;
                }
            }

            if st.patpos >= st.numrow && st.numrow > 0 && st.posjmp == 0 {
                st.posjmp = 3;
            }
            if st.posjmp != 0 {
                st.patpos = if st.numrow != 0 { st.patbrk % st.numrow } else { 0 };
                st.pat_repcrazy = 0;
                st.sngpos += st.posjmp - 2;
                for c in self.control.iter_mut() {
                    c.pat_reppos = -1;
                }
                st.patbrk = 0;
                st.posjmp = 0;
                let end = st.sngpos >= self.m.numpos as i16 || self.m.positions.get(st.sngpos as usize).copied() == Some(255);
                if end && !st.wrap {
                    return; // song over (Player_Active turns false)
                }
                if st.sngpos < 0 {
                    st.sngpos = self.m.numpos as i16 - 1;
                }
            }
            if st.patdly2 == 0 {
                self.notes();
            }
        }

        self.effects_pass1();
        self.setup_voices();
        self.update_voices(mx, 128);
    }

    /// `pt_Notes`: new notes and instruments for this row.
    fn notes(&mut self) {
        let m = &self.m;
        let st = &mut self.st;
        for ch in 0..m.numchn {
            let a = &mut self.control[ch];
            let tr;
            if st.sngpos >= m.numpos as i16 {
                tr = m.numtrk as usize;
                st.numrow = 0;
            } else {
                let pat = m.positions[st.sngpos as usize] as usize;
                tr = m.patterns.get(pat * m.numchn + ch).copied().unwrap_or(m.numtrk) as usize;
                st.numrow = m.pattrows.get(pat).copied().unwrap_or(0);
            }
            a.row = if tr < m.numtrk as usize { find_row(&m.tracks[tr], st.patpos).map(|p| (tr, p)) } else { None };
            a.newsamp = 0;
            if st.vbtick == 0 {
                a.notedelay = 0;
            }
            let Some((tr, start)) = a.row else { continue };
            let mut cur = Cursor::new(&m.tracks[tr], start);
            let mut funky = 0;
            loop {
                let c = cur.byte();
                if c == 0 {
                    break;
                }
                match c {
                    NOTE => {
                        funky |= 1;
                        a.anote = cur.byte();
                        a.kick = KICK_NOTE;
                        a.start = -1;
                        // retrig tremolo and vibrato waves?
                        if a.wavecontrol & 0x80 == 0 {
                            a.trmpos = 0;
                        }
                        if a.wavecontrol & 0x08 == 0 {
                            a.vibpos = 0;
                        }
                    }
                    INSTRUMENT => {
                        let inst = cur.byte();
                        if inst as u16 >= m.numins {
                            continue; // safety valve
                        }
                        funky |= 2;
                        a.retrig = 0;
                        a.s3mtremor = 0;
                        a.sample = inst;
                    }
                    _ => cur.skip(c),
                }
            }

            if funky != 0 {
                a.note = a.anote;
                let sidx = a.sample as usize;
                let Some(s) = m.samples.get(sidx) else { continue };
                if a.s != Some(sidx) {
                    a.s = Some(sidx);
                    a.newsamp = a.period;
                }
                // channel determined panning (SF_OWNPAN is never set by UN05 modules)
                a.panning = st.panning[ch] as i16;
                a.speed = s.speed;
                if funky & 2 != 0 {
                    a.volume = s.volume as i16;
                    a.tmpvolume = s.volume as i16;
                }
                a.tmpperiod = get_period((a.note as u16) << 1, a.speed);
                a.wantedperiod = a.tmpperiod;
                a.keyoff = KEY_KICK;
            }
        }
    }

    /// `pt_EffectsPass1`
    fn effects_pass1(&mut self) {
        let m = &self.m;
        let st = &mut self.st;
        for ch in 0..m.numchn {
            let a = &mut self.control[ch];
            if a.slave {
                let aout = &self.voice[ch];
                a.fadevol = aout.fadevol;
                a.period = aout.period;
                if a.kick == KICK_KEYOFF {
                    a.keyoff = aout.keyoff;
                }
            }
            let Some((tr, start)) = a.row else { continue };
            a.ownper = 0;
            a.ownvol = 0;
            play_effects(m, st, a, ch, Cursor::new(&m.tracks[tr], start), &mut self.rng, &mut self.logged);

            if a.ownper == 0 {
                a.period = a.tmpperiod;
            }
            if a.ownvol == 0 {
                a.volume = a.tmpvolume;
            }
            if let Some(s) = a.s.and_then(|i| m.samples.get(i)) {
                a.outvolume = ((a.volume as i32 * s.globvol as i32) >> 4) as i16;
                if a.outvolume > 256 {
                    a.volume = 256; // sic: the original assigns to `volume`, not `outvolume`
                } else if a.outvolume < 0 {
                    a.outvolume = 0;
                }
            }
        }
    }

    /// `pt_SetupVoices` (non NNA: channel N always drives voice N).
    fn setup_voices(&mut self) {
        for ch in 0..self.m.numchn {
            let a = &mut self.control[ch];
            if a.notedelay != 0 {
                continue;
            }
            if a.kick == KICK_NOTE {
                a.slave = true;
            }
            if a.slave {
                let aout = &mut self.voice[ch];
                aout.kick = a.kick;
                aout.s = a.s;
                aout.period = a.period;
                aout.panning = a.panning;
                aout.chanvol = a.chanvol;
                aout.fadevol = a.fadevol;
                aout.start = a.start;
                aout.volume = a.outvolume;
                aout.keyoff = a.keyoff;
            }
            a.kick = KICK_ABSENT;
        }
    }

    /// `pt_UpdateVoices`: push the voice state into the mixer.
    fn update_voices(&mut self, mx: &mut Mixer, max_volume: u32) {
        let m = &self.m;
        let st = &mut self.st;
        for ch in 0..m.numchn {
            let aout = &mut self.voice[ch];
            let Some(sidx) = aout.s else { continue };
            let s = &m.samples[sidx];
            if s.length == 0 {
                continue;
            }
            if aout.period < 40 {
                aout.period = 40;
            } else if aout.period > 50000 {
                aout.period = 50000;
            }

            if aout.kick == KICK_NOTE || aout.kick == KICK_KEYOFF {
                let start = if aout.start == -1 { 0 } else { aout.start as u32 };
                mx.voice_play(ch, sidx, s.flags, s.length, s.loopstart, s.loopend, start);
                aout.fadevol = 32768;
                aout.aswppos = 0;
            }
            aout.kick = KICK_ABSENT;

            // no envelopes: envvol 256
            let mut tmpvol: u32 = aout.fadevol as u32;
            tmpvol = tmpvol.wrapping_mul(aout.chanvol as u32);
            tmpvol = tmpvol.wrapping_mul(aout.volume as u32);
            tmpvol /= 16384;
            tmpvol = tmpvol.wrapping_mul(256);
            tmpvol = tmpvol.wrapping_mul(st.volume as u32);
            tmpvol /= 4194304;
            tmpvol = tmpvol.wrapping_mul(st.initvolume as u32);
            tmpvol /= 128;
            // fade out
            if st.sngpos >= m.numpos as i16 {
                tmpvol = 0;
            } else {
                tmpvol = tmpvol.wrapping_mul(max_volume) / 128;
            }
            mx.voice_set_volume(ch, tmpvol);
            mx.voice_set_panning(ch, aout.panning as i32);

            // XM style auto vibrato
            let mut vibval: i32 = if aout.period != 0 && s.vibdepth != 0 {
                let pos = st.avibpos[sidx];
                match s.vibtype {
                    0 => {
                        let v = AVIBTAB[(pos & 127) as usize] as i32;
                        if pos & 0x80 != 0 {
                            -v
                        } else {
                            v
                        }
                    }
                    1 => {
                        if pos & 0x80 != 0 {
                            -64
                        } else {
                            64
                        }
                    }
                    2 => 63 - (((pos as i32 + 128) & 255) >> 1),
                    _ => (((pos as i32 + 128) & 255) >> 1) - 64,
                }
            } else {
                0
            };
            let vibdpt: i32 = if aout.keyoff & KEY_OFF == 0 {
                if aout.aswppos < s.vibsweep as u16 {
                    let d = (aout.aswppos as i32 * s.vibdepth as i32) / s.vibsweep as i32;
                    aout.aswppos += 1;
                    d
                } else {
                    s.vibdepth as i32
                }
            } else if aout.aswppos >= s.vibsweep as u16 {
                s.vibdepth as i32
            } else {
                0
            };
            vibval = (vibval * vibdpt) >> 8;
            aout.period = (aout.period as i32 - vibval) as u16;
            st.avibpos[sidx] = (st.avibpos[sidx].wrapping_add(s.vibrate as u16)) & 0xff;

            let playperiod = aout.period;
            if aout.fadevol == 0 {
                mx.voice_stop(ch); // a dead note
            } else {
                mx.voice_set_frequency(ch, get_frequency(playperiod));
            }

            let bpm = st.bpm as i32; // + relspd (always 0 here)
            mx.bpm = bpm.clamp(32, 255) as u16;
        }
    }
}

/// `pt_playeffects`: run every effect of the current row of channel `ch`.
fn play_effects(m: &Module, st: &mut State, a: &mut Ctl, ch: usize, mut cur: Cursor, rng: &mut u32, logged: &mut [bool; 64]) {
    loop {
        let c = cur.byte();
        if c == 0 {
            break;
        }
        match c {
            NOTE | INSTRUMENT => cur.skip(c),
            PT0 => {
                let dat = cur.byte();
                if st.vbtick == 0 {
                    a.arpmem = dat;
                }
                if a.period != 0 {
                    do_arpeggio(a, st, a.arpmem);
                }
            }
            PT1 => {
                let dat = cur.byte();
                if st.vbtick == 0 && dat != 0 {
                    a.slidespeed = (dat as u16) << 2;
                }
                if a.period != 0 && st.vbtick != 0 {
                    a.tmpperiod = a.tmpperiod.wrapping_sub(a.slidespeed);
                }
            }
            PT2 => {
                let dat = cur.byte();
                if st.vbtick == 0 && dat != 0 {
                    a.slidespeed = (dat as u16) << 2;
                }
                if a.period != 0 && st.vbtick != 0 {
                    a.tmpperiod = a.tmpperiod.wrapping_add(a.slidespeed);
                }
            }
            PT3 => {
                let dat = cur.byte();
                if st.vbtick == 0 && dat != 0 {
                    a.portspeed = (dat as u16) << 2;
                }
                if a.period != 0 {
                    tone_slide_kick(a);
                    do_tone_slide(a, st);
                    a.ownper = 1;
                }
            }
            PT4 | XM_4 => {
                let dat = cur.byte();
                if st.vbtick == 0 {
                    vib_params(a, dat);
                } else if a.period != 0 {
                    do_vibrato(a, st, rng);
                    a.ownper = 1;
                }
            }
            PT5 => {
                let dat = cur.byte();
                if a.period != 0 {
                    tone_slide_kick(a);
                    do_tone_slide(a, st);
                    a.ownper = 1;
                }
                do_vol_slide(a, st, dat);
            }
            PT6 => {
                let dat = cur.byte();
                if a.period != 0 && st.vbtick != 0 {
                    do_vibrato(a, st, rng);
                    a.ownper = 1;
                }
                do_vol_slide(a, st, dat);
            }
            PT7 => {
                let dat = cur.byte();
                if st.vbtick == 0 {
                    trm_params(a, dat);
                }
                if a.period != 0 {
                    do_tremolo(a, st, rng);
                    a.ownvol = 1;
                }
            }
            PT8 => {
                let dat = cur.byte();
                if st.panflag {
                    a.panning = dat as i16;
                    st.panning[ch] = dat as u16;
                }
            }
            PT9 => {
                let dat = cur.byte();
                if st.vbtick == 0 {
                    if dat != 0 {
                        a.soffset = (dat as u16) << 8;
                    }
                    a.start = (a.hioffset | a.soffset as u32) as i32;
                    if let Some(s) = a.s.and_then(|i| m.samples.get(i)) {
                        if a.start as u32 > s.length {
                            a.start = (if s.flags & (SF_LOOP | SF_BIDI) != 0 { s.loopstart } else { s.length }) as i32;
                        }
                    }
                }
            }
            PTA => {
                let dat = cur.byte();
                do_vol_slide(a, st, dat);
            }
            PTB => {
                let dat = cur.byte();
                if st.vbtick != 0 || st.patdly2 != 0 {
                    continue;
                }
                // Vincent Voois uses a nasty trick in "Universal Bolero"
                if dat as i16 == st.sngpos && st.patbrk == st.patpos {
                    continue;
                }
                let last = st.sngpos as i32 == m.numpos as i32 - 1;
                if !st.looping && st.patbrk == 0 && ((dat as i16) < st.sngpos || (last && st.patbrk == 0) || (dat as i16 == st.sngpos && m.flags & 0x0080 != 0)) {
                    st.posjmp = 3;
                } else {
                    if last {
                        st.volume = st.initvolume.min(128);
                    }
                    st.sngpos = dat as i16;
                    st.posjmp = 2;
                    st.patpos = 0;
                }
            }
            PTC => {
                let mut dat = cur.byte();
                if st.vbtick != 0 {
                    continue;
                }
                if dat == 255 {
                    dat = 0;
                    a.anote = 0; // note cut
                } else if dat > 64 {
                    dat = 64;
                }
                a.tmpvolume = dat as i16;
            }
            PTD => {
                let mut dat = cur.byte();
                if st.vbtick != 0 || st.patdly2 != 0 {
                    continue;
                }
                if let Some(&pos) = m.positions.get(st.sngpos.max(0) as usize) {
                    if pos != 255 {
                        let rows = m.pattrows.get(pos as usize).copied().unwrap_or(64);
                        if dat as u16 > rows {
                            dat = rows as u8;
                        }
                    }
                }
                st.patbrk = dat as u16;
                if st.posjmp == 0 {
                    let last = st.sngpos as i32 == m.numpos as i32 - 1;
                    let curpat = m.positions.get(st.sngpos.max(0) as usize).copied().unwrap_or(0) as i32;
                    if last && dat != 0 && (st.looping || (curpat == m.numpat as i32 - 1 && m.flags & 0x0080 == 0)) {
                        st.sngpos = 0;
                        st.posjmp = 2;
                    } else {
                        st.posjmp = 3;
                    }
                }
            }
            PTE => {
                let dat = cur.byte();
                do_e_effects(a, st, ch, dat);
            }
            PTF => {
                let dat = cur.byte();
                if st.vbtick != 0 || st.patdly2 != 0 {
                    continue;
                }
                if st.extspd && dat >= 0x20 {
                    st.bpm = dat as u16;
                } else if dat != 0 {
                    st.sngspd = (dat as u16).min(32);
                    st.vbtick = 0;
                }
            }
            S3M_A => {
                let mut speed = cur.byte();
                if st.vbtick != 0 || st.patdly2 != 0 {
                    continue;
                }
                if speed > 128 {
                    speed -= 128;
                }
                if speed != 0 {
                    st.sngspd = speed as u16;
                    st.vbtick = 0;
                }
            }
            S3M_D => {
                let dat = cur.byte();
                do_s3m_vol_slide(a, st, dat);
            }
            S3M_E => {
                let dat = cur.byte();
                if a.period != 0 {
                    do_s3m_slide(a, st, dat, false);
                }
            }
            S3M_F => {
                let dat = cur.byte();
                if a.period != 0 {
                    do_s3m_slide(a, st, dat, true);
                }
            }
            S3M_I => {
                let dat = cur.byte();
                do_s3m_tremor(a, st, dat);
                a.ownvol = 1;
            }
            S3M_Q => {
                let dat = cur.byte();
                if a.period != 0 {
                    do_s3m_retrig(a, st, dat);
                }
            }
            S3M_R => {
                let dat = cur.byte();
                if st.vbtick == 0 {
                    trm_params(a, dat);
                }
                do_s3m_tremolo(a, st, rng);
                a.ownvol = 1;
            }
            S3M_T => {
                let tempo = cur.byte();
                if st.vbtick != 0 || st.patdly2 != 0 {
                    continue;
                }
                st.bpm = if tempo < 32 { 32 } else { tempo as u16 };
            }
            S3M_U => {
                let dat = cur.byte();
                if st.vbtick == 0 {
                    vib_params(a, dat);
                } else if a.period != 0 {
                    do_s3m_fine_vibrato(a, rng);
                    a.ownper = 1;
                }
            }
            KEYOFF => {
                a.keyoff |= KEY_OFF;
                a.keyoff = KEY_KILL; // no volume envelope: always a kill
            }
            _ => {
                if let Some(l) = logged.get_mut(c as usize) {
                    if !*l {
                        *l = true;
                        eprintln!("mikmod: song uses unsupported UNITRK opcode {c}; ignoring it");
                    }
                }
                cur.skip(c);
            }
        }
    }
}
