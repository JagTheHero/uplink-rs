//! Port of source/interface/taskmanager/lanscan.h + source/interface/taskmanager/lanscan.cpp

use uplink_engine::{gl, sound, text};

use crate::app::globals::ALPHA;
use crate::app::opengl_interface as ogl;
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::game::data::data::{LAN_SUBNETRANGE, TICKSREQUIRED_LANSCAN};
use crate::interface::taskmanager::uplinktask::{UplinkTask, UplinkTaskBase};
use crate::objref::ObjRef;
use crate::vanbakel::Vanbakel;
use crate::world::computer::lancomputer::{LANSYSTEMVISIBLE_AWARE, LANSYSTEMVISIBLE_TYPE};

pub const LANSCAN_UNUSED: i32 = 0;
pub const LANSCAN_SCANNING: i32 = 1;
pub const LANSCAN_FINISHED: i32 = 2;

#[derive(Debug)]
pub struct LanScan {
    pub base: UplinkTaskBase,
    pub status: i32,
    pub progress: i32,
    pub numticksrequired: i32,
    /// Target IP (C++ `char ip[SIZE_VLOCATION_IP]`), set from the remote host on GO.
    pub ip: String,
}

/// PID from a button name of the form `"<word> <pid>"` (C++ `sscanf(name, "%s %d", bname, &pid)`).
/// A name without a readable PID gives -1, which matches no task (C++ left `pid` uninitialised).
fn pid_of(name: &str) -> i32 {
    name.split_whitespace().nth(1).and_then(|t| t.parse().ok()).unwrap_or(-1)
}

/// Names of the four buttons the LAN Scan window is made of: title, border, progress, close.
fn button_names(pid: i32) -> [String; 4] {
    [
        format!("lanscan_title {pid}"),
        format!("lanscan_border {pid}"),
        format!("lanscan_progress {pid}"),
        format!("lanscan_close {pid}"),
    ]
}

/// The gradient background of `LanScan::BorderDraw`.
fn draw_border_background(button: &Button, highlighted: bool, clicked: bool) {
    // Corners top-left and bottom-right use `dark`, top-right and bottom-left use `light`.
    let (dark, light): ([f32; 3], [f32; 3]) = if clicked {
        ([0.5, 0.5, 0.6], [0.7, 0.7, 0.6])
    } else if highlighted {
        ([0.2, 0.2, 0.5], [0.5, 0.5, 0.6])
    } else {
        ([0.2, 0.2, 0.4], [0.3, 0.3, 0.5])
    };
    let (x, y, w, h) = (button.x, button.y, button.width, button.height);

    gl::begin(gl::QUADS);
    gl::color4f(dark[0], dark[1], dark[2], ALPHA);
    gl::vertex2i(x, y);
    gl::color4f(light[0], light[1], light[2], ALPHA);
    gl::vertex2i(x + w, y);
    gl::color4f(dark[0], dark[1], dark[2], ALPHA);
    gl::vertex2i(x + w, y + h);
    gl::color4f(light[0], light[1], light[2], ALPHA);
    gl::vertex2i(x, y + h);
    gl::end();
}

/// Sets a LAN system's visibility, looked up through the world by computer name and system index.
fn increase_system_visibility(g: &mut Ctx, compname: &str, sysindex: i32, level: i32) {
    if let Some(system) = g
        .world_mut()
        .get_computer_mut(compname)
        .and_then(|comp| comp.lan_mut())
        .and_then(|lan| lan.systems.get_data_mut(sysindex))
    {
        system.increase_visibility(level);
    }
}

impl LanScan {
    pub fn new() -> Self {
        LanScan {
            base: UplinkTaskBase {
                version: 1.0,
                targetint: -1,
                ..Default::default()
            },
            status: LANSCAN_UNUSED,
            progress: 0,
            numticksrequired: 0,
            ip: "N".to_string(),
        }
    }

    /// C++ `LanScan::BorderDraw` (static button callback).
    pub fn border_draw(_g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        draw_border_background(button, highlighted, clicked);

        if highlighted || clicked {
            gl::color4f(0.3, 0.3, 0.7, 1.0);
            ogl::border_draw(button);
        }
    }

    /// C++ `LanScan::ProgressDraw` (static button callback).
    pub fn progress_draw(_g: &mut Ctx, button: &Button, highlighted: bool, _clicked: bool) {
        let mut scale: f32 = button.width as f32 / 100.0;
        if highlighted {
            scale *= 2.0;
        }

        gl::begin(gl::QUADS);
        gl::color4f(0.0, 1.5 - scale, scale, 0.6);
        gl::vertex3i(button.x, button.y, 0);
        gl::color4f(0.0, 1.5 - scale, scale, 0.6);
        gl::vertex3i(button.x + button.width, button.y, 0);
        gl::color4f(0.0, 1.5 - scale, scale, 0.6);
        gl::vertex3i(button.x + button.width, button.y + button.height, 0);
        gl::color4f(0.0, 1.5 - scale, scale, 0.6);
        gl::vertex3i(button.x, button.y + button.height, 0);
        gl::end();

        let xpos = button.x + 5;
        let ypos = (button.y + button.height / 2) + 3;

        gl::color4f(1.0, 1.0, 1.0, 1.0);
        text::draw_text_styled(xpos, ypos, &button.caption, text::HELVETICA_10);
    }

