#!/bin/bash
# Builds the link-time toolchain for the Windows (x86_64-pc-windows-gnu) cross build in
# ${UPLINK_WIN_TOOLCHAIN:-~/.cache/uplink-win-toolchain}. Needs only what ships with rustup + clang/llvm:
#   clang, llvm-dlltool, llvm-ar, llvm-readobj, rust-lld; the winapi-x86_64-pc-windows-gnu import
#   libraries (fetched by cargo into ~/.cargo/registry), and a Wine x86_64-windows msvcrt.dll to
#   read export names from. See docs/WINDOWS.md.
set -eu
HERE="$(cd "$(dirname "$(readlink -f "$0")")" && pwd)"
TC="${UPLINK_WIN_TOOLCHAIN:-$HOME/.cache/uplink-win-toolchain}"
WINE_PE="${WINE_PE:-$HOME/.local/share/Steam/compatibilitytools.d/GE-Proton11-1/files/lib/wine/x86_64-windows}"
mkdir -p "$TC/lib" "$TC/bin"
# 1. the C runtime shim
clang --target=x86_64-w64-windows-gnu -ffreestanding -fno-builtin -fno-stack-protector -fno-asynchronous-unwind-tables \
      -O2 -DSHIM_EXE -c "$HERE/shim.c" -o "$TC/shim_exe.o"
clang --target=x86_64-w64-windows-gnu -ffreestanding -fno-builtin -fno-stack-protector -fno-asynchronous-unwind-tables \
      -O2 -c "$HERE/shim.c" -o "$TC/shim_dll.o"
# 2. import libraries for kernel32, user32, opengl32, ... from the winapi crate (libwinapi_X.a -> libX.a)
WINAPI=$(ls -d "$HOME"/.cargo/registry/src/*/winapi-x86_64-pc-windows-gnu-0.4.0/lib 2>/dev/null | head -1)
[ -n "$WINAPI" ] || { echo "run 'cargo fetch --target x86_64-pc-windows-gnu' first (winapi import libs)"; exit 1; }
pe_implib() { # NAME: import library from the export table of Wine's NAME.dll
  { echo "LIBRARY $1.dll"; echo EXPORTS
    llvm-readobj --coff-exports "$WINE_PE/$1.dll" | sed -n 's/^ *Name: //p' | grep -v '^[?$@]' | LC_ALL=C sort -u; } > "$TC/$1.def"
  llvm-dlltool -m i386:x86-64 -d "$TC/$1.def" -l "$TC/lib/lib$1.a"
  cp "$TC/lib/lib$1.a" "$TC/lib/libwinapi_$1.a"   # shadows the crate's linker-script stub of that name
}
for f in "$WINAPI"/libwinapi_*.a; do
  b=$(basename "$f"); n=${b#libwinapi_}
  if head -c 5 "$f" | grep -q INPUT; then rm -f "$TC/lib/lib$n"; [ -f "$WINE_PE/${n%.a}.dll" ] && pe_implib "${n%.a}"  # a linker script, not an archive
  else ln -sf "$f" "$TC/lib/lib$n"; fi
done
# 3. msvcrt.dll import library
pe_implib msvcrt
# 4. libraries rustc asks for that the shim replaces: empty archives
for l in gcc_eh gcc mingw32 mingwex moldname pthread; do rm -f "$TC/lib/lib$l.a"; llvm-ar rcs "$TC/lib/lib$l.a"; done
# 5. gcc-compatible linker driver name rustc looks for, and dlltool (rustc uses it for raw-dylib)
ln -sf "$HERE/mingw-ld" "$TC/bin/x86_64-w64-mingw32-gcc"
ln -sf "$(command -v llvm-dlltool)" "$TC/bin/x86_64-w64-mingw32-dlltool"
echo "windows toolchain ready in $TC"
