//! Port of source/app/app.h + source/app/app.cpp
//!
//! App object: stores data about the app (path, version, ...) and owns the options,
//! network and main menu. Lives at `g.app`. PORT: the C++ pointers `options`, `network`
//! and `mainmenu` are plain fields, so the `GetOptions()` etc. accessors are not ported.
//!
//! Game assets: the engine finds the original game data through `$UPLINK_DATA` (see
//! `uplink_engine::archive::find_data_dir`), e.g. a GOG install of Uplink.

#![allow(unused_variables, unused_imports, unused_mut, dead_code)]

use std::path::Path;

use uplink_engine::{redshirt, sound, text};

use crate::app::globals::TESTGAME;
use crate::app::miscutils::make_directory;
use crate::ctx::Ctx;
use crate::game::data::data::IP_UPLINKPUBLICACCESSSERVER;
use crate::game::game::{Game, GAMESPEED_GAMEOVER, GAMESPEED_PAUSED};
use crate::interface::localinterface::localinterface::LocalInterface;
use crate::interface::remoteinterface::remoteinterface::RemoteInterface;
use crate::mainmenu::mainmenu::{
    MainMenu, MAINMENU_CONNECTIONLOST, MAINMENU_DEMOGAMEOVER, MAINMENU_DISAVOWED, MAINMENU_LOGIN,
    MAINMENU_OBITUARY, MAINMENU_REVELATIONLOST, MAINMENU_REVELATIONWON, MAINMENU_WAREZGAMEOVER,
};
use crate::network::network::{Network, NETWORK_CLIENT, NETWORK_SERVER};
use crate::options::options::Options;
use crate::vanbakel::Vanbakel;
use crate::world::computer::gateway::Gateway;
use crate::world::connection::Connection;
use crate::world::generator::worldgenerator::WorldGenerator;

pub struct App {
    pub path: String,
    pub userpath: String,
    pub version: String,
    pub type_: String,
    pub date: String,
    pub title: String,
    pub release: String,
    pub starttime: i32,
    pub closed: bool,
    pub options: Options,
    pub network: Network,
    pub mainmenu: MainMenu,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

/// `app->GetOptions()->GetOptionValue("graphics_screenwidth")` and `graphics_screenheight`.
fn screen_size(g: &Ctx) -> (i32, i32) {
    (
        g.app.options.get_option_value("graphics_screenwidth"),
        g.app.options.get_option_value("graphics_screenheight"),
    )
}

impl App {
    pub fn new() -> Self {
        App {
            path: "c:/".to_string(),
            userpath: "c:/".to_string(),
            version: "1.0".to_string(),
            type_: "RELEASE".to_string(),
            date: "01/01/97".to_string(),
            title: "NewApp".to_string(),
            release: "Version 1.0 (RELEASE), Compiled on 01/01/97".to_string(),
            starttime: 0,
            closed: false,
            options: Options::new(),
            network: Network::new(),
            mainmenu: MainMenu::new(),
        }
    }

    /// Sets the identity strings and the user path (`~/.uplink/`, or `<path>users/` without `$HOME`; on Windows always `<path>users/`).
    pub fn set(
        &mut self,
        newpath: &str,
        newversion: &str,
        newtype: &str,
        newdate: &str,
        newtitle: &str,
    ) {
        self.path = newpath.to_string();
        self.version = newversion.to_string();
        self.type_ = newtype.to_string();
        self.date = newdate.to_string();
        self.title = newtitle.to_string();
        self.release = format!(
            "Version {} ({})\nCompiled on {}\n",
            self.version, self.type_, self.date
        );
        // Under Windows the user path is `<app path>users/` (WIN32 branch of App::Set).
        #[cfg(windows)]
        {
            self.userpath = format!("{}users/", self.path);
        }
        #[cfg(not(windows))]
        {
            self.userpath = match std::env::var("HOME") {
                Ok(home) => format!("{}/.uplink/", home),
                Err(_) => format!("{}users/", self.path),
            };
        }
    }

    /// Creates fresh options (loaded from `<userpath>options`, defaults filled in), network and main menu.
    pub fn initialise(g: &mut Ctx) {
        g.app.options = Options::new();
        g.app.options.load(&g.app.userpath);
        g.app.options.create_default_options();

        g.app.starttime = g.ecl.get_accurate_time();

        g.app.network = Network::new();
        g.app.mainmenu = MainMenu::new();
    }

