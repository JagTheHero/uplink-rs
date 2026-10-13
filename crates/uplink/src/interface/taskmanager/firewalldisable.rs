//! Port of source/interface/taskmanager/firewalldisable.h + source/interface/taskmanager/firewalldisable.cpp

use uplink_engine::{gl, text};

use crate::app::globals::ALPHA;
use crate::app::opengl_interface;
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::game::data::data::{TICKSREQUIRED_ANALYSEFIREWALL, TICKSREQUIRED_DISABLEFIREWALL};
use crate::interface::taskmanager::uplinktask::{UplinkTask, UplinkTaskBase};
use crate::objref::ObjRef;
use crate::vanbakel::Vanbakel;
use crate::world::computer::securitysystem::SECURITY_TYPE_FIREWALL;
use crate::world::connection::Connection;

#[derive(Debug)]
pub struct FirewallDisable {
    pub base: UplinkTaskBase,
    /// Target IP (C++ `char ip[SIZE_VLOCATION_IP]`).
    pub ip: String,
    /// 0 = static, 1 = analysing, 2 = disabling, 3 = unable to proceed, 4 = done
    pub progress: i32,
    pub ticksdone: i32,
    /// Used to syncronise with task manager
    pub ticksrequired: i32,
    /// Used to syncronise events to real time (unix seconds)
    pub timersync: i64,
}

/// C++ `sscanf ( button->name, "%s %d", bname, &pid )`: the PID is the second word of the name.
fn parse_pid(name: &str) -> i32 {
    name.split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(-1)
}

/// C++ `time(NULL)`: unix seconds.
fn time_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// The button names of this task's window: title, border, progress, close.
fn window_names(pid: i32) -> (String, String, String, String) {
    (
        format!("firewalldisable_title {pid}"),
        format!("firewalldisable_border {pid}"),
        format!("firewalldisable_progress {pid}"),
        format!("firewalldisable_close {pid}"),
    )
}

/// Key (`Computer::name`) of the computer at `ip`: C++ `vl->GetComputer()` with both lookups asserted.
fn computer_name_at(g: &Ctx, ip: &str) -> String {
    let world = g.world();
    let vl = world.get_vlocation(ip).expect("UplinkAssert (vl)");
    let comp = vl.get_computer(world).expect("UplinkAssert (comp)");
    comp.name.clone()
}

impl FirewallDisable {
    pub fn new() -> Self {
        FirewallDisable {
            base: UplinkTaskBase {
                version: 1.0,
                targetint: -1,
                ..Default::default()
            },
            ip: String::new(),
            progress: 0,
            ticksdone: 0,
            ticksrequired: 0,
            timersync: 0,
        }
    }

    /// C++ `FirewallDisable::BorderDraw` (static button callback).
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
            opengl_interface::border_draw(button);
        }
    }

    /// C++ `FirewallDisable::ProgressDraw` (static button callback).
    pub fn progress_draw(_g: &mut Ctx, button: &Button, highlighted: bool, _clicked: bool) {
        // C++: (float) button->width / 100.0 is computed in double, then stored as float
        let mut scale = (button.width as f64 / 100.0) as f32;
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

    /// C++ `FirewallDisable::GoClick` (static button callback).
    pub fn go_click(g: &mut Ctx, button: &Button) {
        let pid = parse_pid(&button.name);

        // Look up the target (the remote host is the target; the C++ copied it into `ip` first)
        let remotehost = g.player().remotehost.clone();
        let comp_name = computer_name_at(g, &remotehost);
        let numsystems = g
            .world()
            .get_computer(&comp_name)
            .expect("UplinkAssert (comp)")
            .security
            .num_systems();

        // Set it going
        let thistask = g
            .svb
            .get_task_mut::<FirewallDisable>(pid)
            .expect("UplinkAssert (thistask)");
        thistask.progress = 1;
        thistask.ip = remotehost;
        thistask.ticksrequired = TICKSREQUIRED_ANALYSEFIREWALL * numsystems;
        thistask.ticksdone = 0;

        // Update the display
        // PORT: the C++ caption change to "Analysing..." is commented out there too
    }

    /// C++ `FirewallDisable::CloseClick` (static button callback).
    pub fn close_click(g: &mut Ctx, button: &Button) {
        let pid = parse_pid(&button.name);
        Vanbakel::remove_task(g, pid);
    }
}

impl Default for FirewallDisable {
    fn default() -> Self {
        Self::new()
    }
}

