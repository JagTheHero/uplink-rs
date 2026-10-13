//! Module music playback (MikMod `.uni` files from `music.dat`).
//!
//! The song is decoded by the pure Rust port in [`crate::mikmod`] on a background thread, which
//! also feeds the sound card (ALSA on Linux, through the binding macroquad's audio already links; cpal
//! cannot coexist with it; WinMM `waveOut` on Windows). The thread blocks on the card's ~400 ms buffer, never the game thread:
//! `play`, `set_volume`, `stop` and `finished` only touch atomics. Without a sound device (or on
//! platforms without an output backend yet) the thread renders at real-time speed and discards the
//! audio, so `finished()` still fires after the song's true length and the playlist keeps cycling.

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::mikmod::{Song, SAMPLE_RATE};

/// Frames rendered per write (SDL_mixer's buffer was 4096 frames as well).
const CHUNK_FRAMES: usize = 4096;
/// How far ahead of the wall clock the music thread may render.
const MAX_LEAD: Duration = Duration::from_millis(500);

struct Shared {
    /// SDL_mixer volume, 0..=128.
    volume: AtomicI32,
    stop: AtomicBool,
    finished: AtomicBool,
}

thread_local! {
    static MUSIC: RefCell<Option<Arc<Shared>>> = const { RefCell::new(None) };
}

#[cfg(target_os = "linux")]
mod pcm {
    use quad_alsa_sys as sys;
    use std::ptr;

    /// A playback handle for 44.1 kHz stereo signed 16 bit.
    pub struct Pcm(*mut sys::snd_pcm_t);

    impl Pcm {
        pub fn open(rate: u32) -> Option<Pcm> {
            // SAFETY: plain ALSA calls; the handle is only used from the audio thread.
            unsafe {
                let mut handle = ptr::null_mut();
                // `UPLINK_ALSA_MUSIC_DEVICE` lets a test harness route the music to its own capture device.
                let custom = std::env::var("UPLINK_ALSA_MUSIC_DEVICE").ok().and_then(|d| std::ffi::CString::new(d).ok());
                let devices = [custom.as_deref(), Some(c"default"), Some(c"pipewire")];
                let opened = devices.iter().flatten().any(|d| sys::snd_pcm_open(&mut handle, d.as_ptr(), sys::SND_PCM_STREAM_PLAYBACK, 0) >= 0);
                if !opened {
                    return None;
                }
                // 400 ms of buffer: far above the fixed 2048-frame PipeWire quantum.
                let err = sys::snd_pcm_set_params(handle, sys::SND_PCM_FORMAT_S16_LE, sys::SND_PCM_ACCESS_RW_INTERLEAVED, 2, rate, 1, 400_000);
                if err < 0 {
                    sys::snd_pcm_close(handle);
                    return None;
                }
                Some(Pcm(handle))
            }
        }

        /// Blocks until the card has room. Returns false when the device is unusable.
        pub fn write(&mut self, samples: &[i16]) -> bool {
            let frames = samples.len() / 2;
            let mut done = 0usize;
            while done < frames {
                // SAFETY: the slice stays alive for the call and holds `frames - done` frames after `done`.
                let n = unsafe { sys::snd_pcm_writei(self.0, samples[done * 2..].as_ptr().cast(), (frames - done) as _) };
                if n < 0 {
                    // SAFETY: recover from an underrun/suspend.
                    if unsafe { sys::snd_pcm_recover(self.0, n as i32, 1) } < 0 {
                        return false;
                    }
                } else {
                    done += n as usize;
                }
            }
            true
        }

        pub fn drain(&mut self) {
            // SAFETY: valid handle.
            unsafe { sys::snd_pcm_drain(self.0) };
        }

        pub fn drop_now(&mut self) {
            // SAFETY: valid handle.
            unsafe { sys::snd_pcm_drop(self.0) };
        }
    }

    impl Drop for Pcm {
        fn drop(&mut self) {
            // SAFETY: valid handle, closed once.
            unsafe { sys::snd_pcm_close(self.0) };
        }
    }
}

/// Windows output through the WinMM `waveOut` API (what DirectSound-less 2001 Windows games used):
/// four buffers of one chunk each, refilled as the driver marks them done, so `write` blocks like the
/// ALSA backend does and ~370 ms are queued.
#[cfg(windows)]
mod pcm {
    use std::ffi::c_void;
    use std::mem::size_of;
    use std::ptr;
    use std::time::{Duration, Instant};

