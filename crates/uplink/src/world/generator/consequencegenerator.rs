//! Port of source/world/generator/consequencegenerator.h + source/world/generator/consequencegenerator.cpp

use crate::ctx::Ctx;
use crate::game::data::data::{
    PROB_GENERATETRACEHACKERMISSION, TIME_LEGALACTION, TIME_LEGALACTION_WARNING,
    TIME_TACTICALACTION, TIME_TACTICALACTION_WARNING, TIME_TOPAYLEGALFINE,
};
use crate::interface::remoteinterface::remoteinterface::RemoteInterface;
use crate::world::company::mission::Mission;
use crate::world::computer::bankaccount::BankAccount;
use crate::world::computer::computer::{
    Computer, COMPUTER_TRACEACTION_DISCONNECT, COMPUTER_TRACEACTION_FINE, COMPUTER_TRACEACTION_LEGAL,
    COMPUTER_TRACEACTION_TACTICAL, COMPUTER_TRACEACTION_WARNINGMAIL,
};
use crate::world::computer::logbank::AccessLog;
use crate::world::computer::recordbank::Record;
use crate::world::connection::Connection;
use crate::world::date::Date;
use crate::world::generator::missiongenerator::{
    MissionGenerator, MISSION_CHANGEACCOUNT, MISSION_DESTROYFILE, MISSION_STEALFILE,
    MISSION_TRACEUSER,
};
use crate::world::generator::newsgenerator::NewsGenerator;
use crate::world::generator::numbergenerator::NumberGenerator;
use crate::world::message::Message;
use crate::world::person::Person;
use crate::world::rating::Rating;
use crate::world::scheduler::arrestevent::ArrestEvent;
use crate::world::scheduler::seizegatewayevent::SeizeGatewayEvent;
use crate::world::scheduler::shotbyfedsevent::ShotByFedsEvent;
use crate::world::scheduler::uplinkevent::UplinkEvent;

/// Analyses an action that has happened and generates the subsequent "consequence" actions.
/// Stateless: all functions are associated fns. People, computers are passed by key
/// (Person name, Computer name); missions are passed as snapshots of the mission being judged.
pub struct ConsequenceGenerator;

// ================================================================================================
// Helpers
// ================================================================================================

/// C++ `person == GetPlayer()`: the Person behind `key` is the player's ("PLAYER").
fn is_player(g: &Ctx, key: &str) -> bool {
    g.world().get_person(key).is_some_and(|p| p.name == "PLAYER")
}

/// C++ `person->GetConnection()->GetTarget() && strcmp(target, ip) == 0`.
fn connected_to(g: &Ctx, pname: &str, ip: &str) -> bool {
    g.world()
        .get_person(pname)
        .is_some_and(|p| p.get_connection().get_target() == Some(ip))
}

/// C++ `GetVLocation(ip)->GetComputer()` (asserted): returns the computer's (name, companyname).
fn computer_at(g: &Ctx, ip: &str) -> (String, String) {
    let world = g.world();
    let vl = world.get_vlocation(ip).expect("UplinkAssert(vl)");
    let comp = vl.get_computer(world).expect("UplinkAssert(comp)");
    (comp.name.clone(), comp.companyname.clone())
}

/// C++ `GetPlayer()->handle`: the player's agent handle.
fn player_handle(g: &Ctx) -> String {
    g.world()
        .get_person("PLAYER")
        .and_then(|p| p.agent.as_deref())
        .map(|a| a.handle.clone())
        .unwrap_or_default()
}

/// Mutable form of `RecordGenerator::GetCriminal` (the Rust `get_criminal` is read-only, and the
/// C++ caller edits the record it returns).
fn criminal_record_mut<'a>(g: &'a mut Ctx, personname: &str) -> Option<&'a mut Record> {
    let name = if personname == "PLAYER" {
        player_handle(g)
    } else {
        personname.to_string()
    };
    g.world_mut()
        .get_computer_mut("Global Criminal Database")
        .expect("UplinkAssert(comp)")
        .recordbank
        .get_record_from_name_mut(&name)
}

