//! Port of source/mainmenu/revelationwon_interface.h + source/mainmenu/revelationwon_interface.cpp

use crate::app::opengl_interface::{button_assignbitmaps, textbutton_draw};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::mainmenu::mainmenu::{MainMenu, MAINMENU_LOGIN, MAINMENU_REVELATIONWON};

/// C `time(NULL)`: unix seconds.
fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// The text shown once the revelation-won caption has been cleared.
const CAPTION_TEXT: &str = concat!(
    "There appears to be a serious problem\n",
    "with our Core network service providers.\n\n",
    "This may be the result of a virus.\n\n",
    "We will try to restore serBVs sasa XXXX sas\n",
    "00sd00001010001  101 1 1011 10 \n\n\n",
    "1001 101 10 101000 1010 0 10010 010\n",
    "01010 0010 1001 1010 01001010 01 01 0\n",
    "01 1010 0 10110 00 010 01 0010 001010 \n",
    "01 010 012 0120 12 10 010 010 010 010 0\n\n",
    "00100 01 01 0 01 01 101 010 0010 01 0\n",
    "1001 101 10 101000 1010 0 10010 010\n",
    "01010 0010 1001 1010 01001010 01 01 0\n",
    "01 1010 0 10110 00 010 01 0010 001010 \n",
    "01 010 01 010 10 1 10 010 010 010 010 0\n",
    "00100 01 01 0 01 01 101 010 0010 01 0\n",
    "1001 101 10 1010001 01 1 00 110 010\n",
    "01010 0010 1001 1010 01001010 01 01 0\n",
    "01 1010 0 10110 00 010 01 0010 001010 \n",
    "01101 100 1  010  0 10 1 0 010 10 110 0\n",
    "00100 01 01 0 01 01 101 010 0010 01 0\n",
    "1001 101 10 101000 1010 0 10010 010\n",
    "01010 0010 1001 1010 01001010 01 01 0\n",
    "01 1010 0 10110 00 010 01 0010 001010 \n",
    "01 010 012 0120 12 10 010 010 010 010 0\n",
    "00100 110 1001 010 01001 1000010 01 0\n",
    "1001 101 10 101000 1010 0 10010 010\n",
    "01010 0010 1001 1010 01001010 01 01 0\n",
    "01 1010 0 10110 00 010 01 0010 001010 \n",
    "01 010 012 0120 12 10 010 010 010 010 0\n",
);

/// Revelation Won Interface. Revelation has taken over the Internet.
#[derive(Debug, Default)]
pub struct RevelationWonInterface {
    pub starttime: i32,
}

impl RevelationWonInterface {
    pub fn new() -> Self {
        Self::default()
    }

    /// Static `ReturnToMainMenuClick` (MouseFn).
    pub fn return_to_main_menu_click(g: &mut Ctx, _button: &Button) {
        MainMenu::run_screen(g, MAINMENU_LOGIN);
    }
}

impl InterfaceScreen for RevelationWonInterface {
    fn create(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            g.ecl.register_button(180, 130, 450, 350, " ", "", "revelationwon");
            g.ecl.register_button_callbacks("revelationwon", Some(textbutton_draw), None, None, None);

            self.starttime = now_secs() as i32;
        }
    }
    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            g.ecl.remove_button("revelationwon");
            g.ecl.remove_button("revelationwon_mainmenu");
        }
    }
    fn update(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            if self.starttime != -1 && now_secs() > self.starttime as i64 + 4 {
                let caption_blank = g.ecl.get_button("revelationwon").is_some_and(|b| b.caption == " ");
                if caption_blank {
                    g.ecl.register_caption_change_auto("revelationwon", CAPTION_TEXT, None);
                    self.starttime = now_secs() as i32;
                } else {
                    // Exit button

                    let screenw = g.app.options.get_option_value("graphics_screenwidth");
                    let screenh = g.app.options.get_option_value("graphics_screenheight");

                    g.ecl.register_button(screenw - 40, screenh - 40, 32, 32, "", "Return to Main Menu", "revelationwon_mainmenu");
                    g.ecl.register_button_callback("revelationwon_mainmenu", Self::return_to_main_menu_click);
                    button_assignbitmaps(
                        g,
                        "revelationwon_mainmenu",
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
        g.ecl.get_button("revelationwon").is_some()
    }
    fn screen_id(&self) -> i32 {
        MAINMENU_REVELATIONWON
    }
    crate::impl_as_any!();
}
