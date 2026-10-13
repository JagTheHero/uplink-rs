//! Port of source/interface/localinterface/lan_interface.h + source/interface/localinterface/lan_interface.cpp
//! LAN viewer: draws the systems of the target LAN and their links, and lets the player connect to them.

use crate::app::globals;
use crate::app::opengl::{screen_h, screen_w};
use crate::app::opengl_interface::{
    border_draw, button_assignbitmap, button_click, button_draw, button_highlight, clear_draw,
    set_colour, sx, sy, textbutton_draw,
};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::impl_as_any;
use crate::interface::interfacescreen::{InterfaceScreen, ScreenSlot};
use crate::interface::localinterface::cheat_interface::CheatInterface;
use crate::interface::localinterface::localinterface::{LocalInterface, PANELSIZE, SCREEN_LAN, SCREEN_NONE};
use crate::interface::remoteinterface::remoteinterface::RemoteInterface;
use crate::interface::taskmanager::taskmanager::TaskManager;
use crate::objref::ObjRef;
use crate::world::computer::computer::Computer;
use crate::world::computer::lancomputer::{
    LanComputer, LanComputerLink, LanComputerSystem, LANLINKVISIBLE_AWARE, LANLINKVISIBLE_FROMAWARE,
    LANLINKVISIBLE_NONE, LANLINKVISIBLE_TOAWARE, LANSYSTEMVISIBLE_AWARE, LANSYSTEMVISIBLE_FULL,
    LANSYSTEMVISIBLE_NONE, LANSYSTEMVISIBLE_TYPE, LANSYSTEM_AUTHENTICATION, LANSYSTEM_ISOLATIONBRIDGE,
    LANSYSTEM_LOCK, LANSYSTEM_MAINSERVER, LANSYSTEM_NUMTYPES,
};
use crate::world::computer::lanmonitor::{LanMonitor, SYSADMIN_CURIOUS, SYSADMIN_FOUNDYOU, SYSADMIN_SEARCHING};
use crate::world::generator::langenerator::LanGenerator;
use uplink_engine::{gl, text};

/// C++ `struct LanInterfaceObject`: display data for one LAN system type.
#[derive(Debug, Clone, Copy)]
pub struct LanInterfaceObject {
    pub name: &'static str,
    pub filename: &'static str,
    pub width: i32,
    pub height: i32,
    pub description: &'static str,
}

/// C++ `LanInterface::lanInterfaceObjects[LANSYSTEM_NUMTYPES]`, indexed by the LANSYSTEM_* type.
// PORT: constant display data, so a const table rather than a static field.
pub const LAN_INTERFACE_OBJECTS: [LanInterfaceObject; LANSYSTEM_NUMTYPES as usize] = [
    LanInterfaceObject { name: "None", filename: " ", width: 0, height: 0, description: "None" },
    LanInterfaceObject { name: "Router", filename: "lan/router.tif", width: 48, height: 32, description: "A router.\n\nThe public entry point to this LAN. The main external Internet connection runs through here." },
    LanInterfaceObject { name: "Hub", filename: "lan/hub.tif", width: 48, height: 20, description: "A hub.\n\nConnects numerous seperate systems together." },
    LanInterfaceObject { name: "Switch", filename: "lan/hub.tif", width: 48, height: 20, description: "A switch.\n\nConnects two computer systems together." },
    LanInterfaceObject { name: "Terminal", filename: "lan/terminal.tif", width: 38, height: 37, description: "A terminal.\n\nA system inside the company, used by the employees." },
    LanInterfaceObject { name: "Main Server", filename: "lan/centralsystem.tif", width: 72, height: 51, description: "A main server.\n\nThe central controlling computer system." },
    LanInterfaceObject { name: "Mail Server", filename: "lan/mailserver.tif", width: 48, height: 51, description: "A mail server.\n\nStores employees personal emails, messages and address books." },
    LanInterfaceObject { name: "File Server", filename: "lan/mailserver.tif", width: 48, height: 51, description: "A file server.\n\nThe central file repository of the Network." },
    LanInterfaceObject { name: "Authentication Server", filename: "lan/authentication.tif", width: 48, height: 51, description: "An authentication server\n\nDirectly controls the status of a Lock system, permitting or denying access." },
    LanInterfaceObject { name: "Lock", filename: "lan/lock.tif", width: 32, height: 32, description: "A lock.\n\nPrevents any access to the systems behind the lock when activated." },
    LanInterfaceObject { name: "Isolation Bridge", filename: "lan/isolation.tif", width: 32, height: 32, description: "An isolation bridge\n\nToggles the status of two Lock systems on the network." },
    LanInterfaceObject { name: "Modem", filename: "lan/modem.tif", width: 32, height: 32, description: "A modem.\n\nCan be used as an external access point to the network." },
    LanInterfaceObject { name: "Session Key Server", filename: "lan/session.tif", width: 32, height: 32, description: "A session key server" },
    LanInterfaceObject { name: "Wireless Transmitter", filename: "lan/radiotrans.tif", width: 32, height: 32, description: "A Wireless LAN transmitter.\n\nCan be used to connect to a Wireless receiver on the network." },
    LanInterfaceObject { name: "Wireless Receiver", filename: "lan/radiorec.tif", width: 32, height: 32, description: "A Wireless LAN receiver.\n\nCan be accessed by a Wireless transmitter on the network." },
    LanInterfaceObject { name: "Fax Printer", filename: "lan/hub.tif", width: 48, height: 20, description: "A fax printer" },
    LanInterfaceObject { name: "Log Server", filename: "lan/centralsystem.tif", width: 72, height: 51, description: "A log server.  Stores all access logs for this LAN." },
];

