//! Port of `soundgarden`: sound effects and the music playlist.
//!
//! Sound effects are WAV files from the archive VFS, played through macroquad's audio.
//! Music (`.uni` MikMod modules in `music.dat`) goes through [`crate::music`].

use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

use macroquad::audio::{self, PlaySoundParams, Sound};

use crate::archive;

/// `SDL_MIX_MAXVOLUME / 2`: the volume `playerVolume` starts at.
const SDL_MIX_MAXVOLUME: i32 = 128;
/// `SgPlaySound` plays every chunk at `MIX_MAX_VOLUME / 4`.
const CHUNK_VOLUME: f32 = (SDL_MIX_MAXVOLUME / 4) as f32 / SDL_MIX_MAXVOLUME as f32;

struct SoundState {
    enabled: bool,
    log: bool,
    cache: HashMap<String, Option<Sound>>,
    playlists: Vec<(String, Vec<String>)>,
    // soundgarden.cpp statics
    currentplaylist: String,
    currentsong: String,
    requestedplaylist: String,
    requestedtime: i64,
    songindex: i32,
    mod_volume: i32,
}

impl Default for SoundState {
    fn default() -> Self {
        SoundState {
            enabled: false,
            log: false,
            cache: HashMap::new(),
            playlists: Vec::new(),
            currentplaylist: "None".into(),
            currentsong: "None".into(),
            requestedplaylist: "None".into(),
            requestedtime: -1,
            songindex: -1,
            mod_volume: 20,
        }
    }
}

thread_local! {
    static SG: RefCell<SoundState> = RefCell::new(SoundState::default());
}

/// `EclGetAccurateTime`: milliseconds on a monotonic clock.
fn accurate_time() -> i64 {
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    START.get_or_init(std::time::Instant::now).elapsed().as_millis() as i64
}

/// With `UPLINK_SOUNDLOG` set, every sound event is printed with a wall clock time (ms since the
/// epoch) so a captured audio stream can be lined up with what the game asked for.
pub(crate) fn log_event(msg: &str) {
    if SG.with(|s| s.borrow().log) {
        let ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
        eprintln!("SNDLOG {ms} {msg}");
    }
}

/// Poll a future to completion. macroquad's native audio loaders finish synchronously.
pub(crate) fn block_on<F: Future>(fut: F) -> F::Output {
    fn noop_raw() -> RawWaker {
        fn clone(_: *const ()) -> RawWaker {
            noop_raw()
        }
        fn noop(_: *const ()) {}
        static VT: RawWakerVTable = RawWakerVTable::new(clone, noop, noop, noop);
        RawWaker::new(std::ptr::null(), &VT)
    }
    let waker = unsafe { Waker::from_raw(noop_raw()) };
    let mut cx = Context::from_waker(&waker);
    let mut fut = pin!(fut);
    loop {
        if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
            return v;
        }
        std::thread::yield_now();
    }
}

/// macroquad's WASAPI backend panics on its audio thread when the machine has no output device
/// (and Windows builds here cannot unwind, so that would abort the process): check first.
#[cfg(windows)]
fn has_output_device() -> bool {
    #[link(name = "winmm")]
    extern "system" {
        fn waveOutGetNumDevs() -> u32;
    }
    // SAFETY: takes no arguments and only counts the installed devices.
    let found = unsafe { waveOutGetNumDevs() } > 0;
    if !found {
        eprintln!("SoundGarden: no audio output device, sound is off");
    }
    found
}

#[cfg(not(windows))]
fn has_output_device() -> bool {
    true
}

/// `SgInitialise`. Setting `UPLINK_NOSOUND` keeps sound and music off (headless test runs).
pub fn initialise() {
    let enabled = std::env::var_os("UPLINK_NOSOUND").is_none() && has_output_device();
    let log = std::env::var_os("UPLINK_SOUNDLOG").is_some();
    SG.with(|s| {
        let mut s = s.borrow_mut();
        s.enabled = enabled;
        s.log = log;
    });
}

/// `SgShutdown`
pub fn shutdown() {
    stop_mod();
    SG.with(|s| s.borrow_mut().enabled = false);
}

