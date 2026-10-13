//! Port of source/interface/remoteinterface/swsalesscreen_interface.h + source/interface/remoteinterface/swsalesscreen_interface.cpp

use uplink_engine::{gl, sound, text};

use crate::app::opengl_interface::{border_draw, button_click, button_highlight, clear_draw, set_colour, sy, text_draw};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::remoteinterface::remoteinterface::{RemoteInterface, SCREEN_SWSALESSCREEN};
use crate::interface::scrollbox::ScrollBox;
use crate::objref::ScreenRef;
use crate::world::company::companyuplink::CompanyUplink;
use crate::world::computer::computerscreen::genericscreen::GenericScreen;
use crate::world::computer::databank::{
    Data, DATATYPE_PROGRAM, SOFTWARETYPE_BYPASSER, SOFTWARETYPE_HUDUPGRADE, SOFTWARETYPE_LANTOOL,
};
use crate::world::computer::gateway::{
    HUDUPGRADE_CONNECTIONANALYSIS, HUDUPGRADE_IRCCLIENT, HUDUPGRADE_LANVIEW, HUDUPGRADE_MAPSHOWSTRACE,
};
use crate::world::person::Person;

/// Remote software sales screen: lists software for sale, with version selection.
#[derive(Debug)]
pub struct SWSalesScreenInterface {
    pub cs: Option<ScreenRef>,
    pub baseoffset: i32,
    /// Selected software item (-1 when none).
    pub currentselect: i32,
    /// Selected version of the software item (-1 when none).
    pub currentversion: i32,
}

impl Default for SWSalesScreenInterface {
    fn default() -> Self {
        SWSalesScreenInterface {
            cs: None,
            baseoffset: 0,
            currentselect: -1,
            currentversion: -1,
        }
    }
}

/// Name of list row `i` (`"SWsale %d"` in C++).
fn row_name(i: i32) -> String {
    format!("SWsale {i}")
}

/// `sscanf(name, "SWsale %d", &index)`. PORT: C++ leaves `index` unset when the name does not
/// match; 0 is used instead. Every caller passes a row button name, so this never happens.
fn scan_row(name: &str) -> i32 {
    name.strip_prefix("SWsale ")
        .and_then(|rest| rest.trim_start().parse::<i32>().ok())
        .unwrap_or(0)
}

/// The screen as this type, when it is the one currently shown.
fn screen_mut(g: &mut Ctx) -> Option<&mut SWSalesScreenInterface> {
    g.interface_mut().ri.screen.get_mut::<SWSalesScreenInterface>()
}

/// `(baseoffset, currentselect, currentversion)` of the shown screen. PORT: C++ keeps these as
/// statics; callers fall back to their initial values when no such screen is shown.
fn selection(g: &Ctx) -> Option<(i32, i32, i32)> {
    g.interface()
        .ri
        .screen
        .get::<SWSalesScreenInterface>()
        .map(|s| (s.baseoffset, s.currentselect, s.currentversion))
}

/// The Uplink company's software sales (`GetCompany("Uplink")`, C++ `UplinkAssert`).
fn uplink_company(g: &Ctx) -> Option<&CompanyUplink> {
    g.world().get_company("Uplink")?.uplink.as_deref()
}

/// One `glBegin(GL_QUADS)` block: four corners, each drawn in its own named theme colour.
fn quad_corners(g: &Ctx, corners: [(&str, i32, i32); 4]) {
    gl::begin(gl::QUADS);
    for (colour, x, y) in corners {
        set_colour(g, colour);
        gl::vertex2i(x, y);
    }
    gl::end();
}

/// The `ExitClick` body: runs the generic screen's next page.
fn exit_to_next_page(g: &mut Ctx) {
    let Some(next) = RemoteInterface::get_computer_screen(g)
        .and_then(|cs| cs.as_generic_screen())
        .map(|gs| gs.nextpage)
    else {
        return;
    };
    RemoteInterface::run_screen(g, next);
}

