//! Port of source/world/agent.h + source/world/agent.cpp

use serde::{Deserialize, Serialize};

use crate::app::globals::WAREZRELEASE;
use crate::ctx::Ctx;
use crate::game::data::data::{
    IP_LOCALHOST, MINREQUIREDRATING_DELETELOGLEVEL1, MINREQUIREDRATING_DELETELOGLEVEL2,
    MINREQUIREDRATING_DELETELOGLEVEL3, MINREQUIREDRATING_UNDELETELOGLEVEL1,
    MINREQUIREDRATING_UNDELETELOGLEVEL3, TIME_TOWAREZGAMEOVER,
};
use crate::interface::interfacescreen::ScreenSlot;
use crate::interface::remoteinterface::linksscreen_interface::LinksScreenInterface;
use crate::tosser::BTree;
use crate::world::company::mission::{
    Mission, MISSIONPAYMENT_ALLATSTART, MISSIONPAYMENT_HALFATSTART,
};
use crate::world::computer::bankaccount::BankAccount;
use crate::world::computer::computer::{
    Computer, COMPUTER_TYPE_CENTRALMAINFRAME, COMPUTER_TYPE_INTERNALSERVICESMACHINE,
    COMPUTER_TYPE_PUBLICBANKSERVER,
};
use crate::world::computer::logbank::{
    AccessLog, LogBank, LOG_NOTSUSPICIOUS, LOG_TYPE_CONNECTIONOPENED, LOG_TYPE_DELETED,
    LOG_TYPE_TEXT,
};
use crate::world::computer::recordbank::RECORDBANK_SECURITY;
use crate::world::connection::Connection;
use crate::world::date::Date;
use crate::world::generator::consequencegenerator::ConsequenceGenerator;
use crate::world::generator::missiongenerator::{
    MissionGenerator, MISSION_CHANGEACCOUNT, MISSION_DESTROYFILE, MISSION_PAYFINE,
    MISSION_REMOVECOMPUTER, MISSION_STEALFILE, MISSION_TRACEUSER,
};
use crate::world::generator::numbergenerator::NumberGenerator;
use crate::world::generator::worldgenerator::WorldGenerator;
use crate::world::message::Message;
use crate::world::person::Person;
use crate::world::scheduler::notificationevent::{
    NotificationEvent, NOTIFICATIONEVENT_TYPE_WAREZGAMEOVER,
};
use crate::world::scheduler::uplinkevent::UplinkEvent;
use crate::world::world::World;

/// The bytes a genuine world.dat carries at offset 256 (the anti-tamper marker, C++ `realbuffer`).
const WAREZ_REALBUFFER: [u8; 8] = [0xaa, 0xab, 0x15, 0xdd, 0xdd, 0xee, 0xe9, 0x2d];

/// The Agent-specific data of a Person (PORTING.md §3.6): stored in `Person::agent`.
/// All Agent *methods* are `impl Person` below; they panic (like the C++ cast would misbehave)
/// if the Person is not an agent.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Agent {
    /// The agents' "favourites" (IP), most recent first
    pub links: Vec<String>,
    /// All known access codes, indexed on ip
    pub codes: BTree<String>,
    /// The agents' missions
    pub missions: Vec<Mission>,

    pub handle: String,
}

impl Agent {
    /// `Agent::ParseAccessCode`: access codes have 2 varieties only:
    /// `... 'name' .... 'code'` or `... 'code' ....` (names/codes are enclosed in `'`).
    /// Returns `(username, password)`; in the single-code form both are the code.
    // PORT: C++ copies the code into a 256-byte buffer first; longer codes are not truncated here.
    pub fn parse_access_code(code: &str) -> Option<(String, String)> {
        let quotes: Vec<usize> = code.match_indices('\'').map(|(i, _)| i).collect();
        match quotes.len() {
            2 => {
                let s = &code[quotes[0] + 1..quotes[1]];
                Some((s.to_string(), s.to_string()))
            }
            4 => {
                let name = &code[quotes[0] + 1..quotes[1]];
                let password = &code[quotes[2] + 1..quotes[3]];
                Some((name.to_string(), password.to_string()))
            }
            _ => None,
        }
    }
}

