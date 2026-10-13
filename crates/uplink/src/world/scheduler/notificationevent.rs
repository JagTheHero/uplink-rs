//! Port of source/world/scheduler/notificationevent.h + source/world/scheduler/notificationevent.cpp

use serde::{Deserialize, Serialize};

use crate::app::globals;
use crate::ctx::Ctx;
use crate::game::data::data::{
    COST_UPLINK_PERMONTH, FREQUENCY_ADDINTERESTONLOANS, FREQUENCY_CHECKFORSECURITYBREACHES,
    FREQUENCY_CHECKMISSIONDUEDATES, FREQUENCY_DEMOGENERATENEWMISSION, FREQUENCY_EXPIREOLDSTUFF,
    FREQUENCY_GENERATENEWMISSION, FREQUENCY_GIVEMISSIONTONPC, LARGELOAN_INTEREST, LARGELOAN_MAX,
    MAXLOAN_INTEREST, MEDIUMLOAN_INTEREST, MEDIUMLOAN_MAX, SMALLLOAN_INTEREST, SMALLLOAN_MAX,
    TIME_TOEXPIREMISSIONS, TIME_TOEXPIRENEWS,
};
use crate::game::scriptlibrary::ScriptLibrary;
use crate::world::company::company::Company;
use crate::world::company::companyuplink::CompanyUplink;
use crate::world::computer::bankaccount::BankAccount;
use crate::world::computer::computer::Computer;
use crate::world::date::Date;
use crate::world::generator::demoplotgenerator::DemoPlotGenerator;
use crate::world::generator::missiongenerator::{MissionGenerator, MISSION_SPECIAL, MISSION_TRACEUSER};
use crate::world::generator::numbergenerator::NumberGenerator;
use crate::world::generator::worldgenerator::WorldGenerator;
use crate::world::message::Message;
use crate::world::person::Person;
use crate::world::scheduler::attemptmissionevent::AttemptMissionEvent;
use crate::world::scheduler::uplinkevent::{UplinkEvent, UplinkEventBase};

pub const NOTIFICATIONEVENT_TYPE_NONE: i32 = 0;
pub const NOTIFICATIONEVENT_TYPE_GROWCOMPANIES: i32 = 1;
pub const NOTIFICATIONEVENT_TYPE_GENNEWMISSION: i32 = 2;
pub const NOTIFICATIONEVENT_TYPE_CHECKFORSECURITYBREACHES: i32 = 3;
pub const NOTIFICATIONEVENT_TYPE_CHECKMISSIONDUEDATES: i32 = 4;
pub const NOTIFICATIONEVENT_TYPE_CHECKRECENTHACKCOUNT: i32 = 5;
pub const NOTIFICATIONEVENT_TYPE_GIVEMISSIONTONPC: i32 = 6;
pub const NOTIFICATIONEVENT_TYPE_UPLINKMONTHLYFEE: i32 = 7;
pub const NOTIFICATIONEVENT_TYPE_EXPIREOLDSTUFF: i32 = 8;
pub const NOTIFICATIONEVENT_TYPE_ADDINTERESTONLOANS: i32 = 9;
pub const NOTIFICATIONEVENT_TYPE_DEMOGAMEOVER: i32 = 10;
pub const NOTIFICATIONEVENT_TYPE_DEMOGENNEWMISSION: i32 = 11;
pub const NOTIFICATIONEVENT_TYPE_BUYAGENTLIST: i32 = 12;
pub const NOTIFICATIONEVENT_TYPE_AGENTSONLISTDIE: i32 = 13;
pub const NOTIFICATIONEVENT_TYPE_WAREZGAMEOVER: i32 = 14;

/// Used for events which have no data - notifications of other events.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct NotificationEvent {
    pub base: UplinkEventBase,
    /// `TYPE`
    pub type_: i32,
}

/// `game->GetWorld()->scheduler.ScheduleEvent(...)` for a new notification of `type_` at `rundate`.
fn schedule_notification(g: &mut Ctx, type_: i32, rundate: &Date) {
    let mut event = NotificationEvent::new();
    event.set_type(type_);
    event.base.set_run_date(rundate);
    g.world_mut()
        .scheduler
        .schedule_event(UplinkEvent::NotificationEvent(event));
}

