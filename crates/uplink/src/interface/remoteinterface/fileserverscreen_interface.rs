//! Port of source/interface/remoteinterface/fileserverscreen_interface.h + source/interface/remoteinterface/fileserverscreen_interface.cpp

use uplink_engine::{gl, text};

use crate::app::opengl_interface::{
    border_draw, button_assignbitmaps, button_click, button_highlight, clear_draw, create_msgbox,
};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::remoteinterface::remoteinterface::{RemoteInterface, SCREEN_FILESERVERSCREEN};
use crate::interface::remoteinterface::remoteinterfacescreen::{draw_main_title, draw_sub_title};
use crate::interface::taskmanager::taskmanager::TaskManager;
use crate::objref::{DataBankOwner, ObjRef, ScreenRef};
use crate::world::computer::computerscreen::genericscreen::GenericScreen;
use crate::world::computer::logbank::{AccessLog, LOG_NOTSUSPICIOUS, LOG_TYPE_TEXT};

/// Number of file rows shown at once (C++ literal 15 in the loops).
pub const NUM_FILE_ROWS: i32 = 15;

/// File server screen: lists the files on the remote host's databank, with a scroll bar and
/// select-to-target handling for the task manager.
#[derive(Debug, Default)]
pub struct FileServerScreenInterface {
    pub cs: Option<ScreenRef>,
    // PORT: these were `static` members in C++ (shared by every instance). They are kept per
    // instance here; `create` resets baseoffset, and previousnumfiles only drives redraws.
    /// Index of the first file shown in the list (scroll offset).
    pub baseoffset: i32,
    /// Databank file count seen on the last `update`, used to decide when to redraw rows.
    pub previousnumfiles: i32,
}

/// C++ `sscanf(name, "<prefix> %d", &n)`. Defaults to 0 if the name does not match.
fn parse_suffix(name: &str, prefix: &str) -> i32 {
    name.strip_prefix(prefix)
        .and_then(|rest| rest.trim_start().parse().ok())
        .unwrap_or(0)
}

fn file_row_name(i: i32) -> String {
    format!("fileserverscreen_file {i}")
}

fn click_button_name(nextpage: i32) -> String {
    format!("fileserverscreen_click {nextpage}")
}

/// The `baseoffset` static: 0 if the screen is not the current one.
fn fs_baseoffset(g: &Ctx) -> i32 {
    g.interface()
        .ri
        .screen
        .get::<FileServerScreenInterface>()
        .map(|s| s.baseoffset)
        .unwrap_or(0)
}

fn fs_state_mut(g: &mut Ctx) -> Option<&mut FileServerScreenInterface> {
    g.interface_mut().ri.screen.get_mut::<FileServerScreenInterface>()
}

/// `VLocation *vl = GetVLocation(GetPlayer()->remotehost); vl->computer`: name of the remote computer.
fn remote_computer_name(g: &Ctx) -> String {
    let remotehost = &g.player().person.remotehost;
    g.world()
        .get_vlocation(remotehost)
        .expect("UplinkAssert(vl)")
        .computer
        .clone()
}

/// `GetPlayer()->GetConnection()->GetGhost()`
fn player_ghost(g: &Ctx) -> String {
    g.player()
        .person
        .get_connection()
        .get_ghost()
        .unwrap_or("")
        .to_string()
}

impl FileServerScreenInterface {
    /// `CloseClick` (static MouseFn): runs the page named in the button (`fileserverscreen_click N`).
    pub fn close_click(g: &mut Ctx, button: &Button) {
        let nextpage = parse_suffix(&button.name, "fileserverscreen_click");
        if nextpage != -1 {
            RemoteInterface::run_screen(g, nextpage);
        }
    }

