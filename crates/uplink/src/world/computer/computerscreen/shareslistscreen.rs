//! Port of source/world/computer/computerscreen/shareslistscreen.h + source/world/computer/computerscreen/shareslistscreen.cpp

use serde::{Deserialize, Serialize};

use crate::world::computer::computerscreen::computerscreen::ComputerScreenBase;

pub const SHARESLISTSCREENTYPE_NONE: i32 = 0;
pub const SHARESLISTSCREENTYPE_ALLSHARES: i32 = 1;
pub const SHARESLISTSCREENTYPE_USERSHARES: i32 = 2;
pub const SHARESLISTSCREENTYPE_SPECIFICSHARES: i32 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SharesListScreen {
    pub base: ComputerScreenBase,
    pub nextpage: i32,
    pub viewpage: i32,
    pub screentype: i32,
}

impl Default for SharesListScreen {
    fn default() -> Self {
        // PORT: the C++ constructor left `viewpage` uninitialised; -1 (no page) is used here.
        SharesListScreen {
            base: ComputerScreenBase::default(),
            nextpage: -1,
            viewpage: -1,
            screentype: SHARESLISTSCREENTYPE_NONE,
        }
    }
}

impl SharesListScreen {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_next_page(&mut self, newnextpage: i32) {
        self.nextpage = newnextpage;
    }

    pub fn set_view_page(&mut self, newviewpage: i32) {
        self.viewpage = newviewpage;
    }

    pub fn set_screen_type(&mut self, new_screen_type: i32) {
        self.screentype = new_screen_type;
    }
}
