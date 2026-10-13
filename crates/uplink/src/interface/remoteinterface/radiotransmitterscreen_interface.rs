//! Port of source/interface/remoteinterface/radiotransmitterscreen_interface.h + source/interface/remoteinterface/radiotransmitterscreen_interface.cpp

use std::cell::Cell;

use uplink_engine::{gl, text};

use crate::app::opengl_interface;
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::game::data::data::{RADIOTRANSMITTER_MAXRANGE, RADIOTRANSMITTER_MINRANGE};
use crate::game::scriptlibrary::ScriptLibrary;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::localinterface::localinterface::{LocalInterface, SCREEN_NONE};
use crate::interface::remoteinterface::remoteinterface::{RemoteInterface, SCREEN_RADIOTRANSMITTER};
use crate::interface::remoteinterface::remoteinterfacescreen::{draw_main_title, draw_sub_title};
use crate::objref::ScreenRef;
use crate::world::computer::computer::COMPUTER_TYPE_LAN;
use crate::world::computer::computerscreen::computerscreen::ComputerScreen;
use crate::world::computer::computerscreen::genericscreen::GenericScreen;
use crate::world::computer::lancomputer::LANSYSTEM_RADIORECEIVER;
use crate::world::computer::lanmonitor::LanMonitor;

thread_local! {
    /// C++ `static int frequencyGhz = 160; static int frequencyMhz = 0;` -- shared by all instances
    /// and kept when the screen is closed and opened again.
    static FREQUENCY: Cell<(i32, i32)> = const { Cell::new((160, 0)) };
}

fn frequency() -> (i32, i32) {
    FREQUENCY.with(|f| f.get())
}

fn set_frequency(ghz: i32, mhz: i32) {
    FREQUENCY.with(|f| f.set((ghz, mhz)));
}

/// Remote radio transmitter screen: tune a frequency and connect to the transmitter.
#[derive(Debug, Default)]
pub struct RadioTransmitterScreenInterface {
    pub cs: Option<ScreenRef>,
}

