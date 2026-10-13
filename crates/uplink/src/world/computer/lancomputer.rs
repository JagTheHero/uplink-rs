//! Port of source/world/computer/lancomputer.h + source/world/computer/lancomputer.cpp
//!
//! A computer made up of lots of smaller computers arranged in a network.
//! PORT (§3.3): `LanComputer` holds only the extra fields; it lives in `Computer::kind`
//! (`ComputerKind::LanComputer`). `LanComputer::Update` just calls `Computer::Update`, so it has no
//! Rust counterpart: `Computer::update` handles every kind.

use serde::{Deserialize, Serialize};

use crate::game::data::data::{LAN_LINKPORTRANGE, LAN_SUBNETRANGE};
use crate::tosser::DArray;
use crate::world::computer::computer::{Computer, ComputerKind};
use crate::world::generator::numbergenerator::NumberGenerator;

pub const LANSYSTEM_NUMTYPES: i32 = 17;

pub const LANSYSTEM_NONE: i32 = 0;
pub const LANSYSTEM_ROUTER: i32 = 1;
pub const LANSYSTEM_HUB: i32 = 2;
pub const LANSYSTEM_SWITCH: i32 = 3;
pub const LANSYSTEM_TERMINAL: i32 = 4;
pub const LANSYSTEM_MAINSERVER: i32 = 5; // data = locks that must be unlocked first
pub const LANSYSTEM_MAILSERVER: i32 = 6;
pub const LANSYSTEM_FILESERVER: i32 = 7;
pub const LANSYSTEM_AUTHENTICATION: i32 = 8; // data = systems to unlock
pub const LANSYSTEM_LOCK: i32 = 9; // data1 = 1 (locked) or 0 (unlocked)
pub const LANSYSTEM_ISOLATIONBRIDGE: i32 = 10; // data1 = gateway lock, data2 = critical system lock
pub const LANSYSTEM_MODEM: i32 = 11;
pub const LANSYSTEM_SESSIONKEYSERVER: i32 = 12;
pub const LANSYSTEM_RADIOTRANSMITTER: i32 = 13;
pub const LANSYSTEM_RADIORECEIVER: i32 = 14; // data1 = freq ghz, data2 = freq mhz
pub const LANSYSTEM_FAXPRINTER: i32 = 15;
pub const LANSYSTEM_LOGSERVER: i32 = 16;

pub const LANSYSTEMVISIBLE_NONE: i32 = 0; // Can't see anything
pub const LANSYSTEMVISIBLE_AWARE: i32 = 1; // Aware of its existence
pub const LANSYSTEMVISIBLE_TYPE: i32 = 2; // We know what sort of system it is
pub const LANSYSTEMVISIBLE_FULL: i32 = 3; // We know all about it

pub const LANLINKVISIBLE_NONE: i32 = 0; // Can't see anything
pub const LANLINKVISIBLE_FROMAWARE: i32 = 1; // Aware of where it comes from
pub const LANLINKVISIBLE_TOAWARE: i32 = 2; // Aware of where it goes to
pub const LANLINKVISIBLE_AWARE: i32 = 3; // Aware of the full link, from and to

/// The extra fields of a LanComputer (the Computer base fields live in `Computer`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LanComputer {
    pub systems: DArray<LanComputerSystem>,
    pub links: DArray<LanComputerLink>,
}

impl LanComputer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the index of the new system.
    pub fn add_lan_system(&mut self, system: LanComputerSystem) -> i32 {
        self.systems.put_data(system)
    }

    /// `AddLanSystem(TYPE, x, y, security, screenIndex, dataScreenIndex)`; C++ defaults:
    /// security 2, screen_index -1, data_screen_index -1. Picks a random subnet (NumberGenerator).
    pub fn add_lan_system_new(
        &mut self,
        type_: i32,
        x: i32,
        y: i32,
        security: i32,
        screen_index: i32,
        data_screen_index: i32,
    ) -> i32 {
        let mut system = LanComputerSystem::new();
        system.type_ = type_;
        system.x = x;
        system.y = y;
        system.subnet = NumberGenerator::random_number(LAN_SUBNETRANGE);
        system.security = security;
        system.screen_index = screen_index;
        system.data_screen_index = data_screen_index;
        if type_ == LANSYSTEM_ROUTER {
            system.visible = LANSYSTEMVISIBLE_TYPE;
        }
        self.add_lan_system(system)
    }

    /// C++ default security: 1. Picks a random port (NumberGenerator). Returns the link index.
    pub fn add_lan_link(
        &mut self,
        from: i32,
        from_x: f32,
        from_y: f32,
        to: i32,
        to_x: f32,
        to_y: f32,
        security: i32,
    ) -> i32 {
        let mut link = LanComputerLink::new();
        link.from = from;
        link.to = to;
        link.from_x = from_x;
        link.from_y = from_y;
        link.to_x = to_x;
        link.to_y = to_y;
        link.port = NumberGenerator::random_number(LAN_LINKPORTRANGE);
        link.security = security;
        self.links.put_data(link)
    }
}

impl Computer {
    /// A new `Computer` of kind LanComputer.
    pub fn new_lan_computer() -> Computer {
        Computer {
            kind: ComputerKind::LanComputer(LanComputer::new()),
            ..Computer::new()
        }
    }
}

// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LanComputerSystem {
    pub type_: i32,
    pub x: i32,
    pub y: i32,
    pub visible: i32,
    pub subnet: i32,
    pub security: i32,

    pub screen_index: i32,
    /// Index of a screen in which we can dump information
    pub data_screen_index: i32,

    pub data1: i32,
    pub data2: i32,
    pub data3: i32,

    /// All valid FROM connections
    pub valid_subnets: Vec<i32>,
}

impl Default for LanComputerSystem {
    fn default() -> Self {
        LanComputerSystem {
            type_: LANSYSTEM_NONE,
            x: 0,
            y: 0,
            visible: LANSYSTEMVISIBLE_NONE,
            subnet: 0,
            security: 0,
            screen_index: -1,
            data_screen_index: -1,
            data1: -1,
            data2: -1,
            data3: -1,
            valid_subnets: Vec::new(),
        }
    }
}

impl LanComputerSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn increase_visibility(&mut self, new_value: i32) {
        if new_value > self.visible {
            self.visible = new_value;
        }
    }
}

// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LanComputerLink {
    pub visible: i32,
    pub port: i32,
    pub security: i32,

    pub from: i32,
    pub to: i32,

    /// Range from 0.0 to 1.0, along the X and Y sides, so the link can be positioned at any
    /// point on the edge of the system
    pub from_x: f32,
    pub from_y: f32,

    pub to_x: f32,
    pub to_y: f32,
}

impl Default for LanComputerLink {
    fn default() -> Self {
        LanComputerLink {
            visible: LANLINKVISIBLE_NONE,
            port: 1000,
            security: 1,
            from: -1,
            to: -1,
            from_x: 0.0,
            from_y: 0.0,
            to_x: 0.0,
            to_y: 0.0,
        }
    }
}

impl LanComputerLink {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn increase_visibility(&mut self, new_value: i32) {
        if new_value > self.visible {
            self.visible = new_value;
        }
    }
}
