//! Port of source/interface/remoteinterface/changegatewayscreen_interface.h + source/interface/remoteinterface/changegatewayscreen_interface.cpp

use uplink_engine::{gl, text};

use crate::app::globals;
use crate::app::opengl_interface::{
    border_draw, button_assignbitmap, button_click, button_highlight, create_msgbox,
    imagebutton_draw, set_colour, textbutton_draw,
};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::game::data::data::{DEMO_MAXGATEWAY, GATEWAY_PARTEXCHANGEVALUE, TIME_TOCHANGEGATEWAY};
use crate::interface::interfacescreen::{InterfaceScreen, ScreenSlot};
use crate::interface::remoteinterface::remoteinterface::{RemoteInterface, SCREEN_CHANGEGATEWAY};
use crate::interface::remoteinterface::remoteinterfacescreen::{draw_main_title, draw_sub_title};
use crate::interface::scrollbox::ScrollBox;
use crate::objref::ScreenRef;
use crate::world::computer::computerscreen::genericscreen::GenericScreen;
use crate::world::date::Date;
use crate::world::generator::numbergenerator::NumberGenerator;
use crate::world::message::Message;
use crate::world::person::Person;
use crate::world::scheduler::changegatewayevent::ChangeGatewayEvent;
use crate::world::scheduler::uplinkevent::UplinkEvent;

/// Number of gateway rows shown at once (C++ literal 10 in the loops).
const NUM_GATEWAY_ROWS: i32 = 10;

/// Gateway exchange screen: lets the player trade their gateway for a newer one.
#[derive(Debug)]
pub struct ChangeGatewayScreenInterface {
    pub cs: Option<ScreenRef>,
    // PORT: `baseOffset` was a `static` in C++ (shared by every instance); kept per instance.
    pub base_offset: i32,
    /// Gateway index shown / selected for purchase.
    pub currentselect: i32,
}

impl Default for ChangeGatewayScreenInterface {
    fn default() -> Self {
        ChangeGatewayScreenInterface {
            cs: None,
            base_offset: 0,
            currentselect: 0,
        }
    }
}

/// The remote interface's screen slot (the screen is taken out of it while one of its methods runs).
fn remote_screen_slot(g: &mut Ctx) -> &mut ScreenSlot {
    &mut g.interface_mut().ri.screen
}

/// Runs `f` on this screen with the screen taken out of its slot (see `ScreenSlot::call`), so
/// `f` may use `g` freely. Does nothing if this screen is not the current remote screen.
fn with_screen(g: &mut Ctx, f: impl FnOnce(&mut ChangeGatewayScreenInterface, &mut Ctx)) {
    ScreenSlot::call(g, remote_screen_slot, |s, g| {
        if let Some(me) = s.as_any_mut().downcast_mut::<ChangeGatewayScreenInterface>() {
            f(me, g);
        }
    });
}

/// The current remote screen (C++ `GetInterfaceScreen` + `UplinkAssert`).
fn current(g: &Ctx) -> &ChangeGatewayScreenInterface {
    g.interface()
        .ri
        .screen
        .get::<ChangeGatewayScreenInterface>()
        .expect("ChangeGateway callback without a ChangeGateway screen")
}

/// Reads the row number at the end of a button name such as "changegateway_title 3"
/// (C++ `sscanf(button->name, "%s %d", ...)`).
fn row_index(button: &Button) -> i32 {
    button
        .name
        .rsplit(' ')
        .next()
        .and_then(|s| s.parse().ok())
        .expect("ChangeGateway button name without an index")
}

/// Dirties every gateway row button (C++ loop of `EclDirtyButton("changegateway_title i")`).
fn dirty_title_rows(g: &mut Ctx) {
    for i in 0..NUM_GATEWAY_ROWS {
        g.ecl.dirty_button(&format!("changegateway_title {i}"));
    }
}

