//! Port of source/interface/localinterface/analyser_interface.h + source/interface/localinterface/analyser_interface.cpp
//! Connection analyser: shows the remote host, its security systems and their status.

use uplink_engine::gl;

use crate::app::opengl_interface::{border_draw, button_assignbitmaps, set_colour, text_draw};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::game::data::data::IP_LOCALHOST;
use crate::impl_as_any;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::localinterface::localinterface::{LocalInterface, PANELSIZE, SCREEN_ANALYSER, SCREEN_NONE};
use crate::interface::localinterface::localinterfacescreen;
use crate::interface::taskmanager::taskmanager::TaskManager;
use crate::objref::ObjRef;
use crate::vanbakel::Vanbakel;
use crate::world::computer::computer::Computer;

/// C++ file-scope `STIPPLE[8]`: line stipple patterns cycled by the analyser's connection lines.
pub const STIPPLE: [i32; 8] = [
    0x1111, // 0001000100010001
    0x1111, // 0001000100010001
    0x2222, // 0010001000100010
    0x2222, // 0010001000100010
    0x4444, // 0100010001000100
    0x4444, // 0100010001000100
    0x8888, // 1000100010001000
    0x8888, // 1000100010001000
];

/// C++ `AnalyserInterface` statics (`remotehost`, `moving_stipplepattern`, `static_stipplepattern`,
/// file-static `stippleindex`) are fields on the screen. Static button callbacks reach them through
/// `g.interface_mut().li.screen.get_mut::<AnalyserInterface>()`.
#[derive(Debug, Clone)]
pub struct AnalyserInterface {
    /// `static char remotehost[SIZE_VLOCATION_IP]`: IP of the host being analysed.
    pub remotehost: String,
    /// `static int moving_stipplepattern`
    pub moving_stipplepattern: i32,
    /// `static int static_stipplepattern`
    pub static_stipplepattern: i32,
    /// File-static `stippleindex` (index into `STIPPLE`).
    pub stippleindex: i32,
    pub lastupdate: i32,
    pub stippleupdate: i32,
}

impl Default for AnalyserInterface {
    fn default() -> Self {
        AnalyserInterface {
            remotehost: String::new(),
            moving_stipplepattern: STIPPLE[0],
            static_stipplepattern: STIPPLE[0],
            stippleindex: 0,
            lastupdate: 0,
            stippleupdate: 0,
        }
    }
}

/// C++ `time(NULL)`: whole seconds since the Unix epoch.
fn now_secs() -> i32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i32)
        .unwrap_or(0)
}

/// The analyser screen, if it is the current local screen.
fn analyser_screen(g: &Ctx) -> Option<&AnalyserInterface> {
    g.interface().li.screen.get::<AnalyserInterface>()
}

/// The `remotehost` the analyser is showing (C++ static `AnalyserInterface::remotehost`).
fn remote_host(g: &Ctx) -> String {
    analyser_screen(g).map(|s| s.remotehost.clone()).unwrap_or_default()
}

/// C++ `(screenw, paneltop, panelwidth)` as computed at the top of each analyser method.
fn panel_geometry(g: &Ctx) -> (i32, i32, i32) {
    let screenw = g.app.options.get_option_value("graphics_screenwidth");
    let paneltop = (100.0 * ((screenw as f64 * PANELSIZE) / 188.0) + 30.0) as i32;
    let panelwidth = (screenw as f64 * PANELSIZE) as i32;
    (screenw, paneltop, panelwidth)
}

/// C++ `VLocation *vl = GetVLocation(remotehost); UplinkAssert(vl); Computer *comp = vl->GetComputer(); UplinkAssert(comp);`
fn remote_computer<'a>(g: &'a Ctx, remotehost: &str) -> &'a Computer {
    let vl = g
        .world()
        .get_vlocation(remotehost)
        .expect("analyser: remote host has no VLocation");
    vl.get_computer(g.world())
        .expect("analyser: remote host has no computer")
}

/// C++ `SecurityMonitor::IsMonitored(index)` (static).
fn system_is_monitored(g: &Ctx, index: i32) -> bool {
    g.security_monitor.is_monitored(index)
}

/// Registers the `analyser_system N` button (security system) and its `analyser_system_t N` caption.
fn register_system_button(g: &mut Ctx, index: i32, x: i32, y: i32, text_x: i32, text_y: i32) {
    let name = format!("analyser_system {index}");
    let text_name = format!("analyser_system_t {index}");

    g.ecl.register_button(x, y, 32, 32, "", "Shows a security system", &name);
    button_assignbitmaps(g, &name, "analyser/unknown.tif", "analyser/unknown.tif", "analyser/unknown.tif");
    g.ecl.register_button_callback(&name, AnalyserInterface::system_click);

    g.ecl.register_button(text_x, text_y, 70, 15, "Analysing", "", &text_name);
    g.ecl.register_button_callbacks(&text_name, Some(text_draw), None, None, None);
}

