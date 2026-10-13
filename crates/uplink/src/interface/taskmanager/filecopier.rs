//! Port of source/interface/taskmanager/filecopier.h + source/interface/taskmanager/filecopier.cpp

use uplink_engine::{gl, text};

use crate::app::globals::ALPHA;
use crate::app::opengl_interface;
use crate::ctx::Ctx;
use crate::eclipse::{Button, MOVE_STRAIGHTLINE};
use crate::game::data::data::TICKSREQUIRED_COPY;
use crate::interface::taskmanager::uplinktask::{UplinkTask, UplinkTaskBase};
use crate::objref::{DataBankOwner, ObjRef};
use crate::vanbakel::Vanbakel;
use crate::world::computer::databank::DataBank;
use crate::world::computer::gateway::Gateway;

pub const FILECOPIER_NOTDOWNLOADING: i32 = 0;
pub const FILECOPIER_INPROGRESS: i32 = 1;
pub const FILECOPIER_WAITINGFORTARGET: i32 = 2;
pub const FILECOPIER_FINISHED: i32 = 3;

#[derive(Debug)]
pub struct FileCopier {
    pub base: UplinkTaskBase,
    /// C++ `DataBank *source`: databank the file is copied from.
    pub source: Option<DataBankOwner>,
    pub sourceindex: i32,
    /// 0 = not, 1 = in progress, 2 = waiting for target, 3 = finished
    pub downloading: i32,
    /// True if the target is coming in remotely
    pub remotefile: bool,
    pub numticksrequired: i32,
    pub progress: i32,
}

/// Names of the four buttons of the copier window (`filecopier_title/border/progress/close <pid>`).
fn window_names(pid: i32) -> [String; 4] {
    [
        format!("filecopier_title {pid}"),
        format!("filecopier_border {pid}"),
        format!("filecopier_progress {pid}"),
        format!("filecopier_close {pid}"),
    ]
}

/// C++ `sscanf(button->name, "%s %d", bname, &pid)`: the PID after the button's first word.
fn pid_from_button_name(name: &str) -> Option<i32> {
    name.split_whitespace().nth(1)?.parse().ok()
}

/// C++ `source` (a `DataBank *`): the databank the owner refers to.
fn databank_ref<'a>(g: &'a Ctx, owner: &DataBankOwner) -> Option<&'a DataBank> {
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

impl FileCopier {
    pub fn new() -> Self {
        FileCopier {
            base: UplinkTaskBase {
                version: 1.0,
                targetint: -1,
                ..Default::default()
            },
            source: None,
            sourceindex: -1,
            downloading: FILECOPIER_NOTDOWNLOADING,
            remotefile: false,
            numticksrequired: 0,
            progress: 0,
        }
    }

    /// C++ `FileCopier::BorderDraw` (static button callback).
    pub fn border_draw(_g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        let (a, b) = if clicked {
            ([0.5, 0.5, 0.6], [0.7, 0.7, 0.6])
        } else if highlighted {
            ([0.2, 0.2, 0.5], [0.5, 0.5, 0.6])
        } else {
            ([0.2, 0.2, 0.4], [0.3, 0.3, 0.5])
        };

        gl::begin(gl::QUADS);

        gl::color4f(a[0], a[1], a[2], ALPHA);
        gl::vertex2i(button.x, button.y);

        gl::color4f(b[0], b[1], b[2], ALPHA);
        gl::vertex2i(button.x + button.width, button.y);

        gl::color4f(a[0], a[1], a[2], ALPHA);
        gl::vertex2i(button.x + button.width, button.y + button.height);

        gl::color4f(b[0], b[1], b[2], ALPHA);
        gl::vertex2i(button.x, button.y + button.height);

        gl::end();

        if highlighted || clicked {
            gl::color4f(0.3, 0.3, 0.7, 1.0);
            opengl_interface::border_draw(button);
        }
    }

    /// C++ `FileCopier::ProgressDraw` (static button callback).
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

