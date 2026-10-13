//! Port of source/interface/remoteinterface/hwsalesscreen_interface.h + source/interface/remoteinterface/hwsalesscreen_interface.cpp

use uplink_engine::{gl, text};

use crate::app::opengl_interface::{
    border_draw, button_assignbitmaps, button_click, button_highlight, clear_draw, set_colour,
    text_draw,
};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::game::data::data::{TIME_TOINSTALLHARDWARE, TIME_TOINSTALLHARDWARE_WARNING};
use crate::interface::interfacescreen::{InterfaceScreen, ScreenSlot};
use crate::interface::remoteinterface::remoteinterface::{RemoteInterface, SCREEN_HWSALESSCREEN};
use crate::interface::remoteinterface::remoteinterfacescreen::draw_main_title;
use crate::objref::ScreenRef;
use crate::world::company::sale::Sale;
use crate::world::computer::computerscreen::genericscreen::GenericScreen;
use crate::world::computer::gateway::{
    GATEWAYUPGRADETYPE_MEMORY, GATEWAYUPGRADETYPE_MODEM, GATEWAYUPGRADETYPE_SECURITY,
};
use crate::world::date::Date;
use crate::world::person::Person;
use crate::world::scheduler::installhardwareevent::InstallHardwareEvent;
use crate::world::scheduler::uplinkevent::UplinkEvent;

/// Number of sale rows shown at once (C++ literal 12 in the loops).
pub const NUM_SALE_ROWS: i32 = 12;

/// Hardware sales screen: a menu of upgrade categories (`hw_type == -1`), then a list of the
/// Uplink company's sales for one category with a details panel and a purchase button.
#[derive(Debug)]
pub struct HWSalesScreenInterface {
    pub cs: Option<ScreenRef>,
    /// What type of hardware upgrade is being bought (-1 = the category menu).
    pub hw_type: i32,
    /// Sales of the current type, as indices into the Uplink company's `hw_sales` list.
    /// PORT: C++ held non-owning `Sale *` pointers; the index is the stable key instead.
    pub items: Vec<i32>,
    // PORT: these were `static` members in C++ (shared by every instance). They are kept per
    // instance here; both are reset whenever the sales menu is built.
    /// First sale index shown in the list (scroll offset).
    pub baseoffset: i32,
    /// Index of the sale whose details are showing (-1 = none).
    pub currentselect: i32,
}

impl Default for HWSalesScreenInterface {
    fn default() -> Self {
        HWSalesScreenInterface {
            cs: None,
            hw_type: -1,
            items: Vec::new(),
            baseoffset: 0,
            currentselect: -1,
        }
    }
}

/// The remote interface's screen slot (the screen is taken out of it while one of its methods runs).
fn remote_screen_slot(g: &mut Ctx) -> &mut ScreenSlot {
    &mut g.interface_mut().ri.screen
}

/// Runs `f` on this screen with the screen taken out of its slot (see `ScreenSlot::call`), so
/// `f` may use `g` freely. Does nothing if this screen is not the current remote screen.
fn with_screen(g: &mut Ctx, f: impl FnOnce(&mut HWSalesScreenInterface, &mut Ctx)) {
    ScreenSlot::call(g, remote_screen_slot, |s, g| {
        if let Some(me) = s.as_any_mut().downcast_mut::<HWSalesScreenInterface>() {
            f(me, g);
        }
    });
}

/// The current remote screen (C++ `GetInterfaceScreen` + `UplinkAssert`).
fn current(g: &Ctx) -> &HWSalesScreenInterface {
    g.interface()
        .ri
        .screen
        .get::<HWSalesScreenInterface>()
        .expect("HWSales callback without an HWSales screen")
}

/// Reads the number after `prefix` in a button name (C++ `sscanf(button->name, "... %d", ...)`).
fn name_index(button: &Button, prefix: &str) -> i32 {
    button
        .name
        .strip_prefix(prefix)
        .and_then(|s| s.trim().parse().ok())
        .expect("HWSales button name without an index")
}

/// Dirties every sale row button (C++ loop of `EclDirtyButton("HWsale i")`).
fn dirty_sale_rows(g: &mut Ctx) {
    for i in 0..NUM_SALE_ROWS {
        g.ecl.dirty_button(&format!("HWsale {i}"));
    }
}

