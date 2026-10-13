//! Port of source/interface/remoteinterface/bbsscreen_interface.h + source/interface/remoteinterface/bbsscreen_interface.cpp

use uplink_engine::{gl, text};

use crate::app::globals::DEMOGAME;
use crate::app::opengl_interface::{border_draw, button_click, button_highlight, clear_draw, set_colour, sy, text_draw};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::interface::interfacescreen::{InterfaceScreen, ScreenSlot};
use crate::interface::remoteinterface::contactscreen_interface::{ContactScreenInterface, CONTACT_TYPE_MISSION};
use crate::interface::remoteinterface::remoteinterface::{RemoteInterface, SCREEN_BBSSCREEN};
use crate::interface::scrollbox::ScrollBox;
use crate::objref::ScreenRef;
use crate::world::company::companyuplink::CompanyUplink;
use crate::world::computer::computerscreen::bbsscreen::BBSScreen;
use crate::world::generator::missiongenerator::MISSION_TRACEUSER;
use crate::world::person::Person;

/// Bulletin Board screen: lists the Uplink company's missions and lets the player accept or contact.
#[derive(Debug)]
pub struct BBSScreenInterface {
    pub cs: Option<ScreenRef>,
    // PORT: these were `static` members in C++ (shared by every instance). They are kept per
    // instance here, which is equivalent because `create` resets baseoffset/currentselect.
    /// First mission index shown in the list (scrollbox offset).
    pub baseoffset: i32,
    /// Mission index currently selected (-1 = none).
    pub currentselect: i32,
    pub previousnummessages: i32,
    pub previousupdate: i64,
}

impl Default for BBSScreenInterface {
    fn default() -> Self {
        BBSScreenInterface {
            cs: None,
            baseoffset: 0,
            currentselect: -1,
            previousnummessages: 0,
            previousupdate: 0,
        }
    }
}

/// Name of the Eclipse button for list row `i` (C++ `sprintf(name, "BBmessage %d", i)`).
fn bbs_row_name(i: i32) -> String {
    format!("BBmessage {i}")
}

/// C++ `sscanf(button->name, "BBmessage %d", &index)`.
fn parse_row_index(name: &str) -> i32 {
    name.strip_prefix("BBmessage")
        .and_then(|rest| rest.trim_start().parse().ok())
        .unwrap_or(0)
}

/// `time(NULL)`
fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// `(CompanyUplink *) game->GetWorld()->GetCompany("Uplink")`
fn uplink_company(g: &Ctx) -> &CompanyUplink {
    g.world()
        .get_company("Uplink")
        .and_then(|c| c.uplink.as_deref())
        .expect("UplinkAssert(cu): no Uplink company")
}

fn uplink_company_mut(g: &mut Ctx) -> &mut CompanyUplink {
    g.world_mut()
        .get_company_mut("Uplink")
        .and_then(|c| c.uplink.as_deref_mut())
        .expect("UplinkAssert(cu): no Uplink company")
}

/// The `BBSScreenInterface` statics: `(baseoffset, currentselect)`. Defaults if the screen is not current.
fn bbs_state(g: &Ctx) -> (i32, i32) {
    g.interface()
        .ri
        .screen
        .get::<BBSScreenInterface>()
        .map(|s| (s.baseoffset, s.currentselect))
        .unwrap_or((0, -1))
}

fn bbs_state_mut(g: &mut Ctx) -> Option<&mut BBSScreenInterface> {
    g.interface_mut().ri.screen.get_mut::<BBSScreenInterface>()
}

/// The slot holding the current remote screen (`RemoteInterface::screen`).
fn remote_screen_slot(g: &mut Ctx) -> &mut ScreenSlot {
    &mut g.interface_mut().ri.screen
}

impl BBSScreenInterface {
    /// `ClickBBSButton` (static MouseFn): select a message and show its details.
    pub fn click_bbs_button(g: &mut Ctx, button: &Button) {
        let (baseoffset, currentselect) = bbs_state(g);
        let index = parse_row_index(&button.name) + baseoffset;

        // Dirty the old button
        g.ecl.dirty_button(&bbs_row_name(currentselect - baseoffset));

        let found = uplink_company(g)
            .get_mission(index)
            .map(|m| (m.minuplinkrating, m.get_details().to_string()));

        if let Some((minuplinkrating, details)) = found {
            if let Some(bbs) = bbs_state_mut(g) {
                bbs.currentselect = index;
            }

            if g.player().person.rating.uplinkrating >= minuplinkrating {
                g.ecl.register_caption_change("bbs_details", &details, 2000, None);
            } else {
                g.ecl.register_caption_change(
                    "bbs_details",
                    "You do not have a sufficient Uplink Rating to view this message.",
                    2000,
                    None,
                );
            }
        }
    }

