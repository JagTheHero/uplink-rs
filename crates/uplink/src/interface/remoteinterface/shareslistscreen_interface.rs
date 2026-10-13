//! Port of source/interface/remoteinterface/shareslistscreen_interface.h + source/interface/remoteinterface/shareslistscreen_interface.cpp

use uplink_engine::{gl, text};

use crate::app::opengl_interface::{border_draw, button_assignbitmaps, button_click, button_highlight, clear_draw, textbutton_draw};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::interface::interfacescreen::{InterfaceScreen, ScreenSlot};
use crate::interface::remoteinterface::remoteinterface::{RemoteInterface, SCREEN_SHARESLISTSCREEN};
use crate::interface::remoteinterface::remoteinterfacescreen::{draw_main_title, draw_sub_title};
use crate::interface::remoteinterface::sharesviewscreen_interface::SharesViewScreenInterface;
use crate::objref::ScreenRef;
use crate::world::company::company::{COMPANYTYPE_COMMERCIAL, COMPANYTYPE_FINANCIAL, COMPANYTYPE_INDUSTRIAL};
use crate::world::computer::computerscreen::shareslistscreen::{
    SharesListScreen, SHARESLISTSCREENTYPE_ALLSHARES,
};

/// Remote shares list screen: the stock companies, with a text filter.
#[derive(Debug, Default)]
pub struct SharesListScreenInterface {
    pub cs: Option<ScreenRef>,
    pub baseoffset: i32,
    pub fulllist: Vec<String>,
    pub filteredlist: Vec<String>,
}

/// Number of share rows shown at once (C++ literal 14 in the loops).
const SHARE_ROWS: i32 = 14;
/// Name of the filter text box.
const FILTER_TEXT: &str = "shareslistscreen_filtertext";

/// Name of share row `i` (`"shareslistscreen_share %d"` in C++).
fn share_name(i: i32) -> String {
    format!("shareslistscreen_share {i}")
}

/// `sscanf(name, "shareslistscreen_share %d", &index)`. PORT: C++ leaves the index unset when the
/// name does not match; 0 is used instead. Every caller passes a row button name.
fn scan_share(name: &str) -> i32 {
    name.strip_prefix("shareslistscreen_share ")
        .and_then(|rest| rest.trim_start().parse::<i32>().ok())
        .unwrap_or(0)
}

/// `LList::GetData` for a non-negative index, `None` when out of range (C++ `ValidIndex`).
fn valid_at(list: &[String], index: i32) -> Option<&String> {
    if index < 0 {
        None
    } else {
        list.get(index as usize)
    }
}

/// The slot holding the remote interface's current screen (`screen_slot` helper).
fn ri_screen_slot(g: &mut Ctx) -> &mut ScreenSlot {
    &mut g.interface_mut().ri.screen
}

/// The shown screen as this type.
fn screen_ref(g: &Ctx) -> Option<&SharesListScreenInterface> {
    g.interface().ri.screen.get::<SharesListScreenInterface>()
}

/// The shown screen as this type, mutably.
fn screen_mut(g: &mut Ctx) -> Option<&mut SharesListScreenInterface> {
    g.interface_mut().ri.screen.get_mut::<SharesListScreenInterface>()
}