impl HWSalesScreenInterface {
    /// `SetHWType`: picks the category shown (`-1` for the menu).
    pub fn set_hw_type(&mut self, new_type: i32) {
        self.hw_type = new_type;
    }

    /// `CreateMenu`: builds the category menu buttons.
    pub fn create_menu(&mut self, g: &mut Ctx) {
        if self.is_visible_menu(g) {
            return;
        }

        // Processors

        g.ecl.register_button(80, 180, 32, 32, " ", "Show new Processors", "hwsales_showmenu 1");
        button_assignbitmaps(
            g,
            "hwsales_showmenu 1",
            "upgrades/cpu.tif",
            "upgrades/cpu_h.tif",
            "upgrades/cpu_c.tif",
        );
        g.ecl.register_button(115, 180, 100, 15, "Processors", " ", "hwsales_caption 1");

        // Modems

        g.ecl.register_button(80, 230, 32, 32, " ", "Show new Modems", "hwsales_showmenu 2");
        button_assignbitmaps(
            g,
            "hwsales_showmenu 2",
            "upgrades/modem.tif",
            "upgrades/modem_h.tif",
            "upgrades/modem_c.tif",
        );
        g.ecl.register_button(115, 230, 100, 15, "Modems", " ", "hwsales_caption 2");

        // Cooling (commented out in the C++)

        // Memory

        g.ecl.register_button(280, 180, 32, 32, " ", "Show Memory upgrades", "hwsales_showmenu 4");
        button_assignbitmaps(
            g,
            "hwsales_showmenu 4",
            "upgrades/memory.tif",
            "upgrades/memory_h.tif",
            "upgrades/memory_c.tif",
        );
        g.ecl.register_button(315, 180, 100, 15, "Memory", " ", "hwsales_caption 4");

        // Security

        g.ecl.register_button(280, 230, 32, 32, " ", "Show Security upgrades", "hwsales_showmenu 5");
        button_assignbitmaps(
            g,
            "hwsales_showmenu 5",
            "upgrades/security.tif",
            "upgrades/security_h.tif",
            "upgrades/security_c.tif",
        );
        g.ecl.register_button(315, 230, 100, 15, "Security", " ", "hwsales_caption 5");

        // Power (commented out in the C++)

        g.ecl.register_button(280, 320, 32, 32, " ", "Return to the main menu", "hwsales_exit");
        button_assignbitmaps(
            g,
            "hwsales_exit",
            "upgrades/exit.tif",
            "upgrades/exit_h.tif",
            "upgrades/exit_c.tif",
        );
        g.ecl.register_button(315, 320, 100, 15, "Exit", " ", "hwsales_exitcaption");
        g.ecl.register_button_callback("hwsales_exit", Self::exit_button_click);
        g.ecl.register_button_callbacks("hwsales_exitcaption", Some(draw_main_title), None, None, None);

        g.ecl.register_button_callback("hwsales_showmenu 1", Self::show_sales_menu_click);
        g.ecl.register_button_callback("hwsales_showmenu 2", Self::show_sales_menu_click);
        g.ecl.register_button_callback("hwsales_showmenu 3", Self::show_sales_menu_click);
        g.ecl.register_button_callback("hwsales_showmenu 4", Self::show_sales_menu_click);
        g.ecl.register_button_callback("hwsales_showmenu 5", Self::show_sales_menu_click);
        // "hwsales_showmenu 6" (power) is commented out in the C++

        g.ecl.register_button_callbacks("hwsales_caption 1", Some(draw_main_title), None, None, None);
        g.ecl.register_button_callbacks("hwsales_caption 2", Some(draw_main_title), None, None, None);
        g.ecl.register_button_callbacks("hwsales_caption 3", Some(draw_main_title), None, None, None);
        g.ecl.register_button_callbacks("hwsales_caption 4", Some(draw_main_title), None, None, None);
        g.ecl.register_button_callbacks("hwsales_caption 5", Some(draw_main_title), None, None, None);
        // "hwsales_caption 6" is commented out in the C++
    }

