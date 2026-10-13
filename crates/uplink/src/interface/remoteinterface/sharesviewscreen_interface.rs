//! Port of source/interface/remoteinterface/sharesviewscreen_interface.h + source/interface/remoteinterface/sharesviewscreen_interface.cpp

use uplink_engine::{gl, text};

use crate::app::opengl_interface::{border_draw, clear_draw, textbutton_draw};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::remoteinterface::remoteinterface::{RemoteInterface, SCREEN_SHARESVIEWSCREEN};
use crate::interface::remoteinterface::remoteinterfacescreen::{draw_main_title, draw_sub_title};
use crate::objref::ScreenRef;
use crate::world::computer::computerscreen::genericscreen::GenericScreen;
use crate::world::date::Date;
use crate::world::player::Player;

/// The `ScreenRef` of the shares view screen currently in the remote interface's slot.
fn current_cs(g: &Ctx) -> Option<ScreenRef> {
    g.interface().ri.screen.get::<SharesViewScreenInterface>().and_then(|s| s.cs.clone())
}

/// The company name of the shares view screen currently in the remote interface's slot.
fn current_company(g: &Ctx) -> Option<String> {
    g.interface().ri.screen.get::<SharesViewScreenInterface>().map(|s| s.companyname.clone())
}

/// C++ `companyname[0] != ' '`: a company is selected unless the name is the " " placeholder.
fn company_set(companyname: &str) -> bool {
    companyname.chars().next() != Some(' ')
}

/// `CloseClick` body: runs the next page of the generic screen at `cs`.
fn close_with(g: &mut Ctx, cs: Option<ScreenRef>) {
    let Some(cs) = cs else { return };
    let nextpage = g
        .world()
        .get_computer(&cs.computer)
        .and_then(|c| c.get_computer_screen(cs.index))
        .and_then(|s| s.as_generic_screen())
        .map(|gs| gs.nextpage);
    let Some(nextpage) = nextpage else { return };
    if nextpage != -1 {
        RemoteInterface::run_screen(g, nextpage);
    }
}

/// `UpdateStatus` body for the company `companyname`: refreshes the owned and profit captions.
fn update_status_for(g: &mut Ctx, companyname: &str) {
    if !company_set(companyname) {
        return;
    }
    let Some(currentprice) = g.world().get_company(companyname).map(|c| c.get_share_price(-1)) else { return };
    let numowned = g.player().num_shares_owned(companyname);
    let pricepaid = g.player().shares_price_paid(companyname);
    let profit = (numowned * currentprice) - pricepaid;

    if g.interface().ri.security_level < 10 {
        let owned = format!("You own {numowned} shares\nPaid:{pricepaid}c Value:{}c", numowned * currentprice);
        if let Some(b) = g.ecl.get_button_mut("sharesviewscreen_owned") {
            b.set_caption(&owned);
        }

        let profits = format!("Profit:{profit}c");
        if let Some(b) = g.ecl.get_button_mut("sharesviewscreen_profit") {
            b.set_caption(&profits);
        }
    }
}

/// `BuyClick` / `SellClick` body: trades `howmany` shares of the selected company, then updates the status.
fn trade(g: &mut Ctx, howmany: i32) {
    let Some(companyname) = current_company(g).filter(|n| company_set(n)) else { return };
    Player::trade_shares(g, &companyname, howmany);
    update_status_for(g, &companyname);
}

/// Remote shares view screen: price graph and buy/sell controls for one company.
#[derive(Debug)]
pub struct SharesViewScreenInterface {
    pub cs: Option<ScreenRef>,
    pub companyname: String,
    /// Monthly share prices shown on the graph.
    pub sharehistory: [i32; 12],
    pub lastmonthset: i32,
}

impl Default for SharesViewScreenInterface {
    fn default() -> Self {
        SharesViewScreenInterface {
            cs: None,
            companyname: " ".to_string(),
            sharehistory: [0; 12],
            lastmonthset: 0,
        }
    }
}

