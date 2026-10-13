# uplink-rs

A Rust port of **Uplink** (Introversion Software, 2001), made from the C++ source on the Uplink
Developer CD (version 1.31). The goal is the same game, with the same look and behaviour: where the
port and the original disagree, the original is right and the port has a bug. It is built on
macroquad (window, OpenGL, audio) and runs on Linux and Windows, with an experimental macOS build.

**You need your own copy of Uplink to play.** This repository contains no game data. The port reads
the `.dat` archives from an installed copy of Uplink and refuses to start without one.

This is a non-commercial fan project, published with the permission of Introversion Software. It is
not an official Introversion product. Buy Uplink from [GOG](https://www.gog.com/game/uplink_hacker_elite)
or [Steam](https://store.steampowered.com/app/1510/Uplink/).

## Requirements

- **Uplink game data.** Tested with the GOG release of Uplink (version 1.6). Other releases may work
  too; if you try one, please open an issue and tell us whether it worked. The port needs `data.dat`,
  `graphics.dat` and `fonts.dat`, and also uses `loading.dat`, `sounds.dat`, `music.dat` and
  `patch*.dat` when present.
- A Rust toolchain (stable, edition 2021) with `cargo`.
- Linux: X11 (OpenGL through GLX) and ALSA (`libasound` and its development files to build).
- Windows: see `docs/WINDOWS.md` for cross-compiling a 64-bit `uplink.exe`.
- macOS (experimental): builds as a universal binary (Intel and Apple Silicon) through the
  `macOS build` GitHub Actions workflow, and should work, but has not been play-tested on a Mac yet.
  The release build is unsigned; if macOS blocks it, run `xattr -d com.apple.quarantine uplink`.
  Music output on macOS is new and untested. If you try it, please open an issue and tell us how
  it went.
- English only.

## Build and run

    cd uplink-rs
    cargo build --release -p uplink
    UPLINK_DATA=/path/to/uplink cargo run --release -p uplink

Instead of `UPLINK_DATA` you can copy `target/release/uplink` next to the `.dat` files and run it from
there, or start it from a directory that holds them.

### Where the data is searched for

`find_data_dir` (`crates/engine/src/archive.rs`) takes the first of these directories that contains a
`data.dat`:

1. `$UPLINK_DATA`
2. the directory of the executable
3. the `uplink/` subfolder of the executable's directory
4. the current working directory

If none matches, or the required archives cannot be opened, the game prints an error and exits.

### Command line

Options are passed as in the original: `+name` turns an option on, `-name` turns it off, and
`!name value` sets it to a number (for example `+graphics_safemode`, `!graphics_screenwidth 1280`).
Safe mode (`graphics_safemode`) overrides the graphics options. A malformed argument aborts with an
error, as in the C++.

### Options and saves

Everything the game writes lives in `~/.uplink/` (without `$HOME`, in `users/` below the data
directory):

- `options`: the options file, JSON, version tag `SAV56`. It is read at start-up and written when the
  game shuts down normally.
- `<username>.usr`: one save per agent, JSON. The "NEWAGENT" placeholder is never saved.

These files are not compatible with the original game's binary options and save files (see below).

### Environment variables

| Variable | Effect |
|---|---|
| `UPLINK_DATA` | data directory, searched first (see above) |
| `UPLINK_CHEATS=1` | enable the cheat screen at run time (the C++ needs a build flag for this) |
| `UPLINK_NOSOUND` | keep sound effects and music off (used for headless runs) |
| `UPLINK_DEBUG` | with this set, F11 prints every Eclipse button to stdout |
| `UPLINK_MUSIC_DIR` | engine tests only: a folder with the six `.uni` songs for the music decoder test |
| `HOME` | location of `~/.uplink/` |

The play-test and smoke scripts additionally use `SMOKE_FRAMES` (frames to run in the engine's smoke
example).

### Exit

Quitting from the game's Exit button ends the process with exit code 1, like the C++ `exit(1)`. Closing
the window ends it with code 0. Both paths leave through `uplink_engine::exit_now`, which flushes
stdout and stderr and then calls `_exit`, so the process does not run library or thread-local
destructors while the audio threads are still alive.

## Crate layout

The workspace has two crates under `crates/`:

- `engine`: the platform layer that replaces the C++ libraries the game sat on: `archive` (the `.dat`
  files, mounted into an in-memory file system), `redshirt` (the game's file encryption), `gl` and
  `image` (drawing and picture loading), `text` and `mono` (hinted 1-bit TrueType text as GLTT drew
  it), `glut_fonts` (bitmap font fallback), `sound`, `music` and `mikmod` (a pure-Rust port of the
  MikMod `.uni` module player). It also has examples; `examples/smoke.rs` opens a window, runs
  `SMOKE_FRAMES` frames and exits.
- `uplink`: the game. The source tree mirrors the original C++: `source/a/b/c.cpp` is `src/a/b/c.rs`. The
  directories are `app`, `world`, `game`, `interface` (local and remote screens, task manager),
  `mainmenu`, `network`, `view` and the ports of the two libraries under `lib/`, `eclipse` (buttons
  and animations) and `vanbakel`. `ctx.rs` holds `Ctx`, the single object that replaces the C++
  globals `app` and `game`. The binary (`src/main.rs`) calls `uplink_main::main`.

The porting rules are in `docs/PORTING.md`. `docs/classmap.txt` maps C++ classes to Rust modules.

## Tests

    cd uplink-rs
    cargo test --workspace

(`tools/cargo-gate` is a wrapper that limits how many builds run at once on a shared machine; use it
in place of `cargo` if you share the machine.)

The unit tests open no window. Tests that need the game data mount it themselves with the search
order above and print "skipped" and pass when no data
is found. Tests that write files use a temporary directory, never `~/.uplink`.

Comparing the port against the original game on screen is done with the play-test harness
`tools/ul`, which runs both under virtual X displays, drives them with mouse and keyboard and takes
screenshots. See `docs/PLAYTEST.md`.

## Known differences from the original

- Saves and options are JSON. Original binary `.usr` and options files are not read, and files
  written by the port cannot be loaded by the original game.
- Sound and music come from the engine layer rather than SDL_mixer. Music is decoded by the Rust
  MikMod port and played through ALSA; with no sound device the songs are still timed so the
  playlist keeps cycling.
- Text is drawn with hinted 1-bit TrueType glyphs to match GLTT and the GOG release, with the SDL
  gucci's fallback to the default font for bitmap styles.
- `UplinkObject` load/save/print, `serialise.cpp`, the memory-leak checker, `dos2unix` and `resizex`
  are not ported (replaced by serde and standard collections). See section 6 of `docs/PORTING.md`.
- The port builds the full game configuration on Linux. The reference source is version 1.31 and the
  GOG release is 1.6, so some screens in the GOG game (for example a "Retire" button) have no
  counterpart in the source.

## License

The original Uplink source code is copyright Introversion Software and was released on the Uplink
Developer CD under the Uplink Developers License. This port is a derivative of that source and is
distributed under the same license, with written permission from Introversion Software to publish
the source and free builds. See `LICENSE`. Uplink, its name and its game data belong to
Introversion Software. The vendored `quad-snd` crate is MIT/Apache-2.0.