    /// `CreateSalesMenu`: fills `items` with the sales of `hw_type`, then builds the list,
    /// details panel, purchase and scroll buttons.
    pub fn create_sales_menu(&mut self, g: &mut Ctx) {
        if self.is_visible_sales_menu(g) {
            return;
        }

        //
        // Populate the "items" array with all items of this type
        //

        self.items.clear();
        let cu = g
            .world()
            .get_company("Uplink")
            .and_then(|c| c.uplink.as_deref())
            .expect("Uplink company has no CompanyUplink data");
        for (ic, sale) in cu.hw_sales.iter().enumerate() {
            if sale.swhwtype == self.hw_type {
                self.items.push(ic as i32);
            }
        }

        g.ecl.register_button(20, 30, 328, 15, "Name", "This column shows the name of the item for sale", "hwsales_name");
        g.ecl.register_button(350, 30, 60, 15, "Cost", "This column shows the cost of the item in credits", "hwsales_cost");
        g.ecl.register_button(412, 30, 15, 15, "", "", "hwsales_padding");

        for i in 0..NUM_SALE_ROWS {
            let name = format!("HWsale {i}");
            g.ecl.register_button(20, i * 20 + 50, 390, 17, "", "View details on this item", &name);
            g.ecl.register_button_callbacks(
                &name,
                Some(Self::draw_hw_button),
                Some(Self::click_hw_button),
                Some(Self::mousedown_hw_button),
                Some(Self::highlight_hw_button),
            );
        }

        g.ecl.register_button(20, 290, 330, 110, "", "", "hwsales_details");
        g.ecl.register_button_callbacks("hwsales_details", Some(Self::draw_details), None, None, None);

        g.ecl.register_button(355, 290, 72, 15, "Purchase", "Purchase this item", "hwsales_accept");
        g.ecl.register_button_callback("hwsales_accept", Self::accept_click);
        g.ecl.register_button(355, 387, 72, 15, "Exit Market", "Close the screen and return to the hardware menu", "hwsales_exit");
        g.ecl.register_button_callback("hwsales_exit", Self::exit_button_click);

        g.ecl.register_button(412, 50, 15, 15, "^", "Scroll upwards", "hwsales_scrollup");
        button_assignbitmaps(g, "hwsales_scrollup", "up.tif", "up_h.tif", "up_c.tif");
        g.ecl.register_button_callback("hwsales_scrollup", Self::scroll_up_click);

        g.ecl.register_button(412, 270, 15, 15, "v", "Scroll downwards", "hwsales_scrolldown");
        button_assignbitmaps(g, "hwsales_scrolldown", "down.tif", "down_h.tif", "down_c.tif");
        g.ecl.register_button_callback("hwsales_scrolldown", Self::scroll_down_click);

        g.ecl.register_button(412, 67, 15, 201, "", "", "hwsales_scrollbar");

        self.baseoffset = 0;
        self.currentselect = -1;
    }

    /// `RemoveMenu`
    pub fn remove_menu(&mut self, g: &mut Ctx) {
        for name in [
            "hwsales_showmenu 1",
            "hwsales_showmenu 2",
            "hwsales_showmenu 3",
            "hwsales_showmenu 4",
            "hwsales_showmenu 5",
            // "hwsales_showmenu 6" is commented out in the C++
            "hwsales_caption 1",
            "hwsales_caption 2",
            "hwsales_caption 3",
            "hwsales_caption 4",
            "hwsales_caption 5",
            // "hwsales_caption 6" is commented out in the C++
            "hwsales_exit",
            "hwsales_exitcaption",
        ] {
            g.ecl.remove_button(name);
        }
    }

    /// `RemoveSalesMenu`
    pub fn remove_sales_menu(&mut self, g: &mut Ctx) {
        for i in 0..NUM_SALE_ROWS {
            g.ecl.remove_button(&format!("HWsale {i}"));
        }

        g.ecl.remove_button("hwsales_details");

        g.ecl.remove_button("hwsales_name");
        g.ecl.remove_button("hwsales_cost");
        g.ecl.remove_button("hwsales_padding");

        g.ecl.remove_button("hwsales_scrollup");
        g.ecl.remove_button("hwsales_scrolldown");
        g.ecl.remove_button("hwsales_scrollbar");

        g.ecl.remove_button("hwsales_accept");
        g.ecl.remove_button("hwsales_exit");
    }