impl SharesViewScreenInterface {
    /// `GetComputerScreen` (typed accessor; the C++ casts `cs`).
    pub fn get_computer_screen<'a>(&self, g: &'a Ctx) -> Option<&'a GenericScreen> {
        let cs = self.cs.as_ref()?;
        g.world().get_computer(&cs.computer)?.get_computer_screen(cs.index)?.as_generic_screen()
    }

    /// `SetCompanyName`
    pub fn set_company_name(&mut self, g: &mut Ctx, name: &str) {
        self.companyname = name.to_string();

        // Look up the company and copy the share prices
        let Some(company) = g.world().get_company(name) else { return };
        let mut sharehistory = [0; 12];
        for (i, price) in sharehistory.iter_mut().enumerate() {
            *price = company.get_share_price(i as i32);
        }
        let lastmonthset = company.get_share_last_month_set();
        self.sharehistory = sharehistory;
        self.lastmonthset = lastmonthset;

        if let Some(b) = g.ecl.get_button_mut("sharesviewscreen_subtitle") {
            b.set_caption(&self.companyname);
        }

        // Put those share prices into the text boxes
        let monthnow = lastmonthset;
        let yearnow = g.world().date.get_year();

        for it in 0..12 {
            let mut month = monthnow - it;
            if month < 0 {
                month += 12;
            }

            let bdate = format!("sharesviewscreen_date {it}");
            let bname = format!("sharesviewscreen_numbers {it}");

            let year = if month > monthnow { yearnow - 1 } else { yearnow };
            let date = format!("{}, {}", Date::get_month_name(month + 1), year);
            let value = format!("{} c", sharehistory[month as usize]);

            if let Some(b) = g.ecl.get_button_mut(&bdate) {
                b.set_caption(&date);
            }
            if let Some(b) = g.ecl.get_button_mut(&bname) {
                b.set_caption(&value);
            }
        }

        self.update_status(g);

        // Force a redraw of the graph
        g.ecl.dirty_button("sharesviewscreen_pricegraph");
    }

    /// `UpdateStatus` (static in C++; takes the screen's own state)
    pub fn update_status(&self, g: &mut Ctx) {
        update_status_for(g, &self.companyname);
    }

    /// `DrawPriceGraph` (static DrawFn)
    pub fn draw_price_graph(g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        clear_draw(g, button.x, button.y, button.width, button.height);

        let (x, y, h) = (button.x, button.y, button.height);
        let state = g
            .interface()
            .ri
            .screen
            .get::<SharesViewScreenInterface>()
            .map(|s| (s.companyname.clone(), s.sharehistory, s.lastmonthset));

        if let Some((companyname, sharehistory, monthnow)) = state {
            if company_set(&companyname) {
                // Draw the axis
                gl::color4f(1.0, 1.0, 1.0, 1.0);
                gl::begin(gl::LINES);
                gl::vertex2d((x + 25) as f64, (y + h - 40) as f64); // vertical
                gl::vertex2d((x + 25) as f64, (y + h - 190) as f64); // price
                gl::vertex2d((x + 25) as f64, (y + h - 40) as f64); // horizontal
                gl::vertex2d((x + 190) as f64, (y + h - 40) as f64); // time
                gl::end();

                text::draw_text_default(37, y + h - 40, "0");
                text::draw_text_default(32, y + h - 190, "150");
                text::draw_text_default(40, y + h - 30, "Time");

                // Start on the right (now) and work left (the past)
                gl::color4f(0.2, 0.2, 1.0, 1.0);
                gl::line_width(2.0);
                gl::begin(gl::LINE_STRIP);
                for it in 0..12 {
                    let mut month = monthnow - it;
                    if month < 0 {
                        month += 12;
                    }

                    let value = sharehistory[month as usize];
                    gl::vertex2d((x + 190 - (it * 15)) as f64, (y + h - 40 - value) as f64);
                }
                gl::end();
                gl::line_width(1.0);
            }
        }

        border_draw(button);
    }

    /// `DrawProfit` (static DrawFn; calls the base `draw_main_title`)
    pub fn draw_profit(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        clear_draw(g, button.x, button.y, button.width, button.height);
        draw_main_title(g, button, highlighted, clicked);
    }

    /// `BuyClick` (static MouseFn)
    pub fn buy_click(g: &mut Ctx, _button: &Button) {
        trade(g, 10);
    }

    /// `SellClick` (static MouseFn)
    pub fn sell_click(g: &mut Ctx, _button: &Button) {
        trade(g, -10);
    }

    /// `CloseClick` (static MouseFn)
    pub fn close_click(g: &mut Ctx, _button: &Button) {
        let cs = current_cs(g);
        close_with(g, cs);
    }
}

impl InterfaceScreen for SharesViewScreenInterface {
    crate::impl_as_any!();

