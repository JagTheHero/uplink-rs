//! Port of source/interface/remoteinterface/consolescreen_interface.h + source/interface/remoteinterface/consolescreen_interface.cpp

use uplink_engine::gl;

use crate::app::miscutils::lower_case_string;
use crate::app::opengl_interface as ogl;
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::game::data::data::{TIMEREQUIRED_DELETEONEGIGAQUAD, TIMEREQUIRED_DELETEONELOG};
use crate::interface::interfacescreen::{InterfaceScreen, ScreenSlot};
use crate::interface::remoteinterface::remoteinterface::{RemoteInterface, SCREEN_CONSOLESCREEN};
use crate::objref::ScreenRef;
use crate::world::computer::computer::Computer;
use crate::world::computer::computerscreen::genericscreen::GenericScreen;
use crate::world::computer::databank::{Data, DATATYPE_PROGRAM};
use crate::world::computer::logbank::{AccessLog, LOG_NOTSUSPICIOUS, LOG_TYPE_DELETED, LOG_TYPE_TEXT};
use crate::world::connection::Connection;
use crate::world::date::Date;
use crate::world::generator::plotgenerator::PlotGenerator;

/// Number of visible text lines in the console.
pub const NUMLINES: i32 = 21;

pub const CMDTYPE_NONE: i32 = -1;
pub const CMDTYPE_TEXT: i32 = 0; // Data1 = string
pub const CMDTYPE_DIR: i32 = 1;
pub const CMDTYPE_HELP: i32 = 2;
pub const CMDTYPE_EXIT: i32 = 3;
pub const CMDTYPE_DELETEALL: i32 = 4; // Data1 = directory or NULL to use current
pub const CMDTYPE_CD: i32 = 5;
pub const CMDTYPE_SHUTDOWN: i32 = 6;
pub const CMDTYPE_DISCONNECT: i32 = 7;
pub const CMDTYPE_RUN: i32 = 8; // Data1 = program name
pub const CMDTYPE_RUNPROGRAM: i32 = 9; // Data1 = program name, actually runs it

/// One queued console action (text to print, a command to run, or a delayed step).
#[derive(Debug, Clone)]
pub struct ConsoleCommand {
    /// C++ `TYPE`: one of the `CMDTYPE_*` constants.
    pub type_: i32,
    /// C++ `char *data1` (may be NULL).
    pub data1: Option<String>,
    /// Delay in milliseconds before this command runs.
    pub time: i32,
}

impl Default for ConsoleCommand {
    fn default() -> Self {
        ConsoleCommand {
            type_: CMDTYPE_NONE,
            data1: None,
            time: -1,
        }
    }
}

impl ConsoleCommand {
    /// `ConsoleCommand(int newTYPE, char *newdata1, int newtime)`
    pub fn new(new_type: i32, newdata1: Option<&str>, newtime: i32) -> Self {
        ConsoleCommand {
            type_: new_type,
            data1: newdata1.map(|s| s.to_string()),
            time: newtime,
        }
    }
}

/// Console screen: a text terminal onto a remote computer (dir, cd, delete, run, shutdown, ...).
#[derive(Debug)]
pub struct ConsoleScreenInterface {
    pub cs: Option<ScreenRef>,
    /// 0 is system, 1 is player.
    pub users: Vec<String>,
    /// Commands NOT YET RUN (C++ `queue`: PutDataAtEnd = push, PutDataAtStart = insert at 0).
    pub queue: Vec<ConsoleCommand>,
    /// True while a line of text is "scrolling" on.
    pub waiting: bool,
    /// Ticks at which the next queued command may run (-1 = not waiting on a timer).
    pub timesync: i32,
    /// `char currentdir[8]`: one of "/", "pub", "usr", "log", "rec", "sys".
    pub currentdir: String,
    /// True once the player has deleted the sys directory.
    pub deleted_sys: bool,
}

impl Default for ConsoleScreenInterface {
    /// The C++ constructor: `waiting = false; timesync = -1; deleted_sys = false;`
    fn default() -> Self {
        ConsoleScreenInterface {
            cs: None,
            users: Vec::new(),
            queue: Vec::new(),
            waiting: false,
            timesync: -1,
            currentdir: String::new(),
            deleted_sys: false,
        }
    }
}

