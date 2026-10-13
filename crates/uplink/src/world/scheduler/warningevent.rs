//! Port of source/world/scheduler/warningevent.h + source/world/scheduler/warningevent.cpp

use serde::{Deserialize, Serialize};

use crate::ctx::Ctx;
use crate::world::scheduler::uplinkevent::{UplinkEvent, UplinkEventBase};
use crate::world::world::World;

/// Triggered as a warning that another event is about to happen
/// (e.g. "your gateway will be seized in 10 minutes").
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct WarningEvent {
    pub base: UplinkEventBase,
    /// PORT: owned copy of the event being warned about (the C++ shared the pointer with the
    /// scheduled event, but saved/loaded it by value).
    pub event: Option<Box<UplinkEvent>>,
}

impl WarningEvent {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_event(&mut self, newevent: UplinkEvent) {
        self.event = Some(Box::new(newevent));
    }

    pub fn run(&self, g: &mut Ctx) {
        if let Some(event) = &self.event {
            event.run_warning(g);
        }
    }

    pub fn get_short_string(&self) -> String {
        "Warning Event".to_string()
    }

    pub fn get_long_string(&self, world: &World) -> String {
        let eventstring = self
            .event
            .as_ref()
            .map(|e| e.get_long_string(world))
            .unwrap_or_default();
        format!("Warning Event for :\n{}", eventstring)
    }
}