impl Person {
    /// A new Person that is an Agent (C++ `new Agent()`).
    pub fn new_agent() -> Person {
        Person {
            agent: Some(Box::default()),
            ..Person::default()
        }
    }

    pub fn is_agent(&self) -> bool {
        self.agent.is_some()
    }

    /// The Agent data. Panics if this Person is not an agent.
    pub fn agent(&self) -> &Agent {
        self.agent.as_deref().expect("Person is not an Agent")
    }

    pub fn agent_mut(&mut self) -> &mut Agent {
        self.agent.as_deref_mut().expect("Person is not an Agent")
    }

    pub fn set_handle(&mut self, newhandle: &str) {
        self.agent_mut().handle = newhandle.to_string();
    }

    /// Adds the mission, pays any up-front payment, and gives the mission's links and codes.
    pub fn give_mission(g: &mut Ctx, name: &str, mission: Mission) {
        let paymentmethod = mission.paymentmethod;
        let payment = mission.payment;
        let links = mission.links.clone();
        let codes = mission.codes.clone();

        person_mut(g, name).agent_mut().missions.push(mission);

        // Pay the agent the starting amount
        if paymentmethod == MISSIONPAYMENT_HALFATSTART {
            Person::change_balance(g, name, payment / 2, "Anonymous benefactor");
        } else if paymentmethod == MISSIONPAYMENT_ALLATSTART {
            Person::change_balance(g, name, payment, "Anonymous benefactor");
        }

        // Transfer all the links from the mission to the agent
        for ip in &links {
            Person::give_link(g, name, ip);
        }

        // Transfer all the access codes from the mission to the agent
        for (ip, code) in codes.iter() {
            person_mut(g, name).give_code(ip, code);
        }
    }

    /// Have any missions expired? Fails them / applies consequences.
    pub fn check_mission_due_dates(g: &mut Ctx, name: &str) {
        let mut i: usize = 0;
        loop {
            let m = match person_ref(g, name).agent().missions.get(i) {
                Some(m) => m.clone(),
                None => break,
            };

            // This mission has a due date - check it
            if let Some(due) = m.get_due_date().cloned() {
                if g.world().date.after(&due) {
                    // This mission has expired
                    if m.type_ == MISSION_PAYFINE {
                        // Legal action begins
                        ConsequenceGenerator::didnt_pay_fine(g, name, &m);
                    } else {
                        MissionGenerator::mission_failed(g, &m, name, "Time has run out.");
                    }

                    // Remove the mission (C++ `i--` then `i++`: the next mission takes this index)
                    let list = &mut person_mut(g, name).agent_mut().missions;
                    if i < list.len() {
                        list.remove(i);
                    }
                    continue;
                }
            }

            i += 1;
        }
    }

    /// Adds `ip` to the favourites (at the start), un-hides its VLocation on the map and
    /// refreshes the player's links screen if showing.
    pub fn give_link(g: &mut Ctx, name: &str, ip: &str) {
        if !person_ref(g, name).has_link(ip) {
            person_mut(g, name)
                .agent_mut()
                .links
                .insert(0, ip.to_string());

            // If this was the player and he is looking at his links screen
            // Update it now
            if name == "PLAYER"
                && g.world().get_player().remotehost == IP_LOCALHOST
                && g.interface().ri.currentscreenindex == 0
            {
                let links = person_ref(g, name).agent().links.clone();
                ScreenSlot::call(g, remote_screen_slot, |s, g| {
                    if let Some(ls) = s.as_any_mut().downcast_mut::<LinksScreenInterface>() {
                        ls.set_full_list(g, &links);
                        ls.apply_filter(g, None);
                    }
                });
            }
        }

        // Make sure it is set to appear on the world map
        let vl = g
            .world_mut()
            .get_vlocation_mut(ip)
            .expect("give_link: no VLocation for ip");
        vl.set_displayed(true);
    }

