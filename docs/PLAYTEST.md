# Play-testing the Rust port against the original

The port must look and behave like the original game. You can run both headless, drive them with the
mouse/keyboard from the shell, take screenshots, and compare.

## The harness: `rust/tools/ul`

Use the `ul` of YOUR checkout/worktree (it runs `rust/target/debug/uplink` next to it). Build first:

    cd <checkout>/rust && tools/cargo-gate build -p uplink 2>&1 | grep -E '^error' -A5

Commands (`N` = your session number, each one gets its own virtual X display `:1N0`):

    ul start N --seed     port, logged-out, with a saved agent "Tester" (password "secret") at 1024x768
    ul ref N --seed       the ORIGINAL game (GOG 1.6 under Wine), same saved agent, same resolution
    ul login N            log in as Tester (works on both) -> "Welcome back to your Gateway" -> click OK (≈ 326,360)
    ul click N X Y | rclick | move | drag N X1 Y1 X2 Y2 | type N "text" | key N Return Escape F1 ...
    ul shot N [file]      screenshot (1024x768 PNG); look at it with the Read tool
    ul buttons N          (port only) dump every Eclipse button: name, rectangle, caption, tooltip
    ul log N [lines]      the game's stdout/stderr — panics and "UplinkAssert" failures land here
    ul alive N            is it still running?   ul stop N   stop it (ALWAYS stop sessions you started)

Tips
- Coordinates are screenshot pixels. `ul buttons N` gives exact rectangles on the port; use them.
- Compare the same screen on both: `magick A.png B.png +append -resize 50% cmp.png`, then Read cmp.png.
  Crop to the interesting area for detail (`magick a.png -crop 400x300+0+0 +repage out.png`).
- The original is GOG 1.6, the source is 1.31: small differences (e.g. a "Retire" button) are version
  differences, not port bugs. When in doubt, the C++ in `source/` decides.
- Game speed buttons are in the HUD top bar (pause / play / fast / faster). Time-based things
  (traces, events, payments) need the clock to run.
- Cheats: start the port with `UPLINK_CHEATS=1 ul start N --seed`; then F1 (or the cheat screen)
  gives money, software, rating, etc. Use only to reach content quickly, then test it normally.
- Wine is slow to start (~15 s) and uses ~1 GB RAM: start the reference only when you need to compare,
  and stop it again. RAM is shared and capped; never run more than one port + one reference.
- Sound is off in these sessions (UPLINK_NOSOUND); check sound calls by reading code, not by ear.

## Fixing what you find

- A panic / assert: read the backtrace in `ul log`, find the Rust code, compare with the C++ in
  `source/`, fix the port faithfully (rules: `rust/docs/PORTING.md`). Add a unit test where cheap.
- Wrong behaviour or looks: compare with the reference and the C++; fix the Rust to match the C++.
- Engine (`crates/engine`: GL emulation, text, images, sound) is shared by everything: do not change
  it; report the problem precisely (what, where, screenshot path) instead.
- Rebuild, re-test the exact steps, then commit in your worktree with a clear message.
