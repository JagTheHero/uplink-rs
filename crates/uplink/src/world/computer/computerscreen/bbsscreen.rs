//! Port of source/world/computer/computerscreen/bbsscreen.h + source/world/computer/computerscreen/bbsscreen.cpp

use serde::{Deserialize, Serialize};

use crate::world::computer::computerscreen::computerscreen::ComputerScreenBase;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BBSScreen {
    pub base: ComputerScreenBase,
    pub nextpage: i32,
    pub contactpage: i32,
}

impl Default for BBSScreen {
    fn default() -> Self {
        BBSScreen {
            base: ComputerScreenBase::default(),
            nextpage: -1,
            contactpage: -1,
        }
    }
}

impl BBSScreen {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_next_page(&mut self, newnextpage: i32) {
        self.nextpage = newnextpage;
    }

    pub fn set_contact_page(&mut self, newcontactpage: i32) {
        self.contactpage = newcontactpage;
    }
}
