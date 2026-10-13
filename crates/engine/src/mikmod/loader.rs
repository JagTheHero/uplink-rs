//! UNIMOD ("UN04" / "UN05") loader: a port of `load_uni.c`, `AllocXxx` from `mloader.c`
//! and the sample reading half of `sloader.c` / `VC1_SampleLoad` from `virtch_common.c`.
//!
//! Only the format family the game ships is supported: `UN04`/`UN05` (MikCvt3) modules
//! whose "instruments" are plain one-sample wrappers (`UF_INST` is cleared by the loader,
//! exactly as the original does), with 8 or 16 bit raw or delta coded sample data.
//! Anything else is rejected with an error instead of being played wrongly.

// Module flags (`UF_*`).
pub const UF_XMPERIODS: u16 = 0x0001;
pub const UF_LINEAR: u16 = 0x0002;
pub const UF_INST: u16 = 0x0004;
pub const UF_NOWRAP: u16 = 0x0080;

// Sample flags (`SF_*`) as stored in memory after loading.
pub const SF_16BITS: u16 = 0x0001;
pub const SF_SIGNED: u16 = 0x0004;
pub const SF_BIG_ENDIAN: u16 = 0x0008;
pub const SF_DELTA: u16 = 0x0010;
pub const SF_ITPACKED: u16 = 0x0020;
pub const SF_LOOP: u16 = 0x0100;
pub const SF_BIDI: u16 = 0x0200;
pub const SF_REVERSE: u16 = 0x0400;

/// Number of opcodes the UNITRK format defines (`UNI_LAST`).
pub const UNI_LAST: u8 = 61;

/// Operand byte count of a UNITRK opcode (`unioperands[]` in `munitrk.c`).
pub fn unioperands(op: u8) -> usize {
    match op {
        28 | 57 | 58 | 59 => 0,
        30 | 53 | 55 | 56 => 2,
        1..=60 => 1,
        _ => 0,
    }
}

/// One sample: metadata plus the decoded 16 bit signed data, padded the way the mixer expects.
pub struct Sample {
    pub speed: u32,
    pub volume: u8,
    pub length: u32,
    pub loopstart: u32,
    pub loopend: u32,
    pub flags: u16,
    pub globvol: u8,
    pub vibtype: u8,
    pub vibsweep: u8,
    pub vibdepth: u8,
    pub vibrate: u8,
    /// `length + 20` words; `data[length..]` holds the "unclick" tail.
    pub data: Vec<i16>,
}

pub struct Module {
    pub flags: u16,
    pub numchn: usize,
    pub numpos: u16,
    pub numpat: u16,
    pub numtrk: u16,
    pub numins: u16,
    pub initspeed: u8,
    pub inittempo: u8,
    pub initvolume: u8,
    pub positions: Vec<u16>,
    pub panning: [u16; 64],
    pub chanvol: [u8; 64],
    pub pattrows: Vec<u16>,
    pub patterns: Vec<u16>,
    pub tracks: Vec<Vec<u8>>,
    pub samples: Vec<Sample>,
}

struct Reader<'a> {
    d: &'a [u8],
    p: usize,
}

impl<'a> Reader<'a> {
    fn bytes(&mut self, n: usize, what: &str) -> Result<&'a [u8], String> {
        let end = self.p.checked_add(n).filter(|&e| e <= self.d.len()).ok_or_else(|| format!("truncated file while reading {what}"))?;
        let s = &self.d[self.p..end];
        self.p = end;
        Ok(s)
    }
    fn u8(&mut self, what: &str) -> Result<u8, String> {
        Ok(self.bytes(1, what)?[0])
    }
    fn u16(&mut self, what: &str) -> Result<u16, String> {
        let b = self.bytes(2, what)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }
    fn u32(&mut self, what: &str) -> Result<u32, String> {
        let b = self.bytes(4, what)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    /// `readstring`: a 16 bit length followed by that many bytes.
    fn string(&mut self, what: &str) -> Result<String, String> {
        let n = self.u16(what)? as usize;
        Ok(String::from_utf8_lossy(self.bytes(n, what)?).into_owned())
    }
}

/// Old (UN04/UN05) opcode numbers to the current ones. Opcodes 1..=24 are unchanged.
/// `load_uni.c` computes this mapping in `readtrack` but only uses it to validate the
/// track, so the original player runs the raw numbers (opcode 25 is then tremolo "R"
/// instead of tempo "T"). We keep the raw numbers as well: it is what the game plays.
fn remap_old(op: u8) -> u8 {
    match op {
        25 => 26,
        26 => 32,
        27 => 37,
        28 => 38,
        29 => 40,
        o => o,
    }
}

/// `readtrack`'s validity walk over a track; true when the track is well formed.
fn track_is_valid(t: &[u8]) -> bool {
    let len = t.len();
    let mut cur = 0usize;
    loop {
        let Some(&c) = t.get(cur) else { return false };
        cur += 1;
        if c == 0 {
            return true;
        }
        let mut chunk = (c & 0x1f) as i64 - 1;
        while chunk > 0 {
            if cur >= len {
                return false;
            }
            let raw = t[cur];
            if raw > 29 {
                return false;
            }
            let op = remap_old(raw);
            if op == 0 || op >= UNI_LAST {
                return false;
            }
            let oplen = unioperands(op) + 1;
            cur += oplen;
            chunk -= oplen as i64;
        }
        if chunk < 0 || cur >= len {
            return false;
        }
    }
}

