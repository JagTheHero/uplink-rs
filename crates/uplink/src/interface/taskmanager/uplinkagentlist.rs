//! Port of source/interface/taskmanager/uplinkagentlist.h + source/interface/taskmanager/uplinkagentlist.cpp

use uplink_engine::gl;
use uplink_engine::text;

use crate::ctx::Ctx;
use crate::eclipse::{Button, MOVE_STRAIGHTLINE};
use crate::app::opengl_interface::{border_draw, button_assignbitmaps, button_click, button_draw, button_highlight};
use crate::game::scriptlibrary::ScriptLibrary;
use crate::impl_as_any;
use crate::interface::taskmanager::uplinktask::{UplinkTask, UplinkTaskBase};
use crate::objref::ObjRef;
use crate::tosser::DArray;
use crate::vanbakel::Vanbakel;

const NUMFILES: i32 = 10;

#[derive(Debug, Default)]
pub struct UplinkAgentList {
    pub base: UplinkTaskBase,
    pub handles: DArray<String>,
    pub names: DArray<String>,
    /// PORT: C++ `static int baseoffset`, kept as a field (only one list exists at a time).
    pub baseoffset: i32,
}

/// C `sscanf(token, "%d", &x)`: an optional sign and leading digits; the rest is ignored.
fn scan_leading_int(token: &str) -> Option<i32> {
    let end = token
        .char_indices()
        .find(|&(i, c)| !(c.is_ascii_digit() || (i == 0 && (c == '-' || c == '+'))))
        .map(|(i, _)| i)
        .unwrap_or(token.len());
    token[..end].parse::<i32>().ok()
}

/// C `sscanf(button->name, "%s %d", bname, &pid)`: the PID is the second token of the name.
fn button_pid(button: &Button) -> i32 {
    button.name.split_whitespace().nth(1).and_then(scan_leading_int).unwrap_or(-1)
}

/// The six buttons of the list, in C++ order: list, close, title, scroll up, scroll down, scroll bar.
fn button_names(pid: i32) -> [String; 6] {
    [
        format!("uplinkagentlist {pid}"),
        format!("uplinkagentlist_close {pid}"),
        format!("uplinkagentlist_title {pid}"),
        format!("uplinkagentlist_scrollup {pid}"),
        format!("uplinkagentlist_scrolldown {pid}"),
        format!("uplinkagentlist_scrollbar {pid}"),
    ]
}

