//! Port of source/world/player.h + source/world/player.cpp

use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};

use crate::ctx::Ctx;
use crate::game::data::data::TRACESPEED_VARIANCE;
use crate::tosser::BTree;
use crate::world::computer::computer::{
    COMPUTER_TRACEACTION_FINE, COMPUTER_TYPE_CENTRALMAINFRAME, COMPUTER_TYPE_PUBLICBANKSERVER,
};
use crate::world::computer::gateway::Gateway;
use crate::world::computer::lanmonitor::LanMonitor;
use crate::world::computer::securitymonitor::SecurityMonitor;
use crate::world::generator::consequencegenerator::ConsequenceGenerator;
use crate::world::generator::numbergenerator::NumberGenerator;
use crate::world::message::Message;
use crate::world::person::Person;
use crate::world::world::World;

// game/data/data.h TRACESPEED_MODIFIER_* values used below. The C++ defines are double literals and
// the arithmetic is done in double (`timetonexttrace *= 0.7`), so they are kept as f64 here: the
// f32 constants in data.rs would truncate 10 * 0.7 to 6 instead of 7.
const TRACESPEED_MODIFIER_NOACCOUNT: f64 = 0.1;
const TRACESPEED_MODIFIER_HASACCOUNT: f64 = 0.7;
const TRACESPEED_MODIFIER_ADMINACCESS: f64 = 1.0;
const TRACESPEED_MODIFIER_PUBLICBANKSERVERADMIN: f64 = 1.6;

/// The player: Player ISA Agent ISA Person (PORTING.md §3.6). `person.agent` is always `Some`.
/// Derefs to `Person`, so `player.name`, `player.accounts`, `player.has_link(ip)` work.
/// The world stores it in `World::player`, not in `World::people`; its key is "PLAYER".
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Player {
    pub person: Person,

    /// Indexed on company name, values are "numowned totalpricepaid"
    pub shares: BTree<String>,

    pub gateway: Gateway,

    pub score_peoplefucked: i32,
    pub score_systemsfucked: i32,
    pub score_highsecurityhacks: i32,
}

impl Default for Player {
    fn default() -> Self {
        Player {
            person: Person::new_agent(),
            shares: BTree::new(),
            gateway: Gateway::default(),
            score_peoplefucked: 0,
            score_systemsfucked: 0,
            score_highsecurityhacks: 0,
        }
    }
}

impl Deref for Player {
    type Target = Person;

    fn deref(&self) -> &Person {
        &self.person
    }
}

impl DerefMut for Player {
    fn deref_mut(&mut self) -> &mut Person {
        &mut self.person
    }
}

/// C++ `int *= double`: the product is computed in double and truncated back to int.
fn scale_i32(value: i32, factor: f64) -> i32 {
    (value as f64 * factor) as i32
}

/// C++ `sscanf(stringdata, "%d %d", &numowned, &pricepaid)`.
fn parse_shares(stringdata: &str) -> (i32, i32) {
    let mut it = stringdata
        .split_whitespace()
        .map(|v| v.parse::<i32>().unwrap_or(0));
    (it.next().unwrap_or(0), it.next().unwrap_or(0))
}

impl Player {
    pub fn new() -> Self {
        Player::default()
    }

    /// Gives the player a link to every location in the world.
    pub fn give_all_links(g: &mut Ctx) {
        // locations.ConvertToDArray: BTree pre-order (depends on the insertion history of the tree)
        let ips: Vec<String> = g.world().locations.values().map(|vl| vl.ip.clone()).collect();
        for ip in ips {
            Person::give_link(g, "PLAYER", &ip);
        }
    }

    /// `Player::GiveMessage`: the Agent behaviour, plus a copy of any attached data goes into
    /// the gateway's memory.
    pub fn give_message_player(g: &mut Ctx, message: Message) {
        // The Agent behaviour takes the message, so copy the attachment first (the C++ copied it
        // after the Agent call; Agent::GiveMessage does not touch the attachment).
        let attached = message.get_data().cloned();

        Person::give_message_agent(g, "PLAYER", message);

        // Transfer any attached files from the message into memory
        if let Some(datacopy) = attached {
            g.player_mut().gateway.databank.put_data(datacopy);
        }
    }

