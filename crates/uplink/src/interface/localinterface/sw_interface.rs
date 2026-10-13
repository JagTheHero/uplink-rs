//! Port of source/interface/localinterface/sw_interface.h + source/interface/localinterface/sw_interface.cpp
//! The "Start" button and the software menus that come off it: sub-menus per software type,
//! and the version list for programs with more than one installed version.

use uplink_engine::gl;
use uplink_engine::sound;
use uplink_engine::text::draw_text_default;

use crate::app::globals::ALPHA;
use crate::app::opengl_interface::{button_click, button_draw, button_highlight, getcurrentbutton, set_colour};
use crate::ctx::Ctx;
use crate::eclipse::{Button, MouseFn, MOVE_STRAIGHTLINE};
use crate::impl_as_any;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::localinterface::hud_interface::HUDInterface;
use crate::interface::taskmanager::taskmanager::TaskManager;
use crate::world::computer::databank::{DATATYPE_PROGRAM, SOFTWARETYPE_NONE};

#[derive(Debug, Clone)]
pub struct SWInterface {
    /// C++ `static int currentsubmenu`: the software type whose sub-menu is open (SOFTWARETYPE_NONE = none).
    // PORT: static -> field. In C++ it is the `si` member of HUDInterface, so it is reached through
    // the HUD screen (`li.hud` -> `HUDInterface.si`), not through a field of the local interface.
    pub currentsubmenu: i32,
    /// C++ `static int currentprogrambutton`: index of the highlighted `hud_software N` button (-1 = none).
    pub currentprogrambutton: i32,
    /// C++ `static int timesync`: accurate time when the highlight last changed, for the version-menu delay (-1 = unset).
    pub timesync: i32,
}

impl Default for SWInterface {
    fn default() -> Self {
        SWInterface {
            currentsubmenu: SOFTWARETYPE_NONE,
            currentprogrambutton: -1,
            timesync: -1,
        }
    }
}

/// The start menu entries: (software type, label, tooltip, row multiplier for the slide-in movement).
const START_MENU: [(i32, &str, &str, i32); 7] = [
    (1, "File Utilities", "Eg file copiers, deleters, encrypters etc", 6),
    (2, "Hardware Drivers", "Eg gateway cameras", 5),
    (3, "Security", "Eg log deleters, Trace trackers etc", 4),
    (4, "Crackers", "Eg password breakers, decrypters etc", 3),
    (5, "Bypassers", "Eg proxy bypass, firewall bypass etc", 2),
    (6, "LAN tools", "Eg LAN Scan, LAN probe etc", 1),
    (10, "Other", "Other software programs", 0),
];

/// A program in the player's databank (a `DATATYPE_PROGRAM` entry).
struct Program {
    title: String,
    version: f32,
    software_type: i32,
}

/// The programs in the player's gateway databank, in databank order.
fn programs(g: &Ctx) -> Vec<Program> {
    let db = &g.player().gateway.databank;
    (0..db.get_data_size())
        .filter_map(|i| db.get_data_file(i))
        .filter(|d| d.type_ == DATATYPE_PROGRAM)
        .map(|d| Program {
            title: d.title.clone(),
            version: d.version,
            software_type: d.software_type,
        })
        .collect()
}

/// `sscanf(caption, "%s v%f", name, &version)`. A missing version gives 0.0 (C leaves it uninitialised).
fn scan_name_version(caption: &str) -> (String, f32) {
    let trimmed = caption.trim_start();
    let name_end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
    let name = trimmed[..name_end].to_string();
    let rest = trimmed[name_end..].trim_start();
    let version = rest.strip_prefix('v').and_then(parse_float_prefix).unwrap_or(0.0);
    (name, version)
}

/// The longest leading decimal number in `s`, as `%f` reads it.
fn parse_float_prefix(s: &str) -> Option<f32> {
    let bytes = s.as_bytes();
    let mut end = 0;
    if end < bytes.len() && (bytes[end] == b'+' || bytes[end] == b'-') {
        end += 1;
    }
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    if end < bytes.len() && bytes[end] == b'.' {
        end += 1;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
    }
    s[..end].parse::<f32>().ok()
}

/// `sscanf(name, "%s %d", junk, &index)` on names like `hud_software 3`. Missing index gives -1.
fn scan_index(name: &str) -> i32 {
    let rest = name
        .trim_start()
        .split_once(char::is_whitespace)
        .map(|(_, r)| r.trim_start())
        .unwrap_or("");
    let end = rest
        .char_indices()
        .find(|&(i, c)| !(c.is_ascii_digit() || (i == 0 && (c == '-' || c == '+'))))
        .map_or(rest.len(), |(i, _)| i);
    rest[..end].parse().unwrap_or(-1)
}

