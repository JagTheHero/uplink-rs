//! Port of source/world/computer/computerscreen/messagescreen.h + source/world/computer/computerscreen/messagescreen.cpp

use serde::{Deserialize, Serialize};

use crate::world::computer::computerscreen::computerscreen::ComputerScreenBase;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MessageScreen {
    pub base: ComputerScreenBase,
    pub nextpage: i32,
    pub textmessage: Option<String>,
    pub buttonmessage: Option<String>,
    pub mailthistome: bool,
}

impl Default for MessageScreen {
    fn default() -> Self {
        MessageScreen {
            base: ComputerScreenBase::default(),
            nextpage: -1,
            textmessage: None,
            buttonmessage: None,
            mailthistome: false,
        }
    }
}

impl MessageScreen {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_next_page(&mut self, newnextpage: i32) {
        self.nextpage = newnextpage;
    }

    pub fn set_text_message(&mut self, newtextmessage: &str) {
        self.textmessage = Some(newtextmessage.to_string());
    }

    pub fn set_button_message(&mut self, newbuttonmessage: &str) {
        self.buttonmessage = Some(newbuttonmessage.to_string());
    }

    pub fn set_mail_this_to_me(&mut self, newvalue: bool) {
        self.mailthistome = newvalue;
    }
}
