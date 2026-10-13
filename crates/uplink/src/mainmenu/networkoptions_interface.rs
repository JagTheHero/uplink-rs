//! Port of source/mainmenu/networkoptions_interface.h + source/mainmenu/networkoptions_interface.cpp

use std::cell::Cell;

use crate::app::opengl_interface::{button_click, button_draw, button_highlight, clear_draw, textbutton_draw};
use crate::ctx::Ctx;
use crate::eclipse::{Button, MOVE_STRAIGHTLINE};
use crate::interface::interfacescreen::InterfaceScreen;
use crate::mainmenu::mainmenu::{MainMenu, MAINMENU_NETWORKOPTIONS, MAINMENU_OPTIONS};
use crate::mainmenu::mainmenuscreen;
use crate::network::clientconnection::ClientConnection;
use crate::network::network::{NETWORK_CLIENT, NETWORK_NONE, NETWORK_SERVER};

/// `button->SetCaption(caption)` inside a draw callback. The engine draws from a snapshot, so the
/// real button is updated by name (as the C++ pointer did) and the copy to draw with is returned.
fn caption_snapshot(g: &mut Ctx, button: &Button, caption: &str) -> Button {
    if let Some(real) = g.ecl.get_button_mut(&button.name) {
        real.set_caption(caption);
    }
    let mut snapshot = button.clone();
    snapshot.set_caption(caption);
    snapshot
}

thread_local! {
    /// C++ static `NetworkOptionsInterface::oldNetworkStatus` (initially `NETWORK_NONE`). It is a static in
    /// the original, so it survives the screen being rebuilt by `RunScreen`.
    static OLD_NETWORK_STATUS: Cell<i32> = const { Cell::new(NETWORK_NONE) };
}

/// Network options screen: switch between server and client mode.
#[derive(Debug, Default)]
pub struct NetworkOptionsInterface {}

impl NetworkOptionsInterface {
    pub fn new() -> Self {
        Self::default()
    }

    /// Static `CloseClick` (MouseFn).
    pub fn close_click(g: &mut Ctx, _button: &Button) {
        MainMenu::run_screen(g, MAINMENU_OPTIONS);
    }

    /// Static `ServerButtonClick` (MouseFn).
    pub fn server_button_click(g: &mut Ctx, _button: &Button) {
        if g.app.network.status == NETWORK_NONE {
            g.app.network.start_server();

            if g.app.network.status == NETWORK_SERVER {
                g.app.network.get_server_mut().listen();
            }
        } else {
            g.app.network.stop_server();
        }
    }

    /// Static `ClientButtonClick` (MouseFn).
    pub fn client_button_click(g: &mut Ctx, _button: &Button) {
        if g.app.network.status == NETWORK_SERVER {
            println!("NetworkOptionsInterface::ClientButtonClick, cannot connect to a server when already running as a server");
            return;
        }

        if g.app.network.status == NETWORK_NONE {
            let target = g
                .ecl
                .get_button("network_clienttarget")
                .expect("network_clienttarget button missing")
                .caption
                .clone();
            g.app.network.start_client(&target);
        } else {
            g.app.network.stop_client();
        }
    }

    /// Static `StatusButtonDraw` (DrawFn).
    pub fn status_button_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        let caption = match g.app.network.status {
            NETWORK_NONE => Some("Status : Not connected"),
            NETWORK_CLIENT => Some("Status : Connected to Server"),
            NETWORK_SERVER => Some("Status : Server Running"),
            _ => None,
        };
        let snapshot = match caption {
            Some(c) => caption_snapshot(g, button, c),
            None => button.clone(),
        };

        textbutton_draw(g, &snapshot, highlighted, clicked);
    }

    /// Static `ServerButtonDraw` (DrawFn).
    pub fn server_button_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        match g.app.network.status {
            NETWORK_NONE => {
                let snapshot = caption_snapshot(g, button, "Start Server");
                button_draw(g, &snapshot, highlighted, clicked);
            }
            NETWORK_CLIENT => {
                clear_draw(g, button.x, button.y, button.width, button.height);
            }
            NETWORK_SERVER => {
                let snapshot = caption_snapshot(g, button, "Stop Server");
                button_draw(g, &snapshot, highlighted, clicked);
            }
            _ => {}
        }
    }

    /// Static `ClientButtonDraw` (DrawFn).
    pub fn client_button_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        match g.app.network.status {
            NETWORK_NONE => {
                let snapshot = caption_snapshot(g, button, "Connect to Server");
                button_draw(g, &snapshot, highlighted, clicked);
            }
            NETWORK_CLIENT => {
                let snapshot = caption_snapshot(g, button, "Disconnect");
                button_draw(g, &snapshot, highlighted, clicked);
            }
            NETWORK_SERVER => {
                clear_draw(g, button.x, button.y, button.width, button.height);
            }
            _ => {}
        }
    }
}

