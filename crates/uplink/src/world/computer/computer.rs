//! Port of source/world/computer/computer.h + source/world/computer/computer.cpp
//!
//! A computer in the Uplink world. It belongs to one company and is located at a virtual location.
//! PORT (§3.3): BankComputer and LanComputer are not subclasses; they are the variants of
//! [`ComputerKind`] (structs in bankcomputer.rs / lancomputer.rs). `BankComputer::Update` and
//! `LanComputer::Update` only call `Computer::Update`, so `Computer::update` serves every kind.
//! Methods that read or change the world take `g: &mut Ctx` plus the computer's `name` (its key).

use serde::{Deserialize, Serialize};

use crate::ctx::Ctx;
use crate::game::data::data::{
    IP_ACADEMICDATABASE, IP_GLOBALCRIMINALDATABASE, IP_SOCIALSECURITYDATABASE,
    TIME_REVELATIONREPRODUCE, TIME_TOEXPIRELOGS,
};
use crate::interface::remoteinterface::remoteinterface::RemoteInterface;
use crate::tosser::DArray;
use crate::world::computer::bankcomputer::BankComputer;
use crate::world::computer::computerscreen::computerscreen::ComputerScreen;
use crate::world::computer::databank::{Data, DataBank, DATATYPE_DATA, SOFTWARETYPE_NONE};
use crate::world::computer::lancomputer::LanComputer;
use crate::world::computer::logbank::{
    AccessLog, LogBank, LOG_NOTSUSPICIOUS, LOG_SUSPICIOUS, LOG_SUSPICIOUSANDNOTICED,
    LOG_TYPE_TEXT, LOG_UNDERINVESTIGATION,
};
use crate::world::computer::recordbank::{
    RecordBank, RECORDBANK_ADMIN, RECORDBANK_PASSWORD, RECORDBANK_READONLY, RECORDBANK_READWRITE,
};
use crate::world::computer::security::Security;
use crate::world::connection::Connection;
use crate::world::date::Date;
use crate::world::generator::consequencegenerator::ConsequenceGenerator;
use crate::world::generator::missiongenerator::MISSION_TRACEUSER;
use crate::world::generator::namegenerator::NameGenerator;
use crate::world::generator::newsgenerator::NewsGenerator;
use crate::world::generator::numbergenerator::NumberGenerator;
use crate::world::generator::plotgenerator::PlotGenerator;
use crate::world::generator::worldgenerator::WorldGenerator;

// Computer types

pub const COMPUTER_TYPE_UNKNOWN: i32 = 0;
pub const COMPUTER_TYPE_PUBLICACCESSSERVER: i32 = 1;
pub const COMPUTER_TYPE_INTERNALSERVICESMACHINE: i32 = 2;
pub const COMPUTER_TYPE_CENTRALMAINFRAME: i32 = 4;
pub const COMPUTER_TYPE_PUBLICBANKSERVER: i32 = 8;
pub const COMPUTER_TYPE_PERSONALCOMPUTER: i32 = 16;
pub const COMPUTER_TYPE_VOICEPHONESYSTEM: i32 = 32;
pub const COMPUTER_TYPE_LAN: i32 = 64;

pub const COMPUTER_TYPE_ANY: i32 = COMPUTER_TYPE_PUBLICACCESSSERVER
    | COMPUTER_TYPE_INTERNALSERVICESMACHINE
    | COMPUTER_TYPE_CENTRALMAINFRAME
    | COMPUTER_TYPE_PUBLICBANKSERVER
    | COMPUTER_TYPE_PERSONALCOMPUTER
    | COMPUTER_TYPE_VOICEPHONESYSTEM
    | COMPUTER_TYPE_LAN;

// Trace actions

pub const COMPUTER_TRACEACTION_NONE: i32 = 0;
pub const COMPUTER_TRACEACTION_DISCONNECT: i32 = 1;
pub const COMPUTER_TRACEACTION_WARNINGMAIL: i32 = 2;
pub const COMPUTER_TRACEACTION_FINE: i32 = 4;
pub const COMPUTER_TRACEACTION_LEGAL: i32 = 8;
pub const COMPUTER_TRACEACTION_TACTICAL: i32 = 16;
pub const COMPUTER_TRACEACTION_COVERT: i32 = 32;

