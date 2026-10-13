//! Port of source/interface/remoteinterface/accountscreen_interface.h + source/interface/remoteinterface/accountscreen_interface.cpp

use std::time::{SystemTime, UNIX_EPOCH};

use crate::app::opengl_interface::textbutton_draw;
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::game::data::data::{
    LARGELOAN_INTEREST, LARGELOAN_MAX, MAXLOAN_INTEREST, MEDIUMLOAN_INTEREST, MEDIUMLOAN_MAX,
    SMALLLOAN_INTEREST, SMALLLOAN_MAX,
};
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::remoteinterface::remoteinterface::{RemoteInterface, SCREEN_ACCOUNTSCREEN};
use crate::interface::remoteinterface::remoteinterfacescreen::{draw_main_title, draw_sub_title};
use crate::objref::ScreenRef;
use crate::world::computer::bankaccount::BankAccount;
use crate::world::computer::computerscreen::genericscreen::GenericScreen;
use crate::world::computer::recordbank::RECORDBANK_ACCNO;

/// Bank account summary screen shown on a bank computer.
#[derive(Debug, Default)]
pub struct AccountScreenInterface {
    pub cs: Option<ScreenRef>,
    /// `BankAccount *account`: non-owning; stored as the key into the bank computer's
    /// `accounts` map (the account number string). `None` = no account for this user.
    pub account: Option<String>,
    pub lastupdate: i64,
}

/// `time(NULL)`.
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// `int(100.0 * LOAN_INTEREST)`: the C++ constants are double literals (0.2, 0.4, 0.7, 1.0), but
/// `data.rs` stores them as f32. Widening an f32 directly would turn 0.7 into 0.699999988 and give 69
/// instead of 70, so go through the shortest decimal form to recover the double literal.
fn apr_percent(interest: f32) -> i32 {
    let interest: f64 = interest.to_string().parse().unwrap_or(interest as f64);
    (100.0_f64 * interest) as i32
}

/// `RunScreen(gs->nextpage)` when the generic screen has a next page (not -1).
fn run_next_page(g: &mut Ctx, nextpage: Option<i32>) {
    if let Some(nextpage) = nextpage {
        if nextpage != -1 {
            RemoteInterface::run_screen(g, nextpage);
        }
    }
}

/// `EclGetButton(name)->SetCaption(caption)`.
fn set_caption(g: &mut Ctx, name: &str, caption: &str) {
    if let Some(b) = g.ecl.get_button_mut(name) {
        b.set_caption(caption);
    }
}

impl AccountScreenInterface {
    /// `GetComputerScreen` (typed accessor; the C++ casts `cs`).
    pub fn get_computer_screen<'a>(&self, g: &'a Ctx) -> Option<&'a GenericScreen> {
        let r = self.cs.as_ref()?;
        g.world().get_computer(&r.computer)?.get_computer_screen(r.index)?.as_generic_screen()
    }

    /// The bank account this screen shows (`account`), looked up by its key on the bank computer.
    fn bank_account<'a>(&self, g: &'a Ctx) -> Option<&'a BankAccount> {
        let key = self.account.as_ref()?;
        let gs = self.get_computer_screen(g)?;
        g.world().get_computer(&gs.base.computer)?.bank()?.accounts.get(key)
    }

    /// `CloseClick` (static MouseFn)
    pub fn close_click(g: &mut Ctx, _button: &Button) {
        let nextpage = RemoteInterface::get_computer_screen(g)
            .and_then(|s| s.as_generic_screen())
            .map(|gs| gs.nextpage);
        run_next_page(g, nextpage);
    }

    /// `Update`: refreshes balance/loan captions at most every 5 seconds.
    pub fn update_balance(&mut self, g: &mut Ctx) {
        // Update every 5 seconds
        // (If there is an account to update)

        if now() > self.lastupdate + 5 {
            let figures = self.bank_account(g).map(|a| (a.balance, a.loan));

            if let Some((balance, loan)) = figures {
                // Update the balance

                set_caption(g, "accountscreen_balance", &balance.to_string());

                // Update the loan

                set_caption(g, "accountscreen_loan", &loan.to_string());

                // Update the interest rate

                let percent = if loan <= SMALLLOAN_MAX {
                    apr_percent(SMALLLOAN_INTEREST)
                } else if loan <= MEDIUMLOAN_MAX {
                    apr_percent(MEDIUMLOAN_INTEREST)
                } else if loan <= LARGELOAN_MAX {
                    apr_percent(LARGELOAN_INTEREST)
                } else {
                    apr_percent(MAXLOAN_INTEREST)
                };

                set_caption(g, "accountscreen_loanrate", &format!("({}% apr)", percent));
            }

            self.lastupdate = now();
        }
    }
}

impl InterfaceScreen for AccountScreenInterface {
    crate::impl_as_any!();

