//! Port of source/interface/localinterface/mission_interface.h + source/interface/localinterface/mission_interface.cpp
//! Mission screen: shows one of the player's missions, lets them reply to the employer or abandon it.

use crate::app::opengl_interface::{create_stextbox, remove_stextbox, textbox_draw};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::impl_as_any;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::localinterface::email_interface::{append_links_and_codes, Stream};
use crate::interface::localinterface::localinterface::{LocalInterface, PANELSIZE, SCREEN_MISSION, SCREEN_NONE, SCREEN_SENDMAIL};
use crate::world::company::mission::Mission;
use crate::world::generator::missiongenerator::MissionGenerator;

#[derive(Debug, Clone)]
pub struct MissionInterface {
    /// Index into `player.missions` of the mission being shown (C++ `int index`).
    pub index: i32,
    pub timesync: i32,
    /// C++ `Mission *mission`, a non-owning pointer into `player.missions`. Kept here as a
    /// snapshot clone; the body compares it against `player.missions[index]` where C++ compared pointers.
    // PORT: pointer identity replaced by a snapshot plus the index.
    pub mission: Option<Mission>,
}

impl Default for MissionInterface {
    fn default() -> Self {
        MissionInterface {
            index: -1,
            timesync: 0,
            mission: None,
        }
    }
}

/// `(screenw, screenh, paneltop, panelwidth)`, recomputed from the options as in the C++.
fn panel_metrics(g: &Ctx) -> (i32, i32, i32, i32) {
    let screenw = g.app.options.get_option_value("graphics_screenwidth");
    let screenh = g.app.options.get_option_value("graphics_screenheight");
    let paneltop = (100.0 * ((screenw as f64 * PANELSIZE) / 188.0) + 30.0) as i32;
    let panelwidth = (screenw as f64 * PANELSIZE) as i32;
    (screenw, screenh, paneltop, panelwidth)
}

/// Unix seconds, C++ `time(NULL)`.
fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// `player.missions.GetData(index)`: the mission at `index`, if there is one.
fn mission_at(g: &Ctx, index: i32) -> Option<&Mission> {
    usize::try_from(index).ok().and_then(|i| g.player().agent().missions.get(i))
}

