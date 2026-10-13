//! Port of source/mainmenu/loading_interface.h + source/mainmenu/loading_interface.cpp

use std::rc::Rc;

use crate::app::miscutils::{rand_i32, RAND_MAX};
use crate::app::opengl_interface::{button_assignbitmap, imagebutton_drawtextured, sx, sy, text_draw};
use crate::ctx::Ctx;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::mainmenu::mainmenu::MAINMENU_LOADING;

/// Loading screen interface.
#[derive(Debug, Default)]
pub struct LoadingInterface {}

impl LoadingInterface {
    pub fn new() -> Self {
        Self::default()
    }
}

impl InterfaceScreen for LoadingInterface {
    fn create(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            // The C++ loaded and closed loading.dat here; it is loaded at start up instead
            // (RsCloseArchive's BTree::RemoveData crashed on repeated reloads), so nothing is done here.

            let screenw = g.app.options.get_option_value("graphics_screenwidth");
            let screenh = g.app.options.get_option_value("graphics_screenheight");

            // Decide which graphic to load up
            let filename = g.app.options.theme_filename("loading/filenames.txt");
            // UplinkAssert(file)
            let text = uplink_engine::archive::read_text(&filename).expect("loading/filenames.txt could not be opened");
            // fscanf ( "%s\n" ) reads one whitespace separated word at a time.
            let filenames: Vec<&str> = text.split_whitespace().collect();
            let value = rand_i32() as f32 / RAND_MAX as f32;
            let pictureindex = (value * filenames.len() as f32) as usize;
            // PORT: the C++ indexed past the end when rand() returned RAND_MAX exactly; use the last picture instead.
            let picture = *filenames
                .get(pictureindex)
                .or(filenames.last())
                .expect("loading/filenames.txt lists no pictures");

            g.ecl.register_button(0, 0, screenw, screenh, "", "", "loading_background");
            button_assignbitmap(g, "loading_background", picture);
            // EclGetButton ( "loading_background" )->image_standard->Scale ( 512, 512 )
            if let Some(image) = g
                .ecl
                .get_button_mut("loading_background")
                .and_then(|b| b.image_standard.as_mut())
                .and_then(Rc::get_mut)
            {
                image.scale(512, 512);
            }
            g.ecl.register_button_callbacks("loading_background", Some(imagebutton_drawtextured), None, None, None);

            g.ecl.register_button(sx(440), sy(450), 200, 15, "Connecting to GATEWAY...", "", "loading_text");
            g.ecl.register_button_callbacks("loading_text", Some(text_draw), None, None, None);
        }
    }
    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            g.ecl.remove_button("loading_background");
            g.ecl.remove_button("loading_text");
        }
    }
    fn update(&mut self, _g: &mut Ctx) {}
    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("loading_background").is_some()
    }
    fn screen_id(&self) -> i32 {
        MAINMENU_LOADING
    }
    crate::impl_as_any!();
}
