//! Port of source/interface/remoteinterface/recordscreen_interface.h + source/interface/remoteinterface/recordscreen_interface.cpp

use crate::app::opengl_interface;
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::remoteinterface::remoteinterface::{RemoteInterface, SCREEN_RECORDSCREEN};
use crate::interface::remoteinterface::remoteinterfacescreen::{draw_main_title, draw_sub_title};
use crate::objref::ScreenRef;
use crate::world::computer::computerscreen::genericscreen::GenericScreen;
use crate::world::computer::logbank::{AccessLog, LOG_NOTSUSPICIOUS, LOG_TYPE_TEXT};

/// Remote record screen: edits one record of a database (C++ static `recordindex`).
#[derive(Debug, Default)]
pub struct RecordScreenInterface {
    pub cs: Option<ScreenRef>,
    pub recordindex: i32,
}

/// The screen the remote interface currently shows (the C++ `GetRemoteInterface ()->GetComputerScreen ()`
/// as seen from the static callbacks): its `ScreenRef` and `recordindex`.
fn current(g: &Ctx) -> Option<(ScreenRef, i32)> {
    let me = g.interface().ri.screen.get::<RecordScreenInterface>()?;
    Some((me.cs.clone()?, me.recordindex))
}

/// Adds an access log (stamped now, from the player's ghost ip) to a computer's logbank.
fn add_player_log(g: &mut Ctx, computer: &str, data1: &str) {
    let ghost = g.player().get_connection().get_ghost().unwrap_or(" ").to_string();
    let mut log = AccessLog::new();
    log.set_properties(&g.world().date.clone(), &ghost, "PLAYER", LOG_NOTSUSPICIOUS, LOG_TYPE_TEXT);
    log.set_data1(Some(data1));
    if let Some(comp) = g.world_mut().get_computer_mut(computer) {
        comp.logbank.add_log(log, -1);
    }
}

/// `CreateRecord` for the computer `computer`: removes the old boxes and builds the boxes for
/// record `index` (nothing is built when there is no such record). The caller stores `recordindex`.
fn build_record(g: &mut Ctx, computer: &str, index: i32) {
    // First remove all the old boxes

    let mut i = 0;

    loop {
        let bname_title = format!("recordscreen_title {}", i);
        let bname_value = format!("recordscreen_value {}", i);

        if g.ecl.get_button(&bname_title).is_some() && g.ecl.get_button(&bname_value).is_some() {
            g.ecl.remove_button(&bname_title);
            g.ecl.remove_button(&bname_value);

            i += 1;
        } else {
            break;
        }
    }

    // Create the new record

    // ConvertIndexToDArray / ConvertToDArray: BTree pre-order (insertion dependent)
    let fields: Vec<(String, String)> = match g
        .world()
        .get_computer(computer)
        .and_then(|comp| comp.recordbank.get_record(index))
    {
        Some(rec) => rec.fields.iter().map(|(k, v)| (k.to_string(), v.clone())).collect(),
        None => return,
    };

    let mut current_y = 150;

    for (j, (title, value)) in fields.iter().enumerate() {
        let bname_title = format!("recordscreen_title {}", j);
        let bname_value = format!("recordscreen_value {}", j);

        // Count the number of newlines
        let mut numnewlines = 1; // (the last one)
        for p in value.bytes() {
            if p == b'\n' {
                numnewlines += 1;
            }
        }

        let buttonheight = 15 * numnewlines;

        g.ecl.register_button(20, current_y, 120, buttonheight, title, "", &bname_title);
        g.ecl.register_button(140, current_y, 300, buttonheight, value, "", &bname_value);

        g.ecl.register_button_callbacks(&bname_title, Some(opengl_interface::textbutton_draw), None, None, None);
        g.ecl.register_button_callbacks(
            &bname_value,
            Some(opengl_interface::textbutton_draw),
            None,
            Some(opengl_interface::button_click),
            Some(opengl_interface::button_highlight),
        );

        current_y += buttonheight + 5;
    }
}

/// Static `CreateRecord` as called from the scroll callbacks: builds the boxes and stores the new `recordindex`.
fn create_record_current(g: &mut Ctx, computer: &str, index: i32) {
    build_record(g, computer, index);
    if let Some(me) = g.interface_mut().ri.screen.get_mut::<RecordScreenInterface>() {
        me.recordindex = index;
    }
}

