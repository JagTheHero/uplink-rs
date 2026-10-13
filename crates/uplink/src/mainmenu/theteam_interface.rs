//! Port of source/mainmenu/theteam_interface.h + source/mainmenu/theteam_interface.cpp

use uplink_engine::{gl, text};

use crate::app::opengl_interface::{button_assignbitmap, clear_draw, imagebutton_draw, textbutton_draw, wordwraptext};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::interface::interfacescreen::{InterfaceScreen, ScreenSlot};
use crate::mainmenu::mainmenu::{MainMenu, MAINMENU_LOGIN, MAINMENU_THETEAM};
use crate::world::generator::numbergenerator::NumberGenerator;

/// File-scope `static char *teamnames[]` (the secret "Meet the team" page).
pub const TEAM_NAMES: [&str; 4] = [
    "Chris Delay",
    "Tom Arundel",
    "Mark Morris",
    "John Knottenbelt",
];

/// File-scope `static char *teamwords[]` (the active array; the older one is commented out in C++).
pub const TEAM_WORDS: [&str; 4] = [
    concat!(
        "The last of the bedroom programmers\n",
        "The first of the children\n",
        "\n\n\n\n",
        "and when wizards stay up late,\n",
        "new worlds are born",
    ),
    concat!(
        "Carpe diemLIVE in that moment\n",
        "But dont hesitate to forget it later\n\n\n",
        "Impulsive doesnotdoyou justice\n\n",
        "Throw it away\n",
        "Corner of the room\n",
        "Bottom of the shelf\n",
        "Behind last weeks wonder\n\n\n",
        "It was good while it lasted\n",
        "Was it worth it",
    ),
    concat!(
        "The bird at the front of the flock\n\n",
        "The pitch black tunnel\n",
        "with no light at the end\n\n\n",
        "What do you say to these people\n",
        "What do you say to yourself\n\n\n\n",
        "Oh God when will it ever end",
    ),
    concat!(
        "You are the source\n",
        "It comes from within\n\n\n\n",
        "Dont worry about us",
    ),
];

/// Meet the team! (secret page).
#[derive(Debug)]
pub struct TheTeamInterface {
    pub targetindex: i32,
    pub timesync: i32,
}

impl Default for TheTeamInterface {
    /// C++ constructor: `targetindex = -1; timesync = 0;`
    fn default() -> Self {
        TheTeamInterface { targetindex: -1, timesync: 0 }
    }
}

/// The main menu's current screen slot (C `app->GetMainMenu()->GetMenuScreen()`).
fn main_menu_slot(g: &mut Ctx) -> &mut ScreenSlot {
    &mut g.app.mainmenu.screen
}

/// Runs `f` on the team screen (taken out of its slot while it runs). `None` if the current screen
/// is not the team screen.
fn with_screen<R>(g: &mut Ctx, f: impl FnOnce(&mut TheTeamInterface, &mut Ctx) -> R) -> Option<R> {
    ScreenSlot::call(g, main_menu_slot, |s, g| {
        let this = s.as_any_mut().downcast_mut::<TheTeamInterface>()?;
        Some(f(this, g))
    })
    .flatten()
}

/// The cycle step of `MergeCaption`: 'A' (and below) -> 'z', 'a' -> 'Z', anything else one step down.
fn cycle_down(c: u8) -> u8 {
    if c <= b'A' {
        b'z'
    } else if c == b'a' {
        b'Z'
    } else {
        c - 1
    }
}

/// `sscanf(button->name, "theteam_member %d", &index)`.
fn member_index(name: &str) -> i32 {
    name.split_whitespace().nth(1).and_then(|t| t.parse().ok()).unwrap_or(0)
}

impl TheTeamInterface {
    pub fn new() -> Self {
        Self::default()
    }

    /// Static `NameDraw` (DrawFn).
    pub fn name_draw(g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        // C++ uses a literal 480 for the scissor height here
        gl::scissor(button.x, 480 - (button.y + button.height), button.width, button.height);
        gl::enable(gl::SCISSOR_TEST);

        clear_draw(g, button.x, button.y, button.width, button.height);

        gl::color4f(1.0, 1.0, 1.0, 1.0);
        let ypos = (button.y + button.height / 2) + 5;

        for (i, ch) in button.caption.bytes().enumerate() {
            text::draw_text_styled(button.x + i as i32 * 12, ypos, &char::from(ch).to_string(), text::HELVETICA_18);
        }

        gl::disable(gl::SCISSOR_TEST);
    }

    /// Static `TextDraw` (DrawFn).
    pub fn text_draw(g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        gl::scissor(button.x, 480 - (button.y + button.height), button.width, button.height);
        gl::enable(gl::SCISSOR_TEST);

        clear_draw(g, button.x, button.y, button.width, button.height);

        gl::color4f(1.0, 1.0, 1.0, 1.0);

        let wrapped_text = wordwraptext(&button.caption, button.width);

        for (line, this_line) in wrapped_text.iter().enumerate() {
            let ypos = button.y + 5 + line as i32 * 12;

            for (i, ch) in this_line.bytes().enumerate() {
                text::draw_text_styled(button.x + i as i32 * 6, ypos, &char::from(ch).to_string(), text::HELVETICA_10);
            }
        }

        gl::disable(gl::SCISSOR_TEST);
    }