/// C++ `#define LANHIGHLIGHT_*`: what a highlighted system's outline means.
pub const LANHIGHLIGHT_NONE: i32 = 0;
pub const LANHIGHLIGHT_VALIDSUBNET: i32 = 1;
pub const LANHIGHLIGHT_CONTROLS: i32 = 2;

/// C++ `class LanInterfaceHighlight`: a caption drawn next to a system (e.g. "Locks/Unlocks").
#[derive(Debug, Clone)]
pub struct LanInterfaceHighlight {
    pub system_index: i32,
    /// C++ `char text[64]`.
    // PORT: the C++ strcpy is unchecked; the Rust text is cut to 63 characters to match the buffer.
    pub text: String,
}

impl LanInterfaceHighlight {
    /// C++ `LanInterfaceHighlight(int _systemIndex, char *_text)`
    pub fn new(system_index: i32, text: &str) -> Self {
        LanInterfaceHighlight {
            system_index,
            text: text.chars().take(63).collect(),
        }
    }
}

/// C++ `LanInterface::GetLanInterfaceObject(int TYPE)`
pub fn get_lan_interface_object(ty: i32) -> &'static LanInterfaceObject {
    &LAN_INTERFACE_OBJECTS[ty as usize]
}

// PORT: file-level stipple table and its cycling state (file statics in C++). The state is kept
// on LanInterface (see the fields below) so the draw callbacks can reach it.
pub const STIPPLE: [i32; 8] = [
    0x1111, 0x1111, 0x2222, 0x2222, 0x4444, 0x4444, 0x8888, 0x8888,
];

/// C++ `class LanInterface : LocalInterfaceScreen`. The C++ statics (`offsetX`, `offsetY`, and the
/// file-level stipple state) are fields here, reached through `g.interface().li.screen`.
// PORT: static -> field for every static member of LanInterface.
#[derive(Debug, Clone, Default)]
pub struct LanInterface {
    /// C++ `char ip[SIZE_VLOCATION_IP]` (SIZE_VLOCATION_IP = 24): IP of the LAN being viewed.
    pub ip: String,
    /// C++ `LList<LanInterfaceHighlight *> highlights`
    pub highlights: Vec<LanInterfaceHighlight>,
    /// C++ `static int offsetX`, `offsetY`: pan offset of the LAN view.
    pub offset_x: i32,
    pub offset_y: i32,
    /// C++ file static `stippleindex`
    pub stipple_index: i32,
    /// C++ file static `stipplepattern`
    pub stipple_pattern: i32,
    /// C++ file static `stippleupdate`
    pub stipple_update: i32,
}

/// `(x, y, width, height)` of a button.
type Rect = (i32, i32, i32, i32);

/// C++ `sscanf(button->name, "%s %d", unused, &n)`: the number after the first word of a button name.
// PORT: sscanf leaves the C++ variable unset on a bad name; 0 is used here instead.
fn name_number(name: &str) -> i32 {
    name.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0)
}

/// C++ `EclGetButton("lan_background")` (UplinkAssert'ed): its rectangle.
fn background_rect(g: &Ctx) -> Rect {
    let b = g.ecl.get_button("lan_background").expect("lan_background button is registered");
    (b.x, b.y, b.width, b.height)
}

/// C++ `game->GetWorld()->GetPlayer()->remotehost` -> `GetVLocation` -> `GetComputer` (asserted present).
fn remote_computer(g: &Ctx) -> &Computer {
    let vl = g.world().get_vlocation(&g.player().remotehost).expect("remote host is a VLocation");
    vl.get_computer(g.world()).expect("VLocation has a computer")
}

/// The remote computer as a LAN, or `None` when it is not one (C++ `comp->TYPE != COMPUTER_TYPE_LAN`).
fn remote_lan(g: &Ctx) -> Option<&LanComputer> {
    remote_computer(g).lan()
}

/// The C++ `LanMonitor` statics.
fn lan_monitor(g: &Ctx) -> &LanMonitor {
    &g.lan_monitor
}

fn lan_monitor_mut(g: &mut Ctx) -> &mut LanMonitor {
    &mut g.lan_monitor
}

/// The LAN screen's slot in the local interface (C++ `GetLocalInterface()->GetInterfaceScreen()`).
fn lan_slot(g: &mut Ctx) -> &mut ScreenSlot {
    &mut g.interface_mut().li.screen
}

/// Runs `f` on the LAN screen with it taken out of its slot. `None` when the LAN screen is not current.
fn with_lan<R>(g: &mut Ctx, f: impl FnOnce(&mut LanInterface, &mut Ctx) -> R) -> Option<R> {
    ScreenSlot::call(g, lan_slot, |screen, g| {
        let lan = screen.as_any_mut().downcast_mut::<LanInterface>()?;
        Some(f(lan, g))
    })
    .flatten()
}

/// C++ statics `offsetX`, `offsetY` and `stipplepattern`, or their defaults when the LAN screen is not current.
fn view_state(g: &Ctx) -> (i32, i32, i32) {
    match g.interface().li.screen.get::<LanInterface>() {
        Some(lan) => (lan.offset_x, lan.offset_y, lan.stipple_pattern),
        None => (0, 0, STIPPLE[0]),
    }
}

/// C++ `thisInt->highlights` of the LAN screen (empty when the LAN screen is not current).
fn highlights_of(g: &Ctx) -> &[LanInterfaceHighlight] {
    match g.interface().li.screen.get::<LanInterface>() {
        Some(lan) => &lan.highlights,
        None => &[],
    }
}

/// C++ `ConnectDraw` / `ConnectClick` / `ConnectMouseMove` condition.
fn can_connect(g: &Ctx) -> bool {
    if remote_lan(g).is_none() {
        return false;
    }
    let mon = lan_monitor(g);
    mon.current_system != mon.current_selected && mon.current_selected != -1
}

