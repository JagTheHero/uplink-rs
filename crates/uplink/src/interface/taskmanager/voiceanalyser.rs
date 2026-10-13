//! Port of source/interface/taskmanager/voiceanalyser.h + source/interface/taskmanager/voiceanalyser.cpp

use uplink_engine::gl;
use uplink_engine::sound::play_sound;

use crate::app::globals::DEMOGAME;
use crate::app::opengl_interface::{
    border_draw, button_assignbitmaps, button_click, button_highlight, clear_draw, imagebutton_draw, text_draw,
};
use crate::ctx::Ctx;
use crate::eclipse::{Button, MOVE_STRAIGHTLINE};
use crate::impl_as_any;
use crate::interface::interfacescreen::ScreenSlot;
use crate::interface::remoteinterface::remoteinterface::{SCREEN_VOICEANALYSIS, SCREEN_VOICEPHONE};
use crate::interface::remoteinterface::voiceanalysisscreen_interface::VoiceAnalysisScreenInterface;
use crate::interface::remoteinterface::voicephonescreen_interface::{
    VoicePhoneScreenInterface, VOICESTATUS_HUNGUP, VOICESTATUS_TALKING,
};
use crate::interface::taskmanager::uplinktask::{UplinkTask, UplinkTaskBase};
use crate::vanbakel::Vanbakel;
use crate::world::generator::numbergenerator::NumberGenerator;

pub const VOICEANALYSER_NUMSAMPLES: usize = 25;

pub const VOICEANALYSER_STATUS_NONE: i32 = 0;
pub const VOICEANALYSER_STATUS_WAITING: i32 = 1;
pub const VOICEANALYSER_STATUS_RECORDING: i32 = 2;
pub const VOICEANALYSER_STATUS_ANALYSING: i32 = 3;
pub const VOICEANALYSER_STATUS_READY: i32 = 4;
pub const VOICEANALYSER_STATUS_REPRODUCING: i32 = 5;

#[derive(Debug, Default)]
pub struct VoiceAnalyser {
    pub base: UplinkTaskBase,
    /// C++ `STATUS`.
    pub status: i32,
    pub sample: [i32; VOICEANALYSER_NUMSAMPLES],
    pub personname: String,
    pub timesync: i32,
    pub animsync: i32,
}

/// C++ `sscanf(button->name, "%s %d", name, &pid)`: the PID is the second token of the name.
fn button_pid(button: &Button) -> i32 {
    button.name.split_whitespace().nth(1).and_then(|t| t.parse::<i32>().ok()).unwrap_or(-1)
}

/// The remote interface's screen slot (`GetRemoteInterface()->GetInterfaceScreen()`).
fn remote_screen_slot(g: &mut Ctx) -> &mut ScreenSlot {
    &mut g.interface_mut().ri.screen
}

/// Names of the analyser's five buttons, in C++ order: title, close, text, analyser, play.
fn button_names(pid: i32) -> [String; 5] {
    [
        format!("analyser_title {pid}"),
        format!("analyser_close {pid}"),
        format!("analyser_text {pid}"),
        format!("analyser_analyser {pid}"),
        format!("analyser_play {pid}"),
    ]
}

/// Status of this task with the given PID, or `None` if there is no such task.
fn task_status(g: &Ctx, pid: i32) -> Option<i32> {
    g.svb.get_task::<VoiceAnalyser>(pid).map(|t| t.status)
}

/// Voice phone status, if the current remote screen is a voice phone screen.
fn voice_phone_status(g: &Ctx) -> Option<i32> {
    g.interface().ri.screen.get::<VoicePhoneScreenInterface>().map(|vps| vps.status)
}

/// Company name of the computer the voice phone screen is showing.
fn voice_phone_company(g: &Ctx) -> Option<String> {
    let vps = g.interface().ri.screen.get::<VoicePhoneScreenInterface>()?;
    let cs = vps.get_computer_screen(g)?;
    cs.base.get_computer(g.world()).map(|c| c.companyname.clone())
}

impl VoiceAnalyser {
    pub fn new() -> Self {
        VoiceAnalyser {
            status: VOICEANALYSER_STATUS_NONE,
            personname: " ".to_string(),
            ..Default::default()
        }
    }

    pub fn close_click(g: &mut Ctx, button: &Button) {
        let pid = button_pid(button);
        Vanbakel::remove_task(g, pid);
    }

    pub fn title_click(g: &mut Ctx, button: &Button) {
        let pid = button_pid(button);
        let Some(task) = g.svb.get_task_mut::<VoiceAnalyser>(pid) else { return };
        task.base.followmouse = true;
        g.interface_mut().tm.set_target_program(pid);
    }

