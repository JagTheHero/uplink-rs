//! Port of source/interface/taskmanager/lanspoof.h + source/interface/taskmanager/lanspoof.cpp

use uplink_engine::{gl, text};

use crate::app::globals::ALPHA;
use crate::app::opengl_interface;
use crate::ctx::Ctx;
use crate::eclipse::{Button, MOVE_STRAIGHTLINE};
use crate::game::data::data::TICKSREQUIRED_SCANLANSYSTEM;
use crate::impl_as_any;
use crate::interface::localinterface::lan_interface::get_lan_interface_object;
use crate::interface::taskmanager::uplinktask::{UplinkTask, UplinkTaskBase};
use crate::objref::ObjRef;
use crate::vanbakel::Vanbakel;
use crate::world::computer::lancomputer::LanComputerSystem;

pub const LANSPOOF_UNUSED: i32 = 0;
pub const LANSPOOF_SPOOFING: i32 = 1;
pub const LANSPOOF_FAILED: i32 = 2;
pub const LANSPOOF_SPOOFED: i32 = 3;

/// Lan Spoof: runs on a LAN viewer, targets a system on the LAN and spoofs the LAN into
/// believing you are the system you are spoofing.
#[derive(Debug, Default)]
pub struct LanSpoof {
    pub base: UplinkTaskBase,
    pub status: i32,
    pub progress: i32,
    pub numticksrequired: i32,
    /// C++ `LanComputer *comp`: `ObjRef::Computer(name)`, `ObjRef::None` for NULL.
    pub comp: ObjRef,
    pub system_index: i32,
}

/// The button names of one LanSpoof window (`lanspoof_title N` etc.).
struct Names {
    title: String,
    border: String,
    progress: String,
    close: String,
}

fn names(pid: i32) -> Names {
    Names {
        title: format!("lanspoof_title {pid}"),
        border: format!("lanspoof_border {pid}"),
        progress: format!("lanspoof_progress {pid}"),
        close: format!("lanspoof_close {pid}"),
    }
}

/// C++ `sscanf(button->name, "%s %d", bname, &pid)`: the PID is the second word of the name.
fn pid_from_button(button: &Button) -> Option<i32> {
    button.name.split_whitespace().nth(1)?.parse().ok()
}

/// Picks one of three colours: clicked, highlighted or normal (the C++ if/else-if chains).
fn shade(clicked: bool, highlighted: bool, on_click: (f32, f32, f32), on_highlight: (f32, f32, f32), normal: (f32, f32, f32)) -> (f32, f32, f32) {
    if clicked {
        on_click
    } else if highlighted {
        on_highlight
    } else {
        normal
    }
}

impl LanSpoof {
    pub fn new() -> Self {
        Self::default()
    }

    /// The LAN system we are spoofing (C++ `comp->systems.GetData(systemIndex)`), or `None` if
    /// there is no target computer, it is gone, or the system index is no longer valid.
    fn target_system<'a>(&self, g: &'a Ctx) -> Option<&'a LanComputerSystem> {
        let ObjRef::Computer(name) = &self.comp else { return None };
        g.world().get_computer(name)?.lan()?.systems.get_data(self.system_index)
    }

    pub fn border_draw(_g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        gl::begin(gl::QUADS);

        let (r, gc, b) = shade(clicked, highlighted, (0.5, 0.5, 0.6), (0.2, 0.2, 0.5), (0.2, 0.2, 0.4));
        gl::color4f(r, gc, b, ALPHA);
        gl::vertex2i(button.x, button.y);

        let (r, gc, b) = shade(clicked, highlighted, (0.7, 0.7, 0.6), (0.5, 0.5, 0.6), (0.3, 0.3, 0.5));
        gl::color4f(r, gc, b, ALPHA);
        gl::vertex2i(button.x + button.width, button.y);

        let (r, gc, b) = shade(clicked, highlighted, (0.5, 0.5, 0.6), (0.2, 0.2, 0.5), (0.2, 0.2, 0.4));
        gl::color4f(r, gc, b, ALPHA);
        gl::vertex2i(button.x + button.width, button.y + button.height);

        let (r, gc, b) = shade(clicked, highlighted, (0.7, 0.7, 0.6), (0.5, 0.5, 0.6), (0.3, 0.3, 0.5));
        gl::color4f(r, gc, b, ALPHA);
        gl::vertex2i(button.x, button.y + button.height);

        gl::end();

        if highlighted || clicked {
            gl::color4f(0.3, 0.3, 0.7, 1.0);
            opengl_interface::border_draw(button);
        }
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

    pub fn close_click(g: &mut Ctx, button: &Button) {
        let Some(pid) = pid_from_button(button) else { return };
        Vanbakel::remove_task(g, pid);
    }

    pub fn border_click(g: &mut Ctx, button: &Button) {
        let Some(pid) = pid_from_button(button) else { return };
        g.interface_mut().tm.set_target_program(pid);
    }
}

