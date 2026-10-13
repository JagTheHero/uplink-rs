//! Port of source/interface/remoteinterface/criminalscreen_interface.h + source/interface/remoteinterface/criminalscreen_interface.cpp

use crate::app::opengl_interface::{border_draw, button_assignbitmap, button_draw, button_highlight, clear_draw, create_msgbox, textbutton_draw};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::game::data::data::{TIME_LEGALACTION, TIME_LEGALACTION_WARNING};
use crate::interface::interfacescreen::{InterfaceScreen, ScreenSlot};
use crate::interface::remoteinterface::remoteinterface::{RemoteInterface, SCREEN_CRIMINALSCREEN};
use crate::interface::remoteinterface::remoteinterfacescreen::{draw_main_title, draw_sub_title};
use crate::objref::ScreenRef;
use crate::world::computer::computer::Computer;
use crate::world::computer::computerscreen::genericscreen::GenericScreen;
use crate::world::computer::recordbank::RECORDBANK_NAME;
use crate::world::scheduler::arrestevent::ArrestEvent;
use crate::world::scheduler::uplinkevent::UplinkEvent;

/// Global criminal database search screen (convictions, clearing records, authorising arrests).
#[derive(Debug)]
pub struct CriminalScreenInterface {
    pub cs: Option<ScreenRef>,
    /// Lower-cased name being searched for; `None` when no search is running.
    pub searchname: Option<String>,
    /// Index of the record currently being scanned (-1 when idle).
    pub recordindex: i32,
    pub lastupdate: i32,
}

impl Default for CriminalScreenInterface {
    fn default() -> Self {
        CriminalScreenInterface {
            cs: None,
            searchname: None,
            recordindex: -1,
            lastupdate: 0,
        }
    }
}

const GLOBAL_CRIMINAL_DATABASE: &str = "Global Criminal Database";

/// `game->GetWorld()->GetComputer("Global Criminal Database")`
fn database(g: &Ctx) -> &Computer {
    g.world()
        .get_computer(GLOBAL_CRIMINAL_DATABASE)
        .expect("UplinkAssert(comp): no Global Criminal Database")
}

fn database_mut(g: &mut Ctx) -> &mut Computer {
    g.world_mut()
        .get_computer_mut(GLOBAL_CRIMINAL_DATABASE)
        .expect("UplinkAssert(comp): no Global Criminal Database")
}

/// `GetInterfaceScreen(SCREEN_CRIMINALSCREEN)->recordindex`
fn current_record_index(g: &Ctx) -> i32 {
    g.interface()
        .ri
        .screen
        .get::<CriminalScreenInterface>()
        .expect("UplinkAssert(csi)")
        .recordindex
}

/// The slot holding the current remote screen (`RemoteInterface::screen`).
fn remote_screen_slot(g: &mut Ctx) -> &mut ScreenSlot {
    &mut g.interface_mut().ri.screen
}

/// `csi->UpdateScreen()` on the current criminal screen (taken out of its slot while it runs).
fn update_current_screen(g: &mut Ctx) {
    ScreenSlot::call(g, remote_screen_slot, |s, g| {
        if let Some(csi) = s.as_any_mut().downcast_mut::<CriminalScreenInterface>() {
            csi.update_screen(g);
        }
    });
}

/// `EclGetButton(name)->SetCaption(caption)`, skipped if the button is not there.
fn set_button_caption(g: &mut Ctx, name: &str, caption: &str) {
    if let Some(button) = g.ecl.get_button_mut(name) {
        button.set_caption(caption);
    }
}

impl CriminalScreenInterface {
    /// `SetSearchName`
    pub fn set_search_name(&mut self, newsearchname: &str) {
        self.searchname = Some(newsearchname.to_lowercase());
        self.recordindex = 0;
    }

