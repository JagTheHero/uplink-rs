//! Port of source/interface/taskmanager/lanprobe.h + source/interface/taskmanager/lanprobe.cpp

use uplink_engine::{gl, sound, text};

use crate::app::globals::ALPHA;
use crate::app::opengl_interface;
use crate::ctx::Ctx;
use crate::eclipse::{Button, MOVE_STRAIGHTLINE};
use crate::game::data::data::{LAN_LINKPORTRANGE, TICKSREQUIRED_SCANLANLINKS, TICKSREQUIRED_SCANLANSYSTEM};
use crate::interface::localinterface::lan_interface::get_lan_interface_object;
use crate::interface::taskmanager::uplinktask::{UplinkTask, UplinkTaskBase};
use crate::objref::ObjRef;
use crate::vanbakel::Vanbakel;
use crate::world::computer::lancomputer::{
    LanComputer, LANLINKVISIBLE_AWARE, LANLINKVISIBLE_FROMAWARE, LANLINKVISIBLE_TOAWARE,
    LANSYSTEMVISIBLE_AWARE, LANSYSTEMVISIBLE_TYPE,
};
use crate::world::computer::lanmonitor::LanMonitor;

pub const LANPROBE_UNUSED: i32 = 0;
pub const LANPROBE_SCANNINGSYSTEM: i32 = 1;
pub const LANPROBE_SCANNINGLINKS: i32 = 2;
pub const LANPROBE_FINISHED: i32 = 3;

#[derive(Debug)]
pub struct LanProbe {
    pub base: UplinkTaskBase,
    pub status: i32,
    pub progress: i32,
    pub numticksrequired: i32,
    /// C++ `LanComputer *comp`: name of the LAN computer (key into the world).
    pub comp: Option<String>,
    /// C++ `systemIndex`
    pub systemindex: i32,
}

/// Names of the four buttons of the probe window (`lanprobe_title/border/progress/close <pid>`).
fn window_names(pid: i32) -> [String; 4] {
    [
        format!("lanprobe_title {pid}"),
        format!("lanprobe_border {pid}"),
        format!("lanprobe_progress {pid}"),
        format!("lanprobe_close {pid}"),
    ]
}

/// C++ `sscanf(button->name, "%s %d", bname, &pid)`: the PID after the button's first word.
fn pid_from_button_name(name: &str) -> Option<i32> {
    name.split_whitespace().nth(1)?.parse().ok()
}

/// C++ `comp` (a `LanComputer *`): the LAN computer with this name, if the world has one.
fn lan_ref<'a>(g: &'a Ctx, comp: Option<&str>) -> Option<&'a LanComputer> {
    g.world().get_computer(comp?)?.lan()
}

fn lan_mut<'a>(g: &'a mut Ctx, comp: &str) -> Option<&'a mut LanComputer> {
    g.world_mut().get_computer_mut(comp)?.lan_mut()
}

/// `!comp || !comp->systems.ValidIndex(index)`
fn no_valid_system(g: &Ctx, comp: Option<&str>, index: i32) -> bool {
    !lan_ref(g, comp).is_some_and(|lan| lan.systems.valid_index(index))
}

impl LanProbe {
    pub fn new() -> Self {
        LanProbe {
            base: UplinkTaskBase {
                version: 1.0,
                targetint: -1,
                ..Default::default()
            },
            status: LANPROBE_UNUSED,
            progress: 0,
            numticksrequired: 0,
            comp: None,
            systemindex: 0,
        }
    }

    /// C++ `LanProbe::BorderDraw` (static button callback).
    pub fn border_draw(_g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        let (a, b) = if clicked {
            ([0.5, 0.5, 0.6], [0.7, 0.7, 0.6])
        } else if highlighted {
            ([0.2, 0.2, 0.5], [0.5, 0.5, 0.6])
        } else {
            ([0.2, 0.2, 0.4], [0.3, 0.3, 0.5])
        };

        gl::begin(gl::QUADS);

        gl::color4f(a[0], a[1], a[2], ALPHA);
        gl::vertex2i(button.x, button.y);

        gl::color4f(b[0], b[1], b[2], ALPHA);
        gl::vertex2i(button.x + button.width, button.y);

        gl::color4f(a[0], a[1], a[2], ALPHA);
        gl::vertex2i(button.x + button.width, button.y + button.height);

        gl::color4f(b[0], b[1], b[2], ALPHA);
        gl::vertex2i(button.x, button.y + button.height);

        gl::end();

        if highlighted || clicked {
            gl::color4f(0.3, 0.3, 0.7, 1.0);
            opengl_interface::border_draw(button);
        }
    }

    /// C++ `LanProbe::ProgressDraw` (static button callback).
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

    /// C++ `LanProbe::CloseClick` (static button callback).
    pub fn close_click(g: &mut Ctx, button: &Button) {
        let Some(pid) = pid_from_button_name(&button.name) else { return };
        Vanbakel::remove_task(g, pid);
    }