    pub fn play_click(g: &mut Ctx, button: &Button) {
        let pid = button_pid(button);
        let Some((status, personname)) =
            g.svb.get_task::<VoiceAnalyser>(pid).map(|t| (t.status, t.personname.clone()))
        else {
            return;
        };
        if status != VOICEANALYSER_STATUS_READY {
            return;
        }

        if !DEMOGAME {
            let voiceindex = g
                .world()
                .get_person(&personname)
                .expect("voice analyser: person not found")
                .voiceindex;
            play_sound(&format!("sounds/analyser/verifyme{voiceindex}.wav"), None, true);
        }

        let now = g.ecl.get_accurate_time() + 8000;
        if let Some(task) = g.svb.get_task_mut::<VoiceAnalyser>(pid) {
            task.status = VOICEANALYSER_STATUS_REPRODUCING;
            task.timesync = now;
        }

        g.ecl.register_caption_change_auto(
            &format!("analyser_text {pid}"),
            "Reproducing voice sample...",
            None,
        );

        // IF we're looking at a voice analysis screen, input this voice
        if g.interface().ri.screen.screen_id() == SCREEN_VOICEANALYSIS {
            ScreenSlot::call(g, remote_screen_slot, |s, g| {
                let vas = s
                    .as_any_mut()
                    .downcast_mut::<VoiceAnalysisScreenInterface>()
                    .expect("voice analysis screen");
                vas.start_voice_playback(g);
            });
        }
    }

    pub fn play_mouse_move(g: &mut Ctx, button: &Button) {
        let pid = button_pid(button);
        if task_status(g, pid) == Some(VOICEANALYSER_STATUS_READY) {
            button_highlight(g, button);
        }
    }

    pub fn play_mouse_down(g: &mut Ctx, button: &Button) {
        let pid = button_pid(button);
        if task_status(g, pid) == Some(VOICEANALYSER_STATUS_READY) {
            button_click(g, button);
        }
    }

    pub fn play_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        let pid = button_pid(button);
        if task_status(g, pid) == Some(VOICEANALYSER_STATUS_READY) {
            imagebutton_draw(g, button, highlighted, clicked);
        }
    }

    pub fn draw_analysis(g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        let pid = button_pid(button);
        clear_draw(g, button.x, button.y, button.width, button.height);

        let Some(samples) = g.svb.get_task::<VoiceAnalyser>(pid).map(|t| t.sample) else {
            return;
        };

        gl::begin(gl::LINE_STRIP);
        for (i, &s) in samples.iter().enumerate() {
            let x = button.x + (i as i32) * (button.width / VOICEANALYSER_NUMSAMPLES as i32);
            let y = (button.y + button.height) - s - 1;
            // C: `0.7 - ((float) s / 40.0)` and `(float) s / 40.0` are computed in double.
            let r = (0.7_f64 - (s as f64) / 40.0) as f32;
            let b = ((s as f64) / 40.0) as f32;
            gl::color4f(r, 0.1, b, 1.0);
            gl::vertex2i(x, y);
        }
        gl::end();

        gl::color4f(1.0, 1.0, 1.0, 1.0);
        border_draw(button);
    }

    pub fn main_text_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        let (x, y, w, h) = (button.x, button.y, button.width, button.height);

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
        border_draw(button);

        text_draw(g, button, highlighted, clicked);
    }
}

impl UplinkTask for VoiceAnalyser {
    fn base(&self) -> &UplinkTaskBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut UplinkTaskBase {
        &mut self.base
    }
    fn initialise(&mut self, _g: &mut Ctx) {}

