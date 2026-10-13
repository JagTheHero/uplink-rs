//! Port of source/world/computer/computerscreen/disconnectedscreen.h + source/world/computer/computerscreen/disconnectedscreen.cpp

use std::cell::RefCell;

use serde::{Deserialize, Serialize};

use crate::world::computer::computerscreen::computerscreen::ComputerScreenBase;

thread_local! {
    /// C++ `static DArray<char *> DisconnectedScreen::loginslost`: IPs whose logins were revoked.
    /// PORT: kept as a (single-threaded) module-level list instead of a `Ctx` field; it is not saved.
    static LOGINSLOST: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DisconnectedScreen {
    pub base: ComputerScreenBase,
    pub nextpage: i32,
    pub textmessage: Option<String>,
}

impl Default for DisconnectedScreen {
    fn default() -> Self {
        DisconnectedScreen {
            base: ComputerScreenBase::default(),
            nextpage: -1,
            textmessage: None,
        }
    }
}

impl DisconnectedScreen {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_next_page(&mut self, newnextpage: i32) {
        self.nextpage = newnextpage;
    }

    pub fn set_text_message(&mut self, newtextmessage: &str) {
        self.textmessage = Some(newtextmessage.to_string());
    }

    /// `AddLoginLost`
    pub fn add_login_lost(ip: &str) {
        LOGINSLOST.with(|l| l.borrow_mut().push(ip.to_string()));
    }

    /// `ClearLoginsLost`
    pub fn clear_logins_lost() {
        LOGINSLOST.with(|l| l.borrow_mut().clear());
    }

    /// Snapshot of `loginslost` (the C++ read the static array directly).
    pub fn logins_lost() -> Vec<String> {
        LOGINSLOST.with(|l| l.borrow().clone())
    }
}
