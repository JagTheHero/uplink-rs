//! Port of source/game/game.h + source/game/game.cpp
//!
//! Top level glue object of Uplink. Owns the World, View and Interface (`g.game`).
//! PORT: saving is serde-based; `GameSave` is the whole `.usr` file. The interface is not
//! saved except for the few screen fields the C++ `Interface::Save` wrote (`InterfaceSave`);
//! the screens are rebuilt from them on load, as the C++ `Load` functions did.

#![allow(unused_variables, unused_imports, unused_mut, dead_code)]

use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use uplink_engine::sound;

use crate::app::app::App;
use crate::app::globals::SAVEFILE_VERSION;
use crate::app::opengl::opengl_close;
use crate::app::opengl_interface::create_msgbox;
use crate::ctx::Ctx;
use crate::game::data::data::GAME_START_DATE;
use crate::game::gameobituary::GameObituary;
use crate::interface::interface::Interface;
use crate::interface::localinterface::localinterface::{LocalInterface, SCREEN_LAN, SCREEN_NONE};
use crate::interface::remoteinterface::remoteinterface::RemoteInterface;
use crate::interface::taskmanager::taskmanager::TaskManager;
use crate::mainmenu::mainmenu::MainMenu;
use crate::mainmenu::mainmenu::MAINMENU_LOGIN;
use crate::view::view::View;
use crate::world::date::Date;
use crate::world::generator::numbergenerator::NumberGenerator;
use crate::world::generator::plotgenerator::PlotGenerator;
use crate::world::generator::worldgenerator::WorldGenerator;
use crate::world::scheduler::notificationevent::NotificationEvent;
use crate::world::world::World;

pub const GAMESPEED_GAMEOVER: i32 = -1;
pub const GAMESPEED_PAUSED: i32 = 0;
pub const GAMESPEED_NORMAL: i32 = 1;
pub const GAMESPEED_FAST: i32 = 2;
pub const GAMESPEED_MEGAFAST: i32 = 3;
/// Only in DEBUG mode.
pub const GAMESPEED_OHMYGOD: i32 = 4;

/// Contents of a `.usr` save file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct GameSave {
    /// `globals::SAVEFILE_VERSION` when written.
    pub version: String,
    pub gamespeed: i32,
    pub world: Option<World>,
    pub view: Option<View>,
    /// What the C++ `Interface::Save` wrote (not for a game-over game).
    pub interface: Option<InterfaceSave>,
    /// Only for a game-over game.
    pub gob: Option<GameObituary>,
}

/// The fields the C++ `RemoteInterface::Save` and `LocalInterface::Save` wrote.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct InterfaceSave {
    pub previousscreenindex: i32,
    pub currentscreenindex: i32,
    pub security_name: String,
    pub security_level: i32,
    pub currentscreencode: i32,
    pub screenindex: i32,
}

/// Borrowing twin of [`GameSave`] used for writing, so saving does not clone the world.
#[derive(Serialize)]
struct GameSaveRef<'a> {
    version: &'a str,
    gamespeed: i32,
    world: Option<&'a World>,
    view: Option<&'a View>,
    interface: Option<InterfaceSave>,
    gob: Option<&'a GameObituary>,
}

#[derive(Default)]
pub struct Game {
    pub ui: Option<Box<Interface>>,
    pub view: Option<Box<View>>,
    pub world: Option<Box<World>>,
    pub gamespeed: i32,
    /// Only used if this is a dead person.
    pub gob: Option<Box<GameObituary>>,
    /// Used in autosaving (unix seconds).
    pub lastsave: i64,
}

/// `time(NULL)`: seconds since the Unix epoch.
fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

impl Game {
    pub fn new() -> Self {
        Game::default()
    }

