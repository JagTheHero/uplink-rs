//! Port of source/network/interfaces/clientcommsinterface.h + source/network/interfaces/clientcommsinterface.cpp

use std::rc::Rc;

use uplink_engine::gl;

use crate::app::opengl_interface::{button_assignbitmap, button_assignbitmaps, imagebutton_drawtextured};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::localinterface::worldmap::rectangle::MapRectangle;
use crate::interface::localinterface::worldmap::worldmap_layout::WorldMapLayout;
use crate::mainmenu::mainmenu::{MainMenu, MAINMENU_LOGIN};
use crate::network::interfaces::networkscreen::NetworkScreen;
use crate::network::network::NETWORK_CLIENT;
use crate::network::networkclient::{NetworkClient, CLIENT_COMMS, CLIENT_NONE};
use crate::tosser::BTree;
use crate::world::vlocation::VLocation;

// PORT: C++ file-scope `static int STIPPLE[]`: dash patterns for the connection lines, two frames each
const STIPPLE: [i32; 8] = [
    0x1111, 0x1111, 0x2222, 0x2222, 0x4444, 0x4444, 0x8888, 0x8888,
];

/// The C++ class statics (and file-scope statics) of ClientCommsInterface. In C++ they outlive every
/// screen object, so the client's data survives leaving the comms screen and coming back (the server
/// does not resend it). `remove` parks them in `NetworkClient::comms_statics`, `create` takes them back.
// PORT: `layout` is not kept: it is deleted by the destructor and made again by `Create`.
pub struct ClientCommsStatics {
    stipplepattern: i32,
    stippleindex: i32,
    lastupdate: i32,
    locations: BTree<VLocation>,
    connection: Vec<String>,
    traceprogress: i32,
}

/// Global communications screen: the world map with the client's connection drawn on it.
// PORT: the C++ class statics (stipplepattern, layout, locations, connection, traceprogress) are
// instance fields here, carried from one screen object to the next through `ClientCommsStatics`.
// Only one comms screen exists at a time (NetworkClient::screen).
#[derive(Default)]
pub struct ClientCommsInterface {
    /// C++ static `stipplepattern`: pattern used for the connection lines
    stipplepattern: i32,
    /// C++ file-scope `stippleindex`
    stippleindex: i32,
    /// C++ file-scope `lastupdate`: eclipse time of the last map redraw
    lastupdate: i32,
    /// C++ static `layout` (owned; freed by the destructor)
    layout: Option<Box<WorldMapLayout>>,
    /// C++ static `locations`: known locations keyed by IP
    pub locations: BTree<VLocation>,
    /// C++ static `connection`: IPs making up the player's connection, in order
    pub connection: Vec<String>,
    /// C++ static `traceprogress`
    pub traceprogress: i32,
}

impl ClientCommsInterface {
    /// C++ constructor
    pub fn new() -> Self {
        ClientCommsInterface {
            stipplepattern: STIPPLE[0],
            ..Default::default()
        }
    }

    /// C++ `locations.PutData(ip, vl)`
    pub fn put_location(&mut self, ip: &str, vl: VLocation) {
        self.locations.put_data(ip, vl);
    }

    /// `CycleStipplePattern`: advance the dash pattern used for the connection lines
    pub fn cycle_stipple_pattern(&mut self) {
        self.stippleindex += 1;
        if self.stippleindex > 7 {
            self.stippleindex = 0;
        }
        self.stipplepattern = STIPPLE[self.stippleindex as usize];
    }

    /// `MainMenuClick` (close button): back to the login screen
    pub fn main_menu_click(g: &mut Ctx, _button: &Button) {
        debug_assert!(g.app.network.status == NETWORK_CLIENT, "UplinkAssert: NETWORK_CLIENT");

        NetworkClient::set_client_type(g, CLIENT_NONE);
        MainMenu::run_screen(g, MAINMENU_LOGIN);
    }

    /// `LargeMapDraw`: draws the map background, the connection lines and the location labels
    pub fn large_map_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        // Draw the background bitmap
        imagebutton_drawtextured(g, button, highlighted, clicked);

        let screenheight = g.app.options.get_option_value("graphics_screenheight");
        gl::scissor(button.x, screenheight - (button.y + button.height), button.width, button.height);
        gl::enable(gl::SCISSOR_TEST);

        gl::line_width(2.0);
        gl::color4f(1.0, 1.0, 1.0, 1.0);

        let cci = g
            .app
            .network
            .client
            .screen
            .get_mut::<ClientCommsInterface>()
            .expect("UplinkAssert: comms screen");

        gl::line_stipple(2, cci.stipplepattern as u16);
        cci.cycle_stipple_pattern();
        gl::enable(gl::LINE_STIPPLE);

        // Draw the dotted lines
        gl::begin(gl::LINE_STRIP);

        for li in 0..cci.connection.len() {
            let ip = &cci.connection[li];

            if let Some(vl) = cci.locations.get(ip) {
                gl::vertex2i(button.x + vl.x, button.y + vl.y);

                if cci.traceprogress == (cci.locations.size() - li as i32 - 1) {
                    gl::color4f(1.0, 0.0, 0.0, 1.0);
                    gl::vertex2f((button.x + vl.x) as f32, (button.y + vl.y) as f32);
                }
            }
        }

