//! Port of source/interface/taskmanager/ipprobe.h + source/interface/taskmanager/ipprobe.cpp

use uplink_engine::gl;

use crate::app::globals::ALPHA;
use crate::app::miscutils::strip_carriage_returns;
use crate::app::opengl_interface as ogl;
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::interface::taskmanager::uplinktask::{UplinkTask, UplinkTaskBase};
use crate::vanbakel::Vanbakel;
use crate::world::computer::computer::Computer;
use crate::world::message::Message;

pub const PROBE_IDLE: i32 = 0;
pub const PROBE_INPROGRESS: i32 = 1;
pub const PROBE_FINISHED: i32 = 2;

#[derive(Debug)]
pub struct IPProbe {
    pub base: UplinkTaskBase,
    pub status: i32,
    /// Used to reset to idle after 5 secs (unix seconds)
    pub timeout: i64,
}

/// PID from a button name of the form `"<word> <pid>"` (C++ `sscanf(name, "probe_go %d", &pid)`).
/// A name without a readable PID gives -1, which matches no task (C++ left `pid` uninitialised).
fn pid_of(name: &str) -> i32 {
    name.split_whitespace().nth(1).and_then(|t| t.parse().ok()).unwrap_or(-1)
}

/// C++ `time(NULL)`: unix seconds.
fn time_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Names of the three buttons the IP Probe window is made of: go, display, close.
fn button_names(pid: i32) -> [String; 3] {
    [
        format!("probe_go {pid}"),
        format!("probe_display {pid}"),
        format!("probe_close {pid}"),
    ]
}

/// The body of the probe's result message for a found computer.
fn probe_body(comp: &Computer, version: f32) -> String {
    let mut body = String::new();
    body.push_str("Results of IP Probe\n\n");
    body.push_str(&format!("IP: {}\n", comp.ip));
    body.push_str(&format!("({})\n", comp.name));
    body.push('\n');

    for i in 0..comp.security.num_systems() {
        let Some(ss) = comp.security.get_system(i) else {
            continue;
        };

        if version == 1.0 {
            body.push_str(&format!("{}\n", ss.get_name()));
        } else if version == 2.0 {
            body.push_str(&format!("{} level {}\n", ss.get_name(), ss.level));
        } else if version == 3.0 {
            body.push_str(&format!("{} level {}\n", ss.get_name(), ss.level));
            if ss.bypassed {
                body.push_str("bypassed\n");
            }
            if ss.enabled {
                body.push_str("enabled\n");
            }
            if !ss.enabled {
                body.push_str("disabled\n");
            }
        }
    }

    // C++ appends '\x0' here only to terminate the ostrstream buffer; it is not part of the text.
    body
}

impl IPProbe {
    pub fn new() -> Self {
        IPProbe {
            base: UplinkTaskBase {
                version: 1.0,
                targetint: -1,
                ..Default::default()
            },
            status: PROBE_IDLE,
            timeout: 0,
        }
    }

    /// C++ `IPProbe::DisplayDraw` (static button callback).
    pub fn display_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        // Draw a grey background
        gl::begin(gl::QUADS);
        gl::color4f(0.6, 0.6, 0.6, ALPHA);
        gl::vertex2i(button.x, button.y);
        gl::color4f(0.4, 0.4, 0.4, ALPHA);
        gl::vertex2i(button.x + button.width, button.y);
        gl::color4f(0.6, 0.6, 0.6, ALPHA);
        gl::vertex2i(button.x + button.width, button.y + button.height);
        gl::color4f(0.4, 0.4, 0.4, ALPHA);
        gl::vertex2i(button.x, button.y + button.height);
        gl::end();

        // Draw ordinary text
        ogl::text_draw(g, button, highlighted, clicked);

        // Draw a border
        if highlighted || clicked {
            ogl::border_draw(button);
        }
    }

    /// C++ `IPProbe::CloseClick` (static button callback).
    pub fn close_click(g: &mut Ctx, button: &Button) {
        let pid = pid_of(&button.name);
        Vanbakel::remove_task(g, pid);
    }

    /// C++ `IPProbe::GoClick` (static button callback).
    pub fn go_click(g: &mut Ctx, button: &Button) {
        let pid = pid_of(&button.name);

        let Some(task) = g.svb.get_task_mut::<IPProbe>(pid) else {
            debug_assert!(false, "IPProbe::go_click: no IPProbe task {pid}");
            return;
        };

        if task.status == PROBE_IDLE {
            task.status = PROBE_INPROGRESS;
            task.timeout = time_now() + 1; // (3 seconds to complete)
        } else {
            task.status = PROBE_IDLE;
        }
    }
}