impl SharesListScreenInterface {
    /// `GetComputerScreen` (typed accessor; the C++ casts `cs`).
    pub fn get_computer_screen<'a>(&self, g: &'a Ctx) -> Option<&'a SharesListScreen> {
        let cs = self.cs.as_ref()?;
        g.world()
            .get_computer(&cs.computer)?
            .get_computer_screen(cs.index)?
            .as_shares_list_screen()
    }

    /// `SetFullList`
    pub fn set_full_list(&mut self, g: &mut Ctx, newfulllist: &[String]) {
        self.fulllist = newfulllist.to_vec();
        self.filteredlist.clear();
        self.apply_filter(g, None);
    }

    /// `SetFullListFromFiltered`
    pub fn set_full_list_from_filtered(&mut self, g: &mut Ctx) {
        self.fulllist = self.filteredlist.clone();
        self.apply_filter(g, None);
    }

    /// `ApplyFilter`: `None` means no filter text.
    pub fn apply_filter(&mut self, g: &mut Ctx, filter: Option<&str>) {
        self.filteredlist.clear();
        match filter {
            Some(filter) => {
                // PORT: C++ LowerCaseString lowers ASCII only; so does to_ascii_lowercase.
                let needle = filter.to_ascii_lowercase();
                for name in &self.fulllist {
                    if name.to_ascii_lowercase().contains(&needle) {
                        self.filteredlist.push(name.clone());
                    }
                }
            }
            None => self.filteredlist = self.fulllist.clone(),
        }

        self.baseoffset = 0;

        // Dirty the buttons
        for ib in 0..SHARE_ROWS {
            g.ecl.dirty_button(&share_name(ib));
        }
    }

    /// Whether the share row button `button` holds an entry of the filtered list.
    fn share_is_listed(g: &Ctx, button: &Button) -> bool {
        let baseoffset = screen_ref(g).map_or(0, |s| s.baseoffset);
        let shareindex = scan_share(&button.name) + baseoffset;
        screen_ref(g).is_some_and(|s| valid_at(&s.filteredlist, shareindex).is_some())
    }

    /// `ShareDraw` (static DrawFn)
    pub fn share_draw(g: &mut Ctx, button: &Button, highlighted: bool, _clicked: bool) {
        clear_draw(g, button.x, button.y, button.width, button.height);

        let baseoffset = screen_ref(g).map_or(0, |s| s.baseoffset);
        let shareindex = scan_share(&button.name) + baseoffset;

        let Some(companyname) = screen_ref(g).and_then(|s| valid_at(&s.filteredlist, shareindex)).cloned() else {
            return;
        };
        let Some(company) = g.world().get_company(&companyname) else { return };

        let (x, y, w, h) = (button.x, button.y, button.width, button.height);

        if shareindex % 2 == 0 {
            gl::begin(gl::QUADS);
            gl::color3ub(8, 20, 80);
            gl::vertex2i(x, y);
            gl::color3ub(8, 20, 0);
            gl::vertex2i(x + w, y);
            gl::color3ub(8, 20, 80);
            gl::vertex2i(x + w, y + h);
            gl::color3ub(8, 20, 0);
            gl::vertex2i(x, y + h);
            gl::end();
        } else {
            gl::begin(gl::QUADS);
            gl::color3ub(8, 20, 0);
            gl::vertex2i(x, y);
            gl::color3ub(8, 20, 80);
            gl::vertex2i(x + w, y);
            gl::color3ub(8, 20, 0);
            gl::vertex2i(x + w, y + h);
            gl::color3ub(8, 20, 80);
            gl::vertex2i(x, y + h);
            gl::end();
        }

        gl::color4f(1.0, 1.0, 1.0, 1.0);

        let current_price = format!("{} c", company.get_share_price(-1));

        let change_in_price = if company.get_share_change() != 0 {
            format!("({}%)", company.get_share_change())
        } else {
            "-".to_string()
        };

        // If we own any, put a * by the name
        if g.interface().ri.security_level < 10 && g.player().num_shares_owned(&companyname) > 0 {
            text::draw_text_default(x + 10, y + 10, "*");
        }

        // Draw the text
        text::draw_text_default(x + 20, y + 10, &company.name);
        text::draw_text_default(x + 200, y + 10, &current_price);

        if company.get_share_change() < 0 {
            gl::color4f(1.0, 0.0, 0.0, 1.0);
        }
        text::draw_text_default(x + 300, y + 10, &change_in_price);

        // Draw a bounding box
        if highlighted {
            gl::color4f(1.0, 1.0, 1.0, 1.0);
            border_draw(button);
        }
    }

    /// `ShareClick` (static MouseFn)
    pub fn share_click(g: &mut Ctx, button: &Button) {
        let baseoffset = screen_ref(g).map_or(0, |s| s.baseoffset);
        let fileindex = scan_share(&button.name) + baseoffset;

        let Some(list) = screen_ref(g) else { return };
        let Some(companyname) = valid_at(&list.filteredlist, fileindex).cloned() else { return };
        let Some(viewpage) = list.get_computer_screen(g).map(|cs| cs.viewpage) else { return };

        RemoteInterface::run_screen(g, viewpage);

        ScreenSlot::call(g, ri_screen_slot, |s, g| {
            if let Some(view) = s.as_any_mut().downcast_mut::<SharesViewScreenInterface>() {
                view.set_company_name(g, &companyname);
            }
        });
    }

    /// `ShareMouseDown` (static MouseFn)
    pub fn share_mouse_down(g: &mut Ctx, button: &Button) {
        if Self::share_is_listed(g, button) {
            button_click(g, button);
        }
    }

    /// `ShareMouseMove` (static MouseFn)
    pub fn share_mouse_move(g: &mut Ctx, button: &Button) {
        if Self::share_is_listed(g, button) {
            button_highlight(g, button);
        }
    }

    /// `FilterDraw` (static DrawFn)
    pub fn filter_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        textbutton_draw(g, button, highlighted, clicked);

        gl::color3f(1.0, 1.0, 1.0);
        border_draw(button);
    }

    /// `FilterClick` (static MouseFn)
    pub fn filter_click(g: &mut Ctx, _button: &Button) {
        let filter = g
            .ecl
            .get_button(FILTER_TEXT)
            .map(|b| b.caption.clone())
            .unwrap_or_default();
        let filter_opt = if filter.is_empty() { None } else { Some(filter.as_str()) };

        ScreenSlot::call(g, ri_screen_slot, |s, g| {
            if let Some(list) = s.as_any_mut().downcast_mut::<SharesListScreenInterface>() {
                list.apply_filter(g, filter_opt);
            }
        });
    }

    /// `ScrollUpClick` (static MouseFn)
    pub fn scroll_up_click(g: &mut Ctx, _button: &Button) {
        if let Some(list) = screen_mut(g) {
            list.baseoffset = (list.baseoffset - 1).max(0);
        }
        for i in 0..SHARE_ROWS {
            g.ecl.dirty_button(&share_name(i));
        }
    }

    /// `ScrollDownClick` (static MouseFn)
    pub fn scroll_down_click(g: &mut Ctx, _button: &Button) {
        if let Some(list) = screen_mut(g) {
            list.baseoffset += 1;
        }
        for i in 0..SHARE_ROWS {
            g.ecl.dirty_button(&share_name(i));
        }
    }

    /// `CloseClick` (static MouseFn)
    pub fn close_click(g: &mut Ctx, _button: &Button) {
        let Some(list) = screen_ref(g) else { return };
        let Some(nextpage) = list.get_computer_screen(g).map(|cs| cs.nextpage) else { return };

        if nextpage != -1 {
            RemoteInterface::run_screen(g, nextpage);
        }
    }
}

