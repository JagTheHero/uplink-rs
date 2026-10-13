//! Port of source/interface/remoteinterface/loansscreen_interface.h + source/interface/remoteinterface/loansscreen_interface.cpp

use crate::app::opengl_interface::textbutton_draw;
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::game::data::data::{
    LARGELOAN_INTEREST, LARGELOAN_MAX, MAXLOAN_INTEREST, MEDIUMLOAN_INTEREST, MEDIUMLOAN_MAX,
    SMALLLOAN_INTEREST, SMALLLOAN_MAX,
};
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::remoteinterface::remoteinterface::{RemoteInterface, SCREEN_LOANSSCREEN};
use crate::interface::remoteinterface::remoteinterfacescreen::{draw_main_title, draw_sub_title};
use crate::objref::ScreenRef;
use crate::world::computer::bankaccount::BankAccount;
use crate::world::computer::computerscreen::genericscreen::GenericScreen;
use crate::world::computer::recordbank::RECORDBANK_ACCNO;

/// Loans screen: shown on public bank servers. Lets the player take out or pay off a loan on the
/// account they logged in with. Changes are held in `loanchange` until the screen closes.
#[derive(Debug, Default, Clone)]
pub struct LoansScreenInterface {
    pub cs: Option<ScreenRef>,
    /// Account number of the bank account being edited (C++ `BankAccount *account`).
    /// PORT: non-owning pointer; the account number is the stable key into `bank.accounts`.
    pub account: Option<String>,
    /// Pending change to the loan (and balance) in credits, applied on close.
    pub loanchange: i32,
}

/// `EclGetButton(name)->SetCaption(text)`. A missing button is ignored here; the C++ would crash.
fn set_caption(g: &mut Ctx, name: &str, caption: &str) {
    if let Some(button) = g.ecl.get_button_mut(name) {
        button.set_caption(caption);
    }
}

impl LoansScreenInterface {
    /// Interest rate caption for a loan size, e.g. "(5% apr)" (the four bands in the C++).
    pub fn loan_rate_caption(loan: i32) -> String {
        let interest = if loan <= SMALLLOAN_MAX {
            SMALLLOAN_INTEREST
        } else if loan <= MEDIUMLOAN_MAX {
            MEDIUMLOAN_INTEREST
        } else if loan <= LARGELOAN_MAX {
            LARGELOAN_INTEREST
        } else {
            MAXLOAN_INTEREST
        };
        // PORT: C++ `int (100.0 * INTEREST)` in double precision gives 20, 40, 70, 100. The f32
        // constants would truncate 0.7 to 69, so round to the whole percent instead.
        let percent = (100.0 * interest as f64).round() as i32;
        format!("({}% apr)", percent)
    }