    /// Static `TeamMemberHighlight` (MouseFn).
    pub fn team_member_highlight(g: &mut Ctx, button: &Button) {
        let index = member_index(&button.name);

        let done = with_screen(g, |this, g| {
            if this.targetindex != index {
                this.targetindex = index;
                if let Some(words) = usize::try_from(index - 1).ok().and_then(|i| TEAM_WORDS.get(i)) {
                    g.ecl.register_caption_change_auto("theteam_details", words, None);
                }
            }
        });
        debug_assert!(done.is_some(), "TeamMemberHighlight: team screen is not the current screen");
    }

    /// Static `ExitClick` (MouseFn).
    pub fn exit_click(g: &mut Ctx, _button: &Button) {
        MainMenu::run_screen(g, MAINMENU_LOGIN);
    }

    /// Protected `MergeCaption`: merges the target caption into the named button's caption.
    pub fn merge_caption(&mut self, g: &mut Ctx, button_name: &str, target_caption: &str) {
        let Some(current) = g.ecl.get_button(button_name).map(|b| b.caption.clone()) else {
            debug_assert!(false, "MergeCaption: button missing: {button_name}");
            return;
        };
        let target = target_caption.as_bytes();
        let cur = current.as_bytes();

        //
        // First check - is the current name the right length?
        //

        let mut new_caption: Vec<u8> = if cur.len() < target.len() {
            // Add one random letter on the end
            let mut v = cur.to_vec();
            v.push(b'A' + NumberGenerator::random_number(26) as u8);
            v
        } else if cur.len() > target.len() {
            // strncpy the whole string, then drop the last character
            cur[..cur.len() - 1].to_vec()
        } else {
            cur.to_vec()
        };

        //
        // Go through each letter, cycling as neccesary
        //

        let mut numchanges = 0;

        for i in 0..new_caption.len() {
            if i < target.len() {
                // We are inside the target caption
                // Cycle downwards if we aren't right yet

                if new_caption[i] != target[i] {
                    numchanges += 1;

                    if matches!(target[i], b' ' | b'.' | b',' | b'\n') {
                        new_caption[i] = target[i];
                    } else {
                        new_caption[i] = cycle_down(new_caption[i]);
                    }
                }
            } else {
                // This is outside the target caption
                // So just cycle it anyway

                numchanges += 1;
                new_caption[i] = cycle_down(new_caption[i]);
            }
        }

        //
        // If we have now finished, numchanges would have been zero
        //

        if numchanges == 0 {
            //targetindex = -1;
            return;
        }

        //
        // Finally, set the button to the new caption
        //

        let new_text = String::from_utf8_lossy(&new_caption).into_owned();
        if let Some(button) = g.ecl.get_button_mut(button_name) {
            button.set_caption(&new_text);
        }
    }
}

impl InterfaceScreen for TheTeamInterface {
    fn create(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            // Create the four pictures

            g.ecl.register_button(10, 10, 200, 100, " ", " ", "theteam_member 1");
            button_assignbitmap(g, "theteam_member 1", "mainmenu/team1.tif");
            g.ecl.register_button_callbacks("theteam_member 1", Some(imagebutton_draw), None, None, Some(Self::team_member_highlight));

            g.ecl.register_button(430, 10, 200, 100, " ", " ", "theteam_member 3");
            button_assignbitmap(g, "theteam_member 3", "mainmenu/team3.tif");
            g.ecl.register_button_callbacks("theteam_member 3", Some(imagebutton_draw), None, None, Some(Self::team_member_highlight));

            g.ecl.register_button(430, 370, 200, 100, " ", " ", "theteam_member 2");
            button_assignbitmap(g, "theteam_member 2", "mainmenu/team2.tif");
            g.ecl.register_button_callbacks("theteam_member 2", Some(imagebutton_draw), None, None, Some(Self::team_member_highlight));

            g.ecl.register_button(10, 370, 200, 100, " ", " ", "theteam_member 4");
            button_assignbitmap(g, "theteam_member 4", "mainmenu/team4.tif");
            g.ecl.register_button_callbacks("theteam_member 4", Some(imagebutton_draw), None, None, Some(Self::team_member_highlight));

            // Create the central textbox

            g.ecl.register_button(180, 140, 280, 20, " ", " ", "theteam_name");
            g.ecl.register_button_callbacks("theteam_name", Some(Self::name_draw), None, None, None);

            g.ecl.register_button(180, 170, 280, 300, " ", " ", "theteam_details");
            g.ecl.register_button_callbacks("theteam_details", Some(textbutton_draw), None, None, None);

            // Exit button

            g.ecl.register_button(600, 465, 39, 14, "Exit", "Return to the main menu", "theteam_exit");
            g.ecl.register_button_callback("theteam_exit", Self::exit_click);
        }
    }

    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            g.ecl.remove_button("theteam_member 1");
            g.ecl.remove_button("theteam_member 2");
            g.ecl.remove_button("theteam_member 3");
            g.ecl.remove_button("theteam_member 4");

            g.ecl.remove_button("theteam_name");
            g.ecl.remove_button("theteam_details");

            g.ecl.remove_button("theteam_exit");
        }
    }

    fn update(&mut self, g: &mut Ctx) {
        if self.targetindex != -1 {
            if g.ecl.get_accurate_time() > self.timesync {
                if let Some(name) = usize::try_from(self.targetindex - 1).ok().and_then(|i| TEAM_NAMES.get(i)) {
                    self.merge_caption(g, "theteam_name", name);
                }

                self.timesync = g.ecl.get_accurate_time() + 100;
            }
        }
    }

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("theteam_member 1").is_some()
    }
    fn screen_id(&self) -> i32 {
        MAINMENU_THETEAM
    }
    crate::impl_as_any!();
}
