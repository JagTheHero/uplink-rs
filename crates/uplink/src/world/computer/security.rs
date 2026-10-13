//! Port of source/world/computer/security.h + source/world/computer/security.cpp
//!
//! Part of every computer system: the security systems in place.

use serde::{Deserialize, Serialize};

use crate::tosser::DArray;
use crate::world::computer::securitysystem::{
    SecuritySystem, SECURITY_TYPE_ENCRYPTION, SECURITY_TYPE_FIREWALL, SECURITY_TYPE_MONITOR,
    SECURITY_TYPE_PROXY,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Security {
    systems: DArray<SecuritySystem>,
}

impl Security {
    pub fn new() -> Self {
        Self::default()
    }

    /// `AddSystem(newsystem, index)`; `index == -1` means the first free slot.
    pub fn add_system(&mut self, newsystem: SecuritySystem, index: i32) {
        if index == -1 {
            self.systems.put_data(newsystem);
        } else {
            self.systems.put_data_at(newsystem, index);
        }
    }

    /// `AddSystem(TYPE, level, index)`: creates, enables and adds a new system.
    pub fn add_system_new(&mut self, type_: i32, level: i32, index: i32) {
        let mut ss = SecuritySystem::new();
        ss.set_type(type_);
        ss.set_level(level);
        ss.enable();
        self.add_system(ss, index);
    }

    pub fn remove_all_systems(&mut self) {
        self.systems.empty();
    }

    pub fn get_system(&self, index: i32) -> Option<&SecuritySystem> {
        self.systems.get_data(index)
    }

    pub fn get_system_mut(&mut self, index: i32) -> Option<&mut SecuritySystem> {
        self.systems.get_data_mut(index)
    }

    /// Total number of slots (`systems.Size()`), used or not.
    pub fn num_systems(&self) -> i32 {
        self.systems.size()
    }

    pub fn num_running_systems(&self) -> i32 {
        self.systems
            .values()
            .filter(|s| s.enabled && !s.bypassed)
            .count() as i32
    }

    // Use these to determine status of security system

    pub fn is_running(&self, type_: i32) -> bool {
        self.systems
            .values()
            .any(|s| s.type_ == type_ && s.enabled && !s.bypassed)
    }

    pub fn is_running_proxy(&self) -> bool {
        self.is_running(SECURITY_TYPE_PROXY)
    }

    pub fn is_running_firewall(&self) -> bool {
        self.is_running(SECURITY_TYPE_FIREWALL)
    }

    pub fn is_running_encryption(&self) -> bool {
        self.is_running(SECURITY_TYPE_ENCRYPTION)
    }

    pub fn is_running_monitor(&self) -> bool {
        self.is_running(SECURITY_TYPE_MONITOR)
    }

    pub fn is_anything_enabled(&self) -> bool {
        self.systems.values().any(|s| s.enabled)
    }

    /// Returns true if any system is disabled.
    pub fn is_anything_disabled(&self) -> bool {
        self.systems.values().any(|s| !s.enabled)
    }

    /// Does nothing in the C++ either.
    pub fn update(&mut self) {}
}
