//! Port of source/world/computer/computerscreen/highsecurityscreen.h + source/world/computer/computerscreen/highsecurityscreen.cpp

use serde::{Deserialize, Serialize};

use crate::world::computer::computerscreen::computerscreen::ComputerScreenBase;
use crate::world::computer::computerscreen::menuscreen::MenuScreenOption;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HighSecurityScreen {
    pub base: ComputerScreenBase,
    pub systems: Vec<MenuScreenOption>,
    pub nextpage: i32,
}

impl Default for HighSecurityScreen {
    fn default() -> Self {
        HighSecurityScreen {
            base: ComputerScreenBase::default(),
            systems: Vec::new(),
            nextpage: -1,
        }
    }
}

impl HighSecurityScreen {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_system(&mut self, name: &str, page: i32) {
        let mut mso = MenuScreenOption::new();
        mso.set_caption(name);
        mso.set_next_page(page);
        mso.set_security(10);
        self.systems.push(mso);
    }

    pub fn set_next_page(&mut self, newnextpage: i32) {
        self.nextpage = newnextpage;
    }

    /// Sets all systems to "Enabled" (security 10).
    pub fn reset_security(&mut self) {
        for system in self.systems.iter_mut() {
            system.security = 10;
        }
    }
}
