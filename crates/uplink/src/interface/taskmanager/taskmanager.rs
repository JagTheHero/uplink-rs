//! Port of source/interface/taskmanager/taskmanager.h + source/interface/taskmanager/taskmanager.cpp
#![allow(unused_variables, unused_imports, unused_mut, dead_code)]

use crate::app::opengl::{get_mouse_x, get_mouse_y};
use crate::ctx::Ctx;
use crate::interface::taskmanager::decrypter::Decrypter;
use crate::interface::taskmanager::decypher::Decypher;
use crate::interface::taskmanager::defrag::Defrag;
use crate::interface::taskmanager::dictionaryhacker::DictionaryHacker;
use crate::interface::taskmanager::faith::Faith;
use crate::interface::taskmanager::filecopier::FileCopier;
use crate::interface::taskmanager::filedeleter::FileDeleter;
use crate::interface::taskmanager::firewalldisable::FirewallDisable;
use crate::interface::taskmanager::gatewaynuke::GatewayNuke;
use crate::interface::taskmanager::iplookup::IPLookup;
use crate::interface::taskmanager::ipprobe::IPProbe;
use crate::interface::taskmanager::lanforce::LanForce;
use crate::interface::taskmanager::lanprobe::LanProbe;
use crate::interface::taskmanager::lanscan::LanScan;
use crate::interface::taskmanager::lanspoof::LanSpoof;
use crate::interface::taskmanager::logdeleter::LogDeleter;
use crate::interface::taskmanager::logmodifier::LogModifier;
use crate::interface::taskmanager::logundeleter::LogUnDeleter;
use crate::interface::taskmanager::motionsensor::MotionSensor;
use crate::interface::taskmanager::passwordbreaker::PasswordBreaker;
use crate::interface::taskmanager::proxydisable::ProxyDisable;
use crate::interface::taskmanager::revelation::Revelation;
use crate::interface::taskmanager::revelationtracker::RevelationTracker;
use crate::interface::taskmanager::securitybypass::SecurityBypass;
use crate::interface::taskmanager::tracetracker::TraceTracker;
use crate::interface::taskmanager::tutorial::Tutorial;
use crate::interface::taskmanager::uplinkagentlist::UplinkAgentList;
use crate::interface::taskmanager::uplinktask::UplinkTask;
use crate::interface::taskmanager::voiceanalyser::VoiceAnalyser;
use crate::objref::ObjRef;
use crate::vanbakel::Vanbakel;
use crate::world::computer::securitysystem::{
    SECURITY_TYPE_ENCRYPTION, SECURITY_TYPE_FIREWALL, SECURITY_TYPE_MONITOR, SECURITY_TYPE_PROXY,
};

/// Your local computer's task list; mostly a mapping into the vanbakel library.
// PORT: Load/Save wrote nothing, so nothing here needs to be persisted.
pub struct TaskManager {
    /// PID of the running program targeted (-1 = none)
    pub targetprogram: i32,
    /// Object to be targeted by the program
    pub programtarget: ObjRef,
    /// Used to regulate cycle speed
    pub lastcycle: i32,
}

/// Boxes a task that the C++ `RunSoftware` switch marks with `SetFollowMouse(true)`.
fn follow_mouse<T: UplinkTask>(mut task: T) -> Box<dyn UplinkTask> {
    task.set_follow_mouse(true);
    Box::new(task)
}

impl TaskManager {
    pub fn new() -> Self {
        TaskManager {
            targetprogram: -1,
            programtarget: ObjRef::None,
            lastcycle: 0,
        }
    }

    /// The C++ constructor's `SvbReset ()`; call once after the `Interface` is stored in `g.game.ui`.
    pub fn reset(g: &mut Ctx) {
        Vanbakel::reset(g);
    }