    pub fn has_link(&self, ip: &str) -> bool {
        self.agent().links.iter().any(|l| l == ip)
    }

    pub fn remove_link(&mut self, ip: &str) {
        let links = &mut self.agent_mut().links;
        if let Some(i) = links.iter().position(|l| l == ip) {
            links.remove(i);
        }
    }

    /// Remembers an access code for `newip` (replacing an older code for the same username).
    /// The codes BTree accepts duplicate keys: a code with another username for a known ip is
    /// added as a further entry.
    pub fn give_code(&mut self, newip: &str, newcode: &str) {
        let agent = self.agent_mut();

        // Do we already have this code?
        let mut found = false;

        for (thisip, thiscode) in agent.codes.iter_mut() {
            if thisip == newip {
                if thiscode == newcode {
                    // We already have the IP and the code
                    found = true;
                    break;
                } else {
                    // We have the IP but not exactly the same code
                    let successa = Agent::parse_access_code(newcode);
                    let successb = Agent::parse_access_code(thiscode);

                    if let (Some((newusername, _)), Some((thisusername, _))) = (successa, successb) {
                        if newusername == thisusername {
                            // Usernames match up, but we have an old code
                            // Replace old code with new code
                            *thiscode = newcode.to_string();
                            found = true;
                            break;
                        }
                    }
                }
            }
        }

        if !found {
            agent.codes.put_data(newip, newcode.to_string());
        }
    }

    /// `Agent::GiveMessage`: stores the message, then gives the agent its links and codes.
    pub fn give_message_agent(g: &mut Ctx, name: &str, message: Message) {
        let links = message.links.clone();
        let codes = message.codes.clone();

        // Person::GiveMessage (base): just stores it
        person_mut(g, name).give_message_base(message);

        // Transfer all links from the message to the agent
        for ip in &links {
            Person::give_link(g, name, ip);
        }

        // Transfer all access codes from the message to the agent
        for (ip, code) in codes.iter() {
            person_mut(g, name).give_code(ip, code);
        }
    }

    /// Returns access level or -1
    // PORT: read-only lookups through the world (computer record bank).
    pub fn has_account(&self, world: &World, ip: &str) -> i32 {
        // Parse the access code for this IP
        let code = match self.agent().codes.get(ip) {
            Some(code) => code,
            None => return -1,
        };
        // PORT: C++ ignores the parse result and then reads the uninitialised username/password
        // buffers; an unparseable code is treated as "no account" instead.
        let (username, password) = match Agent::parse_access_code(code) {
            Some(up) => up,
            None => return -1,
        };

        // Lookup the computer
        let vl = match world.get_vlocation(ip) {
            Some(vl) => vl,
            None => return -1,
        };
        let comp = match world.get_computer(&vl.computer) {
            Some(comp) => comp,
            None => return -1,
        };

        // Lookup the account we have compromised
        match comp
            .recordbank
            .get_record_from_name_password(&username, &password)
        {
            Some(rec) => {
                // Check the security level
                let mut security = -1;
                if let Some(securitytext) = rec.get_field(RECORDBANK_SECURITY) {
                    if let Some(value) = scan_int(securitytext) {
                        security = value;
                    }
                }
                security
            }
            None => -1,
        }
    }

    /// `Agent::CreateNewAccount`: base version, then remembers the access code. Returns account number.
    pub fn create_new_account_agent(
        g: &mut Ctx,
        name: &str,
        bankip: &str,
        accname: &str,
        password: &str,
        balance: i32,
        loan: i32,
    ) -> i32 {
        let accno =
            Person::create_new_account_base(g, name, bankip, accname, password, balance, loan);

        // Give the access code to this agent
        let code = Computer::generate_access_code_accno(accno, password);
        person_mut(g, name).give_code(bankip, &code);

        accno
    }

