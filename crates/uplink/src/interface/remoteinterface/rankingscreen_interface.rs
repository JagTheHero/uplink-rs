//! Port of source/interface/remoteinterface/rankingscreen_interface.h + source/interface/remoteinterface/rankingscreen_interface.cpp

use uplink_engine::{gl, text};

use crate::app::opengl_interface;
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::remoteinterface::remoteinterface::{RemoteInterface, SCREEN_RANKINGSCREEN};
use crate::interface::remoteinterface::remoteinterfacescreen::{draw_main_title, draw_sub_title};
use crate::objref::ScreenRef;
use crate::world::computer::computerscreen::computerscreen::ComputerScreen;
use crate::world::computer::computerscreen::genericscreen::GenericScreen;
use crate::world::generator::numbergenerator::NumberGenerator;
use crate::world::person::{Person, PERSON_STATUS_DEAD, PERSON_STATUS_INJAIL, PERSON_STATUS_NONE};

/// Remote ranking screen: shows the player's standing among agents.
#[derive(Debug, Default)]
pub struct RankingScreenInterface {
    pub cs: Option<ScreenRef>,
}

/// Status caption for an agent row (C++ `cstatus` in `Create`).
fn status_caption(status: i32) -> &'static str {
    match status {
        PERSON_STATUS_NONE => "Active",
        PERSON_STATUS_INJAIL => "In Jail",
        PERSON_STATUS_DEAD => "Deceased",
        _ => "Unknown",
    }
}

/// Runs the screen's `nextpage`, as `ExitClick` does (no `-1` check in the C++).
fn run_next_page(g: &mut Ctx, nextpage: Option<i32>) {
    if let Some(nextpage) = nextpage {
        RemoteInterface::run_screen(g, nextpage);
    }
}

impl RankingScreenInterface {
    /// `GetComputerScreen` (typed accessor; the C++ casts `cs`).
    pub fn get_computer_screen<'a>(&self, g: &'a Ctx) -> Option<&'a GenericScreen> {
        let cs = self.cs.as_ref()?;
        match g.world().get_computer(&cs.computer)?.get_computer_screen(cs.index)? {
            ComputerScreen::GenericScreen(gs) => Some(gs),
            _ => None,
        }
    }

    /// `ExitClick` (static MouseFn)
    pub fn exit_click(g: &mut Ctx, _button: &Button) {
        let nextpage = g
            .interface()
            .ri
            .screen
            .get::<Self>()
            .and_then(|me| me.get_computer_screen(g))
            .map(|gs| gs.nextpage);
        run_next_page(g, nextpage);
    }

    /// `ActiveAgentDraw` (static DrawFn)
    pub fn active_agent_draw(g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        opengl_interface::clear_draw(g, button.x, button.y, button.width, button.height);

        opengl_interface::set_colour(g, "DefaultText");

        let xpos = button.x + 10;
        let ypos = button.y + 10;

        text::draw_text_default(xpos, ypos, &button.caption);
    }

    /// `NonActiveAgentDraw` (static DrawFn)
    pub fn non_active_agent_draw(g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        opengl_interface::clear_draw(g, button.x, button.y, button.width, button.height);

        opengl_interface::set_colour(g, "DimmedText");

        let xpos = button.x + 10;
        let ypos = button.y + 10;

        text::draw_text_default(xpos, ypos, &button.caption);
    }

    /// `PlayerDraw` (static DrawFn)
    pub fn player_draw(g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        gl::begin(gl::QUADS);
        opengl_interface::set_colour(g, "PanelBackgroundA");
        gl::vertex2i(button.x - 1, button.y);
        opengl_interface::set_colour(g, "PanelBackgroundB");
        gl::vertex2i(button.x + button.width, button.y);
        opengl_interface::set_colour(g, "PanelBackgroundA");
        gl::vertex2i(button.x + button.width, button.y + button.height);
        opengl_interface::set_colour(g, "PanelBackgroundB");
        gl::vertex2i(button.x, button.y + button.height);
        gl::end();

        opengl_interface::set_colour(g, "PanelBorder");
        opengl_interface::border_draw(button);

        opengl_interface::set_colour(g, "DefaultText");

        let xpos = button.x + 10;
        let ypos = button.y + 10;

        text::draw_text_default(xpos, ypos, &button.caption);
    }
}

impl InterfaceScreen for RankingScreenInterface {
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

        // Build a list of agents sorted on rating order. The C++ walks the people BTree via
        // ConvertToDArray (pre-order of an unbalanced tree, i.e. insertion-dependent); equal scores
        // are tie-broken by that walk order. The player is the entry under the key "PLAYER".

        let candidates: Vec<(&str, &Person)> = g
            .world()
            .all_people()
            .into_iter()
            .filter(|(name, p)| *name == "PLAYER" || p.is_agent())
            .collect();

        // Insert each one before the first entry with a lower-or-equal score (C++ `>=` test).
        let mut sorted: Vec<(&str, &Person)> = Vec::new();
        for entry in candidates {
            let score = entry.1.rating.uplinkscore;
            match sorted.iter().position(|s| score >= s.1.rating.uplinkscore) {
                Some(is) => sorted.insert(is, entry),
                None => sorted.push(entry),
            }
        }

