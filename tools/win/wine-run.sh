#!/bin/bash
# Run the cross-built Windows uplink.exe under Wine on its own Xvfb display (play-testing aid).
#   tools/win/wine-run.sh start N [--seed|--fresh]   N = session number (X display :1N0)
#   tools/win/wine-run.sh stop N                     stop the game, wineserver and Xvfb
# Sessions live in ~/.cache/uplink-wt-scratch/windows-N/ ; drive them with xdotool/import using
# DISPLAY=:1N0 (or `DISPLAY=:1N0 tools/ul click|key|shot ...` is NOT usable: ul works on its own state).
# --seed copies the saved agent "Tester"/"secret" + 1024x768 options into the Windows user dir (<exe dir>/users/).
set -u
ROOT="$(cd "$(dirname "$(readlink -f "$0")")/../.." && pwd)"
EXE="$ROOT/target/x86_64-pc-windows-gnu/debug/uplink.exe"
cmd=${1:-}; n=${2:-}
[ -z "$cmd" ] || [ -z "$n" ] && { sed -n '2,8p' "$0"; exit 2; }
D="$HOME/.cache/uplink-wt-scratch/windows-$n"
export DISPLAY=":1${n}0"
W="$HOME/.local/share/Steam/compatibilitytools.d/GE-Proton11-1/files/bin"
[ -n "${UPLINK_NOSOUND+x}" ] && export UPLINK_NOSOUND=1   # set UPLINK_NOSOUND=1 to skip the sound code entirely
export WINEPREFIX="$D/prefix" WINEDEBUG=${WINEDEBUG:--all}
case $cmd in
start)
  "$0" stop "$n" >/dev/null 2>&1
  data="${UPLINK_DATA_SRC:-$HOME/Games/uplink-gog/app}"
  mkdir -p "$D/app"
  cp -a --reflink=auto "$data"/*.dat "$D/app/"
  cp -f "$EXE" "$D/app/uplink.exe"
  [ "${3:-}" = "--fresh" ] && rm -rf "$D/app/users"
  if [ "${3:-}" = "--seed" ]; then
    rm -rf "$D/app/users"; mkdir -p "$D/app/users"
    cp -a "$HOME/.cache/uplink-ul/seed/home/.uplink/." "$D/app/users/"
  fi
  Xvfb "$DISPLAY" -screen 0 1024x768x24 >/dev/null 2>&1 &
  echo $! > "$D/xvfb.pid"; sleep 1
  printf 'pcm.!default { type null }\nctl.!default { type hw card 0 }\n' > "$D/asound.conf"
  if [ ! -d "$D/prefix" ]; then
    # virtual desktop: without a window manager plain wine windows never get input focus
    "$W/wine" reg add 'HKCU\Software\Wine\Explorer' /v Desktop /d Default /f >/dev/null 2>&1
    "$W/wine" reg add 'HKCU\Software\Wine\Explorer\Desktops' /v Default /d 1024x768 /f >/dev/null 2>&1
    "$W/wineserver" -w
  fi
  ( cd "$D/app" && WINEDLLOVERRIDES="mscoree,mshtml=;winepulse.drv,winealsa.drv,wineoss.drv=" \
      PULSE_SERVER=unix:/nonexistent PIPEWIRE_REMOTE=/nonexistent ALSA_CONFIG_PATH="$D/asound.conf" \
      LIBGL_ALWAYS_SOFTWARE=1 LP_NUM_THREADS=2 UPLINK_DEBUG=1 RUST_BACKTRACE=1 \
      UPLINK_DATA='Z:'"${D//\//\\}"'\app' exec "$W/wine" uplink.exe > "$D/game.log" 2>&1 ) &
  echo $! > "$D/game.pid"
  sleep 15
  kill -0 "$(cat "$D/game.pid")" 2>/dev/null && echo "windows session $n running on $DISPLAY" || { echo "exited:"; tail -20 "$D/game.log"; exit 1; }
  ;;
stop)
  kill "$(cat "$D/game.pid" 2>/dev/null)" 2>/dev/null
  [ -d "$D/prefix" ] && "$W/wineserver" -k 2>/dev/null
  sleep 0.5
  kill "$(cat "$D/xvfb.pid" 2>/dev/null)" 2>/dev/null
  echo stopped
  ;;
*) sed -n '2,8p' "$0"; exit 2 ;;
esac