impl SWSalesScreenInterface {
    /// `GetComputerScreen` (typed accessor; the C++ casts `cs`).
    pub fn get_computer_screen<'a>(&self, g: &'a Ctx) -> Option<&'a GenericScreen> {
        let cs = self.cs.as_ref()?;
        g.world()
            .get_computer(&cs.computer)?
            .get_computer_screen(cs.index)?
            .as_generic_screen()
    }

    /// Number of list rows that fit on the screen.
    pub fn num_items_on_screen(g: &Ctx) -> i32 {
        let screenheight = g.app.options.get_option_value("graphics_screenheight");
        let available_pixels = screenheight - 200 - 45;
        available_pixels / 20
    }

    /// `DrawSWButton` (static DrawFn)
    pub fn draw_sw_button(g: &mut Ctx, button: &Button, highlighted: bool, _clicked: bool) {
        clear_draw(g, button.x, button.y, button.width, button.height);

        let (baseoffset, currentselect, currentversion) = selection(g).unwrap_or((0, -1, -1));
        let index = scan_row(&button.name) + baseoffset;

        let Some(cu) = uplink_company(g) else { return };
        let Some(sale) = cu.get_sw_sale(index) else { return };
        let sv = if index == currentselect {
            sale.get_version(currentversion)
        } else {
            sale.get_version(1)
        };
        let Some(sv) = sv else { return };

        let (x, y, w, h) = (button.x, button.y, button.width, button.height);

        if index == currentselect {
            quad_corners(
                g,
                [
                    ("PanelHighlightA", x, y),
                    ("PanelHighlightB", x + w, y),
                    ("PanelHighlightA", x + w, y + h),
                    ("PanelHighlightB", x, y + h),
                ],
            );
        }

        if highlighted || index == currentselect {
            set_colour(g, "PanelBorder");
            border_draw(button);
        }

        let hasmoney = g.player().get_balance(g.world()) >= sv.cost;
        set_colour(g, if hasmoney { "DefaultText" } else { "DimmedText" });

        text::draw_text_default(x + 5, y + 10, &sale.title);
        text::draw_text_default(x + 240, y + 10, &format!("v{}.0", sv.data));
        text::draw_text_default(x + 270, y + 10, &format!("{}c", sv.cost));
        text::draw_text_default(x + 335, y + 10, &format!("{}Gq", sv.size));
    }

    /// `ClickSWButton` (static MouseFn)
    pub fn click_sw_button(g: &mut Ctx, button: &Button) {
        let (baseoffset, currentselect, _) = selection(g).unwrap_or((0, -1, -1));
        let index = scan_row(&button.name) + baseoffset;

        // Dirty the old highlighted button
        g.ecl.dirty_button(&row_name(currentselect - baseoffset));

        let details = uplink_company(g)
            .and_then(|cu| cu.get_sw_sale(index))
            .and_then(|sale| sale.get_version(1))
            .map(|sv| sv.details.clone().unwrap_or_default());
        let Some(details) = details else { return };

        if let Some(state) = screen_mut(g) {
            state.currentselect = index;
            state.currentversion = 1;
        }
        g.ecl.register_caption_change("swsales_details", &details, 2000, None);
    }

    /// `MouseDownSWButton` (static MouseFn)
    pub fn mousedown_sw_button(g: &mut Ctx, button: &Button) {
        let baseoffset = selection(g).map_or(0, |s| s.0);
        let index = scan_row(&button.name);
        let exists = uplink_company(g)
            .and_then(|cu| cu.get_sw_sale(index + baseoffset))
            .is_some();
        if exists {
            button_click(g, button);
        }
    }

    /// `HighlightSWButton` (static MouseFn)
    pub fn highlight_sw_button(g: &mut Ctx, button: &Button) {
        let baseoffset = selection(g).map_or(0, |s| s.0);
        let index = scan_row(&button.name);
        let exists = uplink_company(g)
            .and_then(|cu| cu.get_sw_sale(index + baseoffset))
            .is_some();
        if exists {
            button_highlight(g, button);
        }
    }

    /// `DrawDetails` (static DrawFn)
    pub fn draw_details(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        let (x, y, w, h) = (button.x, button.y, button.width, button.height);
        quad_corners(
            g,
            [
                ("PanelBackgroundA", x, y + h),
                ("PanelBackgroundB", x, y),
                ("PanelBackgroundA", x + w, y),
                ("PanelBackgroundB", x + w, y + h),
            ],
        );

        set_colour(g, "PanelBorder");
        border_draw(button);

        text_draw(g, button, highlighted, clicked);
    }

    /// `AcceptClick` (static MouseFn)
    pub fn accept_click(g: &mut Ctx, _button: &Button) {
        let (_, currentselect, currentversion) = selection(g).unwrap_or((0, -1, -1));

        // Copy what the purchase needs out of the sale, so the company borrow ends before the
        // purchase changes the player.
        let Some((title, swhwtype, size, data, cost)) = uplink_company(g)
            .and_then(|cu| cu.get_sw_sale(currentselect))
            .and_then(|sale| {
                sale.get_version(currentversion)
                    .map(|sv| (sale.title.clone(), sale.swhwtype, sv.size, sv.data, sv.cost))
            })
        else {
            return;
        };

        let mut newsw = Data::new();
        newsw.set_title(&title);
        newsw.set_details(DATATYPE_PROGRAM, size, 0, 0, data as f32, swhwtype);

        // Check the player has space and money
        let memoryindex = g.player().gateway.databank.find_valid_placement(&newsw);
        let hasmoney = g.player().get_balance(g.world()) >= cost;

        if memoryindex != -1 && hasmoney {
            if swhwtype == SOFTWARETYPE_HUDUPGRADE {
                // Special case - HUD upgrades. The upgrade is granted; the program is not stored.
                let upgrade = match title.as_str() {
                    "HUD_MapShowTrace" => HUDUPGRADE_MAPSHOWSTRACE,
                    "HUD_ConnectionAnalysis" => HUDUPGRADE_CONNECTIONANALYSIS,
                    "HUD_IRC-Client" => HUDUPGRADE_IRCCLIENT,
                    "HUD_LANView" => HUDUPGRADE_LANVIEW,
                    _ => panic!("Unrecognised HUD upgrade"),
                };
                g.player_mut().gateway.give_hud_upgrade(upgrade);
            } else if swhwtype == SOFTWARETYPE_BYPASSER {
                // Special case - Bypass software (requires HUD_ConnectionAnalysis)
                if !g.player().gateway.has_hud_upgrade(HUDUPGRADE_CONNECTIONANALYSIS) {
                    sound::play_sound("sounds/failure.wav", Some("sounds/failure.wav"), true);
                    g.ecl.register_caption_change_auto(
                        "swsales_details",
                        "You require a HUD_ConnectionAnalysis upgrade before purchasing any bypasser.",
                        None,
                    );
                    return;
                }
                g.player_mut().gateway.databank.put_data_at(newsw, memoryindex);
            } else if swhwtype == SOFTWARETYPE_LANTOOL {
                // Special case - Lan Tool (requires HUD_LanView)
                if !g.player().gateway.has_hud_upgrade(HUDUPGRADE_LANVIEW) {
                    sound::play_sound("sounds/failure.wav", Some("sounds/failure.wav"), true);
                    g.ecl.register_caption_change_auto(
                        "swsales_details",
                        "You require a HUD_LanView upgrade before purchasing any LAN tools.",
                        None,
                    );
                    return;
                }
                g.player_mut().gateway.databank.put_data_at(newsw, memoryindex);
            } else {
                g.player_mut().gateway.databank.put_data_at(newsw, memoryindex);
            }

            // Go ahead and buy it
            Person::change_balance(g, "PLAYER", -cost, "Uplink Corporation Software sales");

            g.ecl.register_caption_change_auto("swsales_details", "Item purchased", None);

            // Force refresh of each button (C++ literal 12 rows)
            for i in 0..12 {
                g.ecl.dirty_button(&row_name(i));
            }
        } else if memoryindex == -1 {
            // Not enough space
            sound::play_sound("sounds/failure.wav", Some("sounds/failure.wav"), true);
            g.ecl.register_caption_change_auto(
                "swsales_details",
                "You do not have enough memory to store this program.",
                None,
            );
        } else {
            // Not enough money
            sound::play_sound("sounds/failure.wav", Some("sounds/failure.wav"), true);
            g.ecl.register_caption_change_auto(
                "swsales_details",
                "You do not have enough credits to purchase this program.",
                None,
            );
        }
    }

    /// `NextVersionClick` (static MouseFn)
    pub fn next_version_click(g: &mut Ctx, _button: &Button) {
        let (baseoffset, currentselect, currentversion) = selection(g).unwrap_or((0, -1, -1));

        let next_details = uplink_company(g)
            .and_then(|cu| cu.get_sw_sale(currentselect))
            .and_then(|sale| sale.get_version(currentversion + 1))
            .map(|sv| sv.details.clone().unwrap_or_default());
        let Some(details) = next_details else { return };

        if let Some(state) = screen_mut(g) {
            state.currentversion += 1;
        }
        g.ecl.register_caption_change("swsales_details", &details, 2000, None);
        g.ecl.dirty_button(&row_name(currentselect - baseoffset));
    }

    /// `PrevVersionClick` (static MouseFn)
    pub fn prev_version_click(g: &mut Ctx, _button: &Button) {
        let (baseoffset, currentselect, currentversion) = selection(g).unwrap_or((0, -1, -1));
        if currentversion <= 1 {
            return;
        }

        let prev_details = uplink_company(g)
            .and_then(|cu| cu.get_sw_sale(currentselect))
            .and_then(|sale| sale.get_version(currentversion - 1))
            .map(|sv| sv.details.clone().unwrap_or_default());
        let Some(details) = prev_details else { return };

        if let Some(state) = screen_mut(g) {
            state.currentversion -= 1;
        }
        g.ecl.register_caption_change("swsales_details", &details, 2000, None);
        g.ecl.dirty_button(&row_name(currentselect - baseoffset));
    }

    /// `ExitClick` (static MouseFn)
    pub fn exit_click(g: &mut Ctx, _button: &Button) {
        exit_to_next_page(g);
    }

    /// `ScrollChange` (ScrollBox callback: scroll bar name and new value)
    pub fn scroll_change(g: &mut Ctx, _scrollname: &str, new_value: i32) {
        if let Some(state) = screen_mut(g) {
            state.baseoffset = new_value;
        }
        for i in 0..Self::num_items_on_screen(g) {
            g.ecl.dirty_button(&row_name(i));
        }
    }
}

