//! Port of source/interface/remoteinterface/remoteinterfacescreen.h + source/interface/remoteinterface/remoteinterfacescreen.cpp
#![allow(unused_variables, unused_imports, unused_mut, dead_code)]

// PORT: `RemoteInterfaceScreen` itself is the `InterfaceScreen` trait (`set_computer_screen`
// replaces `Create(ComputerScreen *)`; the stored `cs` becomes `Option<ScreenRef>` in each
// subclass); this module keeps its shared static helpers as free functions.

use uplink_engine::{gl, text};

use crate::app::opengl::screen_h;
use crate::app::opengl_interface::set_colour;
use crate::ctx::Ctx;
use crate::eclipse::button::Button;
use crate::interface::interfacescreen::InterfaceScreen;

/// `RemoteInterfaceScreen::DrawMainTitle` (a `DrawFn`)
pub fn draw_main_title(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
    let screenheight = screen_h();
    gl::scissor(button.x, screenheight - (button.y + button.height), button.width, button.height);
    gl::enable(gl::SCISSOR_TEST);

    set_colour(g, "MenuText");
    let ypos = (button.y + button.height / 2) + 5;
    text::draw_text_styled(button.x, ypos, &button.caption, text::HELVETICA_18);

    gl::disable(gl::SCISSOR_TEST);
}

/// `RemoteInterfaceScreen::DrawSubTitle` (a `DrawFn`)
pub fn draw_sub_title(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
    let screenheight = screen_h();
    gl::scissor(button.x, screenheight - (button.y + button.height), button.width, button.height);
    gl::enable(gl::SCISSOR_TEST);

    set_colour(g, "DefaultText");
    let ypos = (button.y + button.height / 2) + 5;
    text::draw_text_styled(button.x, ypos, &button.caption, text::HELVETICA_12);

    gl::disable(gl::SCISSOR_TEST);
}

/// `RemoteInterfaceScreen::GetInterfaceScreen(screenID)`: the current remote screen, asserting
/// its id. Prefer `g.interface_mut().ri.screen.get_mut::<T>()` when the concrete type is known.
pub fn get_interface_screen(
    g: &mut Ctx,
    screen_id: i32,
) -> Option<&mut (dyn InterfaceScreen + 'static)> {
    let s = g.interface_mut().ri.screen.get_dyn_mut()?;
    debug_assert_eq!(s.screen_id(), screen_id);
    Some(s)
}