impl ChangeGatewayScreenInterface {
    /// `ShowGateway(index)`: fills the stats, text and picture for a gateway.
    pub fn show_gateway(&mut self, g: &mut Ctx, index: i32) {
        self.currentselect = index;

        // Dirty the select buttons

        dirty_title_rows(g);

        // Get the new gateway

        let Some(gd) = g.world().get_gateway_def(self.currentselect, None).cloned() else {
            return;
        };

        g.ecl.register_caption_change_auto(
            "changegateway_stats1",
            &format!("Max CPUs : {}\nMax Memory : {}Gq", gd.maxcpus, gd.maxmemory * 8),
            None,
        );
        g.ecl.register_caption_change_auto(
            "changegateway_stats2",
            &format!("Max Security : {}\nBandwidth : {}Gqs", gd.maxsecurity, gd.bandwidth),
            None,
        );

        g.ecl.register_caption_change("changegateway_text", &gd.description, 1500, None);

        // Show the new picture

        g.ecl
            .get_button_mut("changegateway_picture")
            .expect("changegateway_picture button missing")
            .set_standard_image(None);
        button_assignbitmap(g, "changegateway_picture", &gd.thumbnail);
        g.ecl.register_button_callbacks("changegateway_picture", Some(Self::gateway_picture_draw), None, None, None);
    }

    /// `GetGatewayPrice(index)`: takes into account the part-exchange value of the current gateway.
    pub fn get_gateway_price(&self, g: &Ctx, index: i32) -> i32 {
        let gateway = &g.player().gateway;
        let oldgateway = g
            .world()
            .get_gateway_def(gateway.type_, Some(&gateway.name))
            .expect("player gateway definition missing");

        let newgateway = g
            .world()
            .get_gateway_def(index, None)
            .expect("gateway definition missing");

        (f64::from(newgateway.cost) - f64::from(oldgateway.cost) * f64::from(GATEWAY_PARTEXCHANGEVALUE)) as i32
    }

