//! Port of source/interface/localinterface/worldmap/worldmap_interface.h + source/interface/localinterface/worldmap/worldmap_interface.cpp
//! The world map screen, in two sizes (small in the HUD, large full-screen) with zoom, scroll and
//! connection tracing between gateway and target.
//!
//! The map lives in the HUD (`HUDInterface::wmi`). Static button callbacks reach it through
//! [`with_hud_wmi`], which takes it out of the HUD for the duration of the call and puts it back.
//! Instance methods work on `self` and never re-fetch the map from `g`.

use std::rc::Rc;

use uplink_engine::{gl, sound, text};

use crate::app::opengl::{get_mouse_x, get_mouse_y};
use crate::app::opengl_interface::{
    border_draw, button_assignbitmap, button_assignbitmaps, button_click, button_draw,
    button_highlight, clear_draw, imagebutton_draw, textbutton_draw,
};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::game::data::data::IP_LOCALHOST;
use crate::impl_as_any;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::localinterface::hud_interface::HUDInterface;
use crate::interface::localinterface::localinterface::{LocalInterface, PANELSIZE, SCREEN_MAP, SCREEN_NONE};
use crate::interface::localinterface::phonedialler::PhoneDialler;
use crate::interface::localinterface::worldmap::rectangle::MapRectangle;
use crate::interface::localinterface::worldmap::worldmap_layout::WorldMapLayout;
use crate::interface::remoteinterface::remoteinterface::RemoteInterface;
use crate::vanbakel::Vanbakel;
use crate::world::computer::gateway::HUDUPGRADE_MAPSHOWSTRACE;
use crate::world::connection::Connection;
use crate::world::vlocation::{VIRTUAL_HEIGHT, VIRTUAL_WIDTH};

/// C++ `#define WORLDMAP_*`: which size of map is visible.
pub const WORLDMAP_NONE: i32 = 0;
pub const WORLDMAP_SMALL: i32 = 1;
pub const WORLDMAP_LARGE: i32 = 2;

/// C++ `#define TEXTSIZEX` / `TEXTSIZEY`: size of the text grid.
pub const TEXTSIZEX: i32 = 5;
pub const TEXTSIZEY: i32 = 20;

/// C++ file-level `#define MAXZOOM 4.0`
pub const MAXZOOM: f32 = 4.0;

/// C++ file-level `STIPPLE[]` table for the animated link lines.
pub const STIPPLE: [i32; 8] = [
    0x1111, 0x1111, 0x2222, 0x2222, 0x4444, 0x4444, 0x8888, 0x8888,
];

/// The player's key in the world (C++ `GetPlayer()`).
const PLAYER: &str = "PLAYER";

/// C++ `class WorldMapInterface : LocalInterfaceScreen`. The C++ statics (`stipplepattern`,
/// `scrollX`, `scrollY`, `zoom`, and the file statics `stippleindex`, `lastupdate`,
/// `scrollupdate`, `revelationColour`) are fields here, reached through the local interface.
// PORT: static -> field for every static member. Button callbacks reach them through `g`.
#[derive(Debug)]
pub struct WorldMapInterface {
    /// C++ `static int stipplepattern`
    pub stipple_pattern: i32,
    /// C++ file static `stippleindex`
    pub stipple_index: i32,
    /// C++ file static `lastupdate`
    pub last_update: i32,
    /// C++ file static `scrollupdate`
    pub scroll_update: i32,
    /// C++ file static `revelationColour`
    pub revelation_colour: f32,
    /// C++ `static float scrollX`, `scrollY`, `zoom`
    pub scroll_x: f32,
    pub scroll_y: f32,
    pub zoom: f32,
    /// C++ `WorldMapLayout *layout` (owned; NULL until Create).
    pub layout: Option<WorldMapLayout>,
    /// C++ `LList<char *> savedconnection`
    pub saved_connection: Vec<String>,
}

impl Default for WorldMapInterface {
    /// C++ `WorldMapInterface::WorldMapInterface()`: layout starts NULL, zoom 1.0, scroll 0.
    fn default() -> Self {
        WorldMapInterface {
            stipple_pattern: STIPPLE[0],
            stipple_index: 0,
            last_update: 0,
            scroll_update: 0,
            revelation_colour: 1.0,
            scroll_x: 0.0,
            scroll_y: 0.0,
            zoom: 1.0,
            layout: None,
            saved_connection: Vec::new(),
        }
    }
}

/// The HUD's world map, read-only. `None` while the HUD is absent or busy.
fn hud_wmi(g: &Ctx) -> Option<&WorldMapInterface> {
    g.interface().li.hud.get::<HUDInterface>().map(|h| &h.wmi)
}

fn hud_mut(g: &mut Ctx) -> Option<&mut HUDInterface> {
    g.interface_mut().li.hud.get_mut::<HUDInterface>()
}

/// Runs `f` with the HUD's world map, taking it out of the HUD for the call and putting it back.
/// Returns `None` if there is no HUD to take it from. Calls must not nest: a nested call sees the
/// HUD's map as absent.
fn with_hud_wmi<R>(g: &mut Ctx, f: impl FnOnce(&mut Ctx, &mut WorldMapInterface) -> R) -> Option<R> {
    let mut wmi = std::mem::take(&mut hud_mut(g)?.wmi);
    let r = f(g, &mut wmi);
    if let Some(hud) = hud_mut(g) {
        hud.wmi = wmi;
    }
    Some(r)
}

/// Builds the small or large map through the HUD's map (see [`with_hud_wmi`]).
fn create_world_map_from_hud(g: &mut Ctx, size: i32) {
    with_hud_wmi(g, |g, wmi| WorldMapInterface::create_world_map_interface(g, wmi, size));
}

/// The links the player has (C++ `GetPlayer()->links`), cloned so callers may change buttons.
fn player_links(g: &Ctx) -> Vec<String> {
    g.player().agent.as_deref().map(|a| a.links.clone()).unwrap_or_default()
}

/// C++ `sscanf(button->name, "worldmap %s %d", ip, &index)`: the IP is the word after "worldmap".
fn location_ip(button_name: &str) -> Option<String> {
    let mut words = button_name.split_whitespace();
    if words.next()? != "worldmap" {
        return None;
    }
    words.next().map(|ip| ip.to_string())
}

impl WorldMapInterface {
    /// C++ `WorldMapInterface::CycleStipplePattern` (static)
    pub fn cycle_stipple_pattern(&mut self) {
        self.stipple_index += 1;
        if self.stipple_index > 7 {
            self.stipple_index = 0;
        }
        self.stipple_pattern = STIPPLE[self.stipple_index as usize];
    }

