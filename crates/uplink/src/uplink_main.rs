//! Port of source/uplink.cpp
//!
//! Program entry point and the start-up sequence (`Init_App` ... `Run_Game`). PORT: the original game
//! assets (`data.dat`, `graphics.dat`, ...) are found through `$UPLINK_DATA` (see
//! `uplink_engine::archive::find_data_dir`), e.g. the folder of a GOG install of Uplink.
//! The Windows `WinMain` command-line splitting is not needed. Because the macroquad window is
//! created from the options (resolution, fullscreen), the context is built and the options loaded
//! first, and everything that needs the window runs inside the macroquad future.

#![allow(unused_variables, unused_imports, unused_mut, dead_code)]

use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

use uplink_engine::{archive, sound, text};

use crate::app::app::App;
use crate::app::globals;
use crate::app::miscutils;
use crate::app::opengl::{opengl_initialise, opengl_run, timer_func};
use crate::app::opengl_interface::set_window_scale_factor;
use crate::ctx::Ctx;
use crate::game::game::Game;
use crate::game::scriptlibrary::ScriptLibrary;
use crate::mainmenu::mainmenu::{MainMenu, MAINMENU_FIRSTLOAD, MAINMENU_LOGIN};

/// Standard entry point (C++ `main` / `RunUplink`).
pub fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut g = init_app(args.first().map(String::as_str).unwrap_or("uplink"));
    init_options(&mut g, &args);
    load_data(&mut g);
    init_game(&mut g);
    init_graphics(&mut g);

    let options = &g.app.options;
    let conf = macroquad::prelude::Conf {
        window_title: g.app.title.clone(),
        window_width: options.get_option_value("graphics_screenwidth"),
        window_height: options.get_option_value("graphics_screenheight"),
        fullscreen: options.is_option_equal_to("graphics_fullscreen", 1)
            && !options.is_option_equal_to("graphics_safemode", 1),
        window_resizable: false,
        ..Default::default()
    };
    macroquad::Window::from_config(conf, async move {
        init_opengl(&mut g);
        init_fonts(&mut g);
        init_sound(&mut g);
        init_music(&mut g);
        run_main_menu(&mut g);
        run_game(&mut g).await;
    });
    // Only reached when the window is closed with the window manager's close button (App::close
    // ends the process itself): leave the same way, without the exit-time teardown that can
    // segfault against the audio threads.
    uplink_engine::exit_now(0);
}

