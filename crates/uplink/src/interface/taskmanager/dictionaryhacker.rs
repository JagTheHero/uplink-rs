//! Port of source/interface/taskmanager/dictionaryhacker.h + source/interface/taskmanager/dictionaryhacker.cpp

use uplink_engine::gl;

use crate::app::opengl_interface;
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::game::data::data::TICKSREQUIRED_DICTIONARYHACKER;
use crate::interface::taskmanager::uplinktask::{UplinkTask, UplinkTaskBase};
use crate::objref::ObjRef;
use crate::vanbakel::Vanbakel;
use crate::world::computer::computer::Computer;
use crate::world::computer::computerscreen::computerscreen::ComputerScreen;
use crate::world::computer::recordbank::RECORDBANK_PASSWORD;
use crate::world::connection::Connection;

pub const DICTIONARYHACKER_WAITING: i32 = 0;
pub const DICTIONARYHACKER_INPROGRESS: i32 = 1;

#[derive(Debug)]
pub struct DictionaryHacker {
    pub base: UplinkTaskBase,
    pub status: i32,
    /// Index into world.passwords
    pub currentindex: i32,
    /// C++ `char *username`: NULL means admin.
    pub username: Option<String>,
    /// C++ `char *password`: the password being searched for.
    pub password: Option<String>,
    pub numticksrequired: f32,
    pub progress: i32,
}

/// The two kinds of screen the dictionary hacker can be aimed at (C++ `OID_PASSWORDSCREEN` / `OID_USERIDSCREEN`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScreenKind {
    Password,
    UserId,
}

/// C++ `uo->GetOBJECTID()` for a target: `Some` only for a PasswordScreen or UserIDScreen.
fn target_screen_kind(g: &Ctx, target: &ObjRef) -> Option<ScreenKind> {
    let ObjRef::ComputerScreen(sref) = target else { return None };
    match g.world().get_computer(&sref.computer)?.get_computer_screen(sref.index)? {
        ComputerScreen::PasswordScreen(_) => Some(ScreenKind::Password),
        ComputerScreen::UserIDScreen(_) => Some(ScreenKind::UserId),
        _ => None,
    }
}

/// C++ `sscanf(button->name, "%s %d", bname, &pid)`: the PID after the button's first word.
fn pid_from_button_name(name: &str) -> Option<i32> {
    name.split_whitespace().nth(1)?.parse().ok()
}

/// Names of the window's buttons: the wordlist, three border strips, and the close button.
fn button_names(pid: i32) -> [String; 5] {
    [
        format!("dictionaryhacker {pid}"),
        format!("dictionaryhacker_b1 {pid}"),
        format!("dictionaryhacker_b2 {pid}"),
        format!("dictionaryhacker_b3 {pid}"),
        format!("dictionaryhacker_close {pid}"),
    ]
}

impl DictionaryHacker {
    pub fn new() -> Self {
        DictionaryHacker {
            base: UplinkTaskBase {
                version: 1.0,
                targetint: -1,
                ..Default::default()
            },
            status: DICTIONARYHACKER_WAITING,
            currentindex: 0,
            username: None,
            password: None,
            numticksrequired: 0.0,
            progress: 0,
        }
    }

    /// C++ `DictionaryHacker::DictionaryHackerDraw` (static button callback).
    pub fn dictionary_hacker_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        gl::begin(gl::QUADS);

        gl::color3ub(0, 14, 59);
        gl::vertex2i(button.x, button.y);
        gl::color3ub(36, 72, 146);
        gl::vertex2i(button.x + button.width, button.y);
        gl::color3ub(82, 134, 206);
        gl::vertex2i(button.x + button.width, button.y + button.height);
        gl::color3ub(73, 122, 194);
        gl::vertex2i(button.x, button.y + button.height);

        gl::end();

        opengl_interface::text_draw(g, button, highlighted, clicked);

        if highlighted || clicked {
            opengl_interface::border_draw(button);
        }
    }

    /// C++ `DictionaryHacker::DictionaryHackerClick` (static button callback).
    pub fn dictionary_hacker_click(g: &mut Ctx, button: &Button) {
        let Some(pid) = pid_from_button_name(&button.name) else { return };
        g.interface_mut().tm.set_target_program(pid);
    }

    /// C++ `DictionaryHacker::CloseClick` (static button callback).
    pub fn close_click(g: &mut Ctx, button: &Button) {
        let Some(pid) = pid_from_button_name(&button.name) else { return };
        Vanbakel::remove_task(g, pid);
    }
}