    /// `GetComputerScreen` (typed accessor; the C++ casts `cs`).
    pub fn get_computer_screen<'a>(&self, g: &'a Ctx) -> Option<&'a GenericScreen> {
        let r = self.cs.as_ref()?;
        g.world()
            .get_computer(&r.computer)?
            .get_computer_screen(r.index)?
            .as_generic_screen()
    }

    /// `UpdateScreen`: uses `recordindex`.
    pub fn update_screen(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            return;
        }

        // Update display with the current record
        let rec = database(g)
            .recordbank
            .get_record(self.recordindex)
            .expect("UplinkAssert(rec)");
        let thisname = rec.get_field(RECORDBANK_NAME).map(str::to_string);
        let history = rec.get_field("Convictions").map(str::to_string);

        let photoindex = thisname
            .as_deref()
            .and_then(|name| g.world().get_person(name))
            .map(|person| person.photoindex);

        if let Some(name) = &thisname {
            set_button_caption(g, "criminal_name", name);
        }
        if let Some(history) = &history {
            set_button_caption(g, "criminal_history", history);
        }

        // Show a photo
        let filename = match photoindex {
            Some(index) => format!("photos/image{index}.tif"),
            None => "photos/image0.tif".to_string(),
        };
        button_assignbitmap(g, "criminal_photo", &filename);
    }

    /// `NameDraw` (static DrawFn)
    pub fn name_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        clear_draw(g, button.x, button.y, button.width, button.height);
        draw_main_title(g, button, highlighted, clicked);
    }

    /// `NameBorderDraw` (static DrawFn)
    pub fn name_border_draw(_g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        border_draw(button);
    }

    /// `HistoryDraw` (static DrawFn)
    pub fn history_draw(g: &mut Ctx, button: &Button, highlighted: bool, _clicked: bool) {
        textbutton_draw(g, button, highlighted, true);
    }

    /// `CloseClick` (static MouseFn)
    pub fn close_click(g: &mut Ctx, _button: &Button) {
        let nextpage = RemoteInterface::get_computer_screen(g)
            .and_then(|cs| cs.as_generic_screen())
            .expect("UplinkAssert(gs)")
            .nextpage;

        RemoteInterface::run_screen(g, nextpage);
    }

    /// `AddConvictionClick` (static MouseFn)
    pub fn add_conviction_click(g: &mut Ctx, _button: &Button) {
        let record_index = current_record_index(g);
        if record_index == -1 {
            return;
        }

        let newconviction = g
            .ecl
            .get_button("criminal_newconvictiontext")
            .expect("UplinkAssert(newconviction)")
            .caption
            .clone();

        if database(g).security.is_running_proxy() {
            create_msgbox(g, "Error", "Denied access by Proxy Server");
            return;
        }

        let existing = database(g)
            .recordbank
            .get_record(record_index)
            .expect("UplinkAssert(rec)")
            .get_field("Convictions")
            .expect("UplinkAssert(existing)")
            .to_string();

        let mut newconvictions = String::new();
        if !existing.contains("None") {
            newconvictions.push_str(&existing);
            newconvictions.push('\n');
        }
        newconvictions.push_str(&newconviction);

        database_mut(g)
            .recordbank
            .get_record_mut(record_index)
            .expect("UplinkAssert(rec)")
            .change_field("Convictions", &newconvictions);

        set_button_caption(g, "criminal_newconvictiontext", "");
        update_current_screen(g);
    }

    /// `ClearClick` (static MouseFn)
    pub fn clear_click(g: &mut Ctx, _button: &Button) {
        let record_index = current_record_index(g);
        if record_index == -1 {
            return;
        }

        if database(g).security.is_running_proxy() {
            create_msgbox(g, "Error", "Denied access by Proxy Server");
            return;
        }

        database_mut(g)
            .recordbank
            .get_record_mut(record_index)
            .expect("UplinkAssert(rec)")
            .change_field("Convictions", "None");

        update_current_screen(g);
    }

    /// `ArrestClick` (static MouseFn): authorises an arrest when the suspect qualifies.
    pub fn arrest_click(g: &mut Ctx, _button: &Button) {
        let record_index = current_record_index(g);
        if record_index == -1 {
            return;
        }

        if database(g).security.is_running_proxy() {
            create_msgbox(g, "Error", "Denied access by Proxy Server");
            return;
        }

        let rec = database(g)
            .recordbank
            .get_record(record_index)
            .expect("UplinkAssert(rec)");
        let name = rec.get_field(RECORDBANK_NAME).expect("UplinkAssert(name)").to_string();
        let convictions = rec.get_field("Convictions").expect("UplinkAssert(convictions)").to_string();

        // Only do this if the person now has at least 3 convicions
        // AND has broken parole
        let numconvictions = convictions.matches('\n').count() as i32;
        let brokenparole = convictions.contains("parole") || convictions.contains("Parole");

        if numconvictions >= 2 && brokenparole {
            // This person is going down
            let mut rundate = g.world().date.clone();
            rundate.advance_minute(TIME_LEGALACTION);

            let mut arrest = ArrestEvent::new();
            arrest.set_name(&name);
            arrest.set_reason("breaking parole after 2 previous convictions");
            let mut event = UplinkEvent::ArrestEvent(arrest);
            event.set_run_date(&rundate);

            g.world_mut().scheduler.schedule_event(event.clone());

            rundate.advance_minute(TIME_LEGALACTION_WARNING * -1);
            g.world_mut().scheduler.schedule_warning(&event, &rundate);

            let message = format!(
                "Authorisation Accepted\nThis man will be arrested in {} hours.",
                TIME_LEGALACTION / 60
            );
            create_msgbox(g, "Arrest Authorised", &message);
        } else {
            create_msgbox(
                g,
                "Arrest Not Authorised",
                "Authorisation rejected\nA suspect must have 2 prior convictions and must have broken parole before an arrest can be authorised remotely.",
            );
        }
    }
}