/// `Init_App`: builds the `App` (identity strings, user path), prints the banner, mounts the archives
/// (`RsInitialise`), creates the `Ctx` (with an empty `Game`) and runs `App::initialise`.
pub fn init_app(argv0: &str) -> Ctx {
    // Set up App object

    // PORT: the Windows WinMain works the exe path out of GetCommandLine because argv[0] is unreliable
    // there (it may be a bare "uplink.exe"); the Rust equivalent is the running executable's path.
    #[cfg(windows)]
    let argv0 = &std::env::current_exe().map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|_| argv0.to_string());
    let apppath = miscutils::get_file_path(argv0);
    let mut app = App::new();
    // PORT: __DATE__ / __TIME__ have no Rust equivalent; the crate version stands in for the build stamp.
    let currenttime = env!("UPLINK_BUILD_DATE").to_string();
    app.set(&apppath, globals::VERSION_NUMBER, globals::VERSION_NAME, &currenttime, "Uplink");

    if globals::CODECARD_ENABLED {
        if globals::CODECARD_CHRIS {
            app.release.push_str("(code-C)");
        }
        if globals::CODECARD_MARK {
            app.release.push_str("(code-M)");
        }
        if globals::CODECARD_TOM {
            app.release.push_str("(code-T)");
        }
    }

    println!("=============================");
    println!("=                           =");
    println!("=        U P L I N K        =");
    println!("=                           =");
    println!("=        Version {:<10} =", app.version);

    if globals::FULLGAME {
        println!("=     - R E L E A S E -     =");
    }
    if globals::DEMOGAME {
        println!("=        - D E M O -        =");
    }
    if globals::TESTGAME {
        println!("=    - I N T E R N A L -    =");
    }

    println!("=                           =");
    println!("=============================");
    println!();
    println!("{}", app.release);

    if globals::DEBUGLOG_ENABLED {
        miscutils::make_directory(&app.userpath);
        let debugpath = format!("{}debug.log", app.userpath);

        if !freopen_append(&debugpath, 1) {
            println!("WARNING : Failed to open users/debug.log for writing stdout");
        }
        if !freopen_append(&debugpath, 2) {
            println!("WARNING : Failed to open users/debug.log for writing stderr");
        }

        // Print some basic info to the file

        let (tm_hour, tm_min, tm_mday, tm_mon, tm_year) = local_time_now();

        println!();
        println!();
        println!("===============================================");
        println!(
            "NEW GAME     {}:{}, {}/{}/{}",
            tm_hour,
            tm_min,
            tm_mday,
            tm_mon + 1,
            tm_year + 1900
        );
        println!("===============================================");

        println!("Version : {}", app.version);

        if globals::FULLGAME {
            println!("RELEASE");
        }
        if globals::DEMOGAME {
            println!("DEMO");
        }
        if globals::TESTGAME {
            println!("INTERNAL");
        }
        println!("{}", if cfg!(windows) { "Win32 Build" } else { "Linux Build" });
        println!("{}", app.release);
    }

    // RsInitialise: the data directory is set and the .dat archives are mounted (archive::init).
    // PORT: the C++ RsInitialise made a temp directory (unused by the port) and RsLoadArchive opened each file.
    // PORT: the port ships no game data, so it refuses to run without an installed copy of Uplink.
    let data_dir = archive::find_data_dir().unwrap_or_else(|| PathBuf::from(app.path.clone()));
    let required = ["data.dat", "graphics.dat", "fonts.dat"];
    if archive::init(&data_dir) < required.len() || !required.iter().all(|f| data_dir.join(f).is_file()) {
        eprintln!(
            "Uplink game data not found.\n\
             This port needs an installed copy of Uplink (for example the GOG release).\n\
             Set UPLINK_DATA to the folder that holds data.dat, graphics.dat and fonts.dat,\n\
             or run the executable from that folder."
        );
        std::process::exit(1);
    }

    let mut g = Ctx::new(app, Game::new());
    App::initialise(&mut g);
    g
}

/// `freopen(path, "a", stream)` for stdout (fd 1) or stderr (fd 2): opens `path` for appending and
/// puts it on that file descriptor, so everything written to the stream goes to the file.
/// Returns success.
#[cfg(unix)]
fn freopen_append(path: &str, fd: i32) -> bool {
    use std::os::unix::io::AsRawFd;

    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    let Ok(file) = std::fs::OpenOptions::new().create(true).append(true).open(path) else {
        return false;
    };
    // SAFETY: both descriptors are valid; dup2 only replaces `fd`, and `file` closes its own copy.
    unsafe { libc::dup2(file.as_raw_fd(), fd) != -1 }
}

#[cfg(not(unix))]
fn freopen_append(_path: &str, _fd: i32) -> bool {
    false
}

/// `localtime(time(NULL))` as (tm_hour, tm_min, tm_mday, tm_mon, tm_year); all zero if the
/// conversion fails.
#[cfg(unix)]
fn local_time_now() -> (i32, i32, i32, i32, i32) {
    // SAFETY: time(NULL) has no preconditions; localtime_r only writes into `tm`, a plain C struct
    // for which all-zero bytes are a valid value.
    let tm = unsafe {
        let timet = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&timet, &mut tm);
        tm
    };
    (tm.tm_hour, tm.tm_min, tm.tm_mday, tm.tm_mon, tm.tm_year)
}

#[cfg(not(unix))]
fn local_time_now() -> (i32, i32, i32, i32, i32) {
    (0, 0, 0, 0, 0)
}

