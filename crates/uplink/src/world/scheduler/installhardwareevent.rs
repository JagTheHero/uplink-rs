//! Port of source/world/scheduler/installhardwareevent.h + source/world/scheduler/installhardwareevent.cpp

use serde::{Deserialize, Serialize};

use crate::ctx::Ctx;
use crate::world::company::sale::Sale;
use crate::world::computer::databank::{Data, DATATYPE_PROGRAM, SOFTWARETYPE_HWDRIVER};
use crate::world::computer::gateway::{
    Gateway, GATEWAYUPGRADETYPE_CPU, GATEWAYUPGRADETYPE_MEMORY, GATEWAYUPGRADETYPE_MODEM,
    GATEWAYUPGRADETYPE_SECURITY,
};
use crate::world::message::Message;
use crate::world::person::Person;
use crate::world::scheduler::uplinkevent::UplinkEventBase;

/// Occurs when the player has ordered a hardware upgrade for his gateway and it arrives.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct InstallHardwareEvent {
    pub base: UplinkEventBase,
    /// PORT: the C++ held a `Sale *` into the company's sale list but saved/loaded it by value;
    /// here it is always an owned copy.
    pub hwsale: Option<Sale>,
    pub version: i32,
}

impl Default for InstallHardwareEvent {
    fn default() -> Self {
        InstallHardwareEvent {
            base: UplinkEventBase::default(),
            hwsale: None,
            version: -1,
        }
    }
}

impl InstallHardwareEvent {
    pub fn new() -> Self {
        Self::default()
    }

    /// Stores a copy of `newsale`; asserts that `newsale.get_version(newversion)` exists.
    pub fn set_hw_sale(&mut self, newsale: &Sale, newversion: i32) {
        self.hwsale = Some(newsale.clone());
        self.version = newversion;

        assert!(
            newsale.get_version(newversion).is_some(),
            "UplinkAssert(hwsale->GetVersion(version))"
        );
    }

    pub fn run(&self, g: &mut Ctx) {
        let hwsale = self.hwsale.as_ref().expect("UplinkAssert(hwsale)");

        let sv = hwsale
            .get_version(self.version)
            .expect("UplinkAssert(sv)");

        let mut success = true;
        let mut failure = String::new();

        //
        // MODEM - Change the player's gateway's transfer rate
        //

        if hwsale.swhwtype == GATEWAYUPGRADETYPE_MODEM {
            g.player_mut()
                .gateway
                .set_modem_type(&hwsale.title, sv.data);
        }

        //
        // CPU   - Change the player's gateway's processor speed
        //

        if hwsale.swhwtype == GATEWAYUPGRADETYPE_CPU {
            Gateway::give_cpu(g, &hwsale.title);
        }

        //
        // MEMORY  - Give the player's gateway more memory space
        //

        if hwsale.swhwtype == GATEWAYUPGRADETYPE_MEMORY {
            let (gatewaytype, gatewayname, memorysize) = {
                let gateway = &g.player().gateway;
                (gateway.type_, gateway.name.clone(), gateway.memorysize)
            };
            let gd = g
                .world()
                .get_gateway_def(gatewaytype, Some(&gatewayname))
                .expect("UplinkAssert(gd)");

            if memorysize + sv.data > gd.maxmemory * 8 {
                success = false;
                failure = "There was not enough space to fit the memory into your gateway."
                    .to_string();
                Person::change_balance(g, "PLAYER", sv.cost, "Refund from Uplink Corporation");
            } else {
                g.player_mut().gateway.change_memory_size(sv.data);
            }
        }

        //
        // SECURITY - check the player has room for the device before installing it
        //

        if hwsale.swhwtype == GATEWAYUPGRADETYPE_SECURITY {
            let (gatewaytype, gatewayname) = {
                let gateway = &g.player().gateway;
                (gateway.type_, gateway.name.clone())
            };
            let gd = g
                .world()
                .get_gateway_def(gatewaytype, Some(&gatewayname))
                .expect("UplinkAssert(gd)");
            let maxsecurity = gd.maxsecurity;

            if g.player().gateway.get_num_security() + 1 > maxsecurity {
                success = false;
                failure = "There was not enough space to fit the security device into your gateway."
                    .to_string();
                Person::change_balance(g, "PLAYER", sv.cost, "Refund from Uplink Corporation");
            } else {
                g.player_mut().gateway.give_hardware(&hwsale.title);

                let mut driver = Data::new();
                driver.set_details(DATATYPE_PROGRAM, 1, 0, 0, 1.0, SOFTWARETYPE_HWDRIVER);

                if hwsale.title == "Gateway Self Destruct" {
                    driver.set_title("Gateway_Nuke");
                } else if hwsale.title == "Gateway Motion Sensor" {
                    driver.set_title("Motion_Sensor");
                }

                g.player_mut().gateway.databank.put_data(driver);
            }
        }

        //
        // Notify the player
        //

        let mut msg = Message::new();
        msg.set_to("PLAYER");
        msg.set_from("Uplink Corporation");

        let body = if success {
            msg.set_subject("Gateway upgrade complete");
            format!(
                "The following hardware item has been successfully installed \
                 in your gateway machine : \n\n{}",
                hwsale.title
            )
        } else {
            msg.set_subject("Gateway upgrade failed");
            format!(
                "The following hardware item was not installed \
                 in your gateway machine : \n\n{}\n\nReason: {}\nYour money has been refunded.",
                hwsale.title, failure
            )
        };

        msg.set_body(&body);
        msg.send(g);

        //
        // The engineer walks away
        //

        g.player_mut().gateway.decrement_proximity(1);
    }

    pub fn run_warning(&self, g: &mut Ctx) {
        let hwsale = self.hwsale.as_ref().expect("UplinkAssert(hwsale)");

        //
        // Notify the player
        //

        let body = format!(
            "We are about to begin installation of the following hardware item \
             in your gateway machine : \n\n{}\n\n\
             We anticipate this will take 30 - 60 minutes to complete.",
            hwsale.title
        );

        let mut msg = Message::new();
        msg.set_to("PLAYER");
        msg.set_from("Uplink Corporation");
        msg.set_subject("Gateway upgrade commencing");
        msg.set_body(&body);
        msg.send(g);

        //
        // Someone is near the gateway
        //

        g.player_mut().gateway.increment_proximity(1);
    }

    pub fn get_short_string(&self) -> String {
        let title = self.hwsale.as_ref().map(|s| s.title.as_str()).unwrap_or("");
        format!("HW Install of {}, version {}", title, self.version)
    }

    pub fn get_long_string(&self) -> String {
        let title = self.hwsale.as_ref().map(|s| s.title.as_str()).unwrap_or("");
        format!(
            "Install Hardware Event\nItem : {}\nVersion : {}\n",
            title, self.version
        )
    }
}