struct RawSample {
    c2spd: u16,
    transpose: i8,
    volume: u8,
    length: u32,
    loopstart: u32,
    loopend: u32,
    flags: u16,
    vibtype: u8,
    vibsweep: u8,
    vibdepth: u8,
    vibrate: u8,
}

/// Parse a UN04/UN05 module and decode its samples.
pub fn load(bytes: &[u8]) -> Result<Module, String> {
    let mut r = Reader { d: bytes, p: 0 };
    let id = r.bytes(4, "id")?;
    if &id[..3] != b"UN0" || !(b'4'..=b'5').contains(&id[3]) {
        return Err(format!("not a UN04/UN05 module (id {:?})", String::from_utf8_lossy(id)));
    }
    let universion = id[3] - b'0';

    let numchn = r.u8("header")? as usize;
    let numpos = r.u16("header")?;
    let _reppos = if universion == 5 { r.u16("header")? } else { 0 };
    let numpat = r.u16("header")?;
    let numtrk = r.u16("header")?;
    let numins = r.u16("header")?;
    let initspeed = r.u8("header")?;
    let inittempo = r.u8("header")?;
    let hpositions = r.bytes(256, "positions")?;
    let hpanning = r.bytes(32, "panning")?;
    let mut flags = r.u8("header")? as u16;
    flags &= UF_XMPERIODS | UF_LINEAR;
    flags |= UF_INST | UF_NOWRAP;

    let _name = r.string("song name")?;
    let _old_type = r.string("old type")?; // universion < 0x102
    let _comment = r.string("comment")?;

    if numpos == 0 || numpat == 0 || numchn == 0 || numtrk == 0 || numins == 0 {
        return Err("empty module".into());
    }
    if numpos > 256 || numchn > 32 {
        return Err("bad module header".into());
    }

    let mut positions = vec![0u16; numpos as usize];
    for (t, p) in positions.iter_mut().enumerate() {
        *p = hpositions[t] as u16;
    }
    // `of.panning[t] = ((t+1)&2) ? 255 : 0` is overwritten for the first numchn channels.
    let mut panning = [0u16; 64];
    for (t, p) in panning.iter_mut().enumerate() {
        *p = if (t + 1) & 2 != 0 { 255 } else { 0 };
    }
    for t in 0..numchn {
        panning[t] = hpanning[t] as u16;
    }
    let chanvol = [64u8; 64];

    // Instruments (`loadinstr5`): each owns numsmp samples, stored right after it.
    struct Ins {
        samplenumber: [u16; 120],
        samplenote: [u8; 120],
        volpts: u8,
        panpts: u8,
    }
    let mut raw: Vec<RawSample> = Vec::new();
    let mut inst: Vec<Ins> = Vec::new();
    for _ in 0..numins {
        let nsmp = r.u8("instrument")? as usize;
        let mut samplenumber = [0xffffu16; 120];
        for u in 0..96 {
            samplenumber[u] = (raw.len() as u16).wrapping_add(r.u8("instrument")? as u16);
        }
        let mut pts = [0u8; 2];
        for e in 0..2 {
            let _flg = r.u8("envelope")?;
            pts[e] = r.u8("envelope")?;
            r.bytes(3, "envelope")?; // susbeg, beg, end
            r.bytes(12 * 4, "envelope points")?;
        }
        let vibtype = r.u8("instrument")?;
        let vibsweep = r.u8("instrument")?;
        let vibdepth = r.u8("instrument")?;
        let vibrate = r.u8("instrument")?;
        let _volfade = r.u16("instrument")?;
        let _insname = r.string("instrument name")?;
        for _ in 0..nsmp {
            let c2spd = r.u16("sample info")?;
            let transpose = r.u8("sample info")? as i8;
            let volume = r.u8("sample info")?;
            let _panning = r.u8("sample info")?;
            let length = r.u32("sample info")?;
            let loopstart = r.u32("sample info")?;
            let loopend = r.u32("sample info")?;
            let flags = r.u16("sample info")?;
            let _smpname = r.string("sample name")?;
            raw.push(RawSample { c2spd, transpose, volume, length, loopstart, loopend, flags, vibtype, vibsweep, vibdepth, vibrate });
        }
        inst.push(Ins { samplenumber, samplenote: [0; 120], volpts: pts[0], panpts: pts[1] });
    }
    if raw.is_empty() {
        return Err("module has no samples".into());
    }
    let numsmp = raw.len();
    for d in inst.iter_mut() {
        for t in 0..120 {
            d.samplenote[t] = if d.samplenumber[t] as usize >= numsmp {
                255
            } else {
                (t as i32 + raw[d.samplenumber[t] as usize].transpose as i32) as u8
            };
        }
    }

    // "Check if the original file had no instruments" (an instrument is just its sample).
    let mut trivial = numsmp == numins as usize;
    if trivial {
        for (t, d) in inst.iter().enumerate() {
            if d.volpts != 0 || d.panpts != 0 {
                trivial = false;
                break;
            }
            if (0..96).any(|u| d.samplenumber[u] as usize != t || d.samplenote[u] as usize != u) {
                trivial = false;
                break;
            }
        }
    }
    if !trivial {
        return Err("module uses real instruments (envelopes/keymaps), which this player does not support".into());
    }
    flags &= !(UF_INST | UF_NOWRAP);

    // Patterns (`AllocPatterns` + read). One extra pattern follows the real ones.
    let np = numpat as usize;
    let mut pattrows = vec![64u16; np + 1];
    let mut patterns = vec![0u16; (np + 1) * numchn];
    let mut tracks_n = 0u16;
    for p in patterns.iter_mut() {
        *p = tracks_n;
        tracks_n = tracks_n.wrapping_add(1);
    }
    for t in 0..np {
        pattrows[t] = r.u16("pattern rows")?;
    }
    for t in 0..np * numchn {
        patterns[t] = r.u16("pattern table")?;
    }

    // Tracks.
    let mut tracks = Vec::with_capacity(numtrk as usize);
    for _ in 0..numtrk {
        let len = r.u16("track")? as usize;
        if len == 0 {
            return Err("empty track".into());
        }
        let t = r.bytes(len, "track")?.to_vec();
        if !track_is_valid(&t) {
            return Err("malformed track".into());
        }
        tracks.push(t);
    }

    // Sample data follows the tracks, in sample order (only samples with a length).
    let mut samples = Vec::with_capacity(numsmp);
    for s in &raw {
        let mut f = 0u16;
        if s.flags & 128 != 0 {
            f |= SF_REVERSE;
        }
        if s.flags & 32 != 0 {
            f |= SF_BIDI;
        }
        if s.flags & 16 != 0 {
            f |= SF_LOOP;
        }
        if s.flags & 8 != 0 {
            f |= SF_BIG_ENDIAN;
        }
        if s.flags & 4 != 0 {
            f |= SF_DELTA;
        }
        if s.flags & 2 != 0 {
            f |= SF_SIGNED;
        }
        if s.flags & 1 != 0 {
            f |= SF_16BITS;
        }
        if f & SF_ITPACKED != 0 {
            return Err("IT packed samples are not supported".into());
        }
        let data = if s.length > 0 { read_sample(&mut r, s, f)? } else { Vec::new() };
        samples.push(Sample {
            speed: s.c2spd as u32,
            volume: s.volume,
            length: s.length,
            loopstart: s.loopstart,
            loopend: s.loopend,
            flags: f,
            globvol: 64,
            vibtype: s.vibtype,
            vibsweep: s.vibsweep,
            vibdepth: s.vibdepth,
            vibrate: s.vibrate,
            data,
        });
    }

    Ok(Module {
        flags,
        numchn,
        numpos,
        numpat,
        numtrk,
        numins,
        initspeed,
        inittempo,
        initvolume: 128,
        positions,
        panning,
        chanvol,
        pattrows,
        patterns,
        tracks,
        samples,
    })
}