    /// `GetComputerScreen` (typed accessor; the C++ casts `cs`).
    pub fn get_computer_screen<'a>(&self, g: &'a Ctx) -> Option<&'a GenericScreen> {
        let cs = self.cs.as_ref()?;
        g.world()
            .get_computer(&cs.computer)?
            .get_computer_screen(cs.index)?
            .as_generic_screen()
    }

    /// The maintitle and subtitle buttons (C++ `GetComputerScreen()->maintitle` / `subtitle`).
    fn create_titles(&self, g: &mut Ctx) {
        let (maintitle, subtitle) = {
            let gs = self
                .get_computer_screen(g)
                .expect("ChangeGateway screen without a generic computer screen");
            (gs.base.maintitle.clone(), gs.base.subtitle.clone())
        };

        g.ecl.register_button(80, 60, 350, 25, &maintitle, "", "changegateway_maintitle");
        g.ecl.register_button_callbacks("changegateway_maintitle", Some(draw_main_title), None, None, None);
        g.ecl.register_button(80, 80, 350, 20, &subtitle, "", "changegateway_subtitle");
        g.ecl.register_button_callbacks("changegateway_subtitle", Some(draw_sub_title), None, None, None);
    }

    /// `CreateAreYouSure()`: the confirmation page shown before the exchange is paid for.
    pub fn create_are_you_sure(&mut self, g: &mut Ctx) {
        // Titles

        self.create_titles(g);

        // Proceed and cancel

        g.ecl.register_button(130, 380, 100, 15, "Proceed", "Click to go ahead with the exchange", "changegateway_buy");
        g.ecl.register_button_callback("changegateway_buy", Self::buy_confirm_click);

        g.ecl.register_button(300, 380, 100, 15, "Cancel", "Cancel the exchange", "changegateway_close");
        g.ecl.register_button_callback("changegateway_close", Self::buy_cancel_click);

        // Information

        g.ecl.register_button(40, 130, 400, 250, " ", " ", "changegateway_text");
        g.ecl.register_button_callbacks("changegateway_text", Some(textbutton_draw), None, None, None);

        let (old_index, new_name, new_index) = {
            let gateway = &g.player().gateway;
            let oldgateway = g
                .world()
                .get_gateway_def(gateway.type_, Some(&gateway.name))
                .expect("player gateway definition missing");
            let newgateway = g
                .world()
                .get_gateway_def(self.currentselect, None)
                .expect("gateway definition missing");
            (oldgateway.index, newgateway.name.clone(), newgateway.index)
        };

        let price = self.get_gateway_price(g, self.currentselect);

        let mut caption = format!("You have chosen to part exchange your current gateway for a {new_name}.\n");
        caption.push_str("If you chose to proceed, the money will be deducted from your account and Uplink Corporation ");
        caption.push_str("will prepare your new gateway.  We will install identical hardware in your new gateway, and ");
        caption.push_str("we will copy your memory banks from your current system to your new system.\n\n");
        caption.push_str("This process will take around 24 hours.  After this time, you will have to log off from the Net ");
        caption.push_str("and log back in again.  You will then be re-routed to your new gateway, and your old one will ");
        caption.push_str("be immediately dismantled.\n\n");

        if old_index > new_index {
            caption.push_str("Because you are downgrading, you may lose some software or hardware during the transfer.\n");
        }

        if price > 0 {
            caption.push_str(&format!("This exchange will cost you {price} credits.\n"));
        } else {
            caption.push_str(&format!("You will be given {} credits to balance the exchange.\n", price * -1));
        }

        caption.push_str("Click Proceed to continue or Cancel to abort.");

        g.ecl.register_caption_change("changegateway_text", &caption, 8000, None);
    }

    /// `GatewayButtonDraw` (static DrawFn)
    pub fn gateway_button_draw(g: &mut Ctx, button: &Button, highlighted: bool, _clicked: bool) {
        let me = current(g);
        let index = row_index(button) + me.base_offset;

        let gd = g
            .world()
            .get_gateway_def(index, None)
            .expect("gateway definition missing");

        // Fill in the background

        if index % 2 == 0 {
            gl::begin(gl::QUADS);
            set_colour(g, "DarkPanelA");
            gl::vertex2i(button.x, button.y + button.height);
            set_colour(g, "DarkPanelB");
            gl::vertex2i(button.x, button.y);
            set_colour(g, "DarkPanelA");
            gl::vertex2i(button.x + button.width, button.y);
            set_colour(g, "DarkPanelB");
            gl::vertex2i(button.x + button.width, button.y + button.height);
            gl::end();
        } else {
            gl::begin(gl::QUADS);
            set_colour(g, "DarkPanelB");
            gl::vertex2i(button.x, button.y + button.height);
            set_colour(g, "DarkPanelA");
            gl::vertex2i(button.x, button.y);
            set_colour(g, "DarkPanelB");
            gl::vertex2i(button.x + button.width, button.y);
            set_colour(g, "DarkPanelA");
            gl::vertex2i(button.x + button.width, button.y + button.height);
            gl::end();
        }

        if highlighted {
            set_colour(g, "PanelHighlightBorder");
            border_draw(button);
        } else if index == me.currentselect {
            set_colour(g, "PanelBorder");
            border_draw(button);
        }

        // Draw the title and cost

        let price = me.get_gateway_price(g, index);
        let hasmoney = g.player().get_balance(g.world()) >= price;

        if hasmoney {
            set_colour(g, "DefaultText");
        } else {
            set_colour(g, "DimmedText");
        }

        let cost = format!("{price}c");

        text::draw_text_default(button.x + 5, button.y + 10, &gd.name);
        text::draw_text_default(
            (button.x + button.width - 10) - text::text_width_default(&cost),
            button.y + 10,
            &cost,
        );
    }

    /// `InfoTextDraw` (static DrawFn)
    pub fn info_text_draw(g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        textbutton_draw(g, button, true, true);
    }

    /// `GatewayButtonClick` (static MouseFn)
    pub fn gateway_button_click(g: &mut Ctx, button: &Button) {
        let index = row_index(button) + current(g).base_offset;

        with_screen(g, |me, g| me.show_gateway(g, index));
    }

    /// `CloseClick` body: goes to the generic screen's next page (if it has one).
    fn close(&mut self, g: &mut Ctx) {
        let nextpage = self
            .get_computer_screen(g)
            .expect("ChangeGateway screen without a generic computer screen")
            .nextpage;

        if nextpage != -1 {
            RemoteInterface::run_screen(g, nextpage);
        }
    }

    /// `CloseClick` (static MouseFn)
    pub fn close_click(g: &mut Ctx, _button: &Button) {
        with_screen(g, |me, g| me.close(g));
    }

    /// `BuyClick` (static MouseFn): asks for confirmation.
    pub fn buy_click(g: &mut Ctx, _button: &Button) {
        with_screen(g, |me, g| {
            let price = me.get_gateway_price(g, me.currentselect);
            let hasmoney = g.player().get_balance(g.world()) >= price;

            if hasmoney {
                if globals::DEMOGAME && me.currentselect > DEMO_MAXGATEWAY {
                    create_msgbox(
                        g,
                        "Sorry",
                        "Not available in demo!\n\
                         Visit www.introversion.co.uk to buy the full game.",
                    );
                    return;
                }

                // Create the message for the player to read
                // Remove the current interface first

                me.remove(g);
                me.create_are_you_sure(g);
            }
        });
    }

    /// `BuyConfirmClick` (static MouseFn): pays, schedules the gateway change and mails the player.
    pub fn buy_confirm_click(g: &mut Ctx, _button: &Button) {
        with_screen(g, |me, g| {
            // Take the money and run

            let price = me.get_gateway_price(g, me.currentselect);
            let player_name = g.player().name.clone();
            Person::change_balance(g, &player_name, -price, "Uplink Corporation gateway exchange");

            // Schedule a Change Gateway Event

            let mut rundate = Date::default();
            rundate.set_date_from(&g.world().date);
            rundate.advance_minute(TIME_TOCHANGEGATEWAY);
            rundate.advance_minute(NumberGenerator::random_number(60));

            let mut cge = ChangeGatewayEvent::new();
            cge.base.set_run_date(&rundate);
            cge.set_new_type(me.currentselect);
            g.world_mut()
                .scheduler
                .schedule_event(UplinkEvent::ChangeGatewayEvent(cge));

            // Send a confirmation email to the player

            let mut msg = Message::new();
            msg.set_to("PLAYER");
            msg.set_from("Uplink Corporation");
            msg.set_subject("Gateway Upgrade");
            msg.set_body(
                "Uplink Corporation is now preparing your new gateway computer.\n\
                 This will take around 24 hours, at which point we will notify you to \
                 disconnect and reconnect.  This will allow us to re-route you to your \
                 new gateway.\n\
                 [END]",
            );
            msg.send(g);

            // Return to the upgrade screen

            me.remove(g);
            me.create(g);
        });
    }

    /// `BuyCancelClick` (static MouseFn)
    pub fn buy_cancel_click(g: &mut Ctx, _button: &Button) {
        with_screen(g, |me, g| {
            me.remove(g);
            me.create(g);
        });
    }

    /// `GatewayPictureDraw` (static DrawFn)
    pub fn gateway_picture_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        imagebutton_draw(g, button, highlighted, clicked);

        let me = current(g);

        gl::color3f(1.0, 1.0, 1.0);
        border_draw(button);

        if globals::DEMOGAME && me.currentselect > DEMO_MAXGATEWAY {
            gl::color3f(1.0, 1.0, 1.0);
            text::draw_text_default(button.x + 50, button.y + button.height / 2, "NOT AVAILABLE IN DEMO");
        }
    }

    /// `ScrollChange` (scrollbox callback): sets `base_offset` and dirties the rows.
    pub fn scroll_change(g: &mut Ctx, _scrollname: &str, new_index: i32) {
        with_screen(g, |me, g| {
            me.base_offset = new_index;

            dirty_title_rows(g);
        });
    }
}