    /// Generates a world, view and interface and runs until (at least) the game start date.
    pub fn new_game(g: &mut Ctx) {
        // Delete the old game (the C++ deleted ui, view, world and gob first)
        g.game.ui = None;
        g.game.view = None;
        g.game.world = None;
        g.game.gob = None;

        // Initialise the world
        let world = World::new(g);
        g.game.world = Some(Box::new(world));
        NotificationEvent::schedule_starting_events(g);
        WorldGenerator::generate_all(g);
        PlotGenerator::initialise(g);
        g.world_mut().demoplotgenerator.initialise();

        sound::play_sound("sounds/ringout.wav", None, false);

        // Initialise the view
        let mut view = View::new();
        view.initialise(&mut g.ecl);
        g.game.view = Some(Box::new(view));

        // Initialise the interface
        g.game.ui = Some(Box::new(Interface::new()));
        TaskManager::reset(g);
        Interface::create(g);

        // Start the game running
        g.game.set_game_speed(GAMESPEED_NORMAL);

        // Run until (at least) the game start date
        // Generate a few extra "easy" missions for the player to get started with
        let mut gamestart = Date::default();
        let (second, minute, hour, day, month, year) = GAME_START_DATE;
        gamestart.set_date(second, minute, hour, day, month, year);

        loop {
            World::update(g);
            g.world_mut().date.advance_day(1);
            g.world_mut()
                .date
                .advance_hour(NumberGenerator::random_number(12) - 6);
            g.world_mut()
                .date
                .advance_minute(NumberGenerator::random_number(60));

            if NumberGenerator::random_number(5) == 0 {
                WorldGenerator::generate_simple_starting_mission_a(g);
            }
            if NumberGenerator::random_number(5) == 0 {
                WorldGenerator::generate_simple_starting_mission_b(g);
            }

            if !g.world().date.before(&gamestart) {
                break;
            }
        }
    }

    /// C++ `ExitGame`: closes the OpenGL layer (`opengl_close`).
    pub fn exit_game(g: &mut Ctx) {
        opengl_close(g);
    }

    pub fn set_game_speed(&mut self, newspeed: i32) {
        self.gamespeed = newspeed;
    }

    pub fn game_speed(&self) -> i32 {
        self.gamespeed
    }

    pub fn is_running(&self) -> bool {
        self.gamespeed > GAMESPEED_PAUSED
    }

    /// Ends the current game.
    pub fn game_over(&mut self, reason: &str) {
        self.gamespeed = GAMESPEED_GAMEOVER;
        let gob = self.gob.get_or_insert_with(Default::default);
        gob.set_game_over_reason(reason);
        gob.evaluate(self.world.as_deref().expect("Game::game_over: no world"));
    }

    /// This demo game has come to an end.
    pub fn demo_game_over(&mut self) {
        self.gamespeed = GAMESPEED_GAMEOVER;
        let gob = self.gob.get_or_insert_with(Default::default);
        gob.set_game_over_reason(
            "You reached the maximum permissable Agent rating for a Demo user.\nYou can visit www.introversion.co.uk to buy the full product.",
        );
        gob.set_demo_game_over(true);
        gob.evaluate(
            self.world
                .as_deref()
                .expect("Game::demo_game_over: no world"),
        );
    }

    /// This WAREZ copy has come to an end.
    pub fn warez_game_over(&mut self) {
        self.gamespeed = GAMESPEED_GAMEOVER;
        let gob = self.gob.get_or_insert_with(Default::default);
        gob.set_game_over_reason(
            "You are playing an illegal copy of Uplink.\nYou can visit www.introversion.co.uk to buy the full product.",
        );
        gob.set_warez_game_over(true);
        gob.evaluate(
            self.world
                .as_deref()
                .expect("Game::warez_game_over: no world"),
        );
    }

    // The getters assert their value exists (check `is_running` first).
    pub fn get_interface(&self) -> &Interface {
        self.ui
            .as_deref()
            .expect("Game::get_interface: no interface")
    }

    pub fn get_interface_mut(&mut self) -> &mut Interface {
        self.ui
            .as_deref_mut()
            .expect("Game::get_interface_mut: no interface")
    }

    pub fn get_view(&self) -> &View {
        self.view.as_deref().expect("Game::get_view: no view")
    }

    pub fn get_view_mut(&mut self) -> &mut View {
        self.view
            .as_deref_mut()
            .expect("Game::get_view_mut: no view")
    }

    pub fn get_world(&self) -> &World {
        self.world.as_deref().expect("Game::get_world: no world")
    }

    pub fn get_world_mut(&mut self) -> &mut World {
        self.world
            .as_deref_mut()
            .expect("Game::get_world_mut: no world")
    }

    pub fn get_obituary(&self) -> &GameObituary {
        self.gob
            .as_deref()
            .expect("Game::get_obituary: no obituary")
    }

    pub fn get_obituary_mut(&mut self) -> &mut GameObituary {
        self.gob
            .as_deref_mut()
            .expect("Game::get_obituary_mut: no obituary")
    }

