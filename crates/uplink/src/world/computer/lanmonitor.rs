//! Port of source/world/computer/lanmonitor.h + source/world/computer/lanmonitor.cpp
//!
//! The C++ class is a purely static object watching the player's current LAN attack.
//! PORT: the statics become the fields of this struct. The `LanComputer *lanComputer` pointer
//! becomes the name of the attacked computer (`lan_computer`), looked up through the world (it is a
//! `Computer` whose `kind` is `ComputerKind::LanComputer`).
//! The single instance is `Ctx::lan_monitor`, reached through `state` / `state_mut`.

use uplink_engine::sound;

use crate::ctx::Ctx;
use crate::interface::remoteinterface::remoteinterface::RemoteInterface;
use crate::world::computer::computer::COMPUTER_TYPE_LAN;
use crate::world::computer::lancomputer::{
    LanComputer, LANLINKVISIBLE_AWARE, LANLINKVISIBLE_FROMAWARE, LANLINKVISIBLE_TOAWARE,
    LANSYSTEM_FILESERVER, LANSYSTEM_LOCK, LANSYSTEM_LOGSERVER, LANSYSTEM_MAINSERVER,
    LANSYSTEM_ROUTER, LANSYSTEMVISIBLE_AWARE, LANSYSTEMVISIBLE_NONE, LANSYSTEMVISIBLE_TYPE,
};
use crate::world::connection::Connection;
use crate::world::date::Date;
use crate::world::generator::missiongenerator::MissionGenerator;
use crate::world::generator::numbergenerator::NumberGenerator;
use crate::world::generator::worldgenerator::WorldGenerator;
use crate::world::scheduler::attemptmissionevent::AttemptMissionEvent;
use crate::world::scheduler::uplinkevent::UplinkEvent;

pub const SYSADMIN_ASLEEP: i32 = 0;
pub const SYSADMIN_CURIOUS: i32 = 1;
pub const SYSADMIN_SEARCHING: i32 = 2;
pub const SYSADMIN_FOUNDYOU: i32 = 3;

/// Owner key of the player's connection (`Person` key "PLAYER").
const PLAYER: &str = "PLAYER";
/// Eclipse highlight name of the LAN view HUD button.
const HUD_LANVIEW: &str = "hud_lanview";

/// Read access to the monitor state (the C++ statics). Closures must not call back into it.
fn state<R>(g: &Ctx, f: impl FnOnce(&LanMonitor) -> R) -> R {
    f(&g.lan_monitor)
}

/// Write access to the monitor state (the C++ statics). Closures must not call back into it.
fn state_mut<R>(g: &mut Ctx, f: impl FnOnce(&mut LanMonitor) -> R) -> R {
    f(&mut g.lan_monitor)
}

/// The LAN computer under attack (C++ `lanComputer`), read-only.
fn current_lan(g: &Ctx) -> Option<&LanComputer> {
    let name = state(g, |m| m.lan_computer.clone())?;
    g.world().get_computer(&name)?.lan()
}

/// The LAN computer under attack (C++ `lanComputer`), mutable.
fn current_lan_mut(g: &mut Ctx) -> Option<&mut LanComputer> {
    let name = state(g, |m| m.lan_computer.clone())?;
    g.world_mut().get_computer_mut(&name)?.lan_mut()
}

/// `int timeToDiscover = NumberGenerator::RandomNormalNumber( 10, 5 );` (float truncated to int)
fn time_to_discover() -> i32 {
    NumberGenerator::random_normal_number(10.0, 5.0) as i32
}

#[derive(Debug, Clone)]
pub struct LanMonitor {
    /// Only true if we are connected to a LAN (C++ `currentlyActive`, protected)
    pub currently_active: bool,
    /// Name of the LAN computer under attack (C++ `lanComputer`, protected)
    pub lan_computer: Option<String>,

    /// Index of current system
    pub current_system: i32,
    /// Index of currently selected system
    pub current_selected: i32,
    /// Our current "From" address, maybe spoofed
    pub current_spoof: i32,

    /// The player's connection through the LAN (system indexes)
    pub connection: Vec<i32>,

    pub sys_admin_state: i32,
    pub sys_admin_current_system: i32,
    /// Eclipse accurate time (ms) at which the sysadmin acts next
    pub sys_admin_timer: i32,
}

impl Default for LanMonitor {
    fn default() -> Self {
        LanMonitor {
            currently_active: false,
            lan_computer: None,
            current_system: -1,
            current_selected: -1,
            current_spoof: -1,
            connection: Vec::new(),
            sys_admin_state: -1,
            sys_admin_current_system: -1,
            sys_admin_timer: -1,
        }
    }
}