// PORT: the C++ static callbacks fetch the screen with GetInterfaceScreen. Here the screen is taken
// out of the remote interface's slot with `ScreenSlot::call` for the duration of the callback, so the
// body can use `g` and the screen at the same time. Inside the screen's own methods, `self` is used.
fn console_slot(g: &mut Ctx) -> &mut ScreenSlot {
    &mut g.interface_mut().ri.screen
}

/// Runs `f` on the console screen if it is the current remote screen (and not busy).
fn with_console<R>(g: &mut Ctx, f: impl FnOnce(&mut ConsoleScreenInterface, &mut Ctx) -> R) -> Option<R> {
    ScreenSlot::call(g, console_slot, |s, g| {
        let console = s.as_any_mut().downcast_mut::<ConsoleScreenInterface>()?;
        Some(f(console, g))
    })
    .flatten()
}

/// `GetComputerScreen()->GetComputer()`, read-only (UplinkAssert: the computer must exist).
fn computer<'a>(g: &'a Ctx, name: &str) -> &'a Computer {
    g.world().get_computer(name).expect("ConsoleScreen: computer missing (UplinkAssert)")
}

fn computer_mut<'a>(g: &'a mut Ctx, name: &str) -> &'a mut Computer {
    g.world_mut().get_computer_mut(name).expect("ConsoleScreen: computer missing (UplinkAssert)")
}

/// The first data file whose lower-cased title equals `program`. The C++ loops `break` after the
/// first match, so only the first one is ever acted on.
fn find_program_file(comp: &Computer, program: &str) -> Option<Data> {
    (0..comp.databank.get_data_size())
        .filter_map(|i| comp.databank.get_data_file(i))
        .find(|d| lower_case_string(&d.title) == program)
        .cloned()
}

/// C++ `lccommand + 3` / `lccommand + 4`: the tail of the string from a fixed byte offset.
fn tail_bytes(s: &str, skip: usize) -> String {
    String::from_utf8_lossy(&s.as_bytes()[skip..]).into_owned()
}

impl ConsoleScreenInterface {
    /// `IsVisibleInterface` (static)
    pub fn is_visible_interface(g: &Ctx) -> bool {
        g.ecl.get_button("console_title").is_some()
    }

    /// `WaitingCallback` (static, caption-change callback)
    pub fn waiting_callback(g: &mut Ctx) {
        with_console(g, |c, _| c.waiting = false);
    }