    /// `FileClick` (static MouseFn): logs the access and targets the file's databank entry,
    /// or an empty memory slot. Blocked when the remote firewall is running.
    pub fn file_click(g: &mut Ctx, button: &Button) {
        let fileindex = parse_suffix(&button.name, "fileserverscreen_file") + fs_baseoffset(g);

        let remote = remote_computer_name(g);
        let comp = g.world().get_computer(&remote).expect("UplinkAssert(comp)");

        if comp.security.is_running_firewall() {
            create_msgbox(g, "Error", "Denied access by Firewall");
            return;
        }

        let data_title = comp.databank.get_data_file(fileindex).map(|d| d.title.clone());
        let memoryindex = comp.databank.get_memory_index(fileindex);

        let date = g.world().date.clone();
        let ghost = player_ghost(g);
        let databank = ObjRef::DataBank(DataBankOwner::Computer(remote.clone()));

        if let (Some(title), true) = (data_title, memoryindex != -1) {
            // Log this access
            let action = format!("Accessed file {title}");
            let mut log = AccessLog::new();
            log.set_properties(&date, &ghost, "PLAYER", LOG_NOTSUSPICIOUS, LOG_TYPE_TEXT);
            log.set_data1(Some(&action));
            g.world_mut()
                .get_computer_mut(&remote)
                .expect("UplinkAssert(comp)")
                .logbank
                .add_log(log, -1);

            // Target this databank
            TaskManager::set_program_target(g, databank, &button.name, memoryindex);
        } else {
            // There is nothing here, so target an empty spot in memory
            TaskManager::set_program_target(g, databank, &button.name, -1);

            // Log this access
            let action = format!("Accessed memory file index {fileindex}");
            let mut log = AccessLog::new();
            log.set_properties(&date, &ghost, "PLAYER", LOG_NOTSUSPICIOUS, LOG_TYPE_TEXT);
            log.set_data1(Some(&action));
            g.world_mut()
                .get_computer_mut(&remote)
                .expect("UplinkAssert(comp)")
                .logbank
                .add_log(log, -1);
        }
    }

    /// `ScrollUpClick` (static MouseFn)
    pub fn scroll_up_click(g: &mut Ctx, _button: &Button) {
        if let Some(fs) = fs_state_mut(g) {
            fs.baseoffset -= 1;
            if fs.baseoffset < 0 {
                fs.baseoffset = 0;
            }
        }

        for i in 0..NUM_FILE_ROWS {
            g.ecl.dirty_button(&file_row_name(i));
        }
    }

    /// `ScrollDownClick` (static MouseFn)
    pub fn scroll_down_click(g: &mut Ctx, _button: &Button) {
        if let Some(fs) = fs_state_mut(g) {
            fs.baseoffset += 1;
        }

        for i in 0..NUM_FILE_ROWS {
            g.ecl.dirty_button(&file_row_name(i));
        }
    }

    /// `FileDraw` (static DrawFn): striped row background, file title, size, encryption, compression.
    pub fn file_draw(g: &mut Ctx, button: &Button, highlighted: bool, _clicked: bool) {
        let fileindex = parse_suffix(&button.name, "fileserverscreen_file") + fs_baseoffset(g);

        let remote = remote_computer_name(g);
        let comp = g.world().get_computer(&remote).expect("UplinkAssert(comp)");

        let data = comp
            .databank
            .get_data_file(fileindex)
            .map(|d| (d.title.clone(), d.size, d.encrypted, d.compressed));
        let memory_size = comp.databank.get_size();

        let (x, y, w, h) = (button.x, button.y, button.width, button.height);

        if let Some((title, size, encrypted, compressed)) = data {
            // Striped background
            let (first, second) = if fileindex % 2 == 0 {
                ((8, 20, 80), (8, 20, 0))
            } else {
                ((8, 20, 0), (8, 20, 80))
            };
            let colour = |c: (u8, u8, u8)| gl::color3ub(c.0, c.1, c.2);
            gl::begin(gl::QUADS);
            colour(first);
            gl::vertex2i(x, y);
            colour(second);
            gl::vertex2i(x + w, y);
            colour(first);
            gl::vertex2i(x + w, y + h);
            colour(second);
            gl::vertex2i(x, y + h);
            gl::end();

            gl::color4f(1.0, 1.0, 1.0, 1.0);
            text::draw_text_default(x, y + 10, &title);

            let size_text = format!("{size} GigaQuads");
            text::draw_text_default(x + 150, y + 10, &size_text);

            if encrypted > 0 {
                let encrypttext = format!("Level {encrypted}");
                text::draw_text_default(x + 250, y + 10, &encrypttext);
            }

            if compressed > 0 {
                let compressedtext = format!("Level {compressed}");
                text::draw_text_default(x + 330, y + 10, &compressedtext);
            }
        } else {
            clear_draw(g, x, y, w, h);
        }

        // Draw a bounding box
        if highlighted && fileindex <= memory_size {
            gl::color4f(1.0, 1.0, 1.0, 1.0);
            border_draw(button);
        }
    }