impl InterfaceScreen for ChangeGatewayScreenInterface {
    crate::impl_as_any!();

    /// `Create(ComputerScreen *newcs)` body (the `cs` is already set by `set_computer_screen`).
    fn create(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            return;
        }

        self.create_titles(g);

        g.ecl.register_button(20, 120, 220, 15, "Gateway Name and Cost", " ", "changegateway_title");

        // One button for each gateway

        for i in 0..NUM_GATEWAY_ROWS {
            let name = g
                .world()
                .get_gateway_def(i, None)
                .expect("gateway definition missing")
                .name
                .clone();

            let bname = format!("changegateway_title {i}");
            g.ecl.register_button(20, 140 + i * 20, 220, 15, &name, "View this gateway", &bname);
            g.ecl.register_button_callbacks(
                &bname,
                Some(Self::gateway_button_draw),
                Some(Self::gateway_button_click),
                Some(button_click),
                Some(button_highlight),
            );
        }

        // Scroll bar baby yeah

        let num_items = g.world().gatewaydefs.size();
        if num_items > NUM_GATEWAY_ROWS {
            ScrollBox::create_scroll_box(
                g,
                "changegatewayscroll",
                5,
                140,
                15,
                NUM_GATEWAY_ROWS * 20,
                num_items,
                10,
                self.base_offset,
                Some(Self::scroll_change),
            );
        }

