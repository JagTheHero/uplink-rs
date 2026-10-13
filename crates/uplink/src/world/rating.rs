//! Port of source/world/rating.h + source/world/rating.cpp

use serde::{Deserialize, Serialize};

use crate::app::globals;
use crate::ctx::Ctx;
use crate::game::data::data::{
    DEMO_MAXUPLINKRATING, NEUROMANCERRATING, NUM_NEUROMANCERRATINGS, NUM_UPLINKRATINGS,
    TIME_TOWAREZGAMEOVER, UPLINKRATING, WAREZ_MAXUPLINKRATING,
};
use crate::world::computer::gateway::HUDUPGRADE_CONNECTIONANALYSIS;
use crate::world::date::Date;
use crate::world::generator::demoplotgenerator::DemoPlotGenerator;
use crate::world::message::Message;
use crate::world::scheduler::notificationevent::{NotificationEvent, NOTIFICATIONEVENT_TYPE_WAREZGAMEOVER};
use crate::world::scheduler::uplinkevent::UplinkEvent;

/// Body of the mail the player gets when the Connection Analyser becomes relevant (rating.cpp).
const CONNECTION_ANALYSER_BODY: &str = concat!(
    "Firstly allow us to congratulate you on your recent successes.  We have been following your ",
    "career with interest.  As an up-and-coming Agent, you will find a number ",
    "of new opportunities are now available to you.  At this point we feel we should give ",
    "you an important piece of information.\n\n",
    "Now that the stakes are higher, you will certainly begin to encounter computer systems ",
    "that are more secure than before.  These systems can be difficult to attack.\n\n",
    "Most of our Agents now use the HUD-Connection-Analyser to get around these systems.  ",
    "This item of software was designed by Uplink Corporation specifically for bypassing security ",
    "systems, and can be purchased from Uplink Internal Services.  It is designed to be used ",
    "in conjunction with Security-Bypassers, also available from that site.\n\n",
    "Uplink Corporation wishes you luck.",
);

/// The world rating of a person or the player, i.e. how well qualified they are.
// The rating tables (`UPLINKRATING`, `NEUROMANCERRATING`, `NUM_*RATINGS`) live in
// `crate::game::data::data` (game/data/data.h).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Rating {
    /// Name of the Person this rating belongs to ("PLAYER" for the player).
    pub owner: String,

    pub uplinkrating: i32,
    pub uplinkscore: i32,

    pub neuromancerrating: i32,
    pub neuromancerscore: i32,

    pub creditrating: i32,
}

impl Rating {
    pub fn new() -> Self {
        Rating::default()
    }

    pub fn set_owner(&mut self, newowner: &str) {
        self.owner = newowner.to_string();
    }

    /// Sets rating and score from the `UPLINKRATING` table; raises the credit rating to match.
    pub fn set_uplink_rating(&mut self, rating: i32) {
        debug_assert!(rating >= 0 && rating < NUM_UPLINKRATINGS);
        self.uplinkrating = rating;
        self.uplinkscore = UPLINKRATING[rating as usize].score;

        if self.uplinkrating > self.creditrating {
            self.creditrating = self.uplinkrating;
        }
    }

    pub fn set_neuromancer_rating(&mut self, rating: i32) {
        debug_assert!(rating >= 0 && rating < NUM_NEUROMANCERRATINGS);
        self.neuromancerrating = rating;
        self.neuromancerscore = NEUROMANCERRATING[rating as usize].score;
    }