/// `ami = new AttemptMissionEvent(); ami->SetAgentName(..); ami->SetRunDate(..); ScheduleEvent(ami)`
fn schedule_attempt_mission(g: &mut Ctx, agent_name: &str, rundate: &Date) {
    let mut ami = AttemptMissionEvent::new();
    ami.set_agent_name(agent_name);
    ami.base.set_run_date(rundate);
    g.world_mut()
        .scheduler
        .schedule_event(UplinkEvent::AttemptMissionEvent(ami));
}

/// PORT: the C++ loan rates are `double` literals (0.2, 0.7, ...) but data.rs stores them as
/// `f32`; widening an f32 directly changes results (0.7f32 as f64 != 0.7), so go via the decimal text.
fn loan_rate(rate: f32) -> f64 {
    rate.to_string().parse().unwrap_or(rate as f64)
}

/// `game->GetWorld()->GetCompany("Uplink")` cast to CompanyUplink
fn uplink(g: &Ctx) -> &CompanyUplink {
    g.world()
        .get_company("Uplink")
        .and_then(|c| c.uplink.as_deref())
        .expect("UplinkAssert(uplink)")
}

fn uplink_mut(g: &mut Ctx) -> &mut CompanyUplink {
    g.world_mut()
        .get_company_mut("Uplink")
        .and_then(|c| c.uplink.as_deref_mut())
        .expect("UplinkAssert(uplink)")
}

impl NotificationEvent {
    pub fn new() -> Self {
        Self::default()
    }

    /// `SetTYPE`
    pub fn set_type(&mut self, newtype: i32) {
        self.type_ = newtype;
    }

    pub fn run(&self, g: &mut Ctx) {
        match self.type_ {
            NOTIFICATIONEVENT_TYPE_GROWCOMPANIES => self.apply_monthly_growth(g),
            NOTIFICATIONEVENT_TYPE_GENNEWMISSION => self.generate_new_mission(g),
            NOTIFICATIONEVENT_TYPE_CHECKFORSECURITYBREACHES => self.check_for_security_breaches(g),
            NOTIFICATIONEVENT_TYPE_CHECKMISSIONDUEDATES => self.check_mission_due_dates(g),
            NOTIFICATIONEVENT_TYPE_CHECKRECENTHACKCOUNT => self.check_recent_hack_count(g),
            NOTIFICATIONEVENT_TYPE_GIVEMISSIONTONPC => self.give_mission_to_npc(g),
            NOTIFICATIONEVENT_TYPE_UPLINKMONTHLYFEE => self.pay_uplink_monthly_fee(g),
            NOTIFICATIONEVENT_TYPE_EXPIREOLDSTUFF => self.expire_old_stuff(g),
            NOTIFICATIONEVENT_TYPE_ADDINTERESTONLOANS => self.add_interest_on_loans(g),
            NOTIFICATIONEVENT_TYPE_DEMOGAMEOVER => self.demo_game_over(g),
            NOTIFICATIONEVENT_TYPE_DEMOGENNEWMISSION => self.demo_generate_new_mission(g),
            NOTIFICATIONEVENT_TYPE_BUYAGENTLIST => self.buy_agent_list(g),
            NOTIFICATIONEVENT_TYPE_AGENTSONLISTDIE => self.agents_on_list_die(g),
            NOTIFICATIONEVENT_TYPE_WAREZGAMEOVER => self.warez_game_over(g),
            NOTIFICATIONEVENT_TYPE_NONE => eprintln!("Notification event type not specified"),
            _ => panic!("Unrecognised notification type"),
        }
    }

