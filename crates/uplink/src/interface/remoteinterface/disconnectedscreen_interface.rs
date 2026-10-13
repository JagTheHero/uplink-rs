//! Port of source/interface/remoteinterface/disconnectedscreen_interface.h + source/interface/remoteinterface/disconnectedscreen_interface.cpp

use crate::app::opengl_interface::textbutton_draw;
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::remoteinterface::remoteinterface::{RemoteInterface, SCREEN_DISCONNECTEDSCREEN};
use crate::interface::remoteinterface::remoteinterfacescreen::{draw_main_title, draw_sub_title};
use crate::objref::ScreenRef;
use crate::world::computer::computerscreen::disconnectedscreen::DisconnectedScreen;

/// Disconnected screen: tells the player the connection was dropped and lists any logins that were revoked.
#[derive(Debug, Default)]
pub struct DisconnectedScreenInterface {
    pub cs: Option<ScreenRef>,
}

/// `RunScreen(nextpage)` when the page is not -1.
fn run_next_page(g: &mut Ctx, nextpage: i32) {
    if nextpage != -1 {
        RemoteInterface::run_screen(g, nextpage);
    }
}

/// The eclipse name of the OK button for a page: `"disconnectedscreen_click %d"`.
fn click_button_name(nextpage: i32) -> String {
    format!("disconnectedscreen_click {}", nextpage)
}

impl DisconnectedScreenInterface {
    /// `Click` (static MouseFn): runs the page named in the button (`disconnectedscreen_click N`).
    pub fn click(g: &mut Ctx, button: &Button) {
        // sscanf(button->name, "disconnectedscreen_click %d", &nextpage)
        let nextpage = button
            .name
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(-1);

        run_next_page(g, nextpage);
    }

    /// `GetComputerScreen` (typed accessor; the C++ casts `cs`).
    pub fn get_computer_screen<'a>(&self, g: &'a Ctx) -> Option<&'a DisconnectedScreen> {
        let r = self.cs.as_ref()?;
        g.world().get_computer(&r.computer)?.get_computer_screen(r.index)?.as_disconnected_screen()
    }
}

impl InterfaceScreen for DisconnectedScreenInterface {
    crate::impl_as_any!();

    /// `Create(ComputerScreen *newcs)` body. The C++ no-argument `Create()` overload maps to this
    /// too, since `cs` is already set by `set_computer_screen`.
    fn create(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            return;
        }

        let Some(ds) = self.get_computer_screen(g).cloned() else { return };

        g.ecl.register_button(80, 60, 350, 25, &ds.base.maintitle, "", "disconnectedscreen_maintitle");
        g.ecl.register_button_callbacks("disconnectedscreen_maintitle", Some(draw_main_title), None, None, None);
        g.ecl.register_button(80, 80, 350, 20, &ds.base.subtitle, "", "disconnectedscreen_subtitle");
        g.ecl.register_button_callbacks("disconnectedscreen_subtitle", Some(draw_sub_title), None, None, None);

        g.ecl.register_button(50, 120, 400, 30, "", "", "disconnectedscreen_message");
        g.ecl.register_button_callbacks("disconnectedscreen_message", Some(textbutton_draw), None, None, None);
        g.ecl.register_caption_change(
            "disconnectedscreen_message",
            ds.textmessage.as_deref().unwrap_or(""),
            2000,
            None,
        );

        let loginslost = DisconnectedScreen::logins_lost();
        if !loginslost.is_empty() {
            g.ecl.register_button(50, 180, 400, 200, "", "", "disconnectedscreen_loginslost");
            g.ecl.register_button_callbacks("disconnectedscreen_loginslost", Some(textbutton_draw), None, None, None);

            let mut text = String::from("Your username and password was revoked on these systems:\n\n");

            for ip in &loginslost {
                // PORT: the C++ asserts that the VLocation and its computer exist; a missing one is skipped here.
                let name = g
                    .world()
                    .get_vlocation(ip)
                    .and_then(|vl| vl.get_computer(g.world()))
                    .map(|comp| comp.name.clone());

                if let Some(name) = name {
                    text.push_str(&format!("    - {}\n", name));
                }
            }

            g.ecl.register_caption_change("disconnectedscreen_loginslost", &text, 2000, None);

            DisconnectedScreen::clear_logins_lost();
        }

        let width = 100;
        let name = click_button_name(ds.nextpage);
        g.ecl.register_button(320 - width / 2, 380, width, 20, "OK", "Click to close this screen", &name);
        g.ecl.register_button_callback(&name, Self::click);
    }

    fn remove(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            return;
        }

        g.ecl.remove_button("disconnectedscreen_message");
        g.ecl.remove_button("disconnectedscreen_loginslost");

        g.ecl.remove_button("disconnectedscreen_maintitle");
        g.ecl.remove_button("disconnectedscreen_subtitle");

        if let Some(nextpage) = self.get_computer_screen(g).map(|ds| ds.nextpage) {
            g.ecl.remove_button(&click_button_name(nextpage));
        }
    }

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("disconnectedscreen_message").is_some()
    }

    /// `ReturnKeyPressed`: runs the screen's next page, then always reports the key as handled.
    fn return_key_pressed(&mut self, g: &mut Ctx) -> bool {
        if let Some(nextpage) = self.get_computer_screen(g).map(|ds| ds.nextpage) {
            run_next_page(g, nextpage);
        }

        true
    }

    fn screen_id(&self) -> i32 {
        SCREEN_DISCONNECTEDSCREEN
    }

    fn set_computer_screen(&mut self, cs: ScreenRef) {
        self.cs = Some(cs);
    }

    fn computer_screen_ref(&self) -> Option<&ScreenRef> {
        self.cs.as_ref()
    }
}