/// `Init_Options`: command line options `+name` (on), `-name` (off), `!name value`; safe mode overrides;
/// sets the window scale factor from the screen size.
pub fn init_options(g: &mut Ctx, args: &[String]) {
    // Parse all command line options. args[0] is the program name, as in C argv.

    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        let mut chars = arg.chars();
        let type_ = chars.next();
        // sscanf(argv[i], "%c%s", &type, optionname): %s skips whitespace, then reads one word
        let optionname = chars.as_str().split_ascii_whitespace().next().unwrap_or("").to_string();

        match type_ {
            Some('+') => {
                // Turn option ON (1)
                g.app.options.set_option_value(&optionname, 1);
                if cfg!(debug_assertions) {
                    println!("Parsed command line option : {}", arg);
                }
            }
            Some('-') => {
                // Turn option OFF (0)
                g.app.options.set_option_value(&optionname, 0);
                if cfg!(debug_assertions) {
                    println!("Parsed command line option : {}", arg);
                }
            }
            Some('!') => {
                // Set option value
                i += 1;
                let Some(value_arg) = args.get(i) else {
                    panic!("Command line option !{} is missing its value", optionname);
                };
                // PORT: the C++ left newvalue uninitialised when sscanf failed; 0 is used instead.
                let newvalue = parse_leading_i32(value_arg).unwrap_or(0);
                g.app.options.set_option_value(&optionname, newvalue);
                if cfg!(debug_assertions) {
                    println!("Parsed command line option : {} {}", optionname, newvalue);
                }
            }
            _ => {
                // UplinkAbort
                panic!("Error parsing command line option : {}", arg);
            }
        }

        i += 1;
    }

    //
    // Safe mode?
    //

    if g.app.options.get_option_value("graphics_safemode") == 1 {
        g.app.options.set_option_value("graphics_fullscreen", 0);
        g.app.options.set_option_value("graphics_screenrefresh", -1);
        g.app.options.set_option_value("graphics_screendepth", -1);
        g.app.options.set_option_value("graphics_softwaremouse", 1);
    }

    println!();

    let screen_width = g.app.options.get_option_value("graphics_screenwidth");
    let screen_height = g.app.options.get_option_value("graphics_screenheight");
    set_window_scale_factor(screen_width as f32 / 640.0, screen_height as f32 / 480.0);

    if g.app.options.is_option_equal_to("game_debugstart", 1) {
        println!("=====DEBUGGING INFORMATION ENABLED=====");
    }
}

/// `UplinkAssert` as it is active in the release build: prints the failure, dumps the core and exits with 255.
fn assertion_failed(app: &App, condition: &str) -> ! {
    println!("\nAn Uplink Assertion Failure has occured");
    println!("=======================================");
    println!(" Condition : {}", condition);
    println!(" Location  : {}, line {}", file!(), line!());
    app.core_dump();
    std::process::exit(255);
}

/// Parses a command line value like C `sscanf("%d")`: leading whitespace, optional sign, then digits.
fn parse_leading_i32(s: &str) -> Option<i32> {
    let t = s.trim_start();
    let end = t
        .char_indices()
        .find(|&(i, c)| !(c.is_ascii_digit() || (i == 0 && (c == '+' || c == '-'))))
        .map(|(i, _)| i)
        .unwrap_or(t.len());
    t[..end].parse::<i32>().ok()
}

/// `Load_Data`: mounts the `.dat` archives; FULLGAME checks `world.dat` (CRC bytes at offset 128, size 14400792).
pub fn load_data(g: &mut Ctx) {
    let debugging = g.app.options.is_option_equal_to("game_debugstart", 1);

    if debugging {
        println!("Loading application data");
    }

    // PORT: init_app mounted the archives (archive::init), so only the RsLoadArchive reports are printed here.
    let data_dir = archive::data_dir().unwrap_or_else(|| PathBuf::from(g.app.path.clone()));
    for name in [
        "data.dat",
        "graphics.dat",
        "loading.dat",
        "sounds.dat",
        "music.dat",
        "fonts.dat",
        "patch.dat",
    ] {
        if data_dir.join(name).is_file() {
            println!("Successfully loaded data archive {}", name);
        }
    }

    if globals::FULLGAME {
        // Generate the file CRC

        let realbuffer: [u8; 8] = [0xe7, 0x6b, 0x7e, 0x6b, 0x4c, 0x4f, 0x57, 0x7d];

        // Ensure the world.dat file is in place and has the right CRC
        // PORT: looked up in the data directory (next to the .dat archives), not app->path.

        let worlddatfilename = data_dir.join("world.dat");
        let mut file = std::fs::File::open(&worlddatfilename)
            .unwrap_or_else(|_| assertion_failed(&g.app, "file"));

        let mut readbuffer = [0u8; 8];
        let read_ok = file.seek(SeekFrom::Start(128)).is_ok() && file.read_exact(&mut readbuffer).is_ok();
        if !(read_ok && readbuffer == realbuffer) {
            assertion_failed(&g.app, "match == 0");
        }

        // Ensure the filesize is correct

        let size_ok = file.metadata().map(|m| m.len() == 14_400_792).unwrap_or(false);
        if !size_ok {
            assertion_failed(&g.app, "match2");
        }
    }

    if debugging {
        println!("Finished loading application data");
    }
}

