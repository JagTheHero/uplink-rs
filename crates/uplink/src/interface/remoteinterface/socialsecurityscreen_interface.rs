//! Port of source/interface/remoteinterface/socialsecurityscreen_interface.h + source/interface/remoteinterface/socialsecurityscreen_interface.cpp

use crate::app::miscutils::lower_case_string;
use crate::app::opengl_interface::{button_assignbitmap, button_click, button_highlight, create_msgbox, textbutton_draw};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::remoteinterface::remoteinterface::{RemoteInterface, SCREEN_SOCSECSCREEN};
use crate::interface::remoteinterface::remoteinterfacescreen::{draw_main_title, draw_sub_title};
use crate::objref::ScreenRef;
use crate::world::computer::computerscreen::genericscreen::GenericScreen;
use crate::world::computer::recordbank::RECORDBANK_NAME;

/// The computer that holds the social security records.
const COMPUTER_NAME: &str = "International Social Security Database";

/// The `ScreenRef` of the social security screen currently in the remote interface's slot.
fn current_cs(g: &Ctx) -> Option<ScreenRef> {
    g.interface().ri.screen.get::<SocialSecurityScreenInterface>().and_then(|s| s.cs.clone())
}

/// The caption of a button, or an empty string when the button does not exist.
fn caption_of(g: &Ctx, name: &str) -> String {
    g.ecl.get_button(name).map(|b| b.caption.clone()).unwrap_or_default()
}

/// Sets a button's caption when the button exists.
fn set_button_caption(g: &mut Ctx, name: &str, caption: &str) {
    if let Some(button) = g.ecl.get_button_mut(name) {
        button.set_caption(caption);
    }
}

/// `CloseClick` body: runs the `nextpage` of the generic screen behind `cs` (no -1 check, as in C++).
fn run_nextpage(g: &mut Ctx, cs: Option<ScreenRef>) {
    let Some(cs) = cs else { return };
    let nextpage = g
        .world()
        .get_computer(&cs.computer)
        .and_then(|c| c.get_computer_screen(cs.index))
        .and_then(|s| s.as_generic_screen())
        .map(|gs| gs.nextpage);
    let Some(nextpage) = nextpage else { return };
    RemoteInterface::run_screen(g, nextpage);
}

/// Remote social security screen: looks up a person's record by name.
#[derive(Debug)]
pub struct SocialSecurityScreenInterface {
    pub cs: Option<ScreenRef>,
    /// Name being searched for; `None` when no search is running.
    pub searchname: Option<String>,
    /// Index of the record being shown (-1 when none).
    pub recordindex: i32,
    pub lastupdate: i32,
}

impl Default for SocialSecurityScreenInterface {
    fn default() -> Self {
        SocialSecurityScreenInterface {
            cs: None,
            searchname: None,
            recordindex: -1,
            lastupdate: 0,
        }
    }
}