    /// C++ `LoadGame(FILE*)`: `json` is the text of a `.usr` file (`GameSave`). Returns false (and shows
    /// the login screen plus an error message box) if the save version is wrong or the file is unreadable.
    pub fn load_game(g: &mut Ctx, json: &str) -> bool {
        // PORT: the C++ srand(time(NULL)) is not ported; the random number generator is not reseeded.
        // PORT: the C++ LoadID / LoadID_END block markers are not needed with serde.

        // Check the file version is correct. An unreadable file has no version, so it fails the same way.
        let save: GameSave = serde_json::from_str(json).unwrap_or_default();
        if save.version != SAVEFILE_VERSION {
            MainMenu::run_screen(g, MAINMENU_LOGIN);

            let message = format!(
                "Failed to load user profile\nThe save file is not compatable\n\nSave file is Version [{}]\nRequired Version is  [{}]",
                save.version, SAVEFILE_VERSION
            );
            create_msgbox(g, "Error", &message);

            return false;
        }

        // Reset currently running game (the interface is rebuilt by App::load_game, as in the C++)
        g.game.ui = Some(Box::new(Interface::new()));
        TaskManager::reset(g);
        g.game.view = Some(Box::new(View::new()));
        let world = World::new(g);
        g.game.world = Some(Box::new(world));

        // Is this a "GameOver" game?
        g.game.gamespeed = save.gamespeed;

        if save.gamespeed != GAMESPEED_GAMEOVER {
            // Load each of the modules
            if let Some(mut world) = save.world {
                // PORT: passwords and gatewaydefs are not serialised (the C++ World::Load fills the
                // World made by `new World`, which had already loaded them): keep the fresh ones.
                if let Some(fresh) = g.game.world.as_deref_mut() {
                    world.passwords = std::mem::take(&mut fresh.passwords);
                    world.gatewaydefs = std::mem::take(&mut fresh.gatewaydefs);
                }
                // PORT: saves made before `people` became a tosser BTree have no "PLAYER" key.
                if world.player.is_some() && !world.people.contains_key("PLAYER") {
                    world.people.put_placeholder("PLAYER");
                }
                g.game.world = Some(Box::new(world));
            }
            if let Some(view) = save.view {
                g.game.view = Some(Box::new(view));
            }
            // Interface::Load: RemoteInterface::Load reads its fields then calls Create, and
            // LocalInterface::Load reads its fields then creates the HUD and runs the saved screen
            if let Some(i) = save.interface {
                let ri = &mut g.interface_mut().ri;
                ri.previousscreenindex = i.previousscreenindex;
                ri.currentscreenindex = i.currentscreenindex;
                ri.security_name = i.security_name;
                ri.security_level = i.security_level;
                let li = &mut g.interface_mut().li;
                li.currentscreencode = i.currentscreencode;
                li.screenindex = i.screenindex;
            }
            RemoteInterface::create(g);
            LocalInterface::create(g);
        } else {
            // This is a "game over" game
            g.game.gob = Some(Box::new(save.gob.unwrap_or_default()));
        }

        true
    }

    /// C++ `Save` (the interface part is `InterfaceSave`): the `.usr` file text, or `None` if the game is paused (nothing is written then).
    pub fn to_json(&self) -> Option<String> {
        if self.gamespeed == GAMESPEED_PAUSED {
            return None;
        }

        let game_over = self.gamespeed == GAMESPEED_GAMEOVER;
        let interface = match self.ui.as_deref() {
            Some(ui) if !game_over => Some(InterfaceSave {
                previousscreenindex: ui.ri.previousscreenindex,
                currentscreenindex: ui.ri.currentscreenindex,
                security_name: ui.ri.security_name.clone(),
                security_level: ui.ri.security_level,
                // If we're looking at the LAN, don't save that fact
                // otherwise the window will be recreated on re-load, covering the screen
                currentscreencode: if ui.li.currentscreencode == SCREEN_LAN {
                    SCREEN_NONE
                } else {
                    ui.li.currentscreencode
                },
                screenindex: ui.li.screenindex,
            }),
            _ => None,
        };
        let save = GameSaveRef {
            version: SAVEFILE_VERSION,
            gamespeed: self.gamespeed,
            world: if game_over { None } else { self.world.as_deref() },
            view: if game_over { None } else { self.view.as_deref() },
            interface,
            gob: if game_over { self.gob.as_deref() } else { None },
        };

        serde_json::to_string(&save).ok()
    }