/// C++ `BackDraw` / `BackMouseMove` condition.
fn can_back(g: &Ctx) -> bool {
    remote_lan(g).is_some() && lan_monitor(g).connection.len() > 1
}

/// C++ `lanComp->systems.ValidIndex(i)` followed by `GetData(i)`.
fn system_at(lan: &LanComputer, index: i32) -> Option<&LanComputerSystem> {
    if lan.systems.valid_index(index) {
        lan.systems.get_data(index)
    } else {
        None
    }
}

/// The visibility of system `index` of the remote LAN, or `None` when there is no such system.
fn system_visibility(g: &Ctx, index: i32) -> Option<i32> {
    system_at(remote_lan(g)?, index).map(|s| s.visible)
}

/// One valid system of a LAN, placed relative to the background box.
struct SystemSpot {
    index: i32,
    x: i32,
    y: i32,
    obj: &'static LanInterfaceObject,
}

fn system_spots(lan: &LanComputer) -> Vec<SystemSpot> {
    let mut spots = Vec::new();
    for i in 0..lan.systems.size() {
        if let Some(system) = system_at(lan, i) {
            spots.push(SystemSpot {
                index: i,
                x: system.x,
                y: system.y,
                obj: get_lan_interface_object(system.type_),
            });
        }
    }
    spots
}

/// C++ `LanInterface::LanBackgroundDraw` helper: a LINE_LOOP around a box grown by `grow` pixels.
fn draw_outline(x: i32, y: i32, width: i32, height: i32, grow: i32) {
    gl::begin(gl::LINE_LOOP);
    gl::vertex2i(x - grow, y - grow);
    gl::vertex2i(x + width + grow, y - grow);
    gl::vertex2i(x + width + grow, y + height + grow);
    gl::vertex2i(x - grow, y + height + grow);
    gl::end();
}

/// C++ link end point: a filled 4x4 square centred on the point.
fn draw_point_quad(x: i32, y: i32) {
    gl::begin(gl::QUADS);
    gl::vertex2i(x - 2, y - 2);
    gl::vertex2i(x + 2, y - 2);
    gl::vertex2i(x + 2, y + 2);
    gl::vertex2i(x - 2, y + 2);
    gl::end();
}

/// C++ connecting point of a link: `base + size * frac + offset`, computed in float and truncated.
fn link_point(base: i32, size: i32, frac: f32, offset: i32) -> i32 {
    (base as f32 + size as f32 * frac + offset as f32) as i32
}

/// C++ `LanInterface::DrawLink` body: one link between two points, routed by its connection sides.
fn draw_link_lines(link: &LanComputerLink, from_x: f32, from_y: f32, to_x: f32, to_y: f32) {
    if (link.from_y == 1.0 && link.to_y == 0.0) || (link.from_y == 0.0 && link.to_y == 1.0) {
        // Bottom to top, top to bottom
        gl::begin(gl::LINE_STRIP);
        gl::vertex2i(from_x as i32, from_y as i32);
        if from_x > to_x {
            let mid_y = (from_y + (to_y - from_y) * link.from_x) as i32;
            gl::vertex2i(from_x as i32, mid_y);
            gl::vertex2i(to_x as i32, mid_y);
        } else {
            // C++ evaluates `1.0 - link->fromX` in double precision.
            let mid_y = (from_y as f64 + (to_y as f64 - from_y as f64) * (1.0 - link.from_x as f64)) as i32;
            gl::vertex2i(from_x as i32, mid_y);
            gl::vertex2i(to_x as i32, mid_y);
        }
        gl::vertex2i(to_x as i32, to_y as i32);
        gl::end();
    } else if (link.from_x == 1.0 && link.to_x == 0.0) || (link.from_x == 0.0 && link.to_x == 1.0) {
        // Right to left, left to right
        gl::begin(gl::LINE_STRIP);
        gl::vertex2i(from_x as i32, from_y as i32);
        if from_y > to_y {
            let mid_x = (from_x + (to_x - from_x) * link.from_y) as i32;
            gl::vertex2i(mid_x, from_y as i32);
            gl::vertex2i(mid_x, to_y as i32);
        } else {
            // C++ evaluates `1.0 - link->fromY` in double precision.
            let mid_x = (from_x as f64 + (to_x as f64 - from_x as f64) * (1.0 - link.from_y as f64)) as i32;
            gl::vertex2i(mid_x, from_y as i32);
            gl::vertex2i(mid_x, to_y as i32);
        }
        gl::vertex2i(to_x as i32, to_y as i32);
        gl::end();
    } else if (link.from_y == 0.0 && link.to_x == 0.0)
        || (link.from_y == 1.0 && link.to_x == 1.0)
        || (link.from_y == 1.0 && link.to_x == 0.0)
        || (link.from_y == 0.0 && link.to_x == 1.0)
    {
        // Top to left, bottom to right, bottom to left, top to right
        gl::begin(gl::LINE_STRIP);
        gl::vertex2i(from_x as i32, from_y as i32);
        gl::vertex2i(from_x as i32, to_y as i32);
        gl::vertex2i(to_x as i32, to_y as i32);
        gl::end();
    } else {
        // Right to top, left to bottom, left to top, right to bottom, and the C++ "Fuck knows"
        // fallback: all of these draw the same route.
        gl::begin(gl::LINE_STRIP);
        gl::vertex2i(from_x as i32, from_y as i32);
        gl::vertex2i(to_x as i32, from_y as i32);
        gl::vertex2i(to_x as i32, to_y as i32);
        gl::end();
    }
}

