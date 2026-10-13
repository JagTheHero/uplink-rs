//! Port of source/interface/taskmanager/defrag.h + source/interface/taskmanager/defrag.cpp

use uplink_engine::{gl, text};

use crate::app::globals::ALPHA;
use crate::app::opengl_interface;
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::game::data::data::TICKSREQUIRED_DEFRAG;
use crate::interface::interfacescreen::ScreenSlot;
use crate::interface::localinterface::localinterface::SCREEN_MEMORY;
use crate::interface::localinterface::memory_interface::MemoryInterface;
use crate::interface::taskmanager::uplinktask::{UplinkTask, UplinkTaskBase};
use crate::vanbakel::Vanbakel;
use crate::world::computer::databank::Data;

#[derive(Debug)]
pub struct Defrag {
    pub base: UplinkTaskBase,
    /// 0 = static, 1 = defragging, 4 = finished, 2 = done
    pub progress: i32,
    pub ticksdone: i32,
    /// Used to syncronise with task manager
    pub ticksrequired: i32,
    /// Used to syncronise events to real time (unix seconds)
    pub timersync: i64,
    pub currentmemoryslot: i32,
    pub firstfreememoryslot: i32,
    /// C++ `LList<Data *> transitData`: files picked up from memory, waiting to be dropped.
    pub transitdata: Vec<Data>,
}

/// C++ `sscanf ( button->name, "%s %d", bname, &pid )`: the PID is the second word of the name.
fn parse_pid(name: &str) -> i32 {
    name.split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(-1)
}

/// C++ `time(NULL)`: unix seconds.
fn time_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// The button names of this task's window: title, border, progress, close.
fn window_names(pid: i32) -> (String, String, String, String) {
    (
        format!("defrag_title {pid}"),
        format!("defrag_border {pid}"),
        format!("defrag_progress {pid}"),
        format!("defrag_close {pid}"),
    )
}

/// C++ `EclGetButton ( sprogress )->width = w; EclDirtyButton ( sprogress );`
fn set_button_width(g: &mut Ctx, name: &str, width: i32) {
    if let Some(b) = g.ecl.get_button_mut(name) {
        b.width = width;
    }
    g.ecl.dirty_button(name);
}

/// The local interface's screen slot (used with `ScreenSlot::call`).
fn local_screen_slot(g: &mut Ctx) -> &mut ScreenSlot {
    &mut g.interface_mut().li.screen
}

impl Defrag {
    pub fn new() -> Self {
        Defrag {
            base: UplinkTaskBase {
                version: 1.0,
                targetint: -1,
                ..Default::default()
            },
            progress: 0,
            ticksdone: 0,
            ticksrequired: 0,
            timersync: 0,
            currentmemoryslot: -1,
            firstfreememoryslot: -1,
            transitdata: Vec::new(),
        }
    }

    /// C++ `Defrag::BorderDraw` (static button callback).
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

    /// C++ `Defrag::ProgressDraw` (static button callback).
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

    /// C++ `Defrag::GoClick` (static button callback).
    pub fn go_click(g: &mut Ctx, button: &Button) {
        let pid = parse_pid(&button.name);

        // Set it going
        let totalmemory = g.player().gateway.databank.get_size();

        let thistask = g
            .svb
            .get_task_mut::<Defrag>(pid)
            .expect("UplinkAssert (thistask)");
        thistask.progress = 1;
        thistask.ticksrequired = TICKSREQUIRED_DEFRAG * totalmemory;
        thistask.ticksdone = 0;
        thistask.currentmemoryslot = -1;
        thistask.firstfreememoryslot = -1;
    }

    /// C++ `Defrag::CloseClick` (static button callback).
    pub fn close_click(g: &mut Ctx, button: &Button) {
        let pid = parse_pid(&button.name);

        // Should dump any files in transit to memory now
        // Or they will be lost
        let transit = {
            let thistask = g
                .svb
                .get_task_mut::<Defrag>(pid)
                .expect("UplinkAssert (thistask)");
            std::mem::take(&mut thistask.transitdata)
        };
        for data in transit {
            g.player_mut().gateway.databank.put_data(data);
        }

        // Now we can shut down
        Vanbakel::remove_task(g, pid);
    }