    /// C++ `WorldMapInterface::RemoveWorldMapInterface` (static)
    pub fn remove_world_map_interface(g: &mut Ctx) {
        let mapsize = Self::is_visible_world_map_interface(g);

        if mapsize == WORLDMAP_SMALL {
            g.ecl.remove_button("worldmap_smallmap");
            g.ecl.remove_button("worldmap_connect");
            g.ecl.remove_button("worldmap_toprightclick");
        } else if mapsize == WORLDMAP_LARGE {
            for name in [
                "worldmap_largemap",
                "worldmap_saveconnection",
                "worldmap_loadconnection",
                "worldmap_zoomout",
                "worldmap_zoom",
                "worldmap_zoomin",
                "worldmap_connect",
                "worldmap_cancel",
                "worldmap_close",
                "worldmap_scrollleft",
                "worldmap_scrollright",
                "worldmap_scrollup",
                "worldmap_scrolldown",
            ] {
                g.ecl.remove_button(name);
            }

            // The link buttons are named by their IP (the same string as the link), so no lookup is needed.
            for (i, ip) in player_links(g).iter().enumerate() {
                g.ecl.remove_button(&format!("worldmap {} {}", ip, i));
            }

            g.ecl.remove_button("worldmap_texthelp");
            g.ecl.remove_button("worldmap 127.0.0.1 0");
        }
    }

    /// C++ `WorldMapInterface::FullScreenClick` (static button callback).
    pub fn full_screen_click(g: &mut Ctx, _button: &Button) {
        LocalInterface::run_screen(g, SCREEN_NONE, -1);
        create_world_map_from_hud(g, WORLDMAP_LARGE);
    }

    /// C++ `WorldMapInterface::LocationClick` (static button callback).
    pub fn location_click(g: &mut Ctx, button: &Button) {
        if !g.player().is_connected() {
            let Some(ip) = location_ip(&button.name) else { return };

            g.player_mut().get_connection_mut().add_vlocation(&ip);

            sound::play_sound("sounds/addlink.wav", None, false);
        } else {
            g.ecl.register_caption_change_auto(
                "worldmap_texthelp",
                "You cannot modify an existing connection. Disconnect first.",
                None,
            );
        }
    }

    /// C++ `WorldMapInterface::ConnectClick` (static button callback).
    pub fn connect_click(g: &mut Ctx, _button: &Button) {
        if g.player().is_connected() {
            // Disconnect
            let mapmaximised = Self::is_visible_world_map_interface(g) == WORLDMAP_LARGE;

            Connection::disconnect(g, PLAYER);
            Connection::reset(g, PLAYER);

            RemoteInterface::run_new_location(g);
            RemoteInterface::run_screen(g, 1);

            if mapmaximised {
                // Re-build the large map as it is now covered by the new remote interface
                Self::remove_world_map_interface(g);
                create_world_map_from_hud(g, WORLDMAP_LARGE);
            }
        } else if Self::is_visible_world_map_interface(g) == WORLDMAP_LARGE {
            // Connect
            create_world_map_from_hud(g, WORLDMAP_SMALL);

            let target = g.player().connection.get_target().unwrap_or("").to_string();
            let mut pd = PhoneDialler::default();
            pd.create_at(g, 100, 100);
            pd.dial_number(g, &target);
            pd.remove(g);

            Connection::connect(g, PLAYER);
            RemoteInterface::run_new_location(g);
        }
    }

    /// C++ `WorldMapInterface::ConnectMouseMove` (static mouse-move callback).
    pub fn connect_mouse_move(g: &mut Ctx, button: &Button) {
        let connected = g.player().is_connected();
        if connected || Self::is_visible_world_map_interface(g) == WORLDMAP_LARGE {
            let tooltip = if connected {
                "Break the connection to your current remote host"
            } else {
                "Establish the connection to your current remote host"
            };
            // C++ changes the real button, which is the same object the highlight then reads.
            if let Some(real) = g.ecl.get_button_mut(&button.name) {
                real.set_tooltip(tooltip);
            }
            let current = g.ecl.get_button(&button.name).cloned().unwrap_or_else(|| button.clone());
            button_highlight(g, &current);
        }
    }

    /// C++ `WorldMapInterface::ConnectMouseDown` (static mouse-down callback).
    pub fn connect_mouse_down(g: &mut Ctx, button: &Button) {
        if g.player().is_connected() || Self::is_visible_world_map_interface(g) == WORLDMAP_LARGE {
            button_click(g, button);
        }
    }