    /// `DrawBBSButton` (static DrawFn)
    pub fn draw_bbs_button(g: &mut Ctx, button: &Button, highlighted: bool, _clicked: bool) {
        let (baseoffset, currentselect) = bbs_state(g);
        let index = parse_row_index(&button.name) + baseoffset;

        let screenheight = g.app.options.get_option_value("graphics_screenheight");
        gl::scissor(button.x, screenheight - (button.y + button.height), button.width, button.height);
        gl::enable(gl::SCISSOR_TEST);

        clear_draw(g, button.x, button.y, button.width, button.height);

        // Get the text from mission number (index + baseoffset)
        let mission = uplink_company(g).get_mission(index).map(|m| {
            (m.minuplinkrating, m.description.clone(), m.createdate.get_short_string())
        });

        if let Some((minuplinkrating, description, date_string)) = mission {
            if index == currentselect {
                gl::begin(gl::QUADS);
                set_colour(g, "PanelHighlightA");
                gl::vertex2i(button.x, button.y);
                set_colour(g, "PanelHighlightB");
                gl::vertex2i(button.x + button.width, button.y);
                set_colour(g, "PanelHighlightA");
                gl::vertex2i(button.x + button.width, button.y + button.height);
                set_colour(g, "PanelHighlightB");
                gl::vertex2i(button.x, button.y + button.height);
                gl::end();
            }

            if highlighted {
                set_colour(g, "PanelHighlightBorder");
                border_draw(button);
            }

            let uplinkrating = g.player().person.rating.uplinkrating;
            let encrypted = uplinkrating < minuplinkrating;

            let (subject, date) = if !encrypted {
                let mut ratingdif = (uplinkrating - minuplinkrating) as f32;
                if ratingdif > 5.0 {
                    ratingdif = 5.0;
                }

                let k: f32 = if DEMOGAME { 0.2 } else { 0.1 };
                gl::color4f(1.0 - ratingdif * k, 1.0 - ratingdif * k, 1.0 - ratingdif * k, 1.0);
                (description, date_string)
            } else {
                gl::color4f(0.2, 0.2, 0.2, 1.0);
                (
                    "Encrypted (Insufficient Uplink Rating)".to_string(),
                    "Unknown".to_string(),
                )
            };

            text::draw_text_default(button.x + 110, button.y + 10, &subject);
            text::draw_text_default(button.x + 5, button.y + 10, &date);
        }

        gl::disable(gl::SCISSOR_TEST);
    }

    /// `DrawDetails` (static DrawFn)
    pub fn draw_details(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        gl::begin(gl::QUADS);
        set_colour(g, "PanelBackgroundA");
        gl::vertex2i(button.x, button.y + button.height);
        set_colour(g, "PanelBackgroundB");
        gl::vertex2i(button.x, button.y);
        set_colour(g, "PanelBackgroundA");
        gl::vertex2i(button.x + button.width, button.y);
        set_colour(g, "PanelBackgroundB");
        gl::vertex2i(button.x + button.width, button.y + button.height);
        gl::end();

        set_colour(g, "PanelBorder");
        border_draw(button);

        text_draw(g, button, highlighted, clicked);
    }

    /// `MousedownBBSButton` (static MouseFn)
    pub fn mousedown_bbs_button(g: &mut Ctx, button: &Button) {
        let index = parse_row_index(&button.name);
        let baseoffset = bbs_state(g).0;

        if uplink_company(g).get_mission(index + baseoffset).is_some() {
            button_click(g, button);
        }
    }

    /// `HighlightBBSButton` (static MouseFn, mouse move)
    pub fn highlight_bbs_button(g: &mut Ctx, button: &Button) {
        let index = parse_row_index(&button.name);
        let baseoffset = bbs_state(g).0;

        if uplink_company(g).get_mission(index + baseoffset).is_some() {
            button_highlight(g, button);
        }
    }

    /// `ExitClick` (static MouseFn)
    pub fn exit_click(g: &mut Ctx, _button: &Button) {
        let nextpage = RemoteInterface::get_computer_screen(g)
            .and_then(|cs| cs.as_bbs_screen())
            .expect("UplinkAssert(bbs)")
            .nextpage;

        RemoteInterface::run_screen(g, nextpage);
    }