    fn create(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            let Some(gs) = self.get_computer_screen(g) else { return };
            let maintitle = gs.base.maintitle.clone();
            let subtitle = gs.base.subtitle.clone();

            // Draw the screen titles
            g.ecl.register_button(80, 60, 350, 25, &maintitle, "", "sharesviewscreen_maintitle");
            g.ecl.register_button_callbacks("sharesviewscreen_maintitle", Some(draw_main_title), None, None, None);
            g.ecl.register_button(80, 80, 350, 20, &subtitle, "", "sharesviewscreen_subtitle");
            g.ecl.register_button_callbacks("sharesviewscreen_subtitle", Some(draw_sub_title), None, None, None);

            // Share value graph
            g.ecl.register_button(30, 120, 200, 15, "Share value graph", "", "sharesviewscreen_valuetitle");
            g.ecl.register_button(30, 135, 200, 200, "", "", "sharesviewscreen_pricegraph");
            g.ecl.register_button_callbacks(
                "sharesviewscreen_pricegraph",
                Some(Self::draw_price_graph),
                None,
                None,
                None,
            );

            // Share values (numeric)
            g.ecl.register_button(250, 120, 150, 15, "Share values", "", "sharesviewscreen_numberstitle");

            for i in 0..12 {
                let date = format!("sharesviewscreen_date {i}");
                let name = format!("sharesviewscreen_numbers {i}");
                g.ecl.register_button(250, 135 + i * 17, 100, 15, "date", "", &date);
                g.ecl.register_button(350, 135 + i * 17, 50, 15, "bla", "", &name);
                g.ecl.register_button_callbacks(&date, Some(textbutton_draw), None, None, None);
                g.ecl.register_button_callbacks(&name, Some(textbutton_draw), None, None, None);
            }

            // Control buttons
            if g.interface().ri.security_level < 10 {
                g.ecl.register_button(30, 350, 30, 15, "Buy", "Click to buy shares in this company", "sharesviewscreen_buy");
                g.ecl.register_button_callback("sharesviewscreen_buy", Self::buy_click);

                g.ecl.register_button(30, 365, 30, 15, "Sell", "Click to sell shares in this company", "sharesviewscreen_sell");
                g.ecl.register_button_callback("sharesviewscreen_sell", Self::sell_click);

                g.ecl.register_button(70, 350, 150, 30, "", "", "sharesviewscreen_owned");
                g.ecl.register_button_callbacks("sharesviewscreen_owned", Some(textbutton_draw), None, None, None);

                g.ecl.register_button(220, 350, 140, 30, "", "", "sharesviewscreen_profit");
                g.ecl.register_button_callbacks("sharesviewscreen_profit", Some(Self::draw_profit), None, None, None);
            }

            self.update_status(g);

            // Close button
            g.ecl.register_button(360, 365, 45, 15, "Close", "Return to the share listing", "sharesviewscreen_close");
            g.ecl.register_button_callback("sharesviewscreen_close", Self::close_click);
        }
    }

    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            g.ecl.remove_button("sharesviewscreen_maintitle");
            g.ecl.remove_button("sharesviewscreen_subtitle");

            g.ecl.remove_button("sharesviewscreen_valuetitle");
            g.ecl.remove_button("sharesviewscreen_pricegraph");
            g.ecl.remove_button("sharesviewscreen_numberstitle");

            for i in 0..12 {
                g.ecl.remove_button(&format!("sharesviewscreen_date {i}"));
                g.ecl.remove_button(&format!("sharesviewscreen_numbers {i}"));
            }

            g.ecl.remove_button("sharesviewscreen_owned");
            g.ecl.remove_button("sharesviewscreen_profit");
            g.ecl.remove_button("sharesviewscreen_sell");
            g.ecl.remove_button("sharesviewscreen_buy");

            g.ecl.remove_button("sharesviewscreen_close");
        }
    }

    fn update(&mut self, _g: &mut Ctx) {}

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("sharesviewscreen_maintitle").is_some()
    }

    fn screen_id(&self) -> i32 {
        SCREEN_SHARESVIEWSCREEN
    }

    fn escape_key_pressed(&mut self, g: &mut Ctx) -> bool {
        // The screen is taken out of its slot while this runs, so use our own `cs`.
        close_with(g, self.cs.clone());
        true
    }

    fn set_computer_screen(&mut self, cs: ScreenRef) {
        self.cs = Some(cs);
    }

    fn computer_screen_ref(&self) -> Option<&ScreenRef> {
        self.cs.as_ref()
    }
}