    /// Updates world / view / interface when running, and autosaves every minute.
    pub fn update(g: &mut Ctx) {
        //
        // Update the modules
        //
        if g.game.is_running() {
            World::update(g);
            g.game.get_view_mut().update();
            Interface::update(g);
        }

        //
        // Autosave every minute
        //
        if now_secs() > g.game.lastsave + 1 * 60 {
            let handle = g.player().agent().handle.clone();
            App::save_game(g, &handle);
            g.game.lastsave = now_secs();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{bare_ctx, mount_game_data, scratch_dir};
    use crate::world::generator::worldgenerator::WorldGenerator;

    #[test]
    fn a_paused_game_writes_no_save() {
        let game = Game::new();
        assert_eq!(game.game_speed(), GAMESPEED_PAUSED);
        assert!(game.to_json().is_none());
    }

    #[test]
    fn a_running_game_without_a_world_saves_its_version_and_speed() {
        let mut game = Game::new();
        game.set_game_speed(GAMESPEED_FAST);
        let save: GameSave = serde_json::from_str(&game.to_json().unwrap()).unwrap();
        assert_eq!(save.version, SAVEFILE_VERSION);
        assert_eq!(save.gamespeed, GAMESPEED_FAST);
        assert!(save.world.is_none() && save.interface.is_none() && save.gob.is_none());
    }

    #[test]
    fn a_game_over_game_saves_only_the_obituary() {
        let mut game = Game::new();
        game.world = Some(Box::new(World::default()));
        game.gob = Some(Box::new(GameObituary::default()));
        game.set_game_speed(GAMESPEED_GAMEOVER);
        let save: GameSave = serde_json::from_str(&game.to_json().unwrap()).unwrap();
        assert_eq!(save.gamespeed, GAMESPEED_GAMEOVER);
        assert!(save.world.is_none());
        assert!(save.gob.is_some());
    }

    #[test]
    fn an_empty_save_file_is_not_a_valid_version() {
        let save: GameSave = serde_json::from_str("").unwrap_or_default();
        assert_ne!(save.version, SAVEFILE_VERSION);
        let save: GameSave = serde_json::from_str(r#"{"version":"SAV01"}"#).unwrap();
        assert_ne!(save.version, SAVEFILE_VERSION);
    }

    #[test]
    fn a_default_world_survives_the_save_format() {
        let mut game = Game::new();
        game.world = Some(Box::new(World::default()));
        game.set_game_speed(GAMESPEED_NORMAL);
        let json = game.to_json().unwrap();
        let save: GameSave = serde_json::from_str(&json).unwrap();
        let again = serde_json::to_value(save.world.as_ref().unwrap()).unwrap();
        assert_eq!(again, serde_json::to_value(game.world.as_deref().unwrap()).unwrap());
    }

    /// Generates the starting world (needs the original game's data, no window) and checks that
    /// what the save writes reads back to the same world.
    #[test]
    fn a_generated_world_round_trips_through_the_save_file() {
        if !mount_game_data() {
            return;
        }
        let mut g = bare_ctx();
        g.app.options.create_default_options();
        let world = World::new(&g);
        g.game.world = Some(Box::new(world));
        WorldGenerator::generate_all(&mut g);
        g.player_mut().set_handle("Tester");
        g.game.set_game_speed(GAMESPEED_NORMAL);

        assert!(g.world().computers.size() > 50);
        assert!(g.world().people.size() > 20);

        let json = g.game.to_json().unwrap();
        let save: GameSave = serde_json::from_str(&json).unwrap();
        assert_eq!(save.version, SAVEFILE_VERSION);
        let loaded = save.world.unwrap();
        assert_eq!(
            serde_json::to_value(&loaded).unwrap(),
            serde_json::to_value(g.world()).unwrap()
        );
        assert_eq!(loaded.player.as_ref().unwrap().agent().handle, "Tester");
        assert_eq!(loaded.computers.size(), g.world().computers.size());
        // and the loaded world saves to the very same text
        let mut game2 = Game::new();
        game2.world = Some(Box::new(loaded));
        game2.set_game_speed(GAMESPEED_NORMAL);
        assert_eq!(game2.to_json().unwrap(), json);
    }

    #[test]
    fn save_game_writes_a_redshirt_file_that_reads_back() {
        let dir = scratch_dir("save");
        let mut g = bare_ctx();
        g.app.userpath = dir.clone();
        g.game.world = Some(Box::new(World::default()));
        g.game.set_game_speed(GAMESPEED_NORMAL);

        App::save_game(&g, "NEWAGENT");
        assert!(!std::path::Path::new(&format!("{dir}NEWAGENT.usr")).exists());

        App::save_game(&g, "Tester");
        let path = format!("{dir}Tester.usr");
        let raw = std::fs::read(&path).unwrap();
        assert!(uplink_engine::redshirt::is_encrypted(&raw));
        assert!(!raw.windows(7).any(|w| w == b"version"));
        let plain = uplink_engine::redshirt::read_file(std::path::Path::new(&path)).unwrap();
        assert_eq!(String::from_utf8(plain).unwrap(), g.game.to_json().unwrap());

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
