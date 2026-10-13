//! Port of source/interface/localinterface/finance_interface.h + source/interface/localinterface/finance_interface.cpp
//! Financial screen: lists the player's bank accounts and their balances.

use uplink_engine::gl;

use crate::app::globals::ALPHA;
use crate::app::opengl_interface::{border_draw, button_click, button_highlight, clear_draw, set_colour, text_draw};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::impl_as_any;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::localinterface::localinterface::{LocalInterface, PANELSIZE, SCREEN_FINANCE, SCREEN_NONE};
use crate::interface::localinterface::localinterfacescreen;
use crate::interface::localinterface::phonedialler::PhoneDialler;
use crate::interface::remoteinterface::remoteinterface::RemoteInterface;
use crate::world::computer::bankaccount::BankAccount;
use crate::world::computer::recordbank::{RECORDBANK_NAME, RECORDBANK_SECURITY};
use crate::world::connection::Connection;

#[derive(Debug, Clone, Default)]
pub struct FinanceInterface {
    pub oldbalance: i32,
    pub previousnumaccounts: i32,
    pub lastupdate: i32,
}

/// C++ `sscanf(s, "%d", ...)`: skips leading white space, reads an optional sign and digits.
fn parse_leading_i32(s: &str) -> Option<i32> {
    let s = s.trim_start();
    let bytes = s.as_bytes();
    let mut end = 0;
    if end < bytes.len() && (bytes[end] == b'+' || bytes[end] == b'-') {
        end += 1;
    }
    let digits_start = end;
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    if end == digits_start {
        return None;
    }
    s[..end].parse().ok()
}

/// C++ `sscanf(name, "finance_account %d", &index)`.
fn account_button_index(name: &str) -> i32 {
    name.strip_prefix("finance_account ").and_then(parse_leading_i32).unwrap_or(0)
}

/// C++ `sprintf(name, "finance_account %d", i)`.
fn account_button_name(i: i32) -> String {
    format!("finance_account {}", i)
}

/// C++ `sscanf(details, "%s %s", ip, accno)` on a player account entry ("ip accno").
fn split_account(details: &str) -> (String, String) {
    let mut parts = details.split_whitespace();
    let ip = parts.next().unwrap_or("").to_string();
    let accno = parts.next().unwrap_or("").to_string();
    (ip, accno)
}

/// C++ `sscanf(caption, "%s    %d", accno, &balance)` on an account button caption. The balance is
/// `None` when the `%d` does not match (the C++ then keeps its initial -1).
fn scan_account_caption(caption: &str) -> (String, Option<i32>) {
    let s = caption.trim_start();
    let accno_end = s.find(char::is_whitespace).unwrap_or(s.len());
    let accno = s[..accno_end].to_string();
    (accno, parse_leading_i32(&s[accno_end..]))
}

/// `(screenw, screenh, paneltop, panelwidth)`, recomputed from the options as in the C++.
fn panel_metrics(g: &Ctx) -> (i32, i32, i32, i32) {
    let screenw = g.app.options.get_option_value("graphics_screenwidth");
    let screenh = g.app.options.get_option_value("graphics_screenheight");
    let paneltop = (100.0 * ((screenw as f64 * PANELSIZE) / 188.0) + 30.0) as i32;
    let panelwidth = (screenw as f64 * PANELSIZE) as i32;
    (screenw, screenh, paneltop, panelwidth)
}

/// Unix seconds, C++ `time(NULL)`.
fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

impl FinanceInterface {
    /// C++ `FinanceInterface::TitleClick` (static button callback).
    pub fn title_click(g: &mut Ctx, _button: &Button) {
        LocalInterface::run_screen(g, SCREEN_NONE, -1);
    }

    /// C++ `FinanceInterface::DrawAccountsTitle` (static draw callback).
    pub fn draw_accounts_title(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        text_draw(g, button, highlighted, clicked);

        set_colour(g, "TitleText");

        gl::begin(gl::LINES);
        gl::vertex2i(button.x, button.y + button.height);
        gl::vertex2i(button.x + button.width, button.y + button.height);
        gl::end();
    }