    /// `Create(ComputerScreen *newcs)` body (the `cs` is already set by `set_computer_screen`).
    fn create(&mut self, g: &mut Ctx) {
        let Some(gs) = self.get_computer_screen(g).cloned() else { return };

        // Look up the account number based on the log in name

        let security_name = g.interface().ri.security_name.clone();
        let accno = g
            .world()
            .get_computer(&gs.base.computer)
            .and_then(|comp| comp.recordbank.get_record_from_name(&security_name))
            .and_then(|rec| rec.get_field(RECORDBANK_ACCNO))
            .map(str::to_string);

        let Some(accno) = accno else {
            self.account = None;
            return;
        };

        // Now look up the account based on that account number

        let exists = g
            .world()
            .get_computer(&gs.base.computer)
            .and_then(|comp| comp.bank())
            .is_some_and(|bank| bank.accounts.contains_key(&accno));
        self.account = exists.then_some(accno);

        if !self.is_visible(g) {
            g.ecl.register_button(80, 60, 350, 25, &gs.base.maintitle, "", "accountscreen_maintitle");
            g.ecl.register_button_callbacks("accountscreen_maintitle", Some(draw_main_title), None, None, None);
            g.ecl.register_button(80, 80, 350, 20, &gs.base.subtitle, "", "accountscreen_subtitle");
            g.ecl.register_button_callbacks("accountscreen_subtitle", Some(draw_sub_title), None, None, None);

            // Account Name

            g.ecl.register_button(70, 150, 100, 15, "Account Name", "Shows the name your account is in", "accountscreen_accountname_title");
            g.ecl.register_button(190, 150, 200, 15, "Accessing...", "", "accountscreen_accountname");
            g.ecl.register_button_callbacks("accountscreen_accountname", Some(textbutton_draw), None, None, None);

            // Account number

            g.ecl.register_button(70, 175, 100, 15, "Account No.", "Shows your account number", "accountscreen_accountnumber_title");
            g.ecl.register_button(190, 175, 100, 15, "Accessing...", "", "accountscreen_accountnumber");
            g.ecl.register_button_callbacks("accountscreen_accountnumber", Some(textbutton_draw), None, None, None);

            // Balance

            g.ecl.register_button(70, 200, 100, 15, "Balance", "Shows your bank balance", "accountscreen_balance_title");
            g.ecl.register_button(190, 200, 100, 15, "Accessing...", "", "accountscreen_balance");
            g.ecl.register_button_callbacks("accountscreen_balance", Some(textbutton_draw), None, None, None);

            // Loan

            g.ecl.register_button(70, 225, 100, 15, "Loan", "Shows your the size of your loan", "accountscreen_loan_title");
            g.ecl.register_button(190, 225, 100, 15, "Accessing...", "", "accountscreen_loan");
            g.ecl.register_button_callbacks("accountscreen_loan", Some(textbutton_draw), None, None, None);

            // Interest rate

            g.ecl.register_button(300, 225, 80, 15, "", "", "accountscreen_loanrate");
            g.ecl.register_button_callbacks("accountscreen_loanrate", Some(textbutton_draw), None, None, None);

            // Put text in any fields that are constant

            let constants = self.bank_account(g).map(|a| (a.name.clone(), a.accountnumber));
            match constants {
                Some((name, number)) => {
                    // Account name
                    set_caption(g, "accountscreen_accountname", &name);

                    // Account number
                    set_caption(g, "accountscreen_accountnumber", &number.to_string());
                }
                None => {
                    set_caption(g, "accountscreen_accountname", "Unknown");
                    set_caption(g, "accountscreen_accountnumber", "Unknown");
                }
            }

            // OK button

            g.ecl.register_button(150, 300, 100, 15, "OK", "Return to the account management screen", "accountscreen_close");
            g.ecl.register_button_callback("accountscreen_close", Self::close_click);
        }

        self.lastupdate = 0;
        self.update_balance(g);
    }

    fn remove(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            return;
        }

        for name in [
            "accountscreen_maintitle",
            "accountscreen_subtitle",
            "accountscreen_accountname_title",
            "accountscreen_accountname",
            "accountscreen_accountnumber_title",
            "accountscreen_accountnumber",
            "accountscreen_balance_title",
            "accountscreen_balance",
            "accountscreen_loan_title",
            "accountscreen_loan",
            "accountscreen_loanrate",
            "accountscreen_close",
        ] {
            g.ecl.remove_button(name);
        }
    }

    fn update(&mut self, g: &mut Ctx) {
        self.update_balance(g);
    }

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("accountscreen_maintitle").is_some()
    }

    fn screen_id(&self) -> i32 {
        SCREEN_ACCOUNTSCREEN
    }

    fn escape_key_pressed(&mut self, g: &mut Ctx) -> bool {
        let nextpage = self.get_computer_screen(g).map(|gs| gs.nextpage);
        run_next_page(g, nextpage);
        true
    }

    fn set_computer_screen(&mut self, cs: ScreenRef) {
        self.cs = Some(cs);
    }

    fn computer_screen_ref(&self) -> Option<&ScreenRef> {
        self.cs.as_ref()
    }
}
