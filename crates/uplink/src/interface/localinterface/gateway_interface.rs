//! Port of source/interface/localinterface/gateway_interface.h + source/interface/localinterface/gateway_interface.cpp
//! Gateway screen: draws the player's gateway hardware (CPUs, memory, security, modem, PSU) as buttons.
//! It is a sub-screen owned by `HUDInterface::gw` (the C++ `GetHUD()->gw`), not a slot screen.

use uplink_engine::{gl, sound, text};

use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::impl_as_any;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::localinterface::hud_interface::HUDInterface;
use crate::interface::localinterface::localinterface::{LocalInterface, SCREEN_GATEWAY, SCREEN_NONE};
use crate::interface::localinterface::worldmap::worldmap_interface::WorldMapInterface;
use crate::app::opengl_interface::{button_assignbitmap, button_assignbitmaps, button_click, button_highlight, clear_draw, border_draw, imagebutton_draw};
use crate::world::computer::gateway::Gateway;
use crate::world::computer::gatewaydef::{
    SIZE_GATEWAY_CPU_H, SIZE_GATEWAY_CPU_W, SIZE_GATEWAY_MEM_H, SIZE_GATEWAY_MEM_W, SIZE_GATEWAY_MODEM_H,
    SIZE_GATEWAY_MODEM_W, SIZE_GATEWAY_POWER_H, SIZE_GATEWAY_POWER_W, SIZE_GATEWAY_SEC_H, SIZE_GATEWAY_SEC_W,
};

#[derive(Debug, Clone, Default)]
pub struct GatewayInterface {}

/// C++ `EclGetButton(name)->SetTooltip(text)`.
fn set_button_tooltip(g: &mut Ctx, name: &str, tooltip: &str) {
    if let Some(b) = g.ecl.get_button_mut(name) {
        b.set_tooltip(tooltip);
    }
}

impl GatewayInterface {
    /// C++ `GatewayInterface::CloseClick` (static button callback).
    pub fn close_click(g: &mut Ctx, _button: &Button) {
        // C++ `GetHUD()->gw.Remove()`: the sub-screen is taken out of the HUD while it runs.
        let mut gw = std::mem::take(&mut HUDInterface::get_hud_mut(g).gw);
        gw.remove(g);
        HUDInterface::get_hud_mut(g).gw = gw;
    }

    /// C++ `GatewayInterface::DrawPopulatedItem` (static draw callback).
    pub fn draw_populated_item(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        imagebutton_draw(g, button, highlighted, clicked);

        if highlighted || clicked {
            gl::color4f(1.0, 1.0, 1.0, 1.0);
            border_draw(button);
        }
    }

    /// C++ `GatewayInterface::DrawUnPopulatedItem` (static draw callback; draws nothing).
    pub fn draw_unpopulated_item(_g: &mut Ctx, _button: &Button, _highlighted: bool, _clicked: bool) {}

    /// C++ `GatewayInterface::DrawGatewayBackground` (static draw callback).
    pub fn draw_gateway_background(g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        // The C++ gradient (glBegin/glVertex quad) was commented out, so only the clear and border remain.
        clear_draw(g, button.x, button.y, button.width, button.height);
        gl::color3f(1.0, 1.0, 1.0);
        border_draw(button);
    }

    /// C++ `GatewayInterface::DrawGatewayPicture` (static draw callback).
    pub fn draw_gateway_picture(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        // The C++ border_draw in this callback was commented out.
        imagebutton_draw(g, button, highlighted, clicked);
    }

    /// C++ `GatewayInterface::DrawMainTitle` (static draw callback).
    pub fn draw_main_title(_g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        gl::color4f(1.0, 1.0, 1.0, 1.0);
        let ypos = (button.y + button.height / 2) + 5;

        text::draw_text_styled(button.x, ypos, &button.caption, text::HELVETICA_18);
    }
}

impl InterfaceScreen for GatewayInterface {
    fn create(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            return;
        }

        // Remove the HW screen and the small worldmap
        LocalInterface::run_screen(g, SCREEN_NONE, -1);
        WorldMapInterface::remove_world_map_interface(g);

        let gateway = g.player().gateway.clone();
        let gatewaydef = g
            .world()
            .get_gateway_def(gateway.type_, Some(gateway.name.as_str()))
            .cloned()
            .expect("UplinkAssert(gatewaydef)");

        g.ecl.register_button(20, 50, 600, 350, "", "", "gateway_background");
        g.ecl.register_button_callbacks("gateway_background", Some(Self::draw_gateway_background), None, None, None);

        g.ecl.register_button(605, 52, 13, 13, "", "", "gateway_close");
        button_assignbitmaps(g, "gateway_close", "close.tif", "close_h.tif", "close_c.tif");
        g.ecl.register_button_callback("gateway_close", Self::close_click);

        g.ecl.register_button(25, 55, 100, 30, "GATEWAY", " ", "gateway_title");
        g.ecl.register_button_callbacks("gateway_title", Some(Self::draw_main_title), None, None, None);

        // Create the actual gateway graphics

        let centre_x = 320;
        let centre_y = 225;

        let gateway_x = centre_x - gatewaydef.width / 2;
        let gateway_y = centre_y - gatewaydef.height / 2;

        g.ecl.register_button(gateway_x, gateway_y, gatewaydef.width, gatewaydef.height, "", "", "gateway_picture");
        button_assignbitmap(g, "gateway_picture", &gatewaydef.filename);
        g.ecl.register_button_callbacks("gateway_picture", Some(Self::draw_gateway_picture), None, None, None);

        // Create the CPUs

        let cpus = gateway.get_cpus();