/// `Init_Game`: the C++ seeded `rand()` and created the `Game`; the `Game` already exists in the `Ctx`
/// and the RNG is self-seeding, so only the debug message remains.
pub fn init_game(g: &mut Ctx) {
    if g.app.options.is_option_equal_to("game_debugstart", 1) {
        println!("Init_Game called...game object already created");
    }
}

/// `Init_Sound`
pub fn init_sound(_g: &mut Ctx) {
    sound::initialise();
}

/// `Init_Music`: the "main" playlist (the demo build has only the first three songs).
pub fn init_music(_g: &mut Ctx) {
    sound::playlist_initialise();
    sound::set_mod_volume(20);
    sound::playlist_create("main");
    let songs: &[&str] = if globals::DEMOGAME {
        &["bluevalley", "serenity", "mystique"]
    } else {
        &[
            "bluevalley",
            "serenity",
            "mystique",
            "a94final",
            "symphonic",
            "myst2",
        ]
    };
    for song in songs {
        sound::playlist_add_song("main", &format!("music/{song}.uni"));
    }
}

/// `Init_Graphics`: (re)loads the colours of the current theme.
pub fn init_graphics(g: &mut Ctx) {
    let theme = g.app.options.get_theme_name().to_string();
    g.app.options.set_theme_name(&theme);
}

/// `Init_Fonts`: loads the Dungeon true-type font as HELVETICA_10 / 12 / 18 (sizes 9, 11, 16) and sets the default font.
pub fn init_fonts(g: &mut Ctx) {
    let debugging = g.app.options.is_option_equal_to("game_debugstart", 1);

    if debugging {
        println!("Init_Fonts called...setting up system fonts");
    }

    // Set up Gucci graphics library

    text::enable_truetype_support();

    if debugging {
        print!("Registering fonts...");
    }
    // PORT: GciRegisterTrueTypeFont / GciUnregisterTrueTypeFont have no engine equivalent;
    // load_truetype_font reads "fonts/dungeon.ttf" straight from the archive.
    if debugging {
        print!("done\n ");
    }

    if debugging {
        println!("Loading system fonts into memory...");
    }

    // C++ `&` on bools: all three loads are attempted
    let success = text::load_truetype_font(text::HELVETICA_10, "Dungeon", "fonts/dungeon.ttf", 9)
        & text::load_truetype_font(text::HELVETICA_12, "Dungeon", "fonts/dungeon.ttf", 11)
        & text::load_truetype_font(text::HELVETICA_18, "Dungeon", "fonts/dungeon.ttf", 16);

    if debugging {
        println!("done");
    }

    if debugging {
        print!("Unregistering fonts...");
    }
    if debugging {
        print!("done\n ");
    }

    if !success {
        text::disable_truetype_support();
        println!("True type font support is DISABLED");
    }

    text::set_default_font(text::HELVETICA_10);

    if debugging {
        print!("Finished with Init_Fonts\n ");
    }
}

/// `Init_OpenGL`
pub fn init_opengl(g: &mut Ctx) {
    opengl_initialise(g);
}