    /// `GetComputerScreen` (typed accessor; the C++ casts `cs`).
    pub fn get_computer_screen<'a>(&self, g: &'a Ctx) -> Option<&'a GenericScreen> {
        let r = self.cs.as_ref()?;
        g.world()
            .get_computer(&r.computer)?
            .get_computer_screen(r.index)?
            .as_generic_screen()
    }
}

impl InterfaceScreen for FileServerScreenInterface {
    crate::impl_as_any!();

    /// `Create(ComputerScreen *newcs)` body. The C++ no-argument `Create()` overload maps to this
    /// too, since `cs` is already set by `set_computer_screen`.
    fn create(&mut self, g: &mut Ctx) {
        // C++ no-argument Create(): `if (cs) Create(cs); else printf(...)`
        if self.cs.is_none() {
            println!("FileServerScreenInterface::Create, tried to create when GenericScreen==NULL");
            return;
        }

        if self.is_visible(g) {
            return;
        }

        let (computer_name, nextpage, maintitle, subtitle) = {
            let gs = self.get_computer_screen(g).expect("UplinkAssert(gs)");
            (
                gs.base.computer.clone(),
                gs.nextpage,
                gs.base.maintitle.clone(),
                gs.base.subtitle.clone(),
            )
        };

        let num_data_files = g
            .world()
            .get_computer(&computer_name)
            .expect("UplinkAssert(comp)")
            .databank
            .num_data_files();

        let date = g.world().date.clone();
        let ghost = player_ghost(g);
        let mut log = AccessLog::new();
        log.set_properties(&date, &ghost, "PLAYER", LOG_NOTSUSPICIOUS, LOG_TYPE_TEXT);
        log.set_data1(Some("Accessed fileserver"));
        g.world_mut()
            .get_computer_mut(&computer_name)
            .expect("UplinkAssert(comp)")
            .logbank
            .add_log(log, -1);

        g.ecl.register_button(80, 60, 350, 25, &maintitle, "", "fileserverscreen_maintitle");
        g.ecl.register_button_callbacks("fileserverscreen_maintitle", Some(draw_main_title), None, None, None);
        g.ecl.register_button(80, 80, 350, 20, &subtitle, "", "fileserverscreen_subtitle");
        g.ecl.register_button_callbacks("fileserverscreen_subtitle", Some(draw_sub_title), None, None, None);

        g.ecl.register_button(15, 120, 144, 15, "Filename", "", "fileserverscreen_filenametitle");
        g.ecl.register_button(162, 120, 96, 15, "Size", "", "fileserverscreen_sizetitle");
        g.ecl.register_button(261, 120, 76, 15, "Encryption", "", "fileserverscreen_encryption");
        g.ecl.register_button(340, 120, 75, 15, "Compression", "", "fileserverscreen_compression");

        // Create the file entries
        for i in 0..NUM_FILE_ROWS {
            let name = file_row_name(i);
            g.ecl.register_button(15, 140 + i * 15, 400, 14, "", "Select this file", &name);
            g.ecl.register_button_callbacks(
                &name,
                Some(FileServerScreenInterface::file_draw),
                Some(FileServerScreenInterface::file_click),
                Some(button_click),
                Some(button_highlight),
            );
        }

        if num_data_files >= 14 {
            // Create the scroll bar
            g.ecl.register_button(420, 140, 15, 15, "^", "Scroll up", "fileserverscreen_scrollup");
            button_assignbitmaps(g, "fileserverscreen_scrollup", "up.tif", "up_h.tif", "up_c.tif");
            g.ecl.register_button_callback("fileserverscreen_scrollup", FileServerScreenInterface::scroll_up_click);

            g.ecl.register_button(420, 157, 15, 13 * 15 - 4, "", "", "fileserverscreen_scrollbar");

            g.ecl.register_button(420, 140 + 14 * 15, 15, 15, "v", "Scroll down", "fileserverscreen_scrolldown");
            button_assignbitmaps(g, "fileserverscreen_scrolldown", "down.tif", "down_h.tif", "down_c.tif");
            g.ecl.register_button_callback("fileserverscreen_scrolldown", FileServerScreenInterface::scroll_down_click);
        }

        // Create the close button
        let closename = click_button_name(nextpage);
        g.ecl.register_button(421, 121, 13, 13, "", "Close the File Server Screen", &closename);
        button_assignbitmaps(g, &closename, "close.tif", "close_h.tif", "close_c.tif");
        g.ecl.register_button_callback(&closename, FileServerScreenInterface::close_click);

        // Move to the top
        self.baseoffset = 0;
    }