    /// C++ `WorldMapInterface::ConnectDraw` (static draw callback).
    pub fn connect_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        if g.player().is_connected() || Self::is_visible_world_map_interface(g) == WORLDMAP_LARGE {
            let caption = if g.player().is_connected() { "Disconnect" } else { "Connect" };
            if let Some(real) = g.ecl.get_button_mut(&button.name) {
                real.set_caption(caption);
            }
            let current = g.ecl.get_button(&button.name).cloned().unwrap_or_else(|| button.clone());
            button_draw(g, &current, highlighted, clicked);
        } else {
            clear_draw(g, button.x, button.y, button.width, button.height);
        }
    }

    /// C++ `WorldMapInterface::CancelClick` (static button callback).
    pub fn cancel_click(g: &mut Ctx, button: &Button) {
        // Close the screen if a connection is set up to somewhere else
        if g.player().is_connected() {
            Self::close_click(g, button);
        }

        // Close the screen if we are at home without any half set up connection
        let remote_is_home = g.player().get_remote_host(g.world()).map(|vl| vl.ip == IP_LOCALHOST).unwrap_or(false);
        let target_is_home = g.player().connection.get_target() == Some(IP_LOCALHOST);
        if remote_is_home && target_is_home {
            Self::close_click(g, button);
        }

        // Cancel any "half set up" connections
        if !g.player().is_connected() {
            Connection::reset(g, PLAYER);
        }
    }

    /// C++ `WorldMapInterface::CloseClick` (static button callback).
    pub fn close_click(g: &mut Ctx, _button: &Button) {
        create_world_map_from_hud(g, WORLDMAP_SMALL);
        sound::play_sound("sounds/close.wav", None, false);
    }

    /// C++ `WorldMapInterface::SaveConnectionClick` (static button callback).
    pub fn save_connection_click(g: &mut Ctx, _button: &Button) {
        with_hud_wmi(g, |g, wmi| wmi.save_current_connection(g));
        g.ecl.register_caption_change_auto("worldmap_texthelp", "Connection saved", None);
    }

    /// C++ `WorldMapInterface::LoadConnectionClick` (static button callback).
    pub fn load_connection_click(g: &mut Ctx, _button: &Button) {
        with_hud_wmi(g, |g, wmi| wmi.load_connection(g));
    }

    /// C++ `WorldMapInterface::DrawAllObjects`
    pub fn draw_all_objects(&self, g: &Ctx) {
        let Some(layout) = self.layout.as_ref() else { return };

        let offset_x = (self.scroll_x * Self::get_large_map_width(g) as f32) as i32;
        let offset_y = (self.scroll_y * Self::get_large_map_height(g) as f32) as i32;

        // Labels first, then the location dots
        for label in layout.get_labels() {
            label.draw(offset_x, offset_y, self.zoom);
        }
        for location in layout.get_locations() {
            location.draw(offset_x, offset_y, self.zoom);
        }
    }

    /// C++ `WorldMapInterface::DrawWorldMapSmall` (static draw callback).
    pub fn draw_world_map_small(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        // Draw the background bitmap
        imagebutton_draw(g, button, highlighted, clicked);

        with_hud_wmi(g, |g, wmi| {
            let gr: &Ctx = g;

            // Draw the players connection
            if !gr.player().is_connected() {
                // Draw a dot to represent the player
                gl::color4f(1.0, 1.0, 1.0, 1.0);

                // PORT: the C++ dereferences GetLocalHost() unchecked; without one, skip only the dot.
                if let Some(local) = gr.player().get_local_host(gr.world()) {
                    let scaled_x = button.x + wmi.get_scaled_x(gr, local.x, WORLDMAP_SMALL);
                    let scaled_y = button.y + wmi.get_scaled_y(gr, local.y, WORLDMAP_SMALL);

                    gl::begin(gl::QUADS);
                    gl::vertex2f((scaled_x - 1) as f32, (scaled_y - 1) as f32);
                    gl::vertex2f((scaled_x + 2) as f32, (scaled_y - 1) as f32);
                    gl::vertex2f((scaled_x + 2) as f32, (scaled_y + 2) as f32);
                    gl::vertex2f((scaled_x - 1) as f32, (scaled_y + 2) as f32);
                    gl::end();
                }
            } else {
                let connection = &gr.player().connection;
                let size = connection.vlocations.len() as i32;
                let show_trace = gr.player().gateway.has_hud_upgrade(HUDUPGRADE_MAPSHOWSTRACE);

                // Draw the lines
                gl::color4f(1.0, 1.0, 1.0, 1.0);
                gl::line_width(2.0);
                gl::line_stipple(1, wmi.stipple_pattern as u16);
                gl::enable(gl::LINE_STIPPLE);

                gl::begin(gl::LINE_STRIP);
                for (li, ip) in connection.vlocations.iter().enumerate() {
                    let Some(vl) = gr.world().get_vlocation(ip) else { continue };
                    let x = button.x + wmi.get_scaled_x(gr, vl.x, WORLDMAP_SMALL);
                    let y = button.y + wmi.get_scaled_y(gr, vl.y, WORLDMAP_SMALL);
                    gl::vertex2f(x as f32, y as f32);

                    let li = li as i32;
                    if connection.trace_in_progress() && connection.traceprogress == size - li - 1 && show_trace {
                        gl::color4f(1.0, 0.0, 0.0, 1.0);
                        gl::vertex2f(x as f32, y as f32);
                    }
                }
                gl::end();

                gl::line_width(1.0);
                gl::disable(gl::LINE_STIPPLE);
                gl::color4f(1.0, 1.0, 1.0, 1.0);

                // Draw the dots
                gl::begin(gl::QUADS);
                for (di, ip) in connection.vlocations.iter().enumerate() {
                    let Some(vl) = gr.world().get_vlocation(ip) else { continue };
                    let x = button.x + wmi.get_scaled_x(gr, vl.x, WORLDMAP_SMALL);
                    let y = button.y + wmi.get_scaled_y(gr, vl.y, WORLDMAP_SMALL);

                    gl::vertex2f((x - 1) as f32, (y - 1) as f32);
                    gl::vertex2f((x + 2) as f32, (y - 1) as f32);
                    gl::vertex2f((x + 2) as f32, (y + 2) as f32);
                    gl::vertex2f((x - 1) as f32, (y + 2) as f32);

                    let di = di as i32;
                    if connection.traceprogress == size - di - 1 && show_trace {
                        gl::color4f(1.0, 0.0, 0.0, 1.0);
                    }
                }
                gl::end();
            }

            // Draw red circles over computers infected with Revelation
            gl::begin(gl::QUADS);
            for ip in gr.world().plotgenerator.infected.iter() {
                let Some(vl) = gr.world().get_vlocation(ip) else { continue };
                let x = button.x + wmi.get_scaled_x(gr, vl.x, WORLDMAP_SMALL);
                // PORT: the C++ takes the y position with GetScaledX here too; kept as it is.
                let y = button.y + wmi.get_scaled_x(gr, vl.y, WORLDMAP_SMALL);

                gl::color4f(wmi.revelation_colour, 0.0, 0.0, 1.0);
                gl::vertex2f((x - 3) as f32, (y - 3) as f32);
                gl::vertex2f((x + 4) as f32, (y - 3) as f32);
                gl::vertex2f((x + 4) as f32, (y + 4) as f32);
                gl::vertex2f((x - 3) as f32, (y + 4) as f32);

                wmi.revelation_colour -= 0.09;
                if wmi.revelation_colour < 0.0 {
                    wmi.revelation_colour = 1.0;
                }
            }
            gl::end();

            if highlighted || clicked {
                gl::color4f(0.4, 0.4, 0.8, 1.0);
                border_draw(button);
            }
        });
    }

    /// C++ `WorldMapInterface::DrawWorldMapLarge` (static draw callback).
    pub fn draw_world_map_large(g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        // PORT: the C++ LIMIT_FRAMERATE block is an #ifdef, so it is omitted.
        with_hud_wmi(g, |g, wmi| {
            let gr: &Ctx = g;
            let Some(image) = button.image_standard.as_deref() else { return };

            // Clipping
            let screenheight = gr.app.options.get_option_value("graphics_screenheight");
            gl::scissor(button.x, screenheight - (button.y + button.height), button.width, button.height);
            gl::enable(gl::SCISSOR_TEST);

            // Draw the background image
            gl::color4f(1.0, 1.0, 1.0, 1.0);
            gl::enable(gl::TEXTURE_2D);
            gl::bind_texture(Some(image));

            let window_w = 1.0 / wmi.zoom;
            let window_h = 1.0 / wmi.zoom;
            let (sx, sy) = (wmi.scroll_x, wmi.scroll_y);
            let (x0, y0) = (button.x as f32, button.y as f32);
            let (x1, y1) = ((button.x + button.width) as f32, (button.y + button.height) as f32);

            gl::begin(gl::QUADS);
            gl::tex_coord2f(sx, 1.0 - sy);
            gl::vertex2f(x0, y0);
            gl::tex_coord2f(sx + window_w, 1.0 - sy);
            gl::vertex2f(x1, y0);
            gl::tex_coord2f(sx + window_w, 1.0 - (sy + window_h));
            gl::vertex2f(x1, y1);
            gl::tex_coord2f(sx, 1.0 - (sy + window_h));
            gl::vertex2f(x0, y1);
            gl::end();

            gl::disable(gl::TEXTURE_2D);

            // Gimme a border
            gl::color3ub(81, 138, 215);
            border_draw(button);

            // Draw the text labels, dots etc
            wmi.draw_all_objects(gr);

            // Draw red circles over computers infected with Revelation
            gl::begin(gl::QUADS);
            for ip in gr.world().plotgenerator.infected.iter() {
                let Some(vl) = gr.world().get_vlocation(ip) else { continue };
                let x = button.x + wmi.get_scaled_x(gr, vl.x, WORLDMAP_LARGE);
                // PORT: the C++ takes the y position with GetScaledX here too; kept as it is.
                let y = button.y + wmi.get_scaled_x(gr, vl.y, WORLDMAP_LARGE);

                gl::color4f(wmi.revelation_colour, 0.0, 0.0, 1.0);
                gl::vertex2f((x - 6) as f32, (y - 6) as f32);
                gl::vertex2f((x + 7) as f32, (y - 7) as f32);
                gl::vertex2f((x + 7) as f32, (y + 7) as f32);
                gl::vertex2f((x - 6) as f32, (y + 7) as f32);

                wmi.revelation_colour -= 0.09;
                if wmi.revelation_colour < 0.0 {
                    wmi.revelation_colour = 1.0;
                }
            }
            gl::end();

            // Draw connecting lines over the players connection
            let connection = &gr.player().connection;
            let size = connection.vlocations.len() as i32;
            let show_trace = gr.player().gateway.has_hud_upgrade(HUDUPGRADE_MAPSHOWSTRACE);

            gl::line_width(2.0);
            gl::color4f(1.0, 1.0, 1.0, 1.0);
            gl::line_stipple(2, wmi.stipple_pattern as u16);
            gl::enable(gl::LINE_STIPPLE);

            gl::begin(gl::LINE_STRIP);
            for (i, ip) in connection.vlocations.iter().enumerate() {
                let Some(vl) = gr.world().get_vlocation(ip) else { continue };
                let xpos = button.x + wmi.get_scaled_x(gr, vl.x, WORLDMAP_LARGE);
                let ypos = button.y + wmi.get_scaled_y(gr, vl.y, WORLDMAP_LARGE);

                gl::vertex2f(xpos as f32, ypos as f32);

                let i = i as i32;
                if connection.trace_in_progress() && connection.traceprogress == size - i - 1 && show_trace {
                    gl::color4f(1.0, 0.0, 0.0, 1.0);
                    gl::vertex2f(xpos as f32, ypos as f32);
                }
            }
            gl::end();

            gl::line_width(1.0);
            gl::disable(gl::LINE_STIPPLE);
            gl::disable(gl::SCISSOR_TEST);
        });
    }

    /// C++ `WorldMapInterface::DrawLocation` (static draw callback).
    pub fn draw_location(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        // Am I visible?
        let map_rect = Self::get_large_map_rect(g);
        let button_rect = MapRectangle::new(button.x - 2, button.y - 2, button.width + 4, button.height + 4);
        if !map_rect.contains(&button_rect) {
            return;
        }

        // Get some useful information about this location
        let Some(ip) = location_ip(&button.name) else { return };
        let stipple = hud_wmi(g).map(|w| w.stipple_pattern).unwrap_or(STIPPLE[0]);

        // If the player has an account, draw a surrounding box
        let accesslevel = g.player().has_account(g.world(), &ip);
        if accesslevel != -1 {
            let mut x = button.x - 2;
            let mut y = button.y - 2;
            let mut w = button.width + 3;
            let mut h = button.height + 3;

            if accesslevel > 1 {
                gl::line_stipple(1, stipple as u16);
                gl::enable(gl::LINE_STIPPLE);
                x -= 1;
                y -= 1;
                w += 2;
                h += 2;
            }

            gl::color3f(1.0, 1.0, 1.0);

            gl::begin(gl::LINE_LOOP);
            gl::vertex2i(x, y + h);
            gl::vertex2i(x, y);
            gl::vertex2i(x + w, y);
            gl::vertex2i(x + w, y + h);
            gl::end();

            gl::disable(gl::LINE_STIPPLE);
        }

        // If the player is highlighting this button, print some text in a box
        if clicked || highlighted {
            let Some(vl) = g.world().get_vlocation(&ip) else { return };
            let Some(comp) = g.world().get_computer(&vl.computer) else { return };
            let companyname = comp.companyname.clone();

            let Some(largemap) = g.ecl.get_button("worldmap_largemap").cloned() else { return };

            let screenheight = g.app.options.get_option_value("graphics_screenheight");
            let scissor_y = screenheight - (largemap.y + largemap.height);
            gl::scissor(largemap.x, scissor_y, largemap.width, largemap.height);
            gl::enable(gl::SCISSOR_TEST);

            // Draw a box behind the text
            let mut x = button.x - 15;
            let mut y = button.y + 20;

            let w1 = text::text_width_default(&format!("Owner: {}", companyname)) + 10;
            let w2 = text::text_width_default(&format!("IP: {}", ip)) + 10;
            let w = if w1 > w2 { w1 } else { w2 };
            let h = 26;

            // Make sure the info box doesn't get clipped
            if x < 0 {
                x = 0;
            }
            if y < 0 {
                y = 0;
            }
            if x + w + 2 > largemap.x + largemap.width {
                x = largemap.x + largemap.width - w - 2;
            }
            if y + h + 2 > largemap.y + largemap.height {
                y = largemap.y + largemap.height - h - 2;
            }

            // Draw the box
            gl::begin(gl::QUADS);
            gl::color3ub(8, 20, 0);
            gl::vertex2i(x, y + h);
            gl::color3ub(8, 20, 124);
            gl::vertex2i(x, y);
            gl::color3ub(8, 20, 0);
            gl::vertex2i(x + w, y);
            gl::color3ub(8, 20, 124);
            gl::vertex2i(x + w, y + h);
            gl::end();

            gl::color3ub(81, 138, 215);

            gl::begin(gl::LINE_LOOP);
            gl::vertex2i(x, y + h);
            gl::vertex2i(x, y);
            gl::vertex2i(x + w, y);
            gl::vertex2i(x + w, y + h);
            gl::end();

            // Draw the text
            gl::color3f(1.0, 1.0, 1.0);
            text::draw_text_default(x + 5, y + 10, &format!("IP: {}", ip));
            text::draw_text_default(x + 5, y + 20, &format!("Owner: {}", companyname));

            gl::disable(gl::SCISSOR_TEST);
        }
    }

    /// C++ `WorldMapInterface::ZoomInClick` (static button callback).
    pub fn zoom_in_click(g: &mut Ctx, _button: &Button) {
        with_hud_wmi(g, |g, wmi| wmi.change_zoom(g, 0.1));
    }

    /// C++ `WorldMapInterface::ZoomOutClick` (static button callback).
    pub fn zoom_out_click(g: &mut Ctx, _button: &Button) {
        with_hud_wmi(g, |g, wmi| wmi.change_zoom(g, -0.1));
    }

    /// C++ `WorldMapInterface::ZoomButtonClick` (static button callback).
    pub fn zoom_button_click(g: &mut Ctx, button: &Button) {
        let fraction_x = (get_mouse_x() - button.x) as f32 / button.width as f32;
        let zoom = 1.0 + fraction_x * (MAXZOOM - 1.0);
        with_hud_wmi(g, |g, wmi| wmi.set_zoom(g, zoom));
    }

    /// C++ `WorldMapInterface::ZoomButtonDraw` (static draw callback).
    pub fn zoom_button_draw(g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        clear_draw(g, button.x, button.y, button.width, button.height);

        // Zoom bar
        gl::color3ub(81, 138, 215);
        gl::begin(gl::TRIANGLES);
        gl::vertex2f(button.x as f32, (button.y + 5) as f32);
        gl::vertex2f((button.x + button.width) as f32, (button.y + 5) as f32);
        gl::vertex2f((button.x + button.width) as f32, (button.y + 10) as f32);
        gl::end();

        // Slider
        let zoom = hud_wmi(g).map(|w| w.zoom).unwrap_or(1.0);
        gl::color3f(0.0, 0.0, 0.7);
        let slider_x = 2.0 + (button.width - 4) as f32 * (zoom - 1.0) / (MAXZOOM - 1.0);

        gl::begin(gl::QUADS);
        gl::vertex2f(button.x as f32 + slider_x - 1.0, button.y as f32);
        gl::vertex2f(button.x as f32 + slider_x + 1.0, button.y as f32);
        gl::vertex2f(button.x as f32 + slider_x + 1.0, (button.y + button.height) as f32);
        gl::vertex2f(button.x as f32 + slider_x - 1.0, (button.y + button.height) as f32);
        gl::end();
    }

    /// C++ `WorldMapInterface::GetScaledX` (static): maps a map x to a button x for the given size.
    /// The large map depends on this map's zoom and scroll, so it is a method.
    pub fn get_scaled_x(&self, g: &Ctx, x: i32, size: i32) -> i32 {
        debug_assert!(x >= 0 && x < VIRTUAL_WIDTH);
        match size {
            WORLDMAP_SMALL => {
                let width = g.ecl.get_button("worldmap_smallmap").map(|b| b.width).unwrap_or(0);
                ((x as f32 / VIRTUAL_WIDTH as f32) * width as f32) as i32
            }
            WORLDMAP_LARGE => {
                let width = Self::get_large_map_width(g) as f32;
                let full_size = (x as f32 / VIRTUAL_WIDTH as f32) * width;
                (full_size * self.zoom - self.scroll_x * width * self.zoom) as i32
            }
            _ => -1,
        }
    }

    /// C++ `WorldMapInterface::GetScaledY` (static): maps a map y to a button y for the given size.
    pub fn get_scaled_y(&self, g: &Ctx, y: i32, size: i32) -> i32 {
        debug_assert!(y >= 0 && y < VIRTUAL_HEIGHT);
        match size {
            WORLDMAP_SMALL => {
                let height = g.ecl.get_button("worldmap_smallmap").map(|b| b.height).unwrap_or(0);
                ((y as f32 / VIRTUAL_HEIGHT as f32) * height as f32) as i32
            }
            WORLDMAP_LARGE => {
                let height = Self::get_large_map_height(g) as f32;
                let full_size = (y as f32 / VIRTUAL_HEIGHT as f32) * height;
                (full_size * self.zoom - self.scroll_y * height * self.zoom) as i32
            }
            _ => -1,
        }
    }

    /// C++ `WorldMapInterface::GetLargeMapRect` (static)
    pub fn get_large_map_rect(g: &Ctx) -> MapRectangle {
        MapRectangle::new(
            Self::get_large_map_x1(),
            Self::get_large_map_y1(),
            Self::get_large_map_width(g),
            Self::get_large_map_height(g),
        )
    }

    /// C++ `WorldMapInterface::GetLargeMapX1` (static)
    pub fn get_large_map_x1() -> i32 {
        23
    }

    /// C++ `WorldMapInterface::GetLargeMapY1` (static)
    pub fn get_large_map_y1() -> i32 {
        50
    }

    /// C++ `WorldMapInterface::GetLargeMapWidth` (static): screen width minus the side margins.
    pub fn get_large_map_width(g: &Ctx) -> i32 {
        let screenw = g.app.options.get_option_value("graphics_screenwidth");
        screenw - Self::get_large_map_x1() * 2
    }

    /// C++ `WorldMapInterface::GetLargeMapHeight` (static): keeps the 595:316 aspect ratio of the map.
    pub fn get_large_map_height(g: &Ctx) -> i32 {
        (316.0 * (Self::get_large_map_width(g) as f64 / 595.0)) as i32
    }

    /// The large-map position of a feature point at zoom 1 and no scroll. The C++ sets zoom and
    /// scroll to those values while it programs the layout, so the result does not depend on the view.
    fn unzoomed_large_point(g: &Ctx, vx: i32, vy: i32) -> (i32, i32) {
        let width = Self::get_large_map_width(g) as f32;
        let height = Self::get_large_map_height(g) as f32;
        let x = ((vx as f32 / VIRTUAL_WIDTH as f32) * width) as i32 + Self::get_large_map_x1() - 3;
        let y = ((vy as f32 / VIRTUAL_HEIGHT as f32) * height) as i32 + Self::get_large_map_y1() - 3;
        (x, y)
    }

    /// C++ `WorldMapInterface::ProgramLayoutEngine`: feeds the current locations into `layout`.
    pub fn program_layout_engine(&mut self, g: &Ctx) {
        // The connection's locations first, then the other known links that are displayed
        let mut points: Vec<(i32, i32, String)> = Vec::new();

        for ip in g.player().connection.vlocations.iter() {
            let Some(vl) = g.world().get_vlocation(ip) else { continue };
            if vl.displayed {
                let (x, y) = Self::unzoomed_large_point(g, vl.x, vl.y);
                points.push((x, y, vl.computer.clone()));
            }
        }

        for link in player_links(g) {
            let Some(vl) = g.world().get_vlocation(&link) else { continue };
            if !g.player().connection.location_included(&vl.ip) && vl.displayed {
                let (x, y) = Self::unzoomed_large_point(g, vl.x, vl.y);
                points.push((x, y, vl.computer.clone()));
            }
        }

        let Some(layout) = self.layout.as_mut() else { return };
        layout.reset();
        for (x, y, computer) in points {
            layout.add_location(x, y, &computer);
        }
    }

    /// C++ `WorldMapInterface::SaveCurrentConnection`
    pub fn save_current_connection(&mut self, g: &Ctx) {
        self.saved_connection = g.player().connection.vlocations.clone();
    }

    /// C++ `WorldMapInterface::LoadConnection`
    pub fn load_connection(&mut self, g: &mut Ctx) {
        if g.player().is_connected() {
            g.ecl.register_caption_change_auto(
                "worldmap_texthelp",
                "You cannot modify an existing connection. Disconnect first.",
                None,
            );
        } else {
            for ip in self.saved_connection.iter() {
                g.player_mut().get_connection_mut().add_vlocation(ip);
            }
        }
    }

    /// C++ `WorldMapInterface::CreateWorldMapInterface(int SIZE)` (static). `wmi` is the HUD's map,
    /// which builds the large map's layout.
    pub fn create_world_map_interface(g: &mut Ctx, wmi: &mut WorldMapInterface, size: i32) {
        if Self::is_visible_world_map_interface(g) != size {
            Self::remove_world_map_interface(g);

            if size == WORLDMAP_SMALL {
                Self::create_world_map_interface_small(g);
            } else if size == WORLDMAP_LARGE {
                Self::create_world_map_interface_large(g, wmi);
            } else {
                eprintln!("Unrecognised Map size");
            }
        }
    }

    /// C++ `WorldMapInterface::CreateWorldMapInterface_Small` (static)
    pub fn create_world_map_interface_small(g: &mut Ctx) {
        let screenw = g.app.options.get_option_value("graphics_screenwidth");
        let _screenh = g.app.options.get_option_value("graphics_screenheight");

        let fullsize_x = (screenw as f64 * PANELSIZE) as i32;
        let fullsize_y = (100.0 * (fullsize_x as f64 / 188.0)) as i32;

        g.ecl.register_button(
            screenw - fullsize_x - 3,
            3,
            fullsize_x,
            fullsize_y,
            "",
            "Global Communications",
            "worldmap_smallmap",
        );
        button_assignbitmap(g, "worldmap_smallmap", "worldmapsmall.tif");
        // Scale the freshly loaded bitmap; the Rc is not shared yet, so it can be mutated in place.
        if let Some(button) = g.ecl.get_button_mut("worldmap_smallmap") {
            if let Some(image) = button.image_standard.as_mut().and_then(Rc::get_mut) {
                image.scale(fullsize_x, fullsize_y);
            }
        }
        g.ecl.register_button_callbacks(
            "worldmap_smallmap",
            Some(Self::draw_world_map_small),
            Some(Self::full_screen_click),
            Some(button_click),
            Some(button_highlight),
        );

        g.ecl.register_button(screenw - fullsize_x - 3, fullsize_y + 4, fullsize_x, 15, "", "", "worldmap_connect");
        g.ecl.register_button_callbacks(
            "worldmap_connect",
            Some(Self::connect_draw),
            Some(Self::connect_click),
            Some(Self::connect_mouse_down),
            Some(Self::connect_mouse_move),
        );

        g.ecl.register_button(screenw - 3, 0, 3, 3, "", "Global Communications", "worldmap_toprightclick");
        g.ecl.register_button_callbacks(
            "worldmap_toprightclick",
            None,
            Some(Self::full_screen_click),
            Some(button_click),
            Some(button_highlight),
        );
    }

    /// C++ `WorldMapInterface::CreateWorldMapInterface_Large` (static). `wmi` is the HUD's map;
    /// its zoom, scroll and layout place the controls.
    pub fn create_world_map_interface_large(g: &mut Ctx, wmi: &mut WorldMapInterface) {
        sound::play_sound("sounds/openworldmap.wav", None, false);

        let map_rect = Self::get_large_map_rect(g);
        let x1 = map_rect.x1;
        let y1 = map_rect.y1;
        let fullsize_x = map_rect.width;
        let fullsize_y = map_rect.height;

        // Create the background bitmap
        g.ecl.register_button(x1, y1, fullsize_x, fullsize_y, "", "", "worldmap_largemap");
        button_assignbitmap(g, "worldmap_largemap", "worldmaplarge.tif");
        // PORT: the C++ DrawWorldMapLarge uploads the image with glTexImage2D(GL_RGB, GetRGBPixels),
        // i.e. without the alpha that load_themed_image set, so the map is opaque. The textured
        // quad here would use that alpha, so give the image full alpha (the C++ SetAlpha(0.99) line
        // is commented out as well).
        if let Some(img) = g
            .ecl
            .get_button_mut("worldmap_largemap")
            .and_then(|b| b.image_standard.as_mut())
            .and_then(std::rc::Rc::get_mut)
        {
            img.set_alpha(1.0);
            // PORT: C++ opengl.cpp sets GL_TEXTURE_MAG/MIN_FILTER to GL_LINEAR on the shared texture,
            // so the zoomed map is bilinearly filtered; the engine's textures default to nearest.
            img.texture().set_filter(macroquad::texture::FilterMode::Linear);
        }
        g.ecl.register_button_callbacks(
            "worldmap_largemap",
            Some(Self::draw_world_map_large),
            None,
            None,
            Some(button_highlight),
        );

        // The player's own location
        let localhost = g.player().localhost.clone();
        let Some((vx, vy)) = g.world().get_vlocation(&localhost).map(|v| (v.x, v.y)) else { return };
        let (lx, ly) = (
            wmi.get_scaled_x(g, vx, WORLDMAP_LARGE) + x1 - 3,
            wmi.get_scaled_y(g, vy, WORLDMAP_LARGE) + y1 - 3,
        );
        g.ecl.register_button(lx, ly, 7, 7, "Localhost", "Your physical location", "worldmap 127.0.0.1 0");
        g.ecl.register_button_callbacks(
            "worldmap 127.0.0.1 0",
            Some(Self::draw_location),
            None,
            None,
            Some(button_highlight),
        );

        // Create a text box for help messages
        g.ecl.register_button(120, y1 + fullsize_y + 3, 340, 25, "", "", "worldmap_texthelp");
        g.ecl.register_button_callbacks("worldmap_texthelp", Some(textbutton_draw), None, None, None);

        // Create the control buttons
        g.ecl.register_button(
            20,
            y1 + fullsize_y + 5,
            50,
            15,
            "Save",
            "Store the current connection settings",
            "worldmap_saveconnection",
        );
        g.ecl.register_button_callback("worldmap_saveconnection", Self::save_connection_click);

        g.ecl.register_button(
            75,
            y1 + fullsize_y + 5,
            50,
            15,
            "Load",
            "Load previously saved connection settings",
            "worldmap_loadconnection",
        );
        g.ecl.register_button_callback("worldmap_loadconnection", Self::load_connection_click);

        g.ecl.register_button(150, y1 + fullsize_y + 5, 15, 15, "-", "Zoom out", "worldmap_zoomout");
        g.ecl.register_button_callback("worldmap_zoomout", Self::zoom_out_click);

        g.ecl.register_button(170, y1 + fullsize_y + 5, 50, 15, "---", "Click to zoom", "worldmap_zoom");
        g.ecl.register_button_callbacks(
            "worldmap_zoom",
            Some(Self::zoom_button_draw),
            Some(Self::zoom_button_click),
            Some(button_click),
            Some(button_highlight),
        );

        g.ecl.register_button(225, y1 + fullsize_y + 5, 15, 15, "+", "Zoom in", "worldmap_zoomin");
        g.ecl.register_button_callback("worldmap_zoomin", Self::zoom_in_click);

        g.ecl.register_button(
            455,
            y1 + fullsize_y + 5,
            70,
            15,
            "Cancel",
            "Cancels changes made to your connection",
            "worldmap_cancel",
        );
        g.ecl.register_button_callback("worldmap_cancel", Self::cancel_click);

        g.ecl.register_button(530, y1 + fullsize_y + 5, 70, 15, "", "", "worldmap_connect");
        g.ecl.register_button_callbacks(
            "worldmap_connect",
            Some(Self::connect_draw),
            Some(Self::connect_click),
            Some(Self::connect_mouse_down),
            Some(Self::connect_mouse_move),
        );

        g.ecl.register_button(605, y1 + fullsize_y + 6, 13, 13, "", "Close the World Map screen", "worldmap_close");
        button_assignbitmaps(g, "worldmap_close", "close.tif", "close_h.tif", "close_c.tif");
        g.ecl.register_button_callback("worldmap_close", Self::close_click);

        // Create all known links
        for (i, link) in player_links(g).iter().enumerate() {
            let Some((ip, vx, vy, displayed)) =
                g.world().get_vlocation(link).map(|v| (v.ip.clone(), v.x, v.y, v.displayed))
            else {
                continue;
            };

            if displayed || g.player().connection.location_included(&ip) {
                let name = format!("worldmap {} {}", ip, i);
                let tooltip = format!("Connect to IP address {}", ip);
                let x = wmi.get_scaled_x(g, vx, WORLDMAP_LARGE) + x1 - 3;
                let y = wmi.get_scaled_y(g, vy, WORLDMAP_LARGE) + y1 - 3;

                g.ecl.register_button(x, y, 7, 7, &ip, &tooltip, &name);
                g.ecl.register_button_callbacks(
                    &name,
                    Some(Self::draw_location),
                    Some(Self::location_click),
                    Some(button_click),
                    Some(button_highlight),
                );
            }
        }

        if let Some(layout) = wmi.layout.as_mut() {
            layout.full_layout_labels();
        }

        // Bring all running tasks to the front
        Vanbakel::show_all_tasks(g);
    }

    /// C++ `WorldMapInterface::IsVisibleWorldMapInterface` (static): returns WORLDMAP_SMALL, WORLDMAP_LARGE or WORLDMAP_NONE.
    pub fn is_visible_world_map_interface(g: &Ctx) -> i32 {
        if g.ecl.get_button("worldmap_smallmap").is_some() {
            WORLDMAP_SMALL
        } else if g.ecl.get_button("worldmap_largemap").is_some() {
            WORLDMAP_LARGE
        } else {
            WORLDMAP_NONE
        }
    }

    /// Moves every link button to match the current zoom and scroll.
    fn reposition_links(&self, g: &mut Ctx) {
        let x1 = Self::get_large_map_x1();
        let y1 = Self::get_large_map_y1();

        for (i, ip) in player_links(g).iter().enumerate() {
            let Some((vx, vy)) = g.world().get_vlocation(ip).map(|v| (v.x, v.y)) else { continue };
            let x = self.get_scaled_x(g, vx, WORLDMAP_LARGE) + x1 - 3;
            let y = self.get_scaled_y(g, vy, WORLDMAP_LARGE) + y1 - 3;

            let name = format!("worldmap {} {}", ip, i);
            if let Some(button) = g.ecl.get_button_mut(&name) {
                button.x = x;
                button.y = y;
            }
        }
    }

    /// C++ `WorldMapInterface::ScrollX`
    pub fn scroll_x(&mut self, g: &mut Ctx, x: f32) {
        // background
        let window_w = 1.0 / self.zoom;

        self.scroll_x += x;
        if self.scroll_x < 0.0 {
            self.scroll_x = 0.0;
        }
        if self.scroll_x + window_w > 1.0 {
            self.scroll_x = 1.0 - window_w;
        }
        g.ecl.dirty_button("worldmap_largemap");

        // Buttons
        self.reposition_links(g);
    }

    /// C++ `WorldMapInterface::ScrollY`
    pub fn scroll_y(&mut self, g: &mut Ctx, y: f32) {
        // background
        let window_h = 1.0 / self.zoom;

        self.scroll_y += y;
        if self.scroll_y < 0.0 {
            self.scroll_y = 0.0;
        }
        if self.scroll_y + window_h > 1.0 {
            self.scroll_y = 1.0 - window_h;
        }
        g.ecl.dirty_button("worldmap_largemap");

        // Buttons
        self.reposition_links(g);
    }

    /// C++ `WorldMapInterface::SetZoom`
    pub fn set_zoom(&mut self, g: &mut Ctx, z: f32) {
        let old_window_w = 1.0 / self.zoom;
        let old_window_h = 1.0 / self.zoom;

        self.zoom = z;
        if self.zoom < 1.0 {
            self.zoom = 1.0;
        }
        if self.zoom > MAXZOOM {
            self.zoom = MAXZOOM;
        }

        let window_w = 1.0 / self.zoom;
        let window_h = 1.0 / self.zoom;

        self.scroll_x += (old_window_w - window_w) / 2.0;
        self.scroll_y += (old_window_h - window_h) / 2.0;

        if self.scroll_x < 0.0 {
            self.scroll_x = 0.0;
        }
        if self.scroll_x + window_w > 1.0 {
            self.scroll_x = 1.0 - window_w;
        }
        if self.scroll_y < 0.0 {
            self.scroll_y = 0.0;
        }
        if self.scroll_y + window_h > 1.0 {
            self.scroll_y = 1.0 - window_h;
        }

        g.ecl.dirty_button("worldmap_largemap");

        // Buttons
        self.reposition_links(g);

        g.ecl.dirty_button("worldmap_zoom");
    }

    /// C++ `WorldMapInterface::ChangeZoom`
    pub fn change_zoom(&mut self, g: &mut Ctx, z: f32) {
        self.set_zoom(g, self.zoom + z);
    }

    /// C++ `WorldMapInterface::CheckLinksChanged`
    pub fn check_links_changed(&mut self, g: &mut Ctx) {
        let links = player_links(g);

        // Count the number of links that should be displayed
        let mut count = g.player().connection.get_size();
        for link in &links {
            if !g.player().connection.location_included(link) {
                let displayed = g.world().get_vlocation(link).map(|v| v.displayed).unwrap_or(false);
                if displayed {
                    count += 1;
                }
            }
        }

        // Program Layout Engine if the num links has changed
        let current = self.layout.as_ref().map(|l| l.get_locations().len() as i32);
        if current != Some(count) {
            self.program_layout_engine(g);
        }
    }

    /// C++ `WorldMapInterface::Create(int SIZE)`: builds a fresh layout, then the map of that size.
    pub fn create_size(&mut self, g: &mut Ctx, size: i32) {
        self.layout = Some(WorldMapLayout::new(&Self::get_large_map_rect(g)));
        self.program_layout_engine(g);

        Self::create_world_map_interface(g, self, size);
    }
}