impl RecordScreenInterface {
    /// `GetComputerScreen` (typed accessor; the C++ casts `cs`).
    pub fn get_computer_screen<'a>(&self, g: &'a Ctx) -> Option<&'a GenericScreen> {
        let cs = self.cs.as_ref()?;
        g.world().get_computer(&cs.computer)?.get_computer_screen(cs.index)?.as_generic_screen()
    }

    /// `CreateRecord` (a method in the port; the C++ is a member taking an index)
    pub fn create_record(&mut self, g: &mut Ctx, index: i32) {
        let Some(computer) = self.cs.as_ref().map(|cs| cs.computer.clone()) else {
            return;
        };
        build_record(g, &computer, index);
        self.recordindex = index;
    }

    /// `ScrollLeftClick` (static MouseFn)
    pub fn scroll_left_click(g: &mut Ctx, _button: &Button) {
        let Some((cs, recordindex)) = current(g) else {
            return;
        };

        if recordindex > 0 {
            create_record_current(g, &cs.computer, recordindex - 1);
        }
    }

    /// `ScrollRightClick` (static MouseFn)
    pub fn scroll_right_click(g: &mut Ctx, _button: &Button) {
        let Some((cs, recordindex)) = current(g) else {
            return;
        };
        let rec_exists = g
            .world()
            .get_computer(&cs.computer)
            .is_some_and(|comp| comp.recordbank.get_record(recordindex + 1).is_some());

        if rec_exists {
            create_record_current(g, &cs.computer, recordindex + 1);
        }
    }

    /// `CommitClick` (static MouseFn)
    pub fn commit_click(g: &mut Ctx, _button: &Button) {
        // Go through each button-pair on screen, updating each
        // (Get the title from the left box and the value from the right)

        let Some((cs, recordindex)) = current(g) else {
            return;
        };

        let Some(comp) = g.world().get_computer(&cs.computer) else {
            debug_assert!(false, "RecordScreenInterface::commit_click, no computer");
            return;
        };

        if comp.security.is_running_proxy() {
            opengl_interface::create_msgbox(g, "Error", "Denied access by Proxy Server");
            return;
        }

        if comp.recordbank.get_record(recordindex).is_none() {
            debug_assert!(false, "RecordScreenInterface::commit_click, no record {}", recordindex);
            return;
        }

        let mut changes: Vec<(String, String)> = Vec::new();
        let mut i = 0;

        loop {
            let bname_title = format!("recordscreen_title {}", i);
            let bname_value = format!("recordscreen_value {}", i);

            match (g.ecl.get_button(&bname_title), g.ecl.get_button(&bname_value)) {
                (Some(title), Some(value)) => changes.push((title.caption.clone(), value.caption.clone())),
                _ => break,
            }

            i += 1;
        }

        if let Some(rec) = g
            .world_mut()
            .get_computer_mut(&cs.computer)
            .and_then(|comp| comp.recordbank.get_record_mut(recordindex))
        {
            for (title, value) in &changes {
                rec.change_field(title, value);
            }
        }

        // Log this change

        let logcaption = format!("Changed Record #{}", recordindex);
        add_player_log(g, &cs.computer, &logcaption);
    }

    /// `CloseClick` (static MouseFn)
    pub fn close_click(g: &mut Ctx, button: &Button) {
        let Some(nextpage) = button.name.split_whitespace().nth(1).and_then(|s| s.parse::<i32>().ok()) else {
            return;
        };

        if nextpage != -1 {
            RemoteInterface::run_screen(g, nextpage);
        }
    }
}

impl InterfaceScreen for RecordScreenInterface {
    crate::impl_as_any!();

    fn create(&mut self, g: &mut Ctx) {
        let Some(cs) = self.cs.clone() else {
            return;
        };

        if !self.is_visible(g) {
            let Some(gs) = self.get_computer_screen(g) else {
                debug_assert!(false, "RecordScreenInterface::create, no GenericScreen");
                return;
            };
            let maintitle = gs.base.maintitle.clone();
            let subtitle = gs.base.subtitle.clone();
            let nextpage = gs.nextpage;

            // Create the titles

            g.ecl.register_button(80, 60, 350, 25, &maintitle, "", "recordscreen_maintitle");
            g.ecl.register_button_callbacks("recordscreen_maintitle", Some(draw_main_title), None, None, None);
            g.ecl.register_button(80, 80, 350, 20, &subtitle, "", "recordscreen_subtitle");
            g.ecl.register_button_callbacks("recordscreen_subtitle", Some(draw_sub_title), None, None, None);

            // Create record 0

            self.create_record(g, 0);

            // Create the record scroll buttons

            g.ecl.register_button(20, 370, 50, 20, "Prev", "Scroll to the previous record", "recordscreen_scrollleft");
            g.ecl.register_button(75, 370, 50, 20, "Next", "Scroll to the next record", "recordscreen_scrollright");

            g.ecl.register_button_callback("recordscreen_scrollleft", Self::scroll_left_click);
            g.ecl.register_button_callback("recordscreen_scrollright", Self::scroll_right_click);

            // Create the control buttons

            g.ecl.register_button(230, 370, 80, 20, "Commit", "Commit the changes you have made", "recordscreen_commit");
            g.ecl.register_button_callback("recordscreen_commit", Self::commit_click);

            let name = format!("recordscreen_click {}", nextpage);
            g.ecl.register_button(320, 370, 80, 20, "Close", "Close the records screen", &name);
            g.ecl.register_button_callback(&name, Self::close_click);

            // Log this access

            add_player_log(g, &cs.computer, "Accessed logs");
        }
    }

    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            g.ecl.remove_button("recordscreen_maintitle");
            g.ecl.remove_button("recordscreen_subtitle");

            self.create_record(g, -1);

            g.ecl.remove_button("recordscreen_scrollleft");
            g.ecl.remove_button("recordscreen_scrollright");

            g.ecl.remove_button("recordscreen_commit");

            let Some(nextpage) = self.get_computer_screen(g).map(|gs| gs.nextpage) else {
                debug_assert!(false, "RecordScreenInterface::remove, no GenericScreen");
                return;
            };
            let name = format!("recordscreen_click {}", nextpage);
            g.ecl.remove_button(&name);
        }
    }

    fn update(&mut self, _g: &mut Ctx) {}

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("recordscreen_maintitle").is_some()
    }

    fn screen_id(&self) -> i32 {
        SCREEN_RECORDSCREEN
    }

    fn set_computer_screen(&mut self, cs: ScreenRef) {
        self.cs = Some(cs);
    }

    fn computer_screen_ref(&self) -> Option<&ScreenRef> {
        self.cs.as_ref()
    }
}