/// `SgPlaySound(filename, id, synchronised)`. `filename` is an archive path such as
/// `"sounds/login.wav"`. `id` is the cache key (the filename when `None`); `synchronised` is
/// accepted for API parity.
pub fn play_sound(filename: &str, id: Option<&str>, _synchronised: bool) {
    let enabled = SG.with(|s| s.borrow().enabled);
    if !enabled {
        return;
    }
    let key = id.unwrap_or(filename).to_ascii_lowercase();
    let cached = SG.with(|s| s.borrow().cache.get(&key).cloned());
    let sound = match cached {
        Some(s) => s,
        None => {
            let loaded = archive::read(filename).and_then(|bytes| block_on(audio::load_sound_from_bytes(&bytes)).ok());
            if loaded.is_none() {
                eprintln!("SoundGarden: could not load {filename}");
            }
            SG.with(|s| s.borrow_mut().cache.insert(key, loaded.clone()));
            loaded
        }
    };
    if let Some(sound) = sound {
        log_event(&format!("sound {filename}"));
        audio::play_sound(&sound, PlaySoundParams { looped: false, volume: CHUNK_VOLUME });
    }
}

/// SDL_mixer volume (0..=128) for a `playerVolume`. The GOG 1.6 `uplink.exe` runs
/// `Mix_VolumeMusic((int)(playerVolume / 20.0 * 85.333))` (disassembly of `SgSetModVolume`), so the
/// default 20 plays at 85; the 1.31 source passed it straight through, which is about 33 dB quieter
/// once MikMod applies the volume twice (see `Player::set_volume`).
fn sdl_music_volume(player_volume: i32) -> i32 {
    ((player_volume as f64 / 20.0 * 85.333) as i32).clamp(0, SDL_MIX_MAXVOLUME)
}

/// `SgPlayMod(filename)`
pub fn play_mod(filename: &str) {
    if !SG.with(|s| s.borrow().enabled) {
        return;
    }
    let volume = SG.with(|s| s.borrow().mod_volume);
    log_event(&format!("mod {filename} volume {volume}"));
    crate::music::play(filename, sdl_music_volume(volume));
}

/// `SgSetModVolume`: `playerVolume`, 0 = silent, 20 = the game's default.
pub fn set_mod_volume(newvolume: i32) {
    let v = newvolume.clamp(0, 128);
    SG.with(|s| s.borrow_mut().mod_volume = v);
    crate::music::set_volume(sdl_music_volume(v));
}

pub fn get_mod_volume() -> i32 {
    SG.with(|s| s.borrow().mod_volume)
}

/// `SgModFinished`: false while sound is off (the original never got that far either).
pub fn mod_finished() -> bool {
    SG.with(|s| s.borrow().enabled) && crate::music::finished()
}

pub fn stop_mod() {
    crate::music::stop();
}

/// `SgPlaylist_Initialise`
pub fn playlist_initialise() {
    SG.with(|s| {
        let mut s = s.borrow_mut();
        s.currentplaylist = "None".into();
        s.currentsong = "None".into();
        s.requestedplaylist = "None".into();
        s.requestedtime = -1;
        s.songindex = -1;
    });
}

/// `SgPlaylist_Shutdown`
pub fn playlist_shutdown() {
    SG.with(|s| s.borrow_mut().playlists.clear());
}

/// `SgPlaylist_Create`
pub fn playlist_create(playlist: &str) {
    SG.with(|s| s.borrow_mut().playlists.push((playlist.to_string(), Vec::new())));
}

/// `SgPlaylist_AddSong`
pub fn playlist_add_song(playlist: &str, songname: &str) {
    SG.with(|s| match s.borrow_mut().playlists.iter_mut().find(|(n, _)| n == playlist) {
        Some((_, songs)) => songs.push(songname.to_string()),
        None => println!("SgPlaylist_AddSong : Failed because playlist {playlist} does not exist"),
    });
}

/// `SgPlaylist_Play`: starts the playlist at once when nothing is playing, otherwise asks for a
/// fade-out of the current one first (see [`update`]). Asking for the playlist that is already
/// playing does nothing.
pub fn playlist_play(playlist: &str) {
    let start_now = SG.with(|s| {
        let mut s = s.borrow_mut();
        if s.currentplaylist == "None" {
            s.currentplaylist = playlist.to_string();
            s.songindex = -1;
            true
        } else {
            if s.currentplaylist != playlist {
                if s.requestedplaylist == "None" {
                    s.requestedtime = accurate_time();
                }
                s.requestedplaylist = playlist.to_string();
            }
            false
        }
    });
    if start_now {
        set_mod_volume(20);
        playlist_next_song();
    }
}

/// `SgPlaylist_Stop`
pub fn playlist_stop() {
    stop_mod();
    SG.with(|s| {
        let mut s = s.borrow_mut();
        s.currentplaylist = "None".into();
        s.currentsong = "None".into();
        s.requestedplaylist = "None".into();
        s.requestedtime = -1;
        s.songindex = -1;
    });
}

