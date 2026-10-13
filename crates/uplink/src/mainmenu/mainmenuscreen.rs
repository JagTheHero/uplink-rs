//! Port of source/mainmenu/mainmenuscreen.h + source/mainmenu/mainmenuscreen.cpp
#![allow(unused_variables, unused_imports, unused_mut, dead_code)]

// PORT: `MainMenuScreen` itself is the `InterfaceScreen` trait (`return_key_pressed` is its
// `ReturnKeyPressed`); this module keeps the base `Create`/`Remove` that subclasses call.

use crate::app::opengl_interface::{button_assignbitmap, imagebutton_draw, sx};
use crate::ctx::Ctx;

// PORT: the C++ calls here use the 6-argument `EclRegisterButton`, whose tooltip is " " ("none").

/// Base `MainMenuScreen::Create`: registers the logo ("mainmenu_background") and version
/// ("mainmenu_version") image buttons.
pub fn create(g: &mut Ctx) {
    // Background picture
    g.ecl.register_button(sx(320) - 170, 75, 425, 60, "", " ", "mainmenu_background");
    button_assignbitmap(g, "mainmenu_background", "mainmenu/uplinklogo.tif");
    g.ecl.register_button_callbacks("mainmenu_background", Some(imagebutton_draw), None, None, None);

    g.ecl.register_button(sx(320) - 165, 75 + 60, 330, 15, "", " ", "mainmenu_version");
    button_assignbitmap(g, "mainmenu_version", "mainmenu/versionnew.tif");
    g.ecl.register_button_callbacks("mainmenu_version", Some(imagebutton_draw), None, None, None);
}

/// Base `MainMenuScreen::Remove`: removes "mainmenu_background" and "mainmenu_version".
pub fn remove(g: &mut Ctx) {
    g.ecl.remove_button("mainmenu_background");
    g.ecl.remove_button("mainmenu_version");
}
