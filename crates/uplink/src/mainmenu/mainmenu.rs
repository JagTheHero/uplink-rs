//! Port of source/mainmenu/mainmenu.h + source/mainmenu/mainmenu.cpp
#![allow(unused_variables, unused_imports, unused_mut, dead_code)]

use crate::ctx::Ctx;
use crate::interface::interfacescreen::{InterfaceScreen, ScreenSlot};
use crate::mainmenu::{
    closing_interface::ClosingInterface, connectionlost_interface::ConnectionLostInterface,
    demogameover_interface::DemoGameOverInterface, disavowed_interface::DisavowedInterface,
    firsttimeloading_interface::FirstTimeLoadingInterface, genericoptions_interface::GenericOptionsInterface,
    graphicoptions_interface::GraphicOptionsInterface, loading_interface::LoadingInterface,
    login_interface::LoginInterface, networkoptions_interface::NetworkOptionsInterface,
    obituary_interface::ObituaryInterface, options_interface::OptionsInterface,
    revelationlost_interface::RevelationLostInterface, revelationwon_interface::RevelationWonInterface,
    theme_interface::ThemeInterface, theteam_interface::TheTeamInterface,
    warezgameover_interface::WarezGameOverInterface,
};

// === List all screen codes here ===

pub const MAINMENU_UNKNOWN: i32 = 0;
pub const MAINMENU_LOGIN: i32 = 1;
pub const MAINMENU_LOADING: i32 = 2;
pub const MAINMENU_FIRSTLOAD: i32 = 3;
pub const MAINMENU_OPTIONS: i32 = 4;
pub const MAINMENU_OBITUARY: i32 = 5;
pub const MAINMENU_CONNECTIONLOST: i32 = 6;
pub const MAINMENU_DISAVOWED: i32 = 7;
pub const MAINMENU_THETEAM: i32 = 8;
pub const MAINMENU_CLOSING: i32 = 9;
pub const MAINMENU_DEMOGAMEOVER: i32 = 10;
pub const MAINMENU_REVELATIONWON: i32 = 11;
pub const MAINMENU_REVELATIONLOST: i32 = 12;
pub const MAINMENU_WAREZGAMEOVER: i32 = 13;

pub const MAINMENU_GENERICOPTIONS: i32 = 20;
pub const MAINMENU_NETWORKOPTIONS: i32 = 21;
pub const MAINMENU_THEME: i32 = 22;
pub const MAINMENU_GRAPHICOPTIONS: i32 = 23;

fn screen_slot(g: &mut Ctx) -> &mut ScreenSlot {
    &mut g.app.mainmenu.screen
}

/// Main menu controller. Lives at `g.app.mainmenu`; `screen` holds a `MainMenuScreen` subclass.
pub struct MainMenu {
    pub currentscreencode: i32,
    pub screen: ScreenSlot,
}

impl MainMenu {
    pub fn new() -> Self {
        MainMenu {
            currentscreencode: 0,
            screen: ScreenSlot::new(),
        }
    }

    /// Resets Eclipse to the configured screen size, then runs the current screen code.
    pub fn create(g: &mut Ctx) {
        let width = g.app.options.get_option_value("graphics_screenwidth");
        let height = g.app.options.get_option_value("graphics_screenheight");
        g.ecl.reset(width, height);

        let screencode = g.app.mainmenu.currentscreencode;
        Self::run_screen(g, screencode);
    }

    pub fn remove(g: &mut Ctx) {
        if g.app.mainmenu.is_visible() {
            ScreenSlot::call(g, screen_slot, |s, g| s.remove(g));
        }
    }

    pub fn is_visible(&self) -> bool {
        self.in_screen() != MAINMENU_UNKNOWN
    }

    /// Returns the id code of the current screen (MAINMENU_UNKNOWN if none).
    pub fn in_screen(&self) -> i32 {
        self.screen.screen_id()
    }

    /// Replaces the screen (via `ScreenSlot::replace(g, screen_slot, ..)`) with the one for
    /// `screencode`; aborts on MAINMENU_UNKNOWN / unknown codes.
    pub fn run_screen(g: &mut Ctx, screencode: i32) {
        g.app.mainmenu.currentscreencode = screencode;

        let screen: Box<dyn InterfaceScreen> = match screencode {
            MAINMENU_LOGIN => Box::new(LoginInterface::new()),
            MAINMENU_LOADING => Box::new(LoadingInterface::new()),
            MAINMENU_FIRSTLOAD => Box::new(FirstTimeLoadingInterface::new()),
            MAINMENU_OPTIONS => Box::new(OptionsInterface::new()),
            MAINMENU_OBITUARY => Box::new(ObituaryInterface::new()),
            MAINMENU_CONNECTIONLOST => Box::new(ConnectionLostInterface::new()),
            MAINMENU_DISAVOWED => Box::new(DisavowedInterface::new()),
            MAINMENU_THETEAM => Box::new(TheTeamInterface::new()),
            MAINMENU_CLOSING => Box::new(ClosingInterface::new()),
            MAINMENU_DEMOGAMEOVER => Box::new(DemoGameOverInterface::new()),
            MAINMENU_REVELATIONWON => Box::new(RevelationWonInterface::new()),
            MAINMENU_REVELATIONLOST => Box::new(RevelationLostInterface::new()),
            MAINMENU_WAREZGAMEOVER => Box::new(WarezGameOverInterface::new()),

            MAINMENU_GENERICOPTIONS => Box::new(GenericOptionsInterface::new()),
            MAINMENU_NETWORKOPTIONS => Box::new(NetworkOptionsInterface::new()),
            MAINMENU_THEME => Box::new(ThemeInterface::new()),
            MAINMENU_GRAPHICOPTIONS => Box::new(GraphicOptionsInterface::new()),

            MAINMENU_UNKNOWN => panic!("Tried to create a local screen with MAINMENU_UNKNOWN SCREENCODE"),
            _ => panic!("Tried to create a local screen with unknown SCREENCODE"),
        };

        // Removes the old screen, stores the new one and creates it (deferred if called from
        // inside the current screen's own method).
        ScreenSlot::replace(g, screen_slot, Some(screen));
    }

    pub fn update(g: &mut Ctx) {
        ScreenSlot::call(g, screen_slot, |s, g| s.update(g));
    }
}

impl Default for MainMenu {
    fn default() -> Self {
        MainMenu::new()
    }
}
