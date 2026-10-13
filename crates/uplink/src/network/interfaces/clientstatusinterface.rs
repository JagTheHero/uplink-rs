//! Port of source/network/interfaces/clientstatusinterface.h + source/network/interfaces/clientstatusinterface.cpp
//!
//! Client Status Interface: shows info on the world, the player and the gateway.

use uplink_engine::gl;
use uplink_engine::text::{draw_text_styled, HELVETICA_18};

use crate::app::opengl_interface::{border_draw, button_assignbitmaps, text_draw_xy};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::mainmenu::mainmenu::{MainMenu, MAINMENU_LOGIN};
use crate::network::interfaces::networkscreen::NetworkScreen;
use crate::network::network::NETWORK_CLIENT;
use crate::network::networkclient::{NetworkClient, CLIENT_NONE, CLIENT_STATUS};

#[derive(Debug)]
pub struct ClientStatusInterface {
    // Gateway data
    hardware: String,
    hudupgrades: String,
    connection: String,

    // Personal data
    ratings: String,
    criminal: String,
    financial: String,

    /// C++ `LList<char *> news`: headlines, newest first (at most 6)
    news: Vec<String>,
}

impl ClientStatusInterface {
    /// C++ constructor: panels start blank, with one blank headline
    pub fn new() -> Self {
        ClientStatusInterface {
            hardware: " ".to_string(),
            hudupgrades: " ".to_string(),
            connection: " ".to_string(),
            ratings: " ".to_string(),
            criminal: " ".to_string(),
            financial: " ".to_string(),
            news: vec![" ".to_string()],
        }
    }

    /// `BackgroundDraw`: the panel background and border (no game state needed)
    pub fn background_draw(button: &Button, _highlighted: bool, _clicked: bool) {
        gl::begin(gl::QUADS);
        gl::color3ub(8, 20, 0);
        gl::vertex2i(button.x, button.y + button.height);
        gl::color3ub(8, 20, 124);
        gl::vertex2i(button.x, button.y);
        gl::color3ub(8, 20, 0);
        gl::vertex2i(button.x + button.width, button.y);
        gl::color3ub(8, 20, 124);
        gl::vertex2i(button.x + button.width, button.y + button.height);
        gl::end();

        gl::color3ub(81, 138, 215);
        border_draw(button);
    }

    /// `GatewayPanelDraw`: hardware, HUD upgrades and connection
    pub fn gateway_panel_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        let screenheight = g.app.options.get_option_value("graphics_screenheight");
        gl::scissor(button.x, screenheight - (button.y + button.height), button.width, button.height);
        gl::enable(gl::SCISSOR_TEST);

        let thisint = g
            .app
            .network
            .client
            .screen
            .get::<ClientStatusInterface>()
            .expect("UplinkAssert: status screen");

        Self::background_draw(button, highlighted, clicked);

        let x = button.x;
        let y = button.y;

        gl::color4f(1.0, 1.0, 1.0, 1.0);

        draw_text_styled(x + 55, y + 40, "Hardware", HELVETICA_18);
        text_draw_xy(x + 20, y + 70, &thisint.hardware, button.width);

        draw_text_styled(x + 40, y + 200, "HUD Upgrades", HELVETICA_18);
        text_draw_xy(x + 20, y + 230, &thisint.hudupgrades, button.width);

        draw_text_styled(x + 50, y + 300, "Connection", HELVETICA_18);
        text_draw_xy(x + 20, y + 330, &thisint.connection, button.width);