/// C++ `IsVisibleSubMenu` against an explicit state, for use while the state is taken out of the HUD.
fn sub_menu_visible(g: &Ctx, sw: &SWInterface, software_type: i32) -> bool {
    g.ecl.get_button("hud_software 0").is_some() && sw.currentsubmenu == software_type
}

/// Takes the software menu state out of the HUD so the menu code can also use the rest of `g`.
/// None while the HUD itself is busy (taken out of its slot), in which case the change is skipped.
fn take_sw(g: &mut Ctx) -> Option<SWInterface> {
    g.interface_mut()
        .li
        .hud
        .get_mut::<HUDInterface>()
        .map(|hud| std::mem::take(&mut hud.si))
}

/// Puts the software menu state back into the HUD, after [`take_sw`].
fn put_sw(g: &mut Ctx, sw: SWInterface) {
    if let Some(hud) = g.interface_mut().li.hud.get_mut::<HUDInterface>() {
        hud.si = sw;
    }
}

/// Copy of the software menu state. Falls back to the defaults while the HUD is busy.
fn sw_snapshot(g: &Ctx) -> SWInterface {
    g.interface()
        .li
        .hud
        .get::<HUDInterface>()
        .map(|hud| hud.si.clone())
        .unwrap_or_default()
}

impl SWInterface {
    /// C++ `SWInterface::SoftwareClick` (static button callback): runs the program if it is in the gateway databank.
    pub fn software_click(g: &mut Ctx, button: &Button) {
        let (softwarename, version) = scan_name_version(&button.caption);

        // Check this is still in memory
        let db = &g.player().gateway.databank;
        let inmemory = (0..db.get_data_size())
            .filter_map(|i| db.get_data_file(i))
            .any(|d| d.title == softwarename && d.version == version);

        // Run it if it is
        if inmemory {
            TaskManager::run_software(g, &softwarename, version);
            sound::play_sound("sounds/runsoftware.wav", None, false);
        }

        // Remove the software menu
        Self::toggle_software_menu(g);
    }

    /// C++ `SWInterface::SoftwareDraw` (static draw callback).
    pub fn software_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        // Highlight this button anyway if it is the currentprogrambutton
        let areyoutheone = scan_index(&button.name);
        let nhighlighted = if areyoutheone == sw_snapshot(g).currentprogrambutton
            && !button.name.contains("version")
        {
            // Don't auto-highlight buttons from a version menu
            true
        } else {
            highlighted
        };

        // Draw the button
        let (col_a, col_b) = if clicked {
            ("ButtonClickedA", "ButtonClickedB")
        } else if highlighted {
            ("ButtonHighlightedA", "ButtonHighlightedB")
        } else {
            ("ButtonNormalA", "ButtonNormalB")
        };

        gl::begin(gl::QUADS);
        set_colour(g, col_a);
        gl::vertex2i(button.x, button.y + button.height);
        set_colour(g, col_b);
        gl::vertex2i(button.x, button.y);
        set_colour(g, col_a);
        gl::vertex2i(button.x + button.width, button.y);
        set_colour(g, col_b);
        gl::vertex2i(button.x + button.width, button.y + button.height);
        gl::end();

        let (softwarename, _) = scan_name_version(&button.caption);

        if nhighlighted || clicked {
            gl::color4f(1.0, 1.0, 1.0, 1.0);
        } else {
            gl::color4f(1.0, 1.0, 1.0, ALPHA);
        }

        let text_y = button.y + button.height / 2 + 3;

        // Write the software name
        draw_text_default(button.x + 5, text_y, &softwarename);