/// Removes every `analyser_system N` button and its `analyser_system_t N` caption, in index order.
fn remove_system_buttons(g: &mut Ctx) {
    let mut systemindex = 0;
    while g.ecl.get_button(&format!("analyser_system {systemindex}")).is_some() {
        g.ecl.remove_button(&format!("analyser_system {systemindex}"));
        g.ecl.remove_button(&format!("analyser_system_t {systemindex}"));
        systemindex += 1;
    }
}

impl AnalyserInterface {
    /// C++ `AnalyserInterface::CycleStipplePattern` (static; touches the static pattern state).
    pub fn cycle_stipple_pattern(&mut self) {
        self.stippleindex -= 1;
        if self.stippleindex < 0 {
            self.stippleindex = 7;
        }
        self.moving_stipplepattern = STIPPLE[self.stippleindex as usize];
    }

    /// C++ `AnalyserInterface::TitleClick` (static button callback).
    pub fn title_click(g: &mut Ctx, _button: &Button) {
        LocalInterface::run_screen(g, SCREEN_NONE, -1);
    }

    /// C++ `AnalyserInterface::SystemClick` (static button callback).
    pub fn system_click(g: &mut Ctx, button: &Button) {
        // C++ `sscanf(button->name, "analyser_system %d", &systemindex)`
        let systemindex: i32 = button
            .name
            .strip_prefix("analyser_system ")
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0);

        let remotehost = remote_host(g);
        let comp_name = remote_computer(g, &remotehost).name.clone();

        // C++ `(UplinkObject *) &(comp->security)`: the security object of that computer.
        TaskManager::set_program_target(g, ObjRef::Security(comp_name), &button.name, systemindex);
    }

    /// C++ `AnalyserInterface::ConnectionDraw` (static draw callback).
    pub fn connection_draw(g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        let (screenw, paneltop, panelwidth) = panel_geometry(g);
        let stipple = analyser_screen(g).map_or(STIPPLE[0], |s| s.moving_stipplepattern);

        // Look up the remote host (the lookup is the C++ UplinkAssert pair)
        let remotehost = remote_host(g);
        remote_computer(g, &remotehost);

        // Draw the standard shaded background
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

        // Draw connecting lines
        if remotehost != IP_LOCALHOST {
            gl::color4f(1.0, 1.0, 1.0, 1.0);
            gl::line_width(2.0);
            gl::line_stipple(2, stipple as u16);
            gl::enable(gl::LINE_STIPPLE);

            gl::begin(gl::LINE_STRIP);
            gl::vertex2i(screenw - panelwidth + 40, paneltop + 75);
            gl::vertex2i(screenw - panelwidth + 40, paneltop + 160);
            gl::vertex2i(screenw - 40, paneltop + 160);
            gl::vertex2i(screenw - 40, paneltop + 240);
            gl::end();

            gl::line_width(1.0);
            gl::disable(gl::LINE_STIPPLE);
        }

        // Draw boxes around any systems that are being monitored
        let mut systemindex = 0;
        while g.ecl.get_button(&format!("analyser_system {systemindex}")).is_some() {
            let name = format!("analyser_system {systemindex}");

            if system_is_monitored(g, systemindex) {
                let (bx, by, bw, bh) = {
                    let b = g.ecl.get_button(&name).expect("analyser_system button");
                    (b.x, b.y, b.width, b.height)
                };

                gl::color4f(1.0, 0.1, 0.1, 1.0);
                gl::line_width(2.0);
                gl::line_stipple(2, stipple as u16);
                gl::enable(gl::LINE_STIPPLE);

                gl::begin(gl::LINE_LOOP);
                gl::vertex2i(bx - 10, by - 10);
                gl::vertex2i(bx + bw + 10, by - 10);
                gl::vertex2i(bx + bw + 10, by + bh + 10);
                gl::vertex2i(bx - 10, by + bh + 10);
                gl::end();

                gl::line_width(1.0);
                gl::disable(gl::LINE_STIPPLE);
            }

            systemindex += 1;
        }
    }
}