    /// Creates the named task (see the name list in the C++), sets its version, registers it
    /// with vanbakel and makes it the target program.
    pub fn run_software(g: &mut Ctx, name: &str, version: f32) {
        let mut task: Box<dyn UplinkTask> = match name {
            "Password_Breaker" => Box::new(PasswordBreaker::new()),
            "Dictionary_Hacker" => Box::new(DictionaryHacker::new()),
            "Trace_Tracker" => Box::new(TraceTracker::new()),
            "File_Copier" => follow_mouse(FileCopier::new()),
            "File_Deleter" => follow_mouse(FileDeleter::new()),
            "Defrag" => Box::new(Defrag::new()),
            "Log_Deleter" => follow_mouse(LogDeleter::new()),
            "Log_UnDeleter" => follow_mouse(LogUnDeleter::new()),
            "Log_Modifier" => follow_mouse(LogModifier::new()),
            "Decrypter" => follow_mouse(Decrypter::new()),
            "IP_Lookup" => Box::new(IPLookup::new()),
            "Gateway_Nuke" => Box::new(GatewayNuke::new()),
            "Motion_Sensor" => Box::new(MotionSensor::new()),
            "Proxy_Disable" => Box::new(ProxyDisable::new()),
            "Firewall_Disable" => Box::new(FirewallDisable::new()),
            "IP_Probe" => Box::new(IPProbe::new()),
            "Proxy_Bypass" => follow_mouse(SecurityBypass::with_type(SECURITY_TYPE_PROXY)),
            "Firewall_Bypass" => follow_mouse(SecurityBypass::with_type(SECURITY_TYPE_FIREWALL)),
            "Encryption_Bypass" => {
                follow_mouse(SecurityBypass::with_type(SECURITY_TYPE_ENCRYPTION))
            }
            "Monitor_Bypass" => follow_mouse(SecurityBypass::with_type(SECURITY_TYPE_MONITOR)),
            "Revelation" => Box::new(Revelation::default()),
            "Tutorial" => Box::new(Tutorial::new()),
            "Decypher" => follow_mouse(Decypher::new()),
            "Voice_Analyser" => Box::new(VoiceAnalyser::new()),
            "Revelation_Tracker" => follow_mouse(RevelationTracker::new()),
            "Uplink_Agent_List" => Box::new(UplinkAgentList::new()),
            "Faith" => Box::new(Faith::new()),
            "LAN_Scan" => Box::new(LanScan::new()),
            "LAN_Probe" => follow_mouse(LanProbe::new()),
            "LAN_Spoof" => follow_mouse(LanSpoof::new()),
            "LAN_Force" => follow_mouse(LanForce::new()),
            _ => {
                println!("Task Manager warning : Called RunSoftware, name not recognised ");
                return;
            }
        };

        // Run the new task
        task.set_version(version);
        let pid = Vanbakel::register_task(g, name, task);
        g.interface_mut().tm.set_target_program(pid);
    }

    /// Links the targeted program (if any) to `newprogramtarget`.
    pub fn set_program_target(
        g: &mut Ctx,
        newprogramtarget: ObjRef,
        targetstring: &str,
        targetint: i32,
    ) {
        let targetprogram = g.interface().tm.targetprogram;
        let has_target = !newprogramtarget.is_none();
        g.interface_mut().tm.programtarget = newprogramtarget.clone();

        if has_target && targetprogram != -1 {
            // Link the program to the target
            Vanbakel::with_task(g, targetprogram, |t, g| {
                t.set_target(g, newprogramtarget, targetstring, targetint)
            });

            let tm = &mut g.interface_mut().tm;
            tm.targetprogram = -1;
            tm.programtarget = ObjRef::None;
        }
    }

    pub fn set_target_program(&mut self, newtargetprogram: i32) {
        self.targetprogram = newtargetprogram;
    }

    /// Can return None
    pub fn get_target_program(g: &Ctx) -> Option<&dyn UplinkTask> {
        let pid = g.interface().tm.targetprogram;
        if pid != -1 {
            g.svb.get_task_dyn(pid)
        } else {
            None
        }
    }

    /// Can return None
    pub fn get_target_program_name(g: &Ctx) -> Option<String> {
        let pid = g.interface().tm.targetprogram;
        if pid != -1 {
            g.svb.get_task_wrapper(pid).map(|tw| tw.name.clone())
        } else {
            None
        }
    }

    /// Cycles the task list ~10 times a second and moves the targeted follow-mouse task to the mouse.
    pub fn update(g: &mut Ctx) {
        let numcyclespersecond = 10;

        // Cycle a few times a second
        let timesincelastcycle = g.ecl.get_accurate_time() - g.interface().tm.lastcycle;
        if timesincelastcycle >= 1000 / numcyclespersecond {
            let cpuspeed = g.player().gateway.get_cpu_speed() / numcyclespersecond;
            Vanbakel::cycle(g, cpuspeed);
            let now = g.ecl.get_accurate_time();
            g.interface_mut().tm.lastcycle = now;
        }

        // Move the current software app to the mouse position
        let pid = g.interface().tm.targetprogram;
        let followmouse = Self::get_target_program(g).is_some_and(|t| t.base().followmouse);
        if followmouse {
            Vanbakel::with_task(g, pid, |t, g| {
                t.move_to(g, get_mouse_x() + 3, get_mouse_y(), 0)
            });
        }
    }
}

impl Default for TaskManager {
    fn default() -> Self {
        TaskManager::new()
    }
}
