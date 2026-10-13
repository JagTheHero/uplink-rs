//! Port of source/interface/taskmanager/tracetracker.h + source/interface/taskmanager/tracetracker.cpp

use uplink_engine::{gl, sound, text};

use crate::app::globals::ALPHA;
use crate::app::opengl;
use crate::app::opengl_interface;
use crate::ctx::Ctx;
use crate::eclipse::{Button, MOVE_STRAIGHTLINE};
use crate::impl_as_any;
use crate::interface::taskmanager::uplinktask::{UplinkTask, UplinkTaskBase};
use crate::vanbakel::Vanbakel;

#[derive(Debug, Default)]
pub struct TraceTracker {
    pub base: UplinkTaskBase,
    pub x: i32,
    pub y: i32,
    /// Trace progress at time of last estimate.
    pub traceprogress: i32,
    /// Time of estimated trace.
    pub traceestimate: i32,
    /// Time of next beep.
    pub nextbeep: i32,
    /// The time between beeps.
    pub beepdelay: i32,
    pub audioon: bool,
}

/// The button names of one TraceTracker window (`tracetracker_display N` etc.).
struct Names {
    display: String,
    close: String,
    audio: String,
}

fn names(pid: i32) -> Names {
    Names {
        display: format!("tracetracker_display {pid}"),
        close: format!("tracetracker_close {pid}"),
        audio: format!("tracetracker_audio {pid}"),
    }
}

/// C++ `sscanf(button->name, "%s %d", bname, &pid)`: the PID is the second word of the name.
fn pid_from_button(button: &Button) -> Option<i32> {
    button.name.split_whitespace().nth(1)?.parse().ok()
}

fn set_caption(g: &mut Ctx, name: &str, caption: &str) {
    if let Some(b) = g.ecl.get_button_mut(name) {
        b.set_caption(caption);
    }
}

impl TraceTracker {
    pub fn new() -> Self {
        TraceTracker {
            x: 0,
            y: 0,
            traceprogress: -1,
            traceestimate: -1,
            nextbeep: -1,
            beepdelay: -1,
            audioon: true,
            ..Default::default()
        }
    }

    pub fn trace_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        // Get the task
        let Some(pid) = pid_from_button(button) else { return };
        let Some((beepdelay, nextbeep)) = g.svb.get_task::<TraceTracker>(pid).map(|t| (t.beepdelay, t.nextbeep)) else {
            return;
        };

        // Work out the brightness
        let brightness: f32 = if g.player().get_connection().trace_in_progress() {
            if beepdelay < 100 {
                0.8
            } else {
                let timeoflastbeep = nextbeep - beepdelay;
                let timepassedsincelastbeep = g.ecl.get_accurate_time() - timeoflastbeep;
                // C++ `0.7` is a double: the product is computed in double, then stored in a float.
                (0.7_f64 * (timepassedsincelastbeep as f32 / beepdelay as f32) as f64) as f32
            }
        } else {
            0.0
        };

        // Draw the button
        let screenheight = opengl::screen_h();
        gl::scissor(button.x, screenheight - (button.y + button.height), button.width, button.height);
        gl::enable(gl::SCISSOR_TEST);

        gl::begin(gl::QUADS);

        gl::color4f(brightness, brightness, 0.7, 0.5);
        gl::vertex2i(button.x, button.y + button.height);

        gl::color4f(brightness, brightness, 0.4, 0.5);
        gl::vertex2i(button.x, button.y);

        gl::color4f(brightness, brightness, 0.7, 0.5);
        gl::vertex2i(button.x + button.width, button.y);

        gl::color4f(brightness, brightness, 0.4, 0.5);
        gl::vertex2i(button.x + button.width, button.y + button.height);

        gl::end();

        // Draw the text
        let xpos = (button.x + button.width / 2) - (text::text_width_default(&button.caption) / 2);
        let ypos = (button.y + button.height / 2) + 2;

        if highlighted || clicked {
            gl::color4f(1.0, 1.0, 1.0, 1.0);
        } else {
            gl::color4f(1.0, 1.0, 1.0, ALPHA);
        }
        text::draw_text_default(xpos, ypos, &button.caption);

        gl::disable(gl::SCISSOR_TEST);
    }

    pub fn audio_draw(g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        let Some(pid) = pid_from_button(button) else { return };
        let Some(audioon) = g.svb.get_task::<TraceTracker>(pid).map(|t| t.audioon) else { return };

        if audioon {
            opengl_interface::imagebutton_draw(g, button, false, false);
        } else {
            opengl_interface::imagebutton_draw(g, button, true, true);
        }
    }

    pub fn close_click(g: &mut Ctx, button: &Button) {
        let Some(pid) = pid_from_button(button) else { return };
        Vanbakel::remove_task(g, pid);
    }

    pub fn audio_click(g: &mut Ctx, button: &Button) {
        let Some(pid) = pid_from_button(button) else { return };
        let Some(thistask) = g.svb.get_task_mut::<TraceTracker>(pid) else { return };

        thistask.audioon = !thistask.audioon;
        let audioon = thistask.audioon;

        let audioname = names(pid).audio;

        if audioon {
            opengl_interface::button_assignbitmaps(g, &audioname, "software/audioon.tif", "software/audioon_h.tif", "software/audioon_c.tif");
        } else {
            opengl_interface::button_assignbitmaps(g, &audioname, "software/audiooff.tif", "software/audiooff_h.tif", "software/audiooff_c.tif");
        }
    }
}

