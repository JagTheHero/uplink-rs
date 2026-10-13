//! Port of source/interface/remoteinterface/academicscreen_interface.h + source/interface/remoteinterface/academicscreen_interface.cpp

use crate::app::miscutils::lower_case_string;
use crate::app::opengl_interface::{
    button_assignbitmap, button_highlight, create_msgbox, textbutton_draw,
};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::remoteinterface::remoteinterface::{RemoteInterface, SCREEN_ACADEMICSCREEN};
use crate::interface::remoteinterface::remoteinterfacescreen::{draw_main_title, draw_sub_title};
use crate::objref::ScreenRef;
use crate::world::computer::computerscreen::genericscreen::GenericScreen;
use crate::world::computer::recordbank::RECORDBANK_NAME;

/// The computer this screen searches and edits (C++ `GetComputer("International Academic Database")`).
const ACADEMIC_DATABASE: &str = "International Academic Database";

/// International academic database search screen (edits a person's academic record).
#[derive(Debug, Clone)]
pub struct AcademicScreenInterface {
    pub cs: Option<ScreenRef>,
    /// Lower-cased name being searched for; `None` when no search is running.
    pub searchname: Option<String>,
    /// Index of the record currently being scanned (-1 when idle).
    pub recordindex: i32,
    pub lastupdate: i32,
}

impl Default for AcademicScreenInterface {
    fn default() -> Self {
        AcademicScreenInterface {
            cs: None,
            searchname: None,
            recordindex: -1,
            lastupdate: 0,
        }
    }
}

/// `EclGetButton(name)->caption`, empty when the button is missing.
fn button_caption(g: &Ctx, name: &str) -> String {
    g.ecl
        .get_button(name)
        .map(|b| b.caption.clone())
        .unwrap_or_default()
}

/// `EclGetButton(name)->SetCaption(text)`; a missing button is ignored here.
fn set_caption(g: &mut Ctx, name: &str, caption: &str) {
    if let Some(button) = g.ecl.get_button_mut(name) {
        button.set_caption(caption);
    }
}

impl AcademicScreenInterface {
    /// `SetSearchName`
    pub fn set_search_name(&mut self, newsearchname: &str) {
        self.searchname = Some(lower_case_string(newsearchname));
        self.recordindex = 0;
    }

    /// `GetComputerScreen` (typed accessor; the C++ casts `cs`).
    pub fn get_computer_screen<'a>(&self, g: &'a Ctx) -> Option<&'a GenericScreen> {
        let cs = self.cs.as_ref()?;
        g.world()
            .get_computer(&cs.computer)?
            .get_computer_screen(cs.index)?
            .as_generic_screen()
    }

    /// `UpdateScreen`: shows the person behind the record at `recordindex`.
    pub fn update_screen(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            return;
        }

        // Read everything needed from the record first, so the buttons can be changed after.
        let Some(comp) = g.world().get_computer(ACADEMIC_DATABASE) else {
            debug_assert!(false, "AcademicScreenInterface: academic database computer missing");
            return;
        };
        let Some(rec) = comp.recordbank.get_record(self.recordindex) else {
            return;
        };
        let thisname = rec.get_field(RECORDBANK_NAME).map(str::to_string);
        let collegequals = rec.get_field("College").map(str::to_string);
        let uniquals = rec.get_field("University").map(str::to_string);
        let otherquals = rec.get_field("Other").map(str::to_string);

        let photoindex = thisname
            .as_deref()
            .and_then(|name| g.world().get_person(name))
            .map(|person| person.photoindex);

        // Show a photo
        let photo = match photoindex {
            Some(index) => format!("photos/image{}.tif", index),
            None => "photos/image0.tif".to_string(),
        };
        button_assignbitmap(g, "academic_photo", &photo);

        // Update display with the current record
        if photoindex.is_some() {
            set_caption(g, "academic_name", thisname.as_deref().unwrap_or(""));
            set_caption(g, "academic_collegequals", collegequals.as_deref().unwrap_or(""));
            set_caption(g, "academic_uniquals", uniquals.as_deref().unwrap_or(""));
            set_caption(g, "academic_otherquals", otherquals.as_deref().unwrap_or(""));
        }
    }

    /// `DetailsDraw` (static DrawFn): the detail fields are always drawn highlighted.
    pub fn details_draw(g: &mut Ctx, button: &Button, highlighted: bool, _clicked: bool) {
        textbutton_draw(g, button, highlighted, true);
    }

    /// `CommitClick` (static MouseFn): writes the three qualification fields back to the record.
    pub fn commit_click(g: &mut Ctx, _button: &Button) {
        let Some(recordindex) = g
            .interface()
            .ri
            .screen
            .get::<AcademicScreenInterface>()
            .map(|s| s.recordindex)
        else {
            return;
        };
        if recordindex == -1 {
            return;
        }

        let Some(comp) = g.world().get_computer(ACADEMIC_DATABASE) else {
            debug_assert!(false, "AcademicScreenInterface: academic database computer missing");
            return;
        };
        if comp.security.is_running_proxy() {
            create_msgbox(g, "Error", "Denied access by Proxy Server");
            return;
        }

        let college = button_caption(g, "academic_collegequals");
        let university = button_caption(g, "academic_uniquals");
        let other = button_caption(g, "academic_otherquals");

        let Some(rec) = g
            .world_mut()
            .get_computer_mut(ACADEMIC_DATABASE)
            .and_then(|comp| comp.recordbank.get_record_mut(recordindex))
        else {
            debug_assert!(false, "AcademicScreenInterface: no record at the commit index");
            return;
        };
        rec.change_field("College", &college);
        rec.change_field("University", &university);
        rec.change_field("Other", &other);

        create_msgbox(g, "Success", "Academic record updated");
    }

    /// `CloseClick` (static MouseFn): runs the screen's next page.
    pub fn close_click(g: &mut Ctx, _button: &Button) {
        let Some(this) = g.interface().ri.screen.get::<AcademicScreenInterface>().cloned() else {
            return;
        };
        Self::close_body(g, &this);
    }

    /// Body of `CloseClick` on a given screen state. Escape passes `self`, because the screen is
    /// taken out of its slot while its own method runs, so the slot lookup would find nothing.
    fn close_body(g: &mut Ctx, this: &AcademicScreenInterface) {
        let nextpage = this.get_computer_screen(g).map_or(-1, |gs| gs.nextpage);
        RemoteInterface::run_screen(g, nextpage);
    }
}

