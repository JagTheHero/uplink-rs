//! Port of source/interface/localinterface/localinterface.h + source/interface/localinterface/localinterface.cpp
#![allow(unused_variables, unused_imports, unused_mut, dead_code)]

use uplink_engine::sound;

use crate::ctx::Ctx;
use crate::interface::interfacescreen::{InterfaceScreen, ScreenSlot};
use crate::interface::localinterface::analyser_interface::AnalyserInterface;
use crate::interface::localinterface::cheat_interface::CheatInterface;
use crate::interface::localinterface::email_interface::EmailInterface;
use crate::interface::localinterface::evtqueue_interface::EventQueueInterface;
use crate::interface::localinterface::finance_interface::FinanceInterface;
use crate::interface::localinterface::hud_interface::HUDInterface;
use crate::interface::localinterface::hw_interface::HWInterface;
use crate::interface::localinterface::irc_interface::IRCInterface;
use crate::interface::localinterface::lan_interface::LanInterface;
use crate::interface::localinterface::memory_interface::MemoryInterface;
use crate::interface::localinterface::mission_interface::MissionInterface;
use crate::interface::localinterface::sendmail_interface::SendMailInterface;
use crate::interface::localinterface::status_interface::StatusInterface;
use crate::vanbakel::Vanbakel;

// === List all screen codes here ===

pub const SCREEN_NONE: i32 = 0;
pub const SCREEN_HUD: i32 = 1;
pub const SCREEN_HW: i32 = 3;
pub const SCREEN_MAP: i32 = 4;
pub const SCREEN_MEMORY: i32 = 6;
pub const SCREEN_STATUS: i32 = 7;
pub const SCREEN_EMAIL: i32 = 8;
pub const SCREEN_FINANCE: i32 = 9;
pub const SCREEN_MISSION: i32 = 10;
pub const SCREEN_SENDMAIL: i32 = 11;
pub const SCREEN_CHEATS: i32 = 12;
pub const SCREEN_EVTQUEUE: i32 = 13;
pub const SCREEN_ANALYSER: i32 = 14;
pub const SCREEN_GATEWAY: i32 = 15;
pub const SCREEN_IRC: i32 = 16;
pub const SCREEN_LAN: i32 = 17;

/// Fraction of the screen width taken up by the right panel.
pub const PANELSIZE: f64 = 0.29;

fn screen_slot(g: &mut Ctx) -> &mut ScreenSlot {
    &mut g.interface_mut().li.screen
}

fn hud_slot(g: &mut Ctx) -> &mut ScreenSlot {
    &mut g.interface_mut().li.hud
}

/// C++ `ValidIndex`: `index` is in range of a list holding `len` entries.
fn valid_index(index: i32, len: usize) -> bool {
    usize::try_from(index).is_ok_and(|i| i < len)
}

/// The interface projected by our local machine gateway (HUD + worldmap/email/... screens).
// PORT: C++ Load/Save persisted only `currentscreencode`/`screenindex` (SCREEN_LAN saved as
// SCREEN_NONE); the owner of the save format must do the same. `screen` holds a
// `LocalInterfaceScreen` subclass, `hud` the `HUDInterface`.
pub struct LocalInterface {
    pub screen: ScreenSlot,
    pub hud: ScreenSlot,
    pub currentscreencode: i32,
    /// Used for e.g. SCREEN_EMAIL (index of email viewed)
    pub screenindex: i32,
    /// The C++ `IRCInterface` statics (connection, buffer, smileys); they outlive the IRC screen.
    pub irc: IRCInterface,
}

impl LocalInterface {
    pub fn new() -> Self {
        let mut hud = ScreenSlot::new();
        hud.set(Box::new(HUDInterface::default()));
        LocalInterface {
            screen: ScreenSlot::new(),
            hud,
            currentscreencode: 0,
            screenindex: 0,
            irc: IRCInterface::default(),
        }
    }

    pub fn reset(&mut self) {
        self.currentscreencode = SCREEN_NONE;
        self.screenindex = 0;
    }

