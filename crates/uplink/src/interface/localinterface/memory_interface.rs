//! Port of source/interface/localinterface/memory_interface.h + source/interface/localinterface/memory_interface.cpp
//! Memory screen: lists the blocks of the player's gateway databank (programs and data files).

use std::sync::atomic::{AtomicI32, Ordering};

use uplink_engine::{gl, text};

use crate::app::globals::ALPHA;
use crate::app::opengl_interface::{button_click, clear_draw, textbutton_draw};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::impl_as_any;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::localinterface::localinterface::{LocalInterface, PANELSIZE, SCREEN_MEMORY, SCREEN_NONE};
use crate::interface::scrollbox::ScrollBox;
use crate::interface::taskmanager::taskmanager::TaskManager;
use crate::objref::{DataBankOwner, ObjRef};
use crate::world::computer::databank::{DataBank, DATATYPE_DATA, DATATYPE_PROGRAM};

/// C++ `static int previousnumfiles`
static PREVIOUSNUMFILES: AtomicI32 = AtomicI32::new(0);
/// C++ `static int baseoffset`: index of the first visible memory block (scroll position).
static BASEOFFSET: AtomicI32 = AtomicI32::new(0);
/// C++ `static int currentprogramindex`: databank program highlighted by the mouse (-1 = none).
static CURRENTPROGRAMINDEX: AtomicI32 = AtomicI32::new(-1);
/// C++ `static int specialHighlight`: block index drawn highlighted (-1 = none).
static SPECIAL_HIGHLIGHT: AtomicI32 = AtomicI32::new(-1);

// PORT: the C++ members are class statics, so they outlive the screen object (the scroll position
// survives closing and reopening the memory screen). The screen object is recreated by RunScreen,
// so they are process-wide atomics here (the game is single threaded) rather than struct fields.
#[derive(Debug, Clone, Default)]
pub struct MemoryInterface;

/// C++ `sscanf(name, "<prefix> %d", ...)`: the number after `prefix`, 0 if there is none.
fn name_index(name: &str, prefix: &str) -> i32 {
    name.strip_prefix(prefix).and_then(parse_leading_i32).unwrap_or(0)
}

/// C++ `sscanf(s, "%d", ...)`: skips leading white space, reads an optional sign and digits.
fn parse_leading_i32(s: &str) -> Option<i32> {
    let s = s.trim_start();
    let bytes = s.as_bytes();
    let mut end = 0;
    if end < bytes.len() && (bytes[end] == b'+' || bytes[end] == b'-') {
        end += 1;
    }
    let digits_start = end;
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    if end == digits_start {
        return None;
    }
    s[..end].parse().ok()
}

/// `"memory_block %d"`
fn block_name(i: i32) -> String {
    format!("memory_block {}", i)
}

/// `(screenw, screenh, paneltop, panelwidth)`, recomputed from the options as in the C++.
fn panel_metrics(g: &Ctx) -> (i32, i32, i32, i32) {
    let screenw = g.app.options.get_option_value("graphics_screenwidth");
    let screenh = g.app.options.get_option_value("graphics_screenheight");
    let paneltop = (100.0 * ((screenw as f64 * PANELSIZE) / 188.0) + 30.0) as i32;
    let panelwidth = (screenw as f64 * PANELSIZE) as i32;
    (screenw, screenh, paneltop, panelwidth)
}

/// Number of memory block rows that fit in the panel.
fn memory_rows(g: &Ctx) -> i32 {
    let (_, screenh, paneltop, _) = panel_metrics(g);
    ((screenh - 50) - (paneltop + 50)) / 10
}

/// `game->GetWorld()->GetPlayer()->gateway.databank`
fn db(g: &Ctx) -> &DataBank {
    &g.player().gateway.databank
}

/// `(baseoffset, currentprogramindex, specialHighlight)`
fn memory_state() -> (i32, i32, i32) {
    (
        BASEOFFSET.load(Ordering::Relaxed),
        CURRENTPROGRAMINDEX.load(Ordering::Relaxed),
        SPECIAL_HIGHLIGHT.load(Ordering::Relaxed),
    )
}

/// Dirties every visible block whose databank file is `program`.
fn dirty_blocks_with_program(g: &mut Ctx, baseoffset: i32, numrows: i32, program: i32) {
    for i in baseoffset..baseoffset + numrows {
        if db(g).get_data_index(i) == program {
            g.ecl.dirty_button(&block_name(i - baseoffset));
        }
    }
}

