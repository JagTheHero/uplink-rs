//! Port of source/world/computer/computerscreen/linksscreen.h + source/world/computer/computerscreen/linksscreen.cpp

use serde::{Deserialize, Serialize};

use crate::world::computer::computerscreen::computerscreen::ComputerScreenBase;

pub const LINKSSCREENTYPE_NONE: i32 = 0;
pub const LINKSSCREENTYPE_ALLLINKS: i32 = 1;
pub const LINKSSCREENTYPE_PLAYERLINKS: i32 = 2;
pub const LINKSSCREENTYPE_LOCALLINKS: i32 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LinksScreen {
    pub base: ComputerScreenBase,
    pub nextpage: i32,
    pub screentype: i32,
}

impl Default for LinksScreen {
    fn default() -> Self {
        LinksScreen {
            base: ComputerScreenBase::default(),
            nextpage: -1,
            screentype: LINKSSCREENTYPE_NONE,
        }
    }
}

impl LinksScreen {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_next_page(&mut self, newnextpage: i32) {
        self.nextpage = newnextpage;
    }

    pub fn set_screen_type(&mut self, new_screen_type: i32) {
        self.screentype = new_screen_type;
    }
}