    fn tick(&mut self, g: &mut Ctx, _n: i32) {
        let mut maxsample = 0;
        let pid = self.base.pid;
        let textbutton = format!("analyser_text {pid}");

        match self.status {
            VOICEANALYSER_STATUS_WAITING => {
                maxsample = 10;

                // Are we looking at a voice phone screen? If so, start recording the voice
                if voice_phone_status(g) == Some(VOICESTATUS_TALKING) {
                    g.ecl.register_caption_change_auto(&textbutton, "Recording voice pattern...", None);
                    self.status = VOICEANALYSER_STATUS_RECORDING;
                    if let Some(name) = voice_phone_company(g) {
                        self.personname = name;
                    }
                }
            }

            VOICEANALYSER_STATUS_RECORDING => {
                maxsample = 40;

                // We are now recording a voice until he finishes talking
                if g.interface().ri.screen.screen_id() != SCREEN_VOICEPHONE {
                    g.ecl.register_caption_change_auto(&textbutton, "Awaiting voice input...", None);
                    self.status = VOICEANALYSER_STATUS_WAITING;
                }

                // PORT: the C++ casts the current screen to a voice phone screen unconditionally;
                // here a screen of another type simply does not count as hung up.
                if voice_phone_status(g) == Some(VOICESTATUS_HUNGUP) {
                    g.ecl.register_caption_change_auto(&textbutton, "Analysing voice recording...", None);
                    self.status = VOICEANALYSER_STATUS_ANALYSING;
                    self.timesync = g.ecl.get_accurate_time() + 5000;
                }
            }

            VOICEANALYSER_STATUS_ANALYSING => {
                maxsample = 10;

                if self.timesync > 0 && g.ecl.get_accurate_time() > self.timesync {
                    // Finished analysing
                    g.ecl.register_caption_change_auto(
                        &textbutton,
                        "Voice analysis complete.\nPlayback ready.",
                        None,
                    );
                    self.status = VOICEANALYSER_STATUS_READY;
                    self.timesync = 0;
                    g.ecl.dirty_button(&format!("analyser_play {pid}"));
                }
            }

            VOICEANALYSER_STATUS_READY => {
                maxsample = 10;
            }

            VOICEANALYSER_STATUS_REPRODUCING => {
                // We are currently playing back
                maxsample = 40;

                if self.timesync > 0 && g.ecl.get_accurate_time() > self.timesync {
                    // Finished playing back
                    g.ecl.register_caption_change_auto(
                        &textbutton,
                        "Finished playing back.\nPress play to repeat.",
                        None,
                    );
                    self.status = VOICEANALYSER_STATUS_READY;
                    self.timesync = 0;
                    g.ecl.dirty_button(&format!("analyser_play {pid}"));

                    // IF we're looking at a voice analyser screen, tell it that we've
                    // finished playing our sample
                    if g.interface().ri.screen.screen_id() == SCREEN_VOICEANALYSIS {
                        let personname = self.personname.clone();
                        ScreenSlot::call(g, remote_screen_slot, |s, g| {
                            let vas = s
                                .as_any_mut()
                                .downcast_mut::<VoiceAnalysisScreenInterface>()
                                .expect("voice analysis screen");
                            vas.finish_voice_playback(g, &personname);
                        });
                    }
                }
            }

            _ => {}
        }

        // ==================================================================================
        // Update the voice analyser image
        //

        if g.ecl.get_accurate_time() > self.animsync + 75 {
            for s in self.sample.iter_mut() {
                *s += NumberGenerator::random_number(maxsample / 2) - (maxsample / 4);
                if *s < 0 {
                    *s = 0;
                }
                if *s > maxsample {
                    *s = maxsample;
                }
            }

            g.ecl.dirty_button(&format!("analyser_analyser {pid}"));
            self.animsync = g.ecl.get_accurate_time();
        }
    }

    fn move_to(&mut self, g: &mut Ctx, x: i32, y: i32, time_ms: i32) {
        if self.is_interface_visible(g) {
            let [title, close, text, analyser, play] = button_names(self.base.pid);

            g.ecl.register_movement(&title, x + 1, y + 1, time_ms, MOVE_STRAIGHTLINE, None);
            g.ecl.register_movement(&close, x + 276, y + 1, time_ms, MOVE_STRAIGHTLINE, None);
            g.ecl.register_movement(&text, x + 1, y + 13, time_ms, MOVE_STRAIGHTLINE, None);
            g.ecl.register_movement(&analyser, x + 188, y + 13, time_ms, MOVE_STRAIGHTLINE, None);
            g.ecl.register_movement(&play, x + 170, y + 45, time_ms, MOVE_STRAIGHTLINE, None);

            g.ecl.button_bring_to_front(&title);
            g.ecl.button_bring_to_front(&close);
            g.ecl.button_bring_to_front(&text);
            g.ecl.button_bring_to_front(&analyser);
            g.ecl.button_bring_to_front(&play);
        }
    }

    fn create_interface(&mut self, g: &mut Ctx) {
        if !self.is_interface_visible(g) {
            let [title, close, text, analyser, play] = button_names(self.base.pid);

            g.ecl.register_button(200, 400, 275, 13, "Voice Analyser", "Click to move this application", &title);
            g.ecl.register_button_callback(&title, VoiceAnalyser::title_click);

            g.ecl.register_button(475, 400, 13, 13, "Close", "Close the analyser software", &close);
            button_assignbitmaps(g, &close, "close.tif", "close_h.tif", "close_c.tif");
            g.ecl.register_button_callback(&close, VoiceAnalyser::close_click);

            g.ecl.register_button(200, 413, 188, 50, "Awaiting voice input...", "", &text);
            g.ecl.register_button_callbacks(&text, Some(VoiceAnalyser::main_text_draw), None, None, None);

            g.ecl.register_button(388, 413, 100, 50, " ", " ", &analyser);
            g.ecl.register_button_callbacks(&analyser, Some(VoiceAnalyser::draw_analysis), None, None, None);

            g.ecl.register_button(370, 445, 15, 15, " ", "Play the analysed voice recording", &play);
            button_assignbitmaps(g, &play, "software/play.tif", "software/play_h.tif", "software/play_c.tif");
            g.ecl.register_button_callbacks(
                &play,
                Some(VoiceAnalyser::play_draw),
                Some(VoiceAnalyser::play_click),
                Some(VoiceAnalyser::play_mouse_down),
                Some(VoiceAnalyser::play_mouse_move),
            );

            self.status = VOICEANALYSER_STATUS_WAITING;
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
        if self.is_interface_visible(g) {
            for name in button_names(self.base.pid) {
                g.ecl.button_bring_to_front(&name);
            }
        }
    }

    fn is_interface_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button(&format!("analyser_title {}", self.base.pid)).is_some()
    }

    impl_as_any!();
}