/// C++ `LanInterface::LanBackgroundDraw` body: the selection boxes, links and highlights of the LAN.
fn draw_lan_contents(g: &Ctx, lan: &LanComputer, bg: Rect) {
    let (bx, by, _, _) = bg;
    let (offset_x, offset_y, stipple) = view_state(g);
    let mon = lan_monitor(g);

    // Draw current selection box
    if let Some(system) = system_at(lan, mon.current_selected) {
        let obj = get_lan_interface_object(system.type_);
        let x = bx + system.x + offset_x;
        let y = by + system.y + offset_y;
        gl::color4f(1.0, 1.0, 1.0, 1.0);
        gl::line_width(2.0);
        gl::line_stipple(2, stipple as u16);
        gl::enable(gl::LINE_STIPPLE);
        draw_outline(x, y, obj.width, obj.height, 4);
        gl::line_width(1.0);
        gl::disable(gl::LINE_STIPPLE);
    }

    // Draw current system box
    if let Some(system) = system_at(lan, mon.current_system) {
        let obj = get_lan_interface_object(system.type_);
        let x = bx + system.x + offset_x;
        let y = by + system.y + offset_y;
        gl::color4f(0.7, 0.7, 1.0, 1.0);
        draw_outline(x, y, obj.width, obj.height, 5);
        draw_outline(x, y, obj.width, obj.height, 3);
        gl::disable(gl::LINE_STIPPLE);
    }

    // Draw connecting lines
    gl::line_width(2.0);
    gl::color4f(0.0, 0.5, 0.6, 1.0);
    for i in 0..lan.links.size() {
        if !lan.links.valid_index(i) {
            continue;
        }
        let link = lan.links.get_data(i).expect("valid link index");
        if link.visible <= LANLINKVISIBLE_NONE {
            continue;
        }
        let from = lan.systems.get_data(link.from).expect("link source is a system");
        let to = lan.systems.get_data(link.to).expect("link target is a system");
        let from_obj = get_lan_interface_object(from.type_);
        let to_obj = get_lan_interface_object(to.type_);

        // Work out connecting points for link
        let from_px = link_point(bx + from.x, from_obj.width, link.from_x, offset_x);
        let from_py = link_point(by + from.y, from_obj.height, link.from_y, offset_y);
        let to_px = link_point(bx + to.x, to_obj.width, link.to_x, offset_x);
        let to_py = link_point(by + to.y, to_obj.height, link.to_y, offset_y);

        // Draw the FROM link point
        if link.visible == LANLINKVISIBLE_FROMAWARE || link.visible >= LANLINKVISIBLE_AWARE {
            draw_point_quad(from_px, from_py);
        }

        // Draw the TO link point
        if link.visible == LANLINKVISIBLE_TOAWARE || link.visible >= LANLINKVISIBLE_AWARE {
            draw_point_quad(to_px, to_py);
        }

        // Draw the link itself
        if link.visible >= LANLINKVISIBLE_AWARE {
            gl::line_width(2.0);
            draw_link_lines(link, from_px as f32, from_py as f32, to_px as f32, to_py as f32);
            gl::line_width(1.0);

            let to_index = mon.get_node_index(link.to);
            let from_index = mon.get_node_index(link.from);
            let sysadmin_index = mon.get_node_index(mon.sys_admin_current_system);

            if to_index != -1 && from_index != -1 && (to_index == from_index - 1 || from_index == to_index - 1) {
                gl::enable(gl::LINE_STIPPLE);
                gl::line_stipple(2, stipple as u16);

                gl::color4f(0.3, 0.3, 0.9, 1.0);
                gl::line_width(4.0);
                draw_link_lines(link, from_px as f32, from_py as f32, to_px as f32, to_py as f32);

                if (sysadmin_index >= to_index && to_index >= from_index)
                    || (sysadmin_index >= from_index && from_index >= to_index)
                {
                    gl::color4f(1.0, 0.0, 0.0, 1.0);
                } else {
                    gl::color4f(1.0, 1.0, 1.0, 1.0);
                }

                gl::line_width(2.0);
                draw_link_lines(link, from_px as f32, from_py as f32, to_px as f32, to_py as f32);

                gl::color4f(0.0, 0.5, 0.6, 1.0);
                gl::disable(gl::LINE_STIPPLE);
                gl::line_width(1.0);
            }
        }
    }

    // Draw highlights
    for lih in highlights_of(g) {
        if !lan.systems.valid_index(lih.system_index) {
            continue;
        }
        let system = lan.systems.get_data(lih.system_index).expect("valid highlight index");
        let obj = get_lan_interface_object(system.type_);
        let x = bx + system.x + offset_x;
        let y = by + system.y + offset_y;

        gl::color4f(0.8, 0.8, 0.1, 1.0);
        gl::line_width(1.0);
        gl::enable(gl::LINE_STIPPLE);
        gl::line_stipple(1, stipple as u16);
        draw_outline(x, y, obj.width, obj.height, 3);
        gl::disable(gl::LINE_STIPPLE);
        gl::line_width(1.0);

        text::draw_text_default(x - 10, y + obj.height + 15, &lih.text);
    }

    gl::line_width(1.0);
}