    /// `IsVisibleMenu`
    pub fn is_visible_menu(&self, g: &Ctx) -> bool {
        g.ecl.get_button("hwsales_showmenu 1").is_some()
    }

    /// `IsVisibleSalesMenu`
    pub fn is_visible_sales_menu(&self, g: &Ctx) -> bool {
        // C++ quirk kept: "hwsales_exit" also exists on the category menu.
        g.ecl.get_button("hwsales_exit").is_some()
    }

    /// `GetComputerScreen` (typed accessor; the C++ casts `cs`).
    pub fn get_computer_screen<'a>(&self, g: &'a Ctx) -> Option<&'a GenericScreen> {
        let cs = self.cs.as_ref()?;
        g.world()
            .get_computer(&cs.computer)?
            .get_computer_screen(cs.index)?
            .as_generic_screen()
    }

    /// The sale at position `index` of `items` (C++ `items.GetData(index)`), or None if out of range.
    fn sale_at<'a>(&self, g: &'a Ctx, index: i32) -> Option<&'a Sale> {
        let position = usize::try_from(index).ok()?;
        let hw_index = *self.items.get(position)?;
        let uplink = g.world().get_company("Uplink")?.uplink.as_deref()?;
        uplink.get_hw_sale(hw_index)
    }

    /// `ShowSalesMenuClick` (static MouseFn): opens the sales list for the category in the button name.
    /// PORT: the C++ fetches `this` from the remote interface; the Rust port fetches it from `g`.
    pub fn show_sales_menu_click(g: &mut Ctx, button: &Button) {
        let new_hw_type = name_index(button, "hwsales_showmenu ");

        with_screen(g, |me, g| {
            me.remove_menu(g);
            me.set_hw_type(new_hw_type);

            if new_hw_type == -1 {
                me.create_menu(g);
            } else {
                me.create_sales_menu(g);
            }
        });
    }

    /// `DrawHWButton` (static DrawFn): one sale row with name and cost, dimmed when unaffordable.
    pub fn draw_hw_button(g: &mut Ctx, button: &Button, highlighted: bool, _clicked: bool) {
        clear_draw(g, button.x, button.y, button.width, button.height);

        let me = current(g);
        let index = name_index(button, "HWsale ") + me.baseoffset;

        // Get the text from sale number (index + baseoffset)
        let Some(sale) = me.sale_at(g, index) else {
            return;
        };
        let Some(sv) = sale.get_version(1) else {
            return;
        };

        if index == me.currentselect {
            gl::begin(gl::QUADS);
            set_colour(g, "PanelHighlightA");
            gl::vertex2i(button.x, button.y);
            set_colour(g, "PanelHighlightB");
            gl::vertex2i(button.x + button.width, button.y);
            set_colour(g, "PanelHighlightA");
            gl::vertex2i(button.x + button.width, button.y + button.height);
            set_colour(g, "PanelHighlightB");
            gl::vertex2i(button.x, button.y + button.height);
            gl::end();
        }

        if highlighted || index == me.currentselect {
            set_colour(g, "PanelBorder");
            border_draw(button);
        }

        let hasmoney = g.player().get_balance(g.world()) >= sv.cost;

        if hasmoney {
            set_colour(g, "DefaultText");
        } else {
            set_colour(g, "DimmedText");
        }

        text::draw_text_default(button.x + 5, button.y + 10, &sale.title);
        text::draw_text_default(button.x + 330, button.y + 10, &format!("{}c", sv.cost));
    }

    /// `ClickHWButton` (static MouseFn): selects a sale and shows its details.
    pub fn click_hw_button(g: &mut Ctx, button: &Button) {
        let row = name_index(button, "HWsale ");

        with_screen(g, |me, g| {
            let index = row + me.baseoffset;

            // Dirty the old highlighted button
            g.ecl.dirty_button(&format!("HWsale {}", me.currentselect - me.baseoffset));

            let details = me
                .sale_at(g, index)
                .and_then(|sale| sale.get_version(1))
                .map(|sv| sv.get_details().unwrap_or("").to_string());

            if let Some(details) = details {
                me.currentselect = index;
                g.ecl.register_caption_change("hwsales_details", &details, 2000, None);
            }
        });
    }

    /// `MousedownHWButton` (static MouseFn): only rows that hold a sale respond.
    pub fn mousedown_hw_button(g: &mut Ctx, button: &Button) {
        let me = current(g);
        let has_sale = me.sale_at(g, name_index(button, "HWsale ") + me.baseoffset).is_some();

        if has_sale {
            button_click(g, button);
        }
    }

    /// `HighlightHWButton` (static MouseFn): only rows that hold a sale highlight.
    pub fn highlight_hw_button(g: &mut Ctx, button: &Button) {
        let me = current(g);
        let has_sale = me.sale_at(g, name_index(button, "HWsale ") + me.baseoffset).is_some();

        if has_sale {
            button_highlight(g, button);
        }
    }

    /// `DrawDetails` (static DrawFn): the details panel background and text.
    pub fn draw_details(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        gl::begin(gl::QUADS);
        set_colour(g, "PanelBackgroundA");
        gl::vertex2i(button.x, button.y + button.height);
        set_colour(g, "PanelBackgroundB");
        gl::vertex2i(button.x, button.y);
        set_colour(g, "PanelBackgroundA");
        gl::vertex2i(button.x + button.width, button.y);
        set_colour(g, "PanelBackgroundB");
        gl::vertex2i(button.x + button.width, button.y + button.height);
        gl::end();

        set_colour(g, "PanelBorder");
        border_draw(button);

        text_draw(g, button, highlighted, clicked);
    }

    /// `AcceptClick` (static MouseFn): checks funds and gateway space or bandwidth, charges the
    /// balance and schedules the hardware install event.
    pub fn accept_click(g: &mut Ctx, _button: &Button) {
        with_screen(g, |me, g| me.accept(g));
    }

    /// `AcceptClick` body: runs on the screen while it is taken out of its slot.
    fn accept(&mut self, g: &mut Ctx) {
        let sale = match self.sale_at(g, self.currentselect) {
            Some(sale) => sale.clone(),
            None => return,
        };
        let sv = sale.get_version(1).expect("hardware sale without a version 1");
        let (cost, data) = (sv.cost, sv.data);

        //
        // Check the player has the money
        //

        if g.player().get_balance(g.world()) < cost {
            g.ecl.register_caption_change_auto(
                "hwsales_details",
                "You do not have enough credits to purchase this device.",
                None,
            );
            return;
        }

        //
        // If this is memory, check he has the space
        //

        if sale.swhwtype == GATEWAYUPGRADETYPE_MEMORY {
            let gateway = &g.player().gateway;
            let gd = g
                .world()
                .get_gateway_def(gateway.type_, Some(&gateway.name))
                .expect("player gateway definition missing");

            if gateway.memorysize + data > gd.maxmemory * 8 {
                g.ecl.register_caption_change_auto(
                    "hwsales_details",
                    "You cannot fit this amount of memory in your gateway.",
                    None,
                );
                return;
            }
        }

        //
        // If this is a security device, check he has the space
        //

        if sale.swhwtype == GATEWAYUPGRADETYPE_SECURITY {
            let gateway = &g.player().gateway;
            let gd = g
                .world()
                .get_gateway_def(gateway.type_, Some(&gateway.name))
                .expect("player gateway definition missing");

            if gateway.get_num_security() + 1 > gd.maxsecurity {
                g.ecl.register_caption_change_auto(
                    "hwsales_details",
                    "You cannot fit this security device in your gateway.",
                    None,
                );
                return;
            }
        }

        //
        // If this is a modem, check his Gateway can handle it
        //

        if sale.swhwtype == GATEWAYUPGRADETYPE_MODEM {
            let gateway = &g.player().gateway;
            let gd = g
                .world()
                .get_gateway_def(gateway.type_, Some(&gateway.name))
                .expect("player gateway definition missing");

            if gd.bandwidth < data {
                g.ecl.register_caption_change_auto(
                    "hwsales_details",
                    "Your gateway does not have sufficient bandwidth to run a modem of that speed.",
                    None,
                );
                return;
            }
        }

        // Item purchased

        let player_name = g.player().name.clone();
        Person::change_balance(g, &player_name, -cost, "Uplink hardware sales");

        // Schedule an Install Hardware Event

        let mut duedate = Date::default();
        duedate.set_date_from(&g.world().date);
        duedate.advance_minute(TIME_TOINSTALLHARDWARE);

        let mut warningdate = Date::default();
        warningdate.set_date_from(&duedate);
        warningdate.advance_minute(TIME_TOINSTALLHARDWARE_WARNING * -1);

        let mut event = InstallHardwareEvent::new();
        event.base.set_run_date(&duedate);
        event.set_hw_sale(&sale, 1);
        let event = UplinkEvent::InstallHardwareEvent(event);

        g.world_mut().scheduler.schedule_warning(&event, &warningdate);
        g.world_mut().scheduler.schedule_event(event);

        // Change the caption

        g.ecl.register_caption_change_auto(
            "hwsales_details",
            "Item purchased\n\
             The Hardware will be installed in the next 24 hours.\n\
             You will be notified when the job is completed.",
            None,
        );

        // Force refresh of each button

        dirty_sale_rows(g);
    }

    /// `ExitClick` (static MouseFn). The C++ ignores its button argument, so the escape key
    /// calls this directly; the callback registration needs a small adapter that drops the button.
    /// From the menu it runs the generic screen's next page; from a sales list it returns to the menu.
    pub fn exit_click(g: &mut Ctx) {
        with_screen(g, |me, g| me.exit_page(g));
    }

    /// Registered mouse-up adapter for `exit_click` (the C++ `ExitClick` takes the button, unused).
    fn exit_button_click(g: &mut Ctx, _button: &Button) {
        Self::exit_click(g);
    }

    /// `ExitClick` body: runs on the screen while it is taken out of its slot (also the escape key).
    fn exit_page(&mut self, g: &mut Ctx) {
        if self.hw_type == -1 {
            // We are viewing the menu
            // So exit to the next page

            let nextpage = self
                .get_computer_screen(g)
                .expect("HWSales screen without a generic computer screen")
                .nextpage;
            RemoteInterface::run_screen(g, nextpage);
        } else {
            // We are looking at a sales screen
            // So exit to the menu

            self.remove_sales_menu(g);
            self.set_hw_type(-1);
            self.create_menu(g);
        }
    }

    /// `ScrollUpClick` (static MouseFn)
    pub fn scroll_up_click(g: &mut Ctx, _button: &Button) {
        with_screen(g, |me, g| {
            me.baseoffset -= 1;
            if me.baseoffset < 0 {
                me.baseoffset = 0;
            }

            dirty_sale_rows(g);
        });
    }

    /// `ScrollDownClick` (static MouseFn): clamps the offset to the end of the list.
    pub fn scroll_down_click(g: &mut Ctx, _button: &Button) {
        with_screen(g, |me, g| {
            me.baseoffset += 1;

            let numitems = me.items.len() as i32;

            if me.baseoffset > numitems - NUM_SALE_ROWS {
                me.baseoffset = numitems - NUM_SALE_ROWS;
            }

            if me.baseoffset < 0 {
                me.baseoffset = 0;
            }

            dirty_sale_rows(g);
        });
    }
}

impl InterfaceScreen for HWSalesScreenInterface {
    crate::impl_as_any!();

    /// `Create(ComputerScreen *cs)`: starts on the category menu (`hw_type == -1`).
    fn create(&mut self, g: &mut Ctx) {
        self.set_hw_type(-1);
        self.create_menu(g);
    }

    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible_menu(g) {
            self.remove_menu(g);
        }
        if self.is_visible_sales_menu(g) {
            self.remove_sales_menu(g);
        }
    }

    fn is_visible(&self, g: &Ctx) -> bool {
        self.is_visible_menu(g) || self.is_visible_sales_menu(g)
    }

    /// `EscapeKeyPressed`: same as the exit button, and always reports the key as handled.
    /// The screen is already taken out of its slot here, so it runs on `self` directly.
    fn escape_key_pressed(&mut self, g: &mut Ctx) -> bool {
        self.exit_page(g);
        true
    }

    fn screen_id(&self) -> i32 {
        SCREEN_HWSALESSCREEN
    }

    fn set_computer_screen(&mut self, cs: ScreenRef) {
        self.cs = Some(cs);
    }

    fn computer_screen_ref(&self) -> Option<&ScreenRef> {
        self.cs.as_ref()
    }
}