    /// `TypeHereDraw` (static DrawFn)
    pub fn type_here_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        ogl::textbutton_draw(g, button, highlighted, clicked);
    }

    /// `PostClick` (static MouseFn): takes the typed line and runs it.
    pub fn post_click(g: &mut Ctx, _button: &Button) {
        with_console(g, |c, g| c.post_click_body(g));
    }

    /// `BorderDraw` (static DrawFn)
    pub fn border_draw(_g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        gl::color3f(1.0, 1.0, 1.0);
        ogl::border_draw(button);
    }

    /// `MessageDraw` (static DrawFn)
    pub fn message_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        ogl::clear_draw(g, button.x, button.y, button.width, button.height);

        gl::color3f(0.6, 1.0, 0.6);
        ogl::text_draw(g, button, highlighted, clicked);
    }

    /// The body of `PostClick`, also used by `ReturnKeyPressed` (which calls `PostClick(NULL)`).
    fn post_click_body(&mut self, g: &mut Ctx) {
        let caption = g
            .ecl
            .get_button("console_typehere")
            .expect("console_typehere missing (UplinkAssert)")
            .caption
            .clone();

        // Copy the actual command: the text after the first '>' in the caption.
        let actualcommand: Option<&str> = caption.find('>').map(|i| &caption[i + 1..]);

        // A NULL command goes through sprintf("%s"), which prints "(null)" with glibc.
        self.put_text(1, actualcommand.unwrap_or("(null)"));

        let newcaption = format!("{}:>", self.currentdir);
        g.ecl.register_caption_change(
            "console_typehere",
            &newcaption,
            1,
            None,
        );

        match actualcommand {
            Some(command) => self.run_command(command),
            None => self.run_command("bollocks"),
        }
    }

    /// C++ `queue.PutDataAtStart`
    fn push_front(&mut self, type_: i32, data1: Option<&str>, time: i32) {
        self.queue.insert(0, ConsoleCommand::new(type_, data1, time));
    }

    /// C++ `queue.PutDataAtEnd`
    fn push_back(&mut self, type_: i32, data1: Option<&str>, time: i32) {
        self.queue.push(ConsoleCommand::new(type_, data1, time));
    }

    /// `queue.GetData(0)` + `RemoveData(0)`, asserting there is a command (UplinkAssert).
    fn pop_front_command(&mut self) -> ConsoleCommand {
        assert!(!self.queue.is_empty(), "ConsoleScreen: queue empty (UplinkAssert)");
        self.queue.remove(0)
    }

    /// Name of the computer the screen belongs to (`GetComputerScreen()->GetComputer()`).
    fn screen_computer_name(&self, g: &Ctx) -> String {
        self.get_computer_screen(g)
            .expect("ConsoleScreen: no computer screen (UplinkAssert)")
            .base
            .computer
            .clone()
    }

    /// `AddUser`: adds a user and logs them on.
    pub fn add_user(&mut self, name: &str) {
        self.users.push(name.to_string());

        if name == "System" {
            self.put_text(0, "Console session started");
        } else {
            let message = format!("User '{name}' logged on");
            self.put_text(0, &message);
        }
    }

    /// `SetCurrentDir`: accepts pub/usr/log/rec/sys/"/" (and "." / ".." -> "/").
    pub fn set_current_dir(&mut self, g: &mut Ctx, newcurrentdir: &str) {
        let lcdir = lower_case_string(newcurrentdir);

        if matches!(lcdir.as_str(), "pub" | "usr" | "log" | "rec" | "sys" | "/") {
            self.currentdir = lcdir;
        } else if lcdir == "." || lcdir == ".." {
            self.currentdir = "/".to_string();
        } else {
            self.put_text(0, "Unrecognised directory");
        }

        let newcaption = format!("{}:>", self.currentdir);
        g.ecl.register_caption_change_auto("console_typehere", &newcaption, None);
    }

    /// `PutText(userid, text)`: adds text to the end of the queue.
    pub fn put_text(&mut self, userid: i32, text: &str) {
        if userid == 0 {
            self.push_back(CMDTYPE_TEXT, Some(text), 0);
        } else if userid == 1 {
            let msg = format!("{}:>{}", self.currentdir, text);
            self.push_back(CMDTYPE_TEXT, Some(&msg), 0);
        }
    }

    /// `PutTextAtStart(userid, text)`: adds text to the front of the queue.
    pub fn put_text_at_start(&mut self, userid: i32, text: &str) {
        if userid == 0 {
            self.push_front(CMDTYPE_TEXT, Some(text), 0);
        } else if userid == 1 {
            let msg = format!("{}:>{}", self.currentdir, text);
            self.push_front(CMDTYPE_TEXT, Some(&msg), 0);
        }
    }

    /// `RunCommand(char *command)`: parses a typed command into a queued ConsoleCommand.
    pub fn run_command(&mut self, command: &str) {
        let lccommand = lower_case_string(command);

        if lccommand.contains("help") {
            self.push_back(CMDTYPE_HELP, None, 0);
        } else if lccommand.contains("dir") {
            self.push_back(CMDTYPE_DIR, None, 0);
        } else if lccommand.contains("cd ") {
            let dir = tail_bytes(&lccommand, 3);
            self.push_back(CMDTYPE_CD, Some(&dir), 0);
        } else if lccommand.contains("delete") {
            self.push_back(CMDTYPE_DELETEALL, None, 0);
        } else if lccommand.contains("run ") {
            let program = tail_bytes(&lccommand, 4);
            self.push_back(CMDTYPE_RUN, Some(&program), 0);
        } else if lccommand.contains("exit") {
            self.push_back(CMDTYPE_EXIT, None, 0);
        } else if lccommand.contains("shutdown") {
            self.push_back(CMDTYPE_SHUTDOWN, None, 0);
        } else if lccommand.contains("disconnect") {
            self.push_back(CMDTYPE_DISCONNECT, None, 0);
        } else {
            self.push_back(CMDTYPE_TEXT, Some("Unrecognised text"), 0);
        }

        self.push_back(CMDTYPE_TEXT, Some(" "), 0);
    }

    /// `RunCommand(ConsoleCommand *cc)`: runs one queued command (or re-queues it if delayed).
    pub fn run_command_cc(&mut self, g: &mut Ctx, mut cc: ConsoleCommand) {
        // Deal with any pauses
        if cc.time > 0 {
            self.timesync = g.ecl.get_accurate_time() + cc.time;
            cc.time = 0;
            self.queue.insert(0, cc);
            return;
        }

        match cc.type_ {
            CMDTYPE_TEXT => self.run_command_text(g, cc.data1.as_deref().unwrap_or("")),
            CMDTYPE_DIR => self.run_command_dir(g),
            CMDTYPE_HELP => self.run_command_help(),
            CMDTYPE_EXIT => self.run_command_exit(g),
            CMDTYPE_DELETEALL => self.run_command_deleteall(g, cc.data1.as_deref()),
            CMDTYPE_RUN => self.run_command_run(g, cc.data1.as_deref().unwrap_or(""), false),
            CMDTYPE_RUNPROGRAM => self.run_command_run(g, cc.data1.as_deref().unwrap_or(""), true),
            CMDTYPE_CD => self.set_current_dir(g, cc.data1.as_deref().unwrap_or("")),
            CMDTYPE_SHUTDOWN => self.run_command_shutdown(g),
            CMDTYPE_DISCONNECT => self.run_command_disconnect(g),
            _ => panic!("Unrecognised ConsoleCommand TYPE (UplinkAbort)"),
        }
    }

    /// `RunCommand_TEXT`
    pub fn run_command_text(&mut self, g: &mut Ctx, text: &str) {
        // Scroll every line up one place
        for i in 0..NUMLINES - 1 {
            let name1 = format!("console_text {i}");
            let name2 = format!("console_text {}", i + 1);

            let caption2 = g
                .ecl
                .get_button(&name2)
                .expect("console_text missing (UplinkAssert)")
                .caption
                .clone();

            g.ecl
                .get_button_mut(&name1)
                .expect("console_text missing (UplinkAssert)")
                .set_caption(&caption2);

            g.ecl.dirty_button(&name1);
        }

        // Set the message on the last button and remove the message from the queue

        //int time = strlen(text) * 15;                     TOO SLOW
        let time = 200;

        let name1 = format!("console_text {}", NUMLINES - 1);
        g.ecl.register_caption_change(&name1, text, time, Some(Self::waiting_callback));
        self.waiting = true;
    }

    /// `RunCommand_DIR`
    pub fn run_command_dir(&mut self, g: &mut Ctx) {
        if self.currentdir == "/" {
            self.put_text_at_start(0, "pub               directory");
            self.put_text_at_start(0, "usr               directory");
            self.put_text_at_start(0, "log               directory");
            self.put_text_at_start(0, "rec               directory");
            self.put_text_at_start(0, "sys               directory");
        } else if self.currentdir == "sys" {
            if self.deleted_sys {
                self.put_text_at_start(0, "No files found");
            } else {
                self.put_text_at_start(0, "   boot.sys");
                self.put_text_at_start(0, "   os.sys");
                self.put_text_at_start(0, "   kernel.sys");
                self.put_text_at_start(0, "Direcory listing of SYS:");
            }
        } else if self.currentdir == "usr" {
            let name = self.screen_computer_name(g);
            let comp = computer(g, &name);

            if comp.databank.get_data_size() == 0 {
                self.put_text_at_start(0, "No files found");
            } else {
                for i in 0..comp.databank.get_data_size() {
                    if let Some(data) = comp.databank.get_data_file(i) {
                        let filesummary = format!("{}    {}Gq", data.title, data.size);
                        self.put_text_at_start(0, &filesummary);
                    }
                }
            }

            self.put_text_at_start(0, "Directory listing for USR:");
        } else if self.currentdir == "log" {
            let name = self.screen_computer_name(g);
            let comp = computer(g, &name);

            if comp.logbank.logs.num_used() == 0 {
                self.put_text_at_start(0, "No files found");
            } else {
                for i in 0..comp.logbank.logs.size() {
                    if comp.logbank.logs.valid_index(i) {
                        let al = comp
                            .logbank
                            .logs
                            .get_data(i)
                            .expect("ConsoleScreen: log slot empty (UplinkAssert)");
                        let description = al.get_description();
                        self.put_text_at_start(0, &description);
                    }
                }
            }

            self.put_text_at_start(0, "Directory listing for LOG:");
        } else if self.currentdir == "pub" {
            // The C++ listing of the computer screens (subtitles) is commented out there.
            self.put_text_at_start(0, "Not accessable from console.");
            self.put_text_at_start(0, "Directory listing for PUB:");
        } else if self.currentdir == "rec" {
            self.put_text_at_start(0, "Not accessable from console.");
            self.put_text_at_start(0, "Directory listing for REC:");
        } else {
            self.put_text_at_start(0, "Invalid directory");
            self.set_current_dir(g, "/");
        }
    }

    /// `RunCommand_HELP`
    pub fn run_command_help(&mut self) {
        self.put_text_at_start(0, "exit            -    return to the menu");
        self.put_text_at_start(0, "disconnect      -    disconnect from this system");
        self.put_text_at_start(0, "shutdown        -    shut down and restart the system");
        self.put_text_at_start(0, "delete          -    delete all files in the current directory");
        self.put_text_at_start(0, "run program\t -\t  run a program on the system");
        self.put_text_at_start(0, "cd dir          -    change directory into 'dir'");
        self.put_text_at_start(0, "dir             -    list all files in current directory");
        self.put_text_at_start(0, "help cmd        -    help on a specific command");
        self.put_text_at_start(0, "Available commands : ");
        self.put_text_at_start(0, "Running UNIX Kernel");
    }

    /// `RunCommand_EXIT`
    pub fn run_command_exit(&mut self, g: &mut Ctx) {
        let nextpage = self
            .get_computer_screen(g)
            .expect("ConsoleScreen: no computer screen (UplinkAssert)")
            .nextpage;
        RemoteInterface::run_screen(g, nextpage);
    }

    /// `RunCommand_DELETEALL`: `dir` is Some for the actual delete, None to queue the steps.
    pub fn run_command_deleteall(&mut self, g: &mut Ctx, dir: Option<&str>) {
        let name = self.screen_computer_name(g);

        if let Some(dir) = dir {
            // This version of the DELETEALL command actually deletes the entire directory,
            // rather than just printing text messages.

            if dir == "sys" {
                self.deleted_sys = true;
            } else if dir == "log" {
                let mut logdate = Date::default();
                logdate.set_date_from(&g.world().date);

                let comp = computer_mut(g, &name);
                for i in 0..comp.logbank.logs.size() {
                    // Delete the log
                    comp.logbank.logs.remove_data(i);

                    // Replace it with a "Deleted" marker
                    let mut al = AccessLog::new();
                    al.set_properties(&logdate, "Unknown", " ", LOG_NOTSUSPICIOUS, LOG_TYPE_DELETED);
                    comp.logbank.logs.put_data_at(al, i);
                }
            } else if dir == "usr" {
                computer_mut(g, &name).databank.format();
            }
        } else {
            // This version of the DELETEALL command doesn't delete anything itself, but it sets up
            // the commands that will eventually do the delete.

            if computer(g, &name).security.is_running_proxy() {
                self.push_front(CMDTYPE_TEXT, Some("Denied access by Proxy Server"), 0);
                return;
            }

            if self.currentdir == "sys" {
                self.push_front(CMDTYPE_DELETEALL, Some("sys"), 5000);
                self.push_front(CMDTYPE_TEXT, Some("Deleting os.sys..."), 5000);
                self.push_front(CMDTYPE_TEXT, Some("Deleting kernel.sys..."), 5000);
                self.push_front(CMDTYPE_TEXT, Some("Deleting boot.sys..."), 0);
            } else if self.currentdir == "log" {
                self.push_front(CMDTYPE_DELETEALL, Some("log"), 0);
                self.push_front(CMDTYPE_TEXT, Some("All files deleted."), 100);

                let comp = computer(g, &name);
                for i in 0..comp.logbank.logs.size() {
                    if comp.logbank.logs.valid_index(i) {
                        if let Some(al) = comp.logbank.logs.get_data(i) {
                            let caption = format!("Deleting log {}...", al.get_description());
                            self.push_front(CMDTYPE_TEXT, Some(&caption), TIMEREQUIRED_DELETEONELOG);
                        }
                    }
                }

                self.push_front(CMDTYPE_TEXT, Some("Deleting all files in LOG..."), 0);
            } else if self.currentdir == "usr" {
                if computer(g, &name).security.is_running_firewall() {
                    self.push_front(CMDTYPE_TEXT, Some("Denied access by Firewall"), 0);
                    self.push_front(CMDTYPE_TEXT, Some("Deleting all files in USR..."), 0);
                    return;
                }

                self.push_front(CMDTYPE_DELETEALL, Some("usr"), 0);
                self.push_front(CMDTYPE_TEXT, Some("All files deleted."), 100);

                let comp = computer(g, &name);
                for i in 0..comp.databank.num_data_files() {
                    if let Some(file) = comp.databank.get_data_file(i) {
                        let caption = format!("Deleting {}...", file.title);
                        let time = TIMEREQUIRED_DELETEONEGIGAQUAD * file.size;
                        self.push_front(CMDTYPE_TEXT, Some(&caption), time);
                    }
                }

                self.push_front(CMDTYPE_TEXT, Some("Deleting all files in USR..."), 0);
            } else if self.currentdir == "pub" {
                self.push_front(CMDTYPE_TEXT, Some("Not accessable from console."), 0);
                self.push_front(CMDTYPE_TEXT, Some("Deleting all files in PUB..."), 0);
            } else if self.currentdir == "rec" {
                self.push_front(CMDTYPE_TEXT, Some("Not accessable from console."), 0);
                self.push_front(CMDTYPE_TEXT, Some("Deleting all files in REC..."), 0);
            } else {
                self.put_text_at_start(0, "Invalid directory");
                self.set_current_dir(g, "/");
            }
        }
    }

    /// `RunCommand_RUN`: `actuallyrun` true runs the program, false checks it is runnable.
    pub fn run_command_run(&mut self, g: &mut Ctx, program: &str, actuallyrun: bool) {
        let name = self.screen_computer_name(g);

        if actuallyrun {
            if let Some(data) = find_program_file(computer(g, &name), program) {
                let ip = computer(g, &name).ip.clone();

                // Only a small number of programs are currently runnable
                if program == "revelation" {
                    PlotGenerator::run_revelation(g, &ip, data.version, true);
                } else if program == "faith" {
                    PlotGenerator::run_faith(g, &ip, data.version, true);

                    if computer(g, &name).isinfected_revelation > 0.0 {
                        self.push_front(CMDTYPE_TEXT, Some("Revelation is still running"), 0);
                        self.push_front(CMDTYPE_TEXT, Some("Failed"), 0);
                    } else {
                        self.push_front(CMDTYPE_TEXT, Some("Revelation has been purged"), 0);
                        self.push_front(CMDTYPE_TEXT, Some("Success"), 0);
                    }
                } else if program == "revelationtracer" {
                    g.world_mut().plotgenerator.run_revelation_tracer(&ip);

                    self.push_front(CMDTYPE_TEXT, Some("Listening for Revelation activity on port 666..."), 0);
                    self.push_front(CMDTYPE_TEXT, Some("RevelationTracer is now INVISIBLE to other users"), 0);
                    self.push_front(CMDTYPE_TEXT, Some("Done."), 0);
                    self.push_front(CMDTYPE_TEXT, Some("Running StEaLtH routines..."), 1000);

                    // Remove file
                    let comp = computer_mut(g, &name);
                    for m in 0..comp.databank.num_data_files() {
                        let is_tracer = comp
                            .databank
                            .get_data_file(m)
                            .is_some_and(|d| d.title == "RevelationTracer");
                        if is_tracer {
                            comp.databank.remove_data_file(m);
                            break;
                        }
                    }
                }
            }
        } else if self.currentdir == "usr" {
            // Can only run user programs
            match find_program_file(computer(g, &name), program) {
                Some(data) => {
                    if data.type_ == DATATYPE_PROGRAM {
                        let text = format!("Starting program {}...", data.title);

                        self.push_front(CMDTYPE_RUNPROGRAM, Some(program), 1000);
                        self.push_front(CMDTYPE_TEXT, Some("Started."), 0);
                        self.push_front(CMDTYPE_TEXT, Some(&text), 0);
                    } else {
                        self.push_front(CMDTYPE_TEXT, Some("This data file is not executable."), 0);
                    }
                }
                // Program not found
                None => self.push_front(CMDTYPE_TEXT, Some("Program not found."), 0),
            }
        } else {
            self.push_front(CMDTYPE_TEXT, Some("Only programs in the usr dir may be run remotely."), 0);
        }
    }

    /// `RunCommand_SHUTDOWN`
    pub fn run_command_shutdown(&mut self, g: &mut Ctx) {
        let name = self.screen_computer_name(g);

        if computer(g, &name).security.is_running_proxy() {
            ogl::create_msgbox(g, "Error", "Denied access by Proxy Server");
            return;
        }

        if self.deleted_sys {
            // Anything but the LAN computer (C++ GetOBJECTID() != OID_LANCOMPUTER) goes down
            let comp = computer_mut(g, &name);
            if comp.lan().is_none() {
                comp.set_is_running(false);
            }

            self.push_front(CMDTYPE_DISCONNECT, None, 3000);
            self.push_front(CMDTYPE_TEXT, Some("System failure - disconnecting remote users..."), 2000);
            self.push_front(CMDTYPE_TEXT, Some("[Failed]"), 5000);
            self.push_front(CMDTYPE_TEXT, Some("Loading Kernel..."), 4000);
            self.push_front(CMDTYPE_TEXT, Some("Rebooting..."), 2000);
            self.push_front(CMDTYPE_TEXT, Some("System shut down complete."), 500);
            self.push_front(CMDTYPE_TEXT, Some("Shutting down system..."), 500);
        } else {
            self.push_front(CMDTYPE_TEXT, Some("Reboot completed."), 2000);
            self.push_front(CMDTYPE_TEXT, Some("Running Console Services..."), 2000);
            self.push_front(CMDTYPE_TEXT, Some("Starting User Services..."), 2000);
            self.push_front(CMDTYPE_TEXT, Some("Loading Operating System..."), 2000);
            self.push_front(CMDTYPE_TEXT, Some("Loading Kernel..."), 4000);
            self.push_front(CMDTYPE_TEXT, Some("Rebooting..."), 2000);
            self.push_front(CMDTYPE_TEXT, Some("System shut down complete."), 500);
            self.push_front(CMDTYPE_TEXT, Some("Shutting down system..."), 500);
        }
    }

    /// `RunCommand_DISCONNECT`
    pub fn run_command_disconnect(&mut self, g: &mut Ctx) {
        // The player's connection lives on the Person keyed "PLAYER"
        Connection::disconnect(g, "PLAYER");
        Connection::reset(g, "PLAYER");

        RemoteInterface::run_new_location(g);
        RemoteInterface::run_screen(g, 2);
    }

    /// `GetComputerScreen` (typed accessor; the C++ casts `cs`).
    pub fn get_computer_screen<'a>(&self, g: &'a Ctx) -> Option<&'a GenericScreen> {
        let cs = self.cs.as_ref()?;
        g.world()
            .get_computer(&cs.computer)?
            .get_computer_screen(cs.index)?
            .as_generic_screen()
    }
}

