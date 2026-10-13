//! Port of source/world/computer/bankaccount.h + source/world/computer/bankaccount.cpp
//!
//! A bank account, owned by a `BankComputer` (`Computer::kind`, `accounts` keyed by account number string).

use serde::{Deserialize, Serialize};

use crate::ctx::Ctx;
use crate::game::data::data::TIME_TOCOVERBANKROBBERY;
use crate::world::computer::logbank::{
    AccessLog, LogBank, LOG_NOTSUSPICIOUS, LOG_TYPE_TEXT, LOG_TYPE_TRANSFERFROM,
    LOG_TYPE_TRANSFERTO,
};
use crate::world::date::Date;
use crate::world::person::Person;
use crate::world::scheduler::bankrobberyevent::BankRobberyEvent;
use crate::world::scheduler::uplinkevent::UplinkEvent;
use crate::world::world::World;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct BankAccount {
    /// Owner's name
    pub name: String,
    pub password: String,
    pub security: i32,
    pub accountnumber: i32,
    pub balance: i32,
    pub loan: i32,

    pub log: LogBank,
}

impl BankAccount {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_owner(&mut self, newname: &str) {
        self.name = newname.to_string();
    }

    pub fn set_security(&mut self, newpassword: &str, newsecurity: i32) {
        self.password = newpassword.to_string();
        self.security = newsecurity;
    }

    pub fn set_acc_number(&mut self, newaccnumber: i32) {
        self.accountnumber = newaccnumber;
    }

    pub fn set_balance(&mut self, newbalance: i32, newloan: i32) {
        self.balance = newbalance;
        self.loan = newloan;
    }

    /// Adds `amount` to the balance and logs it (uses the world date), for the account `accno` at
    /// the bank `bank_ip`. C++ default description: None (logged as "Unknown").
    // PORT: C++ `ChangeBalance` is a method on `this`; the account is found by its key (bank ip, account
    // number). A missing account has nothing to change, so nothing happens.
    pub fn change_balance(
        g: &mut Ctx,
        bank_ip: &str,
        accno: &str,
        amount: i32,
        description: Option<&str>,
    ) {
        let date = g.world().date.clone();
        let Some(account) = BankAccount::get_account_mut(g.world_mut(), bank_ip, accno) else {
            return;
        };

        account.balance += amount;

        // Log this action

        let mut source_log = AccessLog::new();
        source_log.set_properties(&date, "unknown", &account.name, LOG_NOTSUSPICIOUS, LOG_TYPE_TEXT);
        source_log.set_data1(Some(description.unwrap_or("Unknown")));
        account.log.add_log(source_log, -1);
    }

    /// Has this account sent money to the target account? (`self` is the source account; the
    /// target is looked up through `world`.)
    pub fn has_transfer_occured(
        &self,
        world: &World,
        s_ip: &str,
        t_ip: &str,
        t_accno: i32,
        amount: i32,
    ) -> bool {
        //
        // Convert integer values to strings
        //

        let amount_s = amount.to_string();
        let s_accno_s = self.accountnumber.to_string();
        let t_accno_s = t_accno.to_string();

        let mut progress = false;

        //
        // Check the source account (this)
        //

        for (_, al) in self.log.internallogs.iter() {
            if al.type_ == LOG_TYPE_TRANSFERTO {
                let data1 = al.data1.as_deref().expect("UplinkAssert(al->data1)");
                let data2 = al.data2.as_deref().expect("UplinkAssert(al->data2)");

                if data1.contains(t_ip)
                    && data1.contains(t_accno_s.as_str())
                    && data2 == amount_s.as_str()
                {
                    // This is the correct log in the source account

                    progress = true;
                    break;
                }
            }
        }

        if !progress {
            return false;
        }
        progress = false;

        //
        // Check the target account
        //

        let Some(target_acc) = BankAccount::get_account(world, t_ip, &t_accno_s) else {
            return false;
        };

        for (_, al) in target_acc.log.internallogs.iter() {
            if al.type_ == LOG_TYPE_TRANSFERFROM {
                let data1 = al.data1.as_deref().expect("UplinkAssert(al->data1)");
                let data2 = al.data2.as_deref().expect("UplinkAssert(al->data2)");

                if data1.contains(s_ip)
                    && data1.contains(s_accno_s.as_str())
                    && data2 == amount_s.as_str()
                {
                    // This is the correct log in the target account

                    progress = true;
                    break;
                }
            }
        }

        progress
    }

    /// `HasTransferOccured(char *person, int amount)`: has this account received payment from `person`?
    pub fn has_transfer_occured_from(&self, person: &str, amount: i32) -> bool {
        let amount_s = amount.to_string();

        //
        // Go throught each transfer trying to find a match for the amount
        //

        for (_, al) in self.log.internallogs.iter() {
            if al.type_ == LOG_TYPE_TRANSFERFROM {
                let data2 = al.data2.as_deref().expect("UplinkAssert(al->data2)");
                let data3 = al.data3.as_deref().expect("UplinkAssert(al->data3)");

                if data2 == amount_s.as_str() && data3 == person {
                    // This is it
                    return true;
                }
            }
        }

        false
    }

