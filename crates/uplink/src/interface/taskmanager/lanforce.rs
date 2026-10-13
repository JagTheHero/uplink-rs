//! Port of source/interface/taskmanager/lanforce.h + source/interface/taskmanager/lanforce.cpp

use uplink_engine::{gl, text};

use crate::app::globals::ALPHA;
use crate::app::opengl_interface as ogl;
use crate::ctx::Ctx;
use crate::eclipse::{Button, MOVE_STRAIGHTLINE};
use crate::game::data::data::TICKSREQUIRED_FORCELANLOCK;
use crate::interface::localinterface::lan_interface::get_lan_interface_object;
use crate::interface::taskmanager::uplinktask::{UplinkTask, UplinkTaskBase};
use crate::objref::ObjRef;
use crate::vanbakel::Vanbakel;
use crate::world::computer::lancomputer::LANSYSTEM_LOCK;
use crate::world::computer::lanmonitor::LanMonitor;

pub const LANFORCE_UNUSED: i32 = 0;
pub const LANFORCE_FORCING: i32 = 1;
pub const LANFORCE_FINISHED: i32 = 3;

#[derive(Debug)]
pub struct LanForce {
    pub base: UplinkTaskBase,
    pub status: i32,
    pub progress: i32,
    pub numticksrequired: i32,
    /// C++ `LanComputer *comp`: name of the LAN computer (key into the world).
    pub comp: Option<String>,
    /// C++ `systemIndex`
    pub systemindex: i32,
}

/// PID from a button name of the form `"<word> <pid>"` (C++ `sscanf(name, "%s %d", bname, &pid)`).
/// A name without a readable PID gives -1, which matches no task (C++ left `pid` uninitialised).
fn pid_of(name: &str) -> i32 {
    name.split_whitespace().nth(1).and_then(|t| t.parse().ok()).unwrap_or(-1)
}

/// Names of the four buttons the LAN Force window is made of: title, border, progress, close.
fn button_names(pid: i32) -> [String; 4] {
    [
        format!("lanforce_title {pid}"),
        format!("lanforce_border {pid}"),
        format!("lanforce_progress {pid}"),
        format!("lanforce_close {pid}"),
    ]
}

/// The gradient background of `LanForce::BorderDraw`.
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

impl LanForce {
    pub fn new() -> Self {
        LanForce {
            base: UplinkTaskBase {
                version: 1.0,
                targetint: -1,
                ..Default::default()
            },
            status: LANFORCE_UNUSED,
            progress: 0,
            numticksrequired: 0,
            comp: None,
            systemindex: 0,
        }
    }

    /// C++ `LanForce::BorderDraw` (static button callback).
    pub fn border_draw(_g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        draw_border_background(button, highlighted, clicked);

        if highlighted || clicked {
            gl::color4f(0.3, 0.3, 0.7, 1.0);
            ogl::border_draw(button);
        }
    }

    /// C++ `LanForce::ProgressDraw` (static button callback).
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

    /// C++ `LanForce::CloseClick` (static button callback).
    pub fn close_click(g: &mut Ctx, button: &Button) {
        let pid = pid_of(&button.name);
        Vanbakel::remove_task(g, pid);
    }

    /// C++ `LanForce::BorderClick` (static button callback).
    pub fn border_click(g: &mut Ctx, button: &Button) {
        let pid = pid_of(&button.name);
        g.interface_mut().tm.set_target_program(pid);
    }
}

impl Default for LanForce {
    fn default() -> Self {
        Self::new()
    }
}

impl UplinkTask for LanForce {
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
        let sprogress = format!("lanforce_progress {pid}");

