//! Port of source/interface/remoteinterface/useridscreen_interface.h + source/interface/remoteinterface/useridscreen_interface.cpp

use uplink_engine::{gl, sound, text};

use crate::app::globals::cheatmodes_enabled;
use crate::app::opengl::screen_h;
use crate::app::opengl_interface::{
    border_draw, button_assignbitmap, button_assignbitmaps, button_click, button_highlight, imagebutton_draw,
    set_colour, text_draw, textbutton_draw,
};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::remoteinterface::remoteinterface::{RemoteInterface, SCREEN_USERIDSCREEN};
use crate::interface::remoteinterface::remoteinterfacescreen::{draw_main_title, draw_sub_title};
use crate::interface::taskmanager::taskmanager::TaskManager;
use crate::objref::{ObjRef, ScreenRef};
use crate::world::computer::computerscreen::useridscreen::UserIDScreen;
use crate::world::computer::logbank::{AccessLog, LOG_NOTSUSPICIOUS, LOG_TYPE_TEXT};
use crate::world::computer::recordbank::RECORDBANK_SECURITY;
use crate::world::connection::Connection;

/// The `ScreenRef` of the user ID screen currently in the remote interface's slot.
fn current_cs(g: &Ctx) -> Option<ScreenRef> {
    g.interface().ri.screen.get::<UserIDScreenInterface>().and_then(|s| s.cs.clone())
}

/// The computer name and next page of the user ID screen at `cs` (C++ `GetComputerScreen()` + `UplinkAssert`).
fn user_id_info(g: &Ctx, cs: &ScreenRef) -> Option<(String, i32)> {
    let uid = g.world().get_computer(&cs.computer)?.get_computer_screen(cs.index)?.as_user_id_screen()?;
    Some((uid.base.computer.clone(), uid.nextpage))
}

/// `GetPlayer()->GetConnection()->GetGhost()`. C++ passes NULL when there is no ghost; "" is used instead.
fn ghost_ip(g: &Ctx) -> String {
    g.player().connection.get_ghost().unwrap_or("").to_string()
}

/// C++ `sscanf(text, "%d", &security)`: the leading integer of `text`, or 0 if there is none.
fn leading_int(text: &str) -> i32 {
    let t = text.trim_start();
    let end = t
        .char_indices()
        .find(|&(i, c)| !(c.is_ascii_digit() || (i == 0 && (c == '-' || c == '+'))))
        .map(|(i, _)| i)
        .unwrap_or(t.len());
    t[..end].parse().unwrap_or(0)
}

/// The dark password-box background shared by the user ID and code boxes.
fn draw_password_box_background(g: &mut Ctx, button: &Button) {
    let screenheight = screen_h();
    gl::scissor(button.x, screenheight - (button.y + button.height), button.width, button.height);
    gl::enable(gl::SCISSOR_TEST);

    set_colour(g, "PasswordBoxBackground");

    gl::begin(gl::QUADS);
    gl::vertex2i(button.x, button.y);
    gl::vertex2i(button.x + button.width - 1, button.y);
    gl::vertex2i(button.x + button.width - 1, button.y + button.height);
    gl::vertex2i(button.x, button.y + button.height);
    gl::end();
}

/// Remote user ID screen: the player types a user ID and access code to proceed.
#[derive(Debug, Default)]
pub struct UserIDScreenInterface {
    pub cs: Option<ScreenRef>,
}