    /// `GetComputerScreen` (typed accessor; the C++ casts `cs`).
    pub fn get_computer_screen<'a>(&self, g: &'a Ctx) -> Option<&'a GenericScreen> {
        let cs = self.cs.as_ref()?;
        g.world()
            .get_computer(&cs.computer)?
            .get_computer_screen(cs.index)?
            .as_generic_screen()
    }

    /// The bank account this screen edits (C++ `account`), found through the bank computer
    /// the screen is on.
    fn account_of<'a>(&self, g: &'a Ctx) -> Option<&'a BankAccount> {
        let accno = self.account.as_deref()?;
        let cs = self.cs.as_ref()?;
        g.world()
            .get_computer(&cs.computer)?
            .bank()?
            .accounts
            .get(accno)
    }

    /// Caption text for the loan, balance and interest rate, shared by the increase and decrease clicks.
    fn set_loan_captions(g: &mut Ctx, newbalance: i32, newloan: i32) {
        // Update the balance
        set_caption(g, "loansscreen_balance", &newbalance.to_string());
        // Update the loan
        set_caption(g, "loansscreen_loan", &newloan.to_string());
        // Update the interest rate
        set_caption(g, "loansscreen_loanrate", &Self::loan_rate_caption(newloan));
    }

    /// `IncreaseLoanClick` (static MouseFn): adds 100 credits to the loan, capped by the credit rating.
    pub fn increase_loan_click(g: &mut Ctx, _button: &Button) {
        let Some(this) = g.interface().ri.screen.get::<LoansScreenInterface>().cloned() else {
            return;
        };
        let Some(account) = this.account_of(g).cloned() else {
            return;
        };
        if !this.is_visible(g) {
            return;
        }

        // Make sure we don't take out too much
        let maxcredit = account.loan + g.player().rating.creditrating * 100;

        if account.loan < maxcredit {
            let mut loanchange = this.loanchange + 100;
            if account.loan + loanchange > maxcredit {
                loanchange = maxcredit - account.loan;
            }

            let newloan = account.loan + loanchange;
            let newbalance = account.balance + loanchange;

            Self::set_loan_captions(g, newbalance, newloan);

            if let Some(screen) = g.interface_mut().ri.screen.get_mut::<LoansScreenInterface>() {
                screen.loanchange = loanchange;
            }
        }
    }

    /// `DecreaseLoanClick` (static MouseFn): pays 100 credits off the loan, floored at zero owed.
    pub fn decrease_loan_click(g: &mut Ctx, _button: &Button) {
        let Some(this) = g.interface().ri.screen.get::<LoansScreenInterface>().cloned() else {
            return;
        };
        let Some(account) = this.account_of(g).cloned() else {
            return;
        };
        if !this.is_visible(g) {
            return;
        }

        let mut loanchange = this.loanchange - 100;
        if account.loan + loanchange < 0 {
            loanchange = account.loan * -1;
        }

        let newloan = account.loan + loanchange;
        let newbalance = account.balance + loanchange;

        Self::set_loan_captions(g, newbalance, newloan);

        if let Some(screen) = g.interface_mut().ri.screen.get_mut::<LoansScreenInterface>() {
            screen.loanchange = loanchange;
        }
    }

    /// `CloseClick`: applies `loanchange` to the account and the credit rating, then runs the next page.
    /// Used as the OK button's callback (through `close_button_click`), which drops the button argument.
    pub fn close_click(g: &mut Ctx) {
        let Some(this) = g.interface().ri.screen.get::<LoansScreenInterface>().cloned() else {
            return;
        };
        Self::close_body(g, &this);
    }

    /// Button adapter for `close_click`, so it can be registered as a `MouseFn`.
    fn close_button_click(g: &mut Ctx, _button: &Button) {
        Self::close_click(g);
    }

    /// Body of `CloseClick` on a given screen state. Escape passes `self`, because the screen is
    /// taken out of its slot while its own method runs, so the slot lookup would find nothing.
    fn close_body(g: &mut Ctx, this: &LoansScreenInterface) {
        // Update the account if changed
        if let (Some(cs), Some(accno)) = (this.cs.as_ref(), this.account.as_ref()) {
            if this.loanchange != 0 {
                let loanchange = this.loanchange;
                if let Some(account) = g
                    .world_mut()
                    .get_computer_mut(&cs.computer)
                    .and_then(|comp| comp.bank_mut())
                    .and_then(|bank| bank.accounts.get_mut(accno))
                {
                    account.loan += loanchange;
                    account.balance += loanchange;
                }

                let creditchange = this.loanchange / 100;
                g.player_mut()
                    .rating
                    .change_credit_rating(creditchange * -1);
            }
        }

        // Go to the next page
        let nextpage = this.get_computer_screen(g).map_or(-1, |gs| gs.nextpage);
        if nextpage != -1 {
            RemoteInterface::run_screen(g, nextpage);
        }
    }
}

impl InterfaceScreen for LoansScreenInterface {
    crate::impl_as_any!();