impl RadioTransmitterScreenInterface {
    /// `GetComputerScreen` (typed accessor; the C++ casts `cs`).
    pub fn get_computer_screen<'a>(&self, g: &'a Ctx) -> Option<&'a GenericScreen> {
        let cs = self.cs.as_ref()?;
        match g.world().get_computer(&cs.computer)?.get_computer_screen(cs.index)? {
            ComputerScreen::GenericScreen(gs) => Some(gs),
            _ => None,
        }
    }

    /// C++ `Connect()`: searches the remote host's LAN for a radio receiver at the tuned frequency,
    /// connects to it and shows its attached screen. Called by connect script 85.
    pub fn connect(&mut self, g: &mut Ctx) {
        // Search for a radio receiver at our frequency

        let remotehost = g.player().remotehost.clone();
        let (ghz, mhz) = frequency();

        let matches: Vec<i32> = match g
            .world()
            .get_vlocation(&remotehost)
            .and_then(|vl| vl.get_computer(g.world()))
        {
            Some(comp) if comp.type_ == COMPUTER_TYPE_LAN => match comp.lan() {
                Some(lc) => (0..lc.systems.size())
                    .filter(|&i| {
                        lc.systems.get_data(i).is_some_and(|s| {
                            s.type_ == LANSYSTEM_RADIORECEIVER && s.data1 == ghz && s.data2 == mhz
                        })
                    })
                    .collect(),
                None => Vec::new(),
            },
            _ => Vec::new(),
        };

        for i in matches {
            // Found our target system

            LanMonitor::force_extend_connection(g, i);
            LanMonitor::set_current_system(g, i);

            // Show the screen attached to this system

            let screen_index = g
                .world()
                .get_vlocation(&remotehost)
                .and_then(|vl| vl.get_computer(g.world()))
                .and_then(|comp| comp.lan())
                .and_then(|lc| lc.systems.get_data(i))
                .map(|s| s.screen_index);

            if let Some(screen_index) = screen_index {
                if screen_index != -1 {
                    RemoteInterface::run_screen(g, screen_index);
                    LocalInterface::run_screen(g, SCREEN_NONE, -1);
                    return;
                }
            }
        }

        // If we get here then our connection attempt failed

        g.ecl.register_caption_change("radioscreen_statustext", "Not Connected", 0, None);
    }

    /// `BackgroundDraw` (static DrawFn)
    pub fn background_draw(g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        gl::color3f(0.0, 0.0, 0.0);

        gl::begin(gl::QUADS);
        gl::vertex2i(button.x, button.y + button.height);
        gl::vertex2i(button.x, button.y);
        gl::vertex2i(button.x + button.width, button.y);
        gl::vertex2i(button.x + button.width, button.y + button.height);
        gl::end();

        opengl_interface::set_colour(g, "PanelBorder");
        opengl_interface::border_draw(button);
    }

    /// `FrequencyDraw` (static DrawFn)
    pub fn frequency_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        opengl_interface::textbutton_draw(g, button, highlighted, clicked);
        opengl_interface::set_colour(g, "PanelBorder");
        opengl_interface::border_draw(button);

        let (ghz, mhz) = frequency();
        let frequency = format!("{}.{} Ghz", ghz, mhz);

        text::draw_text_default(button.x + 10, button.y + 10, &frequency);
    }

    /// `DownClick` (static MouseFn)
    pub fn down_click(g: &mut Ctx, _button: &Button) {
        let (mut ghz, mut mhz) = frequency();

        mhz -= 1;

        if mhz < 0 {
            mhz = 9;
            ghz -= 1;
        }

        if ghz < RADIOTRANSMITTER_MINRANGE {
            ghz = RADIOTRANSMITTER_MINRANGE;
            mhz = 0;
        }

        set_frequency(ghz, mhz);

        g.ecl.dirty_button("radioscreen_frequency");
    }

    /// `BigDownClick` (static MouseFn)
    pub fn big_down_click(g: &mut Ctx, _button: &Button) {
        let (mut ghz, mut mhz) = frequency();

        ghz -= 1;

        if ghz < RADIOTRANSMITTER_MINRANGE {
            ghz = RADIOTRANSMITTER_MINRANGE;
            mhz = 0;
        }

        set_frequency(ghz, mhz);

        g.ecl.dirty_button("radioscreen_frequency");
    }

    /// `UpClick` (static MouseFn)
    pub fn up_click(g: &mut Ctx, _button: &Button) {
        let (mut ghz, mut mhz) = frequency();

        mhz += 1;

        if mhz > 9 {
            mhz = 0;
            ghz += 1;
        }

        if ghz > RADIOTRANSMITTER_MAXRANGE {
            ghz = RADIOTRANSMITTER_MAXRANGE;
            mhz = 9;
        }

        set_frequency(ghz, mhz);

        g.ecl.dirty_button("radioscreen_frequency");
    }

    /// `BigUpClick` (static MouseFn)
    pub fn big_up_click(g: &mut Ctx, _button: &Button) {
        let (mut ghz, mut mhz) = frequency();

        ghz += 1;

        if ghz > RADIOTRANSMITTER_MAXRANGE {
            ghz = RADIOTRANSMITTER_MAXRANGE;
            mhz = 9;
        }

        set_frequency(ghz, mhz);

        g.ecl.dirty_button("radioscreen_frequency");
    }

    /// `ConnectClick` (static MouseFn)
    pub fn connect_click(g: &mut Ctx, _button: &Button) {
        ScriptLibrary::run_script(g, 85);
    }
}

impl InterfaceScreen for RadioTransmitterScreenInterface {
    crate::impl_as_any!();

