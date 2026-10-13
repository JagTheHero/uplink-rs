//! Port of source/interface/taskmanager/revelationtracker.h + source/interface/taskmanager/revelationtracker.cpp

use uplink_engine::gl;

use crate::app::opengl_interface::{border_draw, button_assignbitmaps, text_draw};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::impl_as_any;
use crate::interface::taskmanager::uplinktask::{UplinkTask, UplinkTaskBase};
use crate::vanbakel::Vanbakel;

/// Revelation Tracker: sits on the player's computer and informs him when computers are infected with Revelation.
#[derive(Debug, Default)]
pub struct RevelationTracker {
    pub base: UplinkTaskBase,
    pub timesync: i32,
    /// In rows.
    pub height: i32,
}

/// `sscanf(name, "%s %d", bname, &pid)`: the number after the first word of a button name.
fn pid_from_name(name: &str) -> i32 {
    name.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(-1)
}

/// C++ `time(NULL)`: whole seconds since the Unix epoch.
fn unix_time() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

impl RevelationTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// The three button names of this task's window: title, box, close.
    fn button_names(pid: i32) -> [String; 3] {
        [
            format!("revelationtracker_title {pid}"),
            format!("revelationtracker_box {pid}"),
            // C++ spelling kept: "revaltiontracker_close"
            format!("revaltiontracker_close {pid}"),
        ]
    }

    pub fn title_click(g: &mut Ctx, button: &Button) {
        let pid = pid_from_name(&button.name);
        g.interface_mut().tm.set_target_program(pid);
    }

    pub fn close_click(g: &mut Ctx, button: &Button) {
        let pid = pid_from_name(&button.name);
        Vanbakel::remove_task(g, pid);
    }

    pub fn main_box_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
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

        text_draw(g, button, highlighted, clicked);
    }
}

impl UplinkTask for RevelationTracker {
    fn base(&self) -> &UplinkTaskBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut UplinkTaskBase {
        &mut self.base
    }

    fn move_to(&mut self, g: &mut Ctx, x: i32, y: i32, time_ms: i32) {
        if self.is_interface_visible(g) {
            let [titlename, boxname, closename] = Self::button_names(self.base.pid);

            g.ecl.register_movement(&titlename, x, y, time_ms, crate::eclipse::MOVE_STRAIGHTLINE, None);
            g.ecl.register_movement(&boxname, x, y + 15, time_ms, crate::eclipse::MOVE_STRAIGHTLINE, None);
            g.ecl.register_movement(&closename, x + 130, y + 1, time_ms, crate::eclipse::MOVE_STRAIGHTLINE, None);
        }
    }

    fn initialise(&mut self, _g: &mut Ctx) {}

    fn tick(&mut self, g: &mut Ctx, _n: i32) {
        if unix_time() > self.timesync as i64 + 3 {
            let pid = self.base.pid;
            let [_, boxname, _] = Self::button_names(pid);
            let width = g.ecl.get_button(&boxname).expect("RevelationTracker: UplinkAssert(button)").width;

            // How many computers are infected?
            let numinfected = g.world().plotgenerator.infected.len() as i32;

            if numinfected != self.height {
                // Resize the box
                g.ecl.register_resize(&boxname, width, 15 + numinfected * 15, 1000, None);
                self.height = numinfected;
            }

            // Rebuild the caption
            if numinfected == 0 {
                g.ecl.get_button_mut(&boxname).expect("RevelationTracker: UplinkAssert(button)").set_caption("No infected systems");
            } else {
                let mut newcaption = String::from("Infected systems\n");

                let world = g.world();
                for ip in &world.plotgenerator.infected {
                    let vl = world.get_vlocation(ip).expect("RevelationTracker: UplinkAssert(vl)");
                    let comp = vl.get_computer(world).expect("RevelationTracker: UplinkAssert(comp)");

                    if comp.isinfected_revelation > 0.0 {
                        newcaption.push_str(&comp.ip);
                        newcaption.push_str(" at ");
                        newcaption.push_str(&format!(
                            "{}:{}",
                            comp.infectiondate.get_hour(),
                            comp.infectiondate.get_minute()
                        ));
                        newcaption.push('\n');
                    }
                }

                g.ecl.get_button_mut(&boxname).expect("RevelationTracker: UplinkAssert(button)").set_caption(&newcaption);
            }
        }
    }

    fn create_interface(&mut self, g: &mut Ctx) {
        if !self.is_interface_visible(g) {
            let pid = self.base.pid;
            let [titlename, boxname, closename] = Self::button_names(pid);

            g.ecl.register_button(0, 50, 130, 15, "Revelation Tracker", "Shows all currently infected computers", &titlename);
            g.ecl.register_button(130, 51, 13, 13, "X", "Close the Revelation Tracker", &closename);
            g.ecl.register_button(0, 65, 143, 30, "Listening...", " ", &boxname);

            button_assignbitmaps(g, &closename, "close.tif", "close_h.tif", "close_c.tif");

            g.ecl.register_button_callback(&titlename, Self::title_click);
            g.ecl.register_button_callback(&closename, Self::close_click);
            g.ecl.register_button_callbacks(&boxname, Some(Self::main_box_draw), None, None, None);

            self.timesync = unix_time() as i32;
            self.height = 0;
        }
    }

    fn remove_interface(&mut self, g: &mut Ctx) {
        if self.is_interface_visible(g) {
            for name in Self::button_names(self.base.pid) {
                g.ecl.remove_button(&name);
            }
        }
    }

    fn show_interface(&mut self, g: &mut Ctx) {
        if self.is_interface_visible(g) {
            for name in Self::button_names(self.base.pid) {
                g.ecl.button_bring_to_front(&name);
            }
        }
    }

    fn is_interface_visible(&self, g: &Ctx) -> bool {
        let [titlename, _, _] = Self::button_names(self.base.pid);
        g.ecl.get_button(&titlename).is_some()
    }

    impl_as_any!();
}
