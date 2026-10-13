//! Port of source/world/computer/computerscreen/genericscreen.h + source/world/computer/computerscreen/genericscreen.cpp

use serde::{Deserialize, Serialize};

use crate::interface::remoteinterface::remoteinterface::SCREEN_UNKNOWN;
use crate::world::computer::computerscreen::computerscreen::ComputerScreenBase;

/// Screens with no data of their own (e.g. the file server screen); `screen_type` is a `SCREEN_*` id.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GenericScreen {
    pub base: ComputerScreenBase,
    pub nextpage: i32,
    pub screen_type: i32,
}

impl Default for GenericScreen {
    fn default() -> Self {
        GenericScreen {
            base: ComputerScreenBase::default(),
            nextpage: -1,
            screen_type: SCREEN_UNKNOWN,
        }
    }
}

impl GenericScreen {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_next_page(&mut self, newnextpage: i32) {
        self.nextpage = newnextpage;
    }

    pub fn set_screen_type(&mut self, new_screen_type: i32) {
        self.screen_type = new_screen_type;
    }
}
