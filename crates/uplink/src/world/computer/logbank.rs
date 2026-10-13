//! Port of source/world/computer/logbank.h + source/world/computer/logbank.cpp
//!
//! A set of access logs for a particular computer.

use serde::{Deserialize, Serialize};

use crate::ctx::Ctx;
use crate::game::data::data::{
    MINREQUIREDRATING_HACKBANKSERVER, MINREQUIREDRATING_HACKGOVERNMENTCOMPUTER,
    MINREQUIREDRATING_UNDELETELOGLEVEL1, MINREQUIREDRATING_UNDELETELOGLEVEL3,
};
use crate::tosser::DArray;
use crate::world::company::company::COMPANYTYPE_GOVERNMENT;
use crate::world::computer::computer::{COMPUTER_TYPE_PERSONALCOMPUTER, COMPUTER_TYPE_PUBLICBANKSERVER};
use crate::world::date::Date;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LogBank {
    pub logs: DArray<AccessLog>,
    /// Never delete from here
    pub internallogs: DArray<AccessLog>,
}

impl LogBank {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds to both `logs` and `internallogs`; `index == -1` appends.
    pub fn add_log(&mut self, log: AccessLog, index: i32) {
        let index = if index == -1 { self.logs.size() } else { index };
        if index >= self.logs.size() {
            self.logs.set_size(index + 1);
        }
        let mut internalcopy = AccessLog::new();
        internalcopy.set_properties_from(&log);
        self.logs.put_data_at(log, index);
        self.internallogs.set_size(self.logs.size());
        self.internallogs.put_data_at(internalcopy, index);
    }

    /// Is the log in internallogs different to that in logs?
    pub fn log_modified(&self, index: i32) -> bool {
        // C++: if either slot is empty, the log is not modified
        let (Some(log), Some(internallog)) = (self.logs.get_data(index), self.internallogs.get_data(index))
        else {
            return false;
        };

        // Do a comparison
        log.type_ != internallog.type_
            || log.suspicious != internallog.suspicious
            || log.fromip != internallog.fromip
            || log.fromname != internallog.fromname
            || !log.date.equal(&internallog.date)
    }

    /// ie source->logbank_ip->to_ip; lookup source and return (recursive).
    /// `self` is the logbank of the computer at `logbank_ip` (looked up through the world, because the
    /// trace also recovers deleted/overwritten logs in it). Returns the ip where the trace ends, or None.
    pub fn trace_log(
        g: &mut Ctx,
        to_ip: &str,
        logbank_ip: &str,
        date: &Date,
        uplinkrating: i32,
    ) -> Option<String> {
        // C++ `this` is the logbank of the computer at `logbank_ip`: resolve it to a computer name.
        let this_name = {
            let world = g.world();
            let vl_local = world
                .get_vlocation(logbank_ip)
                .expect("LogBank::TraceLog: no VLocation for local ip");
            vl_local
                .get_computer(world)
                .expect("LogBank::TraceLog: no Computer at local ip")
                .name
                .clone()
        };
        trace_log_at(g, to_ip, &this_name, logbank_ip, date, uplinkrating)
    }

    pub fn empty(&mut self) {
        self.logs.empty();
        self.internallogs.empty();
    }

    /// Does nothing in the C++ either.
    pub fn update(&mut self) {}
}

// ============================================================================

/// `SIZE_VLOCATION_IP`: size of the C++ ip buffers (strings must be shorter).
const SIZE_VLOCATION_IP: usize = 24;

