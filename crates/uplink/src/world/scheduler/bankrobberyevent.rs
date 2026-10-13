//! Port of source/world/scheduler/bankrobberyevent.h + source/world/scheduler/bankrobberyevent.cpp

use serde::{Deserialize, Serialize};

use crate::ctx::Ctx;
use crate::game::data::data::{
    NEUROMANCERCHANGE_ROBBANK, UPLINKCHANGE_ROBBANK,
};
use crate::world::computer::bankaccount::BankAccount;
use crate::world::computer::logbank::{LOG_TYPE_TRANSFERFROM, LOG_TYPE_TRANSFERTO};
use crate::world::date::Date;
use crate::world::rating::Rating;
use crate::world::scheduler::uplinkevent::UplinkEventBase;

/// Created when the player transfers money into his own bank account; runs roughly two
/// minutes later. If the player hasn't covered his tracks, it's game over.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BankRobberyEvent {
    pub base: UplinkEventBase,
    pub source_ip: String,
    pub source_accno: String,
    pub target_ip: String,
    pub target_accno: String,
    pub amount: i32,
    pub hackdate: Date,
}

impl Default for BankRobberyEvent {
    /// The C++ constructor fills the strings with a single space.
    fn default() -> Self {
        BankRobberyEvent {
            base: UplinkEventBase::default(),
            source_ip: " ".to_string(),
            source_accno: " ".to_string(),
            target_ip: " ".to_string(),
            target_accno: " ".to_string(),
            amount: 0,
            hackdate: Date::default(),
        }
    }
}

impl BankRobberyEvent {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run(&self, g: &mut Ctx) {
        let amount_s = format!("{}", self.amount);

        let mut rumbled = false;

        //
        // Has the player left a log on the source bank system
        //

        if let Some(source) = BankAccount::get_account(g.world(), &self.source_ip, &self.source_accno) {
            for i in 0..source.log.internallogs.size() {
                if let Some(al) = source.log.internallogs.get_data(i) {
                    if al.type_ == LOG_TYPE_TRANSFERTO {
                        if let (Some(data1), Some(data2)) = (&al.data1, &al.data2) {
                            if data1.contains(&self.target_ip)
                                && data1.contains(&self.target_accno)
                                && *data2 == amount_s
                            {
                                rumbled = true;
                                break;
                            }
                        }
                    }
                }
            }
        }

        //
        // Has the player left a log on the target bank system
        //

        if !rumbled {
            if let Some(target) = BankAccount::get_account(g.world(), &self.target_ip, &self.target_accno) {
                for i in 0..target.log.internallogs.size() {
                    if let Some(al) = target.log.internallogs.get_data(i) {
                        if al.type_ == LOG_TYPE_TRANSFERFROM {
                            if let (Some(data1), Some(data2)) = (&al.data1, &al.data2) {
                                if data1.contains(&self.source_ip)
                                    && data1.contains(&self.source_accno)
                                    && *data2 == amount_s
                                {
                                    rumbled = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        //
        // To be or not to be
        //

        if rumbled {
            let world = g.world();
            let vl = world
                .get_vlocation(&self.source_ip)
                .expect("UplinkAssert(vl)");
            let compname = vl
                .get_computer(world)
                .expect("UplinkAssert(comp)")
                .name
                .clone();

            let deathmsg = format!(
                "Disavowed by Uplink Corporation at {}\nFor attempting to steal money from {}\n",
                world.date.get_long_string(),
                compname
            );

            g.game.game_over(&deathmsg);
        } else {
            Rating::change_uplink_score(g, "PLAYER", UPLINKCHANGE_ROBBANK);
            Rating::change_neuromancer_score(g, "PLAYER", NEUROMANCERCHANGE_ROBBANK);
        }
    }

    pub fn set_details(
        &mut self,
        newsourceip: &str,
        newsourceaccno: &str,
        newtargetip: &str,
        newtargetaccno: &str,
        newamount: i32,
        newhackdate: &Date,
    ) {
        self.source_ip = newsourceip.to_string();
        self.source_accno = newsourceaccno.to_string();
        self.target_ip = newtargetip.to_string();
        self.target_accno = newtargetaccno.to_string();
        self.amount = newamount;
        self.hackdate = newhackdate.clone();
    }

    pub fn get_short_string(&self) -> String {
        "Bank Robbery Event".to_string()
    }

    pub fn get_long_string(&self) -> String {
        format!(
            "Bank Robbery Event\nSource : {}, {}\nTarget : {}, {}\nAmount : {}\n{}",
            self.source_ip,
            self.source_accno,
            self.target_ip,
            self.target_accno,
            self.amount,
            self.hackdate.get_long_string()
        )
    }
}