/// C++ `LanInterface::LanBackgroundDraw` tail: the System Administrator's message and the system he is on.
fn draw_sysadmin_contents(g: &Ctx, bg: Rect, text_x: i32, text_y: i32) {
    let (bx, by, _, _) = bg;
    let Some(lan) = remote_lan(g) else { return };
    let (offset_x, offset_y, stipple) = view_state(g);
    let mon = lan_monitor(g);

    gl::color4f(1.0, 0.0, 0.0, 1.0);
    match mon.sys_admin_state {
        SYSADMIN_CURIOUS => text::draw_text_default(text_x, text_y, "The System Administrator has logged on"),
        SYSADMIN_SEARCHING => text::draw_text_default(text_x, text_y, "The System Administrator is searching for you"),
        SYSADMIN_FOUNDYOU => text::draw_text_default(text_x, text_y, "The System Administrator has found you"),
        _ => {}
    }

    if let Some(system) = system_at(lan, mon.sys_admin_current_system) {
        let obj = get_lan_interface_object(system.type_);
        let x = bx + system.x + offset_x;
        let y = by + system.y + offset_y;

        gl::color4f(1.0, 0.0, 0.0, 1.0);
        gl::line_width(2.0);
        gl::line_stipple(2, stipple as u16);
        gl::enable(gl::LINE_STIPPLE);
        draw_outline(x, y, obj.width, obj.height, 4);
        gl::line_width(1.0);
        gl::disable(gl::LINE_STIPPLE);
    }
}

/// C++ `LanInterface::SelectSystem` highlight rules: what each highlight caption means for `system`.
fn build_highlights(lan: &LanComputer, system_index: i32, system: &LanComputerSystem) -> Vec<LanInterfaceHighlight> {
    let mut highlights = Vec::new();
    match system.type_ {
        LANSYSTEM_AUTHENTICATION => {
            highlights.push(LanInterfaceHighlight::new(system.data1, "Locks/Unlocks"));
        }
        LANSYSTEM_ISOLATIONBRIDGE => {
            highlights.push(LanInterfaceHighlight::new(system.data1, "Locks"));
            highlights.push(LanInterfaceHighlight::new(system.data2, "Unlocks"));
        }
        LANSYSTEM_LOCK => {
            for i in 0..lan.systems.size() {
                if let Some(this_sys) = system_at(lan, i) {
                    if this_sys.type_ == LANSYSTEM_AUTHENTICATION
                        && (this_sys.data1 == system_index || this_sys.data2 == system_index || this_sys.data3 == system_index)
                    {
                        highlights.push(LanInterfaceHighlight::new(i, "Controller"));
                    }
                    if this_sys.type_ == LANSYSTEM_ISOLATIONBRIDGE
                        && (this_sys.data1 == system_index || this_sys.data2 == system_index)
                    {
                        highlights.push(LanInterfaceHighlight::new(i, "Controller"));
                    }
                }
            }
        }
        LANSYSTEM_MAINSERVER => {
            if system.data1 != -1 {
                highlights.push(LanInterfaceHighlight::new(system.data1, "GuardedBy"));
            }
            if system.data2 != -1 {
                highlights.push(LanInterfaceHighlight::new(system.data2, "GuardedBy"));
            }
            if system.data3 != -1 {
                highlights.push(LanInterfaceHighlight::new(system.data3, "GuardedBy"));
            }
        }
        _ => {}
    }

    for subnet in &system.valid_subnets {
        highlights.push(LanInterfaceHighlight::new(*subnet, "Valid Subnet"));
    }
    highlights
}

impl LanInterface {
    /// C++ `LanInterface::LanInterface()`: the viewed IP starts as "None" and the stipple at its first pattern.
    pub fn new() -> Self {
        LanInterface { ip: "None".to_string(), stipple_pattern: STIPPLE[0], ..Default::default() }
    }

    /// C++ `LanInterface::TitleClick` (static button callback; also the Close button).
    pub fn title_click(g: &mut Ctx, _button: &Button) {
        LocalInterface::run_screen(g, SCREEN_NONE, -1);
    }

    /// C++ `LanInterface::TitleDraw` (static draw callback).
    pub fn title_draw(g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        set_colour(g, "TitleText");
        text::draw_text_styled(button.x + 10, button.y + 10, &button.caption, text::HELVETICA_18);
    }

    /// C++ `LanInterface::PanelBackgroundDraw` (static draw callback).
    pub fn panel_background_draw(g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        gl::color4f(0.0, 0.0, 0.0, 1.0);
        gl::begin(gl::QUADS);
        gl::vertex2i(button.x, button.y + button.height);
        gl::vertex2i(button.x, button.y);
        gl::vertex2i(button.x + button.width, button.y);
        gl::vertex2i(button.x + button.width, button.y + button.height);
        gl::end();

        set_colour(g, "PanelBorder");
        border_draw(button);
    }

    /// C++ `LanInterface::LanSystemDraw` (static draw callback for a system button).
    pub fn lan_system_draw(g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        // Only draw within the LAN background box
        let (bx, by, bw, bh) = background_rect(g);
        gl::scissor(bx, screen_h() - (by + bh), bw - 2, bh);
        gl::enable(gl::SCISSOR_TEST);

        let Some(visible) = system_visibility(g, name_number(&button.name)) else { return };

        // Draw it
        match visible {
            LANSYSTEMVISIBLE_NONE => {
                clear_draw(g, button.x, button.y, button.width, button.height);
            }
            LANSYSTEMVISIBLE_AWARE => {
                clear_draw(g, button.x, button.y, button.width, button.height);
                gl::color3ub(187, 207, 247);
                border_draw(button);
            }
            LANSYSTEMVISIBLE_TYPE | LANSYSTEMVISIBLE_FULL => {
                let image = button.image_standard.as_ref().expect("UplinkAssert(button->image_standard)");
                image.draw(button.x, button.y);
            }
            _ => {}
        }

        // Un scissor
        gl::disable(gl::SCISSOR_TEST);
    }

