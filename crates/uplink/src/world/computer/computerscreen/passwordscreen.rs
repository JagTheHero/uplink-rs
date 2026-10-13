//! Port of source/world/computer/computerscreen/passwordscreen.h + source/world/computer/computerscreen/passwordscreen.cpp

use serde::{Deserialize, Serialize};

use crate::world::computer::computerscreen::computerscreen::ComputerScreenBase;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PasswordScreen {
    pub base: ComputerScreenBase,
    pub nextpage: i32,
    /// 1 = easy, >1 = getting harder
    pub difficulty: i32,
    pub password: String,
}

impl Default for PasswordScreen {
    fn default() -> Self {
        PasswordScreen {
            base: ComputerScreenBase::default(),
            nextpage: -1,
            difficulty: 1,
            password: String::new(),
        }
    }
}

impl PasswordScreen {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_next_page(&mut self, newnextpage: i32) {
        self.nextpage = newnextpage;
    }

    pub fn set_password(&mut self, newpassword: &str) {
        self.password = newpassword.to_string();
    }

    pub fn set_difficulty(&mut self, newdifficulty: i32) {
        self.difficulty = newdifficulty;
    }
}