    /// C++ `LanScan::GoClick` (static button callback).
    pub fn go_click(g: &mut Ctx, button: &Button) {
        let pid = pid_of(&button.name);

        // strcpy ( thistask->ip, game->GetWorld ()->GetPlayer ()->remotehost );
        let remotehost = g.player().remotehost.clone();

        let Some(thistask) = g.svb.get_task_mut::<LanScan>(pid) else {
            debug_assert!(false, "LanScan::go_click: no LanScan task {pid}");
            return;
        };

        // Set it going
        thistask.ip = remotehost;
        thistask.numticksrequired = TICKSREQUIRED_LANSCAN;
        thistask.progress = 0;
        thistask.status = LANSCAN_SCANNING;

        let sprogress = format!("lanscan_progress {pid}");
        g.ecl.register_caption_change(&sprogress, "Scanning...", 0, None);
    }

    /// C++ `LanScan::CloseClick` (static button callback).
    pub fn close_click(g: &mut Ctx, button: &Button) {
        let pid = pid_of(&button.name);
        Vanbakel::remove_task(g, pid);
    }
}

impl Default for LanScan {
    fn default() -> Self {
        Self::new()
    }
}

impl UplinkTask for LanScan {
    fn base(&self) -> &UplinkTaskBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut UplinkTaskBase {
        &mut self.base
    }
    fn initialise(&mut self, _g: &mut Ctx) {}

    fn tick(&mut self, g: &mut Ctx, n: i32) {
        if !self.is_interface_visible(g) {
            return;
        }

        let pid = self.base.pid;

        // Stop the task if we've disconnected and we were running
        if self.progress > 0 && g.player().remotehost != self.ip {
            Vanbakel::remove_task(g, pid);
            return;
        }

        let sprogress = format!("lanscan_progress {pid}");

        if self.status == LANSCAN_SCANNING {
            let Some(vl) = g.world().get_vlocation(&self.ip) else {
                debug_assert!(false, "LanScan::tick: no VLocation {}", self.ip);
                return;
            };
            let Some(comp) = vl.get_computer(g.world()) else {
                debug_assert!(false, "LanScan::tick: no computer at {}", self.ip);
                return;
            };

            if comp.lan().is_none() {
                // comp->GetOBJECTID () != OID_LANCOMPUTER
                self.status = LANSCAN_FINISHED;
                return;
            }
            let compname = comp.name.clone();

            //
            // Have anymore systems become visible ?

            self.progress += n;

            if let Some(b) = g.ecl.get_button_mut(&sprogress) {
                b.width = (120.0 * (self.progress as f32 / self.numticksrequired as f32)) as i32;
            }
            g.ecl.dirty_button(&sprogress);

            let num_subnets_scanned =
                (LAN_SUBNETRANGE as f32 * (self.progress as f32 / self.numticksrequired as f32)) as i32;

            let version = self.base.version;
            let num_systems = g
                .world()
                .get_computer(&compname)
                .and_then(|c| c.lan())
                .map_or(0, |lan| lan.systems.size());

            for i in 0..num_systems {
                let Some((subnet, visible, security)) = g
                    .world()
                    .get_computer(&compname)
                    .and_then(|c| c.lan())
                    .and_then(|lan| lan.systems.get_data(i))
                    .map(|s| (s.subnet, s.visible, s.security))
                else {
                    continue;
                };

                if subnet <= num_subnets_scanned && visible < LANSYSTEMVISIBLE_TYPE {
                    // int difference = (version - 1) - system->security;  (float, truncated to int)
                    let difference = ((version - 1.0) - security as f32) as i32;

                    if difference == 0 {
                        increase_system_visibility(g, &compname, i, LANSYSTEMVISIBLE_AWARE);
                        sound::play_sound("sounds/done.wav", None, true);
                    } else if difference == 1 {
                        increase_system_visibility(g, &compname, i, LANSYSTEMVISIBLE_TYPE);
                        sound::play_sound("sounds/done.wav", None, true);
                    }
                }
            }

            //
            // Have we finished?

            if self.progress >= self.numticksrequired {
                self.status = LANSCAN_FINISHED;
            }
        } else if self.status == LANSCAN_FINISHED {
            g.ecl.register_caption_change(&sprogress, "Finished...", 0, None);
            self.progress = 0;
            self.numticksrequired = 0;
            self.status = LANSCAN_UNUSED;
        }
    }

    fn create_interface(&mut self, g: &mut Ctx) {
        if self.is_interface_visible(g) {
            return;
        }

        let pid = self.base.pid;
        let [stitle, sborder, sprogress, sclose] = button_names(pid);

        g.ecl.register_button(265, 422, 20, 15, "", "Scan the Local Area Network for systems", &stitle);
        ogl::button_assignbitmap(g, &stitle, "software/go.tif");
        g.ecl.register_button_callback(&stitle, Self::go_click);

        g.ecl.register_button(285, 422, 120, 15, "", "", &sborder);
        g.ecl.register_button_callbacks(&sborder, Some(Self::border_draw), None, None, None);

        g.ecl.register_button(285, 423, 0, 13, "LAN Scan", "Shows the progress of the scan", &sprogress);
        g.ecl.register_button_callbacks(&sprogress, Some(Self::progress_draw), None, None, None);

        g.ecl.register_button(405, 423, 13, 13, "", "Close the LAN Scan", &sclose);
        ogl::button_assignbitmaps(g, &sclose, "close.tif", "close_h.tif", "close_c.tif");
        g.ecl.register_button_callback(&sclose, Self::close_click);
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
        // Checks the title button (unlike the other tools, which check their border).
        let title = format!("lanscan_title {}", self.base.pid);
        g.ecl.get_button(&title).is_some()
    }

    fn set_target(&mut self, _g: &mut Ctx, _target: ObjRef, _targetstring: &str, _targetint: i32) {
        // Don't do anything - we get our target from the current remote host
    }

    fn move_to(&mut self, _g: &mut Ctx, _x: i32, _y: i32, _time_ms: i32) {}

    crate::impl_as_any!();
}
