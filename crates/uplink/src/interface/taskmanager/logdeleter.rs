//! Port of source/interface/taskmanager/logdeleter.h + source/interface/taskmanager/logdeleter.cpp

use uplink_engine::gl;
use uplink_engine::text::{draw_text_styled, HELVETICA_10};

use crate::app::globals::ALPHA;
use crate::app::opengl_interface::{self, button_click, button_highlight};
use crate::ctx::Ctx;
use crate::eclipse::{Button, MOVE_STRAIGHTLINE};
use crate::game::data::data::{IP_LOCALHOST, TICKSREQUIRED_LOGDELETER};
use crate::impl_as_any;
use crate::interface::remoteinterface::remoteinterface::RemoteInterface;
use crate::interface::taskmanager::uplinktask::{UplinkTask, UplinkTaskBase};
use crate::objref::ObjRef;
use crate::vanbakel::Vanbakel;
use crate::world::computer::logbank::{
    AccessLog, LogBank, LOG_NOTSUSPICIOUS, LOG_TYPE_DELETED, LOG_TYPE_TEXT,
};
use crate::world::connection::Connection;
use crate::world::generator::worldgenerator::WorldGenerator;

pub const LOGDELETER_OFF: i32 = 0;
pub const LOGDELETER_INPROGRESS: i32 = 1;
pub const LOGDELETER_REPLACING: i32 = 2;
pub const LOGDELETER_FINISHED: i32 = 3;

#[derive(Debug, Default)]
pub struct LogDeleter {
    pub base: UplinkTaskBase,
    /// C++ `LogBank *source`: `ObjRef::LogBank(computer name)` or `ObjRef::AccountLog`, `ObjRef::None` for NULL.
    pub source: ObjRef,
    pub sourceindex: i32,
    /// Used by later versions to keep track of searches.
    pub currentreplaceindex: i32,
    /// Used by later versions to keep track of searches.
    pub currentreplaceprogress: i32,
    /// 0 = off, 1 = in progress, 2 = replacing, 3 = finished.
    pub status: i32,
    pub numticksrequired: i32,
    /// 0.0 - numticksrequired.
    pub progress: i32,
}

/// `sscanf(button->name, "%s %d", bname, &pid)`: the PID is the number after the button name.
fn pid_from_name(name: &str) -> i32 {
    name.split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(-1)
}

/// `"logdeleter_<name> <pid>"`: the button naming scheme of this task.
fn btn(name: &str, pid: i32) -> String {
    format!("logdeleter_{name} {pid}")
}

/// The bank of the computer that `source` names (`None` for a NULL source or a missing computer).
fn source_bank<'a>(g: &'a Ctx, source: &ObjRef) -> Option<&'a LogBank> {
    source.log_bank(g.world())
}

fn source_bank_mut<'a>(g: &'a mut Ctx, source: &ObjRef) -> Option<&'a mut LogBank> {
    source.log_bank_mut(g.world_mut())
}

/// `source->logs.ValidIndex(index)`. A source whose computer is gone counts as "log not there".
fn source_has_log(g: &Ctx, source: &ObjRef, index: i32) -> bool {
    source_bank(g, source).is_some_and(|bank| bank.logs.valid_index(index))
}

/// `EclGetButton(sprogress)->width = 120 * ((float) progress / (float) numticks)` + `EclDirtyButton`.
fn set_progress_bar(g: &mut Ctx, sprogress: &str, progress: i32, numticks: i32) {
    let width = 120.0 * (progress as f32 / numticks as f32);
    let b = g
        .ecl
        .get_button_mut(sprogress)
        .expect("LogDeleter: progress button");
    b.width = width as i32;
    g.ecl.dirty_button(sprogress);
}

/// `EclDirtyButton("logscreen_log i")` for i in 0..15 (the "hack!" refresh of the log screen).
fn dirty_logscreen_buttons(g: &mut Ctx) {
    for i in 0..15 {
        g.ecl.dirty_button(&format!("logscreen_log {i}"));
    }
}

impl LogDeleter {
    pub fn new() -> Self {
        Self {
            sourceindex: -1,
            ..Default::default()
        }
    }

