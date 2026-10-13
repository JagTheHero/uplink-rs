//! Port of source/interface/taskmanager/passwordbreaker.h + source/interface/taskmanager/passwordbreaker.cpp

use uplink_engine::gl;
use uplink_engine::sound::play_sound;
use uplink_engine::text::{self, BITMAP_15, HELVETICA_12};

use crate::app::miscutils::{rand_i32, RAND_MAX};
use crate::app::opengl_interface::{border_draw, button_assignbitmaps, button_click, button_draw, button_highlight};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::game::data::data::{HACKDIFFICULTY_SECURITYMODIFIER, HACKDIFFICULTY_VARIANCE};
use crate::impl_as_any;
use crate::interface::taskmanager::uplinktask::{UplinkTask, UplinkTaskBase};
use crate::objref::ObjRef;
use crate::vanbakel::Vanbakel;
use crate::world::computer::computer::Computer;
use crate::world::computer::computerscreen::computerscreen::ComputerScreen;
use crate::world::computer::recordbank::{RECORDBANK_PASSWORD, RECORDBANK_SECURITY};
use crate::world::connection::Connection;
use crate::world::generator::numbergenerator::NumberGenerator;

/// C++ `static char alphabet[27]`: the 26 letters (the NUL terminator is not needed).
pub const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz";

#[derive(Debug, Default)]
pub struct PasswordBreaker {
    pub base: UplinkTaskBase,
    pub length: i32,
    pub difficulty: i32,
    pub currentchar: i32,
    pub caption: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    /// C++ `bool *found` (NULL = empty Vec).
    pub found: Vec<bool>,
    /// For next letter.
    pub numticksrequired: i32,
    pub progress: i32,
}

/// C `sscanf(token, "%d", &x)`: an optional sign and leading digits; the rest is ignored.
fn scan_leading_int(token: &str) -> Option<i32> {
    let end = token
        .char_indices()
        .find(|&(i, c)| !(c.is_ascii_digit() || (i == 0 && (c == '-' || c == '+'))))
        .map(|(i, _)| i)
        .unwrap_or(token.len());
    token[..end].parse::<i32>().ok()
}

/// C `sscanf(button->name, "%s %d", bname, &pid)`: the PID is the second token of the name.
fn button_pid(button: &Button) -> i32 {
    button.name.split_whitespace().nth(1).and_then(scan_leading_int).unwrap_or(-1)
}

/// The three buttons of the breaker: main (password) button, close button, fill bar.
fn button_names(pid: i32) -> [String; 3] {
    [
        format!("passwordbreaker {pid}"),
        format!("passwordbreaker_close {pid}"),
        format!("passwordbreaker_fill {pid}"),
    ]
}

/// One random letter, as C `alphabet[ int(26 * ((float) rand() / (float) RAND_MAX)) ]`.
/// Index 26 (only when rand() == RAND_MAX) reads the C NUL terminator, which is `'\0'` here.
fn random_letter() -> char {
    let idx = (26.0_f32 * (rand_i32() as f32 / RAND_MAX as f32)) as usize;
    ALPHABET.get(idx).copied().unwrap_or(0) as char
}

impl PasswordBreaker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn background_draw(_g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
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
        border_draw(button);
    }

    pub fn password_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        Self::background_draw(g, button, highlighted, clicked);

        let xpos = button.x + 10;
        let ypos = button.y + 18;

        gl::color4f(1.0, 1.0, 1.0, 0.8);

        // A C string ends at its first NUL character.
        let shown: String = button.caption.chars().take_while(|&c| c != '\0').collect();

        if shown != "Select target" {
            for (i, c) in shown.chars().enumerate() {
                text::draw_text_styled(xpos + (i as i32) * 25, ypos, &c.to_string(), BITMAP_15);
            }
        } else {
            text::draw_text_styled(xpos, ypos, &shown, HELVETICA_12);
        }
    }

    pub fn password_click(g: &mut Ctx, button: &Button) {
        let pid = button_pid(button);
        g.interface_mut().tm.set_target_program(pid);
    }

    pub fn close_click(g: &mut Ctx, button: &Button) {
        let pid = button_pid(button);
        Vanbakel::remove_task(g, pid);
    }
}