        if self.status == LANFORCE_UNUSED {
            if self.comp.is_some() {
                g.ecl.register_caption_change(&sprogress, "Forcing Lock...", 0, None);
                self.status = LANFORCE_FORCING;
            }
        } else if self.status == LANFORCE_FORCING {
            // Check the system still exists
            let system_ok = self.comp.as_ref().is_some_and(|c| {
                g.world()
                    .get_computer(c)
                    .and_then(|comp| comp.lan())
                    .is_some_and(|lan| lan.systems.valid_index(self.systemindex))
            });
            if !system_ok {
                Vanbakel::remove_task(g, pid);
                return;
            }

            // Check we are still connected
            if !g.player().is_connected() {
                Vanbakel::remove_task(g, pid);
                return;
            }

            self.progress += n;
            debug_assert!(g.ecl.get_button(&sprogress).is_some());
            if let Some(b) = g.ecl.get_button_mut(&sprogress) {
                b.width = (120.0 * (self.progress as f32 / self.numticksrequired as f32)) as i32;
            }
            g.ecl.dirty_button(&sprogress);

            if self.progress >= self.numticksrequired {
                if let Some(compname) = self.comp.clone() {
                    let sysindex = self.systemindex;
                    if let Some(system) = g
                        .world_mut()
                        .get_computer_mut(&compname)
                        .and_then(|comp| comp.lan_mut())
                        .and_then(|lan| lan.systems.get_data_mut(sysindex))
                    {
                        system.data1 = 0;
                    }
                }

                LanMonitor::sys_admin_awaken(g);

                g.ecl.register_caption_change(&sprogress, "Finished", 0, None);
                self.status = LANFORCE_FINISHED;
            }
        } else if self.status == LANFORCE_FINISHED {
            self.progress = 0;
            self.systemindex = -1;
            self.comp = None;
            self.status = LANFORCE_UNUSED;
        }
    }

    fn create_interface(&mut self, g: &mut Ctx) {
        if self.is_interface_visible(g) {
            return;
        }

        let pid = self.base.pid;
        let [stitle, sborder, sprogress, sclose] = button_names(pid);
        let tooltip = format!("LAN Force v{:.1}", self.base.version);

        g.ecl.register_button(265, 450, 20, 15, "", &tooltip, &stitle);
        ogl::button_assignbitmap(g, &stitle, "software/lan.tif");

        g.ecl.register_button(285, 450, 120, 15, "", "", &sborder);
        g.ecl.register_button_callbacks(
            &sborder,
            Some(Self::border_draw),
            Some(Self::border_click),
            Some(ogl::button_click),
            Some(ogl::button_highlight),
        );

        g.ecl.register_button(285, 450, 120, 13, "Select target system", "Shows the progress of the LAN Force", &sprogress);
        g.ecl.register_button_callbacks(
            &sprogress,
            Some(Self::progress_draw),
            Some(Self::border_click),
            Some(ogl::button_click),
            Some(ogl::button_highlight),
        );

        g.ecl.register_button(405, 450, 13, 13, "", "Close the LAN Force", &sclose);
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
        let border = format!("lanforce_border {}", self.base.pid);
        g.ecl.get_button(&border).is_some()
    }

    fn set_target(&mut self, g: &mut Ctx, target: ObjRef, targetstring: &str, targetint: i32) {
        // target : LanComputer object
        // targetstring : name of button representing system
        // targetint : index of system in lan computer

        if self.status != LANFORCE_UNUSED && self.status != LANFORCE_FINISHED {
            return;
        }

        // uo->GetOBJECTID() == OID_LANCOMPUTER
        let ObjRef::Computer(compname) = target else {
            return;
        };
        if g.world().get_computer(&compname).and_then(|c| c.lan()).is_none() {
            return;
        }

        self.comp = Some(compname.clone());
        let uoi = targetint;
        let system_type = match g
            .world()
            .get_computer(&compname)
            .and_then(|c| c.lan())
            .and_then(|lan| lan.systems.get_data(uoi))
        {
            Some(system) => system.type_,
            None => return,
        };
        self.systemindex = uoi;
        let lio = get_lan_interface_object(system_type);

        if system_type == LANSYSTEM_LOCK {
            let Some((bx, by)) = g.ecl.get_button(targetstring).map(|b| (b.x, b.y)) else {
                debug_assert!(false, "LanForce::set_target: no button {targetstring}");
                return;
            };
            self.move_to(g, bx - 20, by + lio.height + 5, 300);

            self.progress = 0;
            self.numticksrequired = TICKSREQUIRED_FORCELANLOCK;
        } else {
            self.comp = None;
            self.systemindex = 0;
        }
    }

    fn move_to(&mut self, g: &mut Ctx, x: i32, y: i32, time_ms: i32) {
        // UplinkTask::MoveTo is empty in C++.
        let pid = self.base.pid;
        let [stitle, sborder, sprogress, sclose] = button_names(pid);

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