    pub fn get_short_string(&self) -> String {
        let s = match self.type_ {
            NOTIFICATIONEVENT_TYPE_GROWCOMPANIES => "Grow companies",
            NOTIFICATIONEVENT_TYPE_GENNEWMISSION => "Generate new mission",
            NOTIFICATIONEVENT_TYPE_CHECKFORSECURITYBREACHES => "Check for security breaches",
            NOTIFICATIONEVENT_TYPE_CHECKMISSIONDUEDATES => "Check mission due dates",
            NOTIFICATIONEVENT_TYPE_CHECKRECENTHACKCOUNT => "Check recent hack count",
            NOTIFICATIONEVENT_TYPE_GIVEMISSIONTONPC => "Give mission to NPC",
            NOTIFICATIONEVENT_TYPE_UPLINKMONTHLYFEE => "Pay Uplink monthly fee",
            NOTIFICATIONEVENT_TYPE_EXPIREOLDSTUFF => "Expire old stuff",
            NOTIFICATIONEVENT_TYPE_ADDINTERESTONLOANS => "Add Interest to loans",
            NOTIFICATIONEVENT_TYPE_DEMOGAMEOVER => "Demo Game Over",
            NOTIFICATIONEVENT_TYPE_DEMOGENNEWMISSION => "Demo Generate Mission",
            NOTIFICATIONEVENT_TYPE_BUYAGENTLIST => "Buy Agent List",
            NOTIFICATIONEVENT_TYPE_AGENTSONLISTDIE => "Agents on list die",
            NOTIFICATIONEVENT_TYPE_WAREZGAMEOVER => "Warez Game Over",
            NOTIFICATIONEVENT_TYPE_NONE => {
                eprintln!("Notification event type not specified");
                "Unknown notification"
            }
            _ => panic!("Unrecognised notification type"),
        };
        s.to_string()
    }

    pub fn get_long_string(&self) -> String {
        self.get_short_string()
    }

    /// Schedules the initial set of recurring notification events for a new game.
    pub fn schedule_starting_events(g: &mut Ctx) {
        let now = g.world().date.clone();

        let mut duedate = Date::default();

        schedule_notification(g, NOTIFICATIONEVENT_TYPE_GROWCOMPANIES, &now);
        schedule_notification(g, NOTIFICATIONEVENT_TYPE_GENNEWMISSION, &now);
        schedule_notification(g, NOTIFICATIONEVENT_TYPE_CHECKFORSECURITYBREACHES, &now);
        schedule_notification(g, NOTIFICATIONEVENT_TYPE_CHECKMISSIONDUEDATES, &now);

        // Start of next month, day 1, 01:01:01
        duedate.set_date_from(&now);
        duedate.advance_month(1);
        duedate.set_date(1, 1, 1, 1, duedate.get_month(), duedate.get_year());
        schedule_notification(g, NOTIFICATIONEVENT_TYPE_CHECKRECENTHACKCOUNT, &duedate);

        schedule_notification(g, NOTIFICATIONEVENT_TYPE_GIVEMISSIONTONPC, &now);

        // Start of next month, day 1, 01:01:01
        duedate.set_date_from(&now);
        duedate.advance_month(1);
        duedate.set_date(1, 1, 1, 1, duedate.get_month(), duedate.get_year());
        schedule_notification(g, NOTIFICATIONEVENT_TYPE_UPLINKMONTHLYFEE, &duedate);

        schedule_notification(g, NOTIFICATIONEVENT_TYPE_EXPIREOLDSTUFF, &now);
        schedule_notification(g, NOTIFICATIONEVENT_TYPE_ADDINTERESTONLOANS, &now);

        if globals::DEMOGAME {
            schedule_notification(g, NOTIFICATIONEVENT_TYPE_DEMOGENNEWMISSION, &now);
        }
    }

    fn apply_monthly_growth(&self, g: &mut Ctx) {
        let names: Vec<String> = g.world().companies.keys_all();
        for name in &names {
            Company::grow(g, name, 30);
            if let Some(company) = g.world_mut().get_company_mut(name) {
                company.vary_growth();
            }
        }

        // Schedule another at the start of the next month

        let mut duedate = Date::default();
        duedate.set_date_from(&self.base.rundate);
        duedate.advance_month(1);
        duedate.set_date(
            duedate.get_second(),
            duedate.get_minute(),
            duedate.get_hour(),
            1,
            duedate.get_month(),
            duedate.get_year(),
        );
        schedule_notification(g, NOTIFICATIONEVENT_TYPE_GROWCOMPANIES, &duedate);
    }