        // Write the version number (the rest of the caption after the name)
        let version_text = button.caption.get(softwarename.len()..).unwrap_or("");
        draw_text_default(button.x + button.width - 30, text_y, version_text);
    }

    /// C++ `SWInterface::SoftwareHighlight` (static mouse-move callback).
    pub fn software_highlight(g: &mut Ctx, button: &Button) {
        // Don't do anything immediately, but set some timer events up so that a list of all versions
        // of this software item appear in a new sub menu in a few milliseconds
        let index = scan_index(&button.name);

        if let Some(mut sw) = take_sw(g) {
            if sw.currentprogrambutton != index {
                // Remove any old version menus
                if is_visible_version_menu(g, None) {
                    toggle_version_menu_in(&mut sw, g, None, 0, 0);
                } else {
                    // Force redraw of previously selected button
                    g.ecl.dirty_button(&format!("hud_software {}", sw.currentprogrambutton));
                }

                sw.currentprogrambutton = index;
                sw.timesync = g.ecl.get_accurate_time();
            }
            put_sw(g, sw);
        }

        button_highlight(g, button);
    }

    /// C++ `SWInterface::HasSoftwareType` (static): does the gateway databank hold a program of this type?
    pub fn has_software_type(g: &Ctx, software_type: i32) -> bool {
        programs(g).iter().any(|p| p.software_type == software_type)
    }

    /// C++ `SWInterface::ToggleSubMenu` (static): opens or closes the sub-menu for `software_type` at (x, y), the bottom-left corner.
    pub fn toggle_sub_menu(g: &mut Ctx, software_type: i32, x: i32, y: i32) {
        if let Some(mut sw) = take_sw(g) {
            toggle_sub_menu_in(&mut sw, g, software_type, x, y);
            put_sw(g, sw);
        }
    }

    /// C++ `SWInterface::IsVisibleSubMenu` (static).
    pub fn is_visible_sub_menu(g: &Ctx, software_type: i32) -> bool {
        g.ecl.get_button("hud_software 0").is_some()
            && g.interface()
                .li
                .hud
                .get::<HUDInterface>()
                .is_some_and(|h| h.si.currentsubmenu == software_type)
    }

    /// C++ `SWInterface::HasVersionMenu` (static): true if the databank holds more than one version of `program`.
    pub fn has_version_menu(g: &Ctx, program: &str) -> bool {
        programs(g).iter().filter(|p| p.title == program).count() > 1
    }

    /// C++ `SWInterface::ToggleVersionMenu` (static). `program` is None when closing.
    pub fn toggle_version_menu(g: &mut Ctx, program: Option<&str>, x: i32, y: i32) {
        if let Some(mut sw) = take_sw(g) {
            toggle_version_menu_in(&mut sw, g, program, x, y);
            put_sw(g, sw);
        }
    }

    /// C++ `SWInterface::IsVisibleVersionMenu` (static). With `program`, also checks that the open menu belongs to it.
    pub fn is_visible_version_menu(g: &Ctx, program: Option<&str>) -> bool {
        is_visible_version_menu(g, program)
    }

    /// C++ `SWInterface::StartMenuItemDraw` (static draw callback for the `hud_swmenu N` buttons).
    pub fn start_menu_item_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        let software_type = scan_index(&button.name);

        if software_type == sw_snapshot(g).currentsubmenu {
            button_draw(g, button, true, clicked);
        } else {
            button_draw(g, button, highlighted, clicked);
        }
    }

    /// C++ `SWInterface::StartMenuItemHighlight` (static mouse-move callback).
    pub fn start_menu_item_highlight(g: &mut Ctx, button: &Button) {
        let software_type = scan_index(&button.name);

        if let Some(mut sw) = take_sw(g) {
            // Create this menu if it doesn't exist
            if !sub_menu_visible(g, &sw, software_type) {
                // Remove the old sub menu
                if is_visible_version_menu(g, None) {
                    toggle_version_menu_in(&mut sw, g, None, 0, 0);
                }

                let current = sw.currentsubmenu;
                if sub_menu_visible(g, &sw, current) {
                    toggle_sub_menu_in(&mut sw, g, current, 0, 0);
                }

                // Create the new sub menu
                toggle_sub_menu_in(&mut sw, g, software_type, button.x + button.width + 2, button.y);
            }

            // Stop any version lists from appearing
            sw.currentprogrambutton = -1;
            put_sw(g, sw);
        }

        button_highlight(g, button);
    }

    /// C++ `SWInterface::StartMenuItemDrawUnavailable` (static draw callback for types with no software).
    pub fn start_menu_item_draw_unavailable(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        // For now ;)
        button_draw(g, button, highlighted, clicked);
    }

    /// C++ `SWInterface::StartMenuItemHighlightUnavailable` (static mouse-move callback).
    pub fn start_menu_item_highlight_unavailable(g: &mut Ctx, button: &Button) {
        if let Some(mut sw) = take_sw(g) {
            // Remove the old sub menus
            if is_visible_version_menu(g, None) {
                toggle_version_menu_in(&mut sw, g, None, 0, 0);
            }

            let current = sw.currentsubmenu;
            if sub_menu_visible(g, &sw, current) {
                toggle_sub_menu_in(&mut sw, g, current, 0, 0);
            }
            put_sw(g, sw);
        }

        button_highlight(g, button);
    }

    /// C++ `SWInterface::ToggleSoftwareMenu` (static): opens or closes the Start menu.
    pub fn toggle_software_menu(g: &mut Ctx) {
        if let Some(mut sw) = take_sw(g) {
            toggle_software_menu_in(&mut sw, g);
            put_sw(g, sw);
        }
    }

    /// C++ `SWInterface::IsVisibleSoftwareMenu` (static).
    pub fn is_visible_software_menu(g: &Ctx) -> bool {
        g.ecl.get_button("hud_swmenu 1").is_some()
    }
}

