//! Port of source/interface/taskmanager/logmodifier.h + source/interface/taskmanager/logmodifier.cpp

use uplink_engine::gl;
use uplink_engine::text::{draw_text_styled, HELVETICA_10};

use crate::app::globals::ALPHA;
use crate::app::miscutils::strip_carriage_returns;
use crate::app::opengl_interface::{self, button_click, button_highlight, textbutton_draw};
use crate::ctx::Ctx;
use crate::eclipse::{Button, MOVE_STRAIGHTLINE};
use crate::game::data::data::TICKSREQUIRED_LOGMODIFIER;
use crate::impl_as_any;
use crate::interface::taskmanager::uplinktask::{UplinkTask, UplinkTaskBase};
use crate::objref::ObjRef;
use crate::vanbakel::Vanbakel;
use crate::world::computer::logbank::{
    AccessLog, LogBank, LOG_NOTSUSPICIOUS, LOG_TYPE_BOUNCE, LOG_TYPE_BOUNCEBEGIN, LOG_TYPE_TEXT,
};

pub const LOGMODIFIER_STATUS_NONE: i32 = 0;
pub const LOGMODIFIER_STATUS_WAITING: i32 = 1;
/// (expanded)
pub const LOGMODIFIER_STATUS_ACTIVE: i32 = 2;
pub const LOGMODIFIER_STATUS_MODIFYING: i32 = 3;
pub const LOGMODIFIER_STATUS_FINISHED: i32 = 4;
/// (V2.0)
pub const LOGMODIFIER_STATUS_CREATING: i32 = 5;

/// C++ `static char *logtypes[]`, indexed by `logtype`.
pub const LOG_TYPES: [&str; 9] = [
    "Deleted",
    "None",
    "Text",
    "ConectionOpened",
    "ConnectionClosed",
    "BounceBegin",
    "Bounce",
    "TransferTo",
    "TransferFrom",
];
/// C++ `static int numlogtypes = 9 - 1;` (valid types are -1 to 7).
pub const NUM_LOG_TYPES: i32 = 9 - 1;

/// The buttons of the expanded (log text) panel, in creation order.
const EXPANDED_BUTTONS: [&str; 10] = [
    "background",
    "tltype",
    "ltype",
    "tfromip",
    "fromip",
    "tdata1",
    "data1",
    "nexttype",
    "commit",
    "close2",
];

#[derive(Debug, Default)]
pub struct LogModifier {
    pub base: UplinkTaskBase,
    /// C++ `LogBank *source`: `ObjRef::LogBank(computer name)` or `ObjRef::AccountLog`, `ObjRef::None` for NULL.
    pub source: ObjRef,
    pub sourceindex: i32,
    pub status: i32,
    pub numticksrequired: i32,
    pub progress: i32,
    pub logtype: i32,
}

/// `sscanf(button->name, "%s %d", bname, &pid)`: the PID is the number after the button name.
fn pid_from_name(name: &str) -> i32 {
    name.split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(-1)
}

/// `"logmodifier_<name> <pid>"`: the button naming scheme of this task.
fn btn(name: &str, pid: i32) -> String {
    format!("logmodifier_{name} {pid}")
}

fn expanded_names(pid: i32) -> Vec<String> {
    EXPANDED_BUTTONS.iter().map(|n| btn(n, pid)).collect()
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
        .expect("LogModifier: progress button");
    b.width = width as i32;
    g.ecl.dirty_button(sprogress);
}

/// `ChangeLogType(pid)` body, for a task whose logtype is already known. The task itself is not looked
/// up because the caller may be a method of that task (it is taken out of the vanbakel list while running).
fn change_log_type_to(g: &mut Ctx, pid: i32, logtype: i32) {
    let ltype = btn("ltype", pid);
    let caption = usize::try_from(logtype + 1)
        .ok()
        .and_then(|i| LOG_TYPES.get(i))
        .copied()
        .unwrap_or("");
    g.ecl
        .get_button_mut(&ltype)
        .expect("LogModifier: ltype button")
        .set_caption(caption);

    let data1 = btn("tdata1", pid);
    // Deleted, None, Text, ConnectionOpened and ConnectionClosed (and any other type) show "NA".
    let data1_caption = match logtype {
        LOG_TYPE_BOUNCEBEGIN | LOG_TYPE_BOUNCE => "To",
        _ => "NA",
    };
    g.ecl
        .get_button_mut(&data1)
        .expect("LogModifier: tdata1 button")
        .set_caption(data1_caption);
}