impl Default for IPProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl UplinkTask for IPProbe {
    fn base(&self) -> &UplinkTaskBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut UplinkTaskBase {
        &mut self.base
    }
    fn initialise(&mut self, _g: &mut Ctx) {}

    fn tick(&mut self, g: &mut Ctx, _n: i32) {
        if !self.is_interface_visible(g) {
            return;
        }

        if self.status == PROBE_IDLE {
            // Do nothing
        } else if self.status == PROBE_INPROGRESS {
            if time_now() >= self.timeout {
                // Try to lookup the IP in the box
                let pid = self.base.pid;
                let name_display = format!("probe_display {pid}");

                let caption = match g.ecl.get_button(&name_display) {
                    Some(b) => b.caption.clone(),
                    None => {
                        debug_assert!(false, "IPProbe::tick: no button {name_display}");
                        return;
                    }
                };
                let ip = strip_carriage_returns(&caption);

                // Found: the report body. Not found: None.
                let report = match g.world().get_vlocation(&ip) {
                    Some(vl) => match vl.get_computer(g.world()) {
                        Some(comp) => Some(probe_body(comp, self.base.version)),
                        None => {
                            // PORT: C++ UplinkAssert(comp) aborts the game here; release builds treat it as not found.
                            debug_assert!(false, "IPProbe::tick: VLocation {ip} has no computer");
                            None
                        }
                    },
                    None => None,
                };

                match report {
                    Some(body) => {
                        // Found - send a message to the player with the details
                        let mut msg = Message::new();
                        msg.set_to("PLAYER");
                        msg.set_from("IP Probe");
                        msg.set_subject("Results of probe");
                        msg.set_body(&body);
                        msg.send(g);
                    }
                    None => {
                        // Not found
                        g.ecl.register_caption_change_auto(&name_display, "IP Not found", None);
                    }
                }

                self.timeout = time_now() + 5;
                self.status = PROBE_FINISHED;
            }
        } else if self.status == PROBE_FINISHED {
            // Revert to idle after 5 secs
            if time_now() > self.timeout {
                let name_display = format!("probe_display {}", self.base.pid);

                g.ecl.register_caption_change_auto(&name_display, "Enter IP", None);
                self.status = PROBE_IDLE;
            }
        } else {
            panic!("Unkown Status");
        }
    }

    fn create_interface(&mut self, g: &mut Ctx) {
        if self.is_interface_visible(g) {
            return;
        }

        let pid = self.base.pid;
        let [name_go, name_display, name_close] = button_names(pid);

        g.ecl.register_button(305, 444, 20, 15, "", "Perform the lookup", &name_go);
        g.ecl.register_button(325, 444, 185, 14, "Enter IP", "Type your IP here", &name_display);
        g.ecl.register_button(510, 445, 13, 13, "", "Close the IP Lookup program", &name_close);

        ogl::button_assignbitmap(g, &name_go, "software/go.tif");
        g.ecl.register_button_callback(&name_go, Self::go_click);
        g.ecl.register_button_callbacks(
            &name_display,
            Some(Self::display_draw),
            None,
            Some(ogl::button_click),
            Some(ogl::button_highlight),
        );
        ogl::button_assignbitmaps(g, &name_close, "close.tif", "close_h.tif", "close_c.tif");
        g.ecl.register_button_callback(&name_close, Self::close_click);

        g.ecl.make_button_editable(&name_display);
    }

    fn remove_interface(&mut self, g: &mut Ctx) {
        if !self.is_interface_visible(g) {
            return;
        }

        for name in button_names(self.base.pid) {
            g.ecl.remove_button(&name);
        }
    }

    fn show_interface(&mut self, g: &mut Ctx) {
        if !self.is_interface_visible(g) {
            self.create_interface(g);
        }

        for name in button_names(self.base.pid) {
            g.ecl.button_bring_to_front(&name);
        }
    }

    fn is_interface_visible(&self, g: &Ctx) -> bool {
        let name_display = format!("probe_display {}", self.base.pid);
        g.ecl.get_button(&name_display).is_some()
    }

    crate::impl_as_any!();
}