impl UserIDScreenInterface {
    /// `GetComputerScreen` (typed accessor; the C++ casts `cs`).
    pub fn get_computer_screen<'a>(&self, g: &'a Ctx) -> Option<&'a UserIDScreen> {
        let cs = self.cs.as_ref()?;
        g.world().get_computer(&cs.computer)?.get_computer_screen(cs.index)?.as_user_id_screen()
    }

    /// `UserIDButtonDraw` (static DrawFn)
    pub fn user_id_button_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        draw_password_box_background(g, button);

        set_colour(g, "DefaultText");
        text_draw(g, button, highlighted, clicked);

        // Draw a border
        if highlighted || clicked {
            border_draw(button);
        }

        gl::disable(gl::SCISSOR_TEST);
    }

    /// `CodeButtonDraw` (static DrawFn)
    pub fn code_button_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        draw_password_box_background(g, button);

        // Print the text as one '*' per byte of the caption
        set_colour(g, "DefaultText");
        let stars = "*".repeat(button.caption.len());
        text::draw_text_styled(button.x + 10, button.y + 10, &stars, text::BITMAP_15);

        // Draw a box around the text if highlighted
        if highlighted || clicked {
            border_draw(button);
        }

        gl::disable(gl::SCISSOR_TEST);
    }

    /// `CodeButtonClick` (static MouseFn)
    pub fn code_button_click(g: &mut Ctx, _button: &Button) {
        let target = current_cs(g).map_or(ObjRef::None, ObjRef::ComputerScreen);
        TaskManager::set_program_target(g, target, "useridscreen_code", -1);
    }

    /// `AccessCodeClick` (static MouseFn)
    pub fn access_code_click(g: &mut Ctx, button: &Button) {
        // Only 2 varieties are permitted, names and codes are always enclosed in ':
        //   1. ... 'name' .... 'code'
        //   2. ... 'code' ....
        let parts: Vec<&str> = button.caption.split('\'').collect();
        match parts.len() - 1 {
            2 => {
                g.ecl.register_caption_change_auto("useridscreen_name", "admin", None);
                g.ecl.register_caption_change_auto("useridscreen_code", parts[1], None);
            }
            4 => {
                g.ecl.register_caption_change_auto("useridscreen_name", parts[1], None);
                g.ecl.register_caption_change_auto("useridscreen_code", parts[3], None);
            }
            _ => eprintln!("Tried to use an invalid code"),
        }
    }

    /// `BypassClick` (static MouseFn)
    pub fn bypass_click(g: &mut Ctx, _button: &Button) {
        // Give admin access
        g.interface_mut().ri.set_security("Admin", 1);

        // Add this into the computer's logs
        let ghost = ghost_ip(g);
        let mut log = AccessLog::new();
        log.set_properties(&g.world().date, &ghost, "PLAYER", LOG_NOTSUSPICIOUS, LOG_TYPE_TEXT);
        log.set_data1(Some("User [Admin] logged on (level 1)"));

        let Some(cs) = current_cs(g) else { return };
        let Some((computer_name, nextpage)) = user_id_info(g, &cs) else { return };
        if let Some(comp) = g.world_mut().get_computer_mut(&computer_name) {
            comp.logbank.add_log(log, -1);
        }

        // Start tracing: disabled in the C++ source

        // Run the next screen
        if nextpage != -1 {
            RemoteInterface::run_screen(g, nextpage);
        }
    }

    /// `ProceedClick` (static MouseFn)
    pub fn proceed_click(g: &mut Ctx, _button: &Button) {
        let cs = current_cs(g);
        Self::proceed(g, cs);
    }

    /// The body of `ProceedClick`, for the user ID screen at `cs`.
    fn proceed(g: &mut Ctx, cs: Option<ScreenRef>) {
        let Some(cs) = cs else { return };
        let Some((computer_name, nextpage)) = user_id_info(g, &cs) else { return };
        let Some(name) = g.ecl.get_button("useridscreen_name").map(|b| b.caption.clone()) else { return };
        let Some(code) = g.ecl.get_button("useridscreen_code").map(|b| b.caption.clone()) else { return };

        // Look up that code in this computer's records
        let record_security = g
            .world()
            .get_computer(&computer_name)
            .and_then(|comp| comp.recordbank.get_record_from_name_password(&name, &code))
            .map(|rec| rec.get_field(RECORDBANK_SECURITY).map(str::to_string));
        let Some(record_security) = record_security else {
            // If we get this far then the access code is not valid
            g.ecl.register_caption_change_auto("useridscreen_message", "Invalid Name or Code", None);
            return;
        };

        // This is our code
        sound::play_sound("sounds/login.wav", None, false);

        let security = match record_security {
            // C++ leaves `security` unset when the field is not a number; 0 is used here.
            Some(text) => leading_int(&text),
            None => 10,
        };
        g.interface_mut().ri.set_security(&name, security);

        // Add this into the computer's logs
        let ghost = ghost_ip(g);
        let logdetails = format!("User [{name}] logged on (level {security})");
        let mut log = AccessLog::new();
        log.set_properties(&g.world().date, &ghost, "PLAYER", LOG_NOTSUSPICIOUS, LOG_TYPE_TEXT);
        log.set_data1(Some(&logdetails));
        if let Some(comp) = g.world_mut().get_computer_mut(&computer_name) {
            comp.logbank.add_log(log, -1);
        }

        if name == "admin" {
            Connection::begin_trace(g, "PLAYER"); // Always trace admin access
        }

        if nextpage != -1 {
            RemoteInterface::run_screen(g, nextpage);
        }
    }
}

impl InterfaceScreen for UserIDScreenInterface {
    crate::impl_as_any!();

