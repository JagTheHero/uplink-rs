//! Port of source/world/scheduler/attemptmissionevent.h + source/world/scheduler/attemptmissionevent.cpp

use serde::{Deserialize, Serialize};

use crate::ctx::Ctx;
use crate::world::person::Person;
use crate::world::scheduler::uplinkevent::UplinkEventBase;
use crate::world::world::World;

/// Represents an NPC agent attempting one of his missions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AttemptMissionEvent {
    pub base: UplinkEventBase,
    pub agentname: String,
}

impl AttemptMissionEvent {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_agent_name(&mut self, newagentname: &str) {
        self.agentname = newagentname.to_string();
    }

    pub fn run(&self, g: &mut Ctx) {
        // C++ casts the Person to Agent; the lookup itself must succeed.
        g.world()
            .get_person(&self.agentname)
            .expect("AttemptMissionEvent::run: agent not found");

        Person::attempt_mission(g, &self.agentname);
    }

    pub fn get_short_string(&self) -> String {
        format!("Attempt Mission for {}", self.agentname)
    }

    /// Needs the agent's first mission, hence the world lookup.
    pub fn get_long_string(&self, world: &World) -> String {
        let agent = world
            .get_person(&self.agentname)
            .expect("AttemptMissionEvent::get_long_string: agent not found");
        let m = agent
            .agent()
            .missions
            .first()
            .expect("AttemptMissionEvent::get_long_string: agent has no mission");

        format!(
            "Attempt Mission Event:\nAgent : {}\nAgents next mission : {}",
            self.agentname, m.description
        )
    }
}