impl Default for DictionaryHacker {
    fn default() -> Self {
        Self::new()
    }
}

impl UplinkTask for DictionaryHacker {
    fn base(&self) -> &UplinkTaskBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut UplinkTaskBase {
        &mut self.base
    }
    fn initialise(&mut self, _g: &mut Ctx) {}

    fn tick(&mut self, g: &mut Ctx, n: i32) {
        if !self.is_interface_visible(g) {
            return;
        }

        if self.status == DICTIONARYHACKER_WAITING {
            // Don't do anything
        } else if self.status == DICTIONARYHACKER_INPROGRESS {
            for _ in 0..n {
                // First check to make sure we are still looking at the same screen
                // i.e the player has not logged out / moved / got past it already
                if g.ecl.get_button(&self.base.targetstring).is_none() {
                    // Terminate this task
                    Vanbakel::remove_task(g, self.base.pid);
                    return;
                }

                // Test to see if we have checked all possible words
                if self.currentindex >= g.world().passwords.size() {
                    // Terminate this task
                    Vanbakel::remove_task(g, self.base.pid);
                    return;
                }

                self.progress += 1;
                if self.progress as f32 >= self.numticksrequired {
                    let numwords = (self.progress as f32 / self.numticksrequired) as i32;

                    for _ in 0..numwords {
                        let word = g.world().get_password(self.currentindex).map(str::to_string);

                        if let Some(word) = word {
                            // Test the current word against the password to see if it is correct
                            if self.password.as_deref() == Some(word.as_str()) {
                                // The full word has been discovered - enter it into the password screen
                                match target_screen_kind(g, &self.base.targetobject) {
                                    Some(ScreenKind::Password) => {
                                        g.ecl.register_caption_change_auto("passwordscreen_password", &word, None);
                                    }
                                    Some(ScreenKind::UserId) => {
                                        g.ecl.register_caption_change_auto("useridscreen_code", &word, None);
                                    }
                                    None => {}
                                }

                                // Also add it into the player's code list
                                let newcode = match &self.username {
                                    Some(name) => Computer::generate_access_code_name(name, &word),
                                    None => Computer::generate_access_code(&word),
                                };

                                let remotehost = g.player().remotehost.clone();
                                g.player_mut().give_code(&remotehost, &newcode);

                                Vanbakel::remove_task(g, self.base.pid);
                                // WARNING - This instance has been deleted - do not access any more data
                                return;
                            }
                        }

                        // This is not the correct word - move onto the next
                        self.currentindex += 1;
                    }

                    self.progress = 0;
                }
            }

            let total = g.world().passwords.size();
            let percentage = ((self.currentindex as f32 / total as f32) * 100.0) as i32;
            let mut caption = format!("\nTrying\nWordlist...{percentage}%\n-> ");
            for i in self.currentindex..self.currentindex + 10 {
                if let Some(word) = g.world().get_password(i) {
                    caption.push_str(word);
                    caption.push('\n');
                }
            }

            let [bname, _, _, _, _] = button_names(self.base.pid);
            if let Some(button) = g.ecl.get_button_mut(&bname) {
                button.set_caption(&caption);
            }
        }
    }

    fn create_interface(&mut self, g: &mut Ctx) {
        if self.is_interface_visible(g) {
            return;
        }

        let [bname, border1, border2, border3, closename] = button_names(self.base.pid);

        g.ecl.register_button(0, 50, 100, 200, "", "", &bname);
        g.ecl.register_button_callbacks(
            &bname,
            Some(Self::dictionary_hacker_draw),
            Some(Self::dictionary_hacker_click),
            Some(opengl_interface::button_click),
            Some(opengl_interface::button_highlight),
        );
        g.ecl.register_caption_change_auto(&bname, "Select target", None);
        // C++ `EclButtonSendToBack ( bname )` is commented out there, so it is not called here.

        g.ecl.register_button(0, 45, 105, 4, "", "", &border1);
        g.ecl.register_button_callbacks(&border1, Some(opengl_interface::buttonborder_draw), None, None, None);

        g.ecl.register_button(101, 45, 4, 210, "", "", &border2);
        g.ecl.register_button_callbacks(&border2, Some(opengl_interface::buttonborder_draw), None, None, None);

        g.ecl.register_button(0, 251, 105, 4, "", "", &border3);
        g.ecl.register_button_callbacks(&border3, Some(opengl_interface::buttonborder_draw), None, None, None);

        g.ecl.register_button(85, 51, 13, 13, "X", "Close the dictionary hacker", &closename);
        opengl_interface::button_assignbitmaps(g, &closename, "close.tif", "close_h.tif", "close_c.tif");
        g.ecl.register_button_callback(&closename, Self::close_click);
    }

