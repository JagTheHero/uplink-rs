//! Platform layer for the Rust port of Uplink.
//!
//! Replaces the original's in-house support libraries:
//! - `bungle` + `redshirt` -> [`archive`], [`redshirt`]
//! - `gucci` (graphics, images, fonts) + raw OpenGL 1.x -> [`gl`], [`image`], [`text`] (+ [`mono`], [`glut_fonts`])
//! - `soundgarden` (sound + mod music) -> [`sound`], [`music`]
//!
//! `eclipse` (buttons/animations), `vanbakel` (task manager) and `tosser`
//! (containers; replaced by std collections) live in the game crate, because their
//! callbacks need the game context.

pub mod archive;
pub mod gl;
pub mod glut_fonts;
pub mod image;
pub mod mono;
pub mod mikmod;
pub mod music;
pub mod redshirt;
pub mod sound;
pub mod text;

pub use macroquad;

/// End the process now with `code`, without running C `atexit` handlers, library destructors or
/// thread-local destructors.
///
/// macroquad starts an audio thread (and the music player another) that keep talking to ALSA and the
/// GL driver until the process dies; a normal `exit` tears those libraries down underneath them, which
/// can segfault at shutdown. Everything the game wants kept is written before this is called; stdout and
/// stderr are flushed here.
pub fn exit_now(code: i32) -> ! {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    #[cfg(unix)]
    {
        extern "C" {
            fn _exit(status: i32) -> !;
        }
        // SAFETY: `_exit` takes no pointers and never returns.
        unsafe { _exit(code) }
    }
    #[cfg(not(unix))]
    std::process::exit(code)
}
