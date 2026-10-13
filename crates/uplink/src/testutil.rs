//! Helpers shared by the unit tests. Nothing here needs a window.

use std::path::PathBuf;

use uplink_engine::archive;

use crate::app::app::App;
use crate::ctx::Ctx;
use crate::game::game::Game;

/// A context with an empty `Game` and default `App` (no window, no game data).
pub fn bare_ctx() -> Ctx {
    Ctx::new(App::new(), Game::new())
}

/// Mounts the original game's `.dat` archives for this test thread (the archive is thread-local).
/// Looks where the game itself looks (`$UPLINK_DATA`, ...), then in `~/Games/uplink-gog/app`.
/// Returns false when no copy of the game is around, so the caller can skip.
pub fn mount_game_data() -> bool {
    let dir = archive::find_data_dir().or_else(|| {
        let d = PathBuf::from(std::env::var("HOME").ok()?).join("Games/uplink-gog/app");
        d.join("data.dat").is_file().then_some(d)
    });
    match dir {
        Some(dir) => archive::init(&dir) > 0,
        None => {
            eprintln!("skipped: the original game's .dat files were not found (set UPLINK_DATA)");
            false
        }
    }
}

/// A fresh empty directory for one test, as a path ending in `/` (like `App::userpath`).
pub fn scratch_dir(tag: &str) -> String {
    let dir = std::env::temp_dir().join(format!("uplink-test-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    format!("{}/", dir.display())
}