pub const LOG_TYPE_DELETED: i32 = -1; // Undeleted version may exist in internallogs
pub const LOG_TYPE_NONE: i32 = 0;
pub const LOG_TYPE_TEXT: i32 = 1; // Caption included in 'data1'
pub const LOG_TYPE_CONNECTIONOPENED: i32 = 2; // Connection received from 'fromip'
pub const LOG_TYPE_CONNECTIONCLOSED: i32 = 3; // Connection closed from 'fromip'
pub const LOG_TYPE_BOUNCEBEGIN: i32 = 4; // Start point for a bounced call to ip 'data1'
pub const LOG_TYPE_BOUNCE: i32 = 5; // User bounces to ip 'data1'
pub const LOG_TYPE_TRANSFERTO: i32 = 6; // Transfer money to 'data1' [IP ACCNO], amount 'data2', person 'data3'
pub const LOG_TYPE_TRANSFERFROM: i32 = 7; // Transfer from 'data1' [IP ACCNO], amount 'data2', person 'data3'

// Most logs are not suspicious. If a suspicious activity occurs, it becomes LOG_SUSPICIOUS.
// Every [few] hours, these logs are changed to SUSPICIOUSANDNOTICED. [A few] hours later,
// they become UNDERINVESTIGATION and are dealt with. They are then changed back to
// NOTSUSPICIOUS by the agent.

pub const LOG_NOTSUSPICIOUS: i32 = 0;
pub const LOG_SUSPICIOUS: i32 = 1;
pub const LOG_SUSPICIOUSANDNOTICED: i32 = 2;
pub const LOG_UNDERINVESTIGATION: i32 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AccessLog {
    pub type_: i32,
    /// Time of access
    pub date: Date,
    /// The IP the access came from
    pub fromip: String,
    /// The person who created the log
    pub fromname: String,
    /// Was this a suspicious action? (C++ `SUSPICIOUS`, one of LOG_NOTSUSPICIOUS..)
    pub suspicious: i32,
    /// Misc data, None = NULL
    pub data1: Option<String>,
    pub data2: Option<String>,
    pub data3: Option<String>,
}

impl Default for AccessLog {
    fn default() -> Self {
        AccessLog {
            type_: LOG_TYPE_NONE,
            date: Date::default(),
            fromip: " ".to_string(),
            fromname: String::new(),
            suspicious: LOG_NOTSUSPICIOUS,
            data1: None,
            data2: None,
            data3: None,
        }
    }
}

impl AccessLog {
    pub fn new() -> Self {
        Self::default()
    }

    /// C++ defaults: newSUSPICIOUS = LOG_NOTSUSPICIOUS, newTYPE = LOG_TYPE_TEXT.
    pub fn set_properties(
        &mut self,
        newdate: &Date,
        newfromip: &str,
        newfromname: &str,
        new_suspicious: i32,
        new_type: i32,
    ) {
        // SetDate rolls over dodgy values and does not copy the update flags
        self.date.set_date_from(newdate);
        self.set_type(new_type);
        self.set_from_ip(newfromip);
        self.fromname = newfromname.to_string();
        self.set_suspicious(new_suspicious);
    }

    /// `SetProperties(AccessLog *copyme)`: copy everything (data1..3 included).
    pub fn set_properties_from(&mut self, copyme: &AccessLog) {
        self.set_properties(
            &copyme.date,
            &copyme.fromip,
            &copyme.fromname,
            copyme.suspicious,
            copyme.type_,
        );
        self.set_data1(copyme.data1.as_deref());
        self.set_data2(copyme.data2.as_deref());
        self.set_data3(copyme.data3.as_deref());
    }

    pub fn set_type(&mut self, new_type: i32) {
        self.type_ = new_type;
    }

    pub fn set_from_ip(&mut self, newfromip: &str) {
        assert!(newfromip.len() < SIZE_VLOCATION_IP, "AccessLog::SetFromIP: ip too long");
        self.fromip = newfromip.to_string();
    }

    pub fn set_suspicious(&mut self, new_suspicious: i32) {
        self.suspicious = new_suspicious;
    }

    pub fn set_data1(&mut self, newdata: Option<&str>) {
        self.data1 = newdata.map(|s| s.to_string());
    }

    pub fn set_data2(&mut self, newdata: Option<&str>) {
        self.data2 = newdata.map(|s| s.to_string());
    }

