//! Port of source/mainmenu/disavowed_interface.h + source/mainmenu/disavowed_interface.cpp

use crate::app::opengl_interface::{button_assignbitmaps, textbutton_draw};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::mainmenu::mainmenu::{MainMenu, MAINMENU_DISAVOWED, MAINMENU_LOGIN};

/// C `time(NULL)`: unix seconds.
fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// The text shown once the disavowed caption has been cleared.
const CAPTION_TEXT: &str = concat!(
    "We have recently been informed by a large corporation that you have \n",
    "been caught hacking into one of their primary computer systems.\n\n",
    "As such, we have been forced to disavow all knowledge of your actions \n",
    "and strike you off the active agents list.\n\n",
    "Your gateway and accounts have already been siezed.\n\n",
    "Let this be a warning to be more careful next time.",
);

/// Disavowed Interface. Shown when the connection is lost because the player has been disavowed.
#[derive(Debug, Default)]
pub struct DisavowedInterface {
    pub starttime: i32,
}

impl DisavowedInterface {
    pub fn new() -> Self {
        Self::default()
    }

    /// Static `ReturnToMainMenuClick` (MouseFn).
    pub fn return_to_main_menu_click(g: &mut Ctx, _button: &Button) {
        MainMenu::run_screen(g, MAINMENU_LOGIN);
    }
}

impl InterfaceScreen for DisavowedInterface {
    fn create(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            g.ecl.register_button(120, 130, 450, 200, " ", "", "disavowed");
            g.ecl.register_button_callbacks("disavowed", Some(textbutton_draw), None, None, None);

            self.starttime = now_secs() as i32;
        }
    }
    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            g.ecl.remove_button("disavowed");
            g.ecl.remove_button("disavowed_mainmenu");
        }
    }
    fn update(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            if self.starttime != -1 && now_secs() > self.starttime as i64 + 4 {
                let caption_blank = g.ecl.get_button("disavowed").is_some_and(|b| b.caption == " ");
                if caption_blank {
                    g.ecl.register_caption_change_auto("disavowed", CAPTION_TEXT, None);
                    self.starttime = now_secs() as i32;
                } else {
                    // Exit button

                    let screenw = g.app.options.get_option_value("graphics_screenwidth");
                    let screenh = g.app.options.get_option_value("graphics_screenheight");

                    g.ecl.register_button(screenw - 40, screenh - 40, 32, 32, "", "Return to Main Menu", "disavowed_mainmenu");
                    g.ecl.register_button_callback("disavowed_mainmenu", Self::return_to_main_menu_click);
                    button_assignbitmaps(
                        g,
                        "disavowed_mainmenu",
                        "mainmenu/exitgame.tif",
                        "mainmenu/exitgame_h.tif",
                        "mainmenu/exitgame_c.tif",
                    );

                    self.starttime = -1;
                }
            }
        }
    }
    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("disavowed").is_some()
    }
    fn screen_id(&self) -> i32 {
        MAINMENU_DISAVOWED
    }
    crate::impl_as_any!();
}