    pub fn create(g: &mut Ctx) {
        ScreenSlot::call(g, hud_slot, |s, g| s.create(g));
        let li = &g.interface().li;
        let (code, index) = (li.currentscreencode, li.screenindex);
        Self::run_screen(g, code, index);
    }

    pub fn remove(g: &mut Ctx) {
        if Self::is_visible(g) {
            ScreenSlot::call(g, screen_slot, |s, g| s.remove(g));
        }
    }

    pub fn is_visible(g: &Ctx) -> bool {
        let li = &g.interface().li;
        li.in_screen() != SCREEN_NONE && li.screen.get_dyn().is_some_and(|s| s.is_visible(g))
    }

    /// Returns the id code of the current screen (SCREEN_NONE if none).
    pub fn in_screen(&self) -> i32 {
        self.screen.screen_id()
    }

    /// `RunScreen(SCREENCODE, index)`; pass `index = -1` where the C++ used the default.
    /// Replaces the screen through `ScreenSlot::replace(g, screen_slot, ..)`, moves the HUD
    /// selecter and brings all running tasks to the front.
    pub fn run_screen(g: &mut Ctx, screencode: i32, index: i32) {
        // A screen method is running (the slot is taken out): switch once it returns.
        if screen_slot(g).is_busy() {
            g.defer(move |g| Self::run_screen(g, screencode, index));
            return;
        }

        // Get rid of the current local interface
        ScreenSlot::replace(g, screen_slot, None);

        {
            let li = &mut g.interface_mut().li;
            li.currentscreencode = screencode;
            li.screenindex = index;
        }
        HUDInterface::move_selecter(g, screencode, index);

        let new_screen: Option<Box<dyn InterfaceScreen>> = match screencode {
            SCREEN_NONE => {
                sound::play_sound("sounds/close.wav", None, false);
                None
            }
            SCREEN_HW => Some(Box::new(HWInterface::default())),
            SCREEN_MEMORY => Some(Box::new(MemoryInterface::default())),
            SCREEN_STATUS => Some(Box::new(StatusInterface::default())),
            SCREEN_FINANCE => Some(Box::new(FinanceInterface::default())),
            SCREEN_ANALYSER => Some(Box::new(AnalyserInterface::default())),
            SCREEN_IRC => Some(Box::new(IRCInterface::default())),
            SCREEN_LAN => Some(Box::new(LanInterface::default())),
            SCREEN_EMAIL => {
                if valid_index(index, g.player().messages.len()) {
                    let mut screen = EmailInterface::default();
                    screen.set_message(g, index);
                    Some(Box::new(screen))
                } else {
                    g.interface_mut().li.currentscreencode = SCREEN_NONE;
                    None
                }
            }
            SCREEN_MISSION => {
                let count = g.player().agent.as_ref().map_or(0, |a| a.missions.len());
                if valid_index(index, count) {
                    let mut screen = MissionInterface::default();
                    screen.set_mission(g, index);
                    Some(Box::new(screen))
                } else {
                    g.interface_mut().li.currentscreencode = SCREEN_NONE;
                    None
                }
            }
            SCREEN_SENDMAIL => Some(Box::new(SendMailInterface::default())),
            SCREEN_CHEATS => Some(Box::new(CheatInterface::default())),
            SCREEN_EVTQUEUE => Some(Box::new(EventQueueInterface::default())),
            _ => panic!("Tried to create a local screen with unknown SCREENCODE"),
        };

        // Creates the new screen (nothing for SCREEN_NONE) and brings all running tasks to the front
        ScreenSlot::replace(g, screen_slot, new_screen);
        Vanbakel::show_all_tasks(g);
    }

    pub fn update(g: &mut Ctx) {
        ScreenSlot::call(g, hud_slot, |s, g| s.update(g));
        ScreenSlot::call(g, screen_slot, |s, g| s.update(g));
    }
}

impl Default for LocalInterface {
    fn default() -> Self {
        LocalInterface::new()
    }
}
