//! Port of source/world/computer/securitysystem.h + source/world/computer/securitysystem.cpp
//!
//! A single security device on a remote computer (firewall, proxy, ...).
//! The SECURITY_TYPE_* values are used as indexes into an array, do not renumber.

use serde::{Deserialize, Serialize};

pub const SECURITY_TYPE_NONE: i32 = 0;
pub const SECURITY_TYPE_PROXY: i32 = 1; // Stops data changes
pub const SECURITY_TYPE_FIREWALL: i32 = 2; // Stops file access
pub const SECURITY_TYPE_ENCRYPTION: i32 = 3; // Stops human readability
pub const SECURITY_TYPE_MONITOR: i32 = 4; // Checks for attacks

const NAMES: [&str; 5] = ["Unknown", "Proxy", "Firewall", "Encrypter", "Monitor"];

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SecuritySystem {
    pub type_: i32,
    pub level: i32, // Difficulty level
    pub enabled: bool,
    pub bypassed: bool,
}

impl SecuritySystem {
    pub fn new() -> Self {
        SecuritySystem {
            type_: SECURITY_TYPE_NONE,
            level: 0,
            enabled: false,
            bypassed: false,
        }
    }

    pub fn set_type(&mut self, new_type: i32) {
        self.type_ = new_type;
    }

    pub fn set_level(&mut self, new_level: i32) {
        self.level = new_level;
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn bypass(&mut self) {
        self.bypassed = true;
    }

    pub fn end_bypass(&mut self) {
        self.bypassed = false;
    }

    pub fn get_name(&self) -> &'static str {
        NAMES
            .get(self.type_.max(0) as usize)
            .copied()
            .unwrap_or("Unknown")
    }

    /// Does nothing in the C++ either.
    pub fn update(&mut self) {}
}