        // Find the player's rank.

        let Some(mut playerrank) = sorted.iter().position(|s| s.0 == "PLAYER") else {
            return;
        };
        let top_player_index = playerrank as i32;

        // Rows for the top ten entries (C++ `sorted.ValidIndex (i)` for i < 10).

        let mut rows: Vec<(String, &'static str, i32)> = Vec::new();
        for (i, (_, person)) in sorted.iter().enumerate().take(10) {
            let handle = person.agent.as_ref().map(|a| a.handle.as_str()).unwrap_or("");
            let cname = format!("{:>2}  :  Agent {}", i + 1, handle);
            rows.push((cname, status_caption(person.status), person.status));
        }

        // If the player is not in the top 10, put his rank at the bottom.
        // Special case: a rating of 1 or 2 is shown much further down, to make the world seem bigger.

        let mut player_row: Option<String> = None;
        if playerrank >= 10 {
            let rating = g.player().rating.uplinkrating;
            if rating == 1 {
                playerrank *= 3;
                playerrank += NumberGenerator::random_number(20) as usize;
            } else if rating == 2 {
                playerrank *= 2;
                playerrank += NumberGenerator::random_number(10) as usize;
            }

            let handle = g.player().agent.as_ref().map(|a| a.handle.clone()).unwrap_or_default();
            player_row = Some(format!("{:>2}  :  Agent {}", playerrank + 1, handle));
        }

        // All borrows of the world end here, so the interface can be built.

        g.ecl.register_button(80, 60, 350, 25, &maintitle, "", "ranking_maintitle");
        g.ecl.register_button_callbacks("ranking_maintitle", Some(draw_main_title), None, None, None);
        g.ecl.register_button(80, 80, 350, 20, &subtitle, "", "ranking_subtitle");
        g.ecl.register_button_callbacks("ranking_subtitle", Some(draw_sub_title), None, None, None);

        for (i, (cname, cstatus, status)) in rows.iter().enumerate() {
            let i = i as i32;
            let name = format!("ranking_name {}", i);
            let status_name = format!("ranking_status {}", i);

            g.ecl.register_button(30, 120 + 20 * i, 170, 15, cname, "", &name);
            g.ecl.register_button(200, 120 + 20 * i, 100, 15, cstatus, "", &status_name);

            if i == top_player_index {
                g.ecl.register_button_callbacks(&name, Some(Self::player_draw), None, None, None);
                g.ecl.register_button_callbacks(&status_name, Some(Self::player_draw), None, None, None);
            } else if *status == PERSON_STATUS_NONE {
                g.ecl.register_button_callbacks(&name, Some(Self::active_agent_draw), None, None, None);
                g.ecl.register_button_callbacks(&status_name, Some(Self::active_agent_draw), None, None, None);
            } else {
                g.ecl.register_button_callbacks(&name, Some(Self::non_active_agent_draw), None, None, None);
                g.ecl.register_button_callbacks(&status_name, Some(Self::non_active_agent_draw), None, None, None);
            }
        }

        if let Some(cname) = player_row {
            g.ecl.register_button(60, 340, 170, 15, &cname, "", "ranking_playerrank");
            g.ecl.register_button(230, 340, 100, 15, "Active", "", "ranking_playerstatus");

            g.ecl.register_button_callbacks("ranking_playerrank", Some(Self::active_agent_draw), None, None, None);
            g.ecl.register_button_callbacks("ranking_playerstatus", Some(Self::active_agent_draw), None, None, None);
        }

        // Exit button

        g.ecl.register_button(355, 387, 70, 15, "Exit", "Close the screen and return to the main menu", "ranking_exit");
        g.ecl.register_button_callback("ranking_exit", Self::exit_click);
    }

    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            g.ecl.remove_button("ranking_maintitle");
            g.ecl.remove_button("ranking_subtitle");

            for i in 0..10 {
                let name = format!("ranking_name {}", i);
                let status = format!("ranking_status {}", i);

                g.ecl.remove_button(&name);
                g.ecl.remove_button(&status);
            }

            g.ecl.remove_button("ranking_playerrank");
            g.ecl.remove_button("ranking_playerstatus");

            g.ecl.remove_button("ranking_exit");
        }
    }

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("ranking_maintitle").is_some()
    }

    fn screen_id(&self) -> i32 {
        SCREEN_RANKINGSCREEN
    }

    fn return_key_pressed(&mut self, g: &mut Ctx) -> bool {
        // C++: ExitClick ( NULL ); return true;
        let nextpage = self.get_computer_screen(g).map(|gs| gs.nextpage);
        run_next_page(g, nextpage);
        true
    }

    fn escape_key_pressed(&mut self, g: &mut Ctx) -> bool {
        // C++: ExitClick ( NULL ); return true;
        let nextpage = self.get_computer_screen(g).map(|gs| gs.nextpage);
        run_next_page(g, nextpage);
        true
    }

    fn set_computer_screen(&mut self, cs: ScreenRef) {
        self.cs = Some(cs);
    }

    fn computer_screen_ref(&self) -> Option<&ScreenRef> {
        self.cs.as_ref()
    }
}