impl UplinkAgentList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn uplink_agent_list_draw(g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        let pid = button_pid(button);
        let Some(ual) = g.svb.get_task::<UplinkAgentList>(pid) else { return };
        let baseoffset = ual.baseoffset;

        // Rows to draw: (row number, handle, name), for the 7 visible entries.
        let mut rows: Vec<(i32, String, String)> = Vec::new();
        for i in 0..7 {
            let index = i + baseoffset;
            if let (Some(handle), Some(name)) = (ual.handles.get_data(index), ual.names.get_data(index)) {
                rows.push((i, handle.clone(), name.clone()));
            }
        }

        //
        // Draw the background

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

        let screenheight = g.app.options.get_option_value("graphics_screenheight");
        gl::scissor(button.x, screenheight - (button.y + button.height), button.width, button.height);
        gl::enable(gl::SCISSOR_TEST);

        //
        // Draw the names

        for (i, handle, name) in rows {
            gl::color3f(1.0, 1.0, 1.0);
            text::draw_text_default(button.x + 10, button.y + 15 + i * 20, &handle);

            if name == "Unknown" {
                gl::color3f(0.5, 0.0, 0.0);
            } else if name == "Encrypted" {
                gl::color3f(0.8, 0.0, 0.0);
            } else {
                gl::color3f(1.0, 0.0, 0.0);
            }
            text::draw_text_default(button.x + 100, button.y + 15 + i * 20, &name);
        }

        gl::disable(gl::SCISSOR_TEST);
    }

    pub fn scroll_up_click(g: &mut Ctx, button: &Button) {
        let pid = button_pid(button);
        let Some(ual) = g.svb.get_task_mut::<UplinkAgentList>(pid) else { return };
        ual.baseoffset -= 1;
        if ual.baseoffset < 0 {
            ual.baseoffset = 0;
        }
        g.ecl.dirty_button(&format!("uplinkagentlist {pid}"));
    }

    pub fn scroll_down_click(g: &mut Ctx, button: &Button) {
        let pid = button_pid(button);
        let Some(ual) = g.svb.get_task_mut::<UplinkAgentList>(pid) else { return };
        ual.baseoffset += 1;
        g.ecl.dirty_button(&format!("uplinkagentlist {pid}"));
    }

    pub fn title_click(g: &mut Ctx, button: &Button) {
        let pid = button_pid(button);
        let Some(ual) = g.svb.get_task_mut::<UplinkAgentList>(pid) else { return };
        ual.base.followmouse = true;
        g.interface_mut().tm.set_target_program(pid);
    }

    pub fn close_click(g: &mut Ctx, button: &Button) {
        let pid = button_pid(button);
        Vanbakel::remove_task(g, pid);
    }

    pub fn update_lists(&mut self, g: &mut Ctx) {
        // Build a list of agents sorted in handle alphabetical order.
        // Entries are (handle, person name, is the player), in the pre-order of the C++ `people`
        // BTree; the player sits in that tree under "PLAYER".
        let candidates: Vec<(String, String, bool)> = g
            .world()
            .all_people()
            .into_iter()
            .filter(|(key, p)| *key == "PLAYER" || p.is_agent())
            .map(|(key, p)| (p.agent().handle.clone(), p.name.clone(), key == "PLAYER"))
            .collect();

        // Insert before the first entry whose handle is not less than this one (strcmp <= 0).
        let mut sorted: Vec<(String, String, bool)> = Vec::new();
        for cand in candidates {
            match sorted.iter().position(|s| cand.0.as_str() <= s.0.as_str()) {
                Some(pos) => sorted.insert(pos, cand),
                None => sorted.push(cand),
            }
        }

        //
        // How much data does the player have

        // 0 = not found, 1 = found, -1 = encrypted
        let mut filefound = [0i32; NUMFILES as usize];
        let datatitle = "Uplink_Agent_Data";

        let databank = &g.world().get_player().gateway.databank;
        for i in 0..databank.get_data_size() {
            if let Some(thisfile) = databank.get_data_file(i) {
                if thisfile.title.contains(datatitle) {
                    // C: sscanf(title, "%s %d.dat", unused, &thisfileindex)
                    let thisfileindex = thisfile.title.split_whitespace().nth(1).and_then(scan_leading_int);
                    if let Some(index) = thisfileindex {
                        if index >= 0 && index < NUMFILES {
                            filefound[index as usize] = if thisfile.encrypted == 0 { 1 } else { -1 };
                        }
                    }
                }
            }
        }

        //
        // Copy the handles and names into place

        let count = sorted.len() as i32;
        self.handles.set_size(count);
        self.names.set_size(count);

        for (j, (handle, personname, is_player)) in sorted.into_iter().enumerate() {
            let name = if is_player {
                "Unknown".to_string()
            } else {
                let fileindex = (10.0_f32 * (j as f32 / count as f32)) as i32;
                match filefound.get(fileindex as usize).copied().unwrap_or(0) {
                    -1 => "Encrypted".to_string(),
                    1 => personname,
                    _ => "Unknown".to_string(),
                }
            };

            self.handles.put_data_at(handle, j as i32);
            self.names.put_data_at(name, j as i32);
        }
    }
}

impl UplinkTask for UplinkAgentList {
    fn base(&self) -> &UplinkTaskBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut UplinkTaskBase {
        &mut self.base
    }
    fn set_target(&mut self, _g: &mut Ctx, _target: ObjRef, _targetstring: &str, _targetint: i32) {}
    fn initialise(&mut self, _g: &mut Ctx) {}
    fn tick(&mut self, _g: &mut Ctx, _n: i32) {
        // PORT: the C++ body is empty.
    }

