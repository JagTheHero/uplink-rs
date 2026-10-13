//! Port of source/interface/taskmanager/logundeleter.h + source/interface/taskmanager/logundeleter.cpp

use uplink_engine::{gl, text};

use crate::app::globals::ALPHA;
use crate::app::opengl_interface::{button_assignbitmap, button_assignbitmaps, button_click, button_highlight};
use crate::ctx::Ctx;
use crate::eclipse::{Button, MOVE_STRAIGHTLINE};
use crate::game::data::data::TICKSREQUIRED_LOGUNDELETER;
use crate::impl_as_any;
use crate::interface::remoteinterface::remoteinterface::RemoteInterface;
use crate::interface::taskmanager::uplinktask::{UplinkTask, UplinkTaskBase};
use crate::objref::ObjRef;
use crate::vanbakel::Vanbakel;
use crate::world::connection::Connection;

pub const LOGUNDELETER_OFF: i32 = 0;
pub const LOGUNDELETER_INPROGRESS: i32 = 1;
pub const LOGUNDELETER_FINISHED: i32 = 2;

#[derive(Debug, Default)]
pub struct LogUnDeleter {
    pub base: UplinkTaskBase,
    /// C++ `LogBank *source`: `ObjRef::LogBank(computer name)` or `ObjRef::AccountLog`, `ObjRef::None` for NULL.
    pub source: ObjRef,
    pub sourceindex: i32,
    pub status: i32,
    pub numticksrequired: i32,
    /// 0.0 - numticksrequired.
    pub progress: i32,
}

/// `sscanf(name, "%s %d", bname, &pid)`: the number after the first word of a button name.
fn pid_from_name(name: &str) -> i32 {
    name.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(-1)
}

impl LogUnDeleter {
    pub fn new() -> Self {
        Self {
            sourceindex: -1,
            ..Default::default()
        }
    }

    /// C++ `source != NULL`.
    fn has_source(&self) -> bool {
        self.source.is_log_bank()
    }

    /// The four button names of this task's window: title, border, progress, close.
    fn button_names(pid: i32) -> [String; 4] {
        [
            format!("logundeleter_title {pid}"),
            format!("logundeleter_border {pid}"),
            format!("logundeleter_progress {pid}"),
            format!("logundeleter_close {pid}"),
        ]
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
            crate::app::opengl_interface::border_draw(button);
        }
    }

    pub fn progress_draw(_g: &mut Ctx, button: &Button, highlighted: bool, _clicked: bool) {
        let mut scale = button.width as f32 / 100.0;
        if highlighted {
            scale *= 2.0;
        }

        gl::begin(gl::QUADS);

        gl::color4f(0.0, 1.5 - scale, scale, 0.6);
        gl::vertex3i(button.x, button.y, 0);

        gl::color4f(0.0, 1.5 - scale, scale, 0.6);
        gl::vertex3i(button.x + button.width, button.y, 0);

        gl::color4f(0.0, 1.5 - scale, scale, 0.6);
        gl::vertex3i(button.x + button.width, button.y + button.height, 0);

        gl::color4f(0.0, 1.5 - scale, scale, 0.6);
        gl::vertex3i(button.x, button.y + button.height, 0);

        gl::end();

        let xpos = button.x + 5;
        let ypos = (button.y + button.height / 2) + 3;

        gl::color4f(1.0, 1.0, 1.0, 1.0);
        text::draw_text_styled(xpos, ypos, &button.caption, text::HELVETICA_10);
    }

    pub fn close_click(g: &mut Ctx, button: &Button) {
        let pid = pid_from_name(&button.name);
        Vanbakel::remove_task(g, pid);
    }

    pub fn border_click(g: &mut Ctx, button: &Button) {
        let pid = pid_from_name(&button.name);
        g.interface_mut().tm.set_target_program(pid);
    }
}

impl UplinkTask for LogUnDeleter {
    fn base(&self) -> &UplinkTaskBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut UplinkTaskBase {
        &mut self.base
    }

    /// `SetTarget(uo, uos, uoi)`: uo = LogBank, uos = name of the button showing the log, uoi = log index.
    fn set_target(&mut self, g: &mut Ctx, target: ObjRef, targetstring: &str, targetint: i32) {
        if self.status != LOGUNDELETER_OFF {
            return;
        }

        // Logbank selected (C++ `uo->GetOBJECTID() == OID_LOGBANK`)
        if !target.is_log_bank() {
            return;
        }

        let valid = {
            let logbank = target.log_bank(g.world()).expect("LogUnDeleter: UplinkAssert(logbank)");
            logbank.logs.valid_index(targetint) && logbank.internallogs.valid_index(targetint)
        };
        if !valid {
            return;
        }

        self.source = target;
        self.sourceindex = targetint;

        self.numticksrequired = TICKSREQUIRED_LOGUNDELETER;
        self.progress = 0;

        let (x, y) = {
            let button = g.ecl.get_button(targetstring).expect("LogUnDeleter: UplinkAssert(button)");
            (button.x, button.y)
        };
        self.move_to(g, x, y, 1000);

        let comp_name = RemoteInterface::get_computer_screen(g)
            .expect("LogUnDeleter: UplinkAssert(computer screen)")
            .base()
            .computer
            .clone();
        let monitor = g.world().get_computer(&comp_name).expect("LogUnDeleter: UplinkAssert(comp)").security.is_running_monitor();
        if monitor {
            Connection::begin_trace(g, "PLAYER");
        }
    }