impl UplinkTask for PasswordBreaker {
    fn base(&self) -> &UplinkTaskBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut UplinkTaskBase {
        &mut self.base
    }

    fn set_target(&mut self, g: &mut Ctx, target: ObjRef, targetstring: &str, targetint: i32) {
        // uo  : Target PasswordScreen or UserIDScreen object
        // uos : name of button with password in
        // uoi : unused
        self.base.targetobject = target;
        self.base.targetstring = targetstring.to_string();
        self.base.targetint = targetint;

        let pid = self.base.pid;

        // Unrecognised object: nothing to break.
        let ObjRef::ComputerScreen(sr) = self.base.targetobject.clone() else { return };
        let target_computer = sr.computer.clone();

        let Some(screen) = g
            .world()
            .get_computer(&target_computer)
            .and_then(|c| c.screens.get_data(sr.index))
            .cloned()
        else {
            return;
        };

        // (length, difficulty, password, username)
        let (length, difficulty, password, username): (i32, i32, String, Option<String>) = match screen {
            ComputerScreen::PasswordScreen(ps) => (ps.password.len() as i32, ps.difficulty, ps.password, None),
            ComputerScreen::UserIDScreen(uid) => {
                // If the player has not entered a UserID then do this for admin
                let name_button = g
                    .ecl
                    .get_button_mut("useridscreen_name")
                    .expect("password breaker: useridscreen_name button");
                if name_button.caption.is_empty() {
                    name_button.set_caption("admin");
                }
                let name = name_button.caption.clone();

                // Look up the user name in this computer's records
                let fields = g
                    .world()
                    .get_computer(&target_computer)
                    .and_then(|c| c.recordbank.get_record_from_name(&name))
                    .map(|rec| {
                        (
                            rec.get_field(RECORDBANK_PASSWORD).map(str::to_string),
                            rec.get_field(RECORDBANK_SECURITY).map(str::to_string),
                        )
                    });

                match fields {
                    Some((passwd, sec)) => {
                        let passwd = passwd.expect("password breaker: record password field");
                        let sec = sec.expect("password breaker: record security field");
                        // C: sscanf(security, "%d", &security), then an index into the modifier table
                        let security = scan_leading_int(&sec).unwrap_or(0).max(0) as usize;
                        let modifier = HACKDIFFICULTY_SECURITYMODIFIER.get(security).copied().unwrap_or(0.0);
                        let difficulty = (modifier * uid.difficulty as f32) as i32;
                        (passwd.len() as i32, difficulty, passwd, Some(name))
                    }
                    None => {
                        // The user name is not recognised
                        Vanbakel::remove_task(g, pid);
                        return;
                    }
                }
            }
            // Unrecognised object
            _ => return,
        };

        // Set up the password breaker to work with this new data

        self.length = length;
        self.difficulty = difficulty;
        self.password = Some(password);
        self.username = username;

        self.caption = Some("x".repeat(length as usize));
        self.found = vec![false; length as usize];

        let [buttonname, closename, fillname] = button_names(pid);
        let width = length * 26;

        let button_x = g.ecl.get_button(&buttonname).map(|b| b.x).unwrap_or(0);
        if let Some(b) = g.ecl.get_button_mut(&buttonname) {
            b.width = width;
        }

        if let Some(b) = g.ecl.get_button_mut(&closename) {
            b.x = button_x + width - 13;
        }
        g.ecl.dirty_button(&closename);

        if let Some(b) = g.ecl.get_button_mut(&fillname) {
            b.width = width - 14;
        }
        g.ecl.dirty_button(&fillname);

        self.numticksrequired = if difficulty == -1 {
            -1
        } else {
            NumberGenerator::apply_variance(difficulty, (HACKDIFFICULTY_VARIANCE * 100.0) as i32)
        };

        self.progress = 0;

        let monitor_running = g
            .world()
            .get_computer(&target_computer)
            .expect("password breaker: target computer")
            .security
            .is_running_monitor();
        if monitor_running {
            Connection::begin_trace(g, "PLAYER");
        }
    }

    /// Called at creation time, necessary.
    fn initialise(&mut self, _g: &mut Ctx) {
        self.caption = Some("Select target".to_string());
    }