impl InterfaceScreen for SWSalesScreenInterface {
    crate::impl_as_any!();

    fn create(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            return;
        }

        let num_items = Self::num_items_on_screen(g);

        g.ecl.register_button(20, 30, 237, 15, "Name", "This column shows the name of the item for sale", "swsales_name");
        g.ecl.register_button(260, 30, 22, 15, "Ver", "This column shows the version number of the software", "swsales_version");
        g.ecl.register_button(285, 30, 60, 15, "Cost", "This column shows the cost of the item in credits", "swsales_cost");
        g.ecl.register_button(350, 30, 57, 15, "Size", "This column shows the space required for this item", "swsales_requires");
        g.ecl.register_button(412, 30, 20 + sy(390) - 395, 15, "", "", "swsales_padding");

        for i in 0..num_items {
            let name = row_name(i);
            g.ecl.register_button(20, i * 20 + 50, sy(390), 17, "", "View details on this item", &name);
            g.ecl.register_button_callbacks(
                &name,
                Some(Self::draw_sw_button),
                Some(Self::click_sw_button),
                Some(Self::mousedown_sw_button),
                Some(Self::highlight_sw_button),
            );
        }

        let base_x = 23 + sy(388) - 55;
        let base_y = 50 + num_items * 20;

        g.ecl.register_button(20, base_y, sy(388) - 55, 110, "", "", "swsales_details");
        g.ecl.register_button_callbacks("swsales_details", Some(Self::draw_details), None, None, None);

