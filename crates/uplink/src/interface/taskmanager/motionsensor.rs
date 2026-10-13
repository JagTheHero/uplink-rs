//! Port of source/interface/taskmanager/motionsensor.h + source/interface/taskmanager/motionsensor.cpp

use uplink_engine::gl;

use crate::app::opengl_interface::{border_draw, button_click, button_highlight};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::game::game::GAMESPEED_NORMAL;
use crate::impl_as_any;
use crate::interface::taskmanager::uplinktask::{UplinkTask, UplinkTaskBase};
use crate::objref::ObjRef;
use crate::vanbakel::Vanbakel;

/// Motion Sensor task: runs on your gateway in the background and keeps track of people near
/// your machine.
#[derive(Debug, Default)]
pub struct MotionSensor {
    pub base: UplinkTaskBase,
    pub oldnumpeople: i32,
}

/// `sscanf(s, "%d", &n)`: the leading integer of a string, if there is one.
fn leading_int(s: &str) -> Option<i32> {
    let t = s.trim_start();
    let mut end = 0;
    for (i, c) in t.char_indices() {
        if c.is_ascii_digit() || (i == 0 && (c == '-' || c == '+')) {
            end = i + c.len_utf8();
        } else {
            break;
        }
    }
    t[..end].parse().ok()
}

/// `sscanf(name, "motionsensor_main %d", &pid)`.
fn pid_from_name(name: &str) -> i32 {
    name.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(-1)
}

impl MotionSensor {
    pub fn new() -> Self {
        Self::default()
    }

    /// The button name of this task's sensor: "motionsensor_main PID".
    fn button_name(pid: i32) -> String {
        format!("motionsensor_main {pid}")
    }

    pub fn sensor_draw(_g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        // Get the number of nearby people from the caption
        let numpeople = leading_int(&button.caption).unwrap_or(0);

        // Draw a coloured box depending on the number of people
        if numpeople == 0 {
            gl::color3f(0.2, 0.2, 0.2);
        } else if numpeople == 1 {
            gl::color3f(0.9, 0.9, 0.2);
        } else if numpeople >= 2 {
            gl::color3f(1.0, 0.3, 0.3);
        }

        gl::begin(gl::QUADS);
        gl::vertex2d(button.x as f64, button.y as f64);
        gl::vertex2d((button.x + button.width) as f64, button.y as f64);
        gl::vertex2d((button.x + button.width) as f64, (button.y + button.height) as f64);
        gl::vertex2d(button.x as f64, (button.y + button.height) as f64);
        gl::end();

        // Border if mouse over
        if highlighted || clicked {
            gl::color3d(1.0, 1.0, 1.0);
            border_draw(button);
        }
    }

    pub fn sensor_close(g: &mut Ctx, button: &Button) {
        // Close it down
        let pid = pid_from_name(&button.name);
        Vanbakel::remove_task(g, pid);
    }
}

impl UplinkTask for MotionSensor {
    fn base(&self) -> &UplinkTaskBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut UplinkTaskBase {
        &mut self.base
    }
    fn set_target(&mut self, _g: &mut Ctx, _target: ObjRef, _targetstring: &str, _targetint: i32) {}
    fn move_to(&mut self, _g: &mut Ctx, _x: i32, _y: i32, _time_ms: i32) {}
    fn initialise(&mut self, _g: &mut Ctx) {}

    fn tick(&mut self, g: &mut Ctx, _n: i32) {
        // No point in doing this n times
        let numpeople = g.player().gateway.proximity;

        if g.game.is_running() && numpeople != self.oldnumpeople {
            g.game.set_game_speed(GAMESPEED_NORMAL);
            g.ecl.dirty_button("hud_speed 0");
            g.ecl.dirty_button("hud_speed 1");
            g.ecl.dirty_button("hud_speed 2");
            g.ecl.dirty_button("hud_speed 3");
        } else {
            return;
        }

        self.oldnumpeople = numpeople;

        let caption = numpeople.to_string();
        let bname = Self::button_name(self.base.pid);

        let button = g.ecl.get_button_mut(&bname).expect("MotionSensor: UplinkAssert(button)");
        button.set_caption(&caption);

        if numpeople == 0 {
            button.set_tooltip("There is nobody near your Gateway");
        } else if numpeople == 1 {
            button.set_tooltip("There is someone near your Gateway");
        } else if numpeople == 2 {
            button.set_tooltip("There is more than one person near your Gateway");
        } else if numpeople >= 3 {
            button.set_tooltip("There are a lot of people near your Gateway");
        }
    }

    fn create_interface(&mut self, g: &mut Ctx) {
        if !self.is_interface_visible(g) {
            let bname = Self::button_name(self.base.pid);

            g.ecl.register_button(280, 20, 40, 5, "0", "Shows the status of the Motion sensor on your Gateway", &bname);
            g.ecl.register_button_callbacks(
                &bname,
                Some(Self::sensor_draw),
                Some(Self::sensor_close),
                Some(button_click),
                Some(button_highlight),
            );
        }
    }

    fn remove_interface(&mut self, g: &mut Ctx) {
        if self.is_interface_visible(g) {
            let bname = Self::button_name(self.base.pid);
            g.ecl.remove_button(&bname);
        }
    }

    fn show_interface(&mut self, g: &mut Ctx) {
        if !self.is_interface_visible(g) {
            self.create_interface(g);
        }

        let bname = Self::button_name(self.base.pid);
        g.ecl.button_bring_to_front(&bname);
    }

    fn is_interface_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button(&Self::button_name(self.base.pid)).is_some()
    }

    impl_as_any!();
}
