//! Port of source/interface/localinterface/hw_interface.h + source/interface/localinterface/hw_interface.cpp
//! Hardware screen ("GATEWAY" panel): CPU, modem, memory, installed devices and HUD upgrades.

use uplink_engine::text;

use crate::app::opengl_interface::{sx, sy, set_colour, text_draw};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::impl_as_any;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::localinterface::hud_interface::HUDInterface;
use crate::interface::localinterface::localinterface::{LocalInterface, PANELSIZE, SCREEN_HW, SCREEN_NONE};
use crate::interface::localinterface::localinterfacescreen;
use crate::world::computer::gateway::{
    Gateway, HUDUPGRADE_CONNECTIONANALYSIS, HUDUPGRADE_IRCCLIENT, HUDUPGRADE_LANVIEW, HUDUPGRADE_MAPSHOWSTRACE,
};

#[derive(Debug, Clone, Default)]
pub struct HWInterface {}

impl HWInterface {
    /// C++ `HWInterface::TitleClick` (static button callback).
    pub fn title_click(g: &mut Ctx, _button: &Button) {
        LocalInterface::run_screen(g, SCREEN_NONE, -1);
    }

    /// C++ `HWInterface::ShowGatewayClick` (static button callback): opens the gateway layout screen.
    pub fn show_gateway_click(g: &mut Ctx, _button: &Button) {
        // C++ `GetHUD()->gw.Create()`: the sub-screen is taken out of the HUD while it runs.
        let mut gw = std::mem::take(&mut HUDInterface::get_hud_mut(g).gw);
        gw.create(g);
        HUDInterface::get_hud_mut(g).gw = gw;
    }

    /// C++ `HWInterface::MiniTitleDraw` (static draw callback).
    pub fn mini_title_draw(g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        set_colour(g, "TitleText");
        text::draw_text_styled(button.x + 10, button.y + 10, &button.caption, text::HELVETICA_18);
    }