    /// n ticks occur.
    fn tick(&mut self, g: &mut Ctx, n: i32) {
        if !self.is_interface_visible(g) {
            return;
        }
        if self.length <= 0 {
            return;
        }

        let pid = self.base.pid;

        for _count in 0..n {
            // First check to make sure we are still looking at this password screen
            // i.e the player has not logged out / moved / got past it already

            if g.ecl.get_button(&self.base.targetstring).is_none() {
                // Target password box has been removed; Terminate this task
                Vanbakel::remove_task(g, pid);
                // WARNING - This instance has been deleted - do not access any more data
                return;
            }

            if self.currentchar < self.length {
                // Some of the word has not been calculated
                // Do it now

                // First see if we have calculated the current character

                self.progress += 1;

                let mut caption: Vec<char> = self.caption.as_deref().unwrap_or("").chars().collect();

                if self.numticksrequired != -1 && self.progress >= self.numticksrequired {
                    // This char has been discovered
                    let password: Vec<char> = self.password.as_deref().unwrap_or("").chars().collect();
                    let i = self.currentchar as usize;
                    if let (Some(slot), Some(&c)) = (caption.get_mut(i), password.get(i)) {
                        *slot = c;
                    }
                    if let Some(f) = self.found.get_mut(i) {
                        *f = true;
                    }
                    self.currentchar += 1;

                    play_sound("sounds/done.wav", None, true);

                    // Work out when the next one will be discovered
                    self.numticksrequired = NumberGenerator::apply_variance(
                        self.difficulty,
                        (HACKDIFFICULTY_VARIANCE * 100.0) as i32,
                    );
                    self.progress = 0;
                }

                // Now refresh the caption with random chars where appropriate
                for i in self.currentchar..self.length {
                    if let Some(slot) = caption.get_mut(i as usize) {
                        *slot = random_letter();
                    }
                }
                self.caption = Some(caption.into_iter().collect());

                if self.is_interface_visible(g) {
                    let buttonname = format!("passwordbreaker {pid}");
                    let caption = self.caption.clone().unwrap_or_default();
                    if let Some(b) = g.ecl.get_button_mut(&buttonname) {
                        b.set_caption(&caption);
                    }
                    // PORT: C++ GciPostRedisplay has no equivalent; eclipse redraws every frame.
                }
            } else {
                // The full word has been discovered - enter it into the password screen

                let password = self.password.clone().unwrap_or_default();
                g.ecl.register_caption_change_auto(&self.base.targetstring, &password, None);

                // Give the code to the player

                let code = match &self.username {
                    Some(username) => Computer::generate_access_code_name(username, &password),
                    None => Computer::generate_access_code(&password),
                };
                let remotehost = g.world().get_player().person.remotehost.clone();
                g.world_mut().get_player_mut().give_code(&remotehost, &code);

                // Remove this task

                Vanbakel::remove_task(g, pid);

                // WARNING - This instance has been deleted - do not access any more data
                return;
            }
        }
    }

    fn create_interface(&mut self, g: &mut Ctx) {
        if !self.is_interface_visible(g) {
            let pid = self.base.pid;
            let [buttonname, closename, fillname] = button_names(pid);
            let caption = self.caption.clone().unwrap_or_default();

            g.ecl.register_button(245, 375, 130, 30, &caption, "Password Breaker", &buttonname);
            g.ecl.register_button_callbacks(
                &buttonname,
                Some(PasswordBreaker::password_draw),
                Some(PasswordBreaker::password_click),
                Some(button_click),
                Some(button_highlight),
            );

            g.ecl.register_button(245, 360, 116, 15, "Password Breaker", "", &fillname);
            g.ecl.register_button_callbacks(&fillname, Some(button_draw), None, None, None);

            g.ecl.register_button(362, 361, 13, 13, "X", "Close the password breaker", &closename);
            button_assignbitmaps(g, &closename, "close.tif", "close_h.tif", "close_c.tif");
            g.ecl.register_button_callback(&closename, PasswordBreaker::close_click);
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
        if !self.is_interface_visible(g) {
            self.create_interface(g);
        }
        for name in button_names(self.base.pid) {
            g.ecl.button_bring_to_front(&name);
        }
    }

    fn is_interface_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button(&format!("passwordbreaker {}", self.base.pid)).is_some()
    }

    impl_as_any!();
}