impl InterfaceScreen for SharesListScreenInterface {
    crate::impl_as_any!();

    fn create(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            return;
        }

        let (maintitle, subtitle, nextpage, screentype) = {
            let Some(screen) = self.get_computer_screen(g) else { return };
            (
                screen.base.maintitle.clone(),
                screen.base.subtitle.clone(),
                screen.nextpage,
                screen.screentype,
            )
        };

        // Draw the screen titles
        g.ecl.register_button(80, 60, 350, 25, &maintitle, "", "shareslistscreen_maintitle");
        g.ecl.register_button_callbacks("shareslistscreen_maintitle", Some(draw_main_title), None, None, None);
        g.ecl.register_button(80, 80, 350, 20, &subtitle, "", "shareslistscreen_subtitle");
        g.ecl.register_button_callbacks("shareslistscreen_subtitle", Some(draw_sub_title), None, None, None);

        g.ecl.register_button(30, 120, 176, 15, "Company name", "", "shareslistscreen_companytitle");
        g.ecl.register_button(210, 120, 96, 15, "Price per share", "", "shareslistscreen_pricetitle");
        g.ecl.register_button(310, 120, 70, 15, "Change (Qtr)", "", "shareslistscreen_changetitle");
        g.ecl.register_button(390, 120, 15, 15, "", "", "shareslistscreen_topright");

        // Create the share entries
        for li in 0..SHARE_ROWS {
            let name = share_name(li);
            g.ecl.register_button(30, 145 + li * 15, 350, 14, "", "View this share in more detail", &name);
            g.ecl.register_button_callbacks(
                &name,
                Some(Self::share_draw),
                Some(Self::share_click),
                Some(Self::share_mouse_down),
                Some(Self::share_mouse_move),
            );
        }

