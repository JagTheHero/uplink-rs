//! Port of source/mainmenu/firsttimeloading_interface.h + source/mainmenu/firsttimeloading_interface.cpp

use crate::app::opengl_interface::textbutton_draw;
use crate::ctx::Ctx;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::mainmenu::mainmenu::MAINMENU_FIRSTLOAD;

/// Loading screen shown to first time players.
#[derive(Debug, Default)]
pub struct FirstTimeLoadingInterface {}

impl FirstTimeLoadingInterface {
    pub fn new() -> Self {
        Self::default()
    }
}

impl InterfaceScreen for FirstTimeLoadingInterface {
    fn create(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            // Play some music

            if g.app.options.is_option_equal_to("sound_musicenabled", 1) {
                //SgPlaylist_Play ("ambient");
                uplink_engine::sound::playlist_play("main");
            }

            let screenw = g.app.options.get_option_value("graphics_screenwidth");
            let screenh = g.app.options.get_option_value("graphics_screenheight");

            g.ecl.register_button(screenw - 370, screenh - 30, 370, 20, "", "", "firsttimeloading_text");
            g.ecl.register_button_callbacks("firsttimeloading_text", Some(textbutton_draw), None, None, None);
        }
    }
    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            g.ecl.remove_button("firsttimeloading_text");
        }
    }
    fn update(&mut self, _g: &mut Ctx) {}
    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("firsttimeloading_text").is_some()
    }
    fn screen_id(&self) -> i32 {
        MAINMENU_FIRSTLOAD
    }
    crate::impl_as_any!();
}