    const WAVE_MAPPER: u32 = u32::MAX;
    const WAVE_FORMAT_PCM: u16 = 1;
    const CALLBACK_NULL: u32 = 0;
    const WHDR_DONE: u32 = 1;
    const NUM_BUFFERS: usize = 4;

    #[repr(C)]
    struct WaveFormatEx {
        format_tag: u16,
        channels: u16,
        samples_per_sec: u32,
        avg_bytes_per_sec: u32,
        block_align: u16,
        bits_per_sample: u16,
        cb_size: u16,
    }

    #[repr(C)]
    struct WaveHdr {
        data: *mut u8,
        buffer_length: u32,
        bytes_recorded: u32,
        user: usize,
        flags: u32,
        loops: u32,
        next: *mut WaveHdr,
        reserved: usize,
    }

    #[link(name = "winmm")]
    extern "system" {
        fn waveOutOpen(out: *mut *mut c_void, device: u32, fmt: *const WaveFormatEx, callback: usize, instance: usize, flags: u32) -> u32;
        fn waveOutPrepareHeader(h: *mut c_void, hdr: *mut WaveHdr, size: u32) -> u32;
        fn waveOutUnprepareHeader(h: *mut c_void, hdr: *mut WaveHdr, size: u32) -> u32;
        fn waveOutWrite(h: *mut c_void, hdr: *mut WaveHdr, size: u32) -> u32;
        fn waveOutReset(h: *mut c_void) -> u32;
        fn waveOutClose(h: *mut c_void) -> u32;
    }

    struct Buffer {
        hdr: Box<WaveHdr>,
        samples: Vec<i16>,
        queued: bool,
    }

    /// A playback handle for 44.1 kHz stereo signed 16 bit.
    pub struct Pcm {
        handle: *mut c_void,
        buffers: Vec<Buffer>,
        next: usize,
    }

    impl Pcm {
        pub fn open(rate: u32) -> Option<Pcm> {
            let fmt = WaveFormatEx {
                format_tag: WAVE_FORMAT_PCM,
                channels: 2,
                samples_per_sec: rate,
                avg_bytes_per_sec: rate * 4,
                block_align: 4,
                bits_per_sample: 16,
                cb_size: 0,
            };
            let mut handle = ptr::null_mut();
            // SAFETY: plain WinMM call; `fmt` outlives it and `handle` is only used from the audio thread.
            if unsafe { waveOutOpen(&mut handle, WAVE_MAPPER, &fmt, 0, 0, CALLBACK_NULL) } != 0 {
                return None;
            }
            let buffers = (0..NUM_BUFFERS)
                .map(|_| Buffer {
                    hdr: Box::new(WaveHdr { data: ptr::null_mut(), buffer_length: 0, bytes_recorded: 0, user: 0, flags: 0, loops: 0, next: ptr::null_mut(), reserved: 0 }),
                    samples: Vec::new(),
                    queued: false,
                })
                .collect();
            Some(Pcm { handle, buffers, next: 0 })
        }

        fn is_done(buf: &Buffer) -> bool {
            // SAFETY: the driver sets WHDR_DONE in the header we gave it; read it without caching.
            !buf.queued || unsafe { ptr::read_volatile(&buf.hdr.flags) } & WHDR_DONE != 0
        }

        fn release(&mut self, i: usize) {
            let handle = self.handle;
            let buf = &mut self.buffers[i];
            if buf.queued {
                // SAFETY: the header was prepared on this handle and is no longer playing.
                unsafe { waveOutUnprepareHeader(handle, &mut *buf.hdr, size_of::<WaveHdr>() as u32) };
                buf.queued = false;
            }
        }

