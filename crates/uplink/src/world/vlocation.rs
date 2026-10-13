//! Port of source/world/vlocation.h + source/world/vlocation.cpp

use serde::{Deserialize, Serialize};

use crate::world::computer::computer::{Computer, COMPUTER_TYPE_LAN};
use crate::world::world::World;

pub const VIRTUAL_WIDTH: i32 = 594;
pub const VIRTUAL_HEIGHT: i32 = 315;

/// Special Virtual Location extra fields: used to specify which screen you enter the system at
/// (currently used on modems, which can jump into a system at any screen).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VLocationSpecial {
    pub screen_index: i32,
    pub security_system_index: i32,
}

impl Default for VLocationSpecial {
    fn default() -> Self {
        VLocationSpecial {
            screen_index: 0,
            security_system_index: -1,
        }
    }
}

impl VLocationSpecial {
    pub fn new() -> Self {
        VLocationSpecial::default()
    }

    pub fn set_screen_index(&mut self, new_screen_index: i32) {
        self.screen_index = new_screen_index;
    }

    pub fn set_security_system_index(&mut self, new_index: i32) {
        self.security_system_index = new_index;
    }
}

/// A location in the Uplink world.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VLocation {
    /// Unique
    pub ip: String,
    /// Name of the Computer here (key into `World::computers`)
    pub computer: String,

    /// Physical location on map
    pub x: i32,
    pub y: i32,
    /// Am I publicly listed?
    pub listed: bool,
    /// Should I be displayed on the world map?
    pub displayed: bool,

    /// `Some` for a `VLocationSpecial` (C++ subclass).
    pub special: Option<VLocationSpecial>,
}

impl Default for VLocation {
    fn default() -> Self {
        VLocation {
            ip: "0.0.0.0".to_string(),
            computer: String::new(),
            x: 0,
            y: 0,
            listed: true,
            displayed: true,
            special: None,
        }
    }
}

impl VLocation {
    pub fn new() -> Self {
        VLocation::default()
    }

    /// A `VLocationSpecial`.
    pub fn new_special() -> Self {
        VLocation {
            special: Some(VLocationSpecial::default()),
            ..VLocation::default()
        }
    }

    pub fn is_special(&self) -> bool {
        self.special.is_some()
    }

    pub fn set_ip(&mut self, newip: &str) {
        self.ip = newip.to_string();
    }

    pub fn set_computer(&mut self, newcomputer: &str) {
        self.computer = newcomputer.to_string();
    }

    pub fn set_plocation(&mut self, newx: i32, newy: i32) {
        debug_assert!(newx >= 0 && newx < VIRTUAL_WIDTH);
        debug_assert!(newy >= 0 && newy < VIRTUAL_HEIGHT);
        self.x = newx;
        self.y = newy;
    }

    pub fn set_listed(&mut self, newlisted: bool) {
        self.listed = newlisted;
    }

    pub fn set_displayed(&mut self, newdisplayed: bool) {
        self.displayed = newdisplayed;
    }

    /// The Computer at this location, or None. Matches on name and ip; a dialup number
    /// (a `VLocationSpecial` pointing at a LAN computer) matches regardless of ip.
    pub fn get_computer<'a>(&self, world: &'a World) -> Option<&'a Computer> {
        // Remember there is no rule that states computer names have to be unique.
        // eg "CompanyName Access Terminal" - there can be many.
        // We must now continue searching until we find a computer that matches
        // both the name and the IP of this VLocation.
        // Also remember that this might be a Dialup Number - in which case its IP
        // (telephone number) will definately NOT match the IP of the computer.
        // (Copies with identical names will always be on the left branch.)
        world.computers.get_all(&self.computer).into_iter().find(|comp| {
            comp.ip == self.ip || (comp.type_ == COMPUTER_TYPE_LAN && self.is_special())
        })
    }
}