        g.ecl.register_button(base_x, base_y, 72, 15, "Purchase", "Purchase this item", "swsales_accept");
        g.ecl.register_button_callback("swsales_accept", Self::accept_click);

        g.ecl.register_button(base_x, base_y + 20, 72, 15, "Next Version", "View the next version", "swsales_nextver");
        g.ecl.register_button_callback("swsales_nextver", Self::next_version_click);

        g.ecl.register_button(base_x, base_y + 40, 72, 15, "Prev Version", "View the previous version", "swsales_prevver");
        g.ecl.register_button_callback("swsales_prevver", Self::prev_version_click);

        g.ecl.register_button(base_x, base_y + 110 - 15, 72, 15, "Exit Market", "Close the screen and return to the main menu", "swsales_exit");
        g.ecl.register_button_callback("swsales_exit", Self::exit_click);

        let num_sales = uplink_company(g).map_or(0, |cu| cu.sw_sales.len() as i32);
        ScrollBox::create_scroll_box(
            g,
            "swsales_scroll",
            22 + sy(390),
            47,
            15,
            num_items * 20,
            num_sales,
            num_items,
            0,
            Some(Self::scroll_change),
        );

        self.baseoffset = 0;
        self.currentselect = -1;
    }

    fn remove(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            return;
        }

        for i in 0..Self::num_items_on_screen(g) {
            g.ecl.remove_button(&row_name(i));
        }

        for name in [
            "swsales_details",
            "swsales_name",
            "swsales_version",
            "swsales_cost",
            "swsales_requires",
            "swsales_padding",
            "swsales_accept",
            "swsales_nextver",
            "swsales_prevver",
            "swsales_exit",
        ] {
            g.ecl.remove_button(name);
        }

        ScrollBox::remove_scroll_box(g, "swsales_scroll");
    }

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("swsales_exit").is_some()
    }

    fn screen_id(&self) -> i32 {
        SCREEN_SWSALESSCREEN
    }

    /// `EscapeKeyPressed`: same as the exit button, and always reports the key as handled.
    fn escape_key_pressed(&mut self, g: &mut Ctx) -> bool {
        if let Some(next) = self.get_computer_screen(g).map(|gs| gs.nextpage) {
            RemoteInterface::run_screen(g, next);
        }
        true
    }

    fn set_computer_screen(&mut self, cs: ScreenRef) {
        self.cs = Some(cs);
    }

    fn computer_screen_ref(&self) -> Option<&ScreenRef> {
        self.cs.as_ref()
    }
}