impl SocialSecurityScreenInterface {
    /// `GetComputerScreen` (typed accessor; the C++ casts `cs`).
    pub fn get_computer_screen<'a>(&self, g: &'a Ctx) -> Option<&'a GenericScreen> {
        let cs = self.cs.as_ref()?;
        g.world().get_computer(&cs.computer)?.get_computer_screen(cs.index)?.as_generic_screen()
    }

    /// `UpdateScreen`: redraws the record details for `recordindex`.
    pub fn update_screen(&self, g: &mut Ctx) {
        if !self.is_visible(g) {
            return;
        }

        // Copy everything needed out of the record bank before touching the interface.
        let (thisname, socsecnum, dob, marital, personal, photo) = {
            let Some(comp) = g.world().get_computer(COMPUTER_NAME) else { return };
            let Some(rec) = comp.recordbank.get_record(self.recordindex) else { return };
            let thisname = rec.get_field(RECORDBANK_NAME).unwrap_or("").to_string();
            let socsecnum = rec.get_field("Social Security").unwrap_or("").to_string();
            let dob = rec.get_field("D.O.B").unwrap_or("").to_string();
            let marital = rec.get_field("Marital Status").unwrap_or("").to_string();
            let personal = rec.get_field("Personal Status").unwrap_or("").to_string();
            let photo = g.world().get_person(&thisname).map(|p| p.photoindex);
            (thisname, socsecnum, dob, marital, personal, photo)
        };

        // Show a photo
        match photo {
            Some(index) => button_assignbitmap(g, "ss_photo", &format!("photos/image{index}.tif")),
            None => button_assignbitmap(g, "ss_photo", "photos/image0.tif"),
        }

        // Update display with the current record
        if photo.is_some() {
            set_button_caption(g, "ss_name", &thisname);
            set_button_caption(g, "ss_num", &socsecnum);
            set_button_caption(g, "ss_dob", &dob);
            set_button_caption(g, "ss_marital", &marital);
            set_button_caption(g, "ss_personal", &personal);
        }
    }

    /// `SetSearchName`
    pub fn set_search_name(&mut self, newsearchname: &str) {
        self.searchname = Some(lower_case_string(newsearchname));
        self.recordindex = 0;
    }

    /// `DetailsDraw` (static DrawFn)
    pub fn details_draw(g: &mut Ctx, button: &Button, highlighted: bool, _clicked: bool) {
        textbutton_draw(g, button, highlighted, true);
    }

    /// `CommitClick` (static MouseFn)
    pub fn commit_click(g: &mut Ctx, _button: &Button) {
        let Some(recordindex) = g.interface().ri.screen.get::<SocialSecurityScreenInterface>().map(|s| s.recordindex) else {
            return;
        };
        if recordindex == -1 {
            return;
        }

        let Some(comp) = g.world().get_computer(COMPUTER_NAME) else { return };
        if comp.security.is_running_proxy() {
            create_msgbox(g, "Error", "Denied access by Proxy Server");
            return;
        }

        let socsecnum = caption_of(g, "ss_num");
        let dob = caption_of(g, "ss_dob");
        let marital = caption_of(g, "ss_marital");
        let personal = caption_of(g, "ss_personal");

        {
            let Some(comp) = g.world_mut().get_computer_mut(COMPUTER_NAME) else { return };
            let Some(rec) = comp.recordbank.get_record_mut(recordindex) else { return };
            rec.change_field("Social Security", &socsecnum);
            rec.change_field("D.O.B", &dob);
            rec.change_field("Marital Status", &marital);
            rec.change_field("Personal Status", &personal);
        }

        create_msgbox(g, "Success", "Social Security Record updated");
    }

    /// `CloseClick` (static MouseFn)
    pub fn close_click(g: &mut Ctx, _button: &Button) {
        let cs = current_cs(g);
        run_nextpage(g, cs);
    }
}

impl InterfaceScreen for SocialSecurityScreenInterface {
    crate::impl_as_any!();

    fn create(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            return;
        }
        let Some(gs) = self.get_computer_screen(g) else { return };
        let maintitle = gs.base.maintitle.clone();
        let subtitle = gs.base.subtitle.clone();

        // Draw the screen titles
        g.ecl.register_button(80, 60, 350, 25, &maintitle, "", "ss_maintitle");
        g.ecl.register_button_callbacks("ss_maintitle", Some(draw_main_title), None, None, None);
        g.ecl.register_button(80, 80, 350, 20, &subtitle, "", "ss_subtitle");
        g.ecl.register_button_callbacks("ss_subtitle", Some(draw_sub_title), None, None, None);

        // Photo
        g.ecl.register_button(30, 130, 128, 160, "", "Shows the person's photograph", "ss_photo");
        button_assignbitmap(g, "ss_photo", "photos/image0.tif");

        // Details
        g.ecl.register_button(180, 130, 200, 15, "Name", "", "ss_nametitle");
        g.ecl.register_button(180, 145, 200, 15, "logging on...", "", "ss_name");
        g.ecl.register_button_callbacks("ss_name", Some(SocialSecurityScreenInterface::details_draw), None, None, None);

        g.ecl.register_button(180, 170, 200, 15, "Social Security Number", "", "ss_numtitle");
        g.ecl.register_button(180, 185, 200, 15, "logging on...", "", "ss_num");
        g.ecl.register_button_callbacks(
            "ss_num",
            Some(SocialSecurityScreenInterface::details_draw),
            None,
            Some(button_click),
            Some(button_highlight),
        );