/// C++ `SWInterface::ToggleSubMenu` body, on an explicit state.
fn toggle_sub_menu_in(sw: &mut SWInterface, g: &mut Ctx, software_type: i32, x: i32, y: i32) {
    if !sub_menu_visible(g, sw, software_type) {
        // Create a list of all software in memory, keeping the highest version of each name
        let mut software: Vec<(String, f32)> = Vec::new();
        for p in programs(g).into_iter().filter(|p| p.software_type == software_type) {
            match software.iter().position(|(title, _)| *title == p.title) {
                None => software.push((p.title, p.version)),
                Some(existing) if p.version > software[existing].1 => {
                    software[existing] = (p.title, p.version);
                }
                Some(_) => {}
            }
        }

        let timesplit = 500 / software.len() as i32; // Ensures total time = 500ms

        for (si, (title, version)) in software.iter().enumerate() {
            let si = si as i32;
            let caption = format!("{title} v{version:.1}");
            let name = format!("hud_software {si}");
            g.ecl.register_button(x, y, 140, 15, &caption, "Runs this software application", &name);
            g.ecl.register_button_callbacks(
                &name,
                Some(SWInterface::software_draw),
                Some(SWInterface::software_click),
                Some(button_click),
                Some(SWInterface::software_highlight),
            );
            g.ecl.register_movement(&name, x, y - si * 17, si * timesplit, MOVE_STRAIGHTLINE, None);
        }

        sw.currentsubmenu = software_type;

        sound::play_sound("sounds/softwaremenu.wav", None, false);
    } else {
        // Remove the software menu
        // We don't know how many entries there could be, so keep deleting until they run out
        let mut i = 0;
        while g.ecl.get_button(&format!("hud_software {i}")).is_some() {
            g.ecl.remove_button(&format!("hud_software {i}"));
            i += 1;
        }

        // Redraw the button that was selected
        let current = sw.currentsubmenu;
        g.ecl.dirty_button(&format!("hud_swmenu {current}"));

        sw.currentsubmenu = SOFTWARETYPE_NONE;
    }
}

/// C++ `SWInterface::ToggleVersionMenu` body, on an explicit state.
fn toggle_version_menu_in(sw: &mut SWInterface, g: &mut Ctx, program: Option<&str>, x: i32, y: i32) {
    // Only opens when a program is named and its menu is not already showing
    let to_create = match program {
        Some(p) if !is_visible_version_menu(g, Some(p)) => Some(p),
        _ => None,
    };

    if let Some(program) = to_create {
        // Create it
        let software: Vec<(String, f32)> = programs(g)
            .into_iter()
            .filter(|p| p.title == program)
            .map(|p| (p.title, p.version))
            .collect();

        let timesplit = 500 / software.len() as i32; // Ensures total time = 500ms

        for (si, (title, version)) in software.iter().enumerate() {
            let si = si as i32;
            let caption = format!("{title} v{version:.1}");
            let name = format!("hud_version {si}");
            g.ecl.register_button(x, y, 120, 15, &caption, "Runs this software application", &name);
            g.ecl.register_button_callbacks(
                &name,
                Some(SWInterface::software_draw),
                Some(SWInterface::software_click),
                Some(button_click),
                Some(button_highlight),
            );
            g.ecl.register_movement(&name, x, y - si * 17, si * timesplit, MOVE_STRAIGHTLINE, None);
        }

        sound::play_sound("sounds/softwaremenu.wav", None, false);
    } else {
        // Remove it
        // We don't know how many entries there could be, so keep deleting until they run out
        let mut i = 0;
        while g.ecl.get_button(&format!("hud_version {i}")).is_some() {
            g.ecl.remove_button(&format!("hud_version {i}"));
            i += 1;
        }

        // Force redraw of the button that fathered this menu
        g.ecl.dirty_button(&format!("hud_software {}", sw.currentprogrambutton));

        sw.currentprogrambutton = -1;
    }
}