    fn generate_new_mission(&self, g: &mut Ctx) {
        MissionGenerator::generate_mission(g);

        // Schedule another

        let mut duedate = Date::default();
        duedate.set_date_from(&self.base.rundate);
        duedate.advance_minute(FREQUENCY_GENERATENEWMISSION);
        schedule_notification(g, NOTIFICATIONEVENT_TYPE_GENNEWMISSION, &duedate);
    }

    fn check_for_security_breaches(&self, g: &mut Ctx) {
        let names: Vec<String> = g.world().computers.keys_all();
        for name in &names {
            Computer::check_for_security_breaches(g, name);
        }

        // Schedule another

        let mut duedate = Date::default();
        duedate.set_date_from(&self.base.rundate);
        duedate.advance_minute(FREQUENCY_CHECKFORSECURITYBREACHES);
        schedule_notification(g, NOTIFICATIONEVENT_TYPE_CHECKFORSECURITYBREACHES, &duedate);
    }

    fn check_mission_due_dates(&self, g: &mut Ctx) {
        Person::check_mission_due_dates(g, "PLAYER");

        // Schedule another

        let mut duedate = Date::default();
        duedate.set_date_from(&self.base.rundate);
        duedate.advance_minute(FREQUENCY_CHECKMISSIONDUEDATES);
        schedule_notification(g, NOTIFICATIONEVENT_TYPE_CHECKMISSIONDUEDATES, &duedate);
    }

    fn check_recent_hack_count(&self, g: &mut Ctx) {
        let names: Vec<String> = g.world().computers.keys_all();
        for name in &names {
            Computer::update_recent_hacks(g, name);
        }

        // Schedule another

        let mut duedate = Date::default();
        duedate.set_date_from(&self.base.rundate);

        // ***** There must be exaclty 4 of these in a month ******

        if duedate.get_day() < 22 {
            duedate.advance_day(7);
        } else {
            duedate.advance_month(1);
            duedate.set_date(1, 1, 1, 1, duedate.get_month(), duedate.get_year());
        }

        schedule_notification(g, NOTIFICATIONEVENT_TYPE_CHECKRECENTHACKCOUNT, &duedate);
    }

    fn give_mission_to_npc(&self, g: &mut Ctx) {
        //
        // Choose a random Uplink Agent (not the player)
        //

        let agent_name = WorldGenerator::get_random_agent(g);
        let agent_uplinkrating = g
            .world()
            .get_person(&agent_name)
            .expect("UplinkAssert(agent)")
            .rating
            .uplinkrating;
        let worlddate = g.world().date.clone();

        //
        // Build a list of candidate missions
        // Sorted into difficulty order
        // Keep an eye out for "priority" missions
        //
        // PORT: the C++ `missions` list holds Mission pointers; here it holds indexes into
        // Uplink's `missions` list (`missions_index` in the C++), which is not changed while
        // the list is built.

        let mut missions: Vec<usize> = Vec::new();
        let mut prioritymission: Option<usize> = None;

        {
            let fullist = &uplink(g).missions;

            for i in 0..fullist.len() {
                let mission = &fullist[i];

                if agent_uplinkrating >= mission.minuplinkrating
                    && mission.type_ != MISSION_SPECIAL
                    && !(mission.type_ == MISSION_TRACEUSER
                        && mission.completiona == agent_name)
                {
                    if mission.npcpriority {
                        // This is a priority mission - accept it!
                        prioritymission = Some(i);
                        break;
                    }

                    let mut testdate = Date::default();
                    testdate.set_date_from(&mission.createdate);
                    testdate.advance_day(7);

                    if testdate.before(&worlddate) {
                        // This is not a priority mission, so let it hang around for a week
                        // before giving it to NPC agents

                        let mut inserted = false;

                        for j in 0..missions.len() {
                            if mission.difficulty < fullist[missions[j]].difficulty {
                                missions.insert(j, i);
                                inserted = true;
                                break;
                            }
                        }

                        if !inserted {
                            missions.push(i);
                        }
                    }
                }
            }
        }

        //
        // If we have found a priority mission, accept it immediately
        // Otherwise, choose a mission from the available list
        //

        if let Some(priorityindex) = prioritymission {
            let mission = uplink(g).missions[priorityindex].clone();
            Person::give_mission(g, &agent_name, mission);
            uplink_mut(g).missions.remove(priorityindex);

            // Schedule an event to make the agent attempt the mission
            // in a few hours

            let mut duedate = Date::default();
            duedate.set_date_from(&self.base.rundate);
            duedate.advance_hour(NumberGenerator::random_number(2) + 1);

            schedule_attempt_mission(g, &agent_name, &duedate);
        } else {
            //
            // Choose a mission based on a Normal  distribution curve, peeking at 1 or 2 difficulty
            // levels higher than your uplink rating.
            //

            let nummissions = missions.len() as i32;

            if nummissions > 0 {
                let mut missionindex = NumberGenerator::random_normal_number(
                    (nummissions / 4) as f32,
                    (nummissions / 2) as f32,
                ) as i32;
                if missionindex < 0 {
                    missionindex = 0;
                }

                let target_index = *missions
                    .get(missionindex as usize)
                    .expect("UplinkAssert(targetmission)");

                //
                // Give this mission to the agent
                // And remove it from the list of available missions
                //

                let mission = uplink(g).missions[target_index].clone();
                Person::give_mission(g, &agent_name, mission);
                uplink_mut(g).missions.remove(target_index);

                // Schedule an event to make the agent attempt the mission
                // in a few hours

                let mut duedate = Date::default();
                duedate.set_date_from(&self.base.rundate);
                duedate.advance_hour(NumberGenerator::random_number(4) + 1);

                schedule_attempt_mission(g, &agent_name, &duedate);
            }
        }

        //
        // Schedule this GiveMissionToNPC event again in a few hours
        //

        let mut duedate = Date::default();
        duedate.set_date_from(&self.base.rundate);
        duedate.advance_minute(FREQUENCY_GIVEMISSIONTONPC);
        schedule_notification(g, NOTIFICATIONEVENT_TYPE_GIVEMISSIONTONPC, &duedate);
    }