    /// `AcceptClick` (static MouseFn)
    pub fn accept_click(g: &mut Ctx, _button: &Button) {
        let currentselect = bbs_state(g).1;
        let Some(mission) = uplink_company(g).get_mission(currentselect).cloned() else {
            return;
        };

        // Can we auto-accept this mission?
        if mission.acceptrating == -1 {
            g.ecl.register_caption_change_auto(
                "bbs_details",
                "This mission cannot be auto-accepted.  You must contact the employersto accept this mission.",
                None,
            );
            return;
        }

        // There are some cases where a mission cannot be accepted
        // -eg trace a hacker who is in fact the player
        // This should be duplicated in the contact button code
        if mission.type_ == MISSION_TRACEUSER && mission.completiona == "PLAYER" {
            g.ecl.register_caption_change_auto(
                "bbs_details",
                "We cannot allow you to accept this mission, on the specific instructions of the employer.",
                None,
            );
            return;
        }

        if g.player().person.rating.uplinkrating >= mission.acceptrating {
            Person::give_mission(g, "PLAYER", mission);

            // C++ `CompanyUplink::missions` is an LList: RemoveData unlinks the mission and the
            // later ones shift down, exactly like Vec::remove.
            let cu = uplink_company_mut(g);
            if currentselect >= 0 && (currentselect as usize) < cu.missions.len() {
                cu.missions.remove(currentselect as usize);
            }

            if let Some(bbs) = bbs_state_mut(g) {
                bbs.currentselect = -1;
            }
            g.ecl.register_caption_change_auto("bbs_details", " ", None);
        } else {
            g.ecl.register_caption_change_auto(
                "bbs_details",
                "You do not have a sufficient Uplink Rating to auto-accept this mission.You must first contact the employers.",
                None,
            );
        }
    }

    /// `ContactClick` (static MouseFn)
    pub fn contact_click(g: &mut Ctx, _button: &Button) {
        // Get the currently selected mission
        let selected = bbs_state(g).1;
        let mission = uplink_company(g)
            .get_mission(selected)
            .map(|m| (m.type_, m.completiona.clone(), m.minuplinkrating));

        // Get the bbs screen
        let contactpage = RemoteInterface::get_computer_screen(g)
            .and_then(|cs| cs.as_bbs_screen())
            .expect("UplinkAssert(bbs)")
            .contactpage;

        let Some((mission_type, completiona, minuplinkrating)) = mission else {
            return;
        };
        if contactpage == -1 {
            return;
        }

        // There are some cases where a mission cannot be dealt with
        // -eg trace a hacker who is in fact the player
        // This code is repeated in the accept button code
        if mission_type == MISSION_TRACEUSER && completiona == "PLAYER" {
            g.ecl.register_caption_change_auto(
                "bbs_details",
                "We cannot allow you to contact this employer, on their specific instructions.",
                None,
            );
            return;
        }

        if g.player().person.rating.uplinkrating >= minuplinkrating {
            RemoteInterface::run_screen(g, contactpage);

            ScreenSlot::call(g, remote_screen_slot, |s, g| {
                let cs = s
                    .as_any_mut()
                    .downcast_mut::<ContactScreenInterface>()
                    .expect("UplinkAssert(cs): remote screen is not the contact screen");
                cs.set_type(CONTACT_TYPE_MISSION);
                cs.set_mission(g, selected);
            });
        } else {
            g.ecl.register_caption_change_auto(
                "bbs_details",
                "You do not have a sufficient Uplink Rating to contact this employer.",
                None,
            );
        }
    }

    /// `ScrollChange` (scrollbox callback): sets `baseoffset` and dirties the rows.
    pub fn scroll_change(g: &mut Ctx, _scrollname: &str, new_value: i32) {
        if let Some(bbs) = bbs_state_mut(g) {
            bbs.baseoffset = new_value;
        }

        for i in 0..Self::num_items_on_screen(g) {
            g.ecl.dirty_button(&bbs_row_name(i));
        }
    }

    /// `NumItemsOnScreen` (static): rows that fit under the current screen height.
    pub fn num_items_on_screen(g: &Ctx) -> i32 {
        let screenheight = g.app.options.get_option_value("graphics_screenheight");
        let available_pixels = screenheight - 200 - 45;
        available_pixels / 20
    }

    /// `GetComputerScreen` (typed accessor; the C++ casts `cs`).
    pub fn get_computer_screen<'a>(&self, g: &'a Ctx) -> Option<&'a BBSScreen> {
        let r = self.cs.as_ref()?;
        g.world()
            .get_computer(&r.computer)?
            .get_computer_screen(r.index)?
            .as_bbs_screen()
    }
}

impl InterfaceScreen for BBSScreenInterface {
    crate::impl_as_any!();

    /// `Create(ComputerScreen *newcs)` body (the `cs` is already set by `set_computer_screen`).
    fn create(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            return;
        }

