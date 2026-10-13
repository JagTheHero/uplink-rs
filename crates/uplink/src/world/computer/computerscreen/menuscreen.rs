//! Port of source/world/computer/computerscreen/menuscreen.h + source/world/computer/computerscreen/menuscreen.cpp

use serde::{Deserialize, Serialize};

use crate::world::computer::computerscreen::computerscreen::ComputerScreenBase;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MenuScreen {
    pub base: ComputerScreenBase,
    pub options: Vec<MenuScreenOption>,
}

impl MenuScreen {
    pub fn new() -> Self {
        Self::default()
    }

    /// `AddOption`; C++ defaults are `security = 10`, `index = -1` (append).
    pub fn add_option(
        &mut self,
        caption: &str,
        tooltip: &str,
        nextpage: i32,
        security: i32,
        index: i32,
    ) {
        let mut mso = MenuScreenOption::new();
        mso.set_caption(caption);
        mso.set_tooltip(tooltip);
        mso.set_next_page(nextpage);
        mso.set_security(security);

        if index == -1 {
            self.options.push(mso);
        } else if index >= 0 && (index as usize) <= self.options.len() {
            self.options.insert(index as usize, mso);
        }
        // PORT: C++ LList::PutDataAtIndex silently drops the item when index runs past the end.
    }

    pub fn num_options(&self) -> i32 {
        self.options.len() as i32
    }

    pub fn get_caption(&self, index: i32) -> &str {
        &self.options[index as usize].caption
    }

    pub fn get_tooltip(&self, index: i32) -> &str {
        &self.options[index as usize].tooltip
    }

    pub fn get_next_page(&self, index: i32) -> i32 {
        self.options[index as usize].nextpage
    }

    pub fn get_security(&self, index: i32) -> i32 {
        self.options[index as usize].security
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MenuScreenOption {
    pub caption: String,
    pub tooltip: String,
    pub nextpage: i32,
    pub security: i32,
}

impl Default for MenuScreenOption {
    fn default() -> Self {
        MenuScreenOption {
            caption: String::new(),
            tooltip: String::new(),
            nextpage: -1,
            security: 10,
        }
    }
}

impl MenuScreenOption {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_caption(&mut self, newcaption: &str) {
        self.caption = newcaption.to_string();
    }

    pub fn set_tooltip(&mut self, newtooltip: &str) {
        self.tooltip = newtooltip.to_string();
    }

    pub fn set_next_page(&mut self, newnextpage: i32) {
        self.nextpage = newnextpage;
    }

    pub fn set_security(&mut self, newsecurity: i32) {
        self.security = newsecurity;
    }

    /// `Update` (does nothing)
    pub fn update(&mut self) {}
}