impl InterfaceScreen for ConsoleScreenInterface {
    crate::impl_as_any!();

    /// `Create(ComputerScreen *newcs)` body (the `cs` is already set by `set_computer_screen`).
    fn create(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            g.ecl.register_button(20, 30, 405, 15, "Console", "", "console_title");

            g.ecl.register_button(20, 45, 405, NUMLINES * 15 + 10, "", "", "console_border");
            g.ecl.register_button_callbacks("console_border", Some(Self::border_draw), None, None, None);

            for i in 0..NUMLINES {
                let y = 50 + i * 15;
                let name = format!("console_text {i}");

                g.ecl.register_button(22, y, 390, 15, "", "", &name);
                g.ecl.register_button_callbacks(&name, Some(Self::message_draw), None, None, None);
            }

            self.set_current_dir(g, "/");

            let newcaption = format!("{}:>", self.currentdir);

            let ybottom = 50 + NUMLINES * 15 + 6;

            g.ecl.register_button(20, ybottom, 355, 15, &newcaption, "", "console_typehere");
            g.ecl.register_button_callbacks(
                "console_typehere",
                Some(Self::type_here_draw),
                None,
                Some(ogl::button_click),
                Some(ogl::button_highlight),
            );

            g.ecl.register_button(375, ybottom, 50, 15, "Post", "Click here to enter your command", "console_post");
            g.ecl.register_button_callback("console_post", Self::post_click);

            g.ecl.make_button_editable("console_typehere");

            // Set up the console

            self.add_user("System");
            let security_name = g.interface().ri.security_name.clone();
            self.add_user(&security_name);

            // Log this access

            // C++ passes GetGhost() straight into the log, so a missing ghost is a crash there too.
            let ghost = g
                .player()
                .person
                .connection
                .get_ghost()
                .expect("ConsoleScreen: player has no ghost location (C++ strcpy of NULL)")
                .to_string();

            let mut log = AccessLog::new();
            log.set_properties(&g.world().date, &ghost, "PLAYER", LOG_NOTSUSPICIOUS, LOG_TYPE_TEXT);
            log.set_data1(Some("Accessed console"));

            let remotehost = g.player().person.remotehost.clone();
            let computer_name = g
                .world()
                .get_vlocation(&remotehost)
                .expect("ConsoleScreen: remote host missing (UplinkAssert)")
                .computer
                .clone();
            computer_mut(g, &computer_name).logbank.add_log(log, -1);
        }
    }

    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            g.ecl.remove_button("console_title");
            g.ecl.remove_button("console_border");

            for ti in 0..NUMLINES {
                let name = format!("console_text {ti}");
                g.ecl.remove_button(&name);
            }

            g.ecl.remove_button("console_typehere");
            g.ecl.remove_button("console_post");
        }
    }

    fn update(&mut self, g: &mut Ctx) {
        if !self.waiting && self.timesync == -1 && !self.queue.is_empty() {
            // Execute next command
            let cc = self.pop_front_command();
            self.run_command_cc(g, cc);
        } else if !self.waiting && self.timesync > -1 && g.ecl.get_accurate_time() > self.timesync {
            self.timesync = -1;

            // Execute next command
            let cc = self.pop_front_command();
            self.run_command_cc(g, cc);
        }
    }

    fn is_visible(&self, g: &Ctx) -> bool {
        Self::is_visible_interface(g)
    }

    fn return_key_pressed(&mut self, g: &mut Ctx) -> bool {
        // PostClick(NULL)
        self.post_click_body(g);
        true
    }

    fn screen_id(&self) -> i32 {
        SCREEN_CONSOLESCREEN
    }

    fn set_computer_screen(&mut self, cs: ScreenRef) {
        self.cs = Some(cs);
    }

    fn computer_screen_ref(&self) -> Option<&ScreenRef> {
        self.cs.as_ref()
    }
}