    /// C++ `FileCopier::CloseClick` (static button callback).
    pub fn close_click(g: &mut Ctx, button: &Button) {
        let Some(pid) = pid_from_button_name(&button.name) else { return };
        Vanbakel::remove_task(g, pid);
    }

    /// C++ `FileCopier::BorderClick` (static button callback).
    pub fn border_click(g: &mut Ctx, button: &Button) {
        let Some(pid) = pid_from_button_name(&button.name) else { return };
        g.interface_mut().tm.set_target_program(pid);
    }

    /// C++ `FileCopier::GetState`: returns `downloading`.
    pub fn get_state(&self) -> i32 {
        self.downloading
    }
}

impl Default for FileCopier {
    fn default() -> Self {
        Self::new()
    }
}

impl UplinkTask for FileCopier {
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
        let sprogress = format!("filecopier_progress {pid}");

        if self.downloading == FILECOPIER_NOTDOWNLOADING {
            // Not downloading - look for a new target
            if self.source.is_some() {
                // A new source file has been specified - start downloading it
                self.downloading = FILECOPIER_INPROGRESS;
                g.ecl.register_caption_change(&sprogress, "Downloading...", 0, None);
            }
        } else if self.downloading == FILECOPIER_INPROGRESS {
            // Check we are still connected
            if self.remotefile && !g.player().is_connected() {
                Vanbakel::remove_task(g, pid);
                return;
            }

            // Download in progress
            let has_data = self
                .source
                .as_ref()
                .and_then(|s| databank_ref(g, s))
                .and_then(|db| db.get_data(self.sourceindex))
                .is_some();
            if !has_data {
                Vanbakel::remove_task(g, pid);
                return;
            }

            for _ in 0..n {
                self.progress += 1;
                if let Some(button) = g.ecl.get_button_mut(&sprogress) {
                    button.width = (120.0 * (self.progress as f32 / self.numticksrequired as f32)) as i32;
                }
                g.ecl.dirty_button(&sprogress);

                if self.progress >= self.numticksrequired {
                    // Finished downloading now
                    self.downloading = FILECOPIER_WAITINGFORTARGET;
                    g.ecl.register_caption_change_auto(&sprogress, "Select memory slot", None);
                    break;
                }
            }
        } else if self.downloading == FILECOPIER_WAITINGFORTARGET {
            // Finished downloading - waiting for a target memory slot
            // Check we are still connected if this was a remote file
            if self.remotefile && !g.player().is_connected() {
                Vanbakel::remove_task(g, pid);
                return;
            }
        } else if self.downloading == FILECOPIER_FINISHED {
            self.downloading = FILECOPIER_NOTDOWNLOADING;
            Vanbakel::remove_task(g, pid);
        }
    }

    fn create_interface(&mut self, g: &mut Ctx) {
        if self.is_interface_visible(g) {
            return;
        }

        let [stitle, sborder, sprogress, sclose] = window_names(self.base.pid);

        g.ecl.register_button(265, 432, 20, 15, "", "File Copier", &stitle);
        opengl_interface::button_assignbitmap(g, &stitle, "software/cpy.tif");

        g.ecl.register_button(285, 432, 120, 15, "", "", &sborder);
        g.ecl.register_button_callbacks(
            &sborder,
            Some(Self::border_draw),
            Some(Self::border_click),
            Some(opengl_interface::button_click),
            Some(opengl_interface::button_highlight),
        );

        g.ecl.register_button(285, 433, 120, 13, "Select source", "Shows the fraction of the download already completed", &sprogress);
        g.ecl.register_button_callbacks(
            &sprogress,
            Some(Self::progress_draw),
            Some(Self::border_click),
            Some(opengl_interface::button_click),
            Some(opengl_interface::button_highlight),
        );

        g.ecl.register_button(405, 433, 13, 13, "", "Close the FileCopier (and stop downloading)", &sclose);
        opengl_interface::button_assignbitmaps(g, &sclose, "close.tif", "close_h.tif", "close_c.tif");
        g.ecl.register_button_callback(&sclose, Self::close_click);
    }

    fn remove_interface(&mut self, g: &mut Ctx) {
        if !self.is_interface_visible(g) {
            return;
        }

        let [stitle, sborder, sprogress, sclose] = window_names(self.base.pid);

        g.ecl.remove_button(&stitle);
        g.ecl.remove_button(&sborder);
        g.ecl.remove_button(&sprogress);
        g.ecl.remove_button(&sclose);
    }

    fn show_interface(&mut self, g: &mut Ctx) {
        if !self.is_interface_visible(g) {
            self.create_interface(g);
        }

        let [stitle, sborder, sprogress, sclose] = window_names(self.base.pid);

        g.ecl.button_bring_to_front(&stitle);
        g.ecl.button_bring_to_front(&sborder);
        g.ecl.button_bring_to_front(&sprogress);
        g.ecl.button_bring_to_front(&sclose);
    }

    fn is_interface_visible(&self, g: &Ctx) -> bool {
        let [_, sborder, _, _] = window_names(self.base.pid);
        g.ecl.get_button(&sborder).is_some()
    }

    fn set_target(&mut self, g: &mut Ctx, target: ObjRef, targetstring: &str, targetint: i32) {
        if self.downloading == FILECOPIER_NOTDOWNLOADING {
            if let ObjRef::DataBank(owner) = target {
                // DataBank selected
                // uo  : DataBank object containing Data to be downloaded
                // uos : Name of button representing downloadable data
                // uoi : memory index of data in databank
                let Some(size) = databank_ref(g, &owner)
                    .and_then(|db| db.get_data(targetint))
                    .map(|data| data.size)
                else {
                    return;
                };

                self.source = Some(owner);
                self.sourceindex = targetint;

                let bandwidth = Gateway::get_bandwidth(g);
                self.numticksrequired = (TICKSREQUIRED_COPY as f32 * (size as f32 / bandwidth as f32)) as i32;
                self.progress = 0;

                self.remotefile = targetstring.contains("fileserverscreen");

                let (bx, by) = g
                    .ecl
                    .get_button(targetstring)
                    .map(|button| (button.x, button.y))
                    .expect("FileCopier::set_target: source button missing");
                self.move_to(g, bx, by, 1000);
            }
        } else if self.downloading == FILECOPIER_WAITINGFORTARGET {
            let Some(owner) = self.source.clone() else { return };

            // Copy the data
            let Some(datacopy) = databank_ref(g, &owner)
                .and_then(|db| db.get_data(self.sourceindex))
                .cloned()
            else {
                Vanbakel::remove_task(g, self.base.pid);
                return;
            };

            match target {
                ObjRef::DataBank(target_owner) => {
                    // Copying the file into a memory bank
                    // Target memory area selected
                    // If memory index is -1, look for a valid placement
                    let Some(db) = databank_mut(g, &target_owner) else { return };
                    let mut memoryindex = targetint;

                    if memoryindex == -1 {
                        memoryindex = db.find_valid_placement(&datacopy);
                    }

                    if db.is_valid_placement(&datacopy, memoryindex) == 0 {
                        db.put_data_at(datacopy, memoryindex);
                        self.downloading = FILECOPIER_FINISHED;
                    }
                }
                // PORT: the C++ also has an `OID_MESSAGE` branch (`Message::AttachData(datacopy)`, then
                // FINISHED). No `SetProgramTarget` call site ever passes a Message, so it is dead code
                // in the C++ too, and `ObjRef` has no Message variant.
                _ => {}
            }
        }
    }

    fn move_to(&mut self, g: &mut Ctx, x: i32, y: i32, time_ms: i32) {
        // The C++ base UplinkTask::MoveTo does nothing, so only the copier's own buttons move.
        let [stitle, sborder, sprogress, sclose] = window_names(self.base.pid);

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
