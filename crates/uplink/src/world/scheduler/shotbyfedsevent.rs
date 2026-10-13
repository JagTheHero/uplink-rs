//! Port of source/world/scheduler/shotbyfedsevent.h + source/world/scheduler/shotbyfedsevent.cpp

use serde::{Deserialize, Serialize};

use crate::ctx::Ctx;
use crate::world::generator::consequencegenerator::ConsequenceGenerator;
use crate::world::person::PERSON_STATUS_DEAD;
use crate::world::scheduler::uplinkevent::UplinkEventBase;

/// Occurs when someone is shot by federal agents.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ShotByFedsEvent {
    pub base: UplinkEventBase,
    pub reason: String,
    pub name: String,
}

impl ShotByFedsEvent {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_name(&mut self, newname: &str) {
        self.name = newname.to_string();
    }

    pub fn set_reason(&mut self, newreason: &str) {
        self.reason = newreason.to_string();
    }

    pub fn run(&self, g: &mut Ctx) {
        let person = g
            .world_mut()
            .get_person_mut(&self.name)
            .expect("ShotByFedsEvent::run: person not found");
        person.set_status(PERSON_STATUS_DEAD);

        ConsequenceGenerator::shot_by_feds(&self.name, &self.reason);
    }

    pub fn get_short_string(&self) -> String {
        format!("ShotByFeds for {}", self.name)
    }

    pub fn get_long_string(&self) -> String {
        format!(
            "Shot By Feds Event\nFor : {}\nReason : {}",
            self.name, self.reason
        )
    }
}