impl InterfaceScreen for AcademicScreenInterface {
    crate::impl_as_any!();

    /// `Create(ComputerScreen *newcs)` body (the `cs` is already set by `set_computer_screen`).
    fn create(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            return;
        }

        // Draw the screen titles
        let (maintitle, subtitle) = self
            .get_computer_screen(g)
            .map(|gs| (gs.base.maintitle.clone(), gs.base.subtitle.clone()))
            .unwrap_or_default();

        g.ecl.register_button(80, 60, 350, 25, &maintitle, "", "academic_maintitle");
        g.ecl.register_button_callbacks("academic_maintitle", Some(draw_main_title), None, None, None);
        g.ecl.register_button(80, 80, 350, 20, &subtitle, "", "academic_subtitle");
        g.ecl.register_button_callbacks("academic_subtitle", Some(draw_sub_title), None, None, None);

        // Photo
        g.ecl.register_button(30, 130, 128, 160, "", "Shows the person's photograph", "academic_photo");
        button_assignbitmap(g, "academic_photo", "photos/image0.tif");

        // Details
        g.ecl.register_button(180, 130, 200, 15, "Name", "", "academic_name");
        g.ecl.register_button_callbacks("academic_name", Some(Self::details_draw), None, None, None);

        g.ecl.register_button(180, 150, 200, 15, "College Qualifications", "", "academic_collegequalstitle");
        g.ecl.register_button(180, 165, 200, 105, "", "", "academic_collegequals");
        g.ecl.register_button_callbacks(
            "academic_collegequals",
            Some(Self::details_draw),
            None,
            None,
            Some(button_highlight),
        );

        g.ecl.register_button(180, 270, 200, 15, "Graduate Qualifications", "", "academic_uniqualstitle");
        g.ecl.register_button(180, 285, 200, 30, "", "", "academic_uniquals");
        g.ecl.register_button_callbacks(
            "academic_uniquals",
            Some(Self::details_draw),
            None,
            None,
            Some(button_highlight),
        );

        g.ecl.register_button(180, 325, 200, 15, "Other Qualifications", "", "academic_otherqualstitle");
        g.ecl.register_button(180, 340, 200, 30, "", "", "academic_otherquals");
        g.ecl.register_button_callbacks(
            "academic_otherquals",
            Some(Self::details_draw),
            None,
            None,
            Some(button_highlight),
        );

        // Control buttons
        if g.interface().ri.security_level <= 2 {
            g.ecl.register_button(30, 335, 128, 15, "Commit", "Commit all changes made", "academic_commit");
            g.ecl.register_button_callback("academic_commit", Self::commit_click);
        }

        g.ecl.register_button(30, 355, 128, 15, "Done", "Close this screen", "academic_close");
        g.ecl.register_button_callback("academic_close", Self::close_click);

        g.ecl.make_button_editable("academic_collegequals");
        g.ecl.make_button_editable("academic_uniquals");
        g.ecl.make_button_editable("academic_otherquals");
    }

    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            for name in [
                "academic_maintitle",
                "academic_subtitle",
                "academic_photo",
                "academic_name",
                "academic_collegequalstitle",
                "academic_collegequals",
                "academic_uniqualstitle",
                "academic_uniquals",
                "academic_otherqualstitle",
                "academic_otherquals",
                "academic_commit",
                "academic_close",
            ] {
                g.ecl.remove_button(name);
            }
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

        // We are searching for a record. Is this one it?
        let Some(comp) = g.world().get_computer(ACADEMIC_DATABASE) else {
            debug_assert!(false, "AcademicScreenInterface: academic database computer missing");
            return;
        };

        // Have we searched all records?
        if self.recordindex == comp.recordbank.records.len() as i32 {
            self.searchname = None;
            self.recordindex = -1;
            return;
        }

        let Some(rec) = comp.recordbank.get_record(self.recordindex) else {
            return;
        };
        let Some(thisname) = rec.get_field(RECORDBANK_NAME).map(str::to_string) else {
            debug_assert!(false, "AcademicScreenInterface: record has no name");
            return;
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
        g.ecl.get_button("academic_photo").is_some()
    }

    fn screen_id(&self) -> i32 {
        SCREEN_ACADEMICSCREEN
    }

    /// `EscapeKeyPressed`: runs the next page like the Done button, and does not block the key.
    fn escape_key_pressed(&mut self, g: &mut Ctx) -> bool {
        Self::close_body(g, self);
        false
    }

    fn set_computer_screen(&mut self, cs: ScreenRef) {
        self.cs = Some(cs);
    }

    fn computer_screen_ref(&self) -> Option<&ScreenRef> {
        self.cs.as_ref()
    }
}
