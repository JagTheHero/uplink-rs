//! Port of source/interface/remoteinterface/voiceanalysisscreen_interface.h + source/interface/remoteinterface/voiceanalysisscreen_interface.cpp

use uplink_engine::gl;

use crate::app::opengl_interface;
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::remoteinterface::remoteinterface::{RemoteInterface, SCREEN_VOICEANALYSIS};
use crate::interface::remoteinterface::remoteinterfacescreen::{draw_main_title, draw_sub_title};
use crate::objref::ScreenRef;
use crate::world::computer::computerscreen::computerscreen::ComputerScreen;
use crate::world::computer::computerscreen::genericscreen::GenericScreen;
use crate::world::generator::numbergenerator::NumberGenerator;

pub const VOICE_NUMSAMPLES: i32 = 25;
pub const VOICEANALYSISSCREEN_NONE: i32 = 0;
pub const VOICEANALYSISSCREEN_LISTENING: i32 = 1;
pub const VOICEANALYSISSCREEN_ANALYSING: i32 = 2;
pub const VOICEANALYSISSCREEN_PASSED: i32 = 3;
pub const VOICEANALYSISSCREEN_FAILED: i32 = 4;

/// Remote voice analysis screen: records a sample and checks it against the voice on file.
#[derive(Debug)]
pub struct VoiceAnalysisScreenInterface {
    pub cs: Option<ScreenRef>,
    pub sample: [i32; VOICE_NUMSAMPLES as usize],
    pub last_update: i32,
    /// One of the `VOICEANALYSISSCREEN_*` states.
    pub status: i32,
}

impl Default for VoiceAnalysisScreenInterface {
    /// A screen built through `Default` must still get the constructor's random samples.
    fn default() -> Self {
        Self::new()
    }
}

impl VoiceAnalysisScreenInterface {
    /// C++ constructor: fills `sample` with random values.
    pub fn new() -> Self {
        let mut sample = [0; VOICE_NUMSAMPLES as usize];
        for s in sample.iter_mut() {
            *s = NumberGenerator::random_number(40);
        }

        VoiceAnalysisScreenInterface {
            cs: None,
            sample,
            last_update: 0,
            status: VOICEANALYSISSCREEN_NONE,
        }
    }

    /// `GetComputerScreen` (typed accessor; the C++ casts `cs`).
    pub fn get_computer_screen<'a>(&self, g: &'a Ctx) -> Option<&'a GenericScreen> {
        let cs = self.cs.as_ref()?;
        match g.world().get_computer(&cs.computer)?.get_computer_screen(cs.index)? {
            ComputerScreen::GenericScreen(gs) => Some(gs),
            _ => None,
        }
    }

    /// Starts the voice playback and listening state.
    pub fn start_voice_playback(&mut self, g: &mut Ctx) {
        self.status = VOICEANALYSISSCREEN_LISTENING;

        opengl_interface::button_assignbitmap(g, "voicescreen_access", "accessdenied.tif");
    }

    /// Ends playback and sets the result for the named voice.
    pub fn finish_voice_playback(&mut self, g: &mut Ctx, voicename: &str) {
        self.status = VOICEANALYSISSCREEN_ANALYSING;

        let administrator = self
            .cs
            .as_ref()
            .and_then(|cs| g.world().get_computer(&cs.computer))
            .and_then(|comp| g.world().get_company(&comp.companyname))
            .map(|company| company.administrator.clone());

        if administrator.as_deref() == Some(voicename) {
            self.status = VOICEANALYSISSCREEN_PASSED;
            opengl_interface::button_assignbitmap(g, "voicescreen_access", "accessgranted.tif");
        } else {
            self.status = VOICEANALYSISSCREEN_FAILED;
            opengl_interface::button_assignbitmap(g, "voicescreen_access", "accessdenied.tif");
        }
    }

    /// `DrawBackground` (static DrawFn)
    pub fn draw_background(_g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
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
        opengl_interface::border_draw(button);
    }

    /// `DrawAnalysis` (static DrawFn)
    pub fn draw_analysis(g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        let Some(sample) = g.interface().ri.screen.get::<Self>().map(|me| me.sample) else {
            opengl_interface::clear_draw(g, button.x, button.y, button.width, button.height);
            return;
        };

        opengl_interface::clear_draw(g, button.x, button.y, button.width, button.height);

        gl::begin(gl::LINE_STRIP);

        for i in 0..VOICE_NUMSAMPLES {
            let x = button.x + i * (button.width / VOICE_NUMSAMPLES);
            let y = (button.y + button.height) - sample[i as usize];
            let c = sample[i as usize] as f32 / 40.0;

            gl::color4f(0.1, 0.1, c, 1.0);
            gl::vertex2i(x, y);
        }

        gl::end();

        gl::color4f(1.0, 1.0, 1.0, 1.0);
        opengl_interface::border_draw(button);
    }

    /// `ClickAccess` (static MouseFn)
    pub fn click_access(g: &mut Ctx, _button: &Button) {
        let (passed, nextpage) = {
            let Some(me) = g.interface().ri.screen.get::<Self>() else {
                return;
            };
            (me.status == VOICEANALYSISSCREEN_PASSED, me.get_computer_screen(g).map(|gs| gs.nextpage))
        };

        if passed {
            g.interface_mut().ri.set_security("Admin", 1);
            if let Some(nextpage) = nextpage {
                if nextpage != -1 {
                    RemoteInterface::run_screen(g, nextpage);
                }
            }
        }
    }
}

