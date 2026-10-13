//! Port of source/interface/taskmanager/faith.h + source/interface/taskmanager/faith.cpp

use uplink_engine::sound;

use crate::app::opengl_interface::create_msgbox;
use crate::ctx::Ctx;
use crate::interface::taskmanager::uplinktask::{UplinkTask, UplinkTaskBase};
use crate::objref::ObjRef;
use crate::vanbakel::Vanbakel;

#[derive(Debug)]
pub struct Faith {
    pub base: UplinkTaskBase,
}

impl Faith {
    pub fn new() -> Self {
        Faith {
            base: UplinkTaskBase {
                version: 1.0,
                targetint: -1,
                ..Default::default()
            },
        }
    }
}

impl Default for Faith {
    fn default() -> Self {
        Self::new()
    }
}

impl UplinkTask for Faith {
    fn base(&self) -> &UplinkTaskBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut UplinkTaskBase {
        &mut self.base
    }
    fn initialise(&mut self, _g: &mut Ctx) {}
    // C++ UplinkTask::SetTarget / MoveTo are empty: the target is not stored.
    fn set_target(&mut self, _g: &mut Ctx, _target: ObjRef, _targetstring: &str, _targetint: i32) {}
    fn tick(&mut self, g: &mut Ctx, _n: i32) {
        // Is revelation running?
        let revelation_pid = g.svb.get_task_by_name("Revelation");
        // None when no task is called "Revelation" (get_task_dyn(-1) finds nothing).
        let revelation_version = g.svb.get_task_dyn(revelation_pid).map(|t| t.base().version);

        sound::play_sound("sounds/faith.wav", None, false);

        match revelation_version {
            Some(version) => {
                if version <= self.base.version {
                    Vanbakel::hide_task(g, revelation_pid); // revelation->RemoveInterface ()
                    Vanbakel::remove_task(g, revelation_pid);
                    Vanbakel::remove_task(g, self.base.pid);
                    create_msgbox(g, "Faith", "Revelation has been purged");
                }
            }
            None => {
                create_msgbox(g, "Faith", "Revelation is not\nrunning locally");
                Vanbakel::remove_task(g, self.base.pid);
            }
        }
    }
    fn create_interface(&mut self, _g: &mut Ctx) {}
    fn remove_interface(&mut self, _g: &mut Ctx) {}
    fn show_interface(&mut self, _g: &mut Ctx) {}
    fn is_interface_visible(&self, _g: &Ctx) -> bool {
        false
    }
    crate::impl_as_any!();
}