        for ic in 0..gatewaydef.maxcpus {
            let gdl = gatewaydef.cpus.get_data(ic).expect("UplinkAssert(gdl)");
            let bname = format!("gateway_cpu {ic}");
            g.ecl.register_button(
                gateway_x + gdl.x,
                gateway_y + gdl.y,
                SIZE_GATEWAY_CPU_W,
                SIZE_GATEWAY_CPU_H,
                " ",
                " ",
                &bname,
            );

            if ic < cpus.len() as i32 {
                set_button_tooltip(g, &bname, &cpus[ic as usize]);
                button_assignbitmap(g, &bname, "gateway/cpu.tif");
                g.ecl.register_button_callbacks(&bname, Some(Self::draw_populated_item), None, Some(button_click), Some(button_highlight));
            } else {
                set_button_tooltip(g, &bname, "Empty CPU slot");
                g.ecl.register_button_callbacks(&bname, Some(Self::draw_unpopulated_item), None, Some(button_click), Some(button_highlight));
            }
        }

        // Create the memory chips

        for im in 0..gatewaydef.maxmemory {
            let gdl = gatewaydef.memory.get_data(im).expect("UplinkAssert(gdl)");

            let bname = format!("gateway_memory {im}");

            g.ecl.register_button(
                gateway_x + gdl.x,
                gateway_y + gdl.y,
                SIZE_GATEWAY_MEM_W,
                SIZE_GATEWAY_MEM_H,
                " ",
                " ",
                &bname,
            );

            if im < gateway.memorysize / 8 {
                set_button_tooltip(g, &bname, "Memory (8 Gqs)");
                button_assignbitmap(g, &bname, "gateway/memory.tif");
                g.ecl.register_button_callbacks(&bname, Some(Self::draw_populated_item), None, Some(button_click), Some(button_highlight));
            } else {
                set_button_tooltip(g, &bname, "Empty memory slot");
                g.ecl.register_button_callbacks(&bname, Some(Self::draw_unpopulated_item), None, Some(button_click), Some(button_highlight));
            }
        }

        // Create the security devices

        let security = gateway.get_security();

        for is in 0..gatewaydef.maxsecurity {
            let gdl = gatewaydef.security.get_data(is).expect("UplinkAssert(gdl)");

            let bname = format!("gateway_security {is}");

            g.ecl.register_button(
                gateway_x + gdl.x,
                gateway_y + gdl.y,
                SIZE_GATEWAY_SEC_W,
                SIZE_GATEWAY_SEC_H,
                " ",
                " ",
                &bname,
            );

            if is < gateway.get_num_security() {
                set_button_tooltip(g, &bname, &security[is as usize]);
                button_assignbitmap(g, &bname, "gateway/security.tif");
                g.ecl.register_button_callbacks(&bname, Some(Self::draw_populated_item), None, Some(button_click), Some(button_highlight));
            } else {
                set_button_tooltip(g, &bname, "Empty security slot");
                g.ecl.register_button_callbacks(&bname, Some(Self::draw_unpopulated_item), None, Some(button_click), Some(button_highlight));
            }
        }

        // Create the modem

        let bandwidth = Gateway::get_bandwidth(g);
        let modemtooltip = format!("Modem ({bandwidth}Gqs)");
        g.ecl.register_button(
            gateway_x + gatewaydef.modem_x,
            gateway_y + gatewaydef.modem_y,
            SIZE_GATEWAY_MODEM_W,
            SIZE_GATEWAY_MODEM_H,
            " ",
            &modemtooltip,
            "gateway_modem",
        );
        button_assignbitmap(g, "gateway_modem", "gateway/modem.tif");
        g.ecl.register_button_callbacks("gateway_modem", Some(Self::draw_populated_item), None, Some(button_click), Some(button_highlight));

        // Create the power supply

        g.ecl.register_button(
            gateway_x + gatewaydef.power_x,
            gateway_y + gatewaydef.power_y,
            SIZE_GATEWAY_POWER_W,
            SIZE_GATEWAY_POWER_H,
            " ",
            "Power Supply Unit",
            "gateway_power",
        );
        button_assignbitmap(g, "gateway_power", "gateway/power.tif");
        g.ecl.register_button_callbacks("gateway_power", Some(Self::draw_populated_item), None, Some(button_click), Some(button_highlight));
    }

    fn remove(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            return;
        }

        sound::play_sound("sounds/close.wav", None, false);

        g.ecl.remove_button("gateway_background");
        g.ecl.remove_button("gateway_title");
        g.ecl.remove_button("gateway_close");
        g.ecl.remove_button("gateway_addhw");
        g.ecl.remove_button("gateway_upgradegw");

        g.ecl.remove_button("gateway_picture");

        // Remove the chips

        let gateway = g.player().gateway.clone();
        let gatewaydef = g
            .world()
            .get_gateway_def(gateway.type_, Some(gateway.name.as_str()))
            .cloned()
            .expect("UplinkAssert(gatewaydef)");

        for ic in 0..gatewaydef.maxcpus {
            g.ecl.remove_button(&format!("gateway_cpu {ic}"));
            g.ecl.remove_button(&format!("gateway_cpu_caption {ic}"));
        }

        for im in 0..gatewaydef.maxmemory {
            g.ecl.remove_button(&format!("gateway_memory {im}"));
        }

        for is in 0..gatewaydef.maxsecurity {
            g.ecl.remove_button(&format!("gateway_security {is}"));
        }

        g.ecl.remove_button("gateway_modem");
        g.ecl.remove_button("gateway_power");

        // Replace the world map

        WorldMapInterface::create_world_map_interface_small(g);
    }

    fn update(&mut self, _g: &mut Ctx) {}

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("gateway_background").is_some()
    }

    fn screen_id(&self) -> i32 {
        SCREEN_GATEWAY
    }

    impl_as_any!();
}