/// Replaces the Computer subclasses (§3.3).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum ComputerKind {
    #[default]
    Plain,
    BankComputer(BankComputer),
    LanComputer(LanComputer),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Computer {
    pub numrecenthacks: i32,
    pub numhacksthismonth: i32,
    pub numhackslastmonth: i32,

    /// COMPUTER_TYPE_*  (C++ `TYPE`)
    pub type_: i32,

    pub name: String,
    pub companyname: String,
    pub ip: String,

    pub tracespeed: i32,
    /// COMPUTER_TRACEACTION_*
    pub traceaction: i32,

    /// Can I be targetted by random events
    pub istargetable: bool,
    /// Can someone connect to me from outside my trusted network
    pub isexternallyopen: bool,
    pub isrunning: bool,
    /// Version number
    pub isinfected_revelation: f32,
    pub infectiondate: Date,

    pub screens: DArray<ComputerScreen>,

    pub databank: DataBank,
    pub logbank: LogBank,
    pub recordbank: RecordBank,
    pub security: Security,

    pub kind: ComputerKind,
}

impl Default for Computer {
    fn default() -> Self {
        Computer {
            numrecenthacks: 0,
            numhacksthismonth: 0,
            numhackslastmonth: 0,
            type_: COMPUTER_TYPE_UNKNOWN,
            name: String::new(),
            companyname: String::new(),
            ip: String::new(),
            tracespeed: 0,
            traceaction: COMPUTER_TRACEACTION_NONE,
            istargetable: true,
            isexternallyopen: true,
            isrunning: true,
            isinfected_revelation: 0.0,
            infectiondate: Date::default(),
            screens: DArray::new(),
            databank: DataBank::new(),
            logbank: LogBank::new(),
            recordbank: RecordBank::new(),
            security: Security::new(),
            kind: ComputerKind::Plain,
        }
    }
}

impl Computer {
    pub fn new() -> Self {
        Self::default()
    }

    // Subclass helpers (§3.3)

    pub fn bank(&self) -> Option<&BankComputer> {
        match &self.kind {
            ComputerKind::BankComputer(b) => Some(b),
            _ => None,
        }
    }

    pub fn bank_mut(&mut self) -> Option<&mut BankComputer> {
        match &mut self.kind {
            ComputerKind::BankComputer(b) => Some(b),
            _ => None,
        }
    }

    pub fn lan(&self) -> Option<&LanComputer> {
        match &self.kind {
            ComputerKind::LanComputer(l) => Some(l),
            _ => None,
        }
    }

    pub fn lan_mut(&mut self) -> Option<&mut LanComputer> {
        match &mut self.kind {
            ComputerKind::LanComputer(l) => Some(l),
            _ => None,
        }
    }

    // Setters

    pub fn set_type(&mut self, new_type: i32) {
        self.type_ = new_type;
    }

    pub fn set_name(&mut self, newname: &str) {
        self.name = newname.to_string();
    }

    /// PORT: the C++ asserted that the company exists in the world; not checked here.
    pub fn set_company_name(&mut self, newname: &str) {
        self.companyname = newname.to_string();
    }

    /// PORT: the C++ asserted that a VLocation with this ip exists; not checked here.
    pub fn set_ip(&mut self, newip: &str) {
        self.ip = newip.to_string();
    }

    pub fn set_trace_speed(&mut self, newtracespeed: i32) {
        self.tracespeed = newtracespeed;
    }

    pub fn set_trace_action(&mut self, newtraceaction: i32) {
        self.traceaction = newtraceaction;
    }

    pub fn set_is_targetable(&mut self, value: bool) {
        self.istargetable = value;
    }

    pub fn set_is_externally_open(&mut self, value: bool) {
        self.isexternallyopen = value;
    }

    pub fn set_is_running(&mut self, value: bool) {
        self.isrunning = value;
    }

    // Revelation virus (touch the plot generator / world date)

    pub fn disinfect_revelation(g: &mut Ctx, computer_name: &str) {
        let (ip, infected) = {
            let c = computer_at(g, computer_name);
            (c.ip.clone(), c.isinfected_revelation)
        };
        if infected > 0.0 {
            PlotGenerator::disinfected(g, &ip);
        }

        computer_at_mut(g, computer_name).isinfected_revelation = 0.0;
    }

    pub fn infect_with_revelation(g: &mut Ctx, computer_name: &str, version: f32) {
        let now = g.world().date.clone();
        let c = computer_at_mut(g, computer_name);

        c.isinfected_revelation = version;
        c.infectiondate.set_date_from(&now);

        if version <= 1.0 {
            // Destroy the computer immediately
            c.set_is_running(false);
            c.databank.format();
            c.logbank.empty();
        }
    }

    // Screens

    /// Tells the screen which computer it belongs to, then stores it. `index == -1` uses the next free
    /// slot. Returns the index used.
    pub fn add_computer_screen(&mut self, mut cs: ComputerScreen, index: i32) -> i32 {
        cs.base_mut().computer = self.name.clone();
        if index == -1 {
            return self.screens.put_data(cs);
        }
        if index >= self.screens.size() {
            self.screens.set_size(index + 1);
        }
        self.screens.put_data_at(cs, index);
        index
    }