    /// Centred on top-right of title image.
    fn move_to(&mut self, g: &mut Ctx, x: i32, y: i32, time_ms: i32) {
        let [stitle, sborder, sprogress, sclose] = Self::button_names(self.base.pid);

        g.ecl.register_movement(&stitle, x, y, time_ms, MOVE_STRAIGHTLINE, None);
        g.ecl.register_movement(&sborder, x + 20, y, time_ms, MOVE_STRAIGHTLINE, None);
        g.ecl.register_movement(&sprogress, x + 20, y + 1, time_ms, MOVE_STRAIGHTLINE, None);
        g.ecl.register_movement(&sclose, x + 140, y + 1, time_ms, MOVE_STRAIGHTLINE, None);

        g.ecl.button_bring_to_front(&stitle);
        g.ecl.button_bring_to_front(&sborder);
        g.ecl.button_bring_to_front(&sprogress);
        g.ecl.button_bring_to_front(&sclose);
    }

    fn initialise(&mut self, _g: &mut Ctx) {}

    fn tick(&mut self, g: &mut Ctx, n: i32) {
        if !self.is_interface_visible(g) {
            return;
        }
        let pid = self.base.pid;
        let sprogress = format!("logundeleter_progress {pid}");

        for _tick in 0..n {
            if self.status == LOGUNDELETER_OFF {
                // Look for a new target
                if self.has_source() {
                    // A new source log has been specified - start undeleting it
                    self.status = LOGUNDELETER_INPROGRESS;

                    g.ecl.register_caption_change(&sprogress, "Recovering log...", 0, None);
                }
            } else if self.status == LOGUNDELETER_INPROGRESS {
                assert!(self.has_source(), "LogUnDeleter: UplinkAssert(source)");

                // Check log is still there
                let (logs_ok, internal_ok) = {
                    let logbank = self.source.log_bank(g.world()).expect("LogUnDeleter: UplinkAssert(logbank)");
                    (logbank.logs.valid_index(self.sourceindex), logbank.internallogs.valid_index(self.sourceindex))
                };
                if !logs_ok {
                    self.status = LOGUNDELETER_FINISHED;
                }
                if !internal_ok {
                    self.status = LOGUNDELETER_FINISHED;
                }

                // Check we are still connected
                if !g.player().is_connected() {
                    Vanbakel::remove_task(g, pid);
                    return;
                }

                // Delete in progress
                self.progress += 1;
                let width = (120.0 * (self.progress as f32 / self.numticksrequired as f32)) as i32;
                g.ecl.get_button_mut(&sprogress).expect("LogUnDeleter: UplinkAssert(progress button)").width = width;
                g.ecl.dirty_button(&sprogress);

                if self.progress >= self.numticksrequired {
                    // Un-delete the log
                    // PORT: the C++ LogBank stores AccessLog pointers, so logs[i] and internallogs[i] alias
                    // the same object after this; the Rust port keeps owned copies.
                    let idx = self.sourceindex;
                    {
                        let logbank = self.source.log_bank_mut(g.world_mut()).expect("LogUnDeleter: UplinkAssert(logbank)");
                        if let Some(log) = logbank.internallogs.get_data(idx).cloned() {
                            logbank.logs.put_data_at(log, idx);
                        }
                    }

                    // Finished
                    self.status = LOGUNDELETER_FINISHED;
                    g.ecl.register_caption_change_auto(&sprogress, "Finished", None);
                }
            } else if self.status == LOGUNDELETER_FINISHED {
                self.source = ObjRef::None;
                self.sourceindex = -1;
                self.numticksrequired = 0;
                self.progress = 0;
                g.ecl.register_caption_change_auto(&sprogress, "Finished", None);
                self.status = LOGUNDELETER_OFF;
            }
        }
    }

    fn create_interface(&mut self, g: &mut Ctx) {
        if !self.is_interface_visible(g) {
            let pid = self.base.pid;
            let [stitle, sborder, sprogress, sclose] = Self::button_names(pid);
            let tooltip = format!("Log Deleter v{:.1}", self.base.version);

            g.ecl.register_button(265, 450, 20, 15, "", &tooltip, &stitle);
            button_assignbitmap(g, &stitle, "software/log.tif");

            g.ecl.register_button(285, 450, 120, 15, "", "", &sborder);
            g.ecl.register_button_callbacks(
                &sborder,
                Some(Self::border_draw),
                Some(Self::border_click),
                Some(button_click),
                Some(button_highlight),
            );

            g.ecl.register_button(285, 450, 120, 13, "Select target log", "Shows the progress of the log undelete", &sprogress);
            g.ecl.register_button_callbacks(
                &sprogress,
                Some(Self::progress_draw),
                Some(Self::border_click),
                Some(button_click),
                Some(button_highlight),
            );

            g.ecl.register_button(405, 450, 13, 13, "", "Close the LogUnDeleter", &sclose);
            button_assignbitmaps(g, &sclose, "close.tif", "close_h.tif", "close_c.tif");
            g.ecl.register_button_callback(&sclose, Self::close_click);
        }
    }

    fn remove_interface(&mut self, g: &mut Ctx) {
        if self.is_interface_visible(g) {
            for name in Self::button_names(self.base.pid) {
                g.ecl.remove_button(&name);
            }
        }
    }

    fn show_interface(&mut self, g: &mut Ctx) {
        if !self.is_interface_visible(g) {
            self.create_interface(g);
        }
        for name in Self::button_names(self.base.pid) {
            g.ecl.button_bring_to_front(&name);
        }
    }

    fn is_interface_visible(&self, g: &Ctx) -> bool {
        // C++ checks the border button here (not the title).
        let [_, sborder, _, _] = Self::button_names(self.base.pid);
        g.ecl.get_button(&sborder).is_some()
    }

    impl_as_any!();
}