    /// Attempts mission 0 (an NPC agent doing a mission), then discards it.
    pub fn attempt_mission(g: &mut Ctx, name: &str) {
        let m = match first_mission(g, name) {
            Some(m) => m,
            None => return,
        };

        // Perform the mission
        match m.type_ {
            MISSION_STEALFILE => Person::attempt_mission_steal_file(g, name),
            MISSION_DESTROYFILE => Person::attempt_mission_delete_file(g, name),
            MISSION_CHANGEACCOUNT => Person::attempt_mission_change_account(g, name),
            MISSION_TRACEUSER => Person::attempt_mission_trace_user(g, name),
            MISSION_REMOVECOMPUTER => Person::attempt_mission_remove_computer(g, name),
            _ => {}
        }

        let list = &mut person_mut(g, name).agent_mut().missions;
        if !list.is_empty() {
            list.remove(0);
        }
    }

    pub fn attempt_mission_steal_file(g: &mut Ctx, name: &str) {
        let m = match first_mission(g, name) {
            Some(m) => m,
            None => return,
        };

        let targetip = m.completiona.clone();
        let filename = m.completionb.clone();

        if filename == "ALL" {
            Person::establish_connection(g, name, &targetip);

            computer_name_at(g, &targetip);

            ConsequenceGenerator::mission_completed(g, &m, name);
        }
        // else: Not worth bothering - nobody will notice a single file stolen
    }

    pub fn attempt_mission_delete_file(g: &mut Ctx, name: &str) {
        let m = match first_mission(g, name) {
            Some(m) => m,
            None => return,
        };

        let targetip = m.completiona.clone();
        let filename = m.completionb.clone();

        if filename == "ALL" {
            Person::establish_connection(g, name, &targetip);

            let compname = computer_name_at(g, &targetip);
            comp_mut(g, &compname).databank.format();

            ConsequenceGenerator::mission_completed(g, &m, name);
        }
        // else: Not worth bothering - nobody will notice a single file deletion
    }

    pub fn attempt_mission_change_account(g: &mut Ctx, name: &str) {
        let m = match first_mission(g, name) {
            Some(m) => m,
            None => return,
        };

        let (sourceip, sourceaccno) = scan_two_words(&m.completiona);
        let (targetip, targetaccno) = scan_two_words(&m.completionb);
        // PORT: C++ leaves `amount` uninitialised (garbage) when completionC has no number; 0 is used.
        let amount = scan_int(&m.completionc).unwrap_or(0);

        Person::establish_connection(g, name, &sourceip);

        BankAccount::get_account(g.world(), &sourceip, &sourceaccno)
            .expect("change account: no account");
        BankAccount::transfer_money(
            g,
            &sourceip,
            &sourceaccno,
            &targetip,
            &targetaccno,
            amount,
            name,
        );

        ConsequenceGenerator::mission_completed(g, &m, name);
    }