        /// Blocks until a buffer is free. Returns false when the device is unusable.
        pub fn write(&mut self, samples: &[i16]) -> bool {
            let i = self.next;
            let start = Instant::now();
            while !Self::is_done(&self.buffers[i]) {
                if start.elapsed() > Duration::from_secs(5) {
                    return false;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            self.release(i);
            let handle = self.handle;
            let buf = &mut self.buffers[i];
            buf.samples.clear();
            buf.samples.extend_from_slice(samples);
            buf.hdr.data = buf.samples.as_mut_ptr().cast();
            buf.hdr.buffer_length = (buf.samples.len() * 2) as u32;
            buf.hdr.flags = 0;
            // SAFETY: `hdr` and `samples` stay alive and unmoved until the buffer is released again.
            let ok = unsafe {
                waveOutPrepareHeader(handle, &mut *buf.hdr, size_of::<WaveHdr>() as u32) == 0
                    && waveOutWrite(handle, &mut *buf.hdr, size_of::<WaveHdr>() as u32) == 0
            };
            buf.queued = ok;
            self.next = (i + 1) % NUM_BUFFERS;
            ok
        }

        pub fn drain(&mut self) {
            let start = Instant::now();
            while self.buffers.iter().any(|b| !Self::is_done(b)) && start.elapsed() < Duration::from_secs(2) {
                std::thread::sleep(Duration::from_millis(5));
            }
        }

        pub fn drop_now(&mut self) {
            // SAFETY: valid handle; marks every queued buffer done.
            unsafe { waveOutReset(self.handle) };
        }
    }

    impl Drop for Pcm {
        fn drop(&mut self) {
            // SAFETY: valid handle; reset first so no buffer is still playing when it is released.
            unsafe { waveOutReset(self.handle) };
            for i in 0..self.buffers.len() {
                self.release(i);
            }
            // SAFETY: closed once.
            unsafe { waveOutClose(self.handle) };
        }
    }
}

/// No output backend on this platform yet: the song is paced in real time and discarded.
#[cfg(not(any(target_os = "linux", windows)))]
mod pcm {
    pub struct Pcm;
    impl Pcm {
        pub fn open(_rate: u32) -> Option<Pcm> {
            None
        }
        pub fn write(&mut self, _samples: &[i16]) -> bool {
            true
        }
        pub fn drain(&mut self) {}
        pub fn drop_now(&mut self) {}
    }
}

fn audio_thread(mut song: Song, shared: Arc<Shared>) {
    let mut pcm = pcm::Pcm::open(SAMPLE_RATE);
    if pcm.is_none() {
        eprintln!("music: no sound device, playing silently");
    }
    let chunk_time = Duration::from_secs_f64(CHUNK_FRAMES as f64 / SAMPLE_RATE as f64);
    let mut buf = vec![0i16; CHUNK_FRAMES * 2];
    let mut volume = shared.volume.load(Ordering::Relaxed);
    // A device that does not block (a null sink) would let us render the whole song in an instant.
    let started = std::time::Instant::now();
    let mut played = Duration::ZERO;
    while song.active() && !shared.stop.load(Ordering::Relaxed) {
        let v = shared.volume.load(Ordering::Relaxed);
        if v != volume {
            volume = v;
            song.set_volume(v);
        }
        song.render(&mut buf);
        played += chunk_time;
        if let Some(wait) = played.checked_sub(started.elapsed() + MAX_LEAD) {
            std::thread::sleep(wait);
        }
        let ok = pcm.as_mut().is_some_and(|p| p.write(&buf));
        if !ok {
            pcm = None;
            std::thread::sleep(chunk_time);
        }
    }
    if let Some(p) = pcm.as_mut() {
        if shared.stop.load(Ordering::Relaxed) {
            p.drop_now();
        } else {
            p.drain();
        }
    }
    shared.finished.store(true, Ordering::Relaxed);
}

/// `volume` is SDL_mixer's 0..=128 (the original passed `playerVolume` straight to `Mix_VolumeMusic`).
pub fn play(filename: &str, volume: i32) {
    stop();
    let volume = volume.clamp(0, 128);
    let Some(bytes) = crate::archive::read(filename) else {
        eprintln!("music: {filename} not found");
        return;
    };
    let song = match Song::new(&bytes, volume) {
        Ok(song) => song,
        Err(e) => {
            eprintln!("music: cannot load {filename}: {e}");
            return;
        }
    };
    let shared = Arc::new(Shared { volume: AtomicI32::new(volume), stop: AtomicBool::new(false), finished: AtomicBool::new(false) });
    let thread_shared = Arc::clone(&shared);
    if std::thread::Builder::new().name("music".into()).spawn(move || audio_thread(song, thread_shared)).is_ok() {
        MUSIC.with(|m| *m.borrow_mut() = Some(shared));
    }
}

pub fn set_volume(volume: i32) {
    MUSIC.with(|m| {
        if let Some(s) = m.borrow().as_ref() {
            s.volume.store(volume.clamp(0, 128), Ordering::Relaxed);
        }
    });
}

pub fn stop() {
    MUSIC.with(|m| {
        if let Some(s) = m.borrow_mut().take() {
            s.stop.store(true, Ordering::Relaxed);
        }
    });
}

pub fn finished() -> bool {
    MUSIC.with(|m| m.borrow().as_ref().is_none_or(|s| s.finished.load(Ordering::Relaxed)))
}