    pub fn border_draw(_g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        gl::begin(gl::QUADS);

        if clicked {
            gl::color4f(0.5, 0.5, 0.6, ALPHA);
        } else if highlighted {
            gl::color4f(0.2, 0.2, 0.5, ALPHA);
        } else {
            gl::color4f(0.2, 0.2, 0.4, ALPHA);
        }
        gl::vertex2i(button.x, button.y);

        if clicked {
            gl::color4f(0.7, 0.7, 0.6, ALPHA);
        } else if highlighted {
            gl::color4f(0.5, 0.5, 0.6, ALPHA);
        } else {
            gl::color4f(0.3, 0.3, 0.5, ALPHA);
        }
        gl::vertex2i(button.x + button.width, button.y);

        if clicked {
            gl::color4f(0.5, 0.5, 0.6, ALPHA);
        } else if highlighted {
            gl::color4f(0.2, 0.2, 0.5, ALPHA);
        } else {
            gl::color4f(0.2, 0.2, 0.4, ALPHA);
        }
        gl::vertex2i(button.x + button.width, button.y + button.height);

        if clicked {
            gl::color4f(0.7, 0.7, 0.6, ALPHA);
        } else if highlighted {
            gl::color4f(0.5, 0.5, 0.6, ALPHA);
        } else {
            gl::color4f(0.3, 0.3, 0.5, ALPHA);
        }
        gl::vertex2i(button.x, button.y + button.height);

        gl::end();

        if highlighted || clicked {
            gl::color4f(0.3, 0.3, 0.7, 1.0);
            opengl_interface::border_draw(button);
        }
    }

    pub fn progress_draw(_g: &mut Ctx, button: &Button, highlighted: bool, _clicked: bool) {
        let mut scale: f32 = button.width as f32 / 100.0;
        if highlighted {
            scale *= 2.0;
        }

        gl::begin(gl::QUADS);
        let corners = [
            (button.x, button.y),
            (button.x + button.width, button.y),
            (button.x + button.width, button.y + button.height),
            (button.x, button.y + button.height),
        ];
        for (x, y) in corners {
            gl::color4f(0.0, 1.5 - scale, scale, 0.6);
            gl::vertex3i(x, y, 0);
        }
        gl::end();

        let xpos = button.x + 5;
        let ypos = (button.y + button.height / 2) + 3;

        gl::color4f(1.0, 1.0, 1.0, 1.0);
        draw_text_styled(xpos, ypos, &button.caption, HELVETICA_10);
    }

    pub fn close_click(g: &mut Ctx, button: &Button) {
        let pid = pid_from_name(&button.name);
        Vanbakel::remove_task(g, pid);
    }

    pub fn border_click(g: &mut Ctx, button: &Button) {
        let pid = pid_from_name(&button.name);
        g.interface_mut().tm.set_target_program(pid);
    }

    /// The `status == INPROGRESS` branch once `progress >= numticksrequired`: replace the log
    /// according to the task version, then switch the task to its next status.
    fn finish_deletion(&mut self, g: &mut Ctx, sprogress: &str) {
        let idx = self.sourceindex;
        // We must keep the date the same (logs are in date order)
        let logdate = source_bank(g, &self.source)
            .and_then(|bank| bank.logs.get_data(idx))
            .map(|log| log.date.clone())
            .expect("LogDeleter::finish_deletion: no log");

        let version = self.base.version;
        if version == 1.0 || version == 2.0 {
            let fromip = if version == 1.0 { "Unknown" } else { " " };

            // Delete the log, then replace it with a "Deleted" marker
            let bank = source_bank_mut(g, &self.source).expect("LogDeleter: source LogBank");
            bank.logs.remove_data(idx);
            let mut al = AccessLog::new();
            al.set_properties(&logdate, fromip, " ", LOG_NOTSUSPICIOUS, LOG_TYPE_DELETED);
            bank.logs.put_data_at(al, idx);

            // Finished deleting now
            self.status = LOGDELETER_FINISHED;
            g.ecl.register_caption_change_auto(sprogress, "Finished", None);
        } else if version == 3.0 {
            // Delete the log
            {
                let bank = source_bank_mut(g, &self.source).expect("LogDeleter: source LogBank");
                bank.logs.remove_data(idx);
            }

            // Look for a valid log to put in the blank place (one made by someone else)
            let replacement = source_bank(g, &self.source).and_then(|bank| {
                bank.logs
                    .iter()
                    .find(|(_, copyme)| copyme.fromip != IP_LOCALHOST)
                    .map(|(_, copyme)| {
                        let mut al = AccessLog::new();
                        al.set_properties_from(copyme);
                        al.date.set_date_from(&logdate);
                        al.set_suspicious(LOG_NOTSUSPICIOUS);
                        al
                    })
            });

            // Could not find log to overwrite with - so make up a new one
            let al = match replacement {
                Some(al) => al,
                None => {
                    let ip = WorldGenerator::get_random_location(g);
                    let mut al = AccessLog::new();
                    al.set_properties(&logdate, &ip, " ", LOG_NOTSUSPICIOUS, LOG_TYPE_TEXT);
                    al.set_data1(Some("Accessed File"));
                    al
                }
            };
            let bank = source_bank_mut(g, &self.source).expect("LogDeleter: source LogBank");
            bank.logs.put_data_at(al, idx);

            // Finished deleting now
            self.status = LOGDELETER_FINISHED;
        } else if version == 4.0 {
            // Delete the log
            let bank = source_bank_mut(g, &self.source).expect("LogDeleter: source LogBank");
            bank.logs.remove_data(idx);

            // Start moving all logs below this one upwards
            self.status = LOGDELETER_REPLACING;
            g.ecl
                .register_caption_change_auto(sprogress, "Shifting logs (0%)", None);
            self.currentreplaceindex = idx;
            self.currentreplaceprogress = 0;
        } else {
            println!("LogDeleter WARNING : Unrecognised version number");
        }

        // Dirty the log-screen buttons (hack!)
        dirty_logscreen_buttons(g);
    }
}