/// C sscanf `%d`: optional whitespace, then a leading integer; 0 when there is none.
fn scan_int(s: &str) -> i32 {
    let s = s.trim_start();
    let end = s
        .char_indices()
        .find(|&(i, c)| !(c.is_ascii_digit() || (i == 0 && (c == '+' || c == '-'))))
        .map_or(s.len(), |(i, _)| i);
    s[..end].parse().unwrap_or(0)
}

/// C sscanf `%s %s`: the first two whitespace-separated words (missing ones read as empty).
fn two_words(s: &str) -> (String, String) {
    let mut words = s.split_whitespace();
    (
        words.next().unwrap_or("").to_string(),
        words.next().unwrap_or("").to_string(),
    )
}

/// `ArrestEvent` scheduled at `due`, with a warning at `warn` (C++ SetName/SetReason/[SetIP]/SetRunDate,
/// then ScheduleWarning and ScheduleEvent).
fn schedule_arrest(
    g: &mut Ctx,
    name: &str,
    reason: &str,
    ip: Option<&str>,
    due: &Date,
    warn: &Date,
) {
    let mut arrest = ArrestEvent::new();
    arrest.set_name(name);
    arrest.set_reason(reason);
    if let Some(ip) = ip {
        arrest.set_ip(ip);
    }
    let mut event = UplinkEvent::ArrestEvent(arrest);
    event.set_run_date(due);
    g.world_mut().scheduler.schedule_warning(&event, warn);
    g.world_mut().scheduler.schedule_event(event);
}

/// `SeizeGatewayEvent` for the player, using the player's current gateway id.
fn schedule_seize_gateway(g: &mut Ctx, name: &str, reason: &str, due: &Date, warn: &Date) {
    let mut seize = SeizeGatewayEvent::new();
    seize.set_name(name);
    seize.set_gateway_id(g.player().gateway.id);
    seize.set_reason(reason);
    let mut event = UplinkEvent::SeizeGatewayEvent(seize);
    event.set_run_date(due);
    g.world_mut().scheduler.schedule_warning(&event, warn);
    g.world_mut().scheduler.schedule_event(event);
}

/// `ShotByFedsEvent` for an NPC agent.
fn schedule_shot_by_feds(g: &mut Ctx, name: &str, reason: &str, due: &Date, warn: &Date) {
    let mut shot = ShotByFedsEvent::new();
    shot.set_name(name);
    shot.set_reason(reason);
    let mut event = UplinkEvent::ShotByFedsEvent(shot);
    event.set_run_date(due);
    g.world_mut().scheduler.schedule_warning(&event, warn);
    g.world_mut().scheduler.schedule_event(event);
}

impl ConsequenceGenerator {
    pub fn initialise() {}

    // Event processing for all people