    pub fn set_data3(&mut self, newdata: Option<&str>) {
        self.data3 = newdata.map(|s| s.to_string());
    }

    pub fn get_description(&self) -> String {
        // It's worth remembering that all logs can now be edited by a gamer (LogModifier tool),
        // so data fields may be missing or malformed. UplinkAssert is active in every build of the
        // C++ (it exits), so the required fields are asserted here too.
        match self.type_ {
            LOG_TYPE_DELETED => "[Log Deleted]".to_string(),

            LOG_TYPE_CONNECTIONOPENED => format!("Connection established from {}", self.fromip),

            LOG_TYPE_CONNECTIONCLOSED => format!("Connection from {} closed", self.fromip),

            LOG_TYPE_BOUNCEBEGIN => {
                let data1 = self
                    .data1
                    .as_deref()
                    .expect("AccessLog::GetDescription: BOUNCEBEGIN without data1");
                format!("Connection opened to {}", data1)
            }

            LOG_TYPE_BOUNCE => {
                let data1 = self
                    .data1
                    .as_deref()
                    .expect("AccessLog::GetDescription: BOUNCE without data1");
                format!("Connection from {} routed to {}", self.fromip, data1)
            }

            LOG_TYPE_TRANSFERTO => match (self.data1.as_deref(), self.data2.as_deref()) {
                (Some(data1), Some(data2)) => {
                    let (ip, accno) = scan_two_words(data1);
                    format!("{}c transfered to IP {}, acc no. {}", data2, ip, accno)
                }
                _ => "Invalid TransferTO log".to_string(),
            },

            LOG_TYPE_TRANSFERFROM => match (self.data1.as_deref(), self.data2.as_deref()) {
                (Some(data1), Some(data2)) => {
                    let (ip, accno) = scan_two_words(data1);
                    format!("{}c deposited from IP {}, acc no. {}", data2, ip, accno)
                }
                _ => "Invalid TransferFROM log".to_string(),
            },

            _ => {
                let data1 = self
                    .data1
                    .as_deref()
                    .expect("AccessLog::GetDescription: default case without data1");
                format!("From {} : {}", self.fromip, data1)
            }
        }
    }
}

/// `sscanf(s, "%s %s", ip, accno)`: the first two whitespace-separated words.
// PORT: the C++ scanned into char[SIZE_VLOCATION_IP] / char[12] with no width limit (and left the
// buffers uninitialised when a word was missing); Rust keeps the whole word and uses "" when missing.
fn scan_two_words(s: &str) -> (&str, &str) {
    let mut words = s.split_whitespace();
    (words.next().unwrap_or(""), words.next().unwrap_or(""))
}

/// `new AccessLog(); SetProperties(copyme)`
fn copy_log(copyme: &AccessLog) -> AccessLog {
    let mut copy = AccessLog::new();
    copy.set_properties_from(copyme);
    copy
}

/// `comp->logbank` for the computer named `computer_name`.
fn logbank_of<'a>(g: &'a Ctx, computer_name: &str) -> &'a LogBank {
    &g.world()
        .get_computer(computer_name)
        .expect("LogBank: no computer with this name")
        .logbank
}

fn logbank_of_mut<'a>(g: &'a mut Ctx, computer_name: &str) -> &'a mut LogBank {
    &mut g
        .world_mut()
        .get_computer_mut(computer_name)
        .expect("LogBank: no computer with this name")
        .logbank
}