impl UplinkTask for LogDeleter {
    fn base(&self) -> &UplinkTaskBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut UplinkTaskBase {
        &mut self.base
    }

    fn set_target(&mut self, g: &mut Ctx, target: ObjRef, targetstring: &str, targetint: i32) {
        // uo  : LogBank object containing AccessLog to be deleted
        // uos : name of button representing data
        // uoi : index of log in logbank
        if self.status != LOGDELETER_OFF {
            return;
        }
        // Logbank selected, with a log at that index
        if !target.is_log_bank() || !source_has_log(g, &target, targetint) {
            return;
        }

        self.source = target;
        self.sourceindex = targetint;

        self.numticksrequired = TICKSREQUIRED_LOGDELETER;
        self.progress = 0;

        let (bx, by) = {
            let b = g
                .ecl
                .get_button(targetstring)
                .expect("LogDeleter::set_target: target button");
            (b.x, b.y)
        };
        self.move_to(g, bx, by, 1000);

        let monitor_running = RemoteInterface::get_computer_screen(g)
            .and_then(|screen| screen.base().get_computer(g.world()))
            .expect("LogDeleter::set_target: computer")
            .security
            .is_running_monitor();
        if monitor_running {
            Connection::begin_trace(g, "PLAYER");
        }
    }

    fn move_to(&mut self, g: &mut Ctx, x: i32, y: i32, time_ms: i32) {
        // UplinkTask::MoveTo is empty in the C++ base class, so there is nothing to call here.
        let pid = self.base.pid;

        g.ecl
            .register_movement(&btn("title", pid), x, y, time_ms, MOVE_STRAIGHTLINE, None);
        g.ecl
            .register_movement(&btn("border", pid), x + 20, y, time_ms, MOVE_STRAIGHTLINE, None);
        g.ecl.register_movement(
            &btn("progress", pid),
            x + 20,
            y + 1,
            time_ms,
            MOVE_STRAIGHTLINE,
            None,
        );
        g.ecl.register_movement(
            &btn("close", pid),
            x + 140,
            y + 1,
            time_ms,
            MOVE_STRAIGHTLINE,
            None,
        );

        g.ecl.button_bring_to_front(&btn("title", pid));
        g.ecl.button_bring_to_front(&btn("border", pid));
        g.ecl.button_bring_to_front(&btn("progress", pid));
        g.ecl.button_bring_to_front(&btn("close", pid));
    }

    fn initialise(&mut self, _g: &mut Ctx) {}

