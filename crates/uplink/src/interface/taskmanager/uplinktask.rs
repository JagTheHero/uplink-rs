//! Port of vanbakel's `Task` + Uplink's `UplinkTask`: the base of every piece of running
//! software (password breaker, log deleter, tracer, ...).

use std::any::Any;

use crate::ctx::Ctx;
use crate::objref::ObjRef;

/// Data every task carries (C++ `UplinkTask` members + the vanbakel PID).
#[derive(Debug, Clone, Default)]
pub struct UplinkTaskBase {
    /// Set by the task list when the task is registered (`SvbLookupPID(this)`).
    pub pid: i32,
    pub version: f32,
    pub followmouse: bool,
    pub targetobject: ObjRef,
    pub targetstring: String,
    pub targetint: i32,
}

pub trait UplinkTask: Any {
    fn base(&self) -> &UplinkTaskBase;
    fn base_mut(&mut self) -> &mut UplinkTaskBase;

    /// Called once when the task is registered.
    fn initialise(&mut self, _g: &mut Ctx) {}
    /// Called by the scheduler; `n` = number of ticks that elapsed.
    fn tick(&mut self, _g: &mut Ctx, _n: i32) {}
    fn create_interface(&mut self, _g: &mut Ctx) {}
    fn remove_interface(&mut self, _g: &mut Ctx) {}
    /// Bring to front (optional).
    fn show_interface(&mut self, _g: &mut Ctx) {}
    fn is_interface_visible(&self, _g: &Ctx) -> bool {
        false
    }

    /// `UplinkTask::SetTarget` default: just store the target.
    fn set_target(&mut self, _g: &mut Ctx, target: ObjRef, targetstring: &str, targetint: i32) {
        let b = self.base_mut();
        b.targetobject = target;
        b.targetstring = targetstring.to_string();
        b.targetint = targetint;
    }

    /// `UplinkTask::MoveTo` default: does nothing (tasks with a window override it).
    fn move_to(&mut self, _g: &mut Ctx, _x: i32, _y: i32, _time_ms: i32) {}

    fn set_version(&mut self, newversion: f32) {
        self.base_mut().version = newversion;
    }

    fn set_follow_mouse(&mut self, newfollowmouse: bool) {
        self.base_mut().followmouse = newfollowmouse;
    }

    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