impl LanMonitor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Call when player connects somewhere
    pub fn begin_attack(g: &mut Ctx) {
        let remotehost = g.player().remotehost.clone();
        let (comp_name, special_first_link, comp_is_lan) = {
            let world = g.world();
            let tvl = world
                .get_vlocation(&remotehost)
                .expect("LanMonitor::BeginAttack, player's remote host is not a VLocation");
            let comp = world
                .get_computer(&tvl.computer)
                .expect("LanMonitor::BeginAttack, VLocation has no computer");
            (
                tvl.computer.clone(),
                tvl.special.as_ref().map(|s| s.security_system_index),
                comp.type_ == COMPUTER_TYPE_LAN,
            )
        };

        state_mut(g, |m| {
            m.current_system = -1;
            m.current_selected = -1;
            m.current_spoof = -1;
            m.sys_admin_state = SYSADMIN_ASLEEP;
            m.sys_admin_current_system = -1;
        });

        if !comp_is_lan {
            state_mut(g, |m| m.currently_active = false);
            return;
        }

        Self::reset_all(g); // Just to be safe

        state_mut(g, |m| {
            m.lan_computer = Some(comp_name.clone());
            m.currently_active = true;
        });

        let mut our_first_link = special_first_link.unwrap_or(-1);

        if our_first_link == -1 {
            // Insert the Router to the connection
            if let Some(lan) = current_lan(g) {
                for (k, system) in lan.systems.iter() {
                    if system.type_ == LANSYSTEM_ROUTER {
                        our_first_link = k;
                        break;
                    }
                }
            }
        }

        debug_assert!(our_first_link != -1, "LanMonitor::BeginAttack, no router in the LAN");

        state_mut(g, |m| {
            m.connection.insert(0, our_first_link); // PutDataAtStart
            m.current_system = our_first_link;
            m.current_selected = our_first_link;
        });

        current_lan_mut(g)
            .and_then(|lan| lan.systems.get_data_mut(our_first_link))
            .expect("LanMonitor::BeginAttack, first link is not a LAN system")
            .increase_visibility(LANSYSTEMVISIBLE_TYPE);
    }

    /// Call when player disconnects
    pub fn end_attack(g: &mut Ctx) {
        Self::reset_all(g);
    }

    /// Empties all data structures (also un-highlights the "hud_lanview" button).
    pub fn reset_all(g: &mut Ctx) {
        state_mut(g, |m| {
            m.connection.clear();
            m.currently_active = false;
            m.lan_computer = None;
            m.current_system = -1;
            m.current_selected = -1;
            m.current_spoof = -1;
        });
        g.ecl.super_unhighlight(HUD_LANVIEW);
    }

    /// Also resets the remote interface security level to "Guest", 10.
    pub fn set_current_system(g: &mut Ctx, new_current_system: i32) {
        state_mut(g, |m| m.current_system = new_current_system);

        // Reset our security level
        g.interface_mut().ri.set_security("Guest", 10);
    }

    pub fn set_current_selected(&mut self, new_current_selected: i32) {
        self.current_selected = new_current_selected;
    }

    pub fn set_current_spoof(&mut self, new_current_spoof: i32) {
        self.current_spoof = new_current_spoof;
    }

    /// May not do anything
    pub fn extend_connection(g: &mut Ctx, new_node: i32) {
        if !Self::is_accessable(g, new_node) {
            return;
        }

        // Is it adjacent to an existing linked system, with a visible link to this one?
        let link_head = match state(g, |m| m.connection.last().copied()) {
            Some(head) => head,
            None => return,
        };

        let linked = {
            let lan = current_lan(g).expect("LanMonitor::ExtendConnection, no LAN computer");
            lan.links.values().any(|link| {
                ((link.from == link_head && link.to == new_node)
                    || (link.to == link_head && link.from == new_node))
                    && link.visible == LANLINKVISIBLE_AWARE
            })
        };

        if linked {
            state_mut(g, |m| m.connection.push(new_node)); // PutDataAtEnd
            current_lan_mut(g)
                .and_then(|lan| lan.systems.get_data_mut(new_node))
                .expect("LanMonitor::ExtendConnection, new node is not a LAN system")
                .increase_visibility(LANSYSTEMVISIBLE_TYPE);
        }
    }

    /// Will extend every time
    pub fn force_extend_connection(g: &mut Ctx, new_node: i32) {
        // Look up the new system
        let exists = {
            let lan = current_lan(g).expect("LanMonitor::ForceExtendConnection, no LAN computer");
            lan.systems.valid_index(new_node)
        };
        if !exists {
            return;
        }

        state_mut(g, |m| m.connection.push(new_node)); // PutDataAtEnd
        current_lan_mut(g)
            .and_then(|lan| lan.systems.get_data_mut(new_node))
            .expect("LanMonitor::ForceExtendConnection, new node is not a LAN system")
            .increase_visibility(LANSYSTEMVISIBLE_TYPE);
    }

    /// Remove last link
    pub fn retract_connection(&mut self) {
        let remove_index = self.connection.len() as i32 - 1;
        if remove_index > 0 {
            self.connection.remove(remove_index as usize);
            self.current_selected = *self.connection.last().unwrap();
        }
    }

    pub fn is_in_connection(&self, node: i32) -> bool {
        self.get_node_index(node) != -1
    }

    /// Position of `node` in `connection`, or -1.
    pub fn get_node_index(&self, node: i32) -> i32 {
        self.connection
            .iter()
            .position(|&n| n == node)
            .map(|i| i as i32)
            .unwrap_or(-1)
    }

    pub fn is_accessable(g: &Ctx, new_node: i32) -> bool {
        // Look up the new system
        let (currently_active, current_system, current_spoof) =
            state(g, |m| (m.currently_active, m.current_system, m.current_spoof));

        if !currently_active {
            return false;
        }

        let lan = current_lan(g).expect("LanMonitor::IsAccessable, no LAN computer");

        if !lan.systems.valid_index(new_node) {
            return false;
        }

        let new_system = lan
            .systems
            .get_data(new_node)
            .expect("LanMonitor::IsAccessable, new node is not a LAN system");

        // Is it visible / known about to the player?
        if new_system.visible == LANSYSTEMVISIBLE_NONE || new_system.visible == LANSYSTEMVISIBLE_AWARE {
            return false;
        }

        // Is the system an 'island'?
        if new_node != current_system {
            let known_links = lan.links.values().any(|link| {
                (link.to == new_node && link.visible == LANLINKVISIBLE_TOAWARE)
                    || (link.from == new_node && link.visible == LANLINKVISIBLE_FROMAWARE)
                    || ((link.to == new_node || link.from == new_node)
                        && link.visible == LANLINKVISIBLE_AWARE)
            });

            if !known_links {
                return false;
            }
        }

        // Is it a locked lock?
        if new_system.type_ == LANSYSTEM_LOCK && new_system.data1 == 1 {
            return false;
        }

        // If it is a Server, all locks must be unlocked
        if new_system.type_ == LANSYSTEM_MAINSERVER {
            let locked = |index: i32| {
                index != -1
                    && lan
                        .systems
                        .get_data(index)
                        .expect("LanMonitor::IsAccessable, lock is not a LAN system")
                        .data1
                        == 1
            };
            if locked(new_system.data1) || locked(new_system.data2) || locked(new_system.data3) {
                return false;
            }
        }

        // If it has restricted FROM addresses, are we coming from one of them?
        if !new_system.valid_subnets.is_empty() {
            if current_spoof == -1 {
                return false;
            }
            return new_system.valid_subnets.contains(&current_spoof);
        }

        true
    }

    /// Needs the Eclipse clock (`EclGetAccurateTime`) and NumberGenerator.
    pub fn sys_admin_awaken(g: &mut Ctx) {
        if state(g, |m| m.sys_admin_state) < SYSADMIN_CURIOUS {
            state_mut(g, |m| m.sys_admin_state = SYSADMIN_CURIOUS);
            let timeout = time_to_discover();
            let now = g.ecl.get_accurate_time();
            state_mut(g, |m| m.sys_admin_timer = now + timeout * 1000);
        }
    }

    pub fn sys_admin_sleep(&mut self) {
        self.sys_admin_state = SYSADMIN_ASLEEP;
        self.sys_admin_current_system = -1;
    }

    /// Call frequently
    pub fn update(g: &mut Ctx) {
        if state(g, |m| m.sys_admin_state) < SYSADMIN_CURIOUS && g.ecl.is_super_highlighted(HUD_LANVIEW)
        {
            g.ecl.super_unhighlight(HUD_LANVIEW);
        } else if state(g, |m| m.sys_admin_state) >= SYSADMIN_CURIOUS
            && !g.ecl.is_super_highlighted(HUD_LANVIEW)
        {
            g.ecl.super_highlight(HUD_LANVIEW);
            sound::play_sound("sounds/siren.wav", Some("sounds/sirens.wav"), true);
        }

        if !state(g, |m| m.currently_active) {
            return;
        }

        // Update the system administrator
        match state(g, |m| m.sys_admin_state) {
            SYSADMIN_ASLEEP => {
                let current_system = state(g, |m| m.current_system);
                let watched = current_lan(g)
                    .and_then(|lan| lan.systems.get_data(current_system))
                    .is_some_and(|sys| {
                        matches!(
                            sys.type_,
                            LANSYSTEM_MAINSERVER | LANSYSTEM_FILESERVER | LANSYSTEM_LOGSERVER
                        )
                    });

                if watched {
                    Self::sys_admin_awaken(g);
                }
            }

            SYSADMIN_CURIOUS => {
                if g.ecl.get_accurate_time() >= state(g, |m| m.sys_admin_timer) {
                    state_mut(g, |m| m.sys_admin_state = SYSADMIN_SEARCHING);
                    match state(g, |m| m.connection.first().copied()) {
                        Some(first) => state_mut(g, |m| m.sys_admin_current_system = first),
                        None => state_mut(g, |m| m.sys_admin_state = SYSADMIN_ASLEEP),
                    }
                    let timeout = time_to_discover();
                    let now = g.ecl.get_accurate_time();
                    state_mut(g, |m| m.sys_admin_timer = now + timeout * 1000);
                    Connection::begin_trace(g, PLAYER);
                }
            }

            SYSADMIN_SEARCHING => {
                let (admin_system, current_system) =
                    state(g, |m| (m.sys_admin_current_system, m.current_system));

                if admin_system == current_system {
                    state_mut(g, |m| m.sys_admin_state = SYSADMIN_FOUNDYOU);
                } else if g.ecl.get_accurate_time() >= state(g, |m| m.sys_admin_timer) {
                    let node_index = state(g, |m| m.get_node_index(admin_system));
                    if node_index == -1 {
                        state_mut(g, |m| m.sys_admin_state = SYSADMIN_CURIOUS);
                    } else {
                        let next = state(g, |m| m.connection.get((node_index + 1) as usize).copied());
                        match next {
                            Some(node) => state_mut(g, |m| m.sys_admin_current_system = node),
                            None => state_mut(g, |m| m.sys_admin_state = SYSADMIN_CURIOUS),
                        }
                    }
                    let timeout = time_to_discover();
                    let now = g.ecl.get_accurate_time();
                    state_mut(g, |m| m.sys_admin_timer = now + timeout * 1000);
                }
            }

            SYSADMIN_FOUNDYOU => {
                // Find an agent who will perform a Trace mission on you in a few seconds
                let agent_name = WorldGenerator::get_random_agent(g);
                debug_assert!(!agent_name.is_empty(), "LanMonitor::Update, no agent");

                let lan_name = state(g, |m| m.lan_computer.clone())
                    .expect("LanMonitor::Update, no LAN computer under attack");
                let mission = MissionGenerator::generate_trace_hacker_computer(g, &lan_name, PLAYER)
                    .expect("LanMonitor::Update, Generate_TraceHacker returned no mission");

                // Remove the mission from Uplink's list. PORT: Generate_TraceHacker posts the
                // mission to Uplink's board (CompanyUplink::CreateMission) and the C++ finds it
                // there by pointer; the Rust generator returns a clone, so match on its contents.
                let cu = g
                    .world_mut()
                    .get_company_mut("Uplink")
                    .and_then(|c| c.uplink.as_mut())
                    .expect("LanMonitor::Update, no Uplink company");
                if let Some(i) = cu.missions.iter().position(|m| {
                    m.type_ == mission.type_
                        && m.payment == mission.payment
                        && m.completiona == mission.completiona
                        && m.details == mission.details
                        && m.fulldetails == mission.fulldetails
                }) {
                    cu.missions.remove(i);
                }

                let agent = g
                    .world_mut()
                    .get_person_mut(&agent_name)
                    .and_then(|p| p.agent.as_mut())
                    .expect("LanMonitor::Update, agent has no Agent data");
                agent.missions.insert(0, mission); // PutDataAtStart

                // Schedule an event to make the agent attempt the mission in a few seconds
                let mut duedate = Date::default();
                duedate.set_date_from(&g.world().date);
                duedate.advance_second(NumberGenerator::random_normal_number(30.0, 10.0) as i32);

                let mut ami = AttemptMissionEvent::new();
                ami.set_agent_name(&agent_name);
                let mut event = UplinkEvent::AttemptMissionEvent(ami);
                event.set_run_date(&duedate);
                g.world_mut().scheduler.schedule_event(event);

                state_mut(g, |m| {
                    m.sys_admin_state = SYSADMIN_ASLEEP;
                    m.sys_admin_current_system = -1;
                    m.sys_admin_timer = 0;
                });

                // Dump the player out
                Connection::disconnect(g, PLAYER);
                Connection::reset(g, PLAYER);
                RemoteInterface::run_new_location(g);
                RemoteInterface::run_screen(g, 2);
            }

            _ => {}
        }
    }
}
