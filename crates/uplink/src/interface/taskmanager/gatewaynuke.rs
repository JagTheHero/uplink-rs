//! Port of source/interface/taskmanager/gatewaynuke.h + source/interface/taskmanager/gatewaynuke.cpp

use uplink_engine::gl;

use crate::app::globals::ALPHA;
use crate::app::opengl_interface::text_draw;
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::interface::remoteinterface::remoteinterface::RemoteInterface;
use crate::interface::taskmanager::uplinktask::{UplinkTask, UplinkTaskBase};
use crate::objref::ObjRef;
use crate::vanbakel::Vanbakel;
use crate::world::computer::gateway::Gateway;
use crate::world::connection::Connection;

pub const GATEWAYNUKE_STATUS_NONE: i32 = 0;
pub const GATEWAYNUKE_STATUS_START: i32 = 1;
pub const GATEWAYNUKE_STATUS_AREYOUSURE: i32 = 2;
pub const GATEWAYNUKE_STATUS_FAILED: i32 = 3;

#[derive(Debug)]
pub struct GatewayNuke {
    pub base: UplinkTaskBase,
    pub status: i32,
}

/// C++ `sscanf(name, "<literal> %d", &pid)`: the integer after `literal` (whitespace skipped).
fn scan_pid(name: &str, literal: &str) -> Option<i32> {
    let rest = name.strip_prefix(literal)?.trim_start();
    let end = rest
        .char_indices()
        .find(|(i, c)| !(c.is_ascii_digit() || (*i == 0 && (*c == '-' || *c == '+'))))
        .map(|(i, _)| i)
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

/// Sets the status of the GatewayNuke task `pid` (no-op if it is missing or running).
fn set_status(g: &mut Ctx, pid: i32, status: i32) {
    if let Some(task) = g.svb.get_task_mut::<GatewayNuke>(pid) {
        task.status = status;
    }
}

impl GatewayNuke {
    pub fn new() -> Self {
        GatewayNuke {
            base: UplinkTaskBase {
                version: 1.0,
                targetint: -1,
                ..Default::default()
            },
            status: GATEWAYNUKE_STATUS_NONE,
        }
    }

    /// C++ `GatewayNuke::TitleDraw` (static button callback).
    pub fn title_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        gl::begin(gl::QUADS);
        gl::color4f(0.2, 0.2, 0.4, ALPHA);
        gl::vertex2i(button.x, button.y);
        gl::color4f(0.3, 0.3, 0.5, ALPHA);
        gl::vertex2i(button.x + button.width, button.y);
        gl::color4f(0.2, 0.2, 0.4, ALPHA);
        gl::vertex2i(button.x + button.width, button.y + button.height);
        gl::color4f(0.3, 0.3, 0.5, ALPHA);
        gl::vertex2i(button.x, button.y + button.height);
        gl::end();

        text_draw(g, button, highlighted, clicked);
    }

    /// C++ `GatewayNuke::YesClick` (static button callback).
    pub fn yes_click(g: &mut Ctx, button: &Button) {
        let Some(pid) = scan_pid(&button.name, "gatenuke_yes") else {
            return;
        };

        let title = format!("gatenuke_title {pid}");

        let Some(status) = g.svb.get_task::<GatewayNuke>(pid).map(|t| t.status) else {
            debug_assert!(false, "GatewayNuke::YesClick: no gateway nuke task {pid}");
            return;
        };

        match status {
            GATEWAYNUKE_STATUS_NONE => {
                panic!("Status not set correctly");
            }

            GATEWAYNUKE_STATUS_START => {
                // Ask for confirmation
                set_status(g, pid, GATEWAYNUKE_STATUS_AREYOUSURE);
                g.ecl.register_caption_change_auto(&title, "Are you sure?", None);
            }

            GATEWAYNUKE_STATUS_AREYOUSURE => {
                // Do it!
                if g.player().gateway.is_hw_installed("Gateway Self Destruct") {
                    Self::nuke_gateway(g);
                    Vanbakel::remove_task(g, pid);
                } else {
                    g.ecl.register_caption_change_auto(&title, "No explosives found", None);
                    set_status(g, pid, GATEWAYNUKE_STATUS_FAILED);
                }
            }

            GATEWAYNUKE_STATUS_FAILED => {
                Vanbakel::remove_task(g, pid);
            }

            _ => {
                panic!("Unknown Status");
            }
        }
    }

    /// C++ `GatewayNuke::NoClick` (static button callback).
    pub fn no_click(g: &mut Ctx, button: &Button) {
        // Abort gate nuke
        let Some(pid) = scan_pid(&button.name, "gatenuke_no") else {
            return;
        };
        Vanbakel::remove_task(g, pid);
    }

    /// C++ `GatewayNuke::NukeGateway` (static): resets the gateway hardware, drops the connection and returns to the main screen.
    pub fn nuke_gateway(g: &mut Ctx) {
        // Reset the hardware
        let owner = g.player().name.clone();
        Gateway::nuke(g);

        // Close any connections
        Connection::disconnect(g, &owner);
        Connection::reset(g, &owner);

        RemoteInterface::run_new_location(g);
        RemoteInterface::run_screen(g, 0);
    }
}

