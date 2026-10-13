//! Port of source/world/computer/bankcomputer.h + source/world/computer/bankcomputer.cpp
//!
//! PORT (§3.3): `BankComputer` holds only the extra fields; it lives in `Computer::kind`
//! (`ComputerKind::BankComputer`). `BankComputer::Update` just calls `Computer::Update`, so it has
//! no Rust counterpart. Methods that also touch the Computer base (the record bank) are `impl Computer`.

use serde::{Deserialize, Serialize};

use crate::tosser::BTree;
use crate::world::computer::bankaccount::BankAccount;
use crate::world::computer::computer::{Computer, ComputerKind};
use crate::world::computer::recordbank::{
    Record, RECORDBANK_ACCNO, RECORDBANK_NAME, RECORDBANK_PASSWORD, RECORDBANK_SECURITY,
};
use crate::world::generator::numbergenerator::NumberGenerator;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct BankComputer {
    /// (protected in C++)
    pub previousaccno: i32,

    /// Keyed by account number as a decimal string.
    pub accounts: BTree<BankAccount>,
}

impl BankComputer {
    /// Starts the account number sequence at a random number.
    pub fn new() -> Self {
        BankComputer {
            previousaccno: NumberGenerator::random_number(999999),
            accounts: BTree::new(),
        }
    }

    pub fn generate_unique_account_number(&mut self) -> i32 {
        // C++ int arithmetic wraps in practice; match that rather than panicking in debug builds.
        self.previousaccno = self
            .previousaccno
            .wrapping_add(NumberGenerator::random_number(999999));

        let mut previousaccno_s = self.previousaccno.to_string();
        while self.accounts.contains_key(&previousaccno_s) {
            self.previousaccno = self
                .previousaccno
                .wrapping_add(NumberGenerator::random_number(999999));
            previousaccno_s = self.previousaccno.to_string();
        }

        self.previousaccno
    }

    pub fn close_account(&mut self, accno: i32) {
        // TODO in the C++ too: does nothing.
        let _ = accno;
    }

    /// A random entry of `accounts.ConvertToDArray` (BTree pre-order).
    pub fn get_random_account(&self) -> Option<&BankAccount> {
        let size = self.accounts.size();
        if size == 0 {
            return None;
        }

        let index = NumberGenerator::random_number(size);
        self.accounts.values().nth(index as usize)
    }

    pub fn get_random_account_mut(&mut self) -> Option<&mut BankAccount> {
        let size = self.accounts.size();
        if size == 0 {
            return None;
        }

        let index = NumberGenerator::random_number(size);
        self.accounts.values_mut().nth(index as usize)
    }
}

impl Computer {
    /// A new `Computer` of kind BankComputer.
    pub fn new_bank_computer() -> Computer {
        Computer {
            kind: ComputerKind::BankComputer(BankComputer::new()),
            ..Computer::new()
        }
    }

    /// `CreateBankAccount(BankAccount *newaccount)`: stores the account and adds its access record to
    /// the record bank. Returns the account number. Must be a bank computer.
    pub fn create_bank_account(&mut self, newaccount: BankAccount) -> i32 {
        if newaccount.accountnumber == 0 {
            println!("BankComputer::CreateBankAccount WARNING : AccountNumber = 0");
        }

        let accno = newaccount.accountnumber;
        let s_account = accno.to_string();

        // Create the access record

        let mut rec = Record::new();
        // C++ kept the owner name out of the record (`AddField("Name", name)` is commented out),
        // and stored the account number in the Name field.
        rec.add_field_int(RECORDBANK_NAME, accno);
        rec.add_field(RECORDBANK_PASSWORD, &newaccount.password);
        rec.add_field_int(RECORDBANK_SECURITY, newaccount.security);
        rec.add_field_int(RECORDBANK_ACCNO, accno);

        let ComputerKind::BankComputer(bank) = &mut self.kind else {
            panic!("CreateBankAccount: not a bank computer");
        };
        debug_assert!(!bank.accounts.contains_key(&s_account));
        bank.accounts.put_data(&s_account, newaccount);

        self.recordbank.add_record_sorted(rec, RECORDBANK_NAME);

        accno
    }

    /// `CreateBankAccount(name, password, security, balance, loan)`: builds the account with a unique
    /// account number and adds it. Returns the account number assigned. Must be a bank computer.
    pub fn create_bank_account_new(
        &mut self,
        name: &str,
        password: &str,
        security: i32,
        balance: i32,
        loan: i32,
    ) -> i32 {
        let mut ba = BankAccount::new();
        ba.set_owner(name);
        ba.set_security(password, security);
        ba.set_balance(balance, loan);

        let accno = self
            .bank_mut()
            .expect("CreateBankAccount: not a bank computer")
            .generate_unique_account_number();
        ba.set_acc_number(accno);

        self.create_bank_account(ba)
    }
}