impl UplinkTask for TraceTracker {
    fn base(&self) -> &UplinkTaskBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut UplinkTaskBase {
        &mut self.base
    }
    fn initialise(&mut self, _g: &mut Ctx) {}

    fn tick(&mut self, g: &mut Ctx, _n: i32) {
        if !self.is_interface_visible(g) {
            return;
        }
        let n = names(self.base.pid);

        // Connection *connection = game->GetWorld ()->GetPlayer ()->GetConnection ();
        // Nothing changes the connection during this tick, so its state is read once.
        let (in_progress, traced, connection_progress, size) = {
            let connection = g.player().get_connection();
            (connection.trace_in_progress(), connection.traced(), connection.traceprogress, connection.get_size())
        };

        // Calculate our best guess for the trace time

        if in_progress && connection_progress != self.traceprogress {
            let timeremaining = g.player().time_remaining(g.world());

            self.traceestimate = g.ecl.get_accurate_time() + timeremaining * 1000;
            self.traceprogress = connection_progress;
        }

        // deal with the bleeping

        if in_progress {
            if g.ecl.get_accurate_time() >= self.nextbeep {
                if self.audioon {
                    sound::play_sound("sounds/tracebleep.wav", None, true);
                }

                let timeremaining = self.traceestimate - g.ecl.get_accurate_time();
                // C++ `(float) timeremaining / 20.0`: float widened to double, divided, truncated.
                self.beepdelay = ((timeremaining as f32) as f64 / 20.0) as i32;
                self.nextbeep = g.ecl.get_accurate_time() + self.beepdelay;
            }

            g.ecl.dirty_button(&n.display);
        }

        if traced {
            set_caption(g, &n.display, "Traced");
        } else if in_progress && connection_progress > 0 {
            if self.base.version == 1.0 {
                if connection_progress >= size - 2 {
                    set_caption(g, &n.display, "Trace Imminent");
                } else {
                    set_caption(g, &n.display, "In progress");
                }
            } else if self.base.version == 2.0 {
                let percent = (100.0 * connection_progress as f32 / (size - 1) as f32) as i32;
                set_caption(g, &n.display, &format!("Trace: {percent}%"));
            } else if self.base.version == 3.0 {
                let timeremaining = g.player().time_remaining(g.world());
                set_caption(g, &n.display, &format!("Trace: {timeremaining} secs"));
            } else if self.base.version == 4.0 {
                let timeremaining = g.player().time_remaining(g.world());

                let mut accuratetimeremaining = (self.traceestimate - g.ecl.get_accurate_time()) / 1000;
                if accuratetimeremaining < 0 {
                    accuratetimeremaining = 0;
                }
                set_caption(g, &n.display, &format!("Trace: {timeremaining}s ({accuratetimeremaining})"));
            } else {
                println!("TraceTracker WARNING : Unrecognised version number\n ");
            }
        } else if in_progress && connection_progress == 0 {
            set_caption(g, &n.display, "Trace begun");
        } else {
            set_caption(g, &n.display, "No Traces");
        }
    }

    fn move_to(&mut self, g: &mut Ctx, x: i32, y: i32, time_ms: i32) {
        if !self.is_interface_visible(g) {
            return;
        }
        let n = names(self.base.pid);

        g.ecl.register_movement(&n.display, x, y, time_ms, MOVE_STRAIGHTLINE, None);
        g.ecl.register_movement(&n.close, x + 100, y, time_ms, MOVE_STRAIGHTLINE, None);
        g.ecl.register_movement(&n.audio, x - 20, y, time_ms, MOVE_STRAIGHTLINE, None);
    }

    fn create_interface(&mut self, g: &mut Ctx) {
        if self.is_interface_visible(g) {
            return;
        }
        let n = names(self.base.pid);

        g.ecl.register_button(25, 300, 100, 13, "Trace Tracker", "Trace Tracker", &n.display);
        g.ecl.register_button_callbacks(&n.display, Some(Self::trace_draw), None, None, None);

        g.ecl.register_button(125, 300, 13, 13, "", "Close the TraceTracker", &n.close);
        opengl_interface::button_assignbitmaps(g, &n.close, "close.tif", "close_h.tif", "close_c.tif");
        g.ecl.register_button_callback(&n.close, Self::close_click);

        g.ecl.register_button(5, 300, 20, 13, "", "Toggle audio beeps", &n.audio);
        opengl_interface::button_assignbitmaps(g, &n.audio, "software/audioon.tif", "software/audioon_h.tif", "software/audioon_c.tif");
        g.ecl.register_button_callbacks(
            &n.audio,
            Some(Self::audio_draw),
            Some(Self::audio_click),
            Some(opengl_interface::button_click),
            Some(opengl_interface::button_highlight),
        );

        let screen_w = opengl::screen_w();
        let screen_h = opengl::screen_h();

        self.move_to(g, screen_w - 115, screen_h - 15, 500);
    }

    fn remove_interface(&mut self, g: &mut Ctx) {
        if self.is_interface_visible(g) {
            let n = names(self.base.pid);

            g.ecl.remove_button(&n.display);
            g.ecl.remove_button(&n.close);
            g.ecl.remove_button(&n.audio);
        }
    }

    fn show_interface(&mut self, g: &mut Ctx) {
        if !self.is_interface_visible(g) {
            self.create_interface(g);
        }

        let n = names(self.base.pid);

        g.ecl.button_bring_to_front(&n.display);
        g.ecl.button_bring_to_front(&n.close);
        g.ecl.button_bring_to_front(&n.audio);
    }

    fn is_interface_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button(&names(self.base.pid).display).is_some()
    }

    impl_as_any!();
}