        gl::end();

        gl::disable(gl::LINE_STIPPLE);

        // C++ also drew dots and text per location here, but that code is commented out.
        cci.draw_all_objects();

        gl::disable(gl::SCISSOR_TEST);
    }

    /// `DrawAllObjects`: labels and points for the locations in the connection
    pub fn draw_all_objects(&mut self) {
        let layout = self.layout.as_mut().expect("UplinkAssert: comms layout");
        layout.full_layout_labels();

        for i in 0..layout.labels.len() {
            let label = &layout.labels[i];
            let point = layout.locations.get(i).expect("UplinkAssert: label point");

            if connected(&self.connection, &self.locations, label.get_caption()) {
                label.draw(0, 0, 1.0);
                point.draw(0, 0, 1.0);
            }
        }
    }

    /// `InConnection`: is a computer with this name in the connection
    pub fn in_connection(&self, computer_name: &str) -> bool {
        connected(&self.connection, &self.locations, computer_name)
    }

    /// `LayoutLabels`: rebuild the world map layout from `locations`
    pub fn layout_labels(&mut self) {
        let alllinks: Vec<(i32, i32, String)> = self
            .locations
            .values()
            .map(|vl| (20 + vl.x, 50 + vl.y, vl.computer.clone()))
            .collect();

        let layout = self.layout.as_mut().expect("UplinkAssert: comms layout");
        layout.reset();

        for (x, y, computer) in &alllinks {
            layout.add_location(*x, *y, computer);
        }
    }
}

/// `InConnection` over the two fields it reads, so the layout can be borrowed mutably alongside it.
fn connected(connection: &[String], locations: &BTree<VLocation>, computer_name: &str) -> bool {
    connection.iter().any(|ip| {
        let vl = locations.get(ip).expect("UplinkAssert: connection location");
        vl.computer == computer_name
    })
}

impl NetworkScreen for ClientCommsInterface {}

impl InterfaceScreen for ClientCommsInterface {
    /// `Create`: close button and the world map background
    fn create(&mut self, g: &mut Ctx) {
        // The C++ statics are still there from the previous comms screen
        if let Some(statics) = g.app.network.client.comms_statics.take() {
            self.stipplepattern = statics.stipplepattern;
            self.stippleindex = statics.stippleindex;
            self.lastupdate = statics.lastupdate;
            self.locations = statics.locations;
            self.connection = statics.connection;
            self.traceprogress = statics.traceprogress;
        }

        if !self.is_visible(g) {
            let screenw = g.app.options.get_option_value("graphics_screenwidth");

            // Create a close button
            g.ecl.register_button(3, 4, 13, 13, "", "Go to the Main Menu", "comms_mainmenu");
            button_assignbitmaps(g, "comms_mainmenu", "close.tif", "close_h.tif", "close_c.tif");
            g.ecl.register_button_callback("comms_mainmenu", Self::main_menu_click);

            // Create the worldmap graphic
            let fullsize_x = screenw - 40;
            let fullsize_y = (316.0 * (fullsize_x as f64 / 595.0)) as i32;

            // Create the background bitmap
            self.layout = Some(Box::new(WorldMapLayout::new(&MapRectangle::new(20, 50, fullsize_x, fullsize_y))));

            g.ecl.register_button(20, 50, fullsize_x, fullsize_y, "", "", "comms_largemap");
            button_assignbitmap(g, "comms_largemap", "worldmaplarge.tif");

            // C++ `ScaleToOpenGL()`. The engine's Image::scale_to_opengl does nothing, and the image is
            // skipped here when the Rc is shared, which loses nothing.
            if let Some(image) = g
                .ecl
                .get_button_mut("comms_largemap")
                .and_then(|b| b.image_standard.as_mut())
                .and_then(Rc::get_mut)
            {
                image.scale_to_opengl();
            }

            g.ecl.register_button_callbacks("comms_largemap", Some(Self::large_map_draw), None, None, None);
        }
    }

    /// `Remove`: remove the buttons made by `create`
    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            g.ecl.remove_button("comms_mainmenu");
            g.ecl.remove_button("comms_largemap");
        }

        // C++ statics outlive this screen object: keep them for the next one
        g.app.network.client.comms_statics = Some(ClientCommsStatics {
            stipplepattern: self.stipplepattern,
            stippleindex: self.stippleindex,
            lastupdate: self.lastupdate,
            locations: std::mem::take(&mut self.locations),
            connection: std::mem::take(&mut self.connection),
            traceprogress: self.traceprogress,
        });
    }

    /// `Update`: marks the map dirty about every 200 ms so the connection lines animate
    fn update(&mut self, g: &mut Ctx) {
        let timesincelastupdate = g.ecl.get_accurate_time() - self.lastupdate;

        if timesincelastupdate > 200 {
            g.ecl.dirty_button("comms_largemap");
            self.lastupdate = g.ecl.get_accurate_time();
        }
    }

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("comms_mainmenu").is_some()
    }

    fn screen_id(&self) -> i32 {
        CLIENT_COMMS
    }

    crate::impl_as_any!();
}