    /// C++ `LanInterface::LanSystemClick` (static button callback).
    pub fn lan_system_click(g: &mut Ctx, button: &Button) {
        let system_index = name_number(&button.name);

        with_lan(g, |lan, g| lan.select_system(g, system_index));

        // Make this system the target for task programs
        let Some(lan) = remote_lan(g) else { return };
        let target_visible = system_at(lan, system_index).map(|s| s.visible);
        let comp_name = remote_computer(g).name.clone();

        if let Some(visible) = target_visible {
            if visible != LANSYSTEMVISIBLE_NONE {
                TaskManager::set_program_target(g, ObjRef::Computer(comp_name), &button.name, system_index);
            }
        }

        g.ecl.dirty_button("localint_background");
    }

    /// C++ `LanInterface::LanSystemMouseMove` (static mouse-move callback).
    pub fn lan_system_mouse_move(g: &mut Ctx, button: &Button) {
        let Some(visible) = system_visibility(g, name_number(&button.name)) else { return };

        let tip = match visible {
            LANSYSTEMVISIBLE_AWARE => "Target this unknown system",
            LANSYSTEMVISIBLE_TYPE | LANSYSTEMVISIBLE_FULL => "Target this system",
            _ => return,
        };

        // C++ `button->SetTooltip(...)`: the registered button is the one updated.
        if let Some(b) = g.ecl.get_button_mut(&button.name) {
            b.set_tooltip(tip);
        }
        button_highlight(g, button);
    }

    /// C++ `LanInterface::LanBackgroundDraw` (static draw callback).
    pub fn lan_background_draw(g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        let screenheight = screen_h();
        gl::scissor(button.x, screenheight - (button.y + button.height), button.width, button.height);
        gl::enable(gl::SCISSOR_TEST);

        clear_draw(g, button.x, button.y, button.width, button.height);
        gl::color3ub(81, 138, 215);
        border_draw(button);

        let bg = background_rect(g);

        // Lookup computer details
        {
            let Some(lan) = remote_lan(g) else {
                // "No Local Area Network (LAN) detected."
                let (bx, by, bw, bh) = bg;
                let message = "No Local Area Network (LAN) detected.";
                gl::color3f(1.0, 1.0, 1.0);
                text::draw_text_default(
                    (bx + bw / 2) - (text::text_width_default(message) / 2),
                    by + bh / 2,
                    message,
                );
                return;
            };
            draw_lan_contents(g, lan, bg);
        }

        // Draw sys Admin details
        let text_x = button.x + 20;
        let text_y = button.y + button.height - 15;
        clear_draw(g, text_x, text_y, 300, 14);
        draw_sysadmin_contents(g, bg, text_x, text_y);

        gl::disable(gl::SCISSOR_TEST);
    }

    /// C++ `LanInterface::LanBackgroundMouseMove` (static mouse-move callback).
    /// The C++ .cpp never defines this function (only the header declares it), so it does nothing.
    pub fn lan_background_mouse_move(_g: &mut Ctx, _button: &Button) {}

