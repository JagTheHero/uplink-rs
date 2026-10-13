//! Port of source/interface/taskmanager/securitybypass.h + source/interface/taskmanager/securitybypass.cpp

use std::rc::Rc;

use crate::app::opengl_interface;
use crate::ctx::Ctx;
use crate::eclipse::{Button, MOVE_STRAIGHTLINE};
use crate::impl_as_any;
use crate::interface::taskmanager::uplinktask::{UplinkTask, UplinkTaskBase};
use crate::objref::ObjRef;
use crate::vanbakel::Vanbakel;
use crate::world::computer::securitysystem::SecuritySystem;

/// Security Bypass: similar to disable but runs continuously, bypassing a single proxy server.
/// A generic task designed to run on any security system.
#[derive(Debug, Default)]
pub struct SecurityBypass {
    pub base: UplinkTaskBase,
    /// Target IP.
    pub ip: String,
    /// Security system type (C++ `TYPE`).
    pub type_: i32,
    /// 0 = static, 1 = working, 2 = paused.
    pub status: i32,
    /// C++ `UplinkObject *targetobject`: `ObjRef::Security(name)` of the target, `ObjRef::None` for NULL.
    pub targetobject: ObjRef,
    /// C++ `int targetint`: index of the target security system.
    pub targetint: i32,
}

/// The button names of one SecurityBypass window (`sbypass_main N` etc.).
struct Names {
    main: String,
    pause: String,
    close: String,
    title: String,
    status: String,
}

fn names(pid: i32) -> Names {
    Names {
        main: format!("sbypass_main {pid}"),
        pause: format!("sbypass_pause {pid}"),
        close: format!("sbypass_close {pid}"),
        title: format!("sbypass_title {pid}"),
        status: format!("sbypass_status {pid}"),
    }
}

/// C++ `sscanf(button->name, "%s %d", bname, &pid)`: the PID is the second word of the name.
fn pid_from_button(button: &Button) -> Option<i32> {
    button.name.split_whitespace().nth(1)?.parse().ok()
}

/// C++ `((Security *) targetobject)->GetSystem (targetint)`, for a `Security` target.
fn system_ref<'a>(g: &'a Ctx, target: &ObjRef, index: i32) -> Option<&'a SecuritySystem> {
    let ObjRef::Security(name) = target else { return None };
    g.world().get_computer(name)?.security.get_system(index)
}

fn system_mut<'a>(g: &'a mut Ctx, target: &ObjRef, index: i32) -> Option<&'a mut SecuritySystem> {
    let ObjRef::Security(name) = target else { return None };
    g.world_mut().get_computer_mut(name)?.security.get_system_mut(index)
}

/// Runs `f` on the target system. C++ asserts the system exists, so a missing one is a bug.
fn with_target_system(g: &mut Ctx, target: &ObjRef, index: i32, f: impl FnOnce(&mut SecuritySystem)) {
    match system_mut(g, target, index) {
        Some(ss) => f(ss),
        None => debug_assert!(false, "UplinkAssert: SecurityBypass target system missing"),
    }
}

impl SecurityBypass {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_type(new_type: i32) -> Self {
        let mut s = Self::new();
        s.set_type(new_type);
        s
    }

    pub fn set_type(&mut self, new_type: i32) {
        self.type_ = new_type;
    }

    pub fn pause_current_bypass(&mut self, g: &mut Ctx) {
        if self.status == 1 {
            with_target_system(g, &self.targetobject, self.targetint, |ss| ss.end_bypass());
            self.status = 2;
        } else {
            println!("WARNING : SecurityBypass::PauseCurrentBypass, no bypass in existence!");
        }
    }

    pub fn resume_current_bypass(&mut self, g: &mut Ctx) {
        if self.status == 2 {
            with_target_system(g, &self.targetobject, self.targetint, |ss| ss.bypass());
            self.status = 1;
        } else {
            println!("WARNING : SecurityBypass::ResumeCurrentBypass, not paused to begin with!");
        }
    }

