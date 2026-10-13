//! Port of source/mainmenu/options_interface.h + source/mainmenu/options_interface.cpp

use crate::app::globals;
use crate::app::opengl_interface::button_assignbitmaps;
use crate::ctx::Ctx;
use crate::eclipse::{Button, MOVE_STRAIGHTLINE};
use crate::interface::interfacescreen::{InterfaceScreen, ScreenSlot};
use crate::mainmenu::genericoptions_interface::GenericOptionsInterface;
use crate::mainmenu::mainmenu::{
    MainMenu, MAINMENU_GENERICOPTIONS, MAINMENU_GRAPHICOPTIONS, MAINMENU_LOGIN, MAINMENU_NETWORKOPTIONS,
    MAINMENU_OPTIONS, MAINMENU_THEME,
};
use crate::mainmenu::mainmenuscreen;

/// The main menu's screen slot (`app->GetMainMenu()->GetMenuScreen()`).
fn menu_screen_slot(g: &mut Ctx) -> &mut ScreenSlot {
    &mut g.app.mainmenu.screen
}

/// `((GenericOptionsInterface *) app->GetMainMenu()->GetMenuScreen())->SetOptionTYPE ( newtype )`
/// after the generic options screen has been run. The screen is taken out of its slot while
/// `set_option_type` runs, because it needs `g` as well.
fn set_generic_option_type(g: &mut Ctx, newtype: &str) {
    ScreenSlot::call(g, menu_screen_slot, |s, g| {
        debug_assert_eq!(s.screen_id(), MAINMENU_GENERICOPTIONS);
        if let Some(generic) = s.as_any_mut().downcast_mut::<GenericOptionsInterface>() {
            generic.set_option_type(g, newtype);
        }
    });
}

/// Options screen: the menu that leads to the game, graphics, sound, theme and network options.
#[derive(Debug, Default)]
pub struct OptionsInterface {}

impl OptionsInterface {
    pub fn new() -> Self {
        Self::default()
    }

    /// Static `GameOptionsClick` (MouseFn).
    pub fn game_options_click(g: &mut Ctx, _button: &Button) {
        MainMenu::run_screen(g, MAINMENU_GENERICOPTIONS);
        set_generic_option_type(g, "game");
    }
    /// Static `GraphicsOptionsClick` (MouseFn).
    // PORT: the C++ kept a commented-out generic-options "graphics" variant; it is dropped.
    pub fn graphics_options_click(g: &mut Ctx, _button: &Button) {
        MainMenu::run_screen(g, MAINMENU_GRAPHICOPTIONS);
    }
    /// Static `SoundOptionsClick` (MouseFn).
    pub fn sound_options_click(g: &mut Ctx, _button: &Button) {
        MainMenu::run_screen(g, MAINMENU_GENERICOPTIONS);
        set_generic_option_type(g, "sound");
    }
    /// Static `ThemeOptionsClick` (MouseFn).
    pub fn theme_options_click(g: &mut Ctx, _button: &Button) {
        MainMenu::run_screen(g, MAINMENU_THEME);
    }
    /// Static `NetworkOptionsClick` (MouseFn).
    pub fn network_options_click(g: &mut Ctx, _button: &Button) {
        MainMenu::run_screen(g, MAINMENU_NETWORKOPTIONS);
    }
    /// Static `ReturnToMainMenuClick` (MouseFn).
    pub fn return_to_main_menu_click(g: &mut Ctx, _button: &Button) {
        MainMenu::run_screen(g, MAINMENU_LOGIN);
    }
}