    fn pay_uplink_monthly_fee(&self, g: &mut Ctx) {
        //
        // Deduct the fee and send a message to
        // everyone with an Uplink Rating
        //
        // people.ConvertToDArray (BTree pre-order); the player is the entry under "PLAYER".

        let names: Vec<String> = g.world().people.keys_all();

        for name in &names {
            let uplinkrating = g
                .world()
                .get_person(name)
                .expect("UplinkAssert(person)")
                .rating
                .uplinkrating;

            if uplinkrating > 0 {
                Person::change_balance(
                    g,
                    name,
                    COST_UPLINK_PERMONTH * -1,
                    "Uplink Corporation : Monthly fee",
                );

                let body = format!(
                    "Uplink Corporation has deducted the standard monthly \
                     membership fee of {} credits from you current account.\n\n\
                     We are confident you will agree that this represents \
                     incredible value for money and wish you another successful \
                     and prosperous month with our organisation.\n\
                     Thank you.",
                    COST_UPLINK_PERMONTH
                );

                let mut m = Message::new();
                m.set_to(name);
                m.set_from("Uplink Corporation");
                m.set_subject("Uplink monthly fee deducted");
                m.set_body(&body);
                m.send(g);
            }
        }

        //
        // Schedule another in a month
        //

        let mut duedate = Date::default();
        duedate.set_date_from(&self.base.rundate);
        duedate.advance_month(1);
        schedule_notification(g, NOTIFICATIONEVENT_TYPE_UPLINKMONTHLYFEE, &duedate);
    }