        gl::disable(gl::SCISSOR_TEST);
    }

    /// `PersonalPanelDraw`: ratings, criminal record and finances
    pub fn personal_panel_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        let screenheight = g.app.options.get_option_value("graphics_screenheight");
        gl::scissor(button.x, screenheight - (button.y + button.height), button.width, button.height);
        gl::enable(gl::SCISSOR_TEST);

        let thisint = g
            .app
            .network
            .client
            .screen
            .get::<ClientStatusInterface>()
            .expect("UplinkAssert: status screen");

        Self::background_draw(button, highlighted, clicked);

        let x = button.x;
        let y = button.y;

        gl::color4f(1.0, 1.0, 1.0, 1.0);

        draw_text_styled(x + 50, y + 40, "Ratings", HELVETICA_18);
        text_draw_xy(x + 20, y + 70, &thisint.ratings, button.width);

        draw_text_styled(x + 50, y + 160, "Criminal", HELVETICA_18);
        text_draw_xy(x + 20, y + 190, &thisint.criminal, button.width);

        draw_text_styled(x + 50, y + 280, "Financial", HELVETICA_18);
        text_draw_xy(x + 20, y + 310, &thisint.financial, button.width);

        gl::disable(gl::SCISSOR_TEST);
    }

    /// `WorldPanelDraw`: headlines
    pub fn world_panel_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        let screenheight = g.app.options.get_option_value("graphics_screenheight");
        gl::scissor(button.x, screenheight - (button.y + button.height), button.width, button.height);
        gl::enable(gl::SCISSOR_TEST);

        let thisint = g
            .app
            .network
            .client
            .screen
            .get::<ClientStatusInterface>()
            .expect("UplinkAssert: status screen");

        Self::background_draw(button, highlighted, clicked);

        let x = button.x;
        let y = button.y;

        gl::color4f(1.0, 1.0, 1.0, 1.0);

        draw_text_styled(x + 50, y + 40, "Headlines", HELVETICA_18);

        for (i, newsitem) in thisint.news.iter().enumerate() {
            text_draw_xy(x + 10, y + 70 + i as i32 * 55, newsitem, button.width + 20);
        }

        gl::disable(gl::SCISSOR_TEST);
    }

    /// `MainMenuClick` (close button): back to the login screen
    pub fn main_menu_click(g: &mut Ctx, _button: &Button) {
        debug_assert!(g.app.network.status == NETWORK_CLIENT, "UplinkAssert: NETWORK_CLIENT");

        NetworkClient::set_client_type(g, CLIENT_NONE);
        MainMenu::run_screen(g, MAINMENU_LOGIN);
    }

    // The setters below are called by NetworkClient while this screen is in its slot. They change
    // the text and dirty the panel buttons, so they take the context and find the screen there.
    // PORT: C++ `SetHUDUpgrades` -> `set_hud_upgrades`.

    /// `SetHardware`
    pub fn set_hardware(g: &mut Ctx, newhw: &str) {
        debug_assert!(newhw.len() < 256, "UplinkAssert: hardware fits its buffer");
        let csi = g
            .app
            .network
            .client
            .screen
            .get_mut::<ClientStatusInterface>()
            .expect("UplinkAssert: status screen");
        csi.hardware = newhw.to_string();

        g.ecl.dirty_button("status_gatewaybg");
    }

    /// `SetHUDUpgrades`
    pub fn set_hud_upgrades(g: &mut Ctx, newhud: &str) {
        debug_assert!(newhud.len() < 256, "UplinkAssert: hud upgrades fit their buffer");
        let csi = g
            .app
            .network
            .client
            .screen
            .get_mut::<ClientStatusInterface>()
            .expect("UplinkAssert: status screen");
        csi.hudupgrades = newhud.to_string();

        g.ecl.dirty_button("status_gatewaybg");
    }

    /// `SetConnection`
    pub fn set_connection(g: &mut Ctx, newconnection: &str) {
        debug_assert!(newconnection.len() < 128, "UplinkAssert: connection fits its buffer");
        let csi = g
            .app
            .network
            .client
            .screen
            .get_mut::<ClientStatusInterface>()
            .expect("UplinkAssert: status screen");
        csi.connection = newconnection.to_string();

        g.ecl.dirty_button("status_gatewaybg");
    }

    /// `SetRating`
    pub fn set_rating(g: &mut Ctx, newrating: &str) {
        debug_assert!(newrating.len() < 256, "UplinkAssert: rating fits its buffer");
        let csi = g
            .app
            .network
            .client
            .screen
            .get_mut::<ClientStatusInterface>()
            .expect("UplinkAssert: status screen");
        csi.ratings = newrating.to_string();

        g.ecl.dirty_button("status_personalbg");
    }

    /// `SetFinancial`
    pub fn set_financial(g: &mut Ctx, newfinancial: &str) {
        debug_assert!(newfinancial.len() < 256, "UplinkAssert: financial fits its buffer");
        let csi = g
            .app
            .network
            .client
            .screen
            .get_mut::<ClientStatusInterface>()
            .expect("UplinkAssert: status screen");
        csi.financial = newfinancial.to_string();

        g.ecl.dirty_button("status_personalbg");
    }

    /// `SetCriminal`
    pub fn set_criminal(g: &mut Ctx, newcriminal: &str) {
        debug_assert!(newcriminal.len() < 256, "UplinkAssert: criminal record fits its buffer");
        let csi = g
            .app
            .network
            .client
            .screen
            .get_mut::<ClientStatusInterface>()
            .expect("UplinkAssert: status screen");
        csi.criminal = newcriminal.to_string();

        g.ecl.dirty_button("status_personalbg");
    }

    /// `AddNewsStory`: put the headline first and keep only the newest 6
    pub fn add_news_story(g: &mut Ctx, newsstory: &str) {
        let csi = g
            .app
            .network
            .client
            .screen
            .get_mut::<ClientStatusInterface>()
            .expect("UplinkAssert: status screen");
        csi.news.insert(0, newsstory.to_string());
        csi.news.truncate(6);

        g.ecl.dirty_button("status_worldbg");
    }
}