impl LogModifier {
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

    pub fn background_draw(_g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        gl::begin(gl::QUADS);
        gl::color3ub(8, 20, 124);
        gl::vertex2i(button.x - 1, button.y);
        gl::color3ub(8, 20, 0);
        gl::vertex2i(button.x + button.width, button.y);
        gl::color3ub(8, 20, 124);
        gl::vertex2i(button.x + button.width, button.y + button.height);
        gl::color3ub(8, 20, 0);
        gl::vertex2i(button.x, button.y + button.height);
        gl::end();

        gl::color3ub(81, 138, 215);
        opengl_interface::border_draw(button);
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

    pub fn border_click(g: &mut Ctx, button: &Button) {
        let pid = pid_from_name(&button.name);
        g.interface_mut().tm.set_target_program(pid);
    }

    pub fn close_click(g: &mut Ctx, button: &Button) {
        let pid = pid_from_name(&button.name);
        Vanbakel::remove_task(g, pid);
    }

    pub fn commit_click(g: &mut Ctx, button: &Button) {
        let pid = pid_from_name(&button.name);

        let thistask = g
            .svb
            .get_task_mut::<LogModifier>(pid)
            .expect("LogModifier::commit_click: no LogModifier task");

        // Stop the existing text buttons from being modified
        g.ecl.remove_button(&btn("nexttype", pid));
        g.ecl.register_button_callbacks(&btn("fromip", pid), Some(textbutton_draw), None, None, None);
        g.ecl.register_button_callbacks(&btn("data1", pid), Some(textbutton_draw), None, None, None);

        // Set the status up for modifying the log
        thistask.status = LOGMODIFIER_STATUS_MODIFYING;
        thistask.numticksrequired = TICKSREQUIRED_LOGMODIFIER;
        thistask.progress = 0;
    }

    pub fn next_type_click(g: &mut Ctx, button: &Button) {
        let pid = pid_from_name(&button.name);

        {
            let thistask = g
                .svb
                .get_task_mut::<LogModifier>(pid)
                .expect("LogModifier::next_type_click: no LogModifier task");
            thistask.logtype += 1;
            if thistask.logtype >= NUM_LOG_TYPES {
                thistask.logtype = -1;
            }
        }

        Self::change_log_type(g, pid);
    }

    /// Updates captions for the given task.
    pub fn change_log_type(g: &mut Ctx, pid: i32) {
        let logtype = g
            .svb
            .get_task::<LogModifier>(pid)
            .expect("LogModifier::change_log_type: no LogModifier task")
            .logtype;
        change_log_type_to(g, pid, logtype);
    }

    pub fn create_expanded_interface(&mut self, g: &mut Ctx) {
        if self.is_expanded_interface_visible(g) {
            return;
        }

        if !source_has_log(g, &self.source, self.sourceindex) {
            self.status = LOGMODIFIER_STATUS_WAITING;
            return;
        }

        let pid = self.base.pid;

        // Find the top left
        let (tx, ty) = {
            let btitle = g
                .ecl
                .get_button(&btn("title", pid))
                .expect("LogModifier: title button");
            (btitle.x, btitle.y)
        };

        // Create the text and buttons
        let background = btn("background", pid);
        let t_ltype = btn("tltype", pid);
        let ltype = btn("ltype", pid);
        let t_fromip = btn("tfromip", pid);
        let fromip = btn("fromip", pid);
        let t_data1 = btn("tdata1", pid);
        let data1 = btn("data1", pid);
        let commit = btn("commit", pid);
        let close = btn("close2", pid);
        let nexttype = btn("nexttype", pid);

        g.ecl
            .register_button(tx + 155, ty, 180, 110, "", "", &background);
        g.ecl
            .register_button_callbacks(&background, Some(Self::background_draw), None, None, None);

        g.ecl
            .register_button(tx + 165, ty + 10, 50, 15, "Type", "", &t_ltype);
        g.ecl
            .register_button(tx + 215, ty + 10, 100, 15, "", "Enter new type here", &ltype);
        g.ecl
            .register_button(tx + 165, ty + 35, 50, 15, "From", "", &t_fromip);
        g.ecl
            .register_button(tx + 215, ty + 35, 100, 15, "", "Enter new data here", &fromip);
        g.ecl
            .register_button(tx + 165, ty + 60, 50, 15, "", "", &t_data1);
        g.ecl
            .register_button(tx + 215, ty + 60, 100, 15, "", "Enter new data here", &data1);

        g.ecl
            .register_button_callbacks(&t_ltype, Some(textbutton_draw), None, None, None);
        g.ecl
            .register_button_callbacks(&ltype, Some(textbutton_draw), None, None, None);
        g.ecl
            .register_button_callbacks(&t_fromip, Some(textbutton_draw), None, None, None);
        g.ecl.register_button_callbacks(
            &fromip,
            Some(textbutton_draw),
            None,
            Some(button_click),
            Some(button_highlight),
        );
        g.ecl
            .register_button_callbacks(&t_data1, Some(textbutton_draw), None, None, None);
        g.ecl.register_button_callbacks(
            &data1,
            Some(textbutton_draw),
            None,
            Some(button_click),
            Some(button_highlight),
        );

        g.ecl.register_button(
            tx + 215 + 100,
            ty + 10,
            15,
            15,
            ">",
            "Change the type of this log",
            &nexttype,
        );
        g.ecl.register_button_callback(&nexttype, Self::next_type_click);

        g.ecl.register_button(
            tx + 165,
            ty + 85,
            50,
            15,
            "Commit",
            "Click here to commit your changes",
            &commit,
        );
        g.ecl.register_button_callback(&commit, Self::commit_click);

        g.ecl
            .register_button(tx + 220, ty + 85, 50, 15, "Close", "Close the log modifier", &close);
        g.ecl.register_button_callback(&close, Self::close_click);

        g.ecl.make_button_editable(&ltype);
        g.ecl.make_button_editable(&fromip);
        g.ecl.make_button_editable(&data1);

        // Register those caption changes
        // PORT: the C++ `log->Print()` debug dump is dropped (PORTING.md §6: Print() -> #[derive(Debug)]).
        let (log_fromip, log_data1, log_type) = {
            let log = source_bank(g, &self.source)
                .and_then(|bank| bank.logs.get_data(self.sourceindex))
                .expect("LogModifier::create_expanded_interface: no log");
            (log.fromip.clone(), log.data1.clone(), log.type_)
        };

        g.ecl.register_caption_change_auto(&fromip, &log_fromip, None);
        if let Some(log_data1) = &log_data1 {
            g.ecl.register_caption_change_auto(&data1, log_data1, None);
        }

        self.logtype = log_type;
        change_log_type_to(g, pid, self.logtype);

        self.status = LOGMODIFIER_STATUS_ACTIVE;
    }

    pub fn remove_expanded_interface(&mut self, g: &mut Ctx) {
        if self.is_expanded_interface_visible(g) {
            for name in expanded_names(self.base.pid) {
                g.ecl.remove_button(&name);
            }
        }
    }

    pub fn is_expanded_interface_visible(&self, g: &Ctx) -> bool {
        g.ecl
            .get_button(&btn("background", self.base.pid))
            .is_some()
    }
}

impl UplinkTask for LogModifier {
    fn base(&self) -> &UplinkTaskBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut UplinkTaskBase {
        &mut self.base
    }