    /// `person` = Person name ("PLAYER" for the player), `comp` = Computer name.
    pub fn caught_hacking(g: &mut Ctx, person: &str, comp: &str) {
        //
        // Get the computer to "remember" that is was hacked
        //

        g.world_mut()
            .get_computer_mut(comp)
            .expect("UplinkAssert(comp)")
            .add_to_recent_hacks(1);

        let (compname, companyname, ip, traceaction) = {
            let c = g.world().get_computer(comp).expect("UplinkAssert(comp)");
            (c.name.clone(), c.companyname.clone(), c.ip.clone(), c.traceaction)
        };
        let pname = g
            .world()
            .get_person(person)
            .expect("UplinkAssert(person)")
            .name
            .clone();

        //
        // Lookup the company owner
        //

        let size = g
            .world()
            .get_company(&companyname)
            .expect("UplinkAssert(company)")
            .size;
        let severity = size / 10 + 1;

        //
        // SPECIAL CASE
        // If this is the player, and he has the VIRTUANET gateway (total anonymity)
        //

        if pname == "PLAYER" && g.player().gateway.type_ == 3 {
            // Disconnect user if he is still connected here
            if connected_to(g, &pname, &ip) {
                Connection::disconnect(g, &pname);
                Connection::reset(g, &pname);
                RemoteInterface::run_new_location(g);
                RemoteInterface::run_screen(g, 2);
            }

            // Don't do anything else
            return;
        }

        //
        // Add this to their criminal record
        // And change their ratings
        //

        if traceaction > 3 {
            // ie not "NONE", "DISCONNECT" or "WARNINGMAIL"

            if let Some(rec) = criminal_record_mut(g, &pname) {
                let existing = rec
                    .get_field("Convictions")
                    .expect("UplinkAssert(existing)")
                    .to_string();

                let mut newconvictions = String::new();
                if !existing.contains("None") {
                    newconvictions.push_str(&existing);
                    newconvictions.push('\n');
                }
                newconvictions.push_str("Unauthorised systems access\n");

                rec.change_field("Convictions", &newconvictions);
            }

            //
            // Decrease ratings
            //

            Rating::change_uplink_score(g, &pname, -severity);
            Rating::change_neuromancer_score(g, &pname, -severity);
        }

        //
        // traceaction is a bitfield - so act on each possibility
        //

        if traceaction & COMPUTER_TRACEACTION_DISCONNECT != 0 {
            // Disconnect user if he is still connected here
            if connected_to(g, &pname, &ip) {
                Connection::disconnect(g, &pname);
                Connection::reset(g, &pname);

                if pname == "PLAYER" {
                    RemoteInterface::run_new_location(g);
                    RemoteInterface::run_screen(g, 2);
                }
            }
        }

        if traceaction & COMPUTER_TRACEACTION_WARNINGMAIL != 0 {
            // Send warning mail
            let mut msg = Message::new();
            msg.set_to(&pname);
            msg.set_from(&companyname);
            msg.set_subject("You were caught hacking our computer systems");
            msg.set_body(concat!(
                "Our system administrator has informed us that you have been using ",
                "one of our accounts to gain unauthorised access to our computer systems.\n",
                "We have disconnected your computer from our system and have changed ",
                "the access codes to this account.  Do not attempt to gain access to ",
                "our systems again.\n",
                "We will be notifying your employer and the police of this action.  ",
                "This was a warning - in future, more severe action will be taken."
            ));
            msg.send(g);

            // Reset defenses
            Computer::change_security_codes(g, comp);
        }

        if traceaction & COMPUTER_TRACEACTION_FINE != 0 {
            // Pay a fine
            let finesize = severity * 1000;

            // No point in fining anyone other than the player
            // As no one would ever notice
            if pname == "PLAYER" {
                // Generate a "pay fine" mission
                let reason = format!(
                    "Our system security agents have caught you making unauthorised access to our computer system {}.",
                    compname
                );

                // You have a few days to complete it
                let mut date = g.world().date.clone();
                date.advance_minute(TIME_TOPAYLEGALFINE);
                let (day, month, year) = (date.get_day(), date.get_month(), date.get_year());
                date.set_date(0, 0, 0, day, month, year); // Round to midnight

                let mission = MissionGenerator::generate_pay_fine(
                    g,
                    &pname,
                    &companyname,
                    finesize,
                    &date,
                    &reason,
                );
                Person::give_mission(g, "PLAYER", mission);
            }

            // Reset defenses
            Computer::change_security_codes(g, comp);
        }

        if traceaction & COMPUTER_TRACEACTION_LEGAL != 0 {
            // Legal action begins - the person is arrested
            // (In a few hours)
            // If its an agent, seize his gateway

            let mut duedate = g.world().date.clone();
            duedate.advance_minute(TIME_LEGALACTION);

            let mut warningdate = duedate.clone();
            warningdate.advance_minute(-TIME_LEGALACTION_WARNING);

            if pname != "PLAYER" {
                let reason = format!("hacking into the {}.\n", compname);
                schedule_arrest(g, &pname, &reason, Some(&ip), &duedate, &warningdate);
            } else {
                // This is the player - seize his gateway
                let reason = format!("Hacking into the {}.\n", compname);
                schedule_seize_gateway(g, &pname, &reason, &duedate, &warningdate);
            }
        }

        if traceaction & COMPUTER_TRACEACTION_TACTICAL != 0 {
            // Tactical action begins - the person is shot
            // (In a few minutes)
            // If its an agent, seize his gateway

            let mut duedate = g.world().date.clone();
            duedate.advance_minute(TIME_TACTICALACTION);

            let mut warningdate = duedate.clone();
            warningdate.advance_minute(-TIME_TACTICALACTION_WARNING);

            if pname != "PLAYER" {
                let reason = format!("hacking into the {}.\n", compname);
                schedule_shot_by_feds(g, &pname, &reason, &duedate, &warningdate);
            } else {
                // This is the player - seize his gateway
                let reason = format!("Hacking into the {}.\n", compname);
                schedule_seize_gateway(g, &pname, &reason, &duedate, &warningdate);
            }
        }
    }