    /// C++ `LanInterface::ConnectDraw` (static draw callback).
    pub fn connect_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        if can_connect(g) {
            button_draw(g, button, highlighted, clicked);
        }
    }

    /// C++ `LanInterface::ConnectClick` (static button callback).
    pub fn connect_click(g: &mut Ctx, _button: &Button) {
        if !can_connect(g) {
            return;
        }
        let selected = lan_monitor(g).current_selected;
        if !lan_monitor(g).is_in_connection(selected) {
            return;
        }

        LanMonitor::set_current_system(g, selected);

        // Remove everything in the connection after the current Selected
        let mon = lan_monitor_mut(g);
        if let Some(i) = mon.connection.iter().rposition(|&n| n == selected) {
            mon.connection.truncate(i + 1);
        }

        // Show the screen attached to this system
        let screen = remote_lan(g)
            .and_then(|lan| system_at(lan, selected))
            .map(|s| s.screen_index);
        if let Some(screen) = screen.filter(|&s| s != -1) {
            RemoteInterface::run_screen(g, screen);
            LocalInterface::run_screen(g, SCREEN_NONE, -1);
        }
    }

    /// C++ `LanInterface::ConnectMouseMove` (static mouse-move callback).
    pub fn connect_mouse_move(g: &mut Ctx, button: &Button) {
        if can_connect(g) {
            button_highlight(g, button);
        }
    }

    /// C++ `LanInterface::CancelClick` (static button callback).
    pub fn cancel_click(g: &mut Ctx, _button: &Button) {
        LanMonitor::begin_attack(g);

        let current_system = lan_monitor(g).current_system;
        let Some(screen) = remote_lan(g)
            .and_then(|lan| lan.systems.get_data(current_system))
            .map(|s| s.screen_index)
        else {
            return;
        };

        RemoteInterface::run_screen(g, screen);
        LocalInterface::run_screen(g, SCREEN_NONE, -1);
        LocalInterface::run_screen(g, SCREEN_LAN, -1);
    }

    /// C++ `LanInterface::BackDraw` (static draw callback).
    pub fn back_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        if can_back(g) {
            button_draw(g, button, highlighted, clicked);
        }
    }

    /// C++ `LanInterface::BackMouseMove` (static mouse-move callback).
    pub fn back_mouse_move(g: &mut Ctx, button: &Button) {
        if can_back(g) {
            button_highlight(g, button);
        }
    }

    /// C++ `LanInterface::BackClick` (static button callback).
    pub fn back_click(g: &mut Ctx, _button: &Button) {
        lan_monitor_mut(g).retract_connection();
    }

    /// C++ `LanInterface::GenerateClick` (static button callback).
    pub fn generate_click(g: &mut Ctx, button: &Button) {
        let size = name_number(&button.name);

        if remote_lan(g).is_none() {
            return;
        }
        let comp_name = remote_computer(g).name.clone();

        LocalInterface::run_screen(g, SCREEN_NONE, -1);

        if let Some(lan) = g.world_mut().get_computer_mut(&comp_name).and_then(|c| c.lan_mut()) {
            lan.systems.empty();
            lan.links.empty();
        }

        // C++ switch: only sizes 0..4 generate a cluster.
        if (0..=4).contains(&size) {
            LanGenerator::generate_lan_cluster(g, &comp_name, size);
        }

        // PORT: C++ passes NULL here; the Rust signature needs a Button, and ShowLANClick does not
        // use its argument, so the generate button is passed instead.
        CheatInterface::show_lan_click(g, button);
        LanMonitor::reset_all(g);
        LocalInterface::run_screen(g, SCREEN_LAN, -1);
    }

    /// C++ `LanInterface::DrawLink` (static): draws one link between two systems.
    pub fn draw_link(_g: &mut Ctx, link: &LanComputerLink, from_x: f32, from_y: f32, to_x: f32, to_y: f32) {
        draw_link_lines(link, from_x, from_y, to_x, to_y);
    }

    /// C++ `LanInterface::ScrollClick` (static button callback).
    pub fn scroll_click(g: &mut Ctx, button: &Button) {
        let direction = name_number(&button.name);
        let scroll_speed = 100;

        with_lan(g, |lan, g| {
            match direction {
                1 => lan.offset_y += scroll_speed,
                2 => lan.offset_y -= scroll_speed,
                3 => lan.offset_x += scroll_speed,
                4 => lan.offset_x -= scroll_speed,
                _ => {}
            }
            lan.position_layout(g);
        });
    }

    /// C++ file-level `CycleStipplePattern()`: advances the animated stipple used for link lines.
    pub fn cycle_stipple_pattern(&mut self) {
        self.stipple_index -= 1;
        if self.stipple_index < 0 {
            self.stipple_index = 7;
        }
        self.stipple_pattern = STIPPLE[self.stipple_index as usize];
    }

    /// C++ `LanInterface::CreateLayout`: registers the buttons for the current LAN.
    pub fn create_layout(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            return;
        }
        let Some(lan) = remote_lan(g) else { return };
        let spots = system_spots(lan);
        let centre = system_at(lan, lan_monitor(g).current_system).map(|s| (s.x, s.y));
        let (bx, by, bw, bh) = background_rect(g);

        // Create icons for each system
        for spot in &spots {
            let name = format!("lansystem {}", spot.index);
            g.ecl.register_button(
                bx + spot.x,
                by + spot.y,
                spot.obj.width,
                spot.obj.height,
                " ",
                "Access this system",
                &name,
            );
            button_assignbitmap(g, &name, spot.obj.filename);
            g.ecl.register_button_callbacks(
                &name,
                Some(Self::lan_system_draw),
                Some(Self::lan_system_click),
                Some(button_click),
                Some(Self::lan_system_mouse_move),
            );
        }

        // Centre on the current system
        if let Some((cx, cy)) = centre {
            self.offset_x = (bw / 2) - cx;
            self.offset_y = (bh / 2) - cy;
        }

        self.position_layout(g);

        // Make sure scroll buttons are still on top
        for name in ["lan_scroll 1", "lan_scroll 2", "lan_scroll 3", "lan_scroll 4"] {
            g.ecl.button_bring_to_front(name);
        }
    }

    /// C++ `LanInterface::RemoveLayout`: removes the buttons registered by `create_layout`.
    pub fn remove_layout(&mut self, g: &mut Ctx) {
        let mut i = 0;
        loop {
            let name = format!("lansystem {}", i);
            if g.ecl.get_button(&name).is_none() {
                break;
            }
            g.ecl.remove_button(&name);
            i += 1;
        }
    }

    /// C++ `LanInterface::PositionLayout`: moves all buttons to match the pan offset.
    pub fn position_layout(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            return;
        }
        let Some(lan) = remote_lan(g) else { return };
        let spots = system_spots(lan);
        let (bx, by, _, _) = background_rect(g);

        for spot in &spots {
            let name = format!("lansystem {}", spot.index);
            let x = bx + spot.x + self.offset_x;
            let y = by + spot.y + self.offset_y;
            if let Some(button) = g.ecl.get_button_mut(&name) {
                button.x = x;
                button.y = y;
            }
            g.ecl.dirty_button(&name);
        }

        g.ecl.dirty_button("localint_background");
    }

    /// C++ `LanInterface::SelectSystem`: selects a system and rebuilds its highlights.
    pub fn select_system(&mut self, g: &mut Ctx, system_index: i32) {
        let Some(lan) = remote_lan(g) else { return };
        let Some(system) = system_at(lan, system_index) else { return };
        if system.visible == LANSYSTEMVISIBLE_NONE {
            return;
        }
        let visible = system.visible;
        let obj = get_lan_interface_object(system.type_);
        let highlights = build_highlights(lan, system_index, system);

        // Update the connection
        if visible > LANSYSTEMVISIBLE_AWARE {
            lan_monitor_mut(g).set_current_selected(system_index);

            if !lan_monitor(g).is_in_connection(system_index) {
                LanMonitor::extend_connection(g, system_index);
            }
        }

        // Update the side panel
        let screenw = screen_w();
        let paneltop = sy(100) + 30;

        g.ecl.remove_button("lan_preview");
        g.ecl.register_button(
            screenw - 90,
            paneltop + 60,
            obj.width,
            obj.height,
            " ",
            "Shows the currently selected system",
            "lan_preview",
        );
        button_assignbitmap(g, "lan_preview", obj.filename);

        g.ecl.register_caption_change_auto("lan_systemname", obj.name, None);
        g.ecl.register_caption_change("lan_systemdesc", obj.description, 2000, None);

        // Update the highlights
        self.highlights = highlights;
    }
}