    /// Adjusts the score, finds the new rating and mails the owner if it changed.
    // PORT: sends Messages / schedules events through the world, so it takes the owner's key
    // (`Person::name`, == `self.owner`) instead of `self`. The rating is `Person::rating` of that
    // person: it is copied out, changed, and stored back before any message is sent and at the end.
    // A missing owner returns early (the C++ always had its `this`; every owner is in the world).
    pub fn change_uplink_score(g: &mut Ctx, owner: &str, amount: i32) {
        let Some(mut r) = g.world().get_person(owner).map(|p| p.rating.clone()) else {
            return;
        };

        if amount < 0 && r.uplinkrating <= 2 {
            return;
        }
        if amount == 0 {
            return;
        }

        r.uplinkscore += amount;
        if amount < 0 && r.uplinkscore < UPLINKRATING[2].score {
            r.uplinkscore = UPLINKRATING[2].score;
        }

        // Find the new uplinkrating (it might / might not have changed)
        let oldrating = r.uplinkrating;

        for i in 0..NUM_UPLINKRATINGS - 1 {
            if r.uplinkscore >= UPLINKRATING[i as usize].score
                && r.uplinkscore < UPLINKRATING[i as usize + 1].score
            {
                r.uplinkrating = i;
                break;
            }
        }

        // Special case - highest uplink rating
        if r.uplinkscore >= UPLINKRATING[(NUM_UPLINKRATINGS - 1) as usize].score {
            r.uplinkrating = NUM_UPLINKRATINGS - 1;
        }

        // Special case - less points than minimum rating
        if r.uplinkscore < UPLINKRATING[2].score {
            r.uplinkscore = UPLINKRATING[2].score;
            r.uplinkrating = 2;
        }

        // Special case - player needs to be told about Connection Analyser
        if owner == "PLAYER" && r.uplinkrating == 6 && oldrating < 6 {
            if !g.player().gateway.has_hud_upgrade(HUDUPGRADE_CONNECTIONANALYSIS) {
                Rating::store(g, owner, &r);

                let mut m = Message::new();
                m.set_to("PLAYER");
                m.set_from("Uplink Corporation");
                m.set_subject("Information for Skilled Agents");
                m.set_body(CONNECTION_ANALYSER_BODY);
                m.send(g);
            }
        }

        // Special case - upper limit reached in Demo game
        // (`DEMOGAME` is a build flag: compiled out here, but kept so nothing is lost.)
        if globals::DEMOGAME && owner == "PLAYER" {
            if r.uplinkrating != oldrating {
                Rating::store(g, owner, &r);
                DemoPlotGenerator::player_rating_change(g);
            }

            if r.uplinkscore >= UPLINKRATING[DEMO_MAXUPLINKRATING as usize].score {
                r.uplinkrating = DEMO_MAXUPLINKRATING;
                r.uplinkscore = UPLINKRATING[DEMO_MAXUPLINKRATING as usize].score;
                Rating::store(g, owner, &r);
                return;
            }
        }

        // Special case - illegal copy
        // (`WAREZRELEASE` is a build flag: compiled out here, but kept so nothing is lost.)
        if globals::WAREZRELEASE && owner == "PLAYER" {
            if r.uplinkscore >= UPLINKRATING[WAREZ_MAXUPLINKRATING as usize].score {
                r.uplinkrating = WAREZ_MAXUPLINKRATING;
                r.uplinkscore = UPLINKRATING[WAREZ_MAXUPLINKRATING as usize].score;

                let mut rundate = Date::default();
                rundate.set_date_from(&g.world().date);
                rundate.advance_minute(TIME_TOWAREZGAMEOVER);

                let mut ne = NotificationEvent::new();
                ne.set_type(NOTIFICATIONEVENT_TYPE_WAREZGAMEOVER);
                let mut event = UplinkEvent::NotificationEvent(ne);
                event.set_run_date(&rundate);

                Rating::store(g, owner, &r);
                g.world_mut().scheduler.schedule_event(event);
                return;
            }
        }

        // Has our rating changed ?

        if r.uplinkrating > oldrating {
            // Score has gone up
            // Send a congrats message to the owner

            if g.world().get_person(owner).is_some() {
                let agenthandle = Rating::agent_handle(g, owner);
                let body = format!(
                    "Congratulations Agent {} - you have reached the next Uplink Rating.\n\n\
                     Your rating is now {}.",
                    agenthandle,
                    Rating::get_uplink_rating_string(r.uplinkrating)
                );

                Rating::store(g, owner, &r);
                Rating::mail_owner(g, owner, "Congratulations", &body);

                // Increase our credit rating
                if r.uplinkrating > r.creditrating {
                    r.creditrating = r.uplinkrating;
                }
            }
        } else if r.uplinkrating < oldrating {
            // Score has gone down
            // Send a warning message to the owner

            if g.world().get_person(owner).is_some() {
                let agenthandle = Rating::agent_handle(g, owner);
                // PORT: the C++ literal has no space after the full stop before "We hope" (adjacent
                // string literals); kept as-is.
                let body = format!(
                    "This is a warning Agent {} - your performance recently has been unsatisfactory.\n\
                     As such we have been forced to review your case, and have decided to decrease your Uplink rating.\n\n\
                     Your rating is now {}.We hope this is sufficient punishment to make our point clear.\n\
                     Uplink Corporation will not tolerate failure.",
                    agenthandle,
                    Rating::get_uplink_rating_string(r.uplinkrating)
                );

                Rating::store(g, owner, &r);
                Rating::mail_owner(g, owner, "Uplink WARNING", &body);
            }
        } else {
            // Score has not changed
        }

        Rating::store(g, owner, &r);
    }