impl MemoryInterface {
    /// C++ `MemoryInterface::TitleClick` (static button callback).
    pub fn title_click(g: &mut Ctx, _button: &Button) {
        LocalInterface::run_screen(g, SCREEN_NONE, -1);
    }

    /// C++ `MemoryInterface::MemoryBlockDraw` (static draw callback).
    pub fn memory_block_draw(g: &mut Ctx, button: &Button, highlighted: bool, _clicked: bool) {
        let (baseoffset, currentprogramindex, special_highlight) = memory_state();
        let index = name_index(&button.name, "memory_block ") + baseoffset;
        let (x, y, w, h) = (button.x, button.y, button.width, button.height);

        if index < db(g).get_size() {
            let data = db(g).get_data(index).cloned();

            // Write the memory address
            // (With a black background)

            clear_draw(g, x, y, 30, h);

            let caption = format!("{:03}", index);
            gl::color4f(1.0, 1.0, 1.0, 1.0);
            text::draw_text_default(x, y + h - 1, &caption);

            // Draw a box, colour coded on the data type
            // Set the colour

            match &data {
                Some(d) if d.type_ == DATATYPE_DATA => gl::color4f(0.2, 0.8, 0.2, ALPHA),
                Some(d) if d.type_ == DATATYPE_PROGRAM => gl::color4f(0.8, 0.2, 0.2, ALPHA),
                Some(_) => gl::color4f(0.4, 0.4, 0.4, ALPHA),
                None => gl::color4f(0.3, 0.3, 0.5, ALPHA),
            }

            // Draw the background colour of the box
            gl::begin(gl::QUADS);
            gl::vertex2i(x + 30, y);
            gl::vertex2i(x + 30, y + h);

            if highlighted || index == special_highlight {
                gl::color4f(0.6, 0.6, 0.8, ALPHA);
            } else {
                gl::color4f(0.3, 0.3, 0.8, ALPHA);
            }

            gl::vertex2i(x + w, y + h);
            gl::vertex2i(x + w, y);
            gl::end();

            // Draw a box if this program is highlighted

            gl::color4f(1.0, 1.0, 1.0, ALPHA);

            if currentprogramindex != -1 && db(g).get_data_index(index) == currentprogramindex {
                gl::begin(gl::LINES);

                gl::vertex2i(x + 30, y);
                gl::vertex2i(x + 30, y + h);
                gl::vertex2i(x + w - 1, y);
                gl::vertex2i(x + w - 1, y + h);

                if db(g).get_data_index(index - 1) != currentprogramindex {
                    gl::vertex2i(x + 30, y);
                    gl::vertex2i(x + w - 1, y);
                }

                if db(g).get_data_index(index + 1) != currentprogramindex {
                    gl::vertex2i(x + 30, y + h - 1);
                    gl::vertex2i(x + w - 1, y + h - 1);
                }

                gl::end();
            }

            // Write the data title and version if there is one

            if let Some(d) = &data {
                if db(g).get_data_index(index) != db(g).get_data_index(index - 1) {
                    text::draw_text_default(x + 30, y + 8, &d.title);

                    if d.type_ == DATATYPE_PROGRAM {
                        let version = format!("v{:.1}", d.version);
                        text::draw_text_default(x + w - 23, y + 8, &version);
                    }
                }
            }
        } else {
            // Index outside memory banks

            clear_draw(g, x, y, w, h);

            if index == db(g).get_size() * 5 {
                gl::color4f(1.0, 1.0, 1.0, 1.0);
                text::draw_text_default(x, y + 8, "Mouse-Button now fucked");
            }
        }
    }

    /// C++ `MemoryInterface::MemoryBlockHighlight` (static mouse-move callback).
    pub fn memory_block_highlight(g: &mut Ctx, button: &Button) {
        // Dirty all blocks that shared the old currentprogramindex;

        let (baseoffset, old_program, _) = memory_state();
        let numrows = memory_rows(g);

        if old_program != -1 {
            dirty_blocks_with_program(g, baseoffset, numrows, old_program);
        }

        // Work out the new current program index

        let index = name_index(&button.name, "memory_block ") + baseoffset;

        let new_program = if index < db(g).get_size() {
            db(g).get_data_index(index)
        } else {
            -1
        };
        CURRENTPROGRAMINDEX.store(new_program, Ordering::Relaxed);

        g.ecl.highlight_button(&button.name);

        // Dirty all other blocks that share this currentprogramindex

        if new_program != -1 {
            dirty_blocks_with_program(g, baseoffset, numrows, new_program);
        }
    }

