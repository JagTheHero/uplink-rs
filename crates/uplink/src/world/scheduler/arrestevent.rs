//! Port of source/world/scheduler/arrestevent.h + source/world/scheduler/arrestevent.cpp

use serde::{Deserialize, Serialize};

use crate::ctx::Ctx;
use crate::world::generator::consequencegenerator::ConsequenceGenerator;
use crate::world::person::PERSON_STATUS_INJAIL;
use crate::world::scheduler::uplinkevent::UplinkEventBase;

/// Occurs when a person is arrested by federal agents (usually for hacking).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ArrestEvent {
    pub base: UplinkEventBase,
    pub reason: String,
    /// IP of the computer they messed with (None = no computer involved).
    pub ip: Option<String>,
    pub name: String,
}

impl ArrestEvent {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_name(&mut self, newname: &str) {
        self.name = newname.to_string();
    }

    pub fn set_reason(&mut self, newreason: &str) {
        self.reason = newreason.to_string();
    }

    pub fn set_ip(&mut self, newip: &str) {
        self.ip = Some(newip.to_string());
    }

    pub fn run(&self, g: &mut Ctx) {
        {
            let person = g
                .world_mut()
                .get_person_mut(&self.name)
                .expect("ArrestEvent::run: person not found");
            person.set_status(PERSON_STATUS_INJAIL);
        }

        // Look up the computer they messed with
        let comp_name: Option<String> = match &self.ip {
            Some(ip) => {
                let world = g.world();
                let vl = world
                    .get_vlocation(ip)
                    .expect("ArrestEvent::run: VLocation not found");
                let comp = vl
                    .get_computer(world)
                    .expect("ArrestEvent::run: VLocation has no computer");
                Some(comp.name.clone())
            }
            None => None,
        };

        ConsequenceGenerator::arrested(g, &self.name, comp_name.as_deref(), &self.reason);
    }

    pub fn get_short_string(&self) -> String {
        format!("Arrest for {}", self.name)
    }

    pub fn get_long_string(&self) -> String {
        format!(
            "ArrestEvent:\nFor : {}\nReason : {}",
            self.name, self.reason
        )
    }
}
