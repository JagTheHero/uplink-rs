//! Port of source/interface/localinterface/phonedialler.h + source/interface/localinterface/phonedialler.cpp
//! Phone dialler pop-up: a keypad that "dials" an IP by lighting each digit and playing its tone.
//! Created on demand by the finance screen (`pd.Create(100, 100)`), not a slot screen.

use uplink_engine::sound;

use crate::app::opengl::display;
use crate::app::opengl_interface::button_assignbitmaps;
use crate::ctx::Ctx;
use crate::impl_as_any;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::localinterface::localinterfacescreen::{self, background_draw};

/// C++ `static char buttonnames[][9]`: button name for digit 1..9 (index = digit - 1).
// PORT: a constant table, so a const rather than a static field.
pub const BUTTON_NAMES: [&str; 9] = [
    "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
];

#[derive(Debug, Clone, Default)]
pub struct PhoneDialler {}

impl PhoneDialler {
    /// C++ `PhoneDialler::Create(int x, int y)`: the dialler needs a position, so it is not
    /// started through `InterfaceScreen::create`.
    pub fn create_at(&mut self, g: &mut Ctx, x: i32, y: i32) {
        if self.is_visible(g) {
            return;
        }

        g.ecl.register_button(x - 10, y - 10, 120, 140, "", " ", "dialler_bg");
        g.ecl.register_button_callbacks("dialler_bg", Some(background_draw), None, None, None);

        g.ecl.register_button(x, y, 29, 29, "1", " ", "one");
        g.ecl.register_button(x + 36, y, 29, 29, "2", " ", "two");
        g.ecl.register_button(x + 72, y, 29, 29, "3", " ", "three");
        g.ecl.register_button(x, y + 36, 29, 29, "4", " ", "four");
        g.ecl.register_button(x + 36, y + 36, 29, 29, "5", " ", "five");
        g.ecl.register_button(x + 72, y + 36, 29, 29, "6", " ", "six");
        g.ecl.register_button(x, y + 72, 29, 29, "7", " ", "seven");
        g.ecl.register_button(x + 36, y + 72, 29, 29, "8", " ", "eight");
        g.ecl.register_button(x + 72, y + 72, 29, 29, "9", " ", "nine");

        for name in BUTTON_NAMES {
            button_assignbitmaps(
                g,
                name,
                &format!("dialler/{name}.tif"),
                &format!("dialler/{name}_h.tif"),
                &format!("dialler/{name}_h.tif"),
            );
        }

        g.ecl.register_button(x, y + 108, 101, 15, "", " ", "dialler_number");
    }

    /// C++ `PhoneDialler::DialNumber(char *number)`: lights each digit and plays its tone.
    pub fn dial_number(&mut self, g: &mut Ctx, number: &str) {
        if !self.is_visible(g) {
            return;
        }

        let bytes = number.as_bytes();
        for (i, &ch) in bytes.iter().enumerate() {
            if (b'1'..=b'9').contains(&ch) {
                g.ecl.highlight_button(BUTTON_NAMES[(ch - b'1') as usize]);

                // C++ `sprintf(caption, "%s", number); caption[i+1] = 0;`
                let caption = String::from_utf8_lossy(&bytes[..=i]).into_owned();
                let dialler_number = g.ecl.get_button_mut("dialler_number");
                debug_assert!(dialler_number.is_some(), "UplinkAssert(dialler_number)");
                if let Some(b) = dialler_number {
                    b.set_caption(&caption);
                }
                display(g);

                let filename = format!("sounds/{}.wav", ch - b'0');
                sound::play_sound(&filename, None, true);

                // Pause for a small time
                let starttime = g.ecl.get_accurate_time();
                while g.ecl.get_accurate_time() - starttime < 110 {}
            }
        }
    }
}

impl InterfaceScreen for PhoneDialler {
    /// C++ `PhoneDialler::Create(int x, int y)` hides, but does not override, the virtual `Create()`,
    /// so the virtual one is still `LocalInterfaceScreen::Create` (the panel background, guarded
    /// by this class's `IsVisible`). The real entry point is [`PhoneDialler::create_at`].
    fn create(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            localinterfacescreen::create(g);
        }
    }

    fn remove(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            return;
        }

        g.ecl.remove_button("dialler_bg");
        for name in BUTTON_NAMES {
            g.ecl.remove_button(name);
        }
        g.ecl.remove_button("dialler_number");
    }

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("dialler_number").is_some()
    }

    impl_as_any!();
}