    fn set_target(&mut self, g: &mut Ctx, target: ObjRef, targetstring: &str, targetint: i32) {
        // uo: LogBank object containing AccessLog to be un-deleted
        // targetstring (uos): name of button representing data
        // targetint (uoi): index of log in logbank
        if !target.is_log_bank() {
            return;
        }

        if self.status != LOGMODIFIER_STATUS_MODIFYING {
            // Copy those variables!
            self.source = target;
            self.sourceindex = targetint;

            if source_has_log(g, &self.source, self.sourceindex) {
                // Create the expanded interface
                self.create_expanded_interface(g);
            } else {
                // Targetting a blank spot - create a new log if >= v2.0
                if self.base.version >= 2.0 {
                    self.status = LOGMODIFIER_STATUS_CREATING;
                    self.numticksrequired = TICKSREQUIRED_LOGMODIFIER;
                    self.progress = 0;
                }
            }

            // Move the whole thing
            let (bx, by) = {
                let b = g
                    .ecl
                    .get_button(targetstring)
                    .expect("LogModifier::set_target: target button");
                (b.x, b.y)
            };
            self.move_to(g, bx, by, 1000);
        }
    }

    fn move_to(&mut self, g: &mut Ctx, x: i32, y: i32, time_ms: i32) {
        // Origin top left
        if !self.is_interface_visible(g) {
            return;
        }

        let pid = self.base.pid;

        g.ecl
            .register_movement(&btn("title", pid), x, y, time_ms, MOVE_STRAIGHTLINE, None);
        g.ecl
            .register_movement(&btn("border", pid), x + 20, y, time_ms, MOVE_STRAIGHTLINE, None);
        g.ecl
            .register_movement(&btn("progress", pid), x + 20, y, time_ms, MOVE_STRAIGHTLINE, None);
        g.ecl
            .register_movement(&btn("close", pid), x + 140, y, time_ms, MOVE_STRAIGHTLINE, None);

        if self.is_expanded_interface_visible(g) {
            let moves = [
                ("background", x + 155, y),
                ("tltype", x + 165, y + 10),
                ("ltype", x + 215, y + 10),
                ("tfromip", x + 165, y + 35),
                ("fromip", x + 215, y + 35),
                ("tdata1", x + 165, y + 60),
                ("data1", x + 215, y + 60),
                ("nexttype", x + 315, y + 10),
                ("commit", x + 165, y + 85),
                ("close2", x + 220, y + 85),
            ];
            for (name, tx, ty) in moves {
                g.ecl
                    .register_movement(&btn(name, pid), tx, ty, time_ms, MOVE_STRAIGHTLINE, None);
            }
        }
    }