    /// `person` = Person name, `comp` = Computer name (`None` when the C++ passed NULL).
    pub fn arrested(g: &mut Ctx, person: &str, comp: Option<&str>, reason: &str) {
        /*
            For future versions : generate news stories,
            vendetta's etc from this

            */

        NewsGenerator::arrested(g, person, comp, reason);
    }

    pub fn didnt_pay_fine(g: &mut Ctx, person: &str, fine: &Mission) {
        //
        // Schedule an ARREST event for this person
        // Or a SeizeGateway event if its the player
        //

        let mut duedate = g.world().date.clone();
        duedate.advance_minute(TIME_LEGALACTION);

        let mut warningdate = duedate.clone();
        warningdate.advance_minute(-TIME_LEGALACTION_WARNING);

        let finesize = scan_int(&fine.completionc);

        let pname = g
            .world()
            .get_person(person)
            .expect("UplinkAssert(person)")
            .name
            .clone();

        if pname != "PLAYER" {
            let jailtime = finesize / 1000;
            let reason = format!(
                "For refusing to pay a {}c fine.\nSend to jail for {} years.\n",
                finesize, jailtime
            );
            schedule_arrest(g, &pname, &reason, None, &duedate, &warningdate);
        } else {
            // This is the player - seize his gateway
            let reason = format!("For refusing to pay a {}c fine.\n", finesize);
            schedule_seize_gateway(g, &pname, &reason, &duedate, &warningdate);
        }
    }

    /// Empty in the C++ ("future versions").
    pub fn seize_gateway(_person: &str, _reason: &str) {}

    /// Empty in the C++ ("do something here").
    pub fn shot_by_feds(_person: &str, _reason: &str) {}

    // Event processing for company computers

    pub fn computer_hacked(g: &mut Ctx, comp: &str, al: &AccessLog) {
        g.world_mut()
            .get_computer_mut(comp)
            .expect("UplinkAssert(comp)")
            .add_to_recent_hacks(1);

        NewsGenerator::computer_hacked(g, comp, al);

        let (ip, traceaction) = {
            let c = g.world().get_computer(comp).expect("UplinkAssert(comp)");
            (c.ip.clone(), c.traceaction)
        };

        if traceaction >= COMPUTER_TRACEACTION_LEGAL {
            let hacker_name = g
                .world()
                .get_person(&al.fromname)
                .expect("UplinkAssert(hacker)")
                .name
                .clone();

            // C++ also built an unused `testdate` (12 hours back); it never affected anything, so it is not ported.
            if hacker_name == "PLAYER"
                || NumberGenerator::random_number(100) < PROB_GENERATETRACEHACKERMISSION
            {
                // If this hasn't already been noticed, generate a new trace hacker mission
                // No point in generating another tracehacker mission if that person and machine
                // are already under investigation

                let alreadynoticed = {
                    let uplink = g
                        .world()
                        .get_company("Uplink")
                        .and_then(|c| c.uplink.as_deref())
                        .expect("UplinkAssert(uplink)");

                    uplink.missions.iter().any(|m| {
                        m.type_ == MISSION_TRACEUSER
                            && m.completiona == hacker_name
                            && m.links.first().is_some_and(|l| *l == ip)
                    })
                };

                if !alreadynoticed {
                    let _ = MissionGenerator::generate_trace_hacker_computer(g, comp, &hacker_name);
                }
            }
        }
    }