impl InterfaceScreen for LanInterface {
    /// C++ `LanInterface::Create`
    fn create(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            return;
        }

        let screenw = screen_w();
        let paneltop = sy(100) + 30;
        let panelwidth = (screenw as f64 * PANELSIZE) as i32;

        // Create the side panel
        g.ecl.register_button(screenw - panelwidth - 3, paneltop, panelwidth, sy(300), "", "", "localint_background");
        g.ecl.register_button_callbacks("localint_background", Some(Self::panel_background_draw), None, None, None);

        g.ecl.register_button(screenw - panelwidth, paneltop + 3, panelwidth - 7, 15, "LOCAL AREA NETWORK", "Remove the LAN screen", "lan_title");
        g.ecl.register_button_callback("lan_title", Self::title_click);

        g.ecl.register_button(screenw - panelwidth + 10, paneltop + 30, panelwidth - 10, 30, " ", " ", "lan_systemname");
        g.ecl.register_button_callbacks("lan_systemname", Some(Self::title_draw), None, None, None);

        g.ecl.register_button(screenw - 90, paneltop + 60, 32, 32, " ", " ", "lan_preview");
        g.ecl.register_button_callbacks("lan_preview", None, None, None, None);

        let info_width = (screenw - 90) - (screenw - panelwidth) - 10;
        g.ecl.register_button(screenw - panelwidth + 10, paneltop + 60, info_width, 200, " ", " ", "lan_systemdesc");
        g.ecl.register_button_callbacks("lan_systemdesc", Some(textbutton_draw), None, None, None);

        g.ecl.register_button(screenw - panelwidth, paneltop + sy(300) - 60, panelwidth - 7, 15, "Back", "Remove the last system from your connection", "lan_back");
        g.ecl.register_button_callbacks("lan_back", Some(Self::back_draw), Some(Self::back_click), Some(button_click), Some(Self::back_mouse_move));

        g.ecl.register_button(screenw - panelwidth, paneltop + sy(300) - 40, panelwidth - 7, 15, "Reset", "Reset the current LAN connection", "lan_cancel");
        g.ecl.register_button_callbacks("lan_cancel", Some(Self::back_draw), Some(Self::cancel_click), Some(button_click), Some(Self::back_mouse_move));

        g.ecl.register_button(screenw - panelwidth, paneltop + sy(300) - 20, panelwidth - 7, 15, "Connect", "Connect to the currently selected system", "lan_connect");
        g.ecl.register_button_callbacks("lan_connect", Some(Self::connect_draw), Some(Self::connect_click), Some(button_click), Some(Self::connect_mouse_move));

        // Create the main box
        g.ecl.register_button(5, 30, sx(440), sy(370), " ", " ", "lan_background");
        g.ecl.register_button_callbacks("lan_background", Some(Self::lan_background_draw), None, None, None);

        self.ip = g.player().remotehost.clone();
        self.stipple_pattern = STIPPLE[self.stipple_index as usize];

        if globals::TESTGAME || globals::cheatmodes_enabled() {
            for level in 0..5 {
                let name = format!("lan_generate {}", level);
                g.ecl.register_button(
                    10 + 20 * level,
                    35,
                    15,
                    15,
                    &format!("L{}", level),
                    &format!("Generate a new Level {} Gateway", level),
                    &name,
                );
                g.ecl.register_button_callback(&name, Self::generate_click);
            }
        }

        g.ecl.register_button(5 + sx(440) - 15, 30, 15, 15, "^", "Scroll up", "lan_scroll 1");
        g.ecl.register_button(5 + sx(440) - 15, 30 + sy(370) - 30, 15, 15, "v", "Scroll down", "lan_scroll 2");
        g.ecl.register_button(5, 30 + sy(370) - 15, 15, 15, "<", "Scroll left", "lan_scroll 3");
        g.ecl.register_button(5 + sx(440) - 30, 30 + sy(370) - 15, 15, 15, ">", "Scroll right", "lan_scroll 4");

        for name in ["lan_scroll 1", "lan_scroll 2", "lan_scroll 3", "lan_scroll 4"] {
            g.ecl.register_button_callback(name, Self::scroll_click);
        }

        self.create_layout(g);
        let current_system = lan_monitor(g).current_system;
        self.select_system(g, current_system);
    }

    /// C++ `LanInterface::Remove`
    fn remove(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            return;
        }

        for name in [
            "localint_background",
            "lan_title",
            "lan_preview",
            "lan_systemname",
            "lan_systemdesc",
            "lan_back",
            "lan_cancel",
            "lan_connect",
            "lan_background",
        ] {
            g.ecl.remove_button(name);
        }

        if globals::TESTGAME || globals::cheatmodes_enabled() {
            for level in 0..5 {
                g.ecl.remove_button(&format!("lan_generate {}", level));
            }
        }

        for n in 1..=4 {
            g.ecl.remove_button(&format!("lan_scroll {}", n));
        }

        self.remove_layout(g);
    }

    /// C++ `LanInterface::Update`
    fn update(&mut self, g: &mut Ctx) {
        // Shut down if we aren't looking at our LAN anymore
        if self.ip != g.player().remotehost {
            LocalInterface::run_screen(g, SCREEN_NONE, -1);
            return;
        }

        // Update the stipple patterns
        // AT A FIXED RATE ;)
        if g.ecl.get_accurate_time() > self.stipple_update {
            self.cycle_stipple_pattern();
            g.ecl.dirty_button("lan_background");
            self.stipple_update = g.ecl.get_accurate_time() + 20;
        }
    }

    /// C++ `LanInterface::IsVisible`
    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("lan_background").is_some()
    }

    fn screen_id(&self) -> i32 {
        SCREEN_LAN
    }

    impl_as_any!();
}