/// C++ `this->TraceLog(to_ip, logbank_ip, date, uplinkrating)` where `this` is the logbank of the
/// computer named `this_name`, whose ip is `logbank_ip`. The recursive call on another computer's
/// logbank re-enters here with that computer's name, so no borrow is held across the recursion.
fn trace_log_at(
    g: &mut Ctx,
    to_ip: &str,
    this_name: &str,
    logbank_ip: &str,
    date: &Date,
    uplinkrating: i32,
) -> Option<String> {
    //
    // Get some information regarding the local machine
    //
    // Nothing in the trace changes the type of these, so they are read once up front.
    let (local_type, is_bank, is_gov) = {
        let world = g.world();
        let vl_local = world
            .get_vlocation(logbank_ip)
            .expect("LogBank::TraceLog: no VLocation for local ip");
        let comp_local = vl_local
            .get_computer(world)
            .expect("LogBank::TraceLog: no Computer at local ip");
        let company_local = world
            .get_company(&comp_local.companyname)
            .expect("LogBank::TraceLog: no Company for local computer");
        (
            comp_local.type_,
            comp_local.type_ == COMPUTER_TYPE_PUBLICBANKSERVER,
            company_local.type_ == COMPANYTYPE_GOVERNMENT,
        )
    };

    // Try to find a log that showed a user bouncing from this machine
    // to to_ip.
    // Then recurse into that source log.

    let count = logbank_of(g, this_name).logs.size();
    for i in 0..count {
        // `logs.ValidIndex(i)`
        let Some(mut al) = logbank_of(g, this_name).logs.get_data(i).cloned() else {
            continue;
        };

        // If the log was deleted/overwritten we may be able to recover it

        if al.type_ == LOG_TYPE_DELETED && uplinkrating >= MINREQUIREDRATING_UNDELETELOGLEVEL1 {
            if let Some(recovered) = logbank_of(g, this_name).internallogs.get_data(i).cloned() {
                logbank_of_mut(g, this_name).logs.put_data_at(copy_log(&recovered), i);
                al = copy_log(&recovered);
            }
        } else if al.type_ != LOG_TYPE_DELETED
            && logbank_of(g, this_name).log_modified(i)
            && uplinkrating >= MINREQUIREDRATING_UNDELETELOGLEVEL3
        {
            // This one isn't deleted but overwritten, however the origional still exists

            if let Some(recovered) = logbank_of(g, this_name).internallogs.get_data(i).cloned() {
                logbank_of_mut(g, this_name).logs.put_data_at(copy_log(&recovered), i);
                al = copy_log(&recovered);
            }
        }

        // Only look at the log if it is within a few seconds of the connection date
        // (Otherwise it could be from anywhere)

        let mut upperdate = Date::default();
        let mut lowerdate = Date::default();

        upperdate.set_date_from(date);
        lowerdate.set_date_from(date);

        upperdate.advance_second(10);
        lowerdate.advance_second(-10);

        if al.date.after(&lowerdate) && al.date.before(&upperdate) {
            // Now look at the log

            if al.type_ == LOG_TYPE_BOUNCEBEGIN && al.data1.as_deref() == Some(to_ip) {
                // This computer is the origin of the bounced call
                // And is therefore the solution to this trace
                return Some(logbank_ip.to_string());
            } else if al.type_ == LOG_TYPE_BOUNCE && al.data1.as_deref() == Some(to_ip) {
                // Look up the source computer that created this log
                let source = {
                    let world = g.world();
                    world
                        .get_vlocation(&al.fromip)
                        .and_then(|vl| vl.get_computer(world))
                        .map(|comp| (comp.name.clone(), comp.ip.clone()))
                };

                if let Some((comp_name, comp_ip)) = source {
                    if (!is_bank || uplinkrating >= MINREQUIREDRATING_HACKBANKSERVER)
                        && (!is_gov || uplinkrating >= MINREQUIREDRATING_HACKGOVERNMENTCOMPUTER)
                    {
                        // comp->logbank.TraceLog(logbank_ip, comp->ip, date, uplinkrating)
                        return trace_log_at(g, logbank_ip, &comp_name, &comp_ip, date, uplinkrating);
                    }
                }
            }
        }
    }

    //
    // That log doesn't exist, or it can't be traced further
    // (this is the end of the trail).
    // If this is someone's personal computer, then he is guilty
    //

    if local_type == COMPUTER_TYPE_PERSONALCOMPUTER {
        Some(logbank_ip.to_string())
    } else {
        None
    }
}