    pub fn get_computer_screen(&self, index: i32) -> Option<&ComputerScreen> {
        self.screens.get_data(index)
    }

    pub fn get_computer_screen_mut(&mut self, index: i32) -> Option<&mut ComputerScreen> {
        self.screens.get_data_mut(index)
    }

    // Maintenance

    /// Call me frequently. Notices suspicious logs, reports destroyed computers, resets defences.
    pub fn check_for_security_breaches(g: &mut Ctx, computer_name: &str) {
        let mut timetochangeunderware = false;

        //
        // Has someone damaged this computer?
        //

        let (isrunning, infected, formatted) = {
            let c = computer_at(g, computer_name);
            (c.isrunning, c.isinfected_revelation, c.databank.formatted)
        };

        if !isrunning {
            if infected == 0.0 {
                NewsGenerator::computer_destroyed(g, computer_name, formatted);
            }

            computer_at_mut(g, computer_name).set_is_running(true);
            Self::disinfect_revelation(g, computer_name);
            computer_at_mut(g, computer_name).databank.formatted = false;

            return;
        }

        //
        // Check for suspect logs
        //

        // The C++ re-reads `logs.Size()` on every pass, so do the same
        let mut next = 0;
        while next < computer_at(g, computer_name).logbank.logs.size() {
            let i = next;
            next += 1;
            let (suspicious, fromname, has_internal) = {
                let c = computer_at(g, computer_name);
                match c.logbank.logs.get_data(i) {
                    Some(al) => (
                        al.suspicious,
                        al.fromname.clone(),
                        c.logbank.internallogs.valid_index(i),
                    ),
                    None => continue,
                }
            };

            if suspicious == LOG_SUSPICIOUS {
                // Now noticed - they have another 3 hours before trace

                // Special case - don't trace if this is the player, and he is performing
                // a Trace Hacker mission for this company
                let mut ignoreme = false;

                if fromname == "PLAYER" {
                    let ip = computer_at(g, computer_name).ip.clone();
                    ignoreme = g
                        .player()
                        .agent()
                        .missions
                        .iter()
                        .any(|m| m.type_ == MISSION_TRACEUSER && m.completionb == ip);
                }

                if !ignoreme {
                    set_suspicious_at(
                        computer_at_mut(g, computer_name),
                        i,
                        LOG_SUSPICIOUSANDNOTICED,
                        has_internal,
                    );
                    timetochangeunderware = true;
                } else {
                    set_suspicious_at(computer_at_mut(g, computer_name), i, LOG_NOTSUSPICIOUS, false);
                }
            } else if suspicious == LOG_SUSPICIOUSANDNOTICED {
                // Noticed and time up - pass this onto the consequence generator
                let al = computer_at(g, computer_name).logbank.logs.get_data(i).cloned();
                if let Some(al) = al {
                    ConsequenceGenerator::computer_hacked(g, computer_name, &al);
                }

                // No longer suspicious - now under investigation
                set_suspicious_at(
                    computer_at_mut(g, computer_name),
                    i,
                    LOG_UNDERINVESTIGATION,
                    has_internal,
                );
            }
        }

        //
        // Have all our files been wiped?
        //

        if computer_at(g, computer_name).databank.formatted {
            let softwaretype = ["SCIENTIFIC", "CORPORATE", "CUSTOMER", "SOFTWARE"];
            let softwareindex = NumberGenerator::random_number(4);

            NewsGenerator::all_files_deleted(g, computer_name, softwaretype[softwareindex as usize]);
            computer_at_mut(g, computer_name).databank.formatted = false;
            timetochangeunderware = true;
        }

        //
        // Reset our defenses if they have been compromised
        //

        if computer_at(g, computer_name).security.is_anything_disabled() || timetochangeunderware {
            Self::change_security_codes(g, computer_name);
        }
    }