impl InterfaceScreen for OptionsInterface {
    fn create(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            mainmenuscreen::create(g);

            let screenw = g.app.options.get_option_value("graphics_screenwidth");
            let screenh = g.app.options.get_option_value("graphics_screenheight");

            g.ecl.register_button(-832, screenh - 50, 32, 32, "", "Edit GAME options", "gameoptions");
            g.ecl.register_button(-672, screenh - 50, 32, 32, "", "Edit GRAPHICS options", "graphicsoptions");
            g.ecl.register_button(-512, screenh - 50, 32, 32, "", "Edit SOUND options", "soundoptions");
            g.ecl.register_button(-352, screenh - 50, 32, 32, "", "Edit MULTIMONITOR options", "networkoptions");
            g.ecl.register_button(-129, screenh - 50, 32, 32, "", "Edit THEME options", "themeoptions");

            g.ecl.register_button(-32, screenh - 50, 32, 32, "", "Return to Main Menu", "mainmenu");

            button_assignbitmaps(
                g,
                "gameoptions",
                "mainmenu/gameoptions.tif",
                "mainmenu/gameoptions_h.tif",
                "mainmenu/gameoptions_c.tif",
            );
            button_assignbitmaps(
                g,
                "graphicsoptions",
                "mainmenu/graphicsoptions.tif",
                "mainmenu/graphicsoptions_h.tif",
                "mainmenu/graphicsoptions_c.tif",
            );
            button_assignbitmaps(
                g,
                "soundoptions",
                "mainmenu/soundoptions.tif",
                "mainmenu/soundoptions_h.tif",
                "mainmenu/soundoptions_c.tif",
            );
            button_assignbitmaps(
                g,
                "networkoptions",
                "mainmenu/networkoptions.tif",
                "mainmenu/networkoptions_h.tif",
                "mainmenu/networkoptions_c.tif",
            );
            button_assignbitmaps(
                g,
                "themeoptions",
                "mainmenu/theme.tif",
                "mainmenu/theme_h.tif",
                "mainmenu/theme_c.tif",
            );
            button_assignbitmaps(
                g,
                "mainmenu",
                "mainmenu/exitgame.tif",
                "mainmenu/exitgame_h.tif",
                "mainmenu/exitgame_c.tif",
            );

            g.ecl.register_button_callback("gameoptions", Self::game_options_click);
            g.ecl.register_button_callback("graphicsoptions", Self::graphics_options_click);
            g.ecl.register_button_callback("soundoptions", Self::sound_options_click);
            g.ecl.register_button_callback("mainmenu", Self::return_to_main_menu_click);

            if globals::DEMOGAME {
                if let Some(b) = g.ecl.get_button_mut("networkoptions") {
                    b.set_tooltip("(FULL GAME ONLY) Edit MULTIMONITOR options");
                }
                if let Some(b) = g.ecl.get_button_mut("themeoptions") {
                    b.set_tooltip("(FULL GAME ONLY) Edit THEME options");
                }
            } else {
                g.ecl.register_button_callback("networkoptions", Self::network_options_click);
                g.ecl.register_button_callback("themeoptions", Self::theme_options_click);
            }

            g.ecl.register_movement("mainmenu", screenw - 40, screenh - 50, 975, MOVE_STRAIGHTLINE, None);
            g.ecl.register_movement("themeoptions", screenw - 80, screenh - 50, 1200, MOVE_STRAIGHTLINE, None);
            g.ecl.register_movement("networkoptions", screenw - 120, screenh - 50, 1425, MOVE_STRAIGHTLINE, None);
            g.ecl.register_movement("soundoptions", screenw - 160, screenh - 50, 1650, MOVE_STRAIGHTLINE, None);
            g.ecl.register_movement("graphicsoptions", screenw - 200, screenh - 50, 1875, MOVE_STRAIGHTLINE, None);
            g.ecl.register_movement("gameoptions", screenw - 240, screenh - 50, 2100, MOVE_STRAIGHTLINE, None);
        }
    }

    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            mainmenuscreen::remove(g);

            g.ecl.remove_button("gameoptions");
            g.ecl.remove_button("graphicsoptions");
            g.ecl.remove_button("soundoptions");
            g.ecl.remove_button("networkoptions");
            g.ecl.remove_button("themeoptions");
            g.ecl.remove_button("mainmenu");
        }
    }

    fn update(&mut self, _g: &mut Ctx) {}

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("gameoptions").is_some()
    }
    fn screen_id(&self) -> i32 {
        MAINMENU_OPTIONS
    }
    crate::impl_as_any!();
}
