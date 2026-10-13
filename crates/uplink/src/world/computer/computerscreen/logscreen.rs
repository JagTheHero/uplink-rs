//! Port of source/world/computer/computerscreen/logscreen.h + source/world/computer/computerscreen/logscreen.cpp

use serde::{Deserialize, Serialize};

use crate::ctx::Ctx;
use crate::objref::ObjRef;
use crate::world::computer::computer::Computer;
use crate::world::computer::computerscreen::computerscreen::ComputerScreenBase;
use crate::world::computer::logbank::LogBank;
use crate::world::computer::recordbank::{RecordBank, RECORDBANK_ACCNO};

pub const LOGSCREEN_TARGET_UNKNOWN: i32 = -1;
pub const LOGSCREEN_TARGET_ACCESSLOGS: i32 = 0;
/// MUST be on a Bank Computer and logged in
pub const LOGSCREEN_TARGET_ACCOUNTLOGS: i32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LogScreen {
    pub base: ComputerScreenBase,
    pub nextpage: i32,
    pub target: i32,
}

impl Default for LogScreen {
    fn default() -> Self {
        LogScreen {
            base: ComputerScreenBase::default(),
            nextpage: -1,
            target: LOGSCREEN_TARGET_UNKNOWN,
        }
    }
}

impl LogScreen {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_next_page(&mut self, newnextpage: i32) {
        self.nextpage = newnextpage;
    }

    /// `SetTARGET`
    pub fn set_target(&mut self, new_target: i32) {
        self.target = new_target;
    }

    /// `GetTargetLogBank`: asserts the log bank exists (access logs of the player's remote host, or
    /// the logged-in bank account's log). Reads `g` (world, player remotehost, remote interface
    /// `security_name`).
    pub fn get_target_log_bank<'a>(&self, g: &'a Ctx) -> &'a LogBank {
        match self.target {
            LOGSCREEN_TARGET_ACCESSLOGS => {
                let comp = target_computer(g);
                &comp.logbank
            }

            LOGSCREEN_TARGET_ACCOUNTLOGS => {
                // Get the bank computer
                let comp = target_computer(g);
                let bank = comp
                    .bank()
                    .expect("LogScreen: account logs need a bank computer");

                // Get the account number
                let accno = account_number_for(&comp.recordbank, &g.interface().ri.security_name);

                // Now look up the account based on that account number
                let account = bank
                    .accounts
                    .get(&accno)
                    .expect("LogScreen: bank account not found");

                &account.log
            }

            LOGSCREEN_TARGET_UNKNOWN => panic!("Unknown LogScreen Target"),

            other => panic!("Unrecognised LogScreen Target:{}", other),
        }
    }

    /// Reference to the log bank [`LogScreen::get_target_log_bank`] returns (what the C++ passes on as
    /// the `LogBank *` task target).
    pub fn target_log_ref(&self, g: &Ctx) -> ObjRef {
        let comp = target_computer(g);
        match self.target {
            LOGSCREEN_TARGET_ACCOUNTLOGS => ObjRef::AccountLog {
                computer: comp.name.clone(),
                accno: account_number_for(&comp.recordbank, &g.interface().ri.security_name),
            },
            _ => ObjRef::LogBank(comp.name.clone()),
        }
    }

    /// Mutable variant of [`LogScreen::get_target_log_bank`], keyed by the screen's `target`.
    pub fn get_target_log_bank_mut(target: i32, g: &mut Ctx) -> &mut LogBank {
        match target {
            LOGSCREEN_TARGET_ACCESSLOGS => {
                let name = target_computer_name(g);
                let comp = g
                    .world_mut()
                    .get_computer_mut(&name)
                    .expect("LogScreen: target computer not found");
                &mut comp.logbank
            }

            LOGSCREEN_TARGET_ACCOUNTLOGS => {
                let name = target_computer_name(g);
                let security_name = g.interface().ri.security_name.clone();
                let comp = g
                    .world_mut()
                    .get_computer_mut(&name)
                    .expect("LogScreen: target computer not found");
                let accno = account_number_for(&comp.recordbank, &security_name);
                let bank = comp
                    .bank_mut()
                    .expect("LogScreen: account logs need a bank computer");
                let account = bank
                    .accounts
                    .get_mut(&accno)
                    .expect("LogScreen: bank account not found");
                &mut account.log
            }

            LOGSCREEN_TARGET_UNKNOWN => panic!("Unknown LogScreen Target"),

            other => panic!("Unrecognised LogScreen Target:{}", other),
        }
    }
}

/// Name of the computer at the player's remote host (`VLocation::computer`). C++ asserts it exists.
fn target_computer_name(g: &Ctx) -> String {
    let remotehost = &g.player().remotehost;
    let vl = g
        .world()
        .get_vlocation(remotehost)
        .expect("LogScreen: player's remote host not found");
    vl.computer.clone()
}

/// `game->GetWorld()->GetComputer(vl->computer)` for the player's remote host. C++ asserts it exists.
fn target_computer(g: &Ctx) -> &Computer {
    let name = target_computer_name(g);
    g.world()
        .get_computer(&name)
        .expect("LogScreen: target computer not found")
}

/// `rec->GetField(RECORDBANK_ACCNO)` of the record called `security_name`. C++ asserts both exist.
fn account_number_for(recordbank: &RecordBank, security_name: &str) -> String {
    let rec = recordbank
        .get_record_from_name(security_name)
        .expect("LogScreen: no record for the logged-in account");
    rec.get_field(RECORDBANK_ACCNO)
        .expect("LogScreen: record has no account number")
        .to_string()
}