    /// C++ `LanProbe::BorderClick` (static button callback).
    pub fn border_click(g: &mut Ctx, button: &Button) {
        let Some(pid) = pid_from_button_name(&button.name) else { return };
        g.interface_mut().tm.set_target_program(pid);
    }
}

impl Default for LanProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl UplinkTask for LanProbe {
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
        let sprogress = format!("lanprobe_progress {pid}");

        if self.status == LANPROBE_UNUSED {
            if self.comp.is_some() {
                g.ecl.register_caption_change(&sprogress, "Scanning system...", 0, None);
                self.status = LANPROBE_SCANNINGSYSTEM;
            }
        } else if self.status == LANPROBE_SCANNINGSYSTEM {
            // Check the system still exists
            if no_valid_system(g, self.comp.as_deref(), self.systemindex) {
                Vanbakel::remove_task(g, pid);
                return;
            }

            // Check we are still connected
            if !g.player().is_connected() {
                Vanbakel::remove_task(g, pid);
                return;
            }

            let sysidx = self.systemindex;
            let system_visible = lan_ref(g, self.comp.as_deref())
                .and_then(|lan| lan.systems.get_data(sysidx))
                .map_or(0, |system| system.visible);

            self.progress += n;
            if let Some(button) = g.ecl.get_button_mut(&sprogress) {
                button.width = (60.0 * (self.progress as f32 / self.numticksrequired as f32)) as i32;
            }
            g.ecl.dirty_button(&sprogress);

            if system_visible >= LANSYSTEMVISIBLE_TYPE || self.progress >= self.numticksrequired {
                if let Some(system) = self
                    .comp
                    .as_deref()
                    .and_then(|c| lan_mut(g, c))
                    .and_then(|lan| lan.systems.get_data_mut(sysidx))
                {
                    system.increase_visibility(LANSYSTEMVISIBLE_TYPE);
                }

                g.ecl.register_caption_change(&sprogress, "Scanning links...", 0, None);
                self.status = LANPROBE_SCANNINGLINKS;
                self.progress = 0;
                self.numticksrequired = TICKSREQUIRED_SCANLANLINKS;
            }
        } else if self.status == LANPROBE_SCANNINGLINKS {
            // Check the system still exists
            if no_valid_system(g, self.comp.as_deref(), self.systemindex) {
                Vanbakel::remove_task(g, pid);
                return;
            }

            // Check we are still connected
            if !g.player().is_connected() {
                Vanbakel::remove_task(g, pid);
                return;
            }

            let sysidx = self.systemindex;

            self.progress += n;
            if let Some(button) = g.ecl.get_button_mut(&sprogress) {
                button.width = (60.0 + 60.0 * (self.progress as f32 / self.numticksrequired as f32)) as i32;
            }
            g.ecl.dirty_button(&sprogress);

            if LanMonitor::is_accessable(g, sysidx) {
                // Every link with a port number <= this has been scanned
                let port_done = (LAN_LINKPORTRANGE as f32 * (self.progress as f32 / TICKSREQUIRED_SCANLANLINKS as f32)) as i32;
                let version = self.base.version;

                if let Some(lan) = self.comp.as_deref().and_then(|c| lan_mut(g, c)) {
                    for i in 0..lan.links.size() {
                        let Some(link) = lan.links.get_data_mut(i) else { continue };

                        if link.visible < LANLINKVISIBLE_AWARE && link.port <= port_done {
                            if link.from == sysidx {
                                sound::play_sound("sounds/done.wav", None, true);

                                if version >= link.security as f32 {
                                    link.increase_visibility(LANLINKVISIBLE_AWARE);

                                    if let Some(target) = lan.systems.get_data_mut(link.to) {
                                        target.increase_visibility(LANSYSTEMVISIBLE_AWARE);
                                    }
                                } else {
                                    link.increase_visibility(LANLINKVISIBLE_FROMAWARE);
                                }
                            } else if link.to == sysidx {
                                sound::play_sound("sounds/done.wav", None, true);

                                if version >= link.security as f32 {
                                    link.increase_visibility(LANLINKVISIBLE_AWARE);

                                    if let Some(target) = lan.systems.get_data_mut(link.from) {
                                        target.increase_visibility(LANSYSTEMVISIBLE_AWARE);
                                    }
                                } else {
                                    link.increase_visibility(LANLINKVISIBLE_TOAWARE);
                                }
                            }
                        }
                    }
                }
            }

            if self.progress >= TICKSREQUIRED_SCANLANLINKS {
                g.ecl.register_caption_change(&sprogress, "Finished", 0, None);
                self.status = LANPROBE_FINISHED;
            }
        } else if self.status == LANPROBE_FINISHED {
            self.progress = 0;
            self.systemindex = -1;
            self.comp = None;
            self.status = LANPROBE_UNUSED;
        }
    }

    fn create_interface(&mut self, g: &mut Ctx) {
        if self.is_interface_visible(g) {
            return;
        }

        let [stitle, sborder, sprogress, sclose] = window_names(self.base.pid);
        let tooltip = format!("LAN Probe v{:.1}", self.base.version);

        g.ecl.register_button(265, 450, 20, 15, "", &tooltip, &stitle);
        opengl_interface::button_assignbitmap(g, &stitle, "software/lan.tif");

        g.ecl.register_button(285, 450, 120, 15, "", "", &sborder);
        g.ecl.register_button_callbacks(
            &sborder,
            Some(Self::border_draw),
            Some(Self::border_click),
            Some(opengl_interface::button_click),
            Some(opengl_interface::button_highlight),
        );

        g.ecl.register_button(285, 450, 120, 13, "Select target system", "Shows the progress of the LAN Probe", &sprogress);
        g.ecl.register_button_callbacks(
            &sprogress,
            Some(Self::progress_draw),
            Some(Self::border_click),
            Some(opengl_interface::button_click),
            Some(opengl_interface::button_highlight),
        );

        g.ecl.register_button(405, 450, 13, 13, "", "Close the LAN Probe", &sclose);
        opengl_interface::button_assignbitmaps(g, &sclose, "close.tif", "close_h.tif", "close_c.tif");
        g.ecl.register_button_callback(&sclose, Self::close_click);
    }

    fn remove_interface(&mut self, g: &mut Ctx) {
        if !self.is_interface_visible(g) {
            return;
        }

        let [stitle, sborder, sprogress, sclose] = window_names(self.base.pid);

        g.ecl.remove_button(&stitle);
        g.ecl.remove_button(&sborder);
        g.ecl.remove_button(&sprogress);
        g.ecl.remove_button(&sclose);
    }

    fn show_interface(&mut self, g: &mut Ctx) {
        if !self.is_interface_visible(g) {
            self.create_interface(g);
        }

        let [stitle, sborder, sprogress, sclose] = window_names(self.base.pid);

        g.ecl.button_bring_to_front(&stitle);
        g.ecl.button_bring_to_front(&sborder);
        g.ecl.button_bring_to_front(&sprogress);
        g.ecl.button_bring_to_front(&sclose);
    }

    fn is_interface_visible(&self, g: &Ctx) -> bool {
        let [_, sborder, _, _] = window_names(self.base.pid);
        g.ecl.get_button(&sborder).is_some()
    }

    fn set_target(&mut self, g: &mut Ctx, target: ObjRef, targetstring: &str, targetint: i32) {
        // uo  : LanComputer object
        // uos : name of button representing the system
        // uoi : index of system in the lan computer
        if self.status == LANPROBE_UNUSED || self.status == LANPROBE_FINISHED {
            let ObjRef::Computer(name) = target else { return };

            // Only a LAN computer can be probed
            if lan_ref(g, Some(&name)).is_none() {
                return;
            }
            self.comp = Some(name);

            let system_height = {
                let Some(lan) = lan_ref(g, self.comp.as_deref()) else { return };
                if !lan.systems.valid_index(targetint) {
                    return;
                }
                self.systemindex = targetint;
                let Some(system) = lan.systems.get_data(targetint) else { return };
                get_lan_interface_object(system.type_).height
            };

            let (bx, by) = g
                .ecl
                .get_button(targetstring)
                .map(|button| (button.x, button.y))
                .expect("LanProbe::set_target: system button missing");
            self.move_to(g, bx - 20, by + system_height + 5, 300);

            self.progress = 0;
            self.numticksrequired = TICKSREQUIRED_SCANLANSYSTEM;
        }
    }

    fn move_to(&mut self, g: &mut Ctx, x: i32, y: i32, time_ms: i32) {
        // The C++ base UplinkTask::MoveTo does nothing, so only the probe's own buttons move.
        let [stitle, sborder, sprogress, sclose] = window_names(self.base.pid);

        g.ecl.register_movement(&stitle, x, y, time_ms, MOVE_STRAIGHTLINE, None);
        g.ecl.register_movement(&sborder, x + 20, y, time_ms, MOVE_STRAIGHTLINE, None);
        g.ecl.register_movement(&sprogress, x + 20, y + 1, time_ms, MOVE_STRAIGHTLINE, None);
        g.ecl.register_movement(&sclose, x + 140, y + 1, time_ms, MOVE_STRAIGHTLINE, None);

        g.ecl.button_bring_to_front(&stitle);
        g.ecl.button_bring_to_front(&sborder);
        g.ecl.button_bring_to_front(&sprogress);
        g.ecl.button_bring_to_front(&sclose);
    }

    crate::impl_as_any!();
}