impl InterfaceScreen for CriminalScreenInterface {
    crate::impl_as_any!();

    /// `Create(ComputerScreen *newcs)` body (the `cs` is already set by `set_computer_screen`).
    fn create(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            return;
        }

        let (maintitle, subtitle) = {
            let gs = self.get_computer_screen(g).expect("UplinkAssert(gs)");
            (gs.base.maintitle.clone(), gs.base.subtitle.clone())
        };

        // Draw the screen titles
        g.ecl.register_button(80, 60, 350, 25, &maintitle, "", "criminalscreen_maintitle");
        g.ecl.register_button_callbacks("criminalscreen_maintitle", Some(draw_main_title), None, None, None);
        g.ecl.register_button(80, 80, 350, 20, &subtitle, "", "criminalscreen_subtitle");
        g.ecl.register_button_callbacks("criminalscreen_subtitle", Some(draw_sub_title), None, None, None);

        g.ecl.register_button(30, 130, 128, 160, "", "Shows the person's photograph", "criminal_photo");
        button_assignbitmap(g, "criminal_photo", "photos/image0.tif");

        g.ecl.register_button(180, 130, 210, 15, "Name", " ", "criminal_nametitle");
        g.ecl.register_button_callbacks("criminal_nametitle", Some(button_draw), None, None, None);
        g.ecl.register_button(190, 146, 190, 38, " ", " ", "criminal_name");
        g.ecl.register_button_callbacks("criminal_name", Some(CriminalScreenInterface::name_draw), None, None, None);
        g.ecl.register_button(180, 145, 210, 40, " ", " ", "criminal_nameborder");
        g.ecl.register_button_callbacks("criminal_nameborder", Some(CriminalScreenInterface::name_border_draw), None, None, None);

        // History text box
        g.ecl.register_button(180, 200, 210, 15, "Criminal History", "", "criminal_historytitle");
        g.ecl.register_button_callbacks("criminal_historytitle", Some(button_draw), None, None, None);
        g.ecl.register_button(180, 215, 210, 140, "", "", "criminal_history");
        g.ecl.register_button_callbacks("criminal_history", Some(CriminalScreenInterface::history_draw), None, None, None);