    pub fn attempt_mission_trace_user(g: &mut Ctx, name: &str) {
        //
        // [ DEEP BREATH ]
        //

        let mut m = match first_mission(g, name) {
            Some(m) => m,
            None => return,
        };

        let hackername = m.completiona.clone();
        let hacker_localhost = g
            .world()
            .get_person(&hackername)
            .expect("trace user: no hacker")
            .localhost
            .clone();

        if !WAREZRELEASE && hackername == "PLAYER" {
            //
            // SECURITY CHECK
            // To detect if a dodgy warez kid has put one of our
            // patches over his hacked version to fix it
            // If so, his world.dat will have the realbuffer shown below
            //
            // PORT-TODO: the Rust port saves JSON rather than the C++ binary world.dat, so an
            // unreadable file is treated as "no match" (C++ asserted that the file opened).
            let worlddat = format!("{}/world.dat", g.app.path);
            let matched = std::fs::read(&worlddat)
                .map(|bytes| bytes.get(256..264) == Some(&WAREZ_REALBUFFER[..]))
                .unwrap_or(false);

            if matched {
                let mut rundate = Date::default();
                rundate.set_date_from(&g.world().date);
                rundate.advance_minute(TIME_TOWAREZGAMEOVER);

                let mut ne = NotificationEvent::new();
                ne.set_type(NOTIFICATIONEVENT_TYPE_WAREZGAMEOVER);
                ne.base.set_run_date(&rundate);

                g.world_mut()
                    .scheduler
                    .schedule_event(UplinkEvent::NotificationEvent(ne));
            }
        }

        //
        // Get the hacked computer system
        //

        let compip = m
            .links
            .first()
            .cloned()
            .expect("trace user: mission has no link");
        let compname = computer_name_at(g, &compip);

        //
        // Go through each log, try to trace back to the hacker
        //

        let urating = person_ref(g, name).rating.uplinkrating;

        let mut il: i32 = 0;
        while il < comp_ref(g, &compname).logbank.logs.size() {
            let mut al = match comp_ref(g, &compname).logbank.logs.get_data(il) {
                Some(al) => al.clone(),
                None => {
                    il += 1;
                    continue;
                }
            };

            // If the log was deleted/overwritten we may be able to recover it
            if al.type_ == LOG_TYPE_DELETED && urating >= MINREQUIREDRATING_UNDELETELOGLEVEL1 {
                if let Some(recovered) = comp_ref(g, &compname)
                    .logbank
                    .internallogs
                    .get_data(il)
                    .cloned()
                {
                    let mut internalcopy = AccessLog::new();
                    internalcopy.set_properties_from(&recovered);
                    comp_mut(g, &compname)
                        .logbank
                        .logs
                        .put_data_at(internalcopy.clone(), il);
                    al = internalcopy;
                }
            } else if al.type_ != LOG_TYPE_DELETED
                && comp_ref(g, &compname).logbank.log_modified(il)
                && urating >= MINREQUIREDRATING_UNDELETELOGLEVEL3
            {
                // Log was overwritten or modified
                if let Some(recovered) = comp_ref(g, &compname)
                    .logbank
                    .internallogs
                    .get_data(il)
                    .cloned()
                {
                    let mut internalcopy = AccessLog::new();
                    internalcopy.set_properties_from(&recovered);
                    comp_mut(g, &compname)
                        .logbank
                        .logs
                        .put_data_at(internalcopy.clone(), il);
                    al = internalcopy;
                }
            }

            // Now look at the log

            if al.suspicious != LOG_NOTSUSPICIOUS
                && al.fromname == hackername
                && al.type_ != LOG_TYPE_CONNECTIONOPENED
            {
                // This is by the hacker but can't be investigated further
                set_log_suspicious(g, &compname, il, LOG_NOTSUSPICIOUS);
            } else if al.suspicious != LOG_NOTSUSPICIOUS
                && al.type_ == LOG_TYPE_CONNECTIONOPENED
                && al.fromname == hackername
            {
                // Now investigated - so set it back to LOG_NOTSUSPICIOUS
                set_log_suspicious(g, &compname, il, LOG_NOTSUSPICIOUS);

                let fromcompname = computer_name_at(g, &al.fromip);
                let comp_ip = comp_ref(g, &compname).ip.clone();
                let fromcomp_ip = comp_ref(g, &fromcompname).ip.clone();

                let traced = LogBank::trace_log(g, &comp_ip, &fromcomp_ip, &al.date, urating);

                if let Some(traced_ip) = traced {
                    // Trace successful
                    // Does the guilty person live at this IP?

                    if hacker_localhost == traced_ip {
                        // Mission successful
                        ConsequenceGenerator::mission_completed(g, &m, name);
                        return;
                    }

                    // The trace has been completed but a different IP has been found
                    // This means the logs have been altered, probably to incriminate
                    // someone else

                    // Look up the unfortunate individual
                    if let Some(framed_name) = framed_person(g, &traced_ip) {
                        // strcpy ( m->completionA, framed->name ): the agent's mission 0 is the same object
                        m.completiona = framed_name.clone();
                        if let Some(first) = person_mut(g, name).agent_mut().missions.first_mut() {
                            first.completiona = framed_name;
                        }

                        ConsequenceGenerator::mission_completed(g, &m, name);
                        return;
                    }
                    // PORT: C++ only returns from here in VERBOSEAI builds, so the scan goes on.
                }
            }

            il += 1;
        }

        //
        // If we reach here then the hacker could not be traced
        // PORT: the VERBOSEAI diagnostic is not ported.
        //
    }