    /// Call me frequently. Deletes logs that are too old and packs the log arrays.
    pub fn manage_old_logs(g: &mut Ctx, computer_name: &str) {
        let now = g.world().date.clone();
        let c = computer_at_mut(g, computer_name);

        //
        // Delete any logs older than a certain age
        //

        for i in (0..c.logbank.logs.size()).rev() {
            let expired = match c.logbank.logs.get_data(i) {
                None => false,
                Some(al) => {
                    let mut testdate = Date::default();
                    testdate.set_date_from(&al.date);
                    testdate.advance_minute(TIME_TOEXPIRELOGS);
                    testdate.before(&now)
                }
            };

            if expired {
                c.logbank.logs.remove_data(i);
                c.logbank.internallogs.remove_data(i);
            }
        }

        //
        // Pack the new log structures together
        //

        let mut nextlog: i32 = 0;

        for il in 0..c.logbank.logs.size() {
            if il > nextlog {
                nextlog = il;
            }

            if !c.logbank.logs.valid_index(il) {
                // This is a blank spot
                // Look for the next available log to fill this space

                let mut found = false;

                while nextlog < c.logbank.logs.size() {
                    if c.logbank.logs.valid_index(nextlog) {
                        move_slot(&mut c.logbank.logs, nextlog, il);
                        move_slot(&mut c.logbank.internallogs, nextlog, il);
                        nextlog += 1;

                        found = true;
                        break;
                    }
                    nextlog += 1;
                }

                if !found {
                    // There are no more valid logs -
                    // So resize the DArray since this is now the max size
                    // Then quit

                    c.logbank.logs.set_size(il);
                    c.logbank.internallogs.set_size(il);
                    break;
                }
            }
        }
    }

    /// Changes passwords, re-enables all security. Returns true if passwords were changed.
    /// PORT: takes `g` because the new passwords come from the name generator (needs the world).
    pub fn change_security_codes(g: &mut Ctx, computer_name: &str) -> bool {
        // Change our admin password

        let mut changed = false;
        let (type_, ip) = {
            let c = computer_at(g, computer_name);
            (c.type_, c.ip.clone())
        };

        match type_ {
            COMPUTER_TYPE_INTERNALSERVICESMACHINE | COMPUTER_TYPE_CENTRALMAINFRAME => {
                change_password(g, computer_name, RECORDBANK_ADMIN);
                changed = true;
            }

            COMPUTER_TYPE_UNKNOWN => {
                if ip == IP_SOCIALSECURITYDATABASE
                    || ip == IP_GLOBALCRIMINALDATABASE
                    || ip == IP_ACADEMICDATABASE
                {
                    change_password(g, computer_name, RECORDBANK_ADMIN);
                    change_password(g, computer_name, RECORDBANK_READWRITE);
                    change_password(g, computer_name, RECORDBANK_READONLY);

                    changed = true;
                }
            }

            _ => {}
        }

        // Re-enable all our security

        let num_systems = computer_at(g, computer_name).security.num_systems();
        for i in 0..num_systems {
            if let Some(system) = computer_at_mut(g, computer_name).security.get_system_mut(i) {
                system.enable();
            }
        }

        changed
    }

    /// Call when this is hacked
    pub fn add_to_recent_hacks(&mut self, n: i32) {
        self.numrecenthacks += n;
        self.numhacksthismonth += n;
    }

    /// Call me 4 times a month
    pub fn update_recent_hacks(g: &mut Ctx, computer_name: &str) {
        // This is called 4 times a month
        // Reduce the number of recent hacks by the number last month / 4
        // (removing all of last months hacks from the count by the end of the month)

        // If it is the end of the month, process the changeover to a new month
        // Also schedule the next check

        let day = g.world().date.get_day();
        let c = computer_at_mut(g, computer_name);

        if day == 1 {
            // New month

            c.numhackslastmonth = c.numhacksthismonth;
            c.numhacksthismonth = 0;
        } else {
            // Reduce hacks this month by a quarter

            let numtoremove = c.numhackslastmonth / 4;
            c.numrecenthacks -= numtoremove;
        }

        // Do some clamping, just to be safe

        if c.numrecenthacks < 0 {
            c.numrecenthacks = 0;
        }
        if c.numhackslastmonth < 0 {
            c.numhackslastmonth = 0;
        }
    }

    pub fn num_recent_hacks(&self) -> i32 {
        self.numrecenthacks
    }