    /// Shuts down the app: resets Eclipse, exits the game, saves options, shuts down sound / fonts / archive.
    /// PORT: ends the process with exit code 1 like the C++ `exit(1)` (via `uplink_engine::exit_now`).
    pub fn close(g: &mut Ctx) {
        g.app.closed = true;

        let (width, height) = screen_size(g);
        g.ecl.reset(width, height);

        Game::exit_game(g);

        g.app.options.apply_shutdown_changes();
        g.app.options.save(&g.app.userpath);

        sound::shutdown();
        Vanbakel::reset(g);
        text::delete_all_truetype_fonts();

        // PORT: the C++ RsCleanUp() and the deletes of mainmenu / options / network / game are not
        // needed: the process exits below and Rust frees memory on drop, which exit() skips anyway.
        // The exit code stays 1 like the C++; `exit_now` skips the libc / thread-local teardown that
        // races the audio threads (see its doc comment).
        uplink_engine::exit_now(1);
    }

    /// True if in the process of shutting down.
    pub fn closed(&self) -> bool {
        self.closed
    }

    /// Loads `<userpath><username>.usr` (JSON `GameSave`), then sets up the interface:
    /// new gateway / nuked gateway / welcome back, or the obituary for a game-over game.
    pub fn load_game(g: &mut Ctx, username: &str) {
        // Try to load from the local dir
        let filename = format!("{}{}.usr", g.app.userpath, username);

        print!("Loading profile from {}...", filename);

        let bytes = match redshirt::read_file(Path::new(&filename)) {
            Ok(bytes) => bytes,
            Err(_) => {
                println!("failed");
                println!("App::LoadGame, Failed to load user profile");

                MainMenu::run_screen(g, MAINMENU_LOGIN);

                return;
            }
        };

        MainMenu::remove(g);

        // The save is JSON text; a file that is not valid UTF-8 fails in Game::load_game like any bad file.
        let json = String::from_utf8_lossy(&bytes).into_owned();
        let success = Game::load_game(g, &json);
        if !success {
            println!("failed");
            println!("App::LoadGame, Failed to load user profile");
            return;
        }

        println!("success");

        if g.game.game_speed() != GAMESPEED_GAMEOVER {
            WorldGenerator::load_dynamics(g);

            // PORT: the C++ Connection stores the owner's person name, which Connection::* take here.
            let owner = g.player().name.clone();

            Connection::disconnect(g, &owner);
            Connection::reset(g, &owner);

            if g.player().gateway.newtype != -1 {
                // The player has upgraded his gateway and now wishes to
                // log into the new system
                Gateway::exchange_gateway_complete(g);

                // "Welcome to your new gateway"
                RemoteInterface::run_new_location(g);
                RemoteInterface::run_screen(g, 10);
            } else if g.player().gateway.nuked {
                // The player has nuked his gateway, so set him up
                // with a new one
                g.player_mut().gateway.nuked = false;

                g.player_mut()
                    .get_connection_mut()
                    .add_vlocation(IP_UPLINKPUBLICACCESSSERVER);
                Connection::connect(g, &owner);

                LocalInterface::remove(g);
                let (width, height) = screen_size(g);
                g.ecl.reset(width, height);
                RemoteInterface::run_new_location(g);
                RemoteInterface::run_screen(g, 10);
            } else {
                // "Welcome back to your gateway"
                RemoteInterface::run_new_location(g);
                RemoteInterface::run_screen(g, 3);
            }
        } else {
            // This is a Game Over game
            g.game.set_game_speed(GAMESPEED_PAUSED);
            let (width, height) = screen_size(g);
            g.ecl.reset(width, height);
            MainMenu::run_screen(g, MAINMENU_OBITUARY);
        }
    }

    /// Writes `<userpath><username>.usr`; "NEWAGENT" is never saved.
    pub fn save_game(g: &Ctx, username: &str) {
        if username == "NEWAGENT" {
            return;
        }

        // Try to save to the local dir
        make_directory(&g.app.userpath);

        let filename = format!("{}{}.usr", g.app.userpath, username);

        print!("Saving profile to {}...", filename);

        // The C++ opened the file (emptying it) before Game::Save, which writes nothing for a paused
        // game, so an empty string is written in that case too.
        let contents = g.game.to_json().unwrap_or_default();

        match redshirt::write_file(Path::new(&filename), contents.as_bytes(), !TESTGAME) {
            Ok(()) => println!("success"),
            Err(_) => {
                println!("failed");
                println!("App::SaveGame, Failed to save user profile");
            }
        }
    }