        // Picture of the gateway

        g.ecl.register_button(245, 120, 200, 140, " ", " ", "changegateway_picture");
        button_assignbitmap(g, "changegateway_picture", "gateway/gateway_t0.tif");
        g.ecl.register_button_callbacks("changegateway_picture", Some(Self::gateway_picture_draw), None, None, None);

        // Stats

        g.ecl.register_button(245, 270, 110, 30, "accessing...", " ", "changegateway_stats1");
        g.ecl.register_button(355, 270, 110, 30, "accessing...", " ", "changegateway_stats2");

        g.ecl.register_button_callbacks("changegateway_stats1", Some(textbutton_draw), None, None, None);
        g.ecl.register_button_callbacks("changegateway_stats2", Some(textbutton_draw), None, None, None);

        g.ecl.register_button(245, 315, 200, 70, "accessing...", " ", "changegateway_text");
        g.ecl.register_button_callbacks("changegateway_text", Some(Self::info_text_draw), None, None, None);

        // Control buttons

        let low_y = 140 + NUM_GATEWAY_ROWS * 20;

        g.ecl.register_button(20, low_y + 10, 220, 15, "Upgrade", "Click to change your gateway for this one", "changegateway_buy");
        g.ecl.register_button_callback("changegateway_buy", Self::buy_click);

        g.ecl.register_button(20, low_y + 30, 220, 15, "Close", "Return to the main menu", "changegateway_close");
        g.ecl.register_button_callback("changegateway_close", Self::close_click);

        // Select your gateway

        let index = {
            let gateway = &g.player().gateway;
            g.world()
                .get_gateway_def(gateway.type_, Some(&gateway.name))
                .expect("player gateway definition missing")
                .index
        };
        self.show_gateway(g, index);
    }

    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            g.ecl.remove_button("changegateway_maintitle");
            g.ecl.remove_button("changegateway_subtitle");

            g.ecl.remove_button("changegateway_title");

            for i in 0..NUM_GATEWAY_ROWS {
                g.ecl.remove_button(&format!("changegateway_title {i}"));
            }

            ScrollBox::remove_scroll_box(g, "changegatewayscroll");

            g.ecl.remove_button("changegateway_picture");
            g.ecl.remove_button("changegateway_stats1");
            g.ecl.remove_button("changegateway_stats2");
            g.ecl.remove_button("changegateway_text");

            g.ecl.remove_button("changegateway_buy");
            g.ecl.remove_button("changegateway_close");
        }
    }

    fn update(&mut self, _g: &mut Ctx) {}

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("changegateway_maintitle").is_some()
    }

    fn return_key_pressed(&mut self, _g: &mut Ctx) -> bool {
        false
    }

    /// `EscapeKeyPressed`: same as the close button, and always reports the key as handled.
    /// The screen is already taken out of its slot here, so it runs on `self` directly.
    fn escape_key_pressed(&mut self, g: &mut Ctx) -> bool {
        self.close(g);
        true
    }

    fn screen_id(&self) -> i32 {
        SCREEN_CHANGEGATEWAY
    }

    fn set_computer_screen(&mut self, cs: ScreenRef) {
        self.cs = Some(cs);
    }

    fn computer_screen_ref(&self) -> Option<&ScreenRef> {
        self.cs.as_ref()
    }
}