    /// C++ `FinanceInterface::ClickAccountButton` (static button callback).
    pub fn click_account_button(g: &mut Ctx, button: &Button) {
        let index = account_button_index(&button.name);

        if index == g.player().currentaccount {
            // Get the account details

            let details = g
                .player()
                .accounts
                .get(g.player().currentaccount as usize)
                .cloned()
                .unwrap_or_default();
            let (ip, accno) = split_account(&details);

            // Connect to this account computer

            Connection::disconnect(g, "PLAYER");
            Connection::reset(g, "PLAYER");

            let mut pd = PhoneDialler::default();
            pd.create_at(g, 100, 100);
            pd.dial_number(g, &ip);
            InterfaceScreen::remove(&mut pd, g);

            g.player_mut().get_connection_mut().add_vlocation(&ip);
            Connection::connect(g, "PLAYER");

            RemoteInterface::run_new_location(g);

            // Lookup the user name and security level

            let lookup = {
                let world = g.world();
                world
                    .get_vlocation(&ip)
                    .and_then(|vl| vl.get_computer(world))
                    .and_then(|bank| bank.recordbank.get_record_from_account_number(&accno))
                    .map(|rec| {
                        (
                            rec.get_field(RECORDBANK_NAME).map(str::to_string),
                            rec.get_field(RECORDBANK_SECURITY).map(str::to_string),
                        )
                    })
            };
            let Some((Some(name), Some(security_string))) = lookup else {
                debug_assert!(false, "FinanceInterface: bank record missing for account {accno}");
                return;
            };
            let security = parse_leading_i32(&security_string).unwrap_or(0);

            // Set security name and level
            // Run the accounts screen

            g.interface_mut().ri.set_security(&name, security);
            RemoteInterface::run_screen(g, 3);
        } else {
            // Set this as the current account

            let oldaccount = g.player().currentaccount;
            g.player_mut().set_current_account(index);

            g.ecl.dirty_button(&account_button_name(oldaccount));
        }
    }

    /// C++ `FinanceInterface::DrawAccountButton` (static draw callback).
    pub fn draw_account_button(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        let index = account_button_index(&button.name);

        // Draw a gray background if this is the player's current account

        if index == g.player().currentaccount {
            gl::begin(gl::QUADS);
            gl::color4f(0.6, 0.6, 0.6, ALPHA);
            gl::vertex2i(button.x, button.y);
            gl::color4f(0.4, 0.4, 0.4, ALPHA);
            gl::vertex2i(button.x + button.width, button.y);
            gl::color4f(0.6, 0.6, 0.6, ALPHA);
            gl::vertex2i(button.x + button.width, button.y + button.height);
            gl::color4f(0.4, 0.4, 0.4, ALPHA);
            gl::vertex2i(button.x, button.y + button.height);
            gl::end();
        } else {
            clear_draw(g, button.x, button.y, button.width, button.height);
        }

        text_draw(g, button, highlighted, clicked);

        // Draw a box around the text if highlighted

        if highlighted || clicked {
            gl::color4f(1.0, 1.0, 1.0, 1.0);
            border_draw(button);
        }
    }

    /// Registers the "Logging on..." button for account `i` (the Create and Update loops share this).
    fn register_account_button(g: &mut Ctx, i: i32, tooltip: &str) {
        let (screenw, _screenh, paneltop, panelwidth) = panel_metrics(g);
        let name = account_button_name(i);
        g.ecl.register_button(
            screenw - panelwidth,
            paneltop + 100 + i * 20,
            panelwidth - 7,
            15,
            "Logging on...",
            tooltip,
            &name,
        );
        g.ecl.register_button_callbacks(
            &name,
            Some(Self::draw_account_button),
            Some(Self::click_account_button),
            Some(button_click),
            Some(button_highlight),
        );
    }
}

impl InterfaceScreen for FinanceInterface {
    fn create(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            return;
        }

        localinterfacescreen::create(g);

        let (screenw, _screenh, paneltop, panelwidth) = panel_metrics(g);

