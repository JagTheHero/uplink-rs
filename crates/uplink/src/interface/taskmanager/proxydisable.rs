//! Port of source/interface/taskmanager/proxydisable.h + source/interface/taskmanager/proxydisable.cpp

use uplink_engine::{gl, text};

use crate::app::globals::ALPHA;
use crate::app::opengl_interface::{button_assignbitmap, button_assignbitmaps};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::game::data::data::{TICKSREQUIRED_ANALYSEPROXY, TICKSREQUIRED_DISABLEPROXY};
use crate::impl_as_any;
use crate::interface::taskmanager::uplinktask::{UplinkTask, UplinkTaskBase};
use crate::objref::ObjRef;
use crate::vanbakel::Vanbakel;
use crate::world::computer::securitysystem::SECURITY_TYPE_PROXY;
use crate::world::connection::Connection;

/// Proxy Disable software, part of the Task Manager sub system.
#[derive(Debug, Default)]
pub struct ProxyDisable {
    pub base: UplinkTaskBase,
    /// Target IP.
    pub ip: String,
    /// 0 = static, 1 = analysing, 2 = disabling, 3 = unable to proceed, 4 = done.
    pub progress: i32,
    pub ticksdone: i32,
    /// Used to syncronise with task manager.
    pub ticksrequired: i32,
    /// Used to syncronise events to real time.
    pub timersync: i32,
}

/// `sscanf(name, "%s %d", bname, &pid)`: the number after the first word of a button name.
fn pid_from_name(name: &str) -> i32 {
    name.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(-1)
}

/// C++ `time(NULL)`: whole seconds since the Unix epoch.
fn unix_time() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// The computer behind VLocation `ip` (C++ `GetVLocation(ip)->GetComputer()`, both UplinkAsserted).
fn target_computer_name(g: &Ctx, ip: &str) -> String {
    let world = g.world();
    let vl = world.get_vlocation(ip).expect("ProxyDisable: UplinkAssert(vl)");
    vl.get_computer(world).expect("ProxyDisable: UplinkAssert(comp)").name.clone()
}

impl ProxyDisable {
    pub fn new() -> Self {
        Self::default()
    }

    /// The four button names of this task's window: title, border, progress, close.
    fn button_names(pid: i32) -> [String; 4] {
        [
            format!("proxydisable_title {pid}"),
            format!("proxydisable_border {pid}"),
            format!("proxydisable_progress {pid}"),
            format!("proxydisable_close {pid}"),
        ]
    }

    pub fn progress_draw(_g: &mut Ctx, button: &Button, highlighted: bool, _clicked: bool) {
        let mut scale = button.width as f32 / 100.0;
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

    pub fn border_draw(_g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        gl::begin(gl::QUADS);

        if clicked {
            gl::color4f(0.5, 0.5, 0.6, ALPHA);
        } else if highlighted {
            gl::color4f(0.2, 0.2, 0.5, ALPHA);
        } else {
            gl::color4f(0.2, 0.2, 0.4, ALPHA);
        }
        gl::vertex2i(button.x, button.y);

        if clicked {
            gl::color4f(0.7, 0.7, 0.6, ALPHA);
        } else if highlighted {
            gl::color4f(0.5, 0.5, 0.6, ALPHA);
        } else {
            gl::color4f(0.3, 0.3, 0.5, ALPHA);
        }
        gl::vertex2i(button.x + button.width, button.y);

        if clicked {
            gl::color4f(0.5, 0.5, 0.6, ALPHA);
        } else if highlighted {
            gl::color4f(0.2, 0.2, 0.5, ALPHA);
        } else {
            gl::color4f(0.2, 0.2, 0.4, ALPHA);
        }
        gl::vertex2i(button.x + button.width, button.y + button.height);

        if clicked {
            gl::color4f(0.7, 0.7, 0.6, ALPHA);
        } else if highlighted {
            gl::color4f(0.5, 0.5, 0.6, ALPHA);
        } else {
            gl::color4f(0.3, 0.3, 0.5, ALPHA);
        }
        gl::vertex2i(button.x, button.y + button.height);

        gl::end();

        if highlighted || clicked {
            gl::color4f(0.3, 0.3, 0.7, 1.0);
            crate::app::opengl_interface::border_draw(button);
        }
    }

    pub fn go_click(g: &mut Ctx, button: &Button) {
        let pid = pid_from_name(&button.name);

        // Look up the target (the current remote host)
        let remotehost = g.player().remotehost.clone();
        let numsystems = {
            let world = g.world();
            let vl = world.get_vlocation(&remotehost).expect("ProxyDisable: UplinkAssert(vl)");
            vl.get_computer(world).expect("ProxyDisable: UplinkAssert(comp)").security.num_systems()
        };

        let thistask = g.svb.get_task_mut::<ProxyDisable>(pid).expect("ProxyDisable: UplinkAssert(thistask)");

        // Set it going
        thistask.progress = 1;
        thistask.ip = remotehost;
        thistask.ticksrequired = TICKSREQUIRED_ANALYSEPROXY * numsystems;
        thistask.ticksdone = 0;

        // Update the display: the C++ caption change ("Analysing...") is commented out there.
    }

    pub fn close_click(g: &mut Ctx, button: &Button) {
        let pid = pid_from_name(&button.name);
        Vanbakel::remove_task(g, pid);
    }
}

impl UplinkTask for ProxyDisable {
    fn base(&self) -> &UplinkTaskBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut UplinkTaskBase {
        &mut self.base
    }
    /// Does nothing: the target comes from the current remote host.
    fn set_target(&mut self, _g: &mut Ctx, _target: ObjRef, _targetstring: &str, _targetint: i32) {}
    fn move_to(&mut self, _g: &mut Ctx, _x: i32, _y: i32, _time_ms: i32) {}
    fn initialise(&mut self, _g: &mut Ctx) {}

