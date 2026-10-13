//! Port of source/interface/remoteinterface/passwordscreen_interface.h + source/interface/remoteinterface/passwordscreen_interface.cpp

use uplink_engine::{gl, sound, text};

use crate::app::globals;
use crate::app::opengl_interface;
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::remoteinterface::remoteinterface::{RemoteInterface, SCREEN_PASSWORDSCREEN};
use crate::interface::remoteinterface::remoteinterfacescreen::{draw_main_title, draw_sub_title};
use crate::interface::taskmanager::taskmanager::TaskManager;
use crate::objref::{ObjRef, ScreenRef};
use crate::world::computer::computerscreen::passwordscreen::PasswordScreen;
use crate::world::computer::logbank::{AccessLog, LOG_NOTSUSPICIOUS, LOG_TYPE_TEXT};

/// Remote password screen: the player types the computer's password or bypasses it.
#[derive(Debug, Default)]
pub struct PasswordScreenInterface {
    pub cs: Option<ScreenRef>,
}

/// Typed lookup of the password screen `cs` refers to.
fn lookup_screen<'a>(g: &'a Ctx, cs: &ScreenRef) -> Option<&'a PasswordScreen> {
    g.world().get_computer(&cs.computer)?.get_computer_screen(cs.index)?.as_password_screen()
}

/// `PasswordScreenInterface::NextPage` for the screen `cs` refers to (static callbacks
/// cannot borrow the interface object itself, so the logic works from its `ScreenRef`).
fn next_page_of(g: &mut Ctx, cs: &ScreenRef) {
    // Give the user max security clearance (only 1 access code for password screen)

    g.interface_mut().ri.set_security("Admin", 1);

    // Add this into the computer's logs

    let ghost = g.player().get_connection().get_ghost().unwrap_or(" ").to_string();
    let mut log = AccessLog::new();
    log.set_properties(&g.world().date.clone(), &ghost, "PLAYER", LOG_NOTSUSPICIOUS, LOG_TYPE_TEXT);
    log.set_data1(Some("Password authentication accepted"));
    if let Some(comp) = g.world_mut().get_computer_mut(&cs.computer) {
        comp.logbank.add_log(log, -1);
    }

    let Some(nextpage) = lookup_screen(g, cs).map(|ps| ps.nextpage) else {
        return;
    };
    if nextpage != -1 {
        RemoteInterface::run_screen(g, nextpage);
    }
}

impl PasswordScreenInterface {
    /// `GetComputerScreen` (typed accessor; the C++ casts `cs`).
    pub fn get_computer_screen<'a>(&self, g: &'a Ctx) -> Option<&'a PasswordScreen> {
        lookup_screen(g, self.cs.as_ref()?)
    }

    /// `NextPage`: moves on to the screen the password unlocks.
    pub fn next_page(&mut self, g: &mut Ctx) {
        let Some(cs) = self.cs.clone() else {
            return;
        };
        next_page_of(g, &cs);
    }

    /// `CodeButtonDraw` (static DrawFn)
    pub fn code_button_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        // Draw a background colour

        opengl_interface::set_colour(g, "PasswordBoxBackground");

        gl::begin(gl::QUADS);

        gl::vertex2i(button.x, button.y);
        gl::vertex2i(button.x + button.width - 1, button.y);
        gl::vertex2i(button.x + button.width - 1, button.y + button.height);
        gl::vertex2i(button.x, button.y + button.height);

        gl::end();

        // Print the text

        opengl_interface::set_colour(g, "DefaultText");

        let caption = "*".repeat(button.caption.len());
        text::draw_text_styled(button.x + 10, button.y + 10, &caption, text::BITMAP_15);

        // Draw a box around the text if highlighted

        if highlighted || clicked {
            gl::begin(gl::LINE_LOOP);

            gl::vertex2i(button.x, button.y);
            gl::vertex2i(button.x + button.width, button.y);
            gl::vertex2i(button.x + button.width, button.y + button.height);
            gl::vertex2i(button.x, button.y + button.height);

            gl::end();
        }
    }

    /// `CursorFlash` (AnimCallback)
    pub fn cursor_flash(g: &mut Ctx) {
        let Some(caption) = g.ecl.get_button("passwordscreen_password").map(|b| b.caption.clone()) else {
            return;
        };

        if caption == "_" {
            g.ecl.register_caption_change("passwordscreen_password", " ", 1000, Some(Self::cursor_flash));
        } else if caption == " " {
            g.ecl.register_caption_change("passwordscreen_password", "_", 1000, Some(Self::cursor_flash));
        }
    }

    /// `PasswordClick` (static MouseFn)
    pub fn password_click(g: &mut Ctx, button: &Button) {
        let Some(cs) = g.interface().ri.screen.get::<Self>().and_then(|ps| ps.cs.clone()) else {
            return;
        };

        TaskManager::set_program_target(g, ObjRef::ComputerScreen(cs), &button.name, -1);
    }

    /// `BypassClick` (static MouseFn)
    pub fn bypass_click(g: &mut Ctx, _button: &Button) {
        let Some(cs) = g.interface().ri.screen.get::<Self>().and_then(|ps| ps.cs.clone()) else {
            return;
        };

        next_page_of(g, &cs);
    }
}