        // Set the full list depending on the screen type. USERSHARES has nothing to fill
        // (the C++ branch is empty).
        if screentype == SHARESLISTSCREENTYPE_ALLSHARES {
            // companies.ConvertToDArray: BTree pre-order
            let allshares: Vec<String> = g
                .world()
                .companies
                .values()
                .filter(|c| matches!(c.type_, COMPANYTYPE_INDUSTRIAL | COMPANYTYPE_COMMERCIAL | COMPANYTYPE_FINANCIAL))
                .map(|c| c.name.clone())
                .collect();
            self.set_full_list(g, &allshares);
        }

        if self.fulllist.len() >= SHARE_ROWS as usize {
            // Create the scrollbar
            g.ecl.register_button(390, 145, 15, 15, "^", "Scroll upwards", "shareslistscreen_scrollup");
            g.ecl.register_button(390, 162, 15, 12 * 15 - 4, "", "", "shareslistscreen_scrollbar");
            g.ecl.register_button(390, 145 + 13 * 15, 15, 15, "v", "Scroll downwards", "shareslistscreen_scrolldown");

            button_assignbitmaps(g, "shareslistscreen_scrollup", "up.tif", "up_h.tif", "up_c.tif");
            button_assignbitmaps(g, "shareslistscreen_scrolldown", "down.tif", "down_h.tif", "down_c.tif");
            g.ecl.register_button_callback("shareslistscreen_scrollup", Self::scroll_up_click);
            g.ecl.register_button_callback("shareslistscreen_scrolldown", Self::scroll_down_click);

            // Create the filter buttons
            g.ecl.register_button(30, 365, 120, 15, "Filter", "Click here to apply the filter", "shareslistscreen_filter");
            g.ecl.register_button_callback("shareslistscreen_filter", Self::filter_click);

            g.ecl.register_button(155, 365, 200, 15, "", "Enter filter pattern here", FILTER_TEXT);
            g.ecl.register_button_callbacks(
                FILTER_TEXT,
                Some(Self::filter_draw),
                None,
                Some(button_click),
                Some(button_highlight),
            );

            g.ecl.make_button_editable(FILTER_TEXT);
        }

        // Create the exit button
        if nextpage != -1 {
            g.ecl.register_button(360, 365, 45, 15, "Close", "Close this screen", "shareslistscreen_close");
            g.ecl.register_button_callback("shareslistscreen_close", Self::close_click);
        }

        self.apply_filter(g, None);
    }

    fn remove(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            return;
        }

        for name in [
            "shareslistscreen_maintitle",
            "shareslistscreen_subtitle",
            "shareslistscreen_companytitle",
            "shareslistscreen_pricetitle",
            "shareslistscreen_changetitle",
            "shareslistscreen_topright",
            "shareslistscreen_filter",
            FILTER_TEXT,
        ] {
            g.ecl.remove_button(name);
        }

        for i in 0..SHARE_ROWS {
            g.ecl.remove_button(&share_name(i));
        }

        if g.ecl.get_button("shareslistscreen_scrollbar").is_some() {
            g.ecl.remove_button("shareslistscreen_scrollup");
            g.ecl.remove_button("shareslistscreen_scrollbar");
            g.ecl.remove_button("shareslistscreen_scrolldown");
        }

        g.ecl.remove_button("shareslistscreen_close");
    }

    fn update(&mut self, _g: &mut Ctx) {}

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("shareslistscreen_maintitle").is_some()
    }

    fn screen_id(&self) -> i32 {
        SCREEN_SHARESLISTSCREEN
    }

    /// `ReturnKeyPressed`: applies the filter text. Handled only when the filter box exists.
    fn return_key_pressed(&mut self, g: &mut Ctx) -> bool {
        let Some(filter) = g.ecl.get_button(FILTER_TEXT).map(|b| b.caption.clone()) else {
            return false;
        };
        let filter_opt = if filter.is_empty() { None } else { Some(filter.as_str()) };
        self.apply_filter(g, filter_opt);
        true
    }

    fn set_computer_screen(&mut self, cs: ScreenRef) {
        self.cs = Some(cs);
    }

    fn computer_screen_ref(&self) -> Option<&ScreenRef> {
        self.cs.as_ref()
    }
}