    /// True if this account is owned by the player
    pub fn is_player_account(&self, bankip: &str, accno: &str) -> bool {
        let search = format!("{} {}", bankip, accno);
        self.accounts.iter().any(|a| *a == search)
    }

    pub fn num_shares_owned(&self, companyname: &str) -> i32 {
        self.shares
            .get(companyname)
            .and_then(|s| s.split_whitespace().next())
            .and_then(|v| v.parse().ok())
            .unwrap_or(0)
    }

    pub fn shares_price_paid(&self, companyname: &str) -> i32 {
        self.shares
            .get(companyname)
            .and_then(|s| s.split_whitespace().nth(1))
            .and_then(|v| v.parse().ok())
            .unwrap_or(0)
    }

    /// Buy (`howmany` > 0) or sell (< 0) shares; moves money through `change_balance`.
    pub fn trade_shares(g: &mut Ctx, companyname: &str, howmany: i32) {
        // Look up all neccisary data on company
        // and the shares we own in it
        let shareprice = g
            .world()
            .get_company(companyname)
            .expect("TradeShares: company not found")
            .get_share_price(-1);

        let stringdata = g.player().shares.get(companyname).cloned();

        match stringdata {
            None => {
                // No existing transactions
                if howmany > 0 {
                    let cost = howmany * shareprice;

                    if g.player().get_balance(g.world()) >= cost {
                        let newentry = format!("{} {}", howmany, cost);
                        g.player_mut().shares.put_data(companyname, newentry);

                        Person::change_balance(
                            g,
                            "PLAYER",
                            howmany * shareprice * -1,
                            "International Stock Market",
                        );
                    }
                }
            }
            Some(stringdata) => {
                // We already have some
                let (mut numowned, mut pricepaid) = parse_shares(&stringdata);

                if howmany > 0 {
                    // We are buying more
                    numowned += howmany;
                    pricepaid += howmany * shareprice;

                    // You can't afford this (the entry is then left as it was)
                    if howmany * shareprice > g.player().get_balance(g.world()) {
                        return;
                    }

                    Person::change_balance(
                        g,
                        "PLAYER",
                        howmany * shareprice * -1,
                        "International Stock Market",
                    );
                } else if howmany < 0 {
                    // We are selling some
                    if numowned > 0 {
                        let fractionsold = howmany as f32 / numowned as f32; // (howmany is a negative number)
                        let fractionremaining = 1.0 + fractionsold;

                        numowned += howmany;
                        pricepaid = (pricepaid as f32 * fractionremaining) as i32;

                        Person::change_balance(
                            g,
                            "PLAYER",
                            howmany * shareprice * -1,
                            "International Stock Market",
                        );
                    }
                }

                // sprintf into the existing entry
                let newentry = format!("{} {}", numowned, pricepaid);
                if let Some(entry) = g.player_mut().shares.get_mut(companyname) {
                    *entry = newentry;
                }
            }
        }
    }

    /// How long for the people at `tracer_ip` to trace through `router_ip`.
    // PORT: read-only world access (the C++ read the player's access level through the world).
    pub fn time_to_trace(world: &World, tracer_ip: &str, router_ip: &str) -> i32 {
        let tracer = world
            .get_vlocation(tracer_ip)
            .expect("TimeToTrace: tracer location not found");
        let comp = tracer
            .get_computer(world)
            .expect("TimeToTrace: tracer has no computer");

        let mut timetonexttrace = comp.tracespeed;

        //
        // Modifiers to the time to next trace

        let playeraccesslevel = world.get_player().has_account(world, router_ip);

        if playeraccesslevel == -1 && comp.traceaction > COMPUTER_TRACEACTION_FINE {
            timetonexttrace = scale_i32(timetonexttrace, TRACESPEED_MODIFIER_NOACCOUNT);
        } else if playeraccesslevel == 1 {
            timetonexttrace = scale_i32(timetonexttrace, TRACESPEED_MODIFIER_ADMINACCESS);
        } else if playeraccesslevel != -1 {
            timetonexttrace = scale_i32(timetonexttrace, TRACESPEED_MODIFIER_HASACCOUNT);
        }

        // PORT: as in the C++, this multiplies by COMPUTER_TYPE_CENTRALMAINFRAME (4), not by
        // TRACESPEED_MODIFIER_CENTRALMAINFRAME (1.3) which the C++ defined for this purpose.
        if playeraccesslevel != -1 && comp.type_ == COMPUTER_TYPE_CENTRALMAINFRAME {
            timetonexttrace *= COMPUTER_TYPE_CENTRALMAINFRAME;
        }

        if playeraccesslevel == 1 && comp.type_ == COMPUTER_TYPE_PUBLICBANKSERVER {
            timetonexttrace = scale_i32(timetonexttrace, TRACESPEED_MODIFIER_PUBLICBANKSERVERADMIN);
        }

        if timetonexttrace < 2 {
            timetonexttrace = 2;
        }

        timetonexttrace
    }