/// C++ `SWInterface::IsVisibleVersionMenu` body. Reads only the buttons, so it needs no state.
fn is_visible_version_menu(g: &Ctx, program: Option<&str>) -> bool {
    match (g.ecl.get_button("hud_version 0"), program) {
        (Some(button), Some(program)) => button.caption.contains(program),
        (Some(_), None) => true,
        (None, _) => false,
    }
}

/// C++ `SWInterface::ToggleSoftwareMenu` body, on an explicit state.
fn toggle_software_menu_in(sw: &mut SWInterface, g: &mut Ctx) {
    if !SWInterface::is_visible_software_menu(g) {
        // Create the "start" menu
        let screen_h = g.app.options.get_option_value("graphics_screenheight");
        let y_pos = screen_h - 90;

        for &(t, label, tooltip, _) in START_MENU.iter() {
            g.ecl.register_button(3, y_pos, 100, 15, label, tooltip, &format!("hud_swmenu {t}"));
        }

        for &(t, _, _, _) in START_MENU.iter() {
            let name = format!("hud_swmenu {t}");
            if SWInterface::has_software_type(g, t) {
                // Type 1 has no mousedown callback, as in the original
                let mousedown: Option<MouseFn> = if t == 1 { None } else { Some(button_click) };
                g.ecl.register_button_callbacks(
                    &name,
                    Some(SWInterface::start_menu_item_draw),
                    None,
                    mousedown,
                    Some(SWInterface::start_menu_item_highlight),
                );
            } else {
                g.ecl.register_button_callbacks(
                    &name,
                    Some(SWInterface::start_menu_item_draw_unavailable),
                    None,
                    None,
                    Some(SWInterface::start_menu_item_highlight_unavailable),
                );
            }
        }

        let timesplit: i32 = 500 / 6; // Ensures total time = 500ms

        for &(t, _, _, mult) in START_MENU.iter() {
            g.ecl.register_movement(
                &format!("hud_swmenu {t}"),
                3,
                y_pos - mult * 17,
                mult * timesplit,
                MOVE_STRAIGHTLINE,
                None,
            );
        }

        sound::play_sound("sounds/softwaremenu.wav", None, false);
    } else {
        // Remove the old sub menus
        if is_visible_version_menu(g, None) {
            toggle_version_menu_in(sw, g, None, 0, 0);
        }

        let current = sw.currentsubmenu;
        if sub_menu_visible(g, sw, current) {
            toggle_sub_menu_in(sw, g, current, 0, 0);
        }

        for &(t, _, _, _) in START_MENU.iter() {
            g.ecl.remove_button(&format!("hud_swmenu {t}"));
        }
    }
}

impl InterfaceScreen for SWInterface {
    /// C++ `SWInterface::Create`: opens the Start menu if it is not already open.
    fn create(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            toggle_software_menu_in(self, g);
        }
    }

    /// C++ `SWInterface::Remove`: closes the Start menu if it is open.
    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            toggle_software_menu_in(self, g);
        }
    }

    /// C++ `SWInterface::Update`: after the highlight has settled for 1 s, opens the version menu for it.
    fn update(&mut self, g: &mut Ctx) {
        // If the user has his mouse over a software button and a few ms passes,
        // then bring up a list of all software items of the same name but all versions
        if self.currentprogrambutton != -1
            && self.timesync != -1
            && g.ecl.get_accurate_time() - self.timesync > 1000
        {
            // Check that the mouse is still over the button in question
            let correctname = format!("hud_software {}", self.currentprogrambutton);

            let currentbutton = getcurrentbutton(g).cloned();

            if let Some(currentbutton) = currentbutton {
                if currentbutton.name == correctname {
                    // Get the name of the software item
                    let (swname, _) = scan_name_version(&currentbutton.caption);

                    // Create a version menu if necessary
                    if SWInterface::has_version_menu(g, &swname)
                        && !is_visible_version_menu(g, Some(&swname))
                    {
                        toggle_version_menu_in(
                            self,
                            g,
                            Some(&swname),
                            currentbutton.x + currentbutton.width + 2,
                            currentbutton.y,
                        );
                    }
                }
            }

            // Stop it from updating needlessly
            self.timesync = -1;
        }
    }

    /// C++ `SWInterface::IsVisible`.
    fn is_visible(&self, g: &Ctx) -> bool {
        SWInterface::is_visible_software_menu(g)
    }

    impl_as_any!();
}
