//! Port of source/interface/localinterface/localinterfacescreen.h + source/interface/localinterface/localinterfacescreen.cpp
#![allow(unused_variables, unused_imports, unused_mut, dead_code)]

// PORT: `LocalInterfaceScreen` itself is the `InterfaceScreen` trait; this module keeps its
// shared static helpers as free functions.

use crate::app::opengl_interface::{border_draw, set_colour, sy};
use crate::ctx::Ctx;
use crate::eclipse::button::Button;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::localinterface::localinterface::PANELSIZE;
use uplink_engine::gl;

/// `LocalInterfaceScreen::BackgroundDraw` (a `DrawFn`)
pub fn background_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
    gl::begin(gl::QUADS);
    set_colour(g, "PanelBackgroundA");
    gl::vertex2i(button.x, button.y + button.height);
    set_colour(g, "PanelBackgroundB");
    gl::vertex2i(button.x, button.y);
    set_colour(g, "PanelBackgroundA");
    gl::vertex2i(button.x + button.width, button.y);
    set_colour(g, "PanelBackgroundB");
    gl::vertex2i(button.x + button.width, button.y + button.height);
    gl::end();

    set_colour(g, "PanelBorder");
    border_draw(button);
}

/// Base `LocalInterfaceScreen::Create`, called by subclasses from their own `create`:
/// registers the "localint_background" panel. The caller has already checked `!is_visible`.
pub fn create(g: &mut Ctx) {
    let screenw = g.app.options.get_option_value("graphics_screenwidth");
    let paneltop = sy(100) + 30;
    // C++: int panelwidth = screenw * PANELSIZE;  (double product truncated to int)
    let panelwidth = (screenw as f64 * PANELSIZE) as i32;

    g.ecl.register_button(screenw - panelwidth - 3, paneltop, panelwidth, sy(300), "", "", "localint_background");
    g.ecl.register_button_callbacks("localint_background", Some(background_draw), None, None, None);
}

/// Base `LocalInterfaceScreen::Remove`, called by subclasses from their own `remove`:
/// removes "localint_background". The caller has already checked `is_visible`.
pub fn remove(g: &mut Ctx) {
    g.ecl.remove_button("localint_background");
}

/// `LocalInterfaceScreen::GetInterfaceScreen(screenID)`: the current local screen, asserting
/// its id. Prefer `g.interface_mut().li.screen.get_mut::<T>()` when the concrete type is known.
pub fn get_interface_screen(
    g: &mut Ctx,
    screen_id: i32,
) -> Option<&mut (dyn InterfaceScreen + 'static)> {
    let s = g.interface_mut().li.screen.get_dyn_mut()?;
    debug_assert_eq!(s.screen_id(), screen_id);
    Some(s)
}
