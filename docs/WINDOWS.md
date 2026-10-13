# Windows build

The port cross-compiles to a 64-bit Windows `uplink.exe` (target `x86_64-pc-windows-gnu`) and has been
run and played under Wine (GE-Proton): login screen, saved agent, game HUD, hardware/finance/email
screens, connecting to a server, fast-forwarding time, save on exit to menu and reload.

## Build

    cd uplink-rs
    tools/win/build.sh            # = cargo build -p uplink --target x86_64-pc-windows-gnu (through cargo-gate)
    # -> target/x86_64-pc-windows-gnu/debug/uplink.exe

Needs the rustup target (`rustup target add x86_64-pc-windows-gnu`) and, **instead of a MinGW-w64
install**, only what ships with rustup plus `clang`, `llvm-dlltool`, `llvm-ar`, `llvm-readobj`:

`tools/win/setup.sh` (run automatically by `build.sh`) builds a link-time toolchain in
`~/.cache/uplink-win-toolchain` (`UPLINK_WIN_TOOLCHAIN` overrides):

- `tools/win/mingw-ld` is the `x86_64-w64-mingw32-gcc` rustc calls; it translates the gcc-style link line
  for `rust-lld -flavor gnu` (MinGW mode).
- `tools/win/shim.c` (compiled freestanding with clang) replaces `crt2.o`/`libmingw32`/`libmingwex`/
  `libgcc_eh`: program entry point, the TLS directory, `round`/`trunc`/`fma`/`sincos`/... that the old
  `msvcrt.dll` lacks, and stubs for the unwinder.
- Import libraries: `kernel32`, `user32`, `opengl32`, `winmm`, ... come from the `winapi-x86_64-pc-windows-gnu`
  crate (cargo fetches it with the first Windows build); `msvcrt` and `shcore` are generated from the export
  tables of Wine's PE DLLs (`WINE_PE`, default the GE-Proton `x86_64-windows` directory).
- `x86_64-w64-mingw32-dlltool` -> `llvm-dlltool` (rustc uses it for `raw-dylib` imports).

If a real `x86_64-w64-mingw32-gcc` is installed, none of this is needed: just
`cargo build -p uplink --target x86_64-pc-windows-gnu`.

Known limits of the shim toolchain (a real MinGW build does not have them):

- No unwinder: a panic prints its message and then ends the panicking thread (the whole process, exit
  code 101, if it is the main thread). `catch_unwind` cannot catch anything. The game does not use it.
- `fma` / `fmaf` are not fused (product rounded, then added).
- Only the `debug` profile was tried; the binary is a console-subsystem exe.

## Behaviour on Windows (mirrors the C++ `#ifdef WIN32`)

- App path = folder of the running exe (`std::env::current_exe`, like `WinMain` working it out from the
  command line); user path = `<app path>users/` (agents `*.usr`, `options`, `debug.log`). No `%HOME%` / `~/.uplink`.
- Game data: `UPLINK_DATA` (e.g. `Z:\home\me\Games\uplink-gog\app` under Wine) or the `.dat` files next to the exe.
- Sound effects use macroquad's WASAPI backend; module music (`engine/src/music.rs`) uses WinMM `waveOut`.
  macroquad's WASAPI thread panics when the machine has no output device, so `sound::initialise` checks
  `waveOutGetNumDevs()` first and runs silent (no sound code is touched) when there is none.
  Audible output has not been heard (Wine here has audio drivers disabled); only the no-device path is tested.

## Trying it under Wine

    tools/win/wine-run.sh start 64 --seed    # own prefix + Xvfb :1640, Wine virtual desktop 1024x768, drivers off
    export DISPLAY=:1640                     # login: click "Tester" (85,501) then Proceed (590,568), OK (328,376)
    import -window root shot.png             # xdotool for mouse and keys
    tools/win/wine-run.sh stop 64            # ALWAYS stop it (wineserver -k + Xvfb)

Session files live in `~/.cache/uplink-wt-scratch/windows-N/` (`app/` = the exe + copied `.dat` files +
`users/`, `game.log` = stdout/stderr). The window has a title bar, so it is ~6 px wider and ~12 px taller than
the 1024x768 desktop and the bottom HUD row is slightly cropped; positions are otherwise as on Linux.
Wine uses 1-2 GB of the shared RAM cap: run one instance at a time.
