//! Port of source/world/scheduler/uplinkevent.h + source/world/scheduler/uplinkevent.cpp

use serde::{Deserialize, Serialize};

use crate::ctx::Ctx;
use crate::world::date::Date;
use crate::world::world::World;

use crate::world::scheduler::arrestevent::ArrestEvent;
use crate::world::scheduler::attemptmissionevent::AttemptMissionEvent;
use crate::world::scheduler::bankrobberyevent::BankRobberyEvent;
use crate::world::scheduler::changegatewayevent::ChangeGatewayEvent;
use crate::world::scheduler::installhardwareevent::InstallHardwareEvent;
use crate::world::scheduler::notificationevent::NotificationEvent;
use crate::world::scheduler::runplotsceneevent::RunPlotSceneEvent;
use crate::world::scheduler::seizegatewayevent::SeizeGatewayEvent;
use crate::world::scheduler::shotbyfedsevent::ShotByFedsEvent;
use crate::world::scheduler::warningevent::WarningEvent;

/// Fields of the C++ base class `UplinkEvent`; embedded as `base` in every event struct.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct UplinkEventBase {
    pub rundate: Date,
}

impl UplinkEventBase {
    pub fn new() -> Self {
        Self::default()
    }

    /// `SetRunDate`
    pub fn set_run_date(&mut self, newrundate: &Date) {
        self.rundate = newrundate.clone();
    }
}

/// All concrete events (PORTING.md §3.2). Variant name = C++ class name.
/// The C++ virtuals (`Run`, `RunWarning`, `GetShortString`, `GetLongString`) dispatch by `match`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UplinkEvent {
    ArrestEvent(ArrestEvent),
    AttemptMissionEvent(AttemptMissionEvent),
    BankRobberyEvent(BankRobberyEvent),
    ChangeGatewayEvent(ChangeGatewayEvent),
    InstallHardwareEvent(InstallHardwareEvent),
    NotificationEvent(NotificationEvent),
    RunPlotSceneEvent(RunPlotSceneEvent),
    SeizeGatewayEvent(SeizeGatewayEvent),
    ShotByFedsEvent(ShotByFedsEvent),
    WarningEvent(WarningEvent),
}

impl UplinkEvent {
    pub fn base(&self) -> &UplinkEventBase {
        match self {
            UplinkEvent::ArrestEvent(e) => &e.base,
            UplinkEvent::AttemptMissionEvent(e) => &e.base,
            UplinkEvent::BankRobberyEvent(e) => &e.base,
            UplinkEvent::ChangeGatewayEvent(e) => &e.base,
            UplinkEvent::InstallHardwareEvent(e) => &e.base,
            UplinkEvent::NotificationEvent(e) => &e.base,
            UplinkEvent::RunPlotSceneEvent(e) => &e.base,
            UplinkEvent::SeizeGatewayEvent(e) => &e.base,
            UplinkEvent::ShotByFedsEvent(e) => &e.base,
            UplinkEvent::WarningEvent(e) => &e.base,
        }
    }

    pub fn base_mut(&mut self) -> &mut UplinkEventBase {
        match self {
            UplinkEvent::ArrestEvent(e) => &mut e.base,
            UplinkEvent::AttemptMissionEvent(e) => &mut e.base,
            UplinkEvent::BankRobberyEvent(e) => &mut e.base,
            UplinkEvent::ChangeGatewayEvent(e) => &mut e.base,
            UplinkEvent::InstallHardwareEvent(e) => &mut e.base,
            UplinkEvent::NotificationEvent(e) => &mut e.base,
            UplinkEvent::RunPlotSceneEvent(e) => &mut e.base,
            UplinkEvent::SeizeGatewayEvent(e) => &mut e.base,
            UplinkEvent::ShotByFedsEvent(e) => &mut e.base,
            UplinkEvent::WarningEvent(e) => &mut e.base,
        }
    }

    /// `event->rundate`
    pub fn rundate(&self) -> &Date {
        &self.base().rundate
    }

    /// `SetRunDate`
    pub fn set_run_date(&mut self, newrundate: &Date) {
        self.base_mut().set_run_date(newrundate);
    }

    /// `Run` — consumes the event (the scheduler removes it from its queue before running it).
    pub fn run(self, g: &mut Ctx) {
        match self {
            UplinkEvent::ArrestEvent(e) => e.run(g),
            UplinkEvent::AttemptMissionEvent(e) => e.run(g),
            UplinkEvent::BankRobberyEvent(e) => e.run(g),
            UplinkEvent::ChangeGatewayEvent(e) => e.run(g),
            UplinkEvent::InstallHardwareEvent(e) => e.run(g),
            UplinkEvent::NotificationEvent(e) => e.run(g),
            UplinkEvent::RunPlotSceneEvent(e) => e.run(g),
            UplinkEvent::SeizeGatewayEvent(e) => e.run(g),
            UplinkEvent::ShotByFedsEvent(e) => e.run(g),
            UplinkEvent::WarningEvent(e) => e.run(g),
        }
    }

    /// `RunWarning` (base implementation does nothing).
    pub fn run_warning(&self, g: &mut Ctx) {
        match self {
            UplinkEvent::ChangeGatewayEvent(e) => e.run_warning(g),
            UplinkEvent::InstallHardwareEvent(e) => e.run_warning(g),
            UplinkEvent::SeizeGatewayEvent(e) => e.run_warning(g),
            _ => {}
        }
    }

    /// `GetShortString`
    pub fn get_short_string(&self) -> String {
        match self {
            UplinkEvent::ArrestEvent(e) => e.get_short_string(),
            UplinkEvent::AttemptMissionEvent(e) => e.get_short_string(),
            UplinkEvent::BankRobberyEvent(e) => e.get_short_string(),
            UplinkEvent::ChangeGatewayEvent(e) => e.get_short_string(),
            UplinkEvent::InstallHardwareEvent(e) => e.get_short_string(),
            UplinkEvent::NotificationEvent(e) => e.get_short_string(),
            UplinkEvent::RunPlotSceneEvent(e) => e.get_short_string(),
            UplinkEvent::SeizeGatewayEvent(e) => e.get_short_string(),
            UplinkEvent::ShotByFedsEvent(e) => e.get_short_string(),
            UplinkEvent::WarningEvent(e) => e.get_short_string(),
        }
    }

    /// `GetLongString`. PORT: takes `&World` because AttemptMissionEvent and WarningEvent
    /// need a world lookup to describe themselves.
    pub fn get_long_string(&self, world: &World) -> String {
        match self {
            UplinkEvent::ArrestEvent(e) => e.get_long_string(),
            UplinkEvent::AttemptMissionEvent(e) => e.get_long_string(world),
            UplinkEvent::BankRobberyEvent(e) => e.get_long_string(),
            UplinkEvent::ChangeGatewayEvent(e) => e.get_long_string(),
            UplinkEvent::InstallHardwareEvent(e) => e.get_long_string(),
            UplinkEvent::NotificationEvent(e) => e.get_long_string(),
            UplinkEvent::RunPlotSceneEvent(e) => e.get_long_string(),
            UplinkEvent::SeizeGatewayEvent(e) => e.get_long_string(),
            UplinkEvent::ShotByFedsEvent(e) => e.get_long_string(),
            UplinkEvent::WarningEvent(e) => e.get_long_string(world),
        }
    }
}
