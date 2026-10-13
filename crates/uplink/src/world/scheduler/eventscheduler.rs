//! Port of source/world/scheduler/eventscheduler.h + source/world/scheduler/eventscheduler.cpp

use serde::{Deserialize, Serialize};

use crate::ctx::Ctx;
use crate::world::date::Date;
use crate::world::scheduler::uplinkevent::UplinkEvent;
use crate::world::scheduler::warningevent::WarningEvent;

/// Keeps track of all future scheduled events (sorted by run date, earliest first)
/// and runs them at the right time. Events run within ~10 seconds of their target time.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct EventScheduler {
    pub events: Vec<UplinkEvent>,
}

impl EventScheduler {
    pub fn new() -> Self {
        Self::default()
    }

    /// `ScheduleEvent`: insert before the first event that runs later, else at the back.
    pub fn schedule_event(&mut self, event: UplinkEvent) {
        // Insert before the first event that is later than this one
        let position = self
            .events
            .iter()
            .position(|ue| event.rundate().before(ue.rundate()));

        match position {
            Some(i) => self.events.insert(i, event),
            // Put it at the back of the queue
            None => self.events.push(event),
        }
    }

    /// `ScheduleWarning`: schedules a `WarningEvent` at `date` that will call `event.run_warning()`.
    /// PORT: the warning holds its own clone of `event` (the caller schedules the real one separately).
    pub fn schedule_warning(&mut self, event: &UplinkEvent, date: &Date) {
        let mut wevent = WarningEvent::new();
        wevent.set_event(event.clone());
        wevent.base.set_run_date(date);

        self.schedule_event(UplinkEvent::WarningEvent(wevent));
    }

    /// `GetDateOfNextEvent`
    pub fn get_date_of_next_event(&self) -> Option<&Date> {
        self.events.first().map(|event| event.rundate())
    }

    /// `Update`: run every event that should have been run so far. Uses `game`, so it takes the context.
    // PORT: the C++ ran the event while it was still at index 0 and removed it afterwards. Here the
    // event is removed first and then run (Run consumes it). Same result unless a running event
    // schedules something dated before its own run date, where the C++ removed the wrong entry.
    pub fn update(g: &mut Ctx) {
        loop {
            let due = match g.world().scheduler.events.first() {
                Some(event) => g.world().date.after(event.rundate()),
                None => false,
            };
            if !due {
                break;
            }

            let event = g.world_mut().scheduler.events.remove(0);
            event.run(g);
        }
    }
}
