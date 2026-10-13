//! Port of source/world/connection.h + source/world/connection.cpp

use serde::{Deserialize, Serialize};

use crate::ctx::Ctx;
use crate::world::computer::computer::Computer;
use crate::world::computer::computerscreen::disconnectedscreen::DisconnectedScreen;
use crate::world::computer::lanmonitor::LanMonitor;
use crate::world::computer::logbank::{
    AccessLog, LOG_NOTSUSPICIOUS, LOG_SUSPICIOUS, LOG_TYPE_BOUNCE, LOG_TYPE_BOUNCEBEGIN,
    LOG_TYPE_CONNECTIONCLOSED, LOG_TYPE_CONNECTIONOPENED,
};
use crate::world::computer::securitymonitor::SecurityMonitor;
use crate::world::date::Date;
use crate::world::person::Person;
use crate::world::vlocation::VLocation;
use crate::world::world::World;

/// The Person `owner` (C++ `GetOwner()`, which asserts it exists).
fn owner_ref<'a>(world: &'a World, owner: &str) -> &'a Person {
    world.get_person(owner).expect("Connection: owner not found")
}

fn owner_mut<'a>(world: &'a mut World, owner: &str) -> &'a mut Person {
    world
        .get_person_mut(owner)
        .expect("Connection: owner not found")
}

/// `game->GetWorld()->GetVLocation(ip)`, asserted to exist.
fn vlocation_at<'a>(world: &'a World, ip: &str) -> &'a VLocation {
    world
        .get_vlocation(ip)
        .expect("Connection: VLocation not found")
}

/// `vl->GetComputer()` for the VLocation at `ip`, asserted to exist.
fn computer_at<'a>(world: &'a World, ip: &str) -> &'a Computer {
    vlocation_at(world, ip)
        .get_computer(world)
        .expect("Connection: no computer at VLocation")
}

/// The target (final ip) of the owner's connection, asserted to exist.
fn target_of(world: &World, owner: &str) -> String {
    owner_ref(world, owner)
        .connection
        .get_target()
        .expect("Connection: empty connection")
        .to_string()
}

/// `vlocations.GetData(index)`. C++ got NULL for an index outside the list (e.g. `i - 1` when
/// `i == 0`); kept as an empty ip here.
// PORT: the NULL case is unreachable for real connections (they always have a bounce node
// before the target), so the empty ip stands in for it.
fn ip_or_empty(vlocations: &[String], index: i32) -> String {
    usize::try_from(index)
        .ok()
        .and_then(|i| vlocations.get(i))
        .cloned()
        .unwrap_or_default()
}

/// The index of the "connection opened" log that the closing log `log1` pairs with, searching
/// back from the newest log. Stops at the first log more than 5 minutes older than `log1`.
fn find_opened_log(comp: &Computer, owner: &str, log1: &AccessLog) -> Option<i32> {
    let logs = &comp.logbank.logs;
    for i in (0..=logs.size()).rev() {
        if !logs.valid_index(i) {
            continue;
        }
        let al = logs
            .get_data(i)
            .expect("Connection::Disconnect: null access log");

        let mut testdate = Date::default();
        testdate.set_date_from(&al.date);
        testdate.advance_minute(5);

        if testdate.before(&log1.date) {
            break; // Every other log is > 5 mins before
        }

        if al.type_ == LOG_TYPE_CONNECTIONOPENED
            && al.fromname == owner
            && al.fromip == log1.fromip
        {
            return Some(i);
        }
    }
    None
}

/// A Person's connection from his physical location to his ultimate virtual location.
/// Lives in `Person::connection`; `owner` is the key of that Person ("PLAYER" for the player).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Connection {
    pub owner: String,
    /// List of IP addresses, source to target order
    pub vlocations: Vec<String>,

    pub traceinprogress: bool,
    /// 0 = no progress, ... , [size of connection-1] = traced
    pub traceprogress: i32,
    /// Date and time of next trace
    pub nexttrace: Date,
}

impl Connection {
    pub fn new() -> Self {
        Connection::default()
    }

    pub fn set_owner(&mut self, newowner: &str) {
        self.owner = newowner.to_string();
    }

    pub fn add_vlocation(&mut self, ip: &str) {
        // First make sure this ip isn't already in there
        if self.location_included(ip) {
            return;
        }
        self.vlocations.push(ip.to_string());
    }

    /// Removes the most recently added
    pub fn remove_last_location(&mut self) {
        self.vlocations.pop();
    }