    /// C++ `MemoryInterface::MemoryBlockClick` (static button callback).
    pub fn memory_block_click(g: &mut Ctx, button: &Button) {
        let index = name_index(&button.name, "memory_block ") + memory_state().0;

        TaskManager::set_program_target(g, ObjRef::DataBank(DataBankOwner::Gateway), &button.name, index);
    }

    /// C++ `MemoryInterface::ScrollChange` (static scroll-box callback).
    pub fn scroll_change(g: &mut Ctx, _scrollname: &str, new_value: i32) {
        BASEOFFSET.store(new_value, Ordering::Relaxed);

        let numrows = memory_rows(g);
        for i in 0..numrows {
            g.ecl.dirty_button(&block_name(i));
        }
    }

    /// C++ `MemoryInterface::ForceUpdateAll`
    pub fn force_update_all(&mut self, g: &mut Ctx) {
        PREVIOUSNUMFILES.store(db(g).num_data_files() + 1, Ordering::Relaxed);
    }

    /// C++ `MemoryInterface::SpecialHighlight`
    pub fn special_highlight(&mut self, memory_index: i32) {
        SPECIAL_HIGHLIGHT.store(memory_index, Ordering::Relaxed);
    }
}

impl InterfaceScreen for MemoryInterface {
    fn create(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            return;
        }

        let (screenw, _screenh, paneltop, panelwidth) = panel_metrics(g);

        g.ecl.register_button(screenw - panelwidth - 3, paneltop, panelwidth, 15, "MEMORY BANKS", "Remove the memory screen", "memory_title");
        g.ecl.register_button_callback("memory_title", Self::title_click);

        g.ecl.register_button(screenw - panelwidth - 3, paneltop + 20, panelwidth, 15, "", "", "memory_capacity");
        g.ecl.register_button_callbacks("memory_capacity", Some(textbutton_draw), None, None, None);

        let numrows = memory_rows(g);

        for i in 0..numrows {
            let name = block_name(i);
            g.ecl.register_button(screenw - panelwidth - 3 + 5, paneltop + 50 + i * 10, panelwidth - 25, 10, "", "", &name);
            g.ecl.register_button_callbacks(
                &name,
                Some(Self::memory_block_draw),
                Some(Self::memory_block_click),
                Some(button_click),
                Some(Self::memory_block_highlight),
            );
        }

        let totalmemory = db(g).get_size();
        ScrollBox::create_scroll_box(
            g,
            "memory_scroll",
            screenw - 20,
            paneltop + 50,
            15,
            numrows * 10,
            totalmemory,
            numrows,
            BASEOFFSET.load(Ordering::Relaxed),
            Some(Self::scroll_change),
        );
    }

    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            g.ecl.remove_button("memory_title");
            g.ecl.remove_button("memory_capacity");
            ScrollBox::remove_scroll_box(g, "memory_scroll");

            let numrows = memory_rows(g);

            for i in 0..numrows {
                g.ecl.remove_button(&block_name(i));
            }
        }
    }

    fn update(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            let capacity = format!("Memory Capacity : {} GigaQuads", db(g).get_size());

            if let Some(b) = g.ecl.get_button_mut("memory_capacity") {
                b.set_caption(&capacity);
            }

            let numfiles = db(g).num_data_files();

            if PREVIOUSNUMFILES.load(Ordering::Relaxed) != numfiles {
                // Something has changed
                // Invalidate all the buttons

                let numrows = memory_rows(g);

                let size = db(g).get_size();
                ScrollBox::set_num_items(g, "memory_scroll", size);

                for i in 0..numrows {
                    g.ecl.dirty_button(&block_name(i));
                }

                PREVIOUSNUMFILES.store(numfiles, Ordering::Relaxed);
            }
        }
    }

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("memory_title").is_some()
    }

    fn screen_id(&self) -> i32 {
        SCREEN_MEMORY
    }

    impl_as_any!();
}
