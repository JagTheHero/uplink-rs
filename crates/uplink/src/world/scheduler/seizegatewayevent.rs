//! Port of source/world/scheduler/seizegatewayevent.h + source/world/scheduler/seizegatewayevent.cpp

use serde::{Deserialize, Serialize};

use crate::ctx::Ctx;
use crate::game::data::data::{NEUROMANCERCHANGE_GATEWAYNEARMISS, UPLINKCHANGE_GATEWAYNEARMISS};
use crate::world::generator::consequencegenerator::ConsequenceGenerator;
use crate::world::message::Message;
use crate::world::rating::Rating;
use crate::world::scheduler::uplinkevent::UplinkEventBase;

/// Occurs when federal agents decide to grab your gateway (only applies to Uplink Agents).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SeizeGatewayEvent {
    pub base: UplinkEventBase,
    pub reason: String,
    pub name: String,
    /// id number of gateway to seize
    pub gateway_id: i32,
}

impl SeizeGatewayEvent {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_name(&mut self, newname: &str) {
        self.name = newname.to_string();
    }

    pub fn set_reason(&mut self, newreason: &str) {
        self.reason = newreason.to_string();
    }

    pub fn set_gateway_id(&mut self, newid: i32) {
        self.gateway_id = newid;
    }

    pub fn run(&self, g: &mut Ctx) {
        assert!(
            g.world().get_person(&self.name).is_some(),
            "UplinkAssert(person)"
        );

        ConsequenceGenerator::seize_gateway(&self.name, &self.reason);

        //
        // If this is the player, then it's GAME OVER
        //

        if self.name == "PLAYER" {
            if self.gateway_id == g.player().gateway.id {
                let deathmsg = format!(
                    "Disavowed by Uplink Corporation at {}\nFor {}\n",
                    g.world().date.get_long_string(),
                    self.reason
                );

                g.game.game_over(&deathmsg);
            } else {
                // The player's old gateway was seized
                // Not quite as severe - not game over

                let mut msg = Message::new();
                msg.set_to("PLAYER");
                msg.set_from("Uplink Corporation");
                msg.set_subject("Gateway seizure");
                msg.set_body(
                    "Your old gateway was recently seized by federal agents, shortly after you blew it up.\n\
                     It seems that no evidence remains of the crime that you didn't commit.\n\n\
                     A very lucky escape, if you ask us.\n",
                );
                msg.send(g);

                // Uplink Corporation won't like you for this (it's reckless after all)
                // Other hackers love this - everybody knows you did it but
                // you got away with it scott free!

                Rating::change_neuromancer_score(g, "PLAYER", NEUROMANCERCHANGE_GATEWAYNEARMISS);
                Rating::change_uplink_score(g, "PLAYER", UPLINKCHANGE_GATEWAYNEARMISS);
            }
        }
    }

    pub fn run_warning(&self, g: &mut Ctx) {
        //
        // Agents are near (2 of em!)
        //

        g.player_mut().gateway.increment_proximity(2);
    }

    pub fn get_short_string(&self) -> String {
        format!("Seize Gateway of {}", self.name)
    }

    pub fn get_long_string(&self) -> String {
        format!(
            "Seize Gateway Event\nFor : {}\nGatewayID : {}\nReason : {}",
            self.name, self.gateway_id, self.reason
        )
    }
}