    /// C++ `Defrag::HandleNewMemorySlot` (protected).
    pub fn handle_new_memory_slot(&mut self, g: &mut Ctx, index: i32) {
        // Should we pick up the file in this memory slot?
        // Pick up all files when they are first encountered
        {
            let databank = &mut g.player_mut().gateway.databank;
            if let Some(file) = databank.get_data(index).cloned() {
                // Data *theFileCopy = new Data ( file );
                let dataindex = databank.get_data_index(index);
                databank.remove_data_file(dataindex);
                self.transitdata.push(file);
            }
        }

        // Should we update the start of blank space?
        let slot_is_free = g.player().gateway.databank.get_data(index).is_none();
        if slot_is_free && self.firstfreememoryslot == -1 {
            self.firstfreememoryslot = index;
        }

        // Should we dump any of our stored files here?
        // Dump files from transitData if there is enough room
        let freespace = 1 + (index - self.firstfreememoryslot);
        if !self.transitdata.is_empty() {
            let size = self.transitdata[0].size;
            if size <= freespace {
                let transit = self.transitdata.remove(0);
                g.player_mut()
                    .gateway
                    .databank
                    .put_data_at(transit, self.firstfreememoryslot);

                if g.player()
                    .gateway
                    .databank
                    .get_data(self.firstfreememoryslot + size)
                    .is_none()
                {
                    self.firstfreememoryslot += size;
                } else {
                    self.firstfreememoryslot = -1;
                }
            }
        }

        // If the player is looking at his memory banks right now
        // Then we have to tell them to update
        let current_screen = g.interface().li.in_screen();
        if current_screen == SCREEN_MEMORY {
            ScreenSlot::call(g, local_screen_slot, |s, g| {
                if let Some(mi) = s.as_any_mut().downcast_mut::<MemoryInterface>() {
                    mi.special_highlight(index);
                    mi.force_update_all(g);
                }
            });
        }
    }
}

impl Default for Defrag {
    fn default() -> Self {
        Self::new()
    }
}

impl UplinkTask for Defrag {
    fn base(&self) -> &UplinkTaskBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut UplinkTaskBase {
        &mut self.base
    }
    fn initialise(&mut self, _g: &mut Ctx) {}
    fn tick(&mut self, g: &mut Ctx, _n: i32) {
        // C++ Defrag::Tick ignores n: progress is driven by ticksdone / ticksrequired
        if !self.is_interface_visible(g) {
            return;
        }
        let pid = self.base.pid;
        let sprogress = format!("defrag_progress {pid}");

        if self.progress == 0 {
            // Do nothing - waiting for user to click GO
        } else if self.progress == 1 {
            // We are defragging
            self.ticksdone += 1;

            let current_progress = self.ticksdone as f32 / self.ticksrequired as f32;
            set_button_width(g, &sprogress, (120.0 * current_progress) as i32);

            let totalmemory = g.player().gateway.databank.get_size();
            let new_memory_slot = (totalmemory as f32 * current_progress) as i32;

            if new_memory_slot > self.currentmemoryslot {
                for i in (self.currentmemoryslot + 1)..=new_memory_slot {
                    self.handle_new_memory_slot(g, i);
                }
                self.currentmemoryslot = new_memory_slot;
            }

            if self.ticksdone >= self.ticksrequired {
                self.progress = 4;
                g.ecl.register_caption_change_auto(&sprogress, "Finished", None);
                self.timersync = time_now() + 2;
            }
        } else if self.progress == 2 {
            // We are done
            if time_now() > self.timersync {
                Vanbakel::remove_task(g, pid);
            }
        }
        // PORT: as in the C++, progress 4 ("Finished") has no branch here, so the task stays
        // open after it reports Finished.
    }

    fn create_interface(&mut self, g: &mut Ctx) {
        if !self.is_interface_visible(g) {
            let pid = self.base.pid;
            let (stitle, sborder, sprogress, sclose) = window_names(pid);

            g.ecl.register_button(265, 422, 20, 15, "", "Defrag the files on your computer", &stitle);
            opengl_interface::button_assignbitmap(g, &stitle, "software/go.tif");
            g.ecl.register_button_callback(&stitle, Self::go_click);

            g.ecl.register_button(285, 422, 120, 15, "", "", &sborder);
            g.ecl.register_button_callbacks(&sborder, Some(Self::border_draw), None, None, None);

            g.ecl.register_button(285, 423, 0, 13, "Defrag", "Shows the progress of the defrag", &sprogress);
            g.ecl.register_button_callbacks(&sprogress, Some(Self::progress_draw), None, None, None);

            g.ecl.register_button(405, 423, 13, 13, "", "Close the Defragger", &sclose);
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
        let stitle = format!("defrag_title {}", self.base.pid);
        g.ecl.get_button(&stitle).is_some()
    }

    crate::impl_as_any!();
}