    /// How long remaining in this trace?
    pub fn time_remaining(&self, world: &World) -> i32 {
        let mut timeremaining = 0;

        let conn = self.get_connection();
        let numlocations = conn.get_size() - conn.traceprogress - 1;

        let tracer_ip = &self
            .get_remote_host(world)
            .expect("TimeRemaining: no remote host")
            .ip;

        for i in (0..=numlocations).rev() {
            let thisip = conn
                .vlocations
                .get(i as usize)
                .expect("TimeRemaining: connection location missing");
            timeremaining += Player::time_to_trace(world, tracer_ip, thisip);
        }

        timeremaining
    }

    /// `Player::Update`: Agent update, security/LAN monitors, and advances the trace (may call
    /// `ConsequenceGenerator::caught_hacking`).
    pub fn update(g: &mut Ctx) {
        Person::update_agent(g, "PLAYER");

        //
        // Update the remote monitors watching the hack on the
        // target computer
        //

        if g.player().is_connected() {
            SecurityMonitor::update(g);
            LanMonitor::update(g);
        }

        //
        // Update the trace on the player if one exists
        //

        // IsConnected() && GetRemoteHost()->GetComputer()->tracespeed > -1 && TraceInProgress()
        // PORT: the C++ dereferenced a NULL computer here if the remote host had none; that case
        // is treated as "no trace" instead.
        let trace_remote_ip = {
            let world = g.world();
            let p = world.get_player();
            let remote = p.get_remote_host(world);
            let traced = p.is_connected()
                && remote
                    .and_then(|vl| vl.get_computer(world))
                    .map_or(false, |c| c.tracespeed > -1)
                && p.get_connection().trace_in_progress();
            if traced {
                remote.map(|vl| vl.ip.clone())
            } else {
                None
            }
        };

        let Some(remote_ip) = trace_remote_ip else {
            return;
        };

        if g.player().get_connection().traceprogress < g.player().get_connection().get_size() - 1 {
            // Trace in progress - maybe advance

            let due = g
                .world()
                .date
                .after(&g.player().get_connection().nexttrace);

            if due {
                let thisip = {
                    let conn = g.player_mut().get_connection_mut();
                    conn.traceprogress += 1;

                    // Work out time to next trace
                    let index = conn.get_size() - conn.traceprogress - 1;
                    conn.vlocations
                        .get(index as usize)
                        .cloned()
                        .expect("Update: connection location missing")
                };

                let timetonexttrace = Player::time_to_trace(g.world(), &remote_ip, &thisip);
                let timetonexttrace = NumberGenerator::apply_variance(
                    timetonexttrace,
                    (TRACESPEED_VARIANCE * 100.0) as i32,
                );

                let now = g.world().date.clone();
                let conn = g.player_mut().get_connection_mut();
                conn.nexttrace.set_date_from(&now);
                conn.nexttrace.advance_second(timetonexttrace);
            }
        }

        // Check if the trace is now complete IMMEDIATELY
        // (So the player can't quickly disconnect between now and the next player update)

        if g.player().get_connection().traceprogress == g.player().get_connection().get_size() - 1 {
            // Trace completed - act on action

            // C++ also read the remote interface's security_name into a local that was never used;
            // that read has no effect and is not ported.
            let comp_name = {
                let world = g.world();
                world
                    .get_player()
                    .get_remote_host(world)
                    .and_then(|vl| vl.get_computer(world))
                    .map(|c| c.name.clone())
            };
            let comp_name = comp_name.expect("Update: trace completed with no target computer");

            ConsequenceGenerator::caught_hacking(g, "PLAYER", &comp_name);
        }
    }
}
