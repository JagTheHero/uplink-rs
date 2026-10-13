//! Port of source/mainmenu/warezgameover_interface.h + source/mainmenu/warezgameover_interface.cpp

use crate::app::opengl_interface::{button_assignbitmaps, textbutton_draw};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::mainmenu::mainmenu::{MainMenu, MAINMENU_LOGIN, MAINMENU_WAREZGAMEOVER};

/// C `time(NULL)`: unix seconds.
fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// The text shown once the warez game over caption has been cleared.
const CAPTION_TEXT: &str = concat!(
    "Uplink Corporation has detected that you are using\n",
    "an illegal copy of this product.\n\n",
    "For security reasons, Uplink Corporation cannot allow you to continue\n",
    "working for our company.\n",
    "As such, we have terminated your Agent status\n",
    "and closed down your accounts.\n\n",
    "If you are interested in becoming an authorised Uplink Agent,\n",
    "you will need to purchase the full Uplink CD.\n",
    "Details can be found on our web site.\n\n",
    "www.introversion.co.uk\n\n",
);

/// Warez Game Over screen.
#[derive(Debug, Default)]
pub struct WarezGameOverInterface {
    pub starttime: i32,
}

impl WarezGameOverInterface {
    pub fn new() -> Self {
        Self::default()
    }

    /// Static `ReturnToMainMenuClick` (MouseFn).
    pub fn return_to_main_menu_click(g: &mut Ctx, _button: &Button) {
        MainMenu::run_screen(g, MAINMENU_LOGIN);
    }
}

impl InterfaceScreen for WarezGameOverInterface {
    fn create(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            g.ecl.register_button(120, 130, 450, 300, " ", "", "warezgameover");
            g.ecl.register_button_callbacks("warezgameover", Some(textbutton_draw), None, None, None);

            self.starttime = now_secs() as i32;
        }
    }
    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            g.ecl.remove_button("warezgameover");
            g.ecl.remove_button("warezgameover_mainmenu");
        }
    }
    fn update(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            if self.starttime != -1 && now_secs() > self.starttime as i64 + 4 {
                let caption_blank = g.ecl.get_button("warezgameover").is_some_and(|b| b.caption == " ");
                if caption_blank {
                    g.ecl.register_caption_change_auto("warezgameover", CAPTION_TEXT, None);
                    self.starttime = now_secs() as i32;
                } else {
                    // Exit button

                    let screenw = g.app.options.get_option_value("graphics_screenwidth");
                    let screenh = g.app.options.get_option_value("graphics_screenheight");

                    g.ecl.register_button(screenw - 40, screenh - 40, 32, 32, "", "Return to Main Menu", "warezgameover_mainmenu");
                    g.ecl.register_button_callback("warezgameover_mainmenu", Self::return_to_main_menu_click);
                    button_assignbitmaps(
                        g,
                        "warezgameover_mainmenu",
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
        g.ecl.get_button("warezgameover").is_some()
    }
    fn screen_id(&self) -> i32 {
        MAINMENU_WAREZGAMEOVER
    }
    crate::impl_as_any!();
}