    /// `Create(ComputerScreen *cs)` body (the `cs` is already set by `set_computer_screen`).
    /// Looks up the logged-in account on the bank computer first; if the record or its account
    /// number is missing, no buttons are made. The C++ no-argument `Create()` overload maps to this too.
    fn create(&mut self, g: &mut Ctx) {
        let Some(cs) = self.cs.clone() else {
            println!("LoansScreenInterface::Create, tried to create when GenericScreen==NULL");
            return;
        };

        // Look up the account number based on the log in name
        let security_name = g.interface().ri.security_name.clone();
        let accno = {
            let bank_comp = g.world().get_computer(&cs.computer);
            debug_assert!(
                bank_comp.is_some_and(|c| c.bank().is_some()),
                "LoansScreenInterface: screen is not on a bank computer"
            );
            bank_comp.and_then(|comp| {
                comp.recordbank
                    .get_record_from_name(&security_name)?
                    .get_field(RECORDBANK_ACCNO)
                    .map(str::to_string)
            })
        };
        let Some(accno) = accno else {
            self.account = None;
            return;
        };

        // Now look up the account based on that account number
        let account_exists = g
            .world()
            .get_computer(&cs.computer)
            .and_then(|comp| comp.bank())
            .is_some_and(|bank| bank.accounts.contains_key(&accno));
        self.account = account_exists.then_some(accno);

        if self.is_visible(g) {
            return;
        }

        let account = self.account_of(g).cloned();
        let (maintitle, subtitle) = self
            .get_computer_screen(g)
            .map(|gs| (gs.base.maintitle.clone(), gs.base.subtitle.clone()))
            .unwrap_or_default();

        g.ecl.register_button(80, 60, 350, 25, &maintitle, "", "loansscreen_maintitle");
        g.ecl.register_button_callbacks("loansscreen_maintitle", Some(draw_main_title), None, None, None);
        g.ecl.register_button(80, 80, 350, 20, &subtitle, "", "loansscreen_subtitle");
        g.ecl.register_button_callbacks("loansscreen_subtitle", Some(draw_sub_title), None, None, None);

        // Account Name
        g.ecl.register_button(70, 150, 100, 15, "Account Name", "Shows the name your account is in", "loansscreen_accountname_title");
        g.ecl.register_button(190, 150, 100, 15, "Accessing...", "", "loansscreen_accountname");
        g.ecl.register_button_callbacks("loansscreen_accountname", Some(textbutton_draw), None, None, None);

        // Account number
        g.ecl.register_button(70, 175, 100, 15, "Account No.", "Shows your account number", "loansscreen_accountnumber_title");
        g.ecl.register_button(190, 175, 100, 15, "Accessing...", "", "loansscreen_accountnumber");
        g.ecl.register_button_callbacks("loansscreen_accountnumber", Some(textbutton_draw), None, None, None);

        // Balance
        g.ecl.register_button(70, 200, 100, 15, "Balance", "Shows your bank balance", "loansscreen_balance_title");
        g.ecl.register_button(190, 200, 100, 15, "Accessing...", "", "loansscreen_balance");
        g.ecl.register_button_callbacks("loansscreen_balance", Some(textbutton_draw), None, None, None);

        // Loan
        g.ecl.register_button(70, 225, 100, 15, "Loan", "Shows your the size of your loan", "loansscreen_loan_title");
        g.ecl.register_button(190, 225, 100, 15, "Accessing...", "", "loansscreen_loan");
        g.ecl.register_button_callbacks("loansscreen_loan", Some(textbutton_draw), None, None, None);

        // Interest rate
        g.ecl.register_button(300, 225, 80, 15, "", "", "loansscreen_loanrate");
        g.ecl.register_button_callbacks("loansscreen_loanrate", Some(textbutton_draw), None, None, None);

        if let Some(account) = &account {
            // Account name
            set_caption(g, "loansscreen_accountname", &account.name);
            // Account number
            set_caption(g, "loansscreen_accountnumber", &account.accountnumber.to_string());
            // Update the balance, loan and interest rate
            Self::set_loan_captions(g, account.balance, account.loan);

            self.loanchange = 0;
        } else {
            set_caption(g, "loansscreen_accountname", "Unknown");
            set_caption(g, "loansscreen_accountnumber", "Unknown");
        }

        // Increase loan button
        g.ecl.register_button(150, 260, 100, 15, "Increase loan", "Click to take out a larger loan", "loansscreen_increaseloan");
        g.ecl.register_button_callback("loansscreen_increaseloan", Self::increase_loan_click);

        // Pay off loan button
        g.ecl.register_button(150, 280, 100, 15, "Decrease loan", "Click to pay off some of the loan", "loansscreen_decreaseloan");
        g.ecl.register_button_callback("loansscreen_decreaseloan", Self::decrease_loan_click);

        // Max available loan button
        // PORT: C++ dereferences `account` here even when it is NULL (a crash); the port uses a
        // loan of 0 instead.
        let maxloan_loan = account.as_ref().map_or(0, |a| a.loan);
        let maxloan = format!(
            "Max loan: {}c",
            maxloan_loan + g.player().rating.creditrating * 100
        );
        g.ecl.register_button(270, 260, 100, 15, &maxloan, "", "loansscreen_maxloan");
        g.ecl.register_button_callbacks("loansscreen_maxloan", Some(textbutton_draw), None, None, None);

        // OK button
        g.ecl.register_button(150, 340, 100, 15, "OK", "Return to the account management screen", "loansscreen_close");
        g.ecl.register_button_callback("loansscreen_close", Self::close_button_click);
    }

    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            for name in [
                "loansscreen_maintitle",
                "loansscreen_subtitle",
                "loansscreen_accountname_title",
                "loansscreen_accountname",
                "loansscreen_accountnumber_title",
                "loansscreen_accountnumber",
                "loansscreen_balance_title",
                "loansscreen_balance",
                "loansscreen_loan_title",
                "loansscreen_loan",
                "loansscreen_loanrate",
                "loansscreen_increaseloan",
                "loansscreen_decreaseloan",
                "loansscreen_maxloan",
                "loansscreen_close",
            ] {
                g.ecl.remove_button(name);
            }
        }
    }

    fn update(&mut self, _g: &mut Ctx) {}

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("loansscreen_maintitle").is_some()
    }

    /// `EscapeKeyPressed`: same as the OK button, and always reports the key as handled.
    fn escape_key_pressed(&mut self, g: &mut Ctx) -> bool {
        Self::close_body(g, self);
        true
    }

    fn screen_id(&self) -> i32 {
        SCREEN_LOANSSCREEN
    }

    fn set_computer_screen(&mut self, cs: ScreenRef) {
        self.cs = Some(cs);
    }

    fn computer_screen_ref(&self) -> Option<&ScreenRef> {
        self.cs.as_ref()
    }
}