impl UplinkTask for LanSpoof {
    fn base(&self) -> &UplinkTaskBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut UplinkTaskBase {
        &mut self.base
    }

    fn set_target(&mut self, g: &mut Ctx, target: ObjRef, targetstring: &str, targetint: i32) {
        // target    : LanComputer object
        // targetstring : name of button representing system
        // targetint : index of system in lan computer

        if self.status == LANSPOOF_UNUSED || self.status == LANSPOOF_SPOOFED || self.status == LANSPOOF_FAILED {
            // uo->GetOBJECTID() == OID_LANCOMPUTER
            let ObjRef::Computer(name) = &target else { return };
            let Some(lan) = g.world().get_computer(name).and_then(|c| c.lan()) else { return };

            // The C++ stores comp before checking the index, so do the same.
            let index_ok = lan.systems.valid_index(targetint);
            let system_type = lan.systems.get_data(targetint).map(|s| s.type_);
            self.comp = target.clone();
            if !index_ok {
                return;
            }
            self.system_index = targetint;

            let Some(system_type) = system_type else { return };
            let lio_height = get_lan_interface_object(system_type).height;

            // Button *button = EclGetButton ( uos ); UplinkAssert ( button );
            let Some((button_x, button_y)) = g.ecl.get_button(targetstring).map(|b| (b.x, b.y)) else { return };
            self.move_to(g, button_x - 20, button_y + lio_height + 5, 300);

            self.progress = 0;
            self.numticksrequired = TICKSREQUIRED_SCANLANSYSTEM;
        }
    }

    fn move_to(&mut self, g: &mut Ctx, x: i32, y: i32, time_ms: i32) {
        // UplinkTask::MoveTo is empty in the C++ base class, so there is nothing to call here.

        let n = names(self.base.pid);

        g.ecl.register_movement(&n.title, x, y, time_ms, MOVE_STRAIGHTLINE, None);
        g.ecl.register_movement(&n.border, x + 20, y, time_ms, MOVE_STRAIGHTLINE, None);
        g.ecl.register_movement(&n.progress, x + 20, y + 1, time_ms, MOVE_STRAIGHTLINE, None);
        g.ecl.register_movement(&n.close, x + 140, y + 1, time_ms, MOVE_STRAIGHTLINE, None);

        g.ecl.button_bring_to_front(&n.title);
        g.ecl.button_bring_to_front(&n.border);
        g.ecl.button_bring_to_front(&n.progress);
        g.ecl.button_bring_to_front(&n.close);
    }

    fn initialise(&mut self, _g: &mut Ctx) {}