/// `Run_MainMenu`: patch detection (`game_version`), first-time installation or the login screen.
pub fn run_main_menu(g: &mut Ctx) {
    if g.app.options.is_option_equal_to("game_debugstart", 1) {
        println!("Creating main menu.");
        println!("====== END OF DEBUGGING INFORMATION ====");
    }

    // Set up the game (mainmenu if not first time, or straight into the game );

    // sscanf(VERSION_NUMBER, "%f") then *= 100, in single precision like the C++
    let current_version: f32 = globals::VERSION_NUMBER.parse().unwrap_or(0.0);
    let current_version = current_version * 100.0;

    let old_version = g.app.options.get_option_value("game_version");

    if current_version as i32 != old_version {
        // A new patch has been installed
        println!("New patch Detected!");
        println!("Old version = {}", old_version);
        println!("New version = {}", current_version as i32);
        println!();

        g.app.options.set_option_value_full("game_version", current_version as i32, "z", false, false);

        if old_version < 120 {
            // Ugrading the game from a version before Nakatomi
            MainMenu::run_screen(g, MAINMENU_FIRSTLOAD);
            ScriptLibrary::run_script(g, 45);
        } else {
            MainMenu::run_screen(g, MAINMENU_LOGIN);
        }
    } else if g.app.options.is_option_equal_to("game_firsttime", 1) {
        // This is a brand new installation

        MainMenu::run_screen(g, MAINMENU_FIRSTLOAD);
        timer_func(2000, ScriptLibrary::run_script, 30);
    } else {
        // Normal startup

        MainMenu::run_screen(g, MAINMENU_LOGIN);
    }
}

/// `Run_Game`: runs the main loop until the app closes.
pub async fn run_game(g: &mut Ctx) {
    opengl_run(g).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_time_is_a_plausible_calendar_time() {
        let (hour, min, mday, mon, year) = local_time_now();
        assert!((0..24).contains(&hour) && (0..60).contains(&min));
        assert!((1..=31).contains(&mday) && (0..12).contains(&mon));
        assert!(year + 1900 >= 2024);
    }

    use crate::testutil::bare_ctx;

    fn args(list: &[&str]) -> Vec<String> {
        std::iter::once("uplink").chain(list.iter().copied()).map(String::from).collect()
    }

    fn ctx_with_defaults() -> Ctx {
        let mut g = bare_ctx();
        g.app.options.create_default_options();
        g
    }

    #[test]
    fn leading_integers_are_scanned_like_sscanf() {
        assert_eq!(parse_leading_i32("1024"), Some(1024));
        assert_eq!(parse_leading_i32("  -5px"), Some(-5));
        assert_eq!(parse_leading_i32("+7"), Some(7));
        assert_eq!(parse_leading_i32("12 34"), Some(12));
        assert_eq!(parse_leading_i32("x1"), None);
        assert_eq!(parse_leading_i32(""), None);
        assert_eq!(parse_leading_i32("-"), None);
    }

    #[test]
    fn command_line_options_turn_on_off_and_set() {
        let mut g = ctx_with_defaults();
        init_options(
            &mut g,
            &args(&["+graphics_softwaremouse", "-graphics_fullscreen", "!graphics_screenwidth", "1024", "!graphics_screenheight", "768"]),
        );
        let o = &g.app.options;
        assert_eq!(o.get_option_value("graphics_softwaremouse"), 1);
        assert_eq!(o.get_option_value("graphics_fullscreen"), 0);
        assert_eq!(o.get_option_value("graphics_screenwidth"), 1024);
        assert_eq!(o.get_option_value("graphics_screenheight"), 768);
    }

    #[test]
    fn unknown_command_line_options_are_ignored() {
        let mut g = ctx_with_defaults();
        init_options(&mut g, &args(&["+no_such_option"]));
        assert!(g.app.options.get_option("no_such_option").is_none());
    }

    #[test]
    fn safe_mode_overrides_the_graphics_options() {
        let mut g = ctx_with_defaults();
        init_options(&mut g, &args(&["+graphics_safemode"]));
        let o = &g.app.options;
        assert_eq!(o.get_option_value("graphics_fullscreen"), 0);
        assert_eq!(o.get_option_value("graphics_screenrefresh"), -1);
        assert_eq!(o.get_option_value("graphics_screendepth"), -1);
        assert_eq!(o.get_option_value("graphics_softwaremouse"), 1);
    }

    #[test]
    #[should_panic(expected = "Error parsing command line option")]
    fn an_argument_without_a_prefix_aborts() {
        let mut g = ctx_with_defaults();
        init_options(&mut g, &args(&["fullscreen"]));
    }

    #[test]
    #[should_panic(expected = "missing its value")]
    fn a_value_option_without_a_value_aborts() {
        let mut g = ctx_with_defaults();
        init_options(&mut g, &args(&["!graphics_screenwidth"]));
    }
}