    pub fn end_current_bypass(&mut self, g: &mut Ctx) {
        if self.status > 0 {
            with_target_system(g, &self.targetobject, self.targetint, |ss| ss.end_bypass());
            self.targetobject = ObjRef::None;
            self.targetint = 0;
            self.status = 0;
        }
    }

    pub fn pause_click(g: &mut Ctx, button: &Button) {
        let Some(pid) = pid_from_button(button) else { return };
        Vanbakel::with_task(g, pid, |t, g| {
            let Some(pb) = t.as_any_mut().downcast_mut::<SecurityBypass>() else { return };
            if pb.status == 1 {
                pb.pause_current_bypass(g);
            } else if pb.status == 2 {
                pb.resume_current_bypass(g);
            }
        });
    }

    pub fn close_click(g: &mut Ctx, button: &Button) {
        let Some(pid) = pid_from_button(button) else { return };
        Vanbakel::with_task(g, pid, |t, g| {
            let Some(pb) = t.as_any_mut().downcast_mut::<SecurityBypass>() else { return };
            pb.end_current_bypass(g);
        });
        Vanbakel::remove_task(g, pid);
    }

    pub fn bypass_click(g: &mut Ctx, button: &Button) {
        let Some(pid) = pid_from_button(button) else { return };
        g.interface_mut().tm.set_target_program(pid);
    }

    pub fn status_light_draw(g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        let Some(pid) = pid_from_button(button) else { return };
        let Some((status, target, index)) = g.svb.get_task::<SecurityBypass>(pid).map(|t| (t.status, t.targetobject.clone(), t.targetint)) else {
            return;
        };

        let use_standard = if status > 0 {
            let Some(ss) = system_ref(g, &target, index) else {
                debug_assert!(false, "UplinkAssert: SecurityBypass target system missing");
                return;
            };
            // C L E V E R ! !
            ss.bypassed || !ss.enabled
        } else {
            false
        };

        let image = if use_standard { &button.image_standard } else { &button.image_highlighted };
        if let Some(img) = image {
            img.draw(button.x, button.y);
        }
    }
}

impl UplinkTask for SecurityBypass {
    fn base(&self) -> &UplinkTaskBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut UplinkTaskBase {
        &mut self.base
    }

    fn set_target(&mut self, g: &mut Ctx, target: ObjRef, targetstring: &str, targetint: i32) {
        // target    : Security object that belongs to target computer
        // targetstring : button name representing system in connection analyser
        // targetint : index of security system

        let ObjRef::Security(name) = &target else { return };

        let Some((system_type, level)) = g
            .world()
            .get_computer(name)
            .and_then(|c| c.security.get_system(targetint))
            .map(|s| (s.type_, s.level))
        else {
            debug_assert!(false, "UplinkAssert: SecurityBypass set_target system missing");
            return;
        };

        if system_type != self.type_ {
            return;
        }

        // If we were already running, stop
        self.end_current_bypass(g);

        // Move the button into place
        let Some((bx, by)) = g.ecl.get_button(targetstring).map(|b| (b.x, b.y + b.height)) else {
            debug_assert!(false, "UplinkAssert: SecurityBypass set_target button missing");
            return;
        };
        self.move_to(g, bx, by, 1000);

        // Store our target
        self.targetobject = target;
        self.targetint = targetint;
        self.ip = g.player().remotehost.clone();

        // Start it running
        if self.base.version >= level as f32 {
            with_target_system(g, &self.targetobject, self.targetint, |ss| ss.bypass());
            self.status = 1;
        } else {
            self.status = 0;
        }
    }