    pub fn get_person<'a>(&self, world: &'a World) -> Option<&'a Person> {
        world.get_person(&self.name)
    }

    /// None if the ip is not a bank computer or the account does not exist.
    pub fn get_account<'a>(world: &'a World, bankip: &str, accno: &str) -> Option<&'a BankAccount> {
        let vl = world.get_vlocation(bankip)?;
        let comp = vl.get_computer(world)?;
        let bank = comp.bank()?;
        bank.accounts.get(accno)
    }

    pub fn get_account_mut<'a>(
        world: &'a mut World,
        bankip: &str,
        accno: &str,
    ) -> Option<&'a mut BankAccount> {
        // Find the computer's name first (immutable), then borrow it mutably.
        let comp_name = world
            .get_vlocation(bankip)?
            .get_computer(world)?
            .name
            .clone();
        let bank = world.get_computer_mut(&comp_name)?.bank_mut()?;
        bank.accounts.get_mut(accno)
    }

    /// `person_name` is the name of the Person who initiates the transfer ("PLAYER" for the player).
    /// Schedules a BankRobberyEvent when the player robs money, logs both sides. Returns success.
    pub fn transfer_money(
        g: &mut Ctx,
        source_ip: &str,
        source_accno: &str,
        target_ip: &str,
        target_accno: &str,
        amount: i32,
        person_name: &str,
    ) -> bool {
        // UplinkAssert(person): the person must exist. `person->name` is the name the logs use, and
        // `person == GetPlayer()` is an identity test, so compare the object itself.
        let (person_name_actual, is_player) = {
            let world = g.world();
            let person = world
                .get_person(person_name)
                .expect("UplinkAssert(person)");
            let is_player = world
                .player
                .as_deref()
                .is_some_and(|p| std::ptr::eq(person, &p.person));
            (person.name.clone(), is_player)
        };

        let source_acc = BankAccount::get_account(g.world(), source_ip, source_accno)
            .map(|a| (a.balance, a.name.clone()));
        let target_acc_name = BankAccount::get_account(g.world(), target_ip, target_accno)
            .map(|a| a.name.clone());

        let source_data = format!("{} {}", target_ip, target_accno);
        let target_data = format!("{} {}", source_ip, source_accno);

        let s_amount = amount.to_string();

        //
        // If the player is trying to rob money,
        // Make sure we know about it

        let robbery = {
            let player = g.world().get_player();
            !player.is_player_account(source_ip, source_accno)
                && player.is_player_account(target_ip, target_accno)
                && is_player
        };
        if robbery {
            let date = g.world().date.clone();

            let mut rundate = Date::default();
            rundate.set_date_from(&date);
            rundate.advance_minute(TIME_TOCOVERBANKROBBERY);

            let mut bre = BankRobberyEvent::new();
            bre.set_details(
                source_ip,
                source_accno,
                target_ip,
                target_accno,
                amount,
                &date,
            );
            bre.base.set_run_date(&rundate);

            g.world_mut()
                .scheduler
                .schedule_event(UplinkEvent::BankRobberyEvent(bre));
        }

        let (Some((source_balance, source_name)), Some(target_name)) =
            (source_acc, target_acc_name)
        else {
            return false;
        };

        if amount <= 0 || source_balance < amount {
            return false;
        }

        // Change the accounts (the source and target may be the same account, so do it in turn)

        BankAccount::get_account_mut(g.world_mut(), source_ip, source_accno)
            .expect("transfer_money: source account")
            .balance -= amount;
        BankAccount::get_account_mut(g.world_mut(), target_ip, target_accno)
            .expect("transfer_money: target account")
            .balance += amount;

        let date = g.world().date.clone();

        // Log on the source account
        let mut source_log = AccessLog::new();
        source_log.set_properties(
            &date,
            source_ip,
            &person_name_actual,
            LOG_NOTSUSPICIOUS,
            LOG_TYPE_TRANSFERTO,
        );
        source_log.set_data1(Some(&source_data));
        source_log.set_data2(Some(&s_amount));
        source_log.set_data3(Some(&target_name));
        BankAccount::get_account_mut(g.world_mut(), source_ip, source_accno)
            .expect("transfer_money: source account")
            .log
            .add_log(source_log, -1);

        // Log on the target account
        let mut target_log = AccessLog::new();
        target_log.set_properties(
            &date,
            source_ip,
            &person_name_actual,
            LOG_NOTSUSPICIOUS,
            LOG_TYPE_TRANSFERFROM,
        );
        target_log.set_data1(Some(&target_data));
        target_log.set_data2(Some(&s_amount));
        target_log.set_data3(Some(&source_name));
        BankAccount::get_account_mut(g.world_mut(), target_ip, target_accno)
            .expect("transfer_money: target account")
            .log
            .add_log(target_log, -1);

        true
    }
}
