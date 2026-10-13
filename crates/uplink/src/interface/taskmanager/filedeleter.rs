//! Port of source/interface/taskmanager/filedeleter.h + source/interface/taskmanager/filedeleter.cpp

use uplink_engine::{gl, text};

use crate::app::globals::ALPHA;
use crate::app::opengl_interface;
use crate::ctx::Ctx;
use crate::eclipse::{Button, MOVE_STRAIGHTLINE};
use crate::game::data::data::TICKSREQUIRED_DELETE;
use crate::interface::taskmanager::uplinktask::{UplinkTask, UplinkTaskBase};
use crate::objref::{DataBankOwner, ObjRef};
use crate::vanbakel::Vanbakel;
use crate::world::computer::databank::DataBank;

pub const FILEDELETER_OFF: i32 = 0;
pub const FILEDELETER_INPROGRESS: i32 = 1;
pub const FILEDELETER_FINISHED: i32 = 2;

#[derive(Debug)]
pub struct FileDeleter {
    pub base: UplinkTaskBase,
    /// C++ `DataBank *source`: databank the file is deleted from.
    pub source: Option<DataBankOwner>,
    pub sourceindex: i32,
    /// 0 = off, 1 = in progress, 2 = finished
    pub status: i32,
    pub remotefile: bool,
    pub numticksrequired: i32,
    pub progress: i32,
}

/// C++ `sscanf ( button->name, "%s %d", bname, &pid )`: the PID is the second word of the name.
fn parse_pid(name: &str) -> i32 {
    name.split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(-1)
}

/// The button names of this task's window: title, border, progress, close.
fn window_names(pid: i32) -> (String, String, String, String) {
    (
        format!("filedeleter_title {pid}"),
        format!("filedeleter_border {pid}"),
        format!("filedeleter_progress {pid}"),
        format!("filedeleter_close {pid}"),
    )
}

/// The `DataBank` a `DataBankOwner` names: the player's gateway memory or a computer's databank.
fn databank<'a>(g: &'a Ctx, owner: &DataBankOwner) -> Option<&'a DataBank> {
    match owner {
        DataBankOwner::Gateway => Some(&g.player().gateway.databank),
        DataBankOwner::Computer(name) => g.world().get_computer(name).map(|c| &c.databank),
    }
}

fn databank_mut<'a>(g: &'a mut Ctx, owner: &DataBankOwner) -> Option<&'a mut DataBank> {
    match owner {
        DataBankOwner::Gateway => Some(&mut g.player_mut().gateway.databank),
        DataBankOwner::Computer(name) => g.world_mut().get_computer_mut(name).map(|c| &mut c.databank),
    }
}

/// C++ `EclGetButton ( sprogress )->width = w; EclDirtyButton ( sprogress );` (with UplinkAssert)
fn set_button_width(g: &mut Ctx, name: &str, width: i32) {
    g.ecl.get_button_mut(name).expect("UplinkAssert (sprogress)").width = width;
    g.ecl.dirty_button(name);
}

impl FileDeleter {
    pub fn new() -> Self {
        FileDeleter {
            base: UplinkTaskBase {
                version: 1.0,
                targetint: -1,
                ..Default::default()
            },
            source: None,
            sourceindex: -1,
            status: FILEDELETER_OFF,
            remotefile: true,
            numticksrequired: 0,
            progress: 0,
        }
    }

    /// C++ `FileDeleter::BorderDraw` (static button callback).
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

    /// C++ `FileDeleter::ProgressDraw` (static button callback).
    pub fn progress_draw(_g: &mut Ctx, button: &Button, highlighted: bool, _clicked: bool) {
        // C++: (float) button->width / 100.0 is computed in double, then stored as float
        let mut scale = (button.width as f64 / 100.0) as f32;
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

    /// C++ `FileDeleter::CloseClick` (static button callback).
    pub fn close_click(g: &mut Ctx, button: &Button) {
        let pid = parse_pid(&button.name);
        Vanbakel::remove_task(g, pid);
    }

    /// C++ `FileDeleter::BorderClick` (static button callback).
    pub fn border_click(g: &mut Ctx, button: &Button) {
        let pid = parse_pid(&button.name);
        g.interface_mut().tm.set_target_program(pid);
    }
}

impl Default for FileDeleter {
    fn default() -> Self {
        Self::new()
    }
}

impl UplinkTask for FileDeleter {
    fn base(&self) -> &UplinkTaskBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut UplinkTaskBase {
        &mut self.base
    }
    fn initialise(&mut self, _g: &mut Ctx) {}
    fn tick(&mut self, g: &mut Ctx, n: i32) {
        if !self.is_interface_visible(g) {
            return;
        }
        let pid = self.base.pid;
        let sprogress = format!("filedeleter_progress {pid}");

        if self.status == FILEDELETER_OFF {
            // Not deleting - look for a new target
            if self.source.is_some() {
                // A new source file has been specified - start deleting it
                self.status = FILEDELETER_INPROGRESS;

                g.ecl.register_caption_change(&sprogress, "Deleting...", 0, None);
            }
        } else if self.status == FILEDELETER_INPROGRESS {
            let owner = self.source.clone().expect("UplinkAssert (source)");

            // Check we are still connected
            if self.remotefile && !g.player().is_connected() {
                Vanbakel::remove_task(g, pid);
                return;
            }

            let sourceindex = self.sourceindex;
            if databank(g, &owner).and_then(|db| db.get_data(sourceindex)).is_none() {
                Vanbakel::remove_task(g, pid);
                return;
            }

            // Delete in progress
            for _count in 0..n {
                self.progress += 1;

                let width = (120.0 * (self.progress as f32 / self.numticksrequired as f32)) as i32;
                set_button_width(g, &sprogress, width);

                if self.progress >= self.numticksrequired {
                    // Finished deleting now
                    self.status = FILEDELETER_FINISHED;
                    g.ecl.register_caption_change_auto(&sprogress, "Finished", None);

                    // Delete the file
                    if let Some(db) = databank_mut(g, &owner) {
                        db.remove_data(sourceindex);
                    }
                    break;
                }
            }
        } else if self.status == FILEDELETER_FINISHED {
            self.status = FILEDELETER_OFF;
            Vanbakel::remove_task(g, pid);
        }
    }

