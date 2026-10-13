//! Port of source/mainmenu/connectionlost_interface.h + source/mainmenu/connectionlost_interface.cpp

use crate::app::opengl_interface::{button_assignbitmaps, textbutton_draw};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::mainmenu::mainmenu::{MainMenu, MAINMENU_CONNECTIONLOST, MAINMENU_LOGIN};

/// C `time(NULL)`: unix seconds.
fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Connection Lost Interface. Shown after the connection from the home PC to the Gateway is lost.
#[derive(Debug, Default)]
pub struct ConnectionLostInterface {
    pub starttime: i32,
}

impl ConnectionLostInterface {
    pub fn new() -> Self {
        Self::default()
    }

    /// Static `ReturnToMainMenuClick` (MouseFn).
    pub fn return_to_main_menu_click(g: &mut Ctx, _button: &Button) {
        MainMenu::run_screen(g, MAINMENU_LOGIN);
    }
}

impl InterfaceScreen for ConnectionLostInterface {
    fn create(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            g.ecl.register_button(220, 230, 450, 200, " ", "", "connectionlost");
            g.ecl.register_button_callbacks("connectionlost", Some(textbutton_draw), None, None, None);

            self.starttime = now_secs() as i32;
        }
    }
    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            g.ecl.remove_button("connectionlost");
            g.ecl.remove_button("connectionlost_mainmenu");
        }
    }
    fn update(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            if self.starttime != -1 && now_secs() > self.starttime as i64 + 4 {
                let caption_blank = g.ecl.get_button("connectionlost").is_some_and(|b| b.caption == " ");
                if caption_blank {
                    g.ecl.register_caption_change_auto("connectionlost", "Connection to GATEWAY lost", None);
                    self.starttime = now_secs() as i32;
                } else {
                    // Exit button

                    let screenw = g.app.options.get_option_value("graphics_screenwidth");
                    let screenh = g.app.options.get_option_value("graphics_screenheight");

                    g.ecl.register_button(screenw - 40, screenh - 40, 32, 32, "", "Return to Main Menu", "connectionlost_mainmenu");
                    g.ecl.register_button_callback("connectionlost_mainmenu", Self::return_to_main_menu_click);
                    button_assignbitmaps(
                        g,
                        "connectionlost_mainmenu",
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
        g.ecl.get_button("connectionlost").is_some()
    }
    fn screen_id(&self) -> i32 {
        MAINMENU_CONNECTIONLOST
    }
    crate::impl_as_any!();
}