impl InterfaceScreen for AnalyserInterface {
    fn create(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            localinterfacescreen::create(g);
            g.ecl.register_button_callbacks("localint_background", Some(Self::connection_draw), None, None, None);

            self.remotehost = " ".to_string();

            let (screenw, paneltop, panelwidth) = panel_geometry(g);

            g.ecl.register_button(screenw - panelwidth, paneltop + 3, panelwidth - 7, 15, "CONNECTION ANALYSER", "Remove this screen", "analyser_title");
            g.ecl.register_button_callback("analyser_title", Self::title_click);

            // Local host button

            g.ecl.register_button(screenw - panelwidth + 20, paneltop + 35, 40, 40, "", "Shows your home computer", "analyser_localhost");
            button_assignbitmaps(g, "analyser_localhost", "analyser/localhost.tif", "analyser/localhost_h.tif", "analyser/localhost_c.tif");

            g.ecl.register_button(screenw - panelwidth + 6, paneltop + 20, 70, 15, "Localhost", "", "analyser_localhost_t");
            g.ecl.register_button_callbacks("analyser_localhost_t", Some(text_draw), None, None, None);
        }
    }

    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            localinterfacescreen::remove(g);

            g.ecl.remove_button("analyser_title");

            g.ecl.remove_button("analyser_localhost");
            g.ecl.remove_button("analyser_localhost_t");

            g.ecl.remove_button("analyser_remotehost");
            g.ecl.remove_button("analyser_remotehost_t");
            g.ecl.remove_button("analyser_remotehost_title");

            // Remove any security system buttons
            remove_system_buttons(g);
        }
    }

    fn update(&mut self, g: &mut Ctx) {
        // Has our remotehost changed since we last updated?
        let player_host = g.player().remotehost.clone();

        if self.remotehost != player_host {
            // Yes it has - redo everything
            self.remotehost = player_host;

            // Remove the old remote host button
            if g.ecl.get_button("analyser_remotehost").is_some() {
                g.ecl.remove_button("analyser_remotehost");
                g.ecl.remove_button("analyser_remotehost_t");
                g.ecl.remove_button("analyser_remotehost_title");
            }

            // Remove any old security system buttons
            remove_system_buttons(g);

            // Get screen dimensions
            let (screenw, paneltop, panelwidth) = panel_geometry(g);

            // Look up the new remote host
            let (comp_ip, comp_name, num_systems) = {
                let comp = remote_computer(g, &self.remotehost);
                (comp.ip.clone(), comp.name.clone(), comp.security.num_systems())
            };

            // Create the new remote host buttons

            if comp_ip != IP_LOCALHOST && g.ecl.get_button("analyser_remotehost").is_none() {
                g.ecl.register_button(screenw - 60, paneltop + 240, 40, 40, "", "Shows the remote computer target", "analyser_remotehost");
                button_assignbitmaps(g, "analyser_remotehost", "analyser/remotehost.tif", "analyser/remotehost_h.tif", "analyser/remotehost_c.tif");

                g.ecl.register_button(screenw - 78, paneltop + 281, 70, 15, "Remotehost", "", "analyser_remotehost_t");
                g.ecl.register_button_callbacks("analyser_remotehost_t", Some(text_draw), None, None, None);

                g.ecl.register_button(screenw - panelwidth + 5, paneltop + 240, 110, 40, "", "", "analyser_remotehost_title");
                g.ecl.register_button_callbacks("analyser_remotehost_title", Some(text_draw), None, None, None);
            }

            g.ecl.register_caption_change_auto("analyser_remotehost_title", &comp_name, None);

            // Create the new security system buttons

            match num_systems {
                1 => {
                    register_system_button(g, 0, screenw - 110, paneltop + 145, screenw - 120, paneltop + 180);
                }
                2 => {
                    register_system_button(g, 0, screenw - panelwidth + 25, paneltop + 145, screenw - panelwidth + 15, paneltop + 180);
                    register_system_button(g, 1, screenw - 55, paneltop + 145, screenw - 65, paneltop + 130);
                }
                3 => {
                    register_system_button(g, 0, screenw - panelwidth + 25, paneltop + 145, screenw - panelwidth + 15, paneltop + 180);
                    register_system_button(g, 1, screenw - panelwidth / 2 - 16, paneltop + 145, screenw - panelwidth / 2 - 26, paneltop + 180);
                    register_system_button(g, 2, screenw - 55, paneltop + 145, screenw - 65, paneltop + 130);
                }
                _ => {}
            }

            // Bring all bypassers to the front
            Vanbakel::show_all_tasks(g);

            // Set up last update for a 3 second delay
            self.lastupdate = now_secs();
        }

        // Update security system buttons with status
        // Every few seconds
        if now_secs() > self.lastupdate + 3 {
            // Look up the host computer
            let mut systemindex = 0;
            while g.ecl.get_button(&format!("analyser_system {systemindex}")).is_some() {
                let name = format!("analyser_system {systemindex}");

                let (secure, label) = {
                    let comp = remote_computer(g, &self.remotehost);
                    let ss = comp.security.get_system(systemindex);
                    (
                        ss.is_some_and(|ss| ss.enabled && !ss.bypassed),
                        ss.expect("analyser: security system missing").get_name(),
                    )
                };

                if secure {
                    button_assignbitmaps(g, &name, "analyser/secure.tif", "analyser/secure_h.tif", "analyser/secure_c.tif");
                } else {
                    button_assignbitmaps(g, &name, "analyser/nonsecure.tif", "analyser/nonsecure_h.tif", "analyser/nonsecure_c.tif");
                }

                g.ecl.register_button_callback(&name, Self::system_click);

                let text_name = format!("analyser_system_t {systemindex}");
                g.ecl
                    .get_button_mut(&text_name)
                    .expect("analyser_system_t button")
                    .set_caption(label);

                systemindex += 1;
            }

            self.lastupdate = now_secs();
        }

        // Update the stipple patterns
        // AT A FIXED RATE ;)
        if g.ecl.get_accurate_time() > self.stippleupdate {
            self.cycle_stipple_pattern();
            g.ecl.dirty_button("localint_background");
            self.stippleupdate = g.ecl.get_accurate_time() + 20;
        }
    }

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("analyser_title").is_some()
    }

    fn screen_id(&self) -> i32 {
        SCREEN_ANALYSER
    }

    impl_as_any!();
}
