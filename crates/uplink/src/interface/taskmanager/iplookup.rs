//! Port of source/interface/taskmanager/iplookup.h + source/interface/taskmanager/iplookup.cpp

use uplink_engine::gl;

use crate::app::globals::ALPHA;
use crate::app::miscutils::strip_carriage_returns;
use crate::app::opengl_interface::{
    border_draw, button_assignbitmap, button_assignbitmaps, button_click, button_highlight, text_draw,
};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::interface::taskmanager::uplinktask::{UplinkTask, UplinkTaskBase};
use crate::objref::ObjRef;
use crate::vanbakel::Vanbakel;
use crate::world::person::Person;

pub const IPLOOKUP_IDLE: i32 = 0;
pub const IPLOOKUP_INPROGRESS: i32 = 1;
pub const IPLOOKUP_FINISHED: i32 = 2;

#[derive(Debug)]
pub struct IPLookup {
    pub base: UplinkTaskBase,
    pub status: i32,
    /// Used to reset to idle after 5 secs (unix seconds)
    pub timeout: i64,
}

/// C++ `sscanf(name, "<literal> %d", &pid)`: the integer after `literal` (whitespace skipped).
fn scan_pid(name: &str, literal: &str) -> Option<i32> {
    let rest = name.strip_prefix(literal)?.trim_start();
    let end = rest
        .char_indices()
        .find(|(i, c)| !(c.is_ascii_digit() || (*i == 0 && (*c == '-' || *c == '+'))))
        .map(|(i, _)| i)
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

/// C++ `time(NULL)`.
fn unix_time() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

impl IPLookup {
    pub fn new() -> Self {
        IPLookup {
            base: UplinkTaskBase {
                version: 1.0,
                targetint: -1,
                ..Default::default()
            },
            status: IPLOOKUP_IDLE,
            timeout: 0,
        }
    }

    /// C++ `IPLookup::DisplayDraw` (static button callback).
    pub fn display_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        // Draw a grey background
        gl::begin(gl::QUADS);
        gl::color4f(0.6, 0.6, 0.6, ALPHA);
        gl::vertex2i(button.x, button.y);
        gl::color4f(0.4, 0.4, 0.4, ALPHA);
        gl::vertex2i(button.x + button.width, button.y);
        gl::color4f(0.6, 0.6, 0.6, ALPHA);
        gl::vertex2i(button.x + button.width, button.y + button.height);
        gl::color4f(0.4, 0.4, 0.4, ALPHA);
        gl::vertex2i(button.x, button.y + button.height);
        gl::end();

        // Draw ordinary text
        text_draw(g, button, highlighted, clicked);

        // Draw a border
        if highlighted || clicked {
            border_draw(button);
        }
    }

    /// C++ `IPLookup::CloseClick` (static button callback).
    pub fn close_click(g: &mut Ctx, button: &Button) {
        let Some(pid) = scan_pid(&button.name, "iplookup_close") else {
            return;
        };
        Vanbakel::remove_task(g, pid);
    }

    /// C++ `IPLookup::GoClick` (static button callback).
    pub fn go_click(g: &mut Ctx, button: &Button) {
        let Some(pid) = scan_pid(&button.name, "iplookup_go") else {
            return;
        };

        if let Some(task) = g.svb.get_task_mut::<IPLookup>(pid) {
            if task.status == IPLOOKUP_IDLE {
                task.status = IPLOOKUP_INPROGRESS;
            } else {
                task.status = IPLOOKUP_IDLE;
            }
        }
    }
}

impl Default for IPLookup {
    fn default() -> Self {
        Self::new()
    }
}