    // Event processing for completed missions

    pub fn mission_completed(g: &mut Ctx, mission: &Mission, person: &str) {
        match mission.type_ {
            MISSION_STEALFILE => Self::mission_completed_steal_file(g, mission, person),
            MISSION_DESTROYFILE => Self::mission_completed_destroy_file(g, mission, person),
            MISSION_CHANGEACCOUNT => Self::mission_completed_change_account(g, mission, person),
            MISSION_TRACEUSER => Self::mission_completed_trace_user(g, mission, person),
            // MISSION_NONE and default: nothing to do
            _ => {}
        }
    }

    pub fn mission_completed_steal_file(g: &mut Ctx, mission: &Mission, person: &str) {
        let ip = mission.completiona.clone();
        let filename = mission.completionb.clone();
        let filetype = mission.completione.clone();

        // C++: sscanf(completionD, "%d %d", &numfiles, &totalfilesize); numfiles is never used.
        let totalfilesize = mission.completiond.split_whitespace().nth(1).map_or(0, scan_int);

        if filename == "ALL" {
            let (compname, _) = computer_at(g, &ip);
            NewsGenerator::all_files_stolen(g, &compname, &filetype, totalfilesize);

            if is_player(g, person)
                || NumberGenerator::random_number(100) < PROB_GENERATETRACEHACKERMISSION
            {
                let _ = MissionGenerator::generate_trace_hacker_mission(g, mission, person);
            }
        }
    }

    pub fn mission_completed_destroy_file(g: &mut Ctx, mission: &Mission, person: &str) {
        let filename = mission.completionb.clone();

        if filename == "ALL" {
            if is_player(g, person)
                || NumberGenerator::random_number(100) < PROB_GENERATETRACEHACKERMISSION
            {
                let _ = MissionGenerator::generate_trace_hacker_mission(g, mission, person);
            }
        }
    }

    pub fn mission_completed_change_account(g: &mut Ctx, mission: &Mission, person: &str) {
        if is_player(g, person)
            || NumberGenerator::random_number(100) < PROB_GENERATETRACEHACKERMISSION
        {
            // Extract values from the mission

            let (source_ip, source_accs) = two_words(&mission.completiona);
            let (target_ip, target_accs) = two_words(&mission.completionb);
            let amount = scan_int(&mission.completionc);

            // Get the source and target computers

            let (source_name, source_company) = computer_at(g, &source_ip);
            let (target_name, _) = computer_at(g, &target_ip);

            // Get the source and target bank accounts

            let source_acc = BankAccount::get_account(g.world(), &source_ip, &source_accs)
                .cloned()
                .expect("UplinkAssert(source_acc)");
            let target_acc = BankAccount::get_account(g.world(), &target_ip, &target_accs)
                .cloned()
                .expect("UplinkAssert(target_acc)");

            // Get the employer

            g.world()
                .get_company(&source_company)
                .expect("UplinkAssert(employer)");

            let _ = MissionGenerator::generate_trace_user_money_transfer(
                g,
                &source_company,
                &source_name,
                &target_name,
                &source_acc,
                &target_acc,
                amount,
                person,
            );
        }
    }

    pub fn mission_completed_trace_user(g: &mut Ctx, mission: &Mission, _person: &str) {
        //
        // Look up the computer that was hacked
        //

        let hackedip = mission.links.first().expect("UplinkAssert(hackedip)").clone();
        let (comp_name, _) = computer_at(g, &hackedip);

        //
        // Look up the guilty person
        //

        let guilty = g
            .world()
            .get_person(&mission.completiona)
            .expect("UplinkAssert(guiltyperson)");
        let guilty_name = guilty.name.clone();
        let status = guilty.status;

        //
        // This person is effectivly caught
        //

        if status == 0 {
            Self::caught_hacking(g, &guilty_name, &comp_name);
        }
    }
}