    fn create_interface(&mut self, g: &mut Ctx) {
        if !self.is_interface_visible(g) {
            let pid = self.base.pid;
            let (stitle, sborder, sprogress, sclose) = window_names(pid);

            g.ecl.register_button(265, 450, 20, 15, "", "File Deleter", &stitle);
            opengl_interface::button_assignbitmap(g, &stitle, "software/del.tif");

            g.ecl.register_button(285, 450, 120, 15, "", "", &sborder);
            g.ecl.register_button_callbacks(
                &sborder,
                Some(Self::border_draw),
                Some(Self::border_click),
                Some(opengl_interface::button_click),
                Some(opengl_interface::button_highlight),
            );

            g.ecl.register_button(285, 450, 120, 13, "Select target", "Shows the progress of the delete", &sprogress);
            g.ecl.register_button_callbacks(
                &sprogress,
                Some(Self::progress_draw),
                Some(Self::border_click),
                Some(opengl_interface::button_click),
                Some(opengl_interface::button_highlight),
            );

            g.ecl.register_button(405, 450, 13, 13, "", "Close the FileDeleter (and stop deleting)", &sclose);
            opengl_interface::button_assignbitmaps(g, &sclose, "close.tif", "close_h.tif", "close_c.tif");
            g.ecl.register_button_callback(&sclose, Self::close_click);
        }
    }

    fn remove_interface(&mut self, g: &mut Ctx) {
        if self.is_interface_visible(g) {
            let pid = self.base.pid;
            let (stitle, sborder, sprogress, sclose) = window_names(pid);

            g.ecl.remove_button(&stitle);
            g.ecl.remove_button(&sborder);
            g.ecl.remove_button(&sprogress);
            g.ecl.remove_button(&sclose);
        }
    }

    fn show_interface(&mut self, g: &mut Ctx) {
        if !self.is_interface_visible(g) {
            self.create_interface(g);
        }

        let pid = self.base.pid;
        let (stitle, sborder, sprogress, sclose) = window_names(pid);

        g.ecl.button_bring_to_front(&stitle);
        g.ecl.button_bring_to_front(&sborder);
        g.ecl.button_bring_to_front(&sprogress);
        g.ecl.button_bring_to_front(&sclose);
    }

    fn is_interface_visible(&self, g: &Ctx) -> bool {
        // PORT: the C++ tests the border button here, not the title
        let sborder = format!("filedeleter_border {}", self.base.pid);
        g.ecl.get_button(&sborder).is_some()
    }

    fn set_target(&mut self, g: &mut Ctx, target: ObjRef, targetstring: &str, targetint: i32) {
        // target : DataBank containing data to be deleted
        // targetstring : name of button representing data
        // targetint : index of data

        if self.status != FILEDELETER_OFF {
            return;
        }

        if let ObjRef::DataBank(owner) = target {
            // Databank selected
            if databank(g, &owner).and_then(|db| db.get_data(targetint)).is_some() {
                self.source = Some(owner.clone());
                self.sourceindex = targetint;

                let size = databank(g, &owner)
                    .and_then(|db| db.get_data(targetint))
                    .expect("UplinkAssert (data)")
                    .size;
                self.numticksrequired = TICKSREQUIRED_DELETE * size;
                self.progress = 0;

                self.remotefile = targetstring.contains("fileserverscreen");

                let (bx, by, bheight) = {
                    let button = g.ecl.get_button(targetstring).expect("UplinkAssert (button)");
                    (button.x, button.y, button.height)
                };
                self.move_to(g, bx, by + bheight + 1, 1000);
            }
        }
    }

    fn move_to(&mut self, g: &mut Ctx, x: i32, y: i32, time_ms: i32) {
        // C++ UplinkTask::MoveTo (base) does nothing
        let pid = self.base.pid;
        let (stitle, sborder, sprogress, sclose) = window_names(pid);

        g.ecl.register_movement(&stitle, x, y, time_ms, MOVE_STRAIGHTLINE, None);
        g.ecl.register_movement(&sborder, x + 20, y, time_ms, MOVE_STRAIGHTLINE, None);
        g.ecl.register_movement(&sprogress, x + 20, y + 1, time_ms, MOVE_STRAIGHTLINE, None);
        g.ecl.register_movement(&sclose, x + 140, y + 1, time_ms, MOVE_STRAIGHTLINE, None);

        g.ecl.button_bring_to_front(&stitle);
        g.ecl.button_bring_to_front(&sborder);
        g.ecl.button_bring_to_front(&sprogress);
        g.ecl.button_bring_to_front(&sclose);
    }

    crate::impl_as_any!();
}