    fn tick(&mut self, g: &mut Ctx, n: i32) {
        if !self.is_interface_visible(g) {
            return;
        }
        let pid = self.base.pid;
        let names = names(pid);
        let sprogress = names.progress;

        if self.status == LANSPOOF_UNUSED {
            if !self.comp.is_none() {
                g.ecl.register_caption_change(&sprogress, "Spoofing system...", 0, None);
                self.status = LANSPOOF_SPOOFING;
            }
        } else if self.status == LANSPOOF_SPOOFING {
            // Check the system still exists
            if self.target_system(g).is_none() {
                Vanbakel::remove_task(g, pid);
                return;
            }

            // Check we are still connected
            if !g.player().is_connected() {
                Vanbakel::remove_task(g, pid);
                return;
            }

            let system_security = self.target_system(g).map(|s| s.security).unwrap_or(0);

            self.progress += n;
            let width = (120.0 * (self.progress as f32 / self.numticksrequired as f32)) as i32;
            match g.ecl.get_button_mut(&sprogress) {
                Some(b) => b.width = width,
                None => debug_assert!(false, "UplinkAssert: LAN Spoof progress button missing"),
            }
            g.ecl.dirty_button(&sprogress);

            if self.progress >= self.numticksrequired {
                if self.base.version >= system_security as f32 {
                    g.lan_monitor.set_current_spoof(self.system_index);
                    g.ecl.register_caption_change(&sprogress, "Spoofed", 0, None);
                    self.status = LANSPOOF_SPOOFED;
                } else {
                    g.ecl.register_caption_change(&sprogress, "Failed", 0, None);
                    self.status = LANSPOOF_FAILED;
                }
            }
        } else if self.status == LANSPOOF_FAILED {
            // Check the system still exists
            if self.target_system(g).is_none() {
                Vanbakel::remove_task(g, pid);
                return;
            }

            // Check we are still connected
            if !g.player().is_connected() {
                Vanbakel::remove_task(g, pid);
                return;
            }
        } else if self.status == LANSPOOF_SPOOFED {
            // Check the system still exists
            if self.target_system(g).is_none() {
                Vanbakel::remove_task(g, pid);
                return;
            }

            // Check we are still connected
            if !g.player().is_connected() {
                Vanbakel::remove_task(g, pid);
                return;
            }
        }
    }

    fn create_interface(&mut self, g: &mut Ctx) {
        if self.is_interface_visible(g) {
            return;
        }
        let n = names(self.base.pid);

        let tooltip = format!("LAN Spoof v{:.1}", self.base.version);

        g.ecl.register_button(265, 450, 20, 15, "", &tooltip, &n.title);
        opengl_interface::button_assignbitmap(g, &n.title, "software/lan.tif");

        g.ecl.register_button(285, 450, 120, 15, "", "", &n.border);
        g.ecl.register_button_callbacks(
            &n.border,
            Some(Self::border_draw),
            Some(Self::border_click),
            Some(opengl_interface::button_click),
            Some(opengl_interface::button_highlight),
        );

        g.ecl.register_button(285, 450, 120, 13, "Select target system", "Shows the progress of the LAN Spoof", &n.progress);
        g.ecl.register_button_callbacks(
            &n.progress,
            Some(Self::progress_draw),
            Some(Self::border_click),
            Some(opengl_interface::button_click),
            Some(opengl_interface::button_highlight),
        );

        g.ecl.register_button(405, 450, 13, 13, "", "Close the LAN Spoof", &n.close);
        opengl_interface::button_assignbitmaps(g, &n.close, "close.tif", "close_h.tif", "close_c.tif");
        g.ecl.register_button_callback(&n.close, Self::close_click);
    }

    fn remove_interface(&mut self, g: &mut Ctx) {
        if self.is_interface_visible(g) {
            let n = names(self.base.pid);

            g.ecl.remove_button(&n.title);
            g.ecl.remove_button(&n.border);
            g.ecl.remove_button(&n.progress);
            g.ecl.remove_button(&n.close);
        }
    }

    fn show_interface(&mut self, g: &mut Ctx) {
        if !self.is_interface_visible(g) {
            self.create_interface(g);
        }

        let n = names(self.base.pid);

        g.ecl.button_bring_to_front(&n.title);
        g.ecl.button_bring_to_front(&n.border);
        g.ecl.button_bring_to_front(&n.progress);
        g.ecl.button_bring_to_front(&n.close);
    }

    fn is_interface_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button(&names(self.base.pid).border).is_some()
    }

    impl_as_any!();
}