    fn tick(&mut self, g: &mut Ctx, _n: i32) {
        if !self.is_interface_visible(g) {
            return;
        }
        let pid = self.base.pid;

        // Stop the task if we've disconnected and we were running
        if self.progress > 0 && g.player().remotehost != self.ip {
            Vanbakel::remove_task(g, pid);
            return;
        }

        let sprogress = format!("proxydisable_progress {pid}");

        match self.progress {
            0 => {
                // Do nothing - waiting for user to click GO
            }
            1 => {
                // We are analysing
                self.ticksdone += 1;

                let width = (40.0 * (self.ticksdone as f32 / self.ticksrequired as f32)) as i32;
                g.ecl.get_button_mut(&sprogress).expect("ProxyDisable: progress button").width = width;
                g.ecl.dirty_button(&sprogress);

                if self.ticksdone >= self.ticksrequired {
                    // Start it disabling
                    self.progress = 2;

                    // Look up the target
                    let name = target_computer_name(g, &self.ip);
                    let comp = g.world().get_computer(&name).expect("ProxyDisable: UplinkAssert(comp)");

                    // Update the display: the C++ caption change ("Disabling...") is commented out there.

                    self.ticksrequired = TICKSREQUIRED_DISABLEPROXY * comp.security.num_systems();
                    self.ticksdone = 0;

                    // They start tracing us
                    let monitor = comp.security.is_running_monitor();
                    if monitor {
                        Connection::begin_trace(g, "PLAYER");
                    }
                }
            }
            2 => {
                // We are disabling
                self.ticksdone += 1;

                let width = (40.0 + 80.0 * (self.ticksdone as f32 / self.ticksrequired as f32)) as i32;
                g.ecl.get_button_mut(&sprogress).expect("ProxyDisable: progress button").width = width;
                g.ecl.dirty_button(&sprogress);

                if self.ticksdone >= self.ticksrequired {
                    // Finished - decide if we were successful or not.
                    // First look up the target
                    let name = target_computer_name(g, &self.ip);
                    let version = self.base.version;

                    let comp = g.world_mut().get_computer_mut(&name).expect("ProxyDisable: UplinkAssert(comp)");
                    let mut failed = false;
                    for i in 0..comp.security.num_systems() {
                        if let Some(ss) = comp.security.get_system_mut(i) {
                            if ss.type_ == SECURITY_TYPE_PROXY && ss.enabled && !ss.bypassed {
                                if version >= ss.level as f32 {
                                    ss.disable();
                                } else {
                                    failed = true;
                                }
                            }
                        }
                    }

                    if failed {
                        self.progress = 3;
                        g.ecl.register_caption_change_auto(&sprogress, "Failed", None);
                    } else {
                        self.progress = 4;
                        g.ecl.register_caption_change_auto(&sprogress, "Success", None);
                    }

                    self.timersync = (unix_time() + 2) as i32;
                }
            }
            3 => {
                // Unable to proceed due to insufficient version
                if unix_time() > self.timersync as i64 {
                    Vanbakel::remove_task(g, pid);
                }
            }
            4 => {
                // We are done
                if unix_time() > self.timersync as i64 {
                    Vanbakel::remove_task(g, pid);
                }
            }
            _ => {}
        }
    }

    fn create_interface(&mut self, g: &mut Ctx) {
        if !self.is_interface_visible(g) {
            let pid = self.base.pid;
            let [stitle, sborder, sprogress, sclose] = Self::button_names(pid);

            g.ecl.register_button(265, 422, 20, 15, "", "Disable any proxy servers running on this machine", &stitle);
            button_assignbitmap(g, &stitle, "software/go.tif");
            g.ecl.register_button_callback(&stitle, Self::go_click);

            g.ecl.register_button(285, 422, 120, 15, "", "", &sborder);
            g.ecl.register_button_callbacks(&sborder, Some(Self::border_draw), None, None, None);

            g.ecl.register_button(285, 423, 0, 13, "Proxy Disable", "Shows the progress of the disable", &sprogress);
            g.ecl.register_button_callbacks(&sprogress, Some(Self::progress_draw), None, None, None);

            g.ecl.register_button(405, 423, 13, 13, "", "Close the Proxy disabler", &sclose);
            button_assignbitmaps(g, &sclose, "close.tif", "close_h.tif", "close_c.tif");
            g.ecl.register_button_callback(&sclose, Self::close_click);
        }
    }

    fn remove_interface(&mut self, g: &mut Ctx) {
        if self.is_interface_visible(g) {
            for name in Self::button_names(self.base.pid) {
                g.ecl.remove_button(&name);
            }
        }
    }

    fn show_interface(&mut self, g: &mut Ctx) {
        if !self.is_interface_visible(g) {
            self.create_interface(g);
        }
        for name in Self::button_names(self.base.pid) {
            g.ecl.button_bring_to_front(&name);
        }
    }

    fn is_interface_visible(&self, g: &Ctx) -> bool {
        let [stitle, _, _, _] = Self::button_names(self.base.pid);
        g.ecl.get_button(&stitle).is_some()
    }

    impl_as_any!();
}
