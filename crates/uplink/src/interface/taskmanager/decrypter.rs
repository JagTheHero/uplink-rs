//! Port of source/interface/taskmanager/decrypter.h + source/interface/taskmanager/decrypter.cpp

use uplink_engine::{gl, text};

use crate::app::globals::ALPHA;
use crate::app::opengl_interface;
use crate::ctx::Ctx;
use crate::eclipse::{Button, MOVE_STRAIGHTLINE};
use crate::game::data::data::TICKSREQUIRED_DECRYPT;
use crate::interface::taskmanager::uplinktask::{UplinkTask, UplinkTaskBase};
use crate::objref::{DataBankOwner, ObjRef};
use crate::vanbakel::Vanbakel;
use crate::world::computer::databank::DataBank;

pub const DECRYPTER_OFF: i32 = 0;
pub const DECRYPTER_INPROGRESS: i32 = 1;
pub const DECRYPTER_FINISHED: i32 = 2;

#[derive(Debug)]
pub struct Decrypter {
    pub base: UplinkTaskBase,
    /// C++ `DataBank *source`: databank holding the file being decrypted.
    pub source: Option<DataBankOwner>,
    pub sourceindex: i32,
    pub status: i32,
    pub remotefile: bool,
    pub numticksrequired: i32,
    /// 0 .. numticksrequired
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
        format!("decrypter_title {pid}"),
        format!("decrypter_border {pid}"),
        format!("decrypter_progress {pid}"),
        format!("decrypter_close {pid}"),
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

impl Decrypter {
    pub fn new() -> Self {
        Decrypter {
            base: UplinkTaskBase {
                version: 1.0,
                targetint: -1,
                ..Default::default()
            },
            source: None,
            sourceindex: -1,
            status: DECRYPTER_OFF,
            remotefile: false,
            numticksrequired: 0,
            progress: 0,
        }
    }

    /// C++ `Decrypter::BorderDraw` (static button callback).
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

    /// C++ `Decrypter::ProgressDraw` (static button callback).
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

    /// C++ `Decrypter::CloseClick` (static button callback).
    pub fn close_click(g: &mut Ctx, button: &Button) {
        let pid = parse_pid(&button.name);
        Vanbakel::remove_task(g, pid);
    }

    /// C++ `Decrypter::BorderClick` (static button callback).
    pub fn border_click(g: &mut Ctx, button: &Button) {
        let pid = parse_pid(&button.name);
        g.interface_mut().tm.set_target_program(pid);
    }
}

impl Default for Decrypter {
    fn default() -> Self {
        Self::new()
    }
}

impl UplinkTask for Decrypter {
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
        let sprogress = format!("decrypter_progress {pid}");

        if self.status == DECRYPTER_OFF {
            // Not decrypting - look for a new target
            if self.source.is_some() {
                // A new source file has been specified - start decrypting it
                self.status = DECRYPTER_INPROGRESS;

                g.ecl.register_caption_change(&sprogress, "Decrypting...", 0, None);
            }
        } else if self.status == DECRYPTER_INPROGRESS {
            let owner = self.source.clone().expect("UplinkAssert (source)");

            // Check we are still connected
            if self.remotefile && !g.player().is_connected() {
                Vanbakel::remove_task(g, pid);
                return;
            }

            let sourceindex = self.sourceindex;
            let encrypted = databank(g, &owner)
                .and_then(|db| db.get_data(sourceindex))
                .expect("UplinkAssert (data)")
                .encrypted;

            for _count in 0..n {
                // Decrypt in progress
                if self.base.version <= encrypted as f32 {
                    self.progress += 1;
                } else {
                    // progress += (version - data->encrypted): float sum truncated back to int
                    self.progress =
                        (self.progress as f32 + (self.base.version - encrypted as f32)) as i32;
                }

                let width = (120.0 * (self.progress as f32 / self.numticksrequired as f32)) as i32;
                set_button_width(g, &sprogress, width);

                if self.progress >= self.numticksrequired {
                    // Finished decrypting now
                    self.status = DECRYPTER_FINISHED;

                    if self.base.version >= encrypted as f32 {
                        g.ecl.register_caption_change_auto(&sprogress, "Finished", None);
                        if let Some(data) = databank_mut(g, &owner).and_then(|db| db.get_data_mut(sourceindex)) {
                            data.encrypted = 0;
                        }
                    } else {
                        g.ecl.register_caption_change_auto(&sprogress, "Failed", None);
                    }

                    break;
                }
            }
        } else if self.status == DECRYPTER_FINISHED {
            self.source = None;
            self.sourceindex = -1;
            self.progress = 0;
            self.numticksrequired = 0;
            self.status = DECRYPTER_OFF;
        }
    }

    fn create_interface(&mut self, g: &mut Ctx) {
        if !self.is_interface_visible(g) {
            let pid = self.base.pid;
            let (stitle, sborder, sprogress, sclose) = window_names(pid);

            g.ecl.register_button(265, 450, 20, 15, "", "Decrypter", &stitle);
            opengl_interface::button_assignbitmap(g, &stitle, "software/dec.tif");

            g.ecl.register_button(285, 450, 120, 15, "", "", &sborder);
            g.ecl.register_button_callbacks(
                &sborder,
                Some(Self::border_draw),
                Some(Self::border_click),
                Some(opengl_interface::button_click),
                Some(opengl_interface::button_highlight),
            );

            g.ecl.register_button(285, 450, 120, 13, "Select target", "Shows the progress of the decryption", &sprogress);
            g.ecl.register_button_callbacks(
                &sprogress,
                Some(Self::progress_draw),
                Some(Self::border_click),
                Some(opengl_interface::button_click),
                Some(opengl_interface::button_highlight),
            );

            g.ecl.register_button(405, 450, 13, 13, "", "Close the Decrypter (and stop decrypting)", &sclose);
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
        let sborder = format!("decrypter_border {}", self.base.pid);
        g.ecl.get_button(&sborder).is_some()
    }

    fn set_target(&mut self, g: &mut Ctx, target: ObjRef, targetstring: &str, targetint: i32) {
        // target : DataBank containing data to be decrypted
        // targetstring : name of button representing data
        // targetint : index of data

        if self.status != DECRYPTER_OFF {
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
                self.numticksrequired = size * TICKSREQUIRED_DECRYPT;
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