impl InterfaceScreen for PasswordScreenInterface {
    crate::impl_as_any!();

    fn create(&mut self, g: &mut Ctx) {
        let Some(cs) = self.cs.clone() else {
            return;
        };

        if !self.is_visible(g) {
            let Some(ps) = lookup_screen(g, &cs) else {
                debug_assert!(false, "PasswordScreenInterface::create, no PasswordScreen");
                return;
            };
            let maintitle = ps.base.maintitle.clone();
            let subtitle = ps.base.subtitle.clone();

            g.ecl.register_button(150, 120, 350, 25, &maintitle, "", "passwordscreen_maintitle");
            g.ecl.register_button_callbacks("passwordscreen_maintitle", Some(draw_main_title), None, None, None);
            g.ecl.register_button(150, 140, 350, 20, &subtitle, "", "passwordscreen_subtitle");
            g.ecl.register_button_callbacks("passwordscreen_subtitle", Some(draw_sub_title), None, None, None);

            g.ecl.register_button(168, 185, 220, 110, "", "", "passwordscreen_message");
            opengl_interface::button_assignbitmap(g, "passwordscreen_message", "passwordscreen.tif");

            g.ecl.register_button(206, 251, 145, 14, "", "Target this password box", "passwordscreen_password");
            g.ecl.register_button_callbacks(
                "passwordscreen_password",
                Some(Self::code_button_draw),
                Some(Self::password_click),
                Some(opengl_interface::button_click),
                Some(opengl_interface::button_highlight),
            );

            g.ecl.make_button_editable("passwordscreen_password");

            // Create the box that will show the currently known codes for this screen
            debug_assert!(g.world().get_computer(&cs.computer).is_some());
            let ip = g.world().get_computer(&cs.computer).map(|c| c.ip.clone()).unwrap_or_default();
            let code = g
                .player()
                .agent
                .as_ref()
                .and_then(|agent| agent.codes.get(&ip))
                .cloned();

            if let Some(code) = code {
                g.ecl.register_button(200, 315, 180, 130, "", "", "passwordscreen_codes");
                g.ecl.register_button_callbacks("passwordscreen_codes", Some(opengl_interface::textbutton_draw), None, None, None);

                let message = format!("KNOWN ACCESS CODES\n\t{}", code);
                g.ecl.register_caption_change("passwordscreen_codes", &message, 1500, None);
            }

            // CHEATMODES_ENABLED: create a symbol for quick entry into this screen
            if globals::cheatmodes_enabled() {
                g.ecl.register_button(3, 20, 30, 15, "#", "Click here to bypass this screen (DEBUG MODE ONLY)", "passwordscreen_bypass");
                g.ecl.register_button_callbacks(
                    "passwordscreen_bypass",
                    Some(opengl_interface::textbutton_draw),
                    Some(Self::bypass_click),
                    Some(opengl_interface::button_click),
                    Some(opengl_interface::button_highlight),
                );
            }
        }
    }

    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            g.ecl.remove_button("passwordscreen_maintitle");
            g.ecl.remove_button("passwordscreen_subtitle");
            g.ecl.remove_button("passwordscreen_message");
            g.ecl.remove_button("passwordscreen_password");
            g.ecl.remove_button("passwordscreen_codes");

            if globals::cheatmodes_enabled() {
                g.ecl.remove_button("passwordscreen_bypass");
            }
        }
    }

    fn update(&mut self, g: &mut Ctx) {
        let Some(caption) = g.ecl.get_button("passwordscreen_password").map(|b| b.caption.clone()) else {
            return;
        };
        let Some(password) = self.get_computer_screen(g).map(|ps| ps.password.clone()) else {
            return;
        };

        if caption == password {
            sound::play_sound("sounds/login.wav", None, true);
            self.next_page(g);
        }
    }

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("passwordscreen_message").is_some()
    }

    fn screen_id(&self) -> i32 {
        SCREEN_PASSWORDSCREEN
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