    pub fn attempt_mission_remove_computer(g: &mut Ctx, name: &str) {
        let m = match first_mission(g, name) {
            Some(m) => m,
            None => return,
        };

        let targetip = m.completiona.clone();

        let compname = computer_name_at(g, &targetip);

        Person::establish_connection(g, name, &targetip);

        comp_mut(g, &compname).set_is_running(false);

        ConsequenceGenerator::mission_completed(g, &m, name);
    }

    /// Bounces a connection to `ip` through random machines, covers the tracks (complexity
    /// based on skill) and disconnects.
    pub fn establish_connection(g: &mut Ctx, name: &str, ip: &str) {
        Connection::reset(g, name);

        //
        // Add in a few bounces
        //

        let urating = person_ref(g, name).rating.uplinkrating;
        let numbounces = 2 + NumberGenerator::random_number(urating / 3);

        let mut i = 0;
        while i < numbounces {
            let compname = WorldGenerator::get_random_computer_of_type(
                g,
                COMPUTER_TYPE_INTERNALSERVICESMACHINE
                    | COMPUTER_TYPE_CENTRALMAINFRAME
                    | COMPUTER_TYPE_PUBLICBANKSERVER,
            );
            let compip = comp_ref(g, &compname).ip.clone();

            if !person_ref(g, name)
                .get_connection()
                .location_included(&compip)
                && compip != ip
            {
                person_mut(g, name)
                    .get_connection_mut()
                    .add_vlocation(&compip);
            } else {
                // C++ `--i`: find a different machine for this bounce
                continue;
            }
            i += 1;
        }

        //
        // Add in the target and establish the connection
        //

        person_mut(g, name).get_connection_mut().add_vlocation(ip);
        Connection::connect(g, name);
        Connection::begin_trace(g, name); // To make it "suspect"

        //
        // Cover our tracks on 1 system (not the target)
        // The date on the logs should remain the same - so they don't get out of sync
        //

        let coverindex = NumberGenerator::random_number(numbounces - 1) + 1;
        let (fromip, toip) = {
            let vlocations = &person_ref(g, name).get_connection().vlocations;
            (
                vlocations
                    .get((coverindex - 1) as usize)
                    .cloned()
                    .expect("establish connection: no source location"),
                vlocations
                    .get(coverindex as usize)
                    .cloned()
                    .expect("establish connection: no cover location"),
            )
        };

        let tocomp = computer_name_at(g, &toip);

        let mut il: i32 = 0;
        while il < comp_ref(g, &tocomp).logbank.logs.size() {
            let log = comp_ref(g, &tocomp).logbank.logs.get_data(il).cloned();

            if let Some(log) = log {
                if log.fromname == name && log.fromip == fromip {
                    let logdate = log.date.clone();

                    if urating >= MINREQUIREDRATING_DELETELOGLEVEL3 {
                        // Copy an existing log over the top
                        comp_mut(g, &tocomp).logbank.logs.remove_data(il);
                        let mut overwritten = false;

                        // Look for a valid log to put in the blank place
                        let mut is: i32 = 0;
                        while is < comp_ref(g, &tocomp).logbank.logs.size() {
                            let copyme = comp_ref(g, &tocomp).logbank.logs.get_data(is).cloned();
                            if let Some(copyme) = copyme {
                                if copyme.fromip != fromip {
                                    // This log was made by someone else
                                    let mut al = AccessLog::new();
                                    al.set_properties_from(&copyme);
                                    al.date.set_date_from(&logdate);
                                    al.set_suspicious(LOG_NOTSUSPICIOUS);
                                    comp_mut(g, &tocomp).logbank.logs.put_data_at(al, il);
                                    overwritten = true;
                                    break;
                                }
                            }
                            is += 1;
                        }

                        if !overwritten {
                            // Could not find log to overwrite with -
                            // So make up a new one
                            let randomip = WorldGenerator::get_random_location(g);
                            let mut al = AccessLog::new();
                            al.set_properties(
                                &logdate,
                                &randomip,
                                " ",
                                LOG_NOTSUSPICIOUS,
                                LOG_TYPE_TEXT,
                            );
                            al.set_data1(Some("Accessed File"));
                            comp_mut(g, &tocomp).logbank.logs.put_data_at(al, il);
                        }
                    } else if urating >= MINREQUIREDRATING_DELETELOGLEVEL2 {
                        // Blank the log
                        let mut al = AccessLog::new();
                        al.set_properties(
                            &logdate,
                            "Unknown",
                            name,
                            LOG_NOTSUSPICIOUS,
                            LOG_TYPE_DELETED,
                        );
                        comp_mut(g, &tocomp).logbank.logs.put_data_at(al, il);
                    } else if urating >= MINREQUIREDRATING_DELETELOGLEVEL1 {
                        // Delete the log, leave marker
                        let mut al = AccessLog::new();
                        al.set_properties(
                            &logdate,
                            "Unknown",
                            name,
                            LOG_NOTSUSPICIOUS,
                            LOG_TYPE_DELETED,
                        );
                        comp_mut(g, &tocomp).logbank.logs.put_data_at(al, il);
                    }
                }
            }

            il += 1;
        }

        //
        // Disconnect me
        //

        Connection::disconnect(g, name);
        Connection::reset(g, name);
    }