/// `SgPlaylist_NextSong`: the songs of a playlist are played in order, wrapping round.
pub fn playlist_next_song() {
    let next = SG.with(|s| {
        let mut s = s.borrow_mut();
        let Some((_, songs)) = s.playlists.iter().find(|(n, _)| *n == s.currentplaylist) else {
            println!("SgPlaylist_NextSong : Failed because playlist {} does not exist", s.currentplaylist);
            return None;
        };
        if songs.is_empty() {
            return None;
        }
        let mut idx = s.songindex + 1;
        if idx >= songs.len() as i32 {
            idx = 0;
        }
        let song = songs[idx as usize].clone();
        s.songindex = idx;
        s.currentsong = song.clone();
        Some(song)
    });
    if let Some(song) = next {
        play_mod(&song);
    }
}

/// `SgPlaylist_RandomSong` (never called by the game, kept for parity).
pub fn playlist_random_song() {
    let next = SG.with(|s| {
        let mut s = s.borrow_mut();
        let Some((_, songs)) = s.playlists.iter().find(|(n, _)| *n == s.currentplaylist) else {
            println!("SgPlaylist_RandomSong : Failed because playlist {} does not exist", s.currentplaylist);
            return None;
        };
        if songs.is_empty() {
            return None;
        }
        let old = s.currentsong.clone();
        let mut song = songs[macroquad::rand::gen_range(0, songs.len())].clone();
        while songs.len() > 1 && song == old {
            song = songs[macroquad::rand::gen_range(0, songs.len())].clone();
        }
        s.currentsong = song.clone();
        Some(song)
    });
    if let Some(song) = next {
        play_mod(&song);
    }
}

/// `SgUpdate`: call every frame. Fades the music out while another playlist is requested (then
/// switches to it), otherwise advances the playlist when the current module has finished.
pub fn update() {
    let (requested, requestedtime, current) = SG.with(|s| {
        let s = s.borrow();
        (s.requestedplaylist.clone(), s.requestedtime, s.currentplaylist.clone())
    });
    if requested != "None" {
        let timediff = accurate_time() - requestedtime;
        let volume = (20.0 - (20.0 * (timediff as f32 / 4000.0_f32))) as i32;
        if volume <= 0 {
            stop_mod();
            SG.with(|s| {
                let mut s = s.borrow_mut();
                s.currentplaylist = requested;
                s.requestedplaylist = "None".into();
                s.requestedtime = -1;
                s.songindex = -1;
            });
            set_mod_volume(20);
            playlist_next_song();
        } else {
            set_mod_volume(volume);
        }
    } else if current != "None" && mod_finished() {
        playlist_next_song();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn current_song() -> String {
        SG.with(|s| s.borrow().currentsong.clone())
    }

    #[test]
    fn playlist_plays_songs_in_order_and_wraps() {
        playlist_shutdown();
        playlist_initialise();
        playlist_create("main");
        for s in ["a", "b", "c"] {
            playlist_add_song("main", s);
        }
        playlist_play("main");
        assert_eq!(current_song(), "a");
        playlist_next_song();
        playlist_next_song();
        assert_eq!(current_song(), "c");
        playlist_next_song();
        assert_eq!(current_song(), "a");
        // asking for the playlist that is already playing changes nothing
        playlist_play("main");
        assert_eq!(current_song(), "a");
        assert_eq!(SG.with(|s| s.borrow().requestedplaylist.clone()), "None");
        playlist_stop();
        assert_eq!(current_song(), "None");
        playlist_shutdown();
    }

    #[test]
    fn another_playlist_is_requested_for_a_fade() {
        playlist_shutdown();
        playlist_initialise();
        playlist_create("main");
        playlist_create("other");
        playlist_add_song("main", "a");
        playlist_add_song("other", "x");
        playlist_play("main");
        playlist_play("other");
        assert_eq!(current_song(), "a");
        assert_eq!(SG.with(|s| s.borrow().requestedplaylist.clone()), "other");
        playlist_stop();
        playlist_shutdown();
    }

    #[test]
    fn default_volume_maps_to_sdl_85() {
        assert_eq!(sdl_music_volume(20), 85);
        assert_eq!(sdl_music_volume(0), 0);
        assert_eq!(sdl_music_volume(10), 42);
        assert_eq!(sdl_music_volume(1000), 128);
        assert_eq!(CHUNK_VOLUME, 0.25);
    }
}