    fn create(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            let Some(uid) = self.get_computer_screen(g) else { return };
            let maintitle = uid.base.maintitle.clone();
            let subtitle = uid.base.subtitle.clone();
            let ip = g.world().get_computer(&uid.base.computer).map(|c| c.ip.clone()).unwrap_or_default();

            g.ecl.register_button(130, 90, 320, 25, &maintitle, "", "useridscreen_maintitle");
            g.ecl.register_button_callbacks("useridscreen_maintitle", Some(draw_main_title), None, None, None);
            g.ecl.register_button(130, 110, 320, 20, &subtitle, "", "useridscreen_subtitle");
            g.ecl.register_button_callbacks("useridscreen_subtitle", Some(draw_sub_title), None, None, None);

            g.ecl.register_button(168, 155, 220, 120, "", "", "useridscreen_image");
            button_assignbitmap(g, "useridscreen_image", "userid.tif");
            g.ecl.register_button_callbacks(
                "useridscreen_image",
                Some(imagebutton_draw),
                Some(Self::code_button_click),
                Some(button_click),
                Some(button_highlight),
            );

            g.ecl.register_button(227, 210, 148, 15, "", "Enter your userID here", "useridscreen_name");
            g.ecl.register_button_callbacks(
                "useridscreen_name",
                Some(Self::user_id_button_draw),
                Some(Self::code_button_click),
                Some(button_click),
                Some(button_highlight),
            );

            g.ecl.register_button(227, 236, 148, 15, "", "Enter your access code here", "useridscreen_code");
            g.ecl.register_button_callbacks(
                "useridscreen_code",
                Some(Self::code_button_draw),
                Some(Self::code_button_click),
                Some(button_click),
                Some(button_highlight),
            );

            g.ecl.register_button(168, 280, 120, 15, "", "", "useridscreen_message");
            g.ecl.register_button_callbacks("useridscreen_message", Some(textbutton_draw), None, None, None);

            g.ecl.register_button(308, 280, 80, 15, "Proceed", "Click here when done", "useridscreen_proceed");
            button_assignbitmaps(g, "useridscreen_proceed", "proceed.tif", "proceed_h.tif", "proceed_c.tif");
            g.ecl.register_button_callback("useridscreen_proceed", Self::proceed_click);

            g.ecl.make_button_editable("useridscreen_name");
            g.ecl.make_button_editable("useridscreen_code");

            // Create the box that will show the currently known codes for this screen
            if g.player().agent().codes.contains_key(&ip) {
                g.ecl.register_button(200, 310, 250, 15, "Known Access Codes", "", "useridscreen_codestitle");
                g.ecl.register_button_callbacks("useridscreen_codestitle", Some(textbutton_draw), None, None, None);

                // Every code for this ip (the BTree allows duplicate keys), in pre-order
                let codes: Vec<String> = g
                    .player()
                    .agent()
                    .codes
                    .iter()
                    .filter(|(codeip, _)| *codeip == ip)
                    .map(|(_, code)| code.clone())
                    .collect();

                for (currentcode, code) in codes.iter().enumerate() {
                    let name = format!("useridscreen_code {}", currentcode);
                    g.ecl.register_button(200, 330 + currentcode as i32 * 15, 250, 15, code, "Use this code", &name);
                    g.ecl.register_button_callbacks(
                        &name,
                        Some(textbutton_draw),
                        Some(Self::access_code_click),
                        Some(button_click),
                        Some(button_highlight),
                    );
                }
            }

            if cheatmodes_enabled() {
                // Create a symbol for quick entry into this screen
                g.ecl.register_button(
                    3,
                    20,
                    30,
                    15,
                    "#",
                    "Click here to bypass this screen (DEBUG MODE ONLY)",
                    "useridscreen_bypass",
                );
                g.ecl.register_button_callbacks(
                    "useridscreen_bypass",
                    Some(textbutton_draw),
                    Some(Self::bypass_click),
                    Some(button_click),
                    Some(button_highlight),
                );
            }
        }
    }

    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            g.ecl.remove_button("useridscreen_maintitle");
            g.ecl.remove_button("useridscreen_subtitle");

            g.ecl.remove_button("useridscreen_image");
            g.ecl.remove_button("useridscreen_name");
            g.ecl.remove_button("useridscreen_code");

            g.ecl.remove_button("useridscreen_message");
            g.ecl.remove_button("useridscreen_proceed");

            g.ecl.remove_button("useridscreen_codestitle");

            let mut currentcode = 0;
            let mut name = format!("useridscreen_code {currentcode}");
            while g.ecl.get_button(&name).is_some() {
                g.ecl.remove_button(&name);
                currentcode += 1;
                name = format!("useridscreen_code {currentcode}");
            }

            if cheatmodes_enabled() {
                g.ecl.remove_button("useridscreen_bypass");
            }
        }
    }

    fn update(&mut self, _g: &mut Ctx) {}

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("useridscreen_image").is_some()
    }

    fn screen_id(&self) -> i32 {
        SCREEN_USERIDSCREEN
    }

    /// Calls `proceed_click` in the C++ source.
    fn return_key_pressed(&mut self, g: &mut Ctx) -> bool {
        // The screen is taken out of its slot while this runs, so use our own `cs`.
        Self::proceed(g, self.cs.clone());
        true
    }

    fn set_computer_screen(&mut self, cs: ScreenRef) {
        self.cs = Some(cs);
    }

    fn computer_screen_ref(&self) -> Option<&ScreenRef> {
        self.cs.as_ref()
    }
}