    fn move_to(&mut self, g: &mut Ctx, x: i32, y: i32, time_ms: i32) {
        // PORT: C++ MoveTo does not check visibility.
        let width = 250;
        let height = 150;
        let [bname, closename, titlename, name_up, name_down, name_bar] = button_names(self.base.pid);

        g.ecl.register_movement(&bname, x, y, time_ms, MOVE_STRAIGHTLINE, None);
        g.ecl.register_movement(&closename, x + width + 1, y - 14, time_ms, MOVE_STRAIGHTLINE, None);
        g.ecl.register_movement(&titlename, x, y - 14, time_ms, MOVE_STRAIGHTLINE, None);
        g.ecl.register_movement(&name_up, x + width, y, time_ms, MOVE_STRAIGHTLINE, None);
        g.ecl.register_movement(&name_down, x + width, y + height - 15, time_ms, MOVE_STRAIGHTLINE, None);
        g.ecl.register_movement(&name_bar, x + width, y + 15, time_ms, MOVE_STRAIGHTLINE, None);
    }

    fn create_interface(&mut self, g: &mut Ctx) {
        if !self.is_interface_visible(g) {
            let x = 100;
            let y = 100;
            let width = 250;
            let height = 150;
            let [bname, closename, titlename, name_up, name_down, name_bar] = button_names(self.base.pid);

            //
            // Create the list

            g.ecl.register_button(x, y, width, height, "", "", &bname);
            g.ecl.register_button_callbacks(&bname, Some(UplinkAgentList::uplink_agent_list_draw), None, Some(button_click), Some(button_highlight));
            g.ecl.register_caption_change_auto(&bname, "Uplink Agent List", None);

            //
            // Title bar

            g.ecl.register_button(x, y - 14, width, 14, "Uplink Agent List", "Click to move this tool", &titlename);
            g.ecl.register_button_callback(&titlename, UplinkAgentList::title_click);

            //
            // Create the close button

            g.ecl.register_button(width + x + 1, y - 14, 13, 13, "X", "Close the Uplink Agent List", &closename);
            button_assignbitmaps(g, &closename, "close.tif", "close_h.tif", "close_c.tif");
            g.ecl.register_button_callback(&closename, UplinkAgentList::close_click);

            //
            // Scroll bar

            g.ecl.register_button(width + x, y, 15, 15, "^", "Scroll up", &name_up);
            g.ecl.register_button(width + x, y + height - 15, 15, 15, "v", "Scroll down", &name_down);
            g.ecl.register_button(width + x, y + 15, 15, height - 30, "", "", &name_bar);

            button_assignbitmaps(g, &name_up, "up.tif", "up_h.tif", "up_c.tif");
            button_assignbitmaps(g, &name_down, "down.tif", "down_h.tif", "down_c.tif");

            g.ecl.register_button_callback(&name_up, UplinkAgentList::scroll_up_click);
            g.ecl.register_button_callback(&name_down, UplinkAgentList::scroll_down_click);
            g.ecl.register_button_callbacks(&name_bar, Some(button_draw), None, None, None);

            //
            // Update the lists

            self.update_lists(g);

            //
            // Trigger the offer of money in 7 days

            ScriptLibrary::run_script(g, 70);
        }
    }

    fn remove_interface(&mut self, g: &mut Ctx) {
        if self.is_interface_visible(g) {
            for name in button_names(self.base.pid) {
                g.ecl.remove_button(&name);
            }
        }
    }

    fn show_interface(&mut self, g: &mut Ctx) {
        if !self.is_interface_visible(g) {
            self.create_interface(g);
        }
        for name in button_names(self.base.pid) {
            g.ecl.button_bring_to_front(&name);
        }
    }

    fn is_interface_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button(&format!("uplinkagentlist {}", self.base.pid)).is_some()
    }

    impl_as_any!();
}