/// `SL_Load` + `VC1_SampleLoad`: decode to signed 16 bit and append the unclick tail.
fn read_sample(r: &mut Reader, s: &RawSample, f: u16) -> Result<Vec<i16>, String> {
    let len = s.length as usize;
    let mut data = vec![0i16; len + 20];
    if f & SF_16BITS != 0 {
        let raw = r.bytes(len.checked_mul(2).ok_or("sample too big")?, "sample data")?;
        for (i, c) in raw.chunks_exact(2).enumerate() {
            data[i] = if f & SF_BIG_ENDIAN != 0 { i16::from_be_bytes([c[0], c[1]]) } else { i16::from_le_bytes([c[0], c[1]]) };
        }
    } else {
        let raw = r.bytes(len, "sample data")?;
        for (i, &b) in raw.iter().enumerate() {
            data[i] = ((b as i8) as i16) << 8;
        }
    }
    if f & SF_DELTA != 0 {
        let mut old = 0i16;
        for v in data[..len].iter_mut() {
            *v = v.wrapping_add(old);
            old = *v;
        }
    }
    if f & SF_SIGNED == 0 {
        for v in data[..len].iter_mut() {
            *v ^= i16::MIN;
        }
    }
    // Unclick: copy the first words of the loop (or silence) behind the loop end / sample end.
    let (ls, le) = (s.loopstart as usize, s.loopend as usize);
    if f & SF_LOOP != 0 {
        for t in 0..16usize {
            let v = if f & SF_BIDI != 0 { le.checked_sub(t + 1).and_then(|i| data.get(i)).copied() } else { data.get(t + ls).copied() };
            if let (Some(v), Some(d)) = (v, data.get_mut(le + t)) {
                *d = v;
            }
        }
    }
    Ok(data)
}