impl InterfaceScreen for NetworkOptionsInterface {
    fn create(&mut self, g: &mut Ctx) {
        // Switch on the listener
        if g.app.network.status == NETWORK_SERVER {
            g.app.network.get_server_mut().listen();
        }

        if !self.is_visible(g) {
            mainmenuscreen::create(g);

            let screenw = g.app.options.get_option_value("graphics_screenwidth");
            let screenh = g.app.options.get_option_value("graphics_screenheight");

            // Title bar
            g.ecl.register_button(screenw - 240, screenh - 40, 230, 15, "MULTIMONITOR options", "Close this options screen", "network_title");
            g.ecl.register_button_callback("network_title", Self::close_click);
            g.ecl.register_movement("network_title", screenw - 240, screenh - 250, 500, MOVE_STRAIGHTLINE, None);

            // Status of network
            g.ecl.register_button(screenw - 240, screenh - 40, 230, 30, "checking...", "", "network_details");
            g.ecl.register_button_callbacks("network_details", Some(textbutton_draw), None, None, None);
            g.ecl.register_movement("network_details", screenw - 240, screenh - 230, 500, MOVE_STRAIGHTLINE, None);

            g.ecl.register_button(screenw - 240, screenh - 40, 230, 15, "checking...", "", "network_status");
            g.ecl.register_button_callbacks("network_status", Some(Self::status_button_draw), None, None, None);
            g.ecl.register_movement("network_status", screenw - 240, screenh - 195, 500, MOVE_STRAIGHTLINE, None);

            // Client / Server buttons
            g.ecl.register_button(screenw - 240, screenh - 40, 110, 30, "...", "...", "network_client");
            g.ecl.register_button_callbacks(
                "network_client",
                Some(Self::client_button_draw),
                Some(Self::client_button_click),
                Some(button_click),
                Some(button_highlight),
            );
            g.ecl.register_movement("network_client", screenw - 240, screenh - 170, 500, MOVE_STRAIGHTLINE, None);

            g.ecl.register_button(screenw - 120, screenh - 40, 110, 30, "...", "...", "network_server");
            g.ecl.register_button_callbacks(
                "network_server",
                Some(Self::server_button_draw),
                Some(Self::server_button_click),
                Some(button_click),
                Some(button_highlight),
            );
            g.ecl.register_movement("network_server", screenw - 120, screenh - 170, 500, MOVE_STRAIGHTLINE, None);

            // Client IP address
            g.ecl.register_button(screenw - 240, screenh - 40, 230, 15, "localhost", "", "network_clienttarget");
            g.ecl.register_button_callbacks("network_clienttarget", Some(textbutton_draw), None, None, Some(button_highlight));
            g.ecl.register_movement("network_clienttarget", screenw - 240, screenh - 135, 500, MOVE_STRAIGHTLINE, None);
            g.ecl.make_button_editable("network_clienttarget");

            // Network connections
            g.ecl.register_button(screenw - 240, screenh - 40, 230, 60, "", "", "network_connections");
            g.ecl.register_button_callbacks("network_connections", Some(textbutton_draw), None, None, None);
            g.ecl.register_movement("network_connections", screenw - 240, screenh - 110, 500, MOVE_STRAIGHTLINE, None);

            // Close button
            g.ecl.register_button(screenw - 60, screenh - 40, 50, 15, "Close", "Return to Options menu", "network_close");
            g.ecl.register_button_callback("network_close", Self::close_click);
        }
    }

    fn remove(&mut self, g: &mut Ctx) {
        // Switch off the listener
        if g.app.network.status == NETWORK_SERVER {
            g.app.network.get_server_mut().stop_listening();
        }

        if self.is_visible(g) {
            mainmenuscreen::remove(g);

            g.ecl.remove_button("network_title");
            g.ecl.remove_button("network_close");
            g.ecl.remove_button("network_status");
            g.ecl.remove_button("network_details");
            g.ecl.remove_button("network_server");
            g.ecl.remove_button("network_client");
            g.ecl.remove_button("network_clienttarget");
            g.ecl.remove_button("network_connections");
        }
    }

    fn update(&mut self, g: &mut Ctx) {
        // Update the network details
        let localhost = g.app.network.get_local_host().unwrap_or_default();
        let localip = g.app.network.get_local_ip().unwrap_or_default();
        let details = format!("Localhost : {localhost}\nLocal IP  : {localip}");
        g.ecl
            .get_button_mut("network_details")
            .expect("network_details button missing")
            .set_caption(&details);

        // If network status has changed, refresh the buttons
        if g.app.network.status != OLD_NETWORK_STATUS.with(|c| c.get()) {
            g.ecl.dirty_button("network_status");
            g.ecl.dirty_button("network_client");
            g.ecl.dirty_button("network_server");

            g.ecl
                .get_button_mut("network_connections")
                .expect("network_connections button missing")
                .set_caption(" ");

            OLD_NETWORK_STATUS.with(|c| c.set(g.app.network.status));
        }

        // Update current connections (if we are a server)
        if g.app.network.status == NETWORK_SERVER {
            let server = g.app.network.get_server();
            let mut cdetails = format!("Active connections : {}", server.clients.num_used());

            let mut entries: Vec<(i32, String, String)> = Vec::new();
            for i in 0..server.clients.size() {
                if server.clients.valid_index(i) {
                    let host = server.get_remote_host(i).unwrap_or_default();
                    let ip = server.get_remote_ip(i).unwrap_or_default();
                    entries.push((i, host, ip));
                }
            }

            for (i, host, ip) in entries {
                let active = ClientConnection::time_active(g, i);
                cdetails.push_str(&format!("\nFrom {host}, ip {ip}, active {active} secs"));
            }

            g.ecl
                .get_button_mut("network_connections")
                .expect("network_connections button missing")
                .set_caption(&cdetails);
        }
    }

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("network_title").is_some()
    }
    fn screen_id(&self) -> i32 {
        MAINMENU_NETWORKOPTIONS
    }
    fn return_key_pressed(&mut self, _g: &mut Ctx) -> bool {
        true
    }
    crate::impl_as_any!();
}
