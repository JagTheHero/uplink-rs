#!/bin/bash
# Cross-build the game for Windows: tools/win/build.sh [cargo build args]  (default: -p uplink)
# Output: rust/target/x86_64-pc-windows-gnu/<profile>/uplink.exe
set -eu
HERE="$(cd "$(dirname "$(readlink -f "$0")")" && pwd)"
export UPLINK_WIN_TOOLCHAIN="${UPLINK_WIN_TOOLCHAIN:-$HOME/.cache/uplink-win-toolchain}"
[ -x "$UPLINK_WIN_TOOLCHAIN/bin/x86_64-w64-mingw32-gcc" ] || "$HERE/setup.sh"
export PATH="$UPLINK_WIN_TOOLCHAIN/bin:$PATH"
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc
# cargo does not know the shim is a link input: relink when setup.sh rebuilt it after the last exe
EXE="$HERE/../../target/x86_64-pc-windows-gnu/debug/uplink.exe"
[ ! -e "$EXE" ] || [ "$UPLINK_WIN_TOOLCHAIN/shim_exe.o" -ot "$EXE" ] || touch "$HERE/../../crates/uplink/src/main.rs"
[ $# -gt 0 ] || set -- -p uplink
exec "$HERE/../cargo-gate" build --target x86_64-pc-windows-gnu "$@"