    fn expire_old_stuff(&self, g: &mut Ctx) {
        //
        // Expire old access logs
        //

        let names: Vec<String> = g.world().computers.keys_all();
        for name in &names {
            Computer::manage_old_logs(g, name);
        }

        //
        // Expire old news stories and old missions
        //

        let worlddate = g.world().date.clone();
        {
            let cu = uplink_mut(g);

            for in_ in (1..cu.news.len()).rev() {
                let mut testdate = Date::default();
                testdate.set_date_from(&cu.news[in_].date);
                testdate.advance_minute(TIME_TOEXPIRENEWS);

                if testdate.before(&worlddate) {
                    cu.news.remove(in_);
                }
            }

            for im in (1..cu.missions.len()).rev() {
                let mut testdate = Date::default();
                testdate.set_date_from(&cu.missions[im].createdate);
                testdate.advance_minute(TIME_TOEXPIREMISSIONS);

                if testdate.before(&worlddate) {
                    cu.missions.remove(im);
                }
            }
        }

        //
        // Schedule another
        //

        let mut duedate = Date::default();
        duedate.set_date_from(&self.base.rundate);
        duedate.advance_minute(FREQUENCY_EXPIREOLDSTUFF);
        schedule_notification(g, NOTIFICATIONEVENT_TYPE_EXPIREOLDSTUFF, &duedate);
    }

    fn add_interest_on_loans(&self, g: &mut Ctx) {
        //
        // This event only does interest on the players loans
        //

        let numaccounts = g.player().accounts.len();
        for i in 0..numaccounts {
            let fullacc = g.player().accounts[i].clone();

            // sscanf ( fullacc, "%s %s", ip, accno )
            let mut parts = fullacc.split_whitespace();
            let ip = parts.next().unwrap_or("").to_string();
            let accno = parts.next().unwrap_or("").to_string();

            let Some(account) = BankAccount::get_account_mut(g.world_mut(), &ip, &accno) else {
                continue;
            };
            if account.loan <= 0 {
                continue;
            }

            let interest = if account.loan <= SMALLLOAN_MAX {
                loan_rate(SMALLLOAN_INTEREST)
            } else if account.loan <= MEDIUMLOAN_MAX {
                loan_rate(MEDIUMLOAN_INTEREST)
            } else if account.loan <= LARGELOAN_MAX {
                loan_rate(LARGELOAN_INTEREST)
            } else {
                loan_rate(MAXLOAN_INTEREST)
            };
            let amountpaid = (account.loan as f64 * interest / 12.0) as i32;

            account.loan += amountpaid;
            let accountnumber = account.accountnumber;
            let newloan = account.loan;

            let compname = {
                let world = g.world();
                let vl = world.get_vlocation(&ip).expect("UplinkAssert(vl)");
                vl.get_computer(world)
                    .expect("UplinkAssert(comp)")
                    .name
                    .clone()
            };

            let body = format!(
                "You have been charged interest on the following account:\n\
                 IP : {}\n\
                 BANK : {}\n\
                 ACCNO : {}\n\n\
                 You have been charged {}c extra on your loan.\
                 Your new loan is {}c.",
                ip, compname, accountnumber, amountpaid, newloan
            );

            let mut msg = Message::new();
            msg.set_from(&compname);
            msg.set_to("PLAYER");
            msg.set_subject("Interest charged");
            msg.set_body(&body);
            msg.send(g);
        }

        //
        // Schedule another
        //

        let mut duedate = Date::default();
        duedate.set_date_from(&self.base.rundate);
        duedate.advance_minute(FREQUENCY_ADDINTERESTONLOANS);
        schedule_notification(g, NOTIFICATIONEVENT_TYPE_ADDINTERESTONLOANS, &duedate);
    }

    fn demo_game_over(&self, g: &mut Ctx) {
        // This is a demo game
        // and the game has come to an end

        g.game.demo_game_over();
    }

    fn demo_generate_new_mission(&self, g: &mut Ctx) {
        DemoPlotGenerator::generate_new_mission(g, false);

        // Schedule another

        let mut duedate = Date::default();
        duedate.set_date_from(&self.base.rundate);
        duedate.advance_minute(FREQUENCY_DEMOGENERATENEWMISSION);
        schedule_notification(g, NOTIFICATIONEVENT_TYPE_DEMOGENNEWMISSION, &duedate);
    }

    fn buy_agent_list(&self, g: &mut Ctx) {
        ScriptLibrary::run_script(g, 71);
    }

    fn agents_on_list_die(&self, g: &mut Ctx) {
        ScriptLibrary::run_script(g, 72);
    }

    fn warez_game_over(&self, g: &mut Ctx) {
        // This is a warez copy
        g.game.warez_game_over();
    }
}