    /// `Agent::Update` (only calls `Person::update`).
    pub fn update_agent(g: &mut Ctx, name: &str) {
        Person::update(g, name);
    }
}

// ---- helpers (private) ----

/// The Person called `name` (the player included).
fn person_ref<'a>(g: &'a Ctx, name: &str) -> &'a Person {
    g.world().get_person(name).expect("agent: no such person")
}

fn person_mut<'a>(g: &'a mut Ctx, name: &str) -> &'a mut Person {
    g.world_mut()
        .get_person_mut(name)
        .expect("agent: no such person")
}

fn comp_ref<'a>(g: &'a Ctx, name: &str) -> &'a Computer {
    g.world()
        .get_computer(name)
        .expect("agent: no such computer")
}

fn comp_mut<'a>(g: &'a mut Ctx, name: &str) -> &'a mut Computer {
    g.world_mut()
        .get_computer_mut(name)
        .expect("agent: no such computer")
}

/// Copy of mission 0 of the agent `name`, if it has one.
fn first_mission(g: &Ctx, name: &str) -> Option<Mission> {
    person_ref(g, name).agent().missions.first().cloned()
}

/// The name of the computer at `ip` (C++ `GetVLocation(ip)->GetComputer()`, both asserted).
fn computer_name_at(g: &Ctx, ip: &str) -> String {
    let vl = g
        .world()
        .get_vlocation(ip)
        .expect("agent: no VLocation at ip");
    g.world()
        .get_computer(&vl.computer)
        .expect("agent: no computer at ip");
    vl.computer.clone()
}

/// `comp->logbank.logs[il]->SetSuspicious(value)` and the same on `internallogs[il]`, if valid.
fn set_log_suspicious(g: &mut Ctx, compname: &str, il: i32, value: i32) {
    let logbank = &mut comp_mut(g, compname).logbank;
    if let Some(al) = logbank.logs.get_data_mut(il) {
        al.set_suspicious(value);
    }
    if let Some(al) = logbank.internallogs.get_data_mut(il) {
        al.set_suspicious(value);
    }
}

/// The first person, in the pre-order of the C++ `people` BTree (player included), who lives
/// at `ip`; its name.
fn framed_person(g: &Ctx, ip: &str) -> Option<String> {
    g.world()
        .all_people()
        .into_iter()
        .find(|(_, person)| person.localhost == ip)
        .map(|(_, person)| person.name.clone())
}