        g.ecl.register_button(screenw - panelwidth, paneltop + 3, panelwidth - 7, 15, "FINANCIAL", "Remove the finance screen", "finance_title");
        g.ecl.register_button_callback("finance_title", Self::title_click);

        g.ecl.register_button(screenw - panelwidth, paneltop + 50, panelwidth - 7, 15, "", "", "finance_balance");
        g.ecl.register_button_callbacks("finance_balance", Some(text_draw), None, None, None);

        g.ecl.register_button(screenw - panelwidth, paneltop + 80, panelwidth - 7, 15, "AccNo     Balance", "", "finance_acctitle");
        g.ecl.register_button_callbacks("finance_acctitle", Some(Self::draw_accounts_title), None, None, None);

        // Create a new button for each open account

        let count = g.player().accounts.len() as i32;
        for i in 0..count {
            Self::register_account_button(g, i, "Set this to be your current account");
        }

        self.previousnumaccounts = count;
        self.lastupdate = 0;
    }

    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            localinterfacescreen::remove(g);

            g.ecl.remove_button("finance_title");
            g.ecl.remove_button("finance_balance");
            g.ecl.remove_button("finance_acctitle");

            // Remove each account button

            let mut i = 0;
            while g.ecl.get_button(&account_button_name(i)).is_some() {
                g.ecl.remove_button(&account_button_name(i));
                i += 1;
            }
        }
    }

    fn update(&mut self, g: &mut Ctx) {
        if now_secs() > self.lastupdate as i64 + 5 && self.is_visible(g) {
            // Create/Destroy new account buttons if neccisary

            let count = g.player().accounts.len() as i32;

            if self.previousnumaccounts < count {
                // Create a new button for each new open account

                for i in self.previousnumaccounts..count {
                    Self::register_account_button(g, i, "Open this account");
                }

                self.previousnumaccounts = count;
            } else if self.previousnumaccounts > count {
                for i in count..self.previousnumaccounts {
                    g.ecl.remove_button(&account_button_name(i));
                }

                self.previousnumaccounts = count;
            }

            // Update each account button with the new balance
            // Also work out the new balance (quicker to do it here than to call player->GetBalance )

            let mut balance = 0;

            for i in 0..count {
                let name = account_button_name(i);
                let Some(caption) = g.ecl.get_button(&name).map(|b| b.caption.clone()) else {
                    debug_assert!(false, "FinanceInterface: missing account button {name}");
                    continue;
                };

                // Try to get the old balance

                let mut oldbalance = -1;

                if caption != "Logging on..." && caption != "Failed to log on" {
                    if let (_, Some(scanned)) = scan_account_caption(&caption) {
                        oldbalance = scanned;
                    }
                }

                let details = g.player().accounts[i as usize].clone();
                let (ip, accno) = split_account(&details);

                // Lookup the balance
                // PORT: C++ dereferences the BankAccount unchecked; a missing account is shown as 0.

                let newbalance = BankAccount::get_account(g.world(), &ip, &accno).map(|ba| ba.balance).unwrap_or(0);
                if oldbalance == -1 {
                    oldbalance = newbalance;
                }
                balance += newbalance;

                let newcaption = if newbalance > oldbalance {
                    format!("{}    {}c     (+{}c)", accno, newbalance, newbalance - oldbalance)
                } else if newbalance < oldbalance {
                    format!("{}    {}c     ({}c)", accno, newbalance, newbalance - oldbalance)
                } else {
                    format!("{}    {}c", accno, newbalance)
                };

                if let Some(b) = g.ecl.get_button_mut(&name) {
                    b.set_caption(&newcaption);
                }

                if newbalance != oldbalance {
                    g.ecl.dirty_button("localint_background");
                }
            }

            // Update the overall balance

            let caption = format!("Balance : {} credits", balance);
            if let Some(b) = g.ecl.get_button_mut("finance_balance") {
                b.set_caption(&caption);
            }

            self.lastupdate = now_secs() as i32;
        }
    }

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("finance_title").is_some()
    }

    fn screen_id(&self) -> i32 {
        SCREEN_FINANCE
    }

    impl_as_any!();
}