impl Default for GatewayNuke {
    fn default() -> Self {
        Self::new()
    }
}

impl UplinkTask for GatewayNuke {
    fn base(&self) -> &UplinkTaskBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut UplinkTaskBase {
        &mut self.base
    }
    fn initialise(&mut self, _g: &mut Ctx) {
        self.status = GATEWAYNUKE_STATUS_START;
    }
    fn tick(&mut self, _g: &mut Ctx, _n: i32) {}
    fn create_interface(&mut self, g: &mut Ctx) {
        if !self.is_interface_visible(g) {
            let pid = self.base.pid;

            let title = format!("gatenuke_title {pid}");
            let yes = format!("gatenuke_yes {pid}");
            let no = format!("gatenuke_no {pid}");

            g.ecl.register_button(40, 20, 107, 15, "Nuke Gateway?", "Click yes to destroy your gateway, no to abort", &title);
            g.ecl.register_button(20, 20, 20, 15, "Yes", "Click here to nuke your gateway", &yes);
            g.ecl.register_button(147, 20, 20, 15, "No", "Click here to abort", &no);

            g.ecl.register_button_callbacks(&title, Some(Self::title_draw), None, None, None);
            g.ecl.register_button_callback(&yes, Self::yes_click);
            g.ecl.register_button_callback(&no, Self::no_click);
        }
    }
    fn remove_interface(&mut self, g: &mut Ctx) {
        if self.is_interface_visible(g) {
            let pid = self.base.pid;

            let title = format!("gatenuke_title {pid}");
            let yes = format!("gatenuke_yes {pid}");
            let no = format!("gatenuke_no {pid}");

            g.ecl.remove_button(&title);
            g.ecl.remove_button(&yes);
            g.ecl.remove_button(&no);
        }
    }
    fn show_interface(&mut self, g: &mut Ctx) {
        if !self.is_interface_visible(g) {
            self.create_interface(g);
        }

        let pid = self.base.pid;

        let title = format!("gatenuke_title {pid}");
        let yes = format!("gatenuke_yes {pid}");
        let no = format!("gatenuke_no {pid}");

        g.ecl.button_bring_to_front(&title);
        g.ecl.button_bring_to_front(&yes);
        g.ecl.button_bring_to_front(&no);
    }
    fn is_interface_visible(&self, g: &Ctx) -> bool {
        let title = format!("gatenuke_title {}", self.base.pid);
        g.ecl.get_button(&title).is_some()
    }
    fn set_target(&mut self, _g: &mut Ctx, _target: ObjRef, _targetstring: &str, _targetint: i32) {}
    fn move_to(&mut self, _g: &mut Ctx, _x: i32, _y: i32, _time_ms: i32) {}
    crate::impl_as_any!();
}
