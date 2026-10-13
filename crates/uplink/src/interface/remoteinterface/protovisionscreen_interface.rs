//! Port of source/interface/remoteinterface/protovisionscreen_interface.h + source/interface/remoteinterface/protovisionscreen_interface.cpp

use crate::app::globals;
use crate::app::opengl_interface;
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::remoteinterface::remoteinterface::{RemoteInterface, SCREEN_PROTOVISION};
use crate::interface::remoteinterface::remoteinterfacescreen::{draw_main_title, draw_sub_title};
use crate::objref::ScreenRef;
use crate::world::computer::computerscreen::genericscreen::GenericScreen;

/// Remote Protovision screen: the page that can start the nuclear war sequence.
#[derive(Debug, Default)]
pub struct ProtovisionScreenInterface {
    pub cs: Option<ScreenRef>,
}

impl ProtovisionScreenInterface {
    /// `GetComputerScreen` (typed accessor; the C++ casts `cs`).
    pub fn get_computer_screen<'a>(&self, g: &'a Ctx) -> Option<&'a GenericScreen> {
        let cs = self.cs.as_ref()?;
        g.world().get_computer(&cs.computer)?.get_computer_screen(cs.index)?.as_generic_screen()
    }

    /// `Close` (static MouseFn)
    pub fn close(g: &mut Ctx, button: &Button) {
        let Some(nextpage) = button.name.split_whitespace().nth(1).and_then(|s| s.parse::<i32>().ok()) else {
            return;
        };

        if nextpage != -1 {
            RemoteInterface::run_screen(g, nextpage);
        }
    }

    /// `NuclearWarDraw` (static DrawFn)
    pub fn nuclear_war_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        opengl_interface::textbutton_draw(g, button, highlighted, clicked);
    }

    /// `NuclearWar` (static MouseFn)
    pub fn nuclear_war(g: &mut Ctx, button: &Button) {
        if globals::DEMOGAME {
            g.ecl.register_caption_change(&button.name, "What, in the demo? ARE YOU SERIOUS?", 1000, None);
        } else {
            RemoteInterface::run_screen(g, 4);
        }
    }
}

/// File-static `NuclearWarStart` in the C++ source.
fn nuclear_war_start(g: &mut Ctx) {
    g.ecl.register_caption_change("protovisionscreen_nuclearwar", "Global Thermonuclear War", 1000, None);
}

impl InterfaceScreen for ProtovisionScreenInterface {
    crate::impl_as_any!();

    fn create(&mut self, g: &mut Ctx) {
        if self.cs.is_none() {
            println!("ProtovisionScreenInterface::Create, tried to create when GenericScreen==NULL");
            return;
        }

        if !self.is_visible(g) {
            let Some(gs) = self.get_computer_screen(g) else {
                debug_assert!(false, "ProtovisionScreenInterface::create, no GenericScreen");
                return;
            };
            let maintitle = gs.base.maintitle.clone();
            let subtitle = gs.base.subtitle.clone();
            let nextpage = gs.nextpage;

            g.ecl.register_button(80, 60, 350, 25, &maintitle, "", "protovisionscreen_maintitle");
            g.ecl.register_button_callbacks("protovisionscreen_maintitle", Some(draw_main_title), None, None, None);
            g.ecl.register_button(80, 80, 350, 20, &subtitle, "", "protovisionscreen_subtitle");
            g.ecl.register_button_callbacks("protovisionscreen_subtitle", Some(draw_sub_title), None, None, None);

            let textmessage = "Falken's maze\n\
                               Black jack\n\
                               Gin rummy\n\
                               hearts\n\
                               bridge\n\
                               checkers\n\
                               chess\n\
                               poker\n\
                               fighter combat\n\
                               guerrilla engagement\n\
                               desert warefare\n\
                               air-to-ground actions\n\
                               theatrewide tactical warefare\n\
                               theatrewide biotoxic and chemical warefare";

            g.ecl.register_button(50, 120, 300, 245, "", "", "protovisionscreen_message");
            g.ecl.register_button_callbacks("protovisionscreen_message", Some(opengl_interface::textbutton_draw), None, None, None);
            g.ecl.register_caption_change("protovisionscreen_message", textmessage, 5000, Some(nuclear_war_start));

            g.ecl.register_button(50, 340, 300, 20, "", "", "protovisionscreen_nuclearwar");
            g.ecl.register_button_callbacks(
                "protovisionscreen_nuclearwar",
                Some(Self::nuclear_war_draw),
                Some(Self::nuclear_war),
                Some(opengl_interface::button_click),
                Some(opengl_interface::button_highlight),
            );

            let width = 80;
            let name = format!("protovisionscreen_close {}", nextpage);
            g.ecl.register_button(320 - width / 2, 370, width, 20, "Close", "Close this screen", &name);
            g.ecl.register_button_callback(&name, Self::close);
        }
    }

    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            let Some(nextpage) = self.get_computer_screen(g).map(|gs| gs.nextpage) else {
                debug_assert!(false, "ProtovisionScreenInterface::remove, no GenericScreen");
                return;
            };

            g.ecl.remove_button("protovisionscreen_maintitle");
            g.ecl.remove_button("protovisionscreen_subtitle");

            g.ecl.remove_button("protovisionscreen_message");
            g.ecl.remove_button("protovisionscreen_nuclearwar");

            let name = format!("protovisionscreen_close {}", nextpage);
            g.ecl.remove_button(&name);
        }
    }

    fn update(&mut self, _g: &mut Ctx) {}

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("protovisionscreen_message").is_some()
    }

    fn screen_id(&self) -> i32 {
        SCREEN_PROTOVISION
    }

    fn return_key_pressed(&mut self, g: &mut Ctx) -> bool {
        let nextpage = self.get_computer_screen(g).map(|gs| gs.nextpage);
        if let Some(nextpage) = nextpage {
            if nextpage != -1 {
                RemoteInterface::run_screen(g, nextpage);
            }
        }

        true
    }

    fn set_computer_screen(&mut self, cs: ScreenRef) {
        self.cs = Some(cs);
    }

    fn computer_screen_ref(&self) -> Option<&ScreenRef> {
        self.cs.as_ref()
    }
}