    /// C++ `HWInterface::CreateHWInterface` (static): registers the screen's buttons.
    pub fn create_hw_interface(g: &mut Ctx) {
        if Self::is_visible_hw_interface(g) {
            return;
        }

        let screenw = g.app.options.get_option_value("graphics_screenwidth");
        let paneltop = sx(100) + 30;
        let panelwidth = (screenw as f64 * PANELSIZE) as i32;

        g.ecl.register_button(screenw - panelwidth, paneltop + 3, panelwidth - 7, 15, "GATEWAY", "Remove the gateway screen", "hw_title");
        g.ecl.register_button_callback("hw_title", Self::title_click);

        let gateway = g.player().gateway.clone();

        // Hardware installed
        g.ecl.register_button(screenw - panelwidth + 40, paneltop + 40, 80, 20, "Hardware", "", "hw_hardwaretitle");
        g.ecl.register_button_callbacks("hw_hardwaretitle", Some(Self::mini_title_draw), None, None, None);

        g.ecl.register_button(screenw - panelwidth, paneltop + 70, panelwidth - 10, 15, "", "", "hw_cpu");
        g.ecl.register_button(screenw - panelwidth, paneltop + 85, panelwidth - 10, 15, "", "", "hw_modem");
        g.ecl.register_button(screenw - panelwidth, paneltop + 100, panelwidth - 10, 15, "", "", "hw_memory");
        g.ecl.register_button_callbacks("hw_cpu", Some(text_draw), None, None, None);
        g.ecl.register_button_callbacks("hw_modem", Some(text_draw), None, None, None);
        g.ecl.register_button_callbacks("hw_memory", Some(text_draw), None, None, None);

        // Total CPU speed
        let cpustats = format!("Total CPU speed: {} Ghz", gateway.get_cpu_speed());
        g.ecl.register_caption_change_auto("hw_cpu", &cpustats, None);

        // Bandwidth
        let bandwidthstats = format!("Bandwidth: {} Gq/s", Gateway::get_bandwidth(g));
        g.ecl.register_caption_change_auto("hw_modem", &bandwidthstats, None);

        // Memory
        let memory = format!("Memory Capacity: {} Gq", gateway.memorysize);
        g.ecl.register_caption_change_auto("hw_memory", &memory, None);

        // Hardware devices
        let mut hardware = String::from("DEVICES\n");
        let security = gateway.get_security();
        if security.is_empty() {
            hardware.push_str("None");
        } else {
            for name in &security {
                hardware.push_str(name);
                hardware.push('\n');
            }
        }

        g.ecl.register_button(screenw - panelwidth, paneltop + 130, panelwidth - 10, sy(150), "", "", "hw_hardware");
        g.ecl.register_button_callbacks("hw_hardware", Some(text_draw), None, None, None);
        g.ecl.register_caption_change("hw_hardware", &hardware, 1000, None);

        // HUD Upgrades installed
        g.ecl.register_button(screenw - panelwidth + 20, paneltop + sy(150) + 50, 80, 20, "HUD Upgrades", "", "hw_hudupgradestitle");
        g.ecl.register_button_callbacks("hw_hudupgradestitle", Some(Self::mini_title_draw), None, None, None);

        g.ecl.register_button(screenw - panelwidth, paneltop + sy(150) + 70, panelwidth - 10, 60, "", "", "hw_hudupgrades");
        g.ecl.register_button_callbacks("hw_hudupgrades", Some(text_draw), None, None, None);

        let mut hudupgrades = String::new();
        if !gateway.has_any_hud_upgrade() {
            hudupgrades.push_str("None\n");
        }
        if gateway.has_hud_upgrade(HUDUPGRADE_MAPSHOWSTRACE) {
            hudupgrades.push_str("Show trace on map\n");
        }
        if gateway.has_hud_upgrade(HUDUPGRADE_CONNECTIONANALYSIS) {
            hudupgrades.push_str("Connection analysis\n");
        }
        if gateway.has_hud_upgrade(HUDUPGRADE_IRCCLIENT) {
            hudupgrades.push_str("IRC Client\n");
        }
        if gateway.has_hud_upgrade(HUDUPGRADE_LANVIEW) {
            hudupgrades.push_str("LAN View\n");
        }
        g.ecl.register_caption_change_auto("hw_hudupgrades", &hudupgrades, None);

        // View gateway
        g.ecl.register_button(
            screenw - panelwidth,
            paneltop + sy(282),
            panelwidth - 7,
            15,
            "View Gateway",
            "Show the layout of your gateway",
            "hw_showgateway",
        );
        g.ecl.register_button_callback("hw_showgateway", Self::show_gateway_click);
    }

    /// C++ `HWInterface::RemoveHWInterface` (static): removes the screen's buttons.
    pub fn remove_hw_interface(g: &mut Ctx) {
        if !Self::is_visible_hw_interface(g) {
            return;
        }

        g.ecl.remove_button("hw_title");
        g.ecl.remove_button("hw_hardwaretitle");
        g.ecl.remove_button("hw_cpu");
        g.ecl.remove_button("hw_modem");
        g.ecl.remove_button("hw_memory");
        g.ecl.remove_button("hw_hardware");
        g.ecl.remove_button("hw_hudupgradestitle");
        g.ecl.remove_button("hw_hudupgrades");
        g.ecl.remove_button("hw_showgateway");
    }

    /// C++ `HWInterface::IsVisibleHWInterface` (static).
    pub fn is_visible_hw_interface(g: &Ctx) -> bool {
        g.ecl.get_button("hw_title").is_some()
    }
}

impl InterfaceScreen for HWInterface {
    fn create(&mut self, g: &mut Ctx) {
        localinterfacescreen::create(g);
        Self::create_hw_interface(g);
    }

    fn remove(&mut self, g: &mut Ctx) {
        localinterfacescreen::remove(g);
        Self::remove_hw_interface(g);
    }

    fn update(&mut self, _g: &mut Ctx) {}

    fn is_visible(&self, g: &Ctx) -> bool {
        Self::is_visible_hw_interface(g)
    }

    fn screen_id(&self) -> i32 {
        SCREEN_HW
    }

    impl_as_any!();
}
