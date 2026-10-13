//! Port of source/interface/remoteinterface/nearestgatewayscreen_interface.h + source/interface/remoteinterface/nearestgatewayscreen_interface.cpp

use uplink_engine::{gl, text};

use crate::app::opengl_interface::{self, sx, sy};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::game::data::data::{NUM_PHYSICALGATEWAYLOCATIONS, PHYSICALGATEWAYLOCATIONS};
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::remoteinterface::remoteinterface::{RemoteInterface, SCREEN_NEARESTGATEWAY};
use crate::interface::remoteinterface::remoteinterfacescreen::draw_main_title;
use crate::objref::ScreenRef;
use crate::world::computer::computerscreen::genericscreen::GenericScreen;

/// Remote nearest-gateway screen: a map of physical gateway locations to pick from.
#[derive(Debug, Default)]
pub struct NearestGatewayScreenInterface {
    pub cs: Option<ScreenRef>,
}

/// `sscanf ( button->name, "%s %d", unused, &index )`: the gateway location index in the button name.
fn location_index(name: &str) -> Option<usize> {
    let index: i32 = name.split_whitespace().nth(1)?.parse().ok()?;
    debug_assert!(index < NUM_PHYSICALGATEWAYLOCATIONS);
    PHYSICALGATEWAYLOCATIONS.get(usize::try_from(index).ok()?).map(|_| index as usize)
}

impl NearestGatewayScreenInterface {
    /// `GetComputerScreen` (typed accessor; the C++ casts `cs`).
    pub fn get_computer_screen<'a>(&self, g: &'a Ctx) -> Option<&'a GenericScreen> {
        let cs = self.cs.as_ref()?;
        g.world().get_computer(&cs.computer)?.get_computer_screen(cs.index)?.as_generic_screen()
    }

    /// `DrawBlack` (static DrawFn)
    pub fn draw_black(g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        opengl_interface::clear_draw(g, button.x, button.y, button.width, button.height);
    }

    /// `DrawLocation` (static DrawFn)
    pub fn draw_location(_g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        gl::color4f(1.0, 1.0, 1.0, 1.0);

        gl::begin(gl::QUADS);
        gl::vertex2i(button.x, button.y);
        gl::vertex2i(button.x + 7, button.y);
        gl::vertex2i(button.x + 7, button.y + 7);
        gl::vertex2i(button.x, button.y + 7);
        gl::end();

        // Write some text

        let Some(index) = location_index(&button.name) else {
            return;
        };
        let pgl = &PHYSICALGATEWAYLOCATIONS[index];

        text::draw_text_default(button.x - 10, button.y + 20, pgl.city);
    }

    /// `ClickLocation` (static MouseFn)
    pub fn click_location(g: &mut Ctx, button: &Button) {
        // Set the players physical location

        let Some(index) = location_index(&button.name) else {
            return;
        };
        let pgl = &PHYSICALGATEWAYLOCATIONS[index];

        let localhost = g.player().localhost.clone();
        if let Some(vl) = g.world_mut().get_vlocation_mut(&localhost) {
            vl.set_plocation(pgl.x, pgl.y);
        }

        // Go on to the next screen

        let Some(nextpage) = g
            .interface()
            .ri
            .screen
            .get::<Self>()
            .and_then(|thisint| thisint.get_computer_screen(g))
            .map(|gs| gs.nextpage)
        else {
            debug_assert!(false, "NearestGatewayScreenInterface::click_location, no GenericScreen");
            return;
        };

        RemoteInterface::run_screen(g, nextpage);
    }
}

impl InterfaceScreen for NearestGatewayScreenInterface {
    crate::impl_as_any!();

    fn create(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            let Some(gs) = self.get_computer_screen(g) else {
                debug_assert!(false, "NearestGatewayScreenInterface::create, no GenericScreen");
                return;
            };
            let subtitle = gs.base.subtitle.clone();

            g.ecl.register_button(40, 10, 350, 25, &subtitle, "", "nearestgateway_maintitle");
            g.ecl.register_button_callbacks("nearestgateway_maintitle", Some(draw_main_title), None, None, None);

            // Black out the computer picture

            g.ecl.register_button(470, 280, 140, 140, " ", " ", "nearestgateway_black");
            g.ecl.register_button_callbacks("nearestgateway_black", Some(Self::draw_black), None, None, None);

            // Work out the size/ratios of the map

            let screenw = g.app.options.get_option_value("graphics_screenwidth");
            let _screenh = g.app.options.get_option_value("graphics_screenheight");

            let x1 = 23;
            let y1 = 50;
            let full_size_x = screenw - x1 * 2;
            let full_size_y = (316.0 * (full_size_x as f64 / 595.0)) as i32;

            // Create the large map graphic

            g.ecl.register_button(x1, y1, full_size_x, full_size_y, "", "", "nearestgateway_largemap");
            opengl_interface::button_assignbitmap(g, "nearestgateway_largemap", "worldmaplarge.tif");
            g.ecl.register_button_callbacks(
                "nearestgateway_largemap",
                Some(opengl_interface::imagebutton_drawtextured),
                None,
                None,
                None,
            );

            // Create a button for each gateway

            for (i, pgl) in PHYSICALGATEWAYLOCATIONS.iter().enumerate() {
                let bname = format!("nearestgateway_location {}", i);
                let tooltip = format!("Select {} as your local Gateway", pgl.city);

                g.ecl.register_button(x1 + sx(pgl.x), y1 + sy(pgl.y), 7, 7, pgl.city, &tooltip, &bname);
                g.ecl.register_button_callbacks(
                    &bname,
                    Some(Self::draw_location),
                    Some(Self::click_location),
                    Some(opengl_interface::button_click),
                    Some(opengl_interface::button_highlight),
                );
            }

            // Create some helpfull text

            g.ecl.register_button(50, y1 + full_size_y + 20, 400, 70, " ", " ", "nearestgateway_helptext");
            g.ecl.register_button_callbacks("nearestgateway_helptext", Some(opengl_interface::textbutton_draw), None, None, None);
            g.ecl.register_caption_change(
                "nearestgateway_helptext",
                "Uplink Corporation has offices all around the world.\n\
                 Please select a nearby location for your Gateway.\n\
                 You will dial into this location when you use Uplink.",
                2000,
                None,
            );
        }
    }

    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            g.ecl.remove_button("nearestgateway_maintitle");
            g.ecl.remove_button("nearestgateway_black");
            g.ecl.remove_button("nearestgateway_largemap");

            for i in 0..NUM_PHYSICALGATEWAYLOCATIONS {
                let bname = format!("nearestgateway_location {}", i);
                g.ecl.remove_button(&bname);
            }

            g.ecl.remove_button("nearestgateway_helptext");
        }
    }

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("nearestgateway_largemap").is_some()
    }

    fn screen_id(&self) -> i32 {
        SCREEN_NEARESTGATEWAY
    }

    fn return_key_pressed(&mut self, _g: &mut Ctx) -> bool {
        false
    }

    fn set_computer_screen(&mut self, cs: ScreenRef) {
        self.cs = Some(cs);
    }

    fn computer_screen_ref(&self) -> Option<&ScreenRef> {
        self.cs.as_ref()
    }
}