impl UplinkTask for FirewallDisable {
    fn base(&self) -> &UplinkTaskBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut UplinkTaskBase {
        &mut self.base
    }
    fn initialise(&mut self, _g: &mut Ctx) {}
    fn tick(&mut self, g: &mut Ctx, _n: i32) {
        // C++ FirewallDisable::Tick ignores n: progress is driven by ticksdone / ticksrequired
        if !self.is_interface_visible(g) {
            return;
        }
        let pid = self.base.pid;

        // Stop the task if we've disconnected and we were running
        if self.progress > 0 && g.player().remotehost != self.ip {
            Vanbakel::remove_task(g, pid);
            return;
        }

        let sprogress = format!("firewalldisable_progress {pid}");

        if self.progress == 0 {
            // Do nothing - waiting for user to click GO
        } else if self.progress == 1 {
            // We are analysing
            self.ticksdone += 1;

            let width = (40.0 * (self.ticksdone as f32 / self.ticksrequired as f32)) as i32;
            set_button_width(g, &sprogress, width);

            if self.ticksdone >= self.ticksrequired {
                // Start it disabling
                self.progress = 2;

                // Look up the target
                let comp_name = computer_name_at(g, &self.ip);
                let numsystems = comp_ref(g, &comp_name).security.num_systems();

                // Update the display
                // PORT: the C++ caption change to "Disabling..." is commented out there too

                self.ticksrequired = TICKSREQUIRED_DISABLEFIREWALL * numsystems;
                self.ticksdone = 0;

                if comp_ref(g, &comp_name).security.is_running_monitor() {
                    let owner = g.player().name.clone();
                    Connection::begin_trace(g, &owner);
                }
            }
        } else if self.progress == 2 {
            // We are disabling
            self.ticksdone += 1;

            let width = (40.0 + 80.0 * (self.ticksdone as f32 / self.ticksrequired as f32)) as i32;
            set_button_width(g, &sprogress, width);

            if self.ticksdone >= self.ticksrequired {
                // Finished - decide if we were successful or not
                // First Look up the target
                let comp_name = computer_name_at(g, &self.ip);
                let numsystems = comp_ref(g, &comp_name).security.num_systems();

                let mut failed = false;

                for i in 0..numsystems {
                    // ss && ss->TYPE == SECURITY_TYPE_FIREWALL && ss->enabled && !ss->bypassed
                    let firewall_level = match comp_ref(g, &comp_name).security.get_system(i) {
                        Some(ss) if ss.type_ == SECURITY_TYPE_FIREWALL && ss.enabled && !ss.bypassed => {
                            Some(ss.level)
                        }
                        _ => None,
                    };

                    if let Some(level) = firewall_level {
                        if self.base.version >= level as f32 {
                            if let Some(ss) = g
                                .world_mut()
                                .get_computer_mut(&comp_name)
                                .and_then(|c| c.security.get_system_mut(i))
                            {
                                ss.disable();
                            }
                            if comp_ref(g, &comp_name).security.is_running_monitor() {
                                let owner = g.player().name.clone();
                                Connection::begin_trace(g, &owner);
                            }
                        } else {
                            failed = true;
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

                self.timersync = time_now() + 2;
            }
        } else if self.progress == 3 {
            // Unable to proceed due to insufficient version
            if time_now() > self.timersync {
                Vanbakel::remove_task(g, pid);
            }
        } else if self.progress == 4 {
            // We are done
            if time_now() > self.timersync {
                Vanbakel::remove_task(g, pid);
            }
        }
    }

    fn create_interface(&mut self, g: &mut Ctx) {
        if !self.is_interface_visible(g) {
            let pid = self.base.pid;
            let (stitle, sborder, sprogress, sclose) = window_names(pid);

            g.ecl.register_button(265, 442, 20, 15, "", "Disable any firewalls running on this machine", &stitle);
            opengl_interface::button_assignbitmap(g, &stitle, "software/go.tif");
            g.ecl.register_button_callback(&stitle, Self::go_click);

            g.ecl.register_button(285, 442, 120, 15, "", "", &sborder);
            g.ecl.register_button_callbacks(&sborder, Some(Self::border_draw), None, None, None);

            g.ecl.register_button(285, 443, 0, 13, "Firewall Disable", "Shows the progress of the disable", &sprogress);
            g.ecl.register_button_callbacks(&sprogress, Some(Self::progress_draw), None, None, None);

            g.ecl.register_button(405, 443, 13, 13, "", "Close the Firewall disabler", &sclose);
            opengl_interface::button_assignbitmaps(g, &sclose, "close.tif", "close_h.tif", "close_c.tif");
            g.ecl.register_button_callback(&sclose, Self::close_click);
        }
    }

    fn remove_interface(&mut self, g: &mut Ctx) {
        if self.is_interface_visible(g) {
            let pid = self.base.pid;
            let (stitle, sborder, sprogress, sclose) = window_names(pid);

            g.ecl.remove_button(&stitle);
            g.ecl.remove_button(&sborder);
            g.ecl.remove_button(&sprogress);
            g.ecl.remove_button(&sclose);
        }
    }

    fn show_interface(&mut self, g: &mut Ctx) {
        if !self.is_interface_visible(g) {
            self.create_interface(g);
        }

        let pid = self.base.pid;
        let (stitle, sborder, sprogress, sclose) = window_names(pid);

        g.ecl.button_bring_to_front(&stitle);
        g.ecl.button_bring_to_front(&sborder);
        g.ecl.button_bring_to_front(&sprogress);
        g.ecl.button_bring_to_front(&sclose);
    }

    fn is_interface_visible(&self, g: &Ctx) -> bool {
        let stitle = format!("firewalldisable_title {}", self.base.pid);
        g.ecl.get_button(&stitle).is_some()
    }

    fn set_target(&mut self, _g: &mut Ctx, _target: ObjRef, _targetstring: &str, _targetint: i32) {
        // Don't do anything - we get our target from the current remote host
    }
    fn move_to(&mut self, _g: &mut Ctx, _x: i32, _y: i32, _time_ms: i32) {}
    crate::impl_as_any!();
}

/// C++ `EclGetButton ( sprogress )->width = w; EclDirtyButton ( sprogress );`
fn set_button_width(g: &mut Ctx, name: &str, width: i32) {
    if let Some(b) = g.ecl.get_button_mut(name) {
        b.width = width;
    }
    g.ecl.dirty_button(name);
}

/// The computer with this key (both callers have already asserted it exists).
fn comp_ref<'a>(g: &'a Ctx, comp_name: &str) -> &'a crate::world::computer::computer::Computer {
    g.world().get_computer(comp_name).expect("UplinkAssert (comp)")
}