impl NetworkScreen for ClientStatusInterface {}

impl InterfaceScreen for ClientStatusInterface {
    /// `Create`: close button and the gateway, world and personal panels
    fn create(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            // Create a close button
            g.ecl.register_button(3, 4, 13, 13, "", "Go to the Main Menu", "status_mainmenu");
            button_assignbitmaps(g, "status_mainmenu", "close.tif", "close_h.tif", "close_c.tif");
            g.ecl.register_button_callback("status_mainmenu", Self::main_menu_click);

            // Create a "gateway" panel
            g.ecl.register_button(10, 35, 200, 400, "", "", "status_gatewaybg");
            g.ecl.register_button_callbacks("status_gatewaybg", Some(Self::gateway_panel_draw), None, None, None);
            g.ecl.register_button(13, 38, 193, 15, "Gateway Status", "Shows the status of your Gateway", "status_gatewaytitle");

            // Create a "world" panel
            g.ecl.register_button(220, 35, 200, 400, "", "", "status_worldbg");
            g.ecl.register_button_callbacks("status_worldbg", Some(Self::world_panel_draw), None, None, None);
            g.ecl.register_button(223, 38, 193, 15, "World Status", "Shows the status of the world", "status_worldtitle");

            // Create a "personal" panel
            g.ecl.register_button(430, 35, 200, 400, "", "", "status_personalbg");
            g.ecl.register_button_callbacks("status_personalbg", Some(Self::personal_panel_draw), None, None, None);
            g.ecl.register_button(433, 38, 193, 15, "Personal Status", "Shows your personal status", "status_personaltitle");
        }
    }

    /// `Remove`: remove the buttons made by `create`
    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            g.ecl.remove_button("status_mainmenu");

            g.ecl.remove_button("status_gatewaytitle");
            g.ecl.remove_button("status_gatewaybg");

            g.ecl.remove_button("status_personaltitle");
            g.ecl.remove_button("status_personalbg");

            g.ecl.remove_button("status_worldtitle");
            g.ecl.remove_button("status_worldbg");
        }
    }

    /// `Update`: nothing to do (the setters dirty the panels when data changes)
    fn update(&mut self, _g: &mut Ctx) {}

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("status_mainmenu").is_some()
    }

    fn screen_id(&self) -> i32 {
        CLIENT_STATUS
    }

    crate::impl_as_any!();
}
