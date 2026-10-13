//! Port of source/mainmenu/revelationlost_interface.h + source/mainmenu/revelationlost_interface.cpp

use crate::app::opengl_interface::{button_assignbitmaps, textbutton_draw};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::mainmenu::mainmenu::{MainMenu, MAINMENU_LOGIN, MAINMENU_REVELATIONLOST};

/// C `time(NULL)`: unix seconds.
fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// The text shown once the revelation-lost caption has been cleared.
const CAPTION_TEXT: &str = concat!(
    "We have terminated your connection to your Gateway.\n\n",
    "We have been watching your moves for the last few days, \n",
    "and we have known for some time that you were working for \n",
    "Andromeda Research Corporation.\n\n",
    "Recent actions of that company, and your part in those actions, \n",
    "have forced us to disavow your agent profile and knock you off \n",
    "the active agent list.\n\n",
    "Despite the damage you have caused, Revelation has not succeeded.\n\n",
    "We hope you will think more carefully about your actions next time.",
);

/// Revelation Lost Interface. The player tried to use Revelation and failed.
#[derive(Debug, Default)]
pub struct RevelationLostInterface {
    pub starttime: i32,
}

impl RevelationLostInterface {
    pub fn new() -> Self {
        Self::default()
    }

    /// Static `ReturnToMainMenuClick` (MouseFn).
    pub fn return_to_main_menu_click(g: &mut Ctx, _button: &Button) {
        MainMenu::run_screen(g, MAINMENU_LOGIN);
    }
}

impl InterfaceScreen for RevelationLostInterface {
    fn create(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            g.ecl.register_button(120, 130, 450, 200, " ", "", "revelationlost");
            g.ecl.register_button_callbacks("revelationlost", Some(textbutton_draw), None, None, None);

            self.starttime = now_secs() as i32;
        }
    }
    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            g.ecl.remove_button("revelationlost");
            g.ecl.remove_button("revelationlost_mainmenu");
        }
    }
    fn update(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            if self.starttime != -1 && now_secs() > self.starttime as i64 + 4 {
                let caption_blank = g.ecl.get_button("revelationlost").is_some_and(|b| b.caption == " ");
                if caption_blank {
                    g.ecl.register_caption_change_auto("revelationlost", CAPTION_TEXT, None);
                    self.starttime = now_secs() as i32;
                } else {
                    // Exit button

                    let screenw = g.app.options.get_option_value("graphics_screenwidth");
                    let screenh = g.app.options.get_option_value("graphics_screenheight");

                    g.ecl.register_button(screenw - 40, screenh - 40, 32, 32, "", "Return to Main Menu", "revelationlost_mainmenu");
                    g.ecl.register_button_callback("revelationlost_mainmenu", Self::return_to_main_menu_click);
                    button_assignbitmaps(
                        g,
                        "revelationlost_mainmenu",
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
        g.ecl.get_button("revelationlost").is_some()
    }
    fn screen_id(&self) -> i32 {
        MAINMENU_REVELATIONLOST
    }
    crate::impl_as_any!();
}
