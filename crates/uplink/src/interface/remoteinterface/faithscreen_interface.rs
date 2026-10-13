//! Port of source/interface/remoteinterface/faithscreen_interface.h + source/interface/remoteinterface/faithscreen_interface.cpp

use crate::app::opengl_interface::{button_assignbitmaps, textbutton_draw};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::remoteinterface::remoteinterface::{RemoteInterface, SCREEN_FAITHSCREEN};
use crate::interface::remoteinterface::remoteinterfacescreen::{draw_main_title, draw_sub_title};
use crate::objref::ScreenRef;
use crate::world::computer::computerscreen::genericscreen::GenericScreen;

/// Faith screen: shows the progress of Faith versus Revelation (version numbers from the plot generator).
#[derive(Debug, Default)]
pub struct FaithScreenInterface {
    pub cs: Option<ScreenRef>,
}

/// `RunScreen(gs->nextpage)` when the generic screen has a next page (not -1).
fn run_next_page(g: &mut Ctx, nextpage: Option<i32>) {
    if let Some(nextpage) = nextpage {
        if nextpage != -1 {
            RemoteInterface::run_screen(g, nextpage);
        }
    }
}

impl FaithScreenInterface {
    /// `CloseClick` (static MouseFn): runs the generic screen's next page.
    /// PORT: the C++ reads the current computer screen from the remote interface rather than `cs`.
    pub fn close_click(g: &mut Ctx, _button: &Button) {
        let nextpage = RemoteInterface::get_computer_screen(g)
            .and_then(|s| s.as_generic_screen())
            .map(|gs| gs.nextpage);
        run_next_page(g, nextpage);
    }

    /// `GetComputerScreen` (typed accessor; the C++ casts `cs`).
    pub fn get_computer_screen<'a>(&self, g: &'a Ctx) -> Option<&'a GenericScreen> {
        let r = self.cs.as_ref()?;
        g.world().get_computer(&r.computer)?.get_computer_screen(r.index)?.as_generic_screen()
    }
}

impl InterfaceScreen for FaithScreenInterface {
    crate::impl_as_any!();

    /// `Create(ComputerScreen *newcs)` body. The C++ no-argument `Create()` overload maps to this
    /// too, since `cs` is already set by `set_computer_screen`.
    fn create(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            return;
        }

        let Some(gs) = self.get_computer_screen(g).cloned() else { return };

        // Create the titles

        g.ecl.register_button(80, 60, 350, 25, &gs.base.maintitle, "", "faithscreen_maintitle");
        g.ecl.register_button_callbacks("faithscreen_maintitle", Some(draw_main_title), None, None, None);
        g.ecl.register_button(80, 80, 350, 20, &gs.base.subtitle, "", "faithscreen_subtitle");
        g.ecl.register_button_callbacks("faithscreen_subtitle", Some(draw_sub_title), None, None, None);

        // Create the information boxes

        let faith = format!("Version {:.1}", g.world().plotgenerator.get_version_faith());
        let revelation = format!("Version {:.1}", g.world().plotgenerator.get_version_revelation());

        g.ecl.register_button(100, 150, 100, 30, "Faith", "", "faithscreen_faithtitle");
        g.ecl.register_button(100, 200, 100, 30, "Revelation", "", "faithscreen_revelationtitle");

        g.ecl.register_button(250, 150, 200, 30, &faith, "", "faithscreen_faith");
        g.ecl.register_button(250, 200, 200, 30, &revelation, "", "faithscreen_revelation");

        g.ecl.register_button_callbacks("faithscreen_faithtitle", Some(draw_sub_title), None, None, None);
        g.ecl.register_button_callbacks("faithscreen_faith", Some(draw_main_title), None, None, None);
        g.ecl.register_button_callbacks("faithscreen_revelationtitle", Some(draw_sub_title), None, None, None);
        g.ecl.register_button_callbacks("faithscreen_revelation", Some(draw_main_title), None, None, None);

        g.ecl.register_button(90, 260, 300, 110, "", "", "faithscreen_text");
        g.ecl.register_button_callbacks("faithscreen_text", Some(textbutton_draw), None, None, None);
        g.ecl.register_caption_change(
            "faithscreen_text",
            "This information is based on unofficial reports from the Web.  No guarantee is given as to the accuracy of this information.\n\nIf you have any information which may help to keep this up to date, please send it to internal@Uplink.net.",
            2000,
            None,
        );

        // Create the close button

        g.ecl.register_button(421, 121, 13, 13, "", "Close the Faith Progress Screen", "faithscreen_close");
        button_assignbitmaps(g, "faithscreen_close", "close.tif", "close_h.tif", "close_c.tif");
        g.ecl.register_button_callback("faithscreen_close", Self::close_click);
    }

    fn remove(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            return;
        }

        for name in [
            "faithscreen_maintitle",
            "faithscreen_subtitle",
            "faithscreen_faithtitle",
            "faithscreen_faith",
            "faithscreen_revelationtitle",
            "faithscreen_revelation",
            "faithscreen_text",
            "faithscreen_close",
        ] {
            g.ecl.remove_button(name);
        }
    }

    fn update(&mut self, _g: &mut Ctx) {}

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("faithscreen_maintitle").is_some()
    }

    /// `EscapeKeyPressed`: closes the screen and always reports the key as handled.
    fn escape_key_pressed(&mut self, g: &mut Ctx) -> bool {
        let nextpage = self.get_computer_screen(g).map(|gs| gs.nextpage);
        run_next_page(g, nextpage);
        true
    }

    fn screen_id(&self) -> i32 {
        SCREEN_FAITHSCREEN
    }

    fn set_computer_screen(&mut self, cs: ScreenRef) {
        self.cs = Some(cs);
    }

    fn computer_screen_ref(&self) -> Option<&ScreenRef> {
        self.cs.as_ref()
    }
}