impl InterfaceScreen for WorldMapInterface {
    /// C++ `WorldMapInterface::Create()`: the large map.
    fn create(&mut self, g: &mut Ctx) {
        self.create_size(g, WORLDMAP_LARGE);
    }

    /// C++ `WorldMapInterface::Update`
    fn update(&mut self, g: &mut Ctx) {
        self.check_links_changed(g);
        if let Some(layout) = self.layout.as_mut() {
            layout.partial_layout_labels();
        }

        if Self::is_visible_world_map_interface(g) != WORLDMAP_NONE {
            let timesincelastupdate = g.ecl.get_accurate_time() - self.last_update;

            // Update the small map
            if timesincelastupdate > 100 && g.ecl.get_button("worldmap_smallmap").is_some() {
                g.ecl.dirty_button("worldmap_smallmap");
                g.ecl.dirty_button("worldmap_connect");
                self.cycle_stipple_pattern();
                self.last_update = g.ecl.get_accurate_time();
            }

            // Update the large map
            if timesincelastupdate > 200 && g.ecl.get_button("worldmap_largemap").is_some() {
                g.ecl.dirty_button("worldmap_largemap");
                self.cycle_stipple_pattern();
                self.last_update = g.ecl.get_accurate_time();
            }

            // Update scrolling
            let timesincelastscroll = g.ecl.get_accurate_time() - self.scroll_update;

            if timesincelastscroll > 20 {
                let Some(worldmap) = g.ecl.get_button("worldmap_largemap").cloned() else { return };

                let mouse_x = get_mouse_x() - worldmap.x;
                let mouse_y = get_mouse_y() - worldmap.y;

                if mouse_x < 0 || mouse_y < 0 || mouse_x > worldmap.width || mouse_y > worldmap.height {
                    return;
                }

                let d_x = (mouse_x as f64 - worldmap.width as f64 / 2.0) as i32;
                let d_y = (mouse_y as f64 - worldmap.height as f64 / 2.0) as i32;

                let scroll_threshold_x = (worldmap.width as f64 * 0.42) as i32;
                let scroll_threshold_y = (worldmap.height as f64 * 0.40) as i32;

                if d_x > scroll_threshold_x {
                    self.scroll_x(g, (d_x - scroll_threshold_x) as f32 / 3500.0);
                }
                if d_x < -scroll_threshold_x {
                    self.scroll_x(g, (d_x + scroll_threshold_x) as f32 / 3500.0);
                }

                if d_y > scroll_threshold_y {
                    self.scroll_y(g, (d_y - scroll_threshold_y) as f32 / 3500.0);
                }
                if d_y < -scroll_threshold_y {
                    self.scroll_y(g, (d_y + scroll_threshold_y) as f32 / 3500.0);
                }

                self.scroll_update = g.ecl.get_accurate_time();
            }
        }
    }

    /// C++ `WorldMapInterface::Remove`
    fn remove(&mut self, g: &mut Ctx) {
        Self::remove_world_map_interface(g);
    }

    /// C++ `WorldMapInterface::IsVisible`
    fn is_visible(&self, g: &Ctx) -> bool {
        Self::is_visible_world_map_interface(g) != WORLDMAP_NONE
    }

    /// C++ `WorldMapInterface::ScreenID`
    fn screen_id(&self) -> i32 {
        SCREEN_MAP
    }

    impl_as_any!();
}