    fn remove_interface(&mut self, g: &mut Ctx) {
        if !self.is_interface_visible(g) {
            return;
        }

        let [bname, border1, border2, border3, closename] = button_names(self.base.pid);

        g.ecl.remove_button(&bname);
        g.ecl.remove_button(&border1);
        g.ecl.remove_button(&border2);
        g.ecl.remove_button(&border3);
        g.ecl.remove_button(&closename);
    }

    fn show_interface(&mut self, g: &mut Ctx) {
        if !self.is_interface_visible(g) {
            self.create_interface(g);
        }

        let [bname, border1, border2, border3, closename] = button_names(self.base.pid);

        g.ecl.button_bring_to_front(&bname);
        g.ecl.button_bring_to_front(&border1);
        g.ecl.button_bring_to_front(&border2);
        g.ecl.button_bring_to_front(&border3);
        g.ecl.button_bring_to_front(&closename);
    }

    fn is_interface_visible(&self, g: &Ctx) -> bool {
        let [bname, _, _, _, _] = button_names(self.base.pid);
        g.ecl.get_button(&bname).is_some()
    }

    fn set_target(&mut self, g: &mut Ctx, target: ObjRef, targetstring: &str, targetint: i32) {
        // uo  : Target PasswordScreen or UserIDScreen object
        // uos : name of button with password in
        // uoi : unused
        self.base.targetobject = target.clone();
        self.base.targetstring = targetstring.to_string();
        self.base.targetint = targetint;

        let Some(kind) = target_screen_kind(g, &target) else { return };
        let ObjRef::ComputerScreen(sref) = &target else { return };

        match kind {
            ScreenKind::Password => {
                let (password, monitor) = {
                    let Some(comp) = g.world().get_computer(&sref.computer) else { return };
                    let Some(ComputerScreen::PasswordScreen(ps)) = comp.get_computer_screen(sref.index) else { return };
                    (ps.password.clone(), comp.security.is_running_monitor())
                };

                self.password = Some(password);
                self.status = DICTIONARYHACKER_INPROGRESS;

                self.username = None;

                self.numticksrequired = TICKSREQUIRED_DICTIONARYHACKER;
                self.progress = 0;

                if monitor {
                    Connection::begin_trace(g, "PLAYER");
                }
            }
            ScreenKind::UserId => {
                // If the player has not entered a UserID then do this for admin
                // Then grab the target name from the useridscreen box
                let name = {
                    let button = g
                        .ecl
                        .get_button_mut("useridscreen_name")
                        .expect("DictionaryHacker::set_target: useridscreen_name button missing");
                    if button.caption.is_empty() {
                        button.set_caption("admin");
                    }
                    button.caption.clone()
                };

                self.username = Some(name.clone());

                // Look up this user name in this computer's records
                let (record_password, monitor) = {
                    let Some(comp) = g.world().get_computer(&sref.computer) else { return };
                    let record_password = comp.recordbank.get_record_from_name(&name).map(|rec| {
                        rec.get_field(RECORDBANK_PASSWORD)
                            .expect("DictionaryHacker::set_target: record has no password")
                            .to_string()
                    });
                    (record_password, comp.security.is_running_monitor())
                };

                match record_password {
                    Some(passwd) => {
                        self.password = Some(passwd);

                        self.numticksrequired = TICKSREQUIRED_DICTIONARYHACKER;
                        self.progress = 0;

                        self.status = DICTIONARYHACKER_INPROGRESS;

                        if monitor {
                            Connection::begin_trace(g, "PLAYER");
                        }
                    }
                    None => {
                        // The user name is not recognised
                        Vanbakel::remove_task(g, self.base.pid);
                    }
                }
            }
        }
    }

    crate::impl_as_any!();
}