    fn remove(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            return;
        }

        g.ecl.remove_button("fileserverscreen_maintitle");
        g.ecl.remove_button("fileserverscreen_subtitle");
        g.ecl.remove_button("fileserverscreen_filenametitle");
        g.ecl.remove_button("fileserverscreen_sizetitle");
        g.ecl.remove_button("fileserverscreen_encryption");
        g.ecl.remove_button("fileserverscreen_compression");

        for i in 0..NUM_FILE_ROWS {
            g.ecl.remove_button(&file_row_name(i));
        }

        g.ecl.remove_button("fileserverscreen_scrollup");
        g.ecl.remove_button("fileserverscreen_scrollbar");
        g.ecl.remove_button("fileserverscreen_scrolldown");

        let nextpage = self
            .get_computer_screen(g)
            .map(|gs| gs.nextpage)
            .expect("UplinkAssert(gs)");
        g.ecl.remove_button(&click_button_name(nextpage));
    }

    fn update(&mut self, g: &mut Ctx) {
        let remote = remote_computer_name(g);
        let newnumfiles = g
            .world()
            .get_computer(&remote)
            .expect("UplinkAssert(comp)")
            .databank
            .num_data_files();

        if newnumfiles != self.previousnumfiles {
            for i in 0..NUM_FILE_ROWS {
                g.ecl.dirty_button(&file_row_name(i));
            }
            self.previousnumfiles = newnumfiles;
        }
    }

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("fileserverscreen_maintitle").is_some()
    }

    /// `EscapeKeyPressed`: closes the screen and always reports the key as handled.
    fn escape_key_pressed(&mut self, g: &mut Ctx) -> bool {
        // C++ builds the close button's name from nextpage, looks it up, and calls CloseClick.
        // The screen is taken out while this runs, so read nextpage from `self`.
        let nextpage = self
            .get_computer_screen(g)
            .map(|gs| gs.nextpage)
            .expect("UplinkAssert(gs)");
        assert!(
            g.ecl.get_button(&click_button_name(nextpage)).is_some(),
            "UplinkAssert(button)"
        );

        if nextpage != -1 {
            RemoteInterface::run_screen(g, nextpage);
        }
        true
    }

    fn screen_id(&self) -> i32 {
        SCREEN_FILESERVERSCREEN
    }

    fn set_computer_screen(&mut self, cs: ScreenRef) {
        self.cs = Some(cs);
    }

    fn computer_screen_ref(&self) -> Option<&ScreenRef> {
        self.cs.as_ref()
    }
}
