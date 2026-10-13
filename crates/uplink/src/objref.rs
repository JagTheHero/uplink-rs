//! Serialisable references to objects inside the world.
//!
//! The C++ passed raw `UplinkObject *` / `ComputerScreen *` / `DataBank *` pointers around
//! (task targets, the remote screen's computer screen, ...). In Rust these are keys that are
//! resolved against the `World` when needed.

use serde::{Deserialize, Serialize};

use crate::world::computer::logbank::LogBank;
use crate::world::world::World;

/// A computer screen: `computer.screens[index]`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ScreenRef {
    pub computer: String,
    pub index: i32,
}

impl ScreenRef {
    pub fn new(computer: &str, index: i32) -> Self {
        ScreenRef { computer: computer.to_string(), index }
    }
}

/// Who owns a `DataBank`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataBankOwner {
    /// The player's gateway memory (`player.gateway.databank`).
    Gateway,
    /// `world.computers[name].databank`
    Computer(String),
}

/// Replacement for a generic `UplinkObject *` target (e.g. `TaskManager::SetProgramTarget`).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ObjRef {
    #[default]
    None,
    /// A whole computer (`Computer *`, including `LanComputer *`).
    Computer(String),
    /// `ComputerScreen *` (password screen, user id screen, cypher screen, ...).
    ComputerScreen(ScreenRef),
    /// `DataBank *` (file server or gateway memory).
    DataBank(DataBankOwner),
    /// `LogBank *` of a computer.
    LogBank(String),
    /// `LogBank *` of a bank account (`account->log`): computer name + account number.
    AccountLog { computer: String, accno: String },
    /// `Security *` of a computer.
    Security(String),
    /// `RecordBank *` of a computer.
    RecordBank(String),
    /// A person by name ("PLAYER" for the player).
    Person(String),
}

impl ObjRef {
    pub fn is_none(&self) -> bool {
        matches!(self, ObjRef::None)
    }

    /// The `LogBank` this reference names (a computer's logbank or a bank account's log).
    pub fn log_bank<'a>(&self, world: &'a World) -> Option<&'a LogBank> {
        match self {
            ObjRef::LogBank(name) => world.get_computer(name).map(|c| &c.logbank),
            ObjRef::AccountLog { computer, accno } => {
                world.get_computer(computer)?.bank()?.accounts.get(accno).map(|a| &a.log)
            }
            _ => None,
        }
    }

    pub fn log_bank_mut<'a>(&self, world: &'a mut World) -> Option<&'a mut LogBank> {
        match self {
            ObjRef::LogBank(name) => world.get_computer_mut(name).map(|c| &mut c.logbank),
            ObjRef::AccountLog { computer, accno } => {
                world.get_computer_mut(computer)?.bank_mut()?.accounts.get_mut(accno).map(|a| &mut a.log)
            }
            _ => None,
        }
    }

    /// C++ `uo->GetOBJECTID() == OID_LOGBANK`.
    pub fn is_log_bank(&self) -> bool {
        matches!(self, ObjRef::LogBank(_) | ObjRef::AccountLog { .. })
    }
}