        let num_items = Self::num_items_on_screen(g);

        g.ecl.register_button(20, 30, 100, 15, "Date", "This column shows the date the message was submitted", "bbs_date");
        g.ecl.register_button(
            125,
            30,
            22 + sy(388) - 125 + 15,
            15,
            "Subject",
            "This column shows the subject of the message",
            "bbs_subject",
        );

        for i in 0..num_items {
            let name = bbs_row_name(i);
            g.ecl.register_button(20, i * 20 + 50, sy(388), 17, "", "Read this Bulletin Board message", &name);
            g.ecl.register_button_callbacks(
                &name,
                Some(BBSScreenInterface::draw_bbs_button),
                Some(BBSScreenInterface::click_bbs_button),
                Some(BBSScreenInterface::mousedown_bbs_button),
                Some(BBSScreenInterface::highlight_bbs_button),
            );
        }

        let base_x = 23 + sy(388) - 55;
        let base_y = 50 + num_items * 20;

        g.ecl.register_button(20, base_y, sy(388) - 55, 110, "", "", "bbs_details");
        g.ecl.register_button_callbacks(
            "bbs_details",
            Some(BBSScreenInterface::draw_details),
            None,
            None,
            None,
        );

        g.ecl.register_button(base_x, base_y, 70, 15, "Accept", "Accept this mission", "bbs_accept");
        g.ecl.register_button_callback("bbs_accept", BBSScreenInterface::accept_click);

        g.ecl.register_button(base_x, base_y + 20, 70, 15, "Contact", "Contact the employer of this mission", "bbs_contact");
        g.ecl.register_button_callback("bbs_contact", BBSScreenInterface::contact_click);

        g.ecl.register_button(
            base_x,
            base_y + 110 - 15,
            70,
            15,
            "Exit BBS",
            "Close the BBS screen and return to the main menu",
            "bbs_exit",
        );
        g.ecl.register_button_callback("bbs_exit", BBSScreenInterface::exit_click);

        let num_missions = uplink_company(g).missions.len() as i32;
        ScrollBox::create_scroll_box(
            g,
            "bbs_scroll",
            22 + sy(388),
            47,
            15,
            num_items * 20,
            num_missions,
            12,
            0,
            Some(BBSScreenInterface::scroll_change),
        );

        self.baseoffset = 0;
        self.currentselect = -1;
    }

    fn remove(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            return;
        }

        for i in 0..Self::num_items_on_screen(g) {
            g.ecl.remove_button(&bbs_row_name(i));
        }

        g.ecl.remove_button("bbs_details");

        g.ecl.remove_button("bbs_date");
        g.ecl.remove_button("bbs_subject");

        g.ecl.remove_button("bbs_accept");
        g.ecl.remove_button("bbs_contact");

        g.ecl.remove_button("bbs_exit");

        ScrollBox::remove_scroll_box(g, "bbs_scroll");
    }

    fn update(&mut self, g: &mut Ctx) {
        let new_num_messages = uplink_company(g).missions.len() as i32;
        if new_num_messages != self.previousnummessages {
            ScrollBox::set_num_items(g, "bbs_scroll", new_num_messages);
            self.previousnummessages = new_num_messages;

            // This screen is taken out of its slot while update runs, so if set_num_items clamped
            // the index and called scroll_change, that callback could not reach `self`. Take the
            // scroll box's index instead.
            if let Some(sb) = ScrollBox::get_scroll_box(g, "bbs_scroll") {
                self.baseoffset = sb.current_index;
            }
        }

        let now = now_secs();
        if now > self.previousupdate {
            for i in 0..Self::num_items_on_screen(g) {
                g.ecl.dirty_button(&bbs_row_name(i));
            }

            self.previousupdate = now + 1;
        }
    }

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("bbs_details").is_some()
    }

    fn screen_id(&self) -> i32 {
        SCREEN_BBSSCREEN
    }

    fn escape_key_pressed(&mut self, g: &mut Ctx) -> bool {
        // C++: ExitClick(NULL). The screen is taken out while this runs, so read `nextpage`
        // from `self` rather than through RemoteInterface.
        let nextpage = self
            .get_computer_screen(g)
            .map(|bbs| bbs.nextpage)
            .expect("UplinkAssert(bbs)");

        RemoteInterface::run_screen(g, nextpage);
        true
    }

    fn set_computer_screen(&mut self, cs: ScreenRef) {
        self.cs = Some(cs);
    }

    fn computer_screen_ref(&self) -> Option<&ScreenRef> {
        self.cs.as_ref()
    }
}
