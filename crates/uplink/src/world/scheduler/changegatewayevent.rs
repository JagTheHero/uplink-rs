//! Port of source/world/scheduler/changegatewayevent.h + source/world/scheduler/changegatewayevent.cpp

use serde::{Deserialize, Serialize};

use crate::ctx::Ctx;
use crate::world::message::Message;
use crate::world::scheduler::uplinkevent::UplinkEventBase;

/// The player is told he must log out, then his gateway upgrades to the new type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ChangeGatewayEvent {
    pub base: UplinkEventBase,
    pub newtype: i32,
}

impl Default for ChangeGatewayEvent {
    fn default() -> Self {
        ChangeGatewayEvent {
            base: UplinkEventBase::default(),
            newtype: -1,
        }
    }
}

impl ChangeGatewayEvent {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_new_type(&mut self, value: i32) {
        self.newtype = value;
    }

    pub fn run(&self, g: &mut Ctx) {
        // Do the exchange
        if self.newtype != -1 {
            g.player_mut().gateway.exchange_gateway(self.newtype);
        }

        // Send a mail to the player
        let mut msg = Message::new();
        msg.set_to("PLAYER");
        msg.set_from("Uplink Corporation");
        msg.set_subject("New Gateway Ready");
        msg.set_body(concat!(
            "We have finished preparing your new gateway machine.  We have installed ",
            "identical hardware in this new gateway, and we have copied your memory ",
            "banks onto the new system.\n\n",
            "In order to complete the upgrade, you must now log out from Uplink.  When ",
            "you next log back in to the system, you will be automatically redirected ",
            "to your new gateway system.\n\n",
            "We will then immediately dismantle your old system and wipe the memory banks ",
            "of all data stored.\n\n",
            "Regards\n",
            "[END]",
        ));
        msg.send(g);
    }

    pub fn run_warning(&self, _g: &mut Ctx) {}

    pub fn get_short_string(&self) -> String {
        format!("Exchanging gateway for TYPE {}", self.newtype)
    }

    pub fn get_long_string(&self) -> String {
        self.get_short_string()
    }
}