        if g.interface().ri.security_level <= 2 {
            g.ecl.register_button(180, 354, 180, 16, "", "Enter new convictions here", "criminal_newconvictiontext");
            g.ecl.register_button_callbacks(
                "criminal_newconvictiontext",
                Some(CriminalScreenInterface::history_draw),
                None,
                None,
                Some(button_highlight),
            );

            g.ecl.register_button(360, 355, 30, 15, "Add", "Click to add this conviction", "criminal_newconvictionadd");
            g.ecl.register_button_callback("criminal_newconvictionadd", CriminalScreenInterface::add_conviction_click);

            // Control buttons
            g.ecl.register_button(30, 315, 128, 15, "Clear history", "Click here to clear this man's record", "criminal_clear");
            g.ecl.register_button_callback("criminal_clear", CriminalScreenInterface::clear_click);

            g.ecl.register_button(30, 335, 128, 15, "Authorise Arrest", "Click here to authorise this man's arrest", "criminal_arrest");
            g.ecl.register_button_callback("criminal_arrest", CriminalScreenInterface::arrest_click);
        }

        g.ecl.register_button(30, 355, 128, 15, "Done", "Close this screen", "criminal_close");
        g.ecl.register_button_callback("criminal_close", CriminalScreenInterface::close_click);

        g.ecl.make_button_editable("criminal_newconvictiontext");
    }

    fn remove(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            return;
        }

        for name in [
            "criminalscreen_maintitle",
            "criminalscreen_subtitle",
            "criminal_photo",
            "criminal_name",
            "criminal_nameborder",
            "criminal_nametitle",
            "criminal_historytitle",
            "criminal_history",
            "criminal_newconvictiontext",
            "criminal_newconvictionadd",
            "criminal_clear",
            "criminal_arrest",
            "criminal_close",
        ] {
            g.ecl.remove_button(name);
        }
    }

    fn update(&mut self, g: &mut Ctx) {
        let Some(searchname) = self.searchname.clone() else {
            return;
        };
        if !self.is_visible(g) {
            return;
        }

        let timems = g.ecl.get_accurate_time() - self.lastupdate;
        if timems <= 100 {
            return;
        }

        // We are searching for a record
        // Is this one it?

        // Have we searched all records?
        if self.recordindex == database(g).recordbank.records.len() as i32 {
            self.searchname = None;
            self.recordindex = -1;
            return;
        }

        let thisname = database(g)
            .recordbank
            .get_record(self.recordindex)
            .expect("UplinkAssert(rec)")
            .get_field(RECORDBANK_NAME)
            .expect("UplinkAssert(thisname)")
            .to_lowercase();

        // Update display with the current record
        self.update_screen(g);

        if searchname == thisname {
            // Record found - stop searching
            self.searchname = None;
        } else {
            // Record not found
            self.recordindex += 1;
        }

        self.lastupdate = g.ecl.get_accurate_time();
    }

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("criminal_photo").is_some()
    }

    fn escape_key_pressed(&mut self, g: &mut Ctx) -> bool {
        // C++: CloseClick(NULL). The screen is taken out while this runs, so read `nextpage`
        // from `self` rather than through RemoteInterface.
        let nextpage = self
            .get_computer_screen(g)
            .map(|gs| gs.nextpage)
            .expect("UplinkAssert(gs)");

        RemoteInterface::run_screen(g, nextpage);
        true
    }

    fn screen_id(&self) -> i32 {
        SCREEN_CRIMINALSCREEN
    }

    fn set_computer_screen(&mut self, cs: ScreenRef) {
        self.cs = Some(cs);
    }

    fn computer_screen_ref(&self) -> Option<&ScreenRef> {
        self.cs.as_ref()
    }
}
