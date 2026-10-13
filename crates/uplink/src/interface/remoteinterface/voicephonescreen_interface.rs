//! Port of source/interface/remoteinterface/voicephonescreen_interface.h + source/interface/remoteinterface/voicephonescreen_interface.cpp

use uplink_engine::sound;

use crate::app::globals;
use crate::app::opengl_interface;
use crate::ctx::Ctx;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::remoteinterface::remoteinterface::SCREEN_VOICEPHONE;
use crate::interface::remoteinterface::remoteinterfacescreen::{draw_main_title, draw_sub_title};
use crate::objref::ScreenRef;
use crate::world::computer::computerscreen::genericscreen::GenericScreen;
use crate::world::generator::numbergenerator::NumberGenerator;

pub const VOICESTATUS_NONE: i32 = 0;
pub const VOICESTATUS_RINGING: i32 = 1;
pub const VOICESTATUS_CONNECTED: i32 = 2;
pub const VOICESTATUS_TALKING: i32 = 3;
pub const VOICESTATUS_HUNGUP: i32 = 5;

/// Remote voice phone screen: a phone call transcript with a voice on the other end.
#[derive(Debug)]
pub struct VoicePhoneScreenInterface {
    pub cs: Option<ScreenRef>,
    pub voiceindex: i32,
    pub numrings: i32,
    pub numwords: i32,
    /// Time of the last text sync (-1 before the first sync).
    pub timesync: i32,
    /// One of the `VOICESTATUS_*` values.
    pub status: i32,
}

impl Default for VoicePhoneScreenInterface {
    fn default() -> Self {
        VoicePhoneScreenInterface {
            cs: None,
            voiceindex: 0,
            numrings: 0,
            numwords: 0,
            timesync: -1,
            status: VOICESTATUS_NONE,
        }
    }
}

impl VoicePhoneScreenInterface {
    /// `GetComputerScreen` (typed accessor; the C++ casts `cs`).
    pub fn get_computer_screen<'a>(&self, g: &'a Ctx) -> Option<&'a GenericScreen> {
        let cs = self.cs.as_ref()?;
        g.world().get_computer(&cs.computer)?.get_computer_screen(cs.index)?.as_generic_screen()
    }

    /// `AddToText`: appends a line of call text.
    pub fn add_to_text(&mut self, g: &mut Ctx, text: &str) {
        let Some(button) = g.ecl.get_button_mut("voicescreen_text") else {
            debug_assert!(false, "VoicePhoneScreenInterface::add_to_text, no voicescreen_text button");
            return;
        };
        let newcaption = format!("{}{}", button.caption, text);
        button.set_caption(&newcaption);
    }
}

impl InterfaceScreen for VoicePhoneScreenInterface {
    crate::impl_as_any!();

    fn create(&mut self, g: &mut Ctx) {
        let Some(cs) = self.cs.clone() else {
            return;
        };

        if !self.is_visible(g) {
            let Some(gs) = self.get_computer_screen(g) else {
                debug_assert!(false, "VoicePhoneScreenInterface::create, no GenericScreen");
                return;
            };
            let maintitle = gs.base.maintitle.clone();
            let subtitle = gs.base.subtitle.clone();

            g.ecl.register_button(80, 60, 350, 25, &maintitle, "", "voicescreen_maintitle");
            g.ecl.register_button_callbacks("voicescreen_maintitle", Some(draw_main_title), None, None, None);
            g.ecl.register_button(80, 80, 350, 20, &subtitle, "", "voicescreen_subtitle");
            g.ecl.register_button_callbacks("voicescreen_subtitle", Some(draw_sub_title), None, None, None);

            g.ecl.register_button(40, 140, 300, 250, " ", " ", "voicescreen_text");
            g.ecl.register_button_callbacks("voicescreen_text", Some(opengl_interface::textbutton_draw), None, None, None);

            // Set up the screen data

            self.numrings = 2 + NumberGenerator::random_number(3);
            self.numwords = 3 + NumberGenerator::random_number(2);
            self.status = VOICESTATUS_RINGING;
            self.timesync = g.ecl.get_accurate_time() + 1000;

            // Look up who this voice screen belongs to

            let companyname = g.world().get_computer(&cs.computer).map(|comp| comp.companyname.clone());
            debug_assert!(companyname.is_some());
            let person = companyname.and_then(|name| g.world().get_person(&name));
            debug_assert!(person.is_some());
            if let Some(person) = person {
                self.voiceindex = person.voiceindex;
            }
        }
    }

    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            g.ecl.remove_button("voicescreen_maintitle");
            g.ecl.remove_button("voicescreen_subtitle");
            g.ecl.remove_button("voicescreen_text");
        }
    }

    fn update(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            if self.timesync != -1 && g.ecl.get_accurate_time() > self.timesync {
                match self.status {
                    VOICESTATUS_RINGING => {
                        self.add_to_text(g, "\nRinging...");
                        self.numrings -= 1;
                        self.timesync = g.ecl.get_accurate_time() + 2500;
                        sound::play_sound("sounds/ringout.wav", None, true);

                        if self.numrings <= 0 {
                            self.status = VOICESTATUS_CONNECTED;
                        }
                    }

                    VOICESTATUS_CONNECTED => {
                        self.add_to_text(g, "\n\nConnected");
                        self.add_to_text(g, "\nVoice channel open\n");
                        self.timesync = g.ecl.get_accurate_time() + 1500;
                        self.status = VOICESTATUS_TALKING;
                        sound::play_sound("sounds/phonepickup.wav", None, true);
                    }

                    VOICESTATUS_TALKING => {
                        self.numwords -= 1;
                        self.timesync = g.ecl.get_accurate_time() + 2500;
                        if self.numwords <= 0 {
                            self.status = VOICESTATUS_HUNGUP;
                        }
                        let mut samplenumber = self.numwords;
                        if samplenumber >= 3 {
                            samplenumber -= 3;
                        }
                        if !globals::DEMOGAME {
                            let filename = format!("sounds/analyser/hello{}_{}.wav", self.voiceindex, samplenumber);
                            sound::play_sound(&filename, None, true);
                        }
                    }

                    VOICESTATUS_HUNGUP => {
                        self.add_to_text(g, "\n\nConnection lost");
                        self.add_to_text(g, "\n(Receiver hung up)");
                        self.timesync = -1;
                        sound::play_sound("sounds/phonepickup.wav", None, true);
                    }

                    _ => {}
                }
            }
        }
    }

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("voicescreen_maintitle").is_some()
    }

    fn screen_id(&self) -> i32 {
        SCREEN_VOICEPHONE
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