    /// `Computer::Update` (also the Update of BankComputer / LanComputer): revelation spreading,
    /// random new files and logs.
    pub fn update(g: &mut Ctx, computer_name: &str) {
        if !computer_at(g, computer_name).isrunning {
            return;
        }

        if computer_at(g, computer_name).isinfected_revelation > 1.0 {
            let mut damagedate = Date::default();
            damagedate.set_date_from(&computer_at(g, computer_name).infectiondate);
            damagedate.advance_minute(TIME_REVELATIONREPRODUCE);

            if g.world().date.after(&damagedate) {
                //
                // Spread to two other computers

                let comp1 = WorldGenerator::get_random_computer_of_type(
                    g,
                    COMPUTER_TYPE_INTERNALSERVICESMACHINE,
                );
                let comp2 = WorldGenerator::get_random_computer_of_type(
                    g,
                    COMPUTER_TYPE_INTERNALSERVICESMACHINE | COMPUTER_TYPE_CENTRALMAINFRAME,
                );

                let comp1_ip = computer_at(g, &comp1).ip.clone();
                let comp2_ip = computer_at(g, &comp2).ip.clone();

                let version = computer_at(g, computer_name).isinfected_revelation;
                PlotGenerator::run_revelation(g, &comp1_ip, version, false);
                let version = computer_at(g, computer_name).isinfected_revelation;
                PlotGenerator::run_revelation(g, &comp2_ip, version, false);

                //
                // Shut down this computer

                let ip = {
                    let c = computer_at_mut(g, computer_name);
                    c.set_is_running(false);
                    c.databank.format();
                    c.logbank.empty();
                    c.ip.clone()
                };
                PlotGenerator::disinfected(g, &ip);

                //
                // Disconnect the player if he is connected

                let player_is_here = g
                    .player()
                    .get_remote_host(g.world())
                    .is_some_and(|host| host.ip == ip);

                if player_is_here {
                    Connection::disconnect(g, "PLAYER");
                    Connection::reset(g, "PLAYER");

                    RemoteInterface::run_new_location(g);
                    RemoteInterface::run_screen(g, 2);
                }
            }
        }

        //
        // Generate some new files
        //

        if computer_at(g, computer_name).databank.num_data_files() > 0
            && NumberGenerator::random_number(1000) == 0
        {
            let mut data = Data::new();
            data.set_title(&NameGenerator::generate_data_name("companyname", DATATYPE_DATA));
            data.set_details(
                DATATYPE_DATA,
                NumberGenerator::random_number(10) + 1,
                0,
                0,
                1.0,
                SOFTWARETYPE_NONE,
            );
            computer_at_mut(g, computer_name).databank.put_data(data);
        }

        //
        // Generate some new logs
        //

        if NumberGenerator::random_number(1000) == 0 {
            let mut al = AccessLog::new();
            let now = g.world().date.clone();
            let fromip = WorldGenerator::get_random_location(g);
            al.set_properties(&now, &fromip, " ", LOG_NOTSUSPICIOUS, LOG_TYPE_TEXT);
            al.set_data1(Some("Accessed File"));
            computer_at_mut(g, computer_name).logbank.add_log(al, -1);
        }
    }

    // Access codes: `GenerateAccessCode(code, result)` and its two overloads return the string.

    pub fn generate_access_code(code: &str) -> String {
        format!("CODE:'{}'", code)
    }

    /// `GenerateAccessCode(name, code, result)`
    pub fn generate_access_code_name(name: &str, code: &str) -> String {
        format!("NAME:'{}', CODE:'{}'", name, code)
    }

    /// `GenerateAccessCode(accNo, code, result)`
    pub fn generate_access_code_accno(acc_no: i32, code: &str) -> String {
        format!("NAME:'{}', CODE:'{}'", acc_no, code)
    }
}

// Private helpers. The C++ held raw pointers to computers, logs and records; here a computer is
// looked up again by its key (`name`) whenever the world is needed.

fn computer_at<'a>(g: &'a Ctx, name: &str) -> &'a Computer {
    g.world().get_computer(name).expect("computer not in world")
}

fn computer_at_mut<'a>(g: &'a mut Ctx, name: &str) -> &'a mut Computer {
    g.world_mut().get_computer_mut(name).expect("computer not in world")
}

/// `record->ChangeField(RECORDBANK_PASSWORD, GenerateComplexPassword())` for the record called `record`.
fn change_password(g: &mut Ctx, computer_name: &str, record: &str) {
    let password = NameGenerator::generate_complex_password(g);
    computer_at_mut(g, computer_name)
        .recordbank
        .get_record_from_name_mut(record)
        .expect("record exists")
        .change_field(RECORDBANK_PASSWORD, &password);
}

/// Sets the suspicion level of log `index`, and of its internal copy too when `also_internal`.
fn set_suspicious_at(c: &mut Computer, index: i32, value: i32, also_internal: bool) {
    if let Some(al) = c.logbank.logs.get_data_mut(index) {
        al.set_suspicious(value);
    }
    if also_internal {
        if let Some(il) = c.logbank.internallogs.get_data_mut(index) {
            il.set_suspicious(value);
        }
    }
}

/// C++ `PutData(GetData(from), to)` followed by `RemoveData(from)`: moves the slot's contents.
fn move_slot<T>(arr: &mut DArray<T>, from: i32, to: i32) {
    if let Some(item) = arr.remove_data(from) {
        arr.put_data_at(item, to);
    }
}