    fn create(&mut self, g: &mut Ctx) {
        // C++ `Create ()` (no argument) warns and does nothing when there is no screen.
        if self.cs.is_none() {
            println!("RadioTransmitterScreenInterface::Create, tried to create when GenericScreen==NULL");
            return;
        }

        if self.is_visible(g) {
            return;
        }

        let Some((maintitle, subtitle)) = self
            .get_computer_screen(g)
            .map(|gs| (gs.base.maintitle.clone(), gs.base.subtitle.clone()))
        else {
            return;
        };

        g.ecl.register_button(80, 60, 350, 25, &maintitle, "", "radioscreen_maintitle");
        g.ecl.register_button_callbacks("radioscreen_maintitle", Some(draw_main_title), None, None, None);
        g.ecl.register_button(80, 80, 350, 20, &subtitle, "", "radioscreen_subtitle");
        g.ecl.register_button_callbacks("radioscreen_subtitle", Some(draw_sub_title), None, None, None);

        g.ecl.register_button(
            100,
            120,
            300,
            100,
            "Wireless LAN transmitter online\nReady to establish Uplink.",
            " ",
            "radioscreen_text",
        );
        g.ecl.register_button_callbacks("radioscreen_text", Some(opengl_interface::textbutton_draw), None, None, None);

        //
        // Frequency selection dialog

        g.ecl.register_button(160, 220, 220, 140, " ", " ", "radioscreen_background");
        g.ecl.register_button_callbacks("radioscreen_background", Some(Self::background_draw), None, None, None);

        g.ecl.register_button(170, 223, 220, 30, "Transmitter Frequency", " ", "radioscreen_bigtext");
        g.ecl.register_button_callbacks("radioscreen_bigtext", Some(draw_main_title), None, None, None);

        g.ecl.register_button(220, 260, 100, 20, " ", "Enter desired frequency here", "radioscreen_frequency");
        g.ecl.register_button_callbacks("radioscreen_frequency", Some(Self::frequency_draw), None, None, None);
        g.ecl.make_button_editable("radioscreen_frequency");

        g.ecl.register_button(200, 260, 20, 20, "<", "Scan lower frequencies", "radioscreen_down");
        g.ecl.register_button(180, 260, 20, 20, "<<", "Scan much lower frequencies", "radioscreen_bigdown");
        g.ecl.register_button(320, 260, 20, 20, ">", "Scan higher frequencies", "radioscreen_up");
        g.ecl.register_button(340, 260, 20, 20, ">>", "Scan much higher frequencies", "radioscreen_bigup");

        g.ecl.register_button_callback("radioscreen_bigdown", Self::big_down_click);
        g.ecl.register_button_callback("radioscreen_down", Self::down_click);
        g.ecl.register_button_callback("radioscreen_bigup", Self::big_up_click);
        g.ecl.register_button_callback("radioscreen_up", Self::up_click);

        g.ecl.register_button(180, 300, 180, 15, "Establish uplink", "Click to establish the radio link", "radioscreen_connect");
        g.ecl.register_button_callback("radioscreen_connect", Self::connect_click);

        g.ecl.register_button(180, 320, 180, 15, "Not Connected", " ", "radioscreen_statustext");
        g.ecl.register_button_callbacks("radioscreen_statustext", Some(opengl_interface::textbutton_draw), None, None, None);
    }

    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            g.ecl.remove_button("radioscreen_maintitle");
            g.ecl.remove_button("radioscreen_subtitle");

            g.ecl.remove_button("radioscreen_background");
            g.ecl.remove_button("radioscreen_bigtext");
            g.ecl.remove_button("radioscreen_text");
            g.ecl.remove_button("radioscreen_down");
            g.ecl.remove_button("radioscreen_bigdown");
            g.ecl.remove_button("radioscreen_frequency");
            g.ecl.remove_button("radioscreen_up");
            g.ecl.remove_button("radioscreen_bigup");

            g.ecl.remove_button("radioscreen_connect");
            g.ecl.remove_button("radioscreen_statustext");
        }
    }

    fn update(&mut self, _g: &mut Ctx) {}

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("radioscreen_maintitle").is_some()
    }

    fn screen_id(&self) -> i32 {
        SCREEN_RADIOTRANSMITTER
    }

    fn return_key_pressed(&mut self, g: &mut Ctx) -> bool {
        // C++: ConnectClick ( NULL ); return true;
        ScriptLibrary::run_script(g, 85);
        true
    }

    fn set_computer_screen(&mut self, cs: ScreenRef) {
        self.cs = Some(cs);
    }

    fn computer_screen_ref(&self) -> Option<&ScreenRef> {
        self.cs.as_ref()
    }
}