    fn initialise(&mut self, _g: &mut Ctx) {}

    fn tick(&mut self, g: &mut Ctx, n: i32) {
        if self.status == LOGMODIFIER_STATUS_MODIFYING {
            let pid = self.base.pid;

            for _ in 0..n {
                // Check log is still there
                if !source_has_log(g, &self.source, self.sourceindex) {
                    self.status = LOGMODIFIER_STATUS_FINISHED;
                    break;
                }

                // Check we are still connected
                if !g.player().is_connected() {
                    Vanbakel::remove_task(g, pid);
                    return;
                }

                let sprogress = btn("progress", pid);

                // Modify in progress
                self.progress += 1;
                set_progress_bar(g, &sprogress, self.progress, self.numticksrequired);

                if self.progress >= self.numticksrequired {
                    // All done - modify the log
                    let fromip_caption = g
                        .ecl
                        .get_button(&btn("fromip", pid))
                        .expect("LogModifier: fromip button")
                        .caption
                        .clone();
                    let data1_caption = g
                        .ecl
                        .get_button(&btn("data1", pid))
                        .expect("LogModifier: data1 button")
                        .caption
                        .clone();
                    let new_fromip = strip_carriage_returns(&fromip_caption);
                    let new_data1 = strip_carriage_returns(&data1_caption);
                    let logtype = self.logtype;
                    let sourceindex = self.sourceindex;

                    let bank = source_bank_mut(g, &self.source)
                        .expect("LogModifier::tick: source LogBank");
                    let log = bank
                        .logs
                        .get_data_mut(sourceindex)
                        .expect("LogModifier::tick: log");
                    log.set_type(logtype);
                    log.set_from_ip(&new_fromip);
                    log.set_data1(Some(&new_data1));

                    // This should be undetectable - so copy the result into internal logs
                    let copy = log.clone();
                    if bank.internallogs.valid_index(sourceindex) {
                        bank.internallogs
                            .get_data_mut(sourceindex)
                            .expect("LogModifier::tick: internal log")
                            .set_properties_from(&copy);
                    } else {
                        let mut internalcopy = AccessLog::new();
                        internalcopy.set_properties_from(&copy);
                        bank.internallogs.put_data_at(internalcopy, sourceindex);
                    }

                    self.status = LOGMODIFIER_STATUS_FINISHED;
                }
            }
        } else if self.status == LOGMODIFIER_STATUS_CREATING {
            for _ in 0..n {
                // Check target area is still empty
                if source_has_log(g, &self.source, self.sourceindex) {
                    self.status = LOGMODIFIER_STATUS_FINISHED;
                    break;
                }

                let pid = self.base.pid;
                let sprogress = btn("progress", pid);

                // Modify in progress
                self.progress += 1;
                set_progress_bar(g, &sprogress, self.progress, self.numticksrequired);

                if self.progress >= self.numticksrequired {
                    // All done - create the new log
                    let date = g.world().date.clone();
                    let mut newlog = AccessLog::new();
                    newlog.set_properties(&date, "New IP", "PLAYER", LOG_NOTSUSPICIOUS, LOG_TYPE_TEXT);
                    newlog.set_data1(Some("Blank Action"));

                    let sourceindex = self.sourceindex;
                    let bank = source_bank_mut(g, &self.source)
                        .expect("LogModifier::tick: source LogBank");
                    bank.add_log(newlog, sourceindex);

                    self.status = LOGMODIFIER_STATUS_FINISHED;
                }
            }
        } else if self.status == LOGMODIFIER_STATUS_FINISHED {
            self.remove_expanded_interface(g);

            let pid = self.base.pid;

            // PORT: the C++ code after this point (reset to WAITING) was commented out, so the
            // task removes itself.
            Vanbakel::remove_task(g, pid);
        }
    }