    fn move_to(&mut self, g: &mut Ctx, x: i32, y: i32, time_ms: i32) {
        // X and Y of bottom left corner
        if !self.is_interface_visible(g) {
            return;
        }
        let n = names(self.base.pid);

        g.ecl.button_bring_to_front(&n.main);
        g.ecl.button_bring_to_front(&n.pause);
        g.ecl.button_bring_to_front(&n.close);
        g.ecl.button_bring_to_front(&n.title);
        g.ecl.button_bring_to_front(&n.status);

        g.ecl.register_movement(&n.main, x, y - 32, time_ms, MOVE_STRAIGHTLINE, None);
        g.ecl.register_movement(&n.pause, x + 1, y - 31, time_ms, MOVE_STRAIGHTLINE, None);
        g.ecl.register_movement(&n.close, x + 19, y - 31, time_ms, MOVE_STRAIGHTLINE, None);
        g.ecl.register_movement(&n.title, x + 1, y - 13, time_ms, MOVE_STRAIGHTLINE, None);
        g.ecl.register_movement(&n.status, x + 19, y - 13, time_ms, MOVE_STRAIGHTLINE, None);
    }

    fn initialise(&mut self, _g: &mut Ctx) {}

    fn tick(&mut self, g: &mut Ctx, _n: i32) {
        // Switch off if we have disconnected
        if self.status > 0 && g.player().remotehost != self.ip {
            self.end_current_bypass(g);
            Vanbakel::remove_task(g, self.base.pid);
        }
    }

    fn create_interface(&mut self, g: &mut Ctx) {
        if self.is_interface_visible(g) {
            return;
        }
        let n = names(self.base.pid);

        g.ecl.register_button(100, 100, 32, 32, "Bypass", "Click to use this proxy bypass", &n.main);
        opengl_interface::button_assignbitmaps(g, &n.main, "software/proxybypass.tif", "software/proxybypass.tif", "software/proxybypass.tif");
        g.ecl.register_button_callback(&n.main, Self::bypass_click);

        g.ecl.register_button(101, 101, 12, 12, "P", "Pause the proxy bypass", &n.pause);
        opengl_interface::button_assignbitmaps(g, &n.pause, "software/pause.tif", "software/pause_h.tif", "software/pause_c.tif");
        g.ecl.register_button_callback(&n.pause, Self::pause_click);

        g.ecl.register_button(119, 101, 12, 12, "", "Close the proxy bypass", &n.close);
        opengl_interface::button_assignbitmaps(g, &n.close, "close.tif", "close_h.tif", "close_c.tif");
        if let Some(b) = g.ecl.get_button_mut(&n.close) {
            for img in [&mut b.image_standard, &mut b.image_highlighted, &mut b.image_clicked] {
                if let Some(img) = img {
                    Rc::make_mut(img).scale(12, 12);
                }
            }
        }
        g.ecl.register_button_callback(&n.close, Self::close_click);

        g.ecl.register_button(101, 119, 12, 12, "", "", &n.title);
        opengl_interface::button_assignbitmap(g, &n.title, "software/proxy.tif");
        g.ecl.register_button_callbacks(&n.title, Some(opengl_interface::imagebutton_draw), None, None, None);

        g.ecl.register_button(119, 119, 12, 12, "", "", &n.status);
        opengl_interface::button_assignbitmaps(g, &n.status, "software/greenlight.tif", "software/redlight.tif", "software/redlight.tif");
        g.ecl.register_button_callbacks(&n.status, Some(Self::status_light_draw), None, None, None);
    }

    fn remove_interface(&mut self, g: &mut Ctx) {
        if self.is_interface_visible(g) {
            let n = names(self.base.pid);

            g.ecl.remove_button(&n.main);
            g.ecl.remove_button(&n.pause);
            g.ecl.remove_button(&n.close);
            g.ecl.remove_button(&n.title);
            g.ecl.remove_button(&n.status);
        }
    }

    fn show_interface(&mut self, g: &mut Ctx) {
        if !self.is_interface_visible(g) {
            self.create_interface(g);
        }

        let n = names(self.base.pid);

        g.ecl.button_bring_to_front(&n.main);
        g.ecl.button_bring_to_front(&n.pause);
        g.ecl.button_bring_to_front(&n.close);
        g.ecl.button_bring_to_front(&n.title);
        g.ecl.button_bring_to_front(&n.status);
    }

    fn is_interface_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button(&names(self.base.pid).main).is_some()
    }

    impl_as_any!();
}