        g.ecl.register_button(180, 210, 200, 15, "Date of Birth", "", "ss_dobtitle");
        g.ecl.register_button(180, 225, 200, 15, "logging on...", "", "ss_dob");
        g.ecl.register_button_callbacks(
            "ss_dob",
            Some(SocialSecurityScreenInterface::details_draw),
            None,
            Some(button_click),
            Some(button_highlight),
        );

        g.ecl.register_button(180, 250, 200, 15, "Marital Status", "", "ss_maritaltitle");
        g.ecl.register_button(180, 265, 200, 15, "logging on...", "", "ss_marital");
        g.ecl.register_button_callbacks(
            "ss_marital",
            Some(SocialSecurityScreenInterface::details_draw),
            None,
            Some(button_click),
            Some(button_highlight),
        );

        g.ecl.register_button(180, 290, 200, 15, "Personal Status", "", "ss_personaltitle");
        g.ecl.register_button(180, 305, 200, 15, "logging on...", "", "ss_personal");
        g.ecl.register_button_callbacks(
            "ss_personal",
            Some(SocialSecurityScreenInterface::details_draw),
            None,
            Some(button_click),
            Some(button_highlight),
        );

        g.ecl.make_button_editable("ss_num");
        g.ecl.make_button_editable("ss_dob");
        g.ecl.make_button_editable("ss_marital");
        g.ecl.make_button_editable("ss_personal");

        // Control buttons
        if g.interface().ri.security_level <= 2 {
            g.ecl.register_button(30, 335, 128, 15, "Commit", "Commit all changes made", "ss_commit");
            g.ecl.register_button_callback("ss_commit", SocialSecurityScreenInterface::commit_click);
        }

        g.ecl.register_button(30, 355, 128, 15, "Done", "Close this screen", "ss_close");
        g.ecl.register_button_callback("ss_close", SocialSecurityScreenInterface::close_click);
    }

    fn remove(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            return;
        }
        for name in [
            "ss_maintitle",
            "ss_subtitle",
            "ss_photo",
            "ss_nametitle",
            "ss_name",
            "ss_numtitle",
            "ss_num",
            "ss_dobtitle",
            "ss_dob",
            "ss_maritaltitle",
            "ss_marital",
            "ss_personaltitle",
            "ss_personal",
            "ss_commit",
            "ss_close",
        ] {
            g.ecl.remove_button(name);
        }
    }

    fn update(&mut self, g: &mut Ctx) {
        if self.searchname.is_none() || !self.is_visible(g) {
            return;
        }

        let timems = g.ecl.get_accurate_time() - self.lastupdate;
        if timems <= 100 {
            return;
        }

        // Have we searched all records?
        let Some(count) = g.world().get_computer(COMPUTER_NAME).map(|c| c.recordbank.records.len() as i32) else {
            return;
        };
        if self.recordindex == count {
            self.searchname = None;
            self.recordindex = -1;
            return;
        }

        // Is this one it?
        let thisname = {
            let Some(comp) = g.world().get_computer(COMPUTER_NAME) else { return };
            let Some(rec) = comp.recordbank.get_record(self.recordindex) else { return };
            rec.get_field(RECORDBANK_NAME).unwrap_or("").to_string()
        };
        let lowercasethisname = lower_case_string(&thisname);

        // Update display with the current record
        self.update_screen(g);

        if self.searchname.as_deref() == Some(lowercasethisname.as_str()) {
            // Record found - stop searching
            self.searchname = None;
        } else {
            // Record not found
            self.recordindex += 1;
        }

        self.lastupdate = g.ecl.get_accurate_time();
    }

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("ss_photo").is_some()
    }

    fn screen_id(&self) -> i32 {
        SCREEN_SOCSECSCREEN
    }

    fn escape_key_pressed(&mut self, g: &mut Ctx) -> bool {
        run_nextpage(g, self.cs.clone());
        false
    }

    fn set_computer_screen(&mut self, cs: ScreenRef) {
        self.cs = Some(cs);
    }

    fn computer_screen_ref(&self) -> Option<&ScreenRef> {
        self.cs.as_ref()
    }
}