    pub fn location_included(&self, ip: &str) -> bool {
        self.vlocations.iter().any(|v| v == ip)
    }

    /// `GetOwner`: read-only world lookup.
    pub fn get_owner<'a>(&self, world: &'a World) -> Option<&'a Person> {
        world.get_person(&self.owner)
    }

    /// First ip (source machine) or None
    pub fn get_source(&self) -> Option<&str> {
        self.vlocations.first().map(|s| s.as_str())
    }

    /// Final ip (target machine) or None
    pub fn get_target(&self) -> Option<&str> {
        self.vlocations.last().map(|s| s.as_str())
    }

    /// Final - 1 ip (apparent source machine) or None
    pub fn get_ghost(&self) -> Option<&str> {
        if self.vlocations.len() > 1 {
            Some(self.vlocations[self.vlocations.len() - 2].as_str())
        } else {
            None
        }
    }

    /// Number of nodes in connection
    pub fn get_size(&self) -> i32 {
        self.vlocations.len() as i32
    }

    // PORT: the next four touch the world (target computer, logs, remote host), so they take the
    // owner's key (== `self.owner`; "PLAYER" for the player's connection) instead of `self`:
    // `Connection::connect(g, "PLAYER")`. They reach the connection via the owner's `Person`.

    /// Starts a trace if the target computer's trace action allows it.
    pub fn begin_trace(g: &mut Ctx, owner: &str) {
        if owner_ref(g.world(), owner).connection.traceinprogress {
            return;
        }

        // Should a trace occur?
        // Check the trace action
        let target = target_of(g.world(), owner);
        let traceaction = computer_at(g.world(), &target).traceaction;

        if traceaction > 0 {
            let conn = &mut owner_mut(g.world_mut(), owner).connection;
            conn.traceinprogress = true;
            conn.traceprogress = 0;
        }
    }

    pub fn connect(g: &mut Ctx, owner: &str) {
        let target = target_of(g.world(), owner);
        let now = g.world().date.clone();

        let person = owner_mut(g.world_mut(), owner);
        person.set_remote_host(&target);
        person.connection.traceprogress = 0;
        person.connection.traceinprogress = false;
        person.connection.nexttrace.set_date_from(&now);

        //
        // If any security system is disabled, begin a trace immediately
        //

        if computer_at(g.world(), &target)
            .security
            .is_anything_disabled()
        {
            Connection::begin_trace(g, owner);
        }

        //
        // Special case if this is the Player
        //

        if owner == "PLAYER" {
            SecurityMonitor::begin_attack(g);
            LanMonitor::begin_attack(g);

            if computer_at(g.world(), &target).companyname == "ARC" {
                g.world_mut().plotgenerator.player_visits_plot_sites();
            }
        }

        //
        // Log this action on each node in the connection
        //

        let vlocations = owner_ref(g.world(), owner).connection.vlocations.clone();
        let size = vlocations.len();

        for i in 0..size {
            let bouncename = match vlocation_at(g.world(), &vlocations[i]).get_computer(g.world())
            {
                Some(c) => c.name.clone(),
                None => continue,
            };

            let date = g.world().date.clone();
            let mut log = AccessLog::new();

            if i == size - 1 {
                // Previous -> This (ie connection opened)
                log.set_properties(
                    &date,
                    &ip_or_empty(&vlocations, i as i32 - 1),
                    owner,
                    LOG_NOTSUSPICIOUS,
                    LOG_TYPE_CONNECTIONOPENED,
                );
            } else {
                // Previous -> Next (ie a bounce)
                let (logtype, fromip) = if i == 0 {
                    (LOG_TYPE_BOUNCEBEGIN, "LOCAL".to_string())
                } else {
                    (LOG_TYPE_BOUNCE, vlocations[i - 1].clone())
                };

                log.set_properties(&date, &fromip, owner, LOG_NOTSUSPICIOUS, logtype);
                log.set_data1(Some(vlocations[i + 1].as_str()));
            }

            g.world_mut()
                .get_computer_mut(&bouncename)
                .expect("Connection::Connect: bounce computer not found")
                .logbank
                .add_log(log, -1);
        }
    }

    pub fn disconnect(g: &mut Ctx, owner: &str) {
        /*
           Log this action on the final node in the connection
           This is a "suspicious" action if the trace was started but not completed
           (A trace is only started if the person is hacking - and if it finished
           then he has already been caught - no need to be suspicious

           */

        let target = target_of(g.world(), owner);
        let comp_name = computer_at(g.world(), &target).name.clone();

        let (trace_suspicious, vlocations, traceprogress) = {
            let conn = &owner_ref(g.world(), owner).connection;
            (
                conn.trace_in_progress() && !conn.traced(),
                conn.vlocations.clone(),
                conn.traceprogress,
            )
        };

        let date = g.world().date.clone();
        let mut log1 = AccessLog::new();
        log1.set_properties(
            &date,
            &ip_or_empty(&vlocations, vlocations.len() as i32 - 2),
            owner,
            LOG_NOTSUSPICIOUS,
            LOG_TYPE_CONNECTIONCLOSED,
        );

        /*
           If it was suspicious, look for previous "connection opened" logs
           made by this person and make them suspicious instead
           (There should be only 1 suspicious log left behind for this hack)

           If it wasn't suspicious, just add a standard "conenction closed" log

           */

        if !trace_suspicious {
            g.world_mut()
                .get_computer_mut(&comp_name)
                .expect("Connection::Disconnect: computer not found")
                .logbank
                .add_log(log1, -1);
        } else {
            match find_opened_log(computer_at(g.world(), &target), owner, &log1) {
                Some(i) => {
                    let comp = g
                        .world_mut()
                        .get_computer_mut(&comp_name)
                        .expect("Connection::Disconnect: computer not found");

                    if let Some(al) = comp.logbank.logs.get_data_mut(i) {
                        al.set_suspicious(LOG_SUSPICIOUS);
                    }
                    if let Some(il) = comp.logbank.internallogs.get_data_mut(i) {
                        il.set_suspicious(LOG_SUSPICIOUS);
                    }

                    // Add in the connection closed log, not suspicious
                    comp.logbank.add_log(log1, -1);
                }
                None => {
                    // Didn't find a connection opened log, so add in the
                    // connection closed log as suspicious

                    log1.set_suspicious(LOG_SUSPICIOUS);
                    g.world_mut()
                        .get_computer_mut(&comp_name)
                        .expect("Connection::Disconnect: computer not found")
                        .logbank
                        .add_log(log1, -1);
                }
            }
        }

        //
        // Do the disconnect
        //

        if owner == "PLAYER" {
            g.security_monitor.end_attack();
            LanMonitor::end_attack(g);

            // Change admin passwords if trace is more than 50% complete

            let size = vlocations.len() as i32;
            let rumbledindex = traceprogress - (size - 1) / 2;

            if rumbledindex >= 0 {
                for i in (0..=rumbledindex).rev() {
                    let rumbled_ip = usize::try_from(size - i - 1)
                        .ok()
                        .and_then(|x| vlocations.get(x))
                        .expect("Connection::Disconnect: rumbled ip not found")
                        .clone();

                    if g.player().has_account(g.world(), &rumbled_ip) != -1 {
                        let (comp_name, comp_ip) = {
                            let comp = computer_at(g.world(), &rumbled_ip);
                            (comp.name.clone(), comp.ip.clone())
                        };

                        let changed = Computer::change_security_codes(g, &comp_name);

                        if changed {
                            DisconnectedScreen::add_login_lost(&comp_ip);
                        }
                    }
                }
            }
        }

        let person = owner_mut(g.world_mut(), owner);
        let localhost = person.localhost.clone();
        person.set_remote_host(&localhost);
        person.connection.traceprogress = 0;
        person.connection.traceinprogress = false;
    }

    /// Empties the connection and re-adds the owner's localhost.
    pub fn reset(g: &mut Ctx, owner: &str) {
        let person = owner_mut(g.world_mut(), owner);
        let localhost = person.localhost.clone();

        let conn = &mut person.connection;
        conn.vlocations.clear();
        conn.add_vlocation(&localhost);

        conn.traceinprogress = false;
        conn.traceprogress = 0;
    }

    pub fn end_trace(&mut self) {
        self.traceinprogress = false;
    }

    pub fn trace_in_progress(&self) -> bool {
        self.traceinprogress
    }

    pub fn traced(&self) -> bool {
        self.traceinprogress && self.traceprogress == self.get_size() - 1
    }

    /// owner's localhost != remotehost
    pub fn connected(&self, world: &World) -> bool {
        match self.get_owner(world) {
            Some(p) => p.localhost != p.remotehost,
            None => false,
        }
    }
}