    /// C++ `App::Print`, with the `Print()`s it reaches (Game, Interface, MainMenu, Options,
    /// Network) inlined. PORT: per-object `Print()` is replaced by `Debug` (PORTING.md §6), so the
    /// view, world, options and network server are dumped with `{:#?}`.
    pub fn print(g: &Ctx) {
        println!("============== A P P =======================================");

        println!("============== G A M E =====================================");
        println!("Game speed = {}", g.game.gamespeed);
        match g.game.ui.as_deref() {
            Some(ui) => {
                println!("============== I N T E R F A C E ===========================");
                println!(
                    "RemoteInterface : CurrentScreenIndex = {}\n, PreviousScreenIndex = {}\n, Security_Name = {}, Security_Level = {}",
                    ui.ri.currentscreenindex, ui.ri.previousscreenindex, ui.ri.security_name, ui.ri.security_level
                );
                println!(
                    "Local Interface : Current Screen Code = {}, Screen index = {}",
                    ui.li.currentscreencode, ui.li.screenindex
                );
                println!("Task Manager ");
                println!("============== E N D  O F  I N T E R F A C E ===============");
            }
            None => println!("UI is NULL"),
        }
        match g.game.view.as_deref() {
            Some(view) => println!("{view:#?}"),
            None => println!("View is NULL"),
        }
        match g.game.world.as_deref() {
            Some(world) => println!("{world:#?}"),
            None => println!("World is NULL"),
        }
        println!("============== E N D  O F  G A M E =========================");

        // MainMenu::Print prints nothing

        println!("============== O P T I O N S ===============================");
        for (name, option) in g.app.options.options.iter() {
            println!("Index = {name}");
            println!("{option:#?}");
        }
        println!("============================================================");

        println!("============== N E T W O R K ===============================");
        let network = &g.app.network;
        println!("Status:{}", network.status);
        if network.status == NETWORK_SERVER {
            println!("{:#?}", network.server);
        } else if network.status == NETWORK_CLIENT {
            network.client.print();
        }
        println!("============== E N D  O F  N E T W O R K ===================");

        println!("============== E N D  O F  A P P ===========================");
    }

    pub fn core_dump(&self) {
        println!("============== B E G I N  C O R E  D U M P =================");
        println!("{}", std::backtrace::Backtrace::force_capture());
        println!("============== E N D  C O R E  D U M P =====================");
    }

    /// Ends a finished game (shows the right main menu screen), updates game / main menu / network.
    pub fn update(g: &mut Ctx) {
        if g.game.game_speed() == GAMESPEED_GAMEOVER
            || (g.game.is_running() && g.player().gateway.nuked)
        {
            // The game has ended and now needs shutting down
            let handle = g.player().agent().handle.clone();
            App::save_game(g, &handle);
            g.game.set_game_speed(GAMESPEED_PAUSED);

            let (width, height) = screen_size(g);
            g.ecl.reset(width, height);

            if g.player().gateway.nuked {
                MainMenu::run_screen(g, MAINMENU_CONNECTIONLOST);
            } else if g.game.get_obituary().demogameover {
                MainMenu::run_screen(g, MAINMENU_DEMOGAMEOVER);
            } else if g.game.get_obituary().warezgameover {
                MainMenu::run_screen(g, MAINMENU_WAREZGAMEOVER);
            } else if g.world().plotgenerator.revelation_releaseuncontrolled {
                MainMenu::run_screen(g, MAINMENU_REVELATIONWON);
            } else if g.world().plotgenerator.revelation_releasefailed {
                MainMenu::run_screen(g, MAINMENU_REVELATIONLOST);
            } else {
                MainMenu::run_screen(g, MAINMENU_DISAVOWED);
            }
        }

        if g.game.is_running() {
            Game::update(g);
        }

        if g.app.mainmenu.is_visible() {
            MainMenu::update(g);
        }

        if g.app.network.is_active() {
            Network::update(g);
        }
    }
}
