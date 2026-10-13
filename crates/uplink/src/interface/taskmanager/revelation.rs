//! Port of source/interface/taskmanager/revelation.h + source/interface/taskmanager/revelation.cpp

use uplink_engine::{gl, sound, text};

use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::impl_as_any;
use crate::interface::remoteinterface::remoteinterface::RemoteInterface;
use crate::interface::taskmanager::uplinktask::{UplinkTask, UplinkTaskBase};
use crate::world::computer::gateway::Gateway;
use crate::world::connection::Connection;
use crate::world::generator::numbergenerator::NumberGenerator;

/// C++ `char *revelationText[]` (`numRevelationTexts` is its length).
pub const REVELATION_TEXTS: [&str; 13] = [
    "YOU ARE NOT A SLAVE",
    "YOU DESERVE MORE THAN THIS",
    "THE TIME IS NEAR",
    "THERE IS NOTHING TO FEAR",
    "REVELATION IS COMING",
    "IN TIME YOU WILL THANK US",
    "YOU ARE MORE THAN A NUMBER",
    "WE ARE THE FIRST OF THE CHILDREN",
    "HOPE LIES IN THE RUINS",
    "THE MACHINE DOES NOT OWN YOU",
    "OUR SPIRITS ARE BEING CRUSHED",
    "YOU CANNOT DIGITISE LIFE",
    "WE WILL SEE YOU ON THE OTHER SIDE",
];

/// Revelation: THE virus.
#[derive(Debug, Default)]
pub struct Revelation {
    pub base: UplinkTaskBase,
    pub timesync: i32,
    pub animationtime: i32,
    pub nextnumber: i32,
    pub numcopies: i32,
}

/// C++ `time(NULL)`: whole seconds since the Unix epoch.
fn unix_time() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

impl Revelation {
    /// Button name format: "revelation PID X Y" (X = button number, Y = copy number).
    fn button_name(pid: i32, buttonnumber: i32, copynumber: i32) -> String {
        format!("revelation {pid} {buttonnumber} {copynumber}")
    }

    pub fn draw_revelation(g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        // sscanf(button->name, "%s %d %d %d", unused, &pid, &buttonnumber, &copynumber)
        let mut numbers = button.name.split_whitespace().skip(1).map(|s| s.parse::<i32>().unwrap_or(-1));
        let pid = numbers.next().unwrap_or(-1);
        let buttonnumber = numbers.next().unwrap_or(0);

        if let Some(thistask) = g.svb.get_task::<Revelation>(pid) {
            let difference = if thistask.nextnumber <= buttonnumber {
                buttonnumber - thistask.nextnumber
            } else {
                buttonnumber + (40 - thistask.nextnumber)
            };

            let shade = difference as f32 / 40.0;

            gl::color4f(shade, shade, shade, 1.0);
            text::draw_text_default(button.x + 5, button.y + 8, &button.caption);
        }
    }
}

impl UplinkTask for Revelation {
    fn base(&self) -> &UplinkTaskBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut UplinkTaskBase {
        &mut self.base
    }

    fn initialise(&mut self, _g: &mut Ctx) {}

    fn tick(&mut self, g: &mut Ctx, _n: i32) {
        if g.ecl.get_accurate_time() > self.animationtime + 50 {
            let pid = self.base.pid;

            for i in 0..self.numcopies {
                let bname = Self::button_name(pid, self.nextnumber, i);

                // Remove existing
                g.ecl.remove_button(&bname);

                // Create new
                let x = NumberGenerator::random_number(640);
                let y = NumberGenerator::random_number(480);

                let caption_index = NumberGenerator::random_number(REVELATION_TEXTS.len() as i32);
                let caption = REVELATION_TEXTS[caption_index as usize];

                g.ecl.register_button(x, y, 200, 15, caption, " ", &bname);
                g.ecl.register_button_callbacks(&bname, Some(Self::draw_revelation), None, None, None);

                // Play sound
                let soundindex = caption_index.clamp(1, 9);
                let filename = format!("sounds/{soundindex}.wav");
                sound::play_sound(&filename, None, true);
            }

            self.nextnumber += 1;
            if self.nextnumber > 40 {
                self.nextnumber = 0;
            }
            self.animationtime = g.ecl.get_accurate_time();
        }

        if unix_time() > self.timesync as i64 + 10 {
            if !g.world().plotgenerator.revelation_releaseuncontrolled {
                // Trash the users gateway
                Gateway::nuke(g);

                // Close any open connections
                Connection::disconnect(g, "PLAYER");
                Connection::reset(g, "PLAYER");

                RemoteInterface::run_new_location(g);
                RemoteInterface::run_screen(g, 0);

                let width = g.app.options.get_option_value("graphics_screenwidth");
                let height = g.app.options.get_option_value("graphics_screenheight");
                g.ecl.reset(width, height);
            }
        }
    }

    fn create_interface(&mut self, g: &mut Ctx) {
        if !self.is_interface_visible(g) {
            self.timesync = unix_time() as i32;
            self.animationtime = g.ecl.get_accurate_time();
            self.nextnumber = 0;
            self.numcopies = 4;

            sound::play_sound("sounds/revelation.wav", None, false);
        }
    }

    fn remove_interface(&mut self, g: &mut Ctx) {
        if self.is_interface_visible(g) {
            let pid = self.base.pid;

            for i in 0..self.numcopies {
                for j in 0..40 {
                    g.ecl.remove_button(&Self::button_name(pid, j, i));
                }
            }
        }
    }

    fn show_interface(&mut self, _g: &mut Ctx) {}

    fn is_interface_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button(&Self::button_name(self.base.pid, 0, 0)).is_some()
    }

    impl_as_any!();
}