    fn tick(&mut self, g: &mut Ctx, n: i32) {
        if !self.is_interface_visible(g) {
            return;
        }

        let pid = self.base.pid;
        let sprogress = btn("progress", pid);

        for _ in 0..n {
            match self.status {
                LOGDELETER_OFF => {
                    // Not downloading - look for a new target
                    if !matches!(self.source, ObjRef::None) {
                        // A new source file has been specified - start downloading it
                        self.status = LOGDELETER_INPROGRESS;
                        g.ecl
                            .register_caption_change(&sprogress, "Removing log...", 0, None);
                    }
                }
                LOGDELETER_INPROGRESS => {
                    // Check log is still there
                    if !source_has_log(g, &self.source, self.sourceindex) {
                        self.status = LOGDELETER_FINISHED;
                        break;
                    }

                    // Check we are still connected
                    if !g.player().is_connected() {
                        Vanbakel::remove_task(g, pid);
                        return;
                    }

                    // Delete in progress
                    self.progress += 1;
                    set_progress_bar(g, &sprogress, self.progress, self.numticksrequired);

                    if self.progress >= self.numticksrequired {
                        self.finish_deletion(g, &sprogress);
                    }
                }
                LOGDELETER_REPLACING => {
                    // Check we are still connected
                    if !g.player().is_connected() {
                        Vanbakel::remove_task(g, pid);
                        return;
                    }

                    // Check to see if an update occurs
                    self.currentreplaceprogress += 1;

                    if self.currentreplaceprogress > 20 {
                        // Check to see if we have finished
                        let size = source_bank(g, &self.source)
                            .expect("LogDeleter: source LogBank")
                            .logs
                            .size();

                        if self.currentreplaceindex >= size - 1 {
                            // Remove the gap from the end permenantly
                            let bank = source_bank_mut(g, &self.source)
                                .expect("LogDeleter: source LogBank");
                            let newsize = bank.logs.size() - 1;
                            bank.logs.set_size(newsize);
                            bank.internallogs.set_size(bank.logs.size());

                            self.status = LOGDELETER_FINISHED;
                        } else {
                            // Shift the next one up into this position
                            let cri = self.currentreplaceindex;
                            let bank = source_bank_mut(g, &self.source)
                                .expect("LogDeleter: source LogBank");
                            if bank.logs.valid_index(cri + 1) {
                                if let Some(al) = bank.logs.remove_data(cri + 1) {
                                    bank.logs.put_data_at(al, cri);
                                }

                                // Move the backups up as well
                                if let Some(al2) = bank.internallogs.remove_data(cri + 1) {
                                    bank.internallogs.put_data_at(al2, cri);
                                }

                                // Dirty the log-screen buttons (hack!)
                                dirty_logscreen_buttons(g);
                            }

                            self.currentreplaceindex += 1;
                            let size = source_bank(g, &self.source)
                                .expect("LogDeleter: source LogBank")
                                .logs
                                .size();
                            let percentagedone = 100 * (self.currentreplaceindex - self.sourceindex)
                                / (size - self.sourceindex);
                            let caption = format!("Shifting logs...({percentagedone}%)");
                            g.ecl
                                .register_caption_change(&sprogress, &caption, 0, None);
                        }

                        self.currentreplaceprogress = 0;
                    }
                }
                LOGDELETER_FINISHED => {
                    self.source = ObjRef::None;
                    self.sourceindex = -1;
                    self.numticksrequired = 0;
                    self.progress = 0;
                    g.ecl.register_caption_change_auto(&sprogress, "Finished", None);
                    self.status = LOGDELETER_OFF;
                }
                _ => {}
            }
        }
    }

    fn create_interface(&mut self, g: &mut Ctx) {
        if self.is_interface_visible(g) {
            return;
        }

        let pid = self.base.pid;
        let tooltip = format!("Log Deleter v{:.1}", self.base.version);

        let stitle = btn("title", pid);
        let sborder = btn("border", pid);
        let sprogress = btn("progress", pid);
        let sclose = btn("close", pid);

        g.ecl.register_button(265, 450, 20, 15, "", &tooltip, &stitle);
        opengl_interface::button_assignbitmap(g, &stitle, "software/log.tif");

        g.ecl.register_button(285, 450, 120, 15, "", "", &sborder);
        g.ecl.register_button_callbacks(
            &sborder,
            Some(Self::border_draw),
            Some(Self::border_click),
            Some(button_click),
            Some(button_highlight),
        );

        g.ecl.register_button(
            285,
            450,
            120,
            13,
            "Select target log",
            "Shows the progress of the log delete",
            &sprogress,
        );
        g.ecl.register_button_callbacks(
            &sprogress,
            Some(Self::progress_draw),
            Some(Self::border_click),
            Some(button_click),
            Some(button_highlight),
        );

        g.ecl.register_button(
            405,
            450,
            13,
            13,
            "",
            "Close the LogDeleter (and stop deleting)",
            &sclose,
        );
        opengl_interface::button_assignbitmaps(g, &sclose, "close.tif", "close_h.tif", "close_c.tif");
        g.ecl.register_button_callback(&sclose, Self::close_click);
    }

    fn remove_interface(&mut self, g: &mut Ctx) {
        if self.is_interface_visible(g) {
            let pid = self.base.pid;

            g.ecl.remove_button(&btn("title", pid));
            g.ecl.remove_button(&btn("border", pid));
            g.ecl.remove_button(&btn("progress", pid));
            g.ecl.remove_button(&btn("close", pid));
        }
    }

    fn show_interface(&mut self, g: &mut Ctx) {
        if !self.is_interface_visible(g) {
            self.create_interface(g);
        }

        let pid = self.base.pid;

        g.ecl.button_bring_to_front(&btn("title", pid));
        g.ecl.button_bring_to_front(&btn("border", pid));
        g.ecl.button_bring_to_front(&btn("progress", pid));
        g.ecl.button_bring_to_front(&btn("close", pid));
    }

    fn is_interface_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button(&btn("border", self.base.pid)).is_some()
    }

    impl_as_any!();
}
