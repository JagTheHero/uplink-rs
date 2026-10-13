//! `Ctx`: the single owner of all game state, replacing the C++ globals `app` and `game`
//! (and the file-scope globals of eclipse/vanbakel). Passed as `g: &mut Ctx` everywhere.
//!
//! Contract with the ported modules (do not rename these fields):
//! - `App`   (app/app.rs):       `pub options: Options`, `pub network: Network`, `pub mainmenu: MainMenu`
//! - `Game`  (game/game.rs):     `pub world: Option<Box<World>>`, `pub ui: Option<Box<Interface>>`, `pub view: Option<Box<View>>`
//! - `World` (world/world.rs):   `pub player: Option<Box<Player>>`
//! - `Interface` (interface/interface.rs): `pub li: LocalInterface`, `pub ri: RemoteInterface`, `pub tm: TaskManager`

use crate::app::app::App;
use crate::eclipse::Eclipse;
use crate::game::game::Game;
use crate::interface::interface::Interface;
use crate::world::computer::lanmonitor::LanMonitor;
use crate::world::computer::securitymonitor::SecurityMonitor;
use crate::vanbakel::Vanbakel;
use crate::world::player::Player;
use crate::world::world::World;

type Deferred = Box<dyn FnOnce(&mut Ctx)>;

pub struct Ctx {
    pub app: App,
    pub game: Game,
    pub ecl: Eclipse,
    pub svb: Vanbakel,
    /// The C++ `LanMonitor` statics (the player's current LAN attack).
    pub lan_monitor: LanMonitor,
    /// The C++ `SecurityMonitor` statics (security systems watching the current connection).
    pub security_monitor: SecurityMonitor,
    /// A number `PhoneDialler::dial_number` asked to dial, with the keypad's position; played by
    /// `opengl_run` (see [`crate::interface::localinterface::phonedialler::play_dial`]).
    pub pending_dial: Option<(i32, i32, String)>,
    deferred: Vec<Deferred>,
    running_deferred: bool,
}

impl Ctx {
    pub fn new(app: App, game: Game) -> Self {
        Ctx { app, game, ecl: Eclipse::new(), svb: Vanbakel::default(), lan_monitor: LanMonitor::default(), security_monitor: SecurityMonitor::default(), pending_dial: None, deferred: Vec::new(), running_deferred: false }
    }

    /// `game->GetWorld()`. Panics if no game is running (the C++ asserted).
    pub fn world(&self) -> &World {
        self.game.world.as_deref().expect("Ctx::world: no world (game not running)")
    }

    pub fn world_mut(&mut self) -> &mut World {
        self.game.world.as_deref_mut().expect("Ctx::world_mut: no world (game not running)")
    }

    /// `game->GetWorld()->GetPlayer()`
    pub fn player(&self) -> &Player {
        self.world().player.as_deref().expect("Ctx::player: no player")
    }

    pub fn player_mut(&mut self) -> &mut Player {
        self.world_mut().player.as_deref_mut().expect("Ctx::player_mut: no player")
    }

    /// `game->GetInterface()`
    pub fn interface(&self) -> &Interface {
        self.game.ui.as_deref().expect("Ctx::interface: no interface")
    }

    pub fn interface_mut(&mut self) -> &mut Interface {
        self.game.ui.as_deref_mut().expect("Ctx::interface_mut: no interface")
    }

    /// Run `f` once the current screen/task method has returned (used for re-entrant
    /// screen switches and task removal). Runs at the latest at the end of the frame.
    pub fn defer(&mut self, f: impl FnOnce(&mut Ctx) + 'static) {
        self.deferred.push(Box::new(f));
    }

    /// Run all deferred actions (in order). Safe to call any time; nested calls are ignored.
    pub fn run_deferred(&mut self) {
        if self.running_deferred {
            return;
        }
        // One batch per call: an action that re-defers itself (because its target is still
        // busy) must not spin here; it runs on the next call (at the latest next frame).
        self.running_deferred = true;
        let batch = std::mem::take(&mut self.deferred);
        for f in batch {
            f(self);
        }
        self.running_deferred = false;
    }
}