    fn create_interface(&mut self, g: &mut Ctx) {
        if self.is_interface_visible(g) {
            return;
        }

        let pid = self.base.pid;
        let tooltip = format!("Log Modifier v{:.1}", self.base.version);

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

        g.ecl
            .register_button(285, 450, 120, 13, "Select target log", "", &sprogress);
        g.ecl.register_button_callbacks(
            &sprogress,
            Some(Self::progress_draw),
            Some(Self::border_click),
            Some(button_click),
            Some(button_highlight),
        );

        g.ecl
            .register_button(405, 450, 13, 13, "", "Close the LogModifier", &sclose);
        opengl_interface::button_assignbitmaps(g, &sclose, "close.tif", "close_h.tif", "close_c.tif");
        g.ecl.register_button_callback(&sclose, Self::close_click);

        self.status = LOGMODIFIER_STATUS_WAITING;
    }

    fn remove_interface(&mut self, g: &mut Ctx) {
        if self.is_interface_visible(g) {
            let pid = self.base.pid;

            g.ecl.remove_button(&btn("title", pid));
            g.ecl.remove_button(&btn("border", pid));
            g.ecl.remove_button(&btn("progress", pid));
            g.ecl.remove_button(&btn("close", pid));

            self.remove_expanded_interface(g);
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

        if self.is_expanded_interface_visible(g) {
            for name in expanded_names(pid) {
                g.ecl.button_bring_to_front(&name);
            }
        }
    }

    fn is_interface_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button(&btn("title", self.base.pid)).is_some()
    }

    impl_as_any!();
}