impl UplinkTask for IPLookup {
    fn base(&self) -> &UplinkTaskBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut UplinkTaskBase {
        &mut self.base
    }
    fn initialise(&mut self, _g: &mut Ctx) {}
    // C++ UplinkTask::SetTarget / MoveTo are empty: the target is not stored.
    fn set_target(&mut self, _g: &mut Ctx, _target: ObjRef, _targetstring: &str, _targetint: i32) {}
    fn tick(&mut self, g: &mut Ctx, _n: i32) {
        if !self.is_interface_visible(g) {
            return;
        }

        if self.status == IPLOOKUP_IDLE {
            // Do nothing
        } else if self.status == IPLOOKUP_INPROGRESS {
            // Try to lookup the IP in the box
            let pid = self.base.pid;
            let name_display = format!("iplookup_display {pid}");

            let Some(button) = g.ecl.get_button(&name_display) else {
                panic!("IPLookup::Tick: display button missing");
            };
            let ip = strip_carriage_returns(&button.caption);

            match g.world().get_vlocation(&ip).map(|vl| vl.computer.clone()) {
                Some(computer) => {
                    // Found - add to links
                    g.ecl.register_caption_change_auto(&name_display, &computer, None);
                    let owner = g.player().name.clone();
                    Person::give_link(g, &owner, &ip);
                }
                None => {
                    // Not found
                    g.ecl.register_caption_change_auto(&name_display, "IP Not found", None);
                }
            }

            self.timeout = unix_time();
            self.status = IPLOOKUP_FINISHED;
        } else if self.status == IPLOOKUP_FINISHED {
            // Revert to idle after 5 secs
            if unix_time() > self.timeout + 5 {
                let name_display = format!("iplookup_display {}", self.base.pid);

                g.ecl.register_caption_change_auto(&name_display, "Enter IP", None);
                self.status = IPLOOKUP_IDLE;
            }
        } else {
            panic!("Unkown Status");
        }
    }
    fn create_interface(&mut self, g: &mut Ctx) {
        if !self.is_interface_visible(g) {
            let pid = self.base.pid;

            let name_go = format!("iplookup_go {pid}");
            let name_display = format!("iplookup_display {pid}");
            let name_close = format!("iplookup_close {pid}");

            g.ecl.register_button(305, 444, 20, 15, "", "Perform the lookup", &name_go);
            g.ecl.register_button(325, 444, 185, 14, "Enter IP", "Type your IP here", &name_display);
            g.ecl.register_button(510, 445, 13, 13, "", "Close the IP Lookup program", &name_close);

            button_assignbitmap(g, &name_go, "software/go.tif");
            g.ecl.register_button_callback(&name_go, Self::go_click);
            g.ecl.register_button_callbacks(
                &name_display,
                Some(Self::display_draw),
                None,
                Some(button_click),
                Some(button_highlight),
            );
            button_assignbitmaps(g, &name_close, "close.tif", "close_h.tif", "close_c.tif");
            g.ecl.register_button_callback(&name_close, Self::close_click);

            g.ecl.make_button_editable(&name_display);
        }
    }
    fn remove_interface(&mut self, g: &mut Ctx) {
        if self.is_interface_visible(g) {
            let pid = self.base.pid;

            let name_go = format!("iplookup_go {pid}");
            let name_display = format!("iplookup_display {pid}");
            let name_close = format!("iplookup_close {pid}");

            g.ecl.remove_button(&name_go);
            g.ecl.remove_button(&name_display);
            g.ecl.remove_button(&name_close);
        }
    }
    fn show_interface(&mut self, g: &mut Ctx) {
        if !self.is_interface_visible(g) {
            self.create_interface(g);
        }

        let pid = self.base.pid;

        let name_go = format!("iplookup_go {pid}");
        let name_display = format!("iplookup_display {pid}");
        let name_close = format!("iplookup_close {pid}");

        g.ecl.button_bring_to_front(&name_go);
        g.ecl.button_bring_to_front(&name_display);
        g.ecl.button_bring_to_front(&name_close);
    }
    fn is_interface_visible(&self, g: &Ctx) -> bool {
        let name_display = format!("iplookup_display {}", self.base.pid);
        g.ecl.get_button(&name_display).is_some()
    }
    crate::impl_as_any!();
}