/// Identity of a mission, compared by content (C++ compared pointers).
fn same_mission(a: &Mission, b: &Mission) -> bool {
    match (serde_json::to_string(a), serde_json::to_string(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

/// Where `snapshot` currently sits in `player.missions`: `hint` if it still matches there, else its
/// new position. C++ DArray removals leave holes so a mission never changes index; `Vec::remove`
/// shifts later missions down, so the pointer-identity check is done by searching for the mission.
fn find_mission(g: &Ctx, snapshot: &Mission, hint: i32) -> Option<usize> {
    if mission_at(g, hint).is_some_and(|real| same_mission(snapshot, real)) {
        return usize::try_from(hint).ok();
    }
    g.player().agent().missions.iter().position(|real| same_mission(snapshot, real))
}

/// The mission this screen is showing, with the index it was set to.
fn current_mission(g: &Ctx) -> Option<(i32, Mission)> {
    let mi = g.interface().li.screen.get::<MissionInterface>()?;
    Some((mi.index, mi.mission.clone()?))
}

impl MissionInterface {
    /// C++ `MissionInterface::TitleClick` (static button callback; also the Close button).
    pub fn title_click(g: &mut Ctx, _button: &Button) {
        LocalInterface::run_screen(g, SCREEN_NONE, -1);
    }

    /// C++ `MissionInterface::AbandonClick` (static button callback). The first click only changes the
    /// caption to "! ABANDON !"; the second click abandons the mission.
    pub fn abandon_click(g: &mut Ctx, button: &Button) {
        if button.caption == "Abandon" {
            if let Some(b) = g.ecl.get_button_mut(&button.name) {
                b.set_caption("! ABANDON !");
            }
            return;
        }

        let Some((index, mission)) = current_mission(g) else {
            debug_assert!(false, "MissionInterface::abandon_click: no mission is shown");
            return;
        };

        // Ensure they are the same
        // ie Player has not lost this mission
        if let Some(real_index) = find_mission(g, &mission, index) {
            MissionGenerator::mission_failed(g, &mission, "PLAYER", "You abandoned the mission.");

            // Remove the mission
            // C++ `Agent::missions` is an LList: RemoveData unlinks the mission and the later ones
            // shift down, exactly like Vec::remove.
            let missions = &mut g.player_mut().agent_mut().missions;
            if real_index < missions.len() {
                missions.remove(real_index);
            }
        }

        // Get rid of this screen
        LocalInterface::run_screen(g, SCREEN_NONE, -1);
    }

    /// C++ `MissionInterface::ReplyClick` (static button callback).
    pub fn reply_click(g: &mut Ctx, _button: &Button) {
        let Some((index, mission)) = current_mission(g) else {
            debug_assert!(false, "MissionInterface::reply_click: no mission is shown");
            return;
        };

        // Ensure they are the same
        // ie Player has not lost this mission
        if find_mission(g, &mission, index).is_none() {
            LocalInterface::run_screen(g, SCREEN_NONE, -1);
            // PORT: C++ falls through here and reads the mission it has just lost; stop instead.
            return;
        }

        let contact = mission.contact.clone();
        let body = format!(
            "I have completed the following mission:\n{}\n\nPlease credit my account with the remaining payment ASAP.\nAdditional requested data:\n",
            mission.description
        );

        LocalInterface::run_screen(g, SCREEN_SENDMAIL, -1);
        LocalInterface::update(g);

        g.ecl.register_caption_change("sendmail_to", &contact, 1, None); // Should occur instantly
        g.ecl.register_caption_change_auto("sendmail_subject", "Mission completed", None);
        g.ecl.register_caption_change("sendmail_body box", &body, 1000, None);
    }

    /// C++ `MissionInterface::SetMission`: looks the mission up in `player.missions`.
    pub fn set_mission(&mut self, g: &mut Ctx, newindex: i32) {
        self.index = newindex;
        self.mission = mission_at(g, newindex).cloned();
        debug_assert!(self.mission.is_some(), "MissionInterface::set_mission: no mission at index {newindex}");
    }
}

impl InterfaceScreen for MissionInterface {
    fn create(&mut self, g: &mut Ctx) {
        let Some(mission) = self.mission.as_ref() else {
            debug_assert!(false, "MissionInterface::create without a mission");
            return;
        };

        if self.is_visible(g) {
            return;
        }

        let (screenw, screenh, paneltop, panelwidth) = panel_metrics(g);

        g.ecl.register_button(screenw - panelwidth - 2, paneltop, panelwidth, 15, "MISSION", "Remove the mission screen", "mission_title");
        g.ecl.register_button_callback("mission_title", Self::title_click);

        let employer = format!("Employer : {}", mission.employer);
        let payment = format!("Payment : {} credits", mission.payment);
        let description = mission.description.clone();

        let mut details = Stream::new();
        // PORT: C++ GetFullDetails asserts the field is set; a missing value is shown as empty here.
        details.put(mission.fulldetails.as_deref().unwrap_or(""));

        // Concatenate any links and codes onto the end of the mission
        append_links_and_codes(&mut details, g, &mission.links, &mission.codes);
        let details = details.into_string();

        g.ecl.register_button(screenw - panelwidth - 2, paneltop + 20, panelwidth, 15, "", "", "mission_employer");
        g.ecl.register_button(screenw - panelwidth - 2, paneltop + 40, panelwidth, 15, "", "", "mission_payment");
        g.ecl.register_button(screenw - panelwidth - 2, paneltop + 60, panelwidth, 30, "", "", "mission_description");
        g.ecl.register_button_callbacks("mission_employer", Some(textbox_draw), None, None, None);
        g.ecl.register_button_callbacks("mission_payment", Some(textbox_draw), None, None, None);
        g.ecl.register_button_callbacks("mission_description", Some(textbox_draw), None, None, None);
        g.ecl.register_caption_change("mission_employer", &employer, 100, None);
        g.ecl.register_caption_change("mission_payment", &payment, 100, None);
        g.ecl.register_caption_change("mission_description", &description, 200, None);

        let boxheight = screenh - (paneltop + 95) - 70;
        create_stextbox(g, screenw - panelwidth - 2, paneltop + 95, panelwidth, boxheight, &details, "mission_details");

        let x1 = screenw - panelwidth - 2;
        let x2 = x1 + panelwidth / 3 + 2;
        let x3 = x2 + panelwidth / 3 + 2;
        let y = paneltop + 95 + boxheight + 5;
        g.ecl.register_button(x1, y, panelwidth / 3 - 2, 15, "Close", "Close this screen", "mission_close");
        g.ecl.register_button_callback("mission_close", Self::title_click);
        g.ecl.register_button(x2, y, panelwidth / 3 - 2, 15, "Reply", "Send a mail to the employer", "mission_reply");
        g.ecl.register_button_callback("mission_reply", Self::reply_click);
        g.ecl.register_button(x3, y, panelwidth / 3 - 2, 15, "Abandon", "Abandon this mission", "mission_abandon");
        g.ecl.register_button_callback("mission_abandon", Self::abandon_click);
    }

    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            g.ecl.remove_button("mission_title");
            g.ecl.remove_button("mission_employer");
            g.ecl.remove_button("mission_payment");
            g.ecl.remove_button("mission_description");
            remove_stextbox(g, "mission_details");
            g.ecl.remove_button("mission_close");
            g.ecl.remove_button("mission_reply");
            g.ecl.remove_button("mission_abandon");
        }
    }

    fn update(&mut self, g: &mut Ctx) {
        if now_secs() > self.timesync as i64 + 1 {
            // Ensure they are the same
            // ie Player has not lost this mission
            let found = self.mission.as_ref().and_then(|snapshot| find_mission(g, snapshot, self.index));
            match found {
                Some(pos) => self.index = pos as i32,
                None => LocalInterface::run_screen(g, SCREEN_NONE, -1),
            }
            self.timesync = now_secs() as i32;
        }
    }

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("mission_title").is_some()
    }

    fn screen_id(&self) -> i32 {
        SCREEN_MISSION
    }

    impl_as_any!();
}