/// The slot of the remote interface's current screen (for `ScreenSlot::call`).
fn remote_screen_slot(g: &mut Ctx) -> &mut ScreenSlot {
    &mut g.interface_mut().ri.screen
}

/// `sscanf(text, "%d", &x)`: leading whitespace, an optional sign, then digits. None if no number.
fn scan_int(text: &str) -> Option<i32> {
    let t = text.trim_start();
    let end = t
        .char_indices()
        .find(|&(i, c)| !(c.is_ascii_digit() || (i == 0 && (c == '-' || c == '+'))))
        .map_or(t.len(), |(i, _)| i);
    t[..end].parse::<i32>().ok()
}

/// `sscanf(text, "%s %s", a, b)`: the first two whitespace-separated words ("" if missing).
fn scan_two_words(text: &str) -> (String, String) {
    let mut words = text.split_whitespace();
    (
        words.next().unwrap_or("").to_string(),
        words.next().unwrap_or("").to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn give_code_keeps_duplicate_ips_like_the_btree() {
        let mut p = Person::new_agent();
        p.give_code("1.1.1.1", "user 'bob' pass 'x'");
        p.give_code("1.1.1.1", "user 'bob' pass 'y'"); // same username: replaced
        p.give_code("1.1.1.1", "user 'ann' pass 'z'"); // other username: a second entry
        p.give_code("1.1.1.1", "user 'ann' pass 'z'"); // already known
        let codes: Vec<(&str, &String)> = p.agent().codes.iter().collect();
        assert_eq!(codes.len(), 2);
        assert_eq!(codes[0].1, "user 'bob' pass 'y'");
        assert_eq!(codes[1].1, "user 'ann' pass 'z'");
        assert_eq!(p.agent().codes.get("1.1.1.1").unwrap(), "user 'bob' pass 'y'");
    }

    fn parse(code: &str) -> Option<(String, String)> {
        Agent::parse_access_code(code)
    }

    #[test]
    fn access_code_with_one_quoted_word_is_name_and_password() {
        assert_eq!(parse("Password is 'rosebud'"), Some(("rosebud".into(), "rosebud".into())));
        assert_eq!(parse("'x'"), Some(("x".into(), "x".into())));
    }

    #[test]
    fn access_code_with_two_quoted_words_is_name_then_password() {
        assert_eq!(
            parse("Username 'admin' and password 'swordfish' for the server"),
            Some(("admin".into(), "swordfish".into()))
        );
    }

    #[test]
    fn access_code_with_any_other_number_of_quotes_is_rejected() {
        assert_eq!(parse(""), None);
        assert_eq!(parse("no quotes at all"), None);
        assert_eq!(parse("one ' quote"), None);
        assert_eq!(parse("'a' 'b' 'c'"), None);
        assert_eq!(parse("'a' 'b' 'c' 'd' 'e'"), None);
    }

    #[test]
    fn scanf_helpers() {
        assert_eq!(scan_int("42"), Some(42));
        assert_eq!(scan_int("  -17 credits"), Some(-17));
        assert_eq!(scan_int("+5x"), Some(5));
        assert_eq!(scan_int("abc"), None);
        assert_eq!(scan_int(""), None);
        assert_eq!(scan_two_words("  first   second third"), ("first".into(), "second".into()));
        assert_eq!(scan_two_words("only"), ("only".into(), String::new()));
        assert_eq!(scan_two_words(""), (String::new(), String::new()));
    }

    #[test]
    fn links_are_remembered_by_ip() {
        let mut p = Person::new_agent();
        p.agent_mut().links.push("234.2.2.2".to_string());
        assert!(p.has_link("234.2.2.2"));
        assert!(!p.has_link("1.1.1.1"));
        p.remove_link("1.1.1.1");
        assert!(p.has_link("234.2.2.2"));
        p.remove_link("234.2.2.2");
        assert!(!p.has_link("234.2.2.2"));
    }
}
