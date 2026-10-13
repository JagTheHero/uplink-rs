//! Port of source/world/computer/computerscreen/useridscreen.h + source/world/computer/computerscreen/useridscreen.cpp

use serde::{Deserialize, Serialize};

use crate::world::computer::computerscreen::computerscreen::ComputerScreenBase;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UserIDScreen {
    pub base: ComputerScreenBase,
    pub nextpage: i32,
    /// 1 = easy, >1 = getting harder
    pub difficulty: i32,
}

impl Default for UserIDScreen {
    fn default() -> Self {
        UserIDScreen {
            base: ComputerScreenBase::default(),
            nextpage: -1,
            difficulty: 1,
        }
    }
}

impl UserIDScreen {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_next_page(&mut self, newnextpage: i32) {
        self.nextpage = newnextpage;
    }

    pub fn set_difficulty(&mut self, newdifficulty: i32) {
        self.difficulty = newdifficulty;
    }
}