    // PORT: as `change_uplink_score`.
    pub fn change_neuromancer_score(g: &mut Ctx, owner: &str, amount: i32) {
        let Some(mut r) = g.world().get_person(owner).map(|p| p.rating.clone()) else {
            return;
        };

        r.neuromancerscore += amount;

        // Find the new neuromancer rating (it might / might not have changed)
        let oldrating = r.neuromancerrating;

        for i in 0..NUM_NEUROMANCERRATINGS - 1 {
            if r.neuromancerscore >= NEUROMANCERRATING[i as usize].score
                && r.neuromancerscore < NEUROMANCERRATING[i as usize + 1].score
            {
                r.neuromancerrating = i;
                break;
            }
        }

        // Special case - highest rating
        if r.neuromancerscore >= NEUROMANCERRATING[(NUM_NEUROMANCERRATINGS - 1) as usize].score {
            r.neuromancerrating = NUM_NEUROMANCERRATINGS - 1;
        }

        // Special case - less points than minimum rating (ie negative)
        if r.neuromancerscore < NEUROMANCERRATING[0].score {
            r.neuromancerrating = 0;
        }

        // Has our rating changed ?

        if r.neuromancerrating > oldrating {
            // Score has gone up
            // Send a congrats message to the owner

            if g.world().get_person(owner).is_some() {
                let body = format!(
                    "Agent {} - your Neuromancer rating appears to have changed.\n\n\
                     Your rating is now {}.",
                    Rating::agent_handle(g, owner),
                    Rating::get_neuromancer_rating_string(r.neuromancerrating)
                );

                Rating::store(g, owner, &r);
                Rating::mail_owner(g, owner, "Neuromancer rating change", &body);
            }
        } else if r.neuromancerrating < oldrating {
            // Score has gone down

            if g.world().get_person(owner).is_some() {
                let body = format!(
                    "It appears your neuromancer rating has changed, in view of your recent actions.\n\n\
                     Your new rating is {}.",
                    Rating::get_neuromancer_rating_string(r.neuromancerrating)
                );

                Rating::store(g, owner, &r);
                Rating::mail_owner(g, owner, "Neuromancer rating change", &body);
            }
        } else {
            // Score has not changed
        }

        Rating::store(g, owner, &r);
    }

    /// Name of the current Uplink rating (`UPLINKRATING[uplinkrating].name`).
    pub fn get_uplink_rating(&self) -> String {
        Rating::get_uplink_rating_string(self.uplinkrating)
    }

    /// Name of the current Neuromancer rating (`NEUROMANCERRATING[neuromancerrating].name`).
    pub fn get_neuromancer_rating(&self) -> String {
        Rating::get_neuromancer_rating_string(self.neuromancerrating)
    }

    pub fn set_credit_rating(&mut self, rating: i32) {
        self.creditrating = rating;
    }

    pub fn change_credit_rating(&mut self, amount: i32) {
        self.creditrating += amount;
    }

    pub fn get_uplink_rating_string(urating: i32) -> String {
        debug_assert!(urating >= 0 && urating < NUM_UPLINKRATINGS);
        UPLINKRATING[urating as usize].name.to_string()
    }

    pub fn get_neuromancer_rating_string(nrating: i32) -> String {
        debug_assert!(nrating >= 0 && nrating < NUM_NEUROMANCERRATINGS);
        NEUROMANCERRATING[nrating as usize].name.to_string()
    }

    // ---- private helpers for the score functions above ----

    /// Writes a working copy of the rating back into the owner's Person.
    fn store(g: &mut Ctx, owner: &str, r: &Rating) {
        if let Some(p) = g.world_mut().get_person_mut(owner) {
            p.rating = r.clone();
        }
    }

    /// `strcmp(owner, "PLAYER") != 0 ? owner : GetPlayer()->handle`
    fn agent_handle(g: &Ctx, owner: &str) -> String {
        if owner != "PLAYER" {
            owner.to_string()
        } else {
            g.player().agent().handle.clone()
        }
    }

    /// Mail from "internal@Uplink.net" to the owner.
    fn mail_owner(g: &mut Ctx, owner: &str, subject: &str, body: &str) {
        let mut msg = Message::new();
        msg.set_from("internal@Uplink.net");
        msg.set_to(owner);
        msg.set_subject(subject);
        msg.set_body(body);
        msg.send(g);
    }
}
