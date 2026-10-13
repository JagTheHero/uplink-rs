//! Port of source/app/globals.h
//!
//! Global defines and build flags. This port builds the FULLGAME configuration on Linux.
//! `UplinkAssert` / `UplinkAbort` / `UplinkWarning` are not ported as items: use
//! `debug_assert!` / `panic!` / `eprintln!` (PORTING.md section 4).

#![allow(unused_variables, unused_imports, unused_mut, dead_code)]

pub const VERSION_NUMBER: &str = "1.31";
pub const VERSION_NAME: &str = "RELEASE";
pub const SAVEFILE_VERSION: &str = "SAV56";

/// Alpha value used for interface.
pub const ALPHA: f32 = 0.85;
/// Alpha value for disabled buttons.
pub const ALPHA_DISABLED: f32 = 0.5;

// Build flavour (the C++ picked one of FULLGAME / DEMOGAME / TESTGAME).
/// The finished, complete game for CD distribution.
pub const FULLGAME: bool = true;
/// Limit rating, stop story progression.
pub const DEMOGAME: bool = false;
/// An internal build, with disabled start sequence.
pub const TESTGAME: bool = false;
/// This version designed for DOC labs.
pub const DOCLABRELEASE: bool = false;
/// Purity Control.
pub const WAREZRELEASE: bool = false;
/// Do the code card lookup thingy at the start.
pub const CODECARD_ENABLED: bool = false;
/// Redirect std* to users/debug.log.
pub const DEBUGLOG_ENABLED: bool = false;

/// Which code cards are accepted (all three after V1.0).
pub const CODECARD_CHRIS: bool = true;
pub const CODECARD_TOM: bool = true;
pub const CODECARD_MARK: bool = true;

/// `CHEATMODES_ENABLED`: runtime switch, set the environment variable `UPLINK_CHEATS=1`.
pub fn cheatmodes_enabled() -> bool {
    std::env::var("UPLINK_CHEATS")
        .map(|v| v == "1")
        .unwrap_or(false)
}