impl InterfaceScreen for VoiceAnalysisScreenInterface {
    crate::impl_as_any!();

    fn create(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            return;
        }

        let Some((maintitle, subtitle)) = self
            .get_computer_screen(g)
            .map(|gs| (gs.base.maintitle.clone(), gs.base.subtitle.clone()))
        else {
            return;
        };

        g.ecl.register_button(80, 60, 350, 25, &maintitle, "", "voicescreen_maintitle");
        g.ecl.register_button_callbacks("voicescreen_maintitle", Some(draw_main_title), None, None, None);
        g.ecl.register_button(80, 80, 350, 20, &subtitle, "", "voicescreen_subtitle");
        g.ecl.register_button_callbacks("voicescreen_subtitle", Some(draw_sub_title), None, None, None);

        g.ecl.register_button(80, 130, 370, 15, " ", " ", "voicescreen_instructions");
        g.ecl.register_button_callbacks(
            "voicescreen_instructions",
            Some(opengl_interface::textbutton_draw),
            None,
            None,
            None,
        );
        g.ecl.register_caption_change(
            "voicescreen_instructions",
            "Please read the following sentence clearly into your microphone.",
            0,
            None,
        );

        g.ecl.register_button(50, 180, 380, 150, " ", " ", "voicescreen_background");
        g.ecl.register_button_callbacks("voicescreen_background", Some(Self::draw_background), None, None, None);

        g.ecl.register_button(75, 225, 100, 50, " ", " ", "voicescreen_analysis");
        g.ecl.register_button_callbacks("voicescreen_analysis", Some(Self::draw_analysis), None, None, None);

        g.ecl.register_button(185, 225, 250, 50, " ", " ", "voicescreen_sentence");
        g.ecl.register_button_callbacks("voicescreen_sentence", Some(opengl_interface::text_draw), None, None, None);
        g.ecl.register_caption_change_auto(
            "voicescreen_sentence",
            "Hello.  I am the system administrator.\nMy voice is my passport.\nVerify me.",
            None,
        );

        g.ecl.register_button(300, 300, 100, 20, " ", " ", "voicescreen_access");
        opengl_interface::button_assignbitmap(g, "voicescreen_access", "accessdenied.tif");
        g.ecl.register_button_callback("voicescreen_access", Self::click_access);
    }

    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            g.ecl.remove_button("voicescreen_maintitle");
            g.ecl.remove_button("voicescreen_subtitle");

            g.ecl.remove_button("voicescreen_background");

            g.ecl.remove_button("voicescreen_instructions");
            g.ecl.remove_button("voicescreen_analysis");
            g.ecl.remove_button("voicescreen_sentence");

            g.ecl.remove_button("voicescreen_access");
        }
    }

    fn update(&mut self, g: &mut Ctx) {
        let maxvalue = match self.status {
            VOICEANALYSISSCREEN_LISTENING => 40,
            _ => 10,
        };

        // Update the voice analysis box

        if g.ecl.get_accurate_time() > self.last_update + 75 {
            for s in self.sample.iter_mut() {
                *s += NumberGenerator::random_number(maxvalue / 2) - (maxvalue / 4);
                if *s < 0 {
                    *s = 0;
                }
                if *s > maxvalue {
                    *s = maxvalue;
                }
            }

            g.ecl.dirty_button("voicescreen_analysis");

            self.last_update = g.ecl.get_accurate_time();
        }
    }

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("voicescreen_maintitle").is_some()
    }

    fn screen_id(&self) -> i32 {
        SCREEN_VOICEANALYSIS
    }

    fn return_key_pressed(&mut self, _g: &mut Ctx) -> bool {
        false
    }

    fn set_computer_screen(&mut self, cs: ScreenRef) {
        self.cs = Some(cs);
    }

    fn computer_screen_ref(&self) -> Option<&ScreenRef> {
        self.cs.as_ref()
    }
}
