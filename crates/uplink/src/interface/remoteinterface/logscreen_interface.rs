//! Port of source/interface/remoteinterface/logscreen_interface.h + source/interface/remoteinterface/logscreen_interface.cpp

use uplink_engine::{gl, text};

use crate::app::opengl_interface;
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::remoteinterface::remoteinterface::{RemoteInterface, SCREEN_LOGSCREEN};
use crate::interface::remoteinterface::remoteinterfacescreen::{draw_main_title, draw_sub_title};
use crate::interface::taskmanager::taskmanager::TaskManager;
use crate::objref::ScreenRef;
use crate::world::computer::computerscreen::computerscreen::ComputerScreen;
use crate::world::computer::computerscreen::logscreen::LogScreen;

/// Remote log screen: lists access or account logs of the computer.
#[derive(Debug, Default)]
pub struct LogScreenInterface {
    pub cs: Option<ScreenRef>,
    pub baseoffset: i32,
    pub previousnumlogs: i32,
}

/// `sscanf(button->name, "logscreen_log %d", &logindex)`: the row number at the end of the name.
fn row_number(name: &str) -> i32 {
    name.rsplit(' ').next().and_then(|s| s.parse().ok()).unwrap_or(0)
}

/// The `LogScreen` a `ScreenRef` points at, looked up in the world.
fn log_screen_of<'a>(g: &'a Ctx, cs: &ScreenRef) -> Option<&'a LogScreen> {
    match g.world().get_computer(&cs.computer)?.get_computer_screen(cs.index)? {
        ComputerScreen::LogScreen(ls) => Some(ls),
        _ => None,
    }
}

/// `for ( int i = 0; i < 15; ++i ) EclDirtyButton ( "logscreen_log %d" )`
fn dirty_log_rows(g: &mut Ctx) {
    for i in 0..15 {
        let name = format!("logscreen_log {}", i);
        g.ecl.dirty_button(&name);
    }
}

/// Body shared by `CloseClick` and `EscapeKeyPressed`: follows the screen's `nextpage`.
fn close_to_next_page(g: &mut Ctx, nextpage: i32) {
    if nextpage != -1 {
        RemoteInterface::run_screen(g, nextpage);
    }
}

impl LogScreenInterface {
    /// `GetComputerScreen` (typed accessor; the C++ casts `cs`).
    pub fn get_computer_screen<'a>(&self, g: &'a Ctx) -> Option<&'a LogScreen> {
        log_screen_of(g, self.cs.as_ref()?)
    }

    /// `LogDraw` (static DrawFn)
    pub fn log_draw(g: &mut Ctx, button: &Button, highlighted: bool, _clicked: bool) {
        let screenheight = g.app.options.get_option_value("graphics_screenheight");
        gl::scissor(button.x, screenheight - (button.y + button.height), button.width, button.height);
        gl::enable(gl::SCISSOR_TEST);

        let logindex = {
            let baseoffset = g.interface().ri.screen.get::<Self>().map(|s| s.baseoffset).unwrap_or(0);
            baseoffset - row_number(&button.name)
        };

        // Copy the row's text out so the world borrow ends before drawing.
        let row = {
            let cs = g.interface().ri.screen.get::<Self>().and_then(|s| s.cs.clone());
            cs.as_ref().and_then(|cs| log_screen_of(g, cs)).and_then(|ls| {
                let logbank = ls.get_target_log_bank(g);
                if logbank.logs.valid_index(logindex) {
                    let log = logbank.logs.get_data(logindex)?;
                    Some((log.date.get_short_string(), log.get_description()))
                } else {
                    None
                }
            })
        };

        match row {
            Some((date, description)) => {
                if logindex % 2 == 0 {
                    gl::begin(gl::QUADS);
                    gl::color3ub(8, 20, 80);
                    gl::vertex2i(button.x, button.y);
                    gl::color3ub(8, 20, 0);
                    gl::vertex2i(button.x + button.width, button.y);
                    gl::color3ub(8, 20, 80);
                    gl::vertex2i(button.x + button.width, button.y + button.height);
                    gl::color3ub(8, 20, 0);
                    gl::vertex2i(button.x, button.y + button.height);
                    gl::end();
                } else {
                    gl::begin(gl::QUADS);
                    gl::color3ub(8, 20, 0);
                    gl::vertex2i(button.x, button.y);
                    gl::color3ub(8, 20, 80);
                    gl::vertex2i(button.x + button.width, button.y);
                    gl::color3ub(8, 20, 0);
                    gl::vertex2i(button.x + button.width, button.y + button.height);
                    gl::color3ub(8, 20, 80);
                    gl::vertex2i(button.x, button.y + button.height);
                    gl::end();
                }

                gl::color4f(1.0, 1.0, 1.0, 1.0);
                text::draw_text_default(button.x + 10, button.y + 10, &date);
                text::draw_text_default(button.x + 100, button.y + 10, &description);

                // Draw a bounding box
                if highlighted {
                    gl::color4f(1.0, 1.0, 1.0, 1.0);
                    opengl_interface::border_draw(button);
                }
            }
            None => {
                opengl_interface::clear_draw(g, button.x, button.y, button.width, button.height);
            }
        }

        gl::disable(gl::SCISSOR_TEST);
    }

    /// `ScrollUpClick` (static MouseFn)
    pub fn scroll_up_click(g: &mut Ctx, _button: &Button) {
        let cs = {
            let Some(me) = g.interface_mut().ri.screen.get_mut::<Self>() else { return };
            me.baseoffset += 1;
            me.cs.clone()
        };
        let Some(cs) = cs else { return };
        let Some(numlogs) = log_screen_of(g, &cs).map(|ls| ls.get_target_log_bank(g).logs.size()) else {
            return;
        };
        if let Some(me) = g.interface_mut().ri.screen.get_mut::<Self>() {
            if me.baseoffset >= numlogs {
                me.baseoffset = numlogs - 1;
            }
        }
        dirty_log_rows(g);
    }

    /// `ScrollDownClick` (static MouseFn)
    pub fn scroll_down_click(g: &mut Ctx, _button: &Button) {
        if let Some(me) = g.interface_mut().ri.screen.get_mut::<Self>() {
            me.baseoffset -= 1;
            if me.baseoffset < 0 {
                me.baseoffset = 0;
            }
        }
        dirty_log_rows(g);
    }

    /// `LogClick` (static MouseFn)
    pub fn log_click(g: &mut Ctx, button: &Button) {
        let Some((base, cs)) = g.interface().ri.screen.get::<Self>().map(|s| (s.baseoffset, s.cs.clone())) else {
            return;
        };
        let logindex = base - row_number(&button.name);

        let Some(cs) = cs else { return };
        let Some(proxy_running) = g.world().get_computer(&cs.computer).map(|c| c.security.is_running_proxy()) else {
            return;
        };
        if proxy_running {
            opengl_interface::create_msgbox(g, "Error", "Denied access by Proxy Server");
            return;
        }

        let Some((logbank_valid, logref)) = log_screen_of(g, &cs)
            .map(|ls| (ls.get_target_log_bank(g).logs.valid_index(logindex), ls.target_log_ref(g)))
        else {
            return;
        };
        if logbank_valid {
            TaskManager::set_program_target(g, logref, &button.name, logindex);
        }
    }

    /// `CloseClick` (static MouseFn)
    pub fn close_click(g: &mut Ctx, _button: &Button) {
        let nextpage = g
            .interface()
            .ri
            .screen
            .get::<Self>()
            .and_then(|me| me.get_computer_screen(g))
            .map(|ls| ls.nextpage);
        if let Some(nextpage) = nextpage {
            close_to_next_page(g, nextpage);
        }
    }
}

impl InterfaceScreen for LogScreenInterface {
    crate::impl_as_any!();

    fn create(&mut self, g: &mut Ctx) {
        // C++ `Create ()` (no argument) warns and does nothing when there is no screen.
        if self.cs.is_none() {
            println!("LogScreenInterface::Create, tried to create when LogScreen==NULL");
            return;
        }

        if self.is_visible(g) {
            return;
        }

        let Some((maintitle, subtitle, logbank_size)) = self
            .get_computer_screen(g)
            .map(|ls| (ls.base.maintitle.clone(), ls.base.subtitle.clone(), ls.get_target_log_bank(g).logs.size()))
        else {
            return;
        };

        g.ecl.register_button(80, 60, 350, 25, &maintitle, "", "logscreen_maintitle");
        g.ecl.register_button_callbacks("logscreen_maintitle", Some(draw_main_title), None, None, None);
        g.ecl.register_button(80, 80, 350, 20, &subtitle, "", "logscreen_subtitle");
        g.ecl.register_button_callbacks("logscreen_subtitle", Some(draw_sub_title), None, None, None);

        g.ecl.register_button(15, 120, 96, 15, "Date", "", "logscreen_datetitle");
        g.ecl.register_button(115, 120, 300, 15, "Action", "", "logscreen_actiontitle");

        // Create the log entries

        for i in 0..15 {
            let name = format!("logscreen_log {}", i);
            g.ecl.register_button(15, 140 + i * 15, 400, 14, "", "Select this log", &name);
            g.ecl.register_button_callbacks(
                &name,
                Some(LogScreenInterface::log_draw),
                Some(LogScreenInterface::log_click),
                Some(opengl_interface::button_click),
                Some(opengl_interface::button_highlight),
            );
        }

        if logbank_size >= 15 {
            // Create the scroll bar

            g.ecl.register_button(420, 140, 15, 15, "^", "Scroll up", "logscreen_scrollup");
            opengl_interface::button_assignbitmaps(g, "logscreen_scrollup", "up.tif", "up_h.tif", "up_c.tif");
            g.ecl.register_button_callback("logscreen_scrollup", LogScreenInterface::scroll_up_click);

            g.ecl.register_button(420, 157, 15, 13 * 15 - 4, "", "", "logscreen_scrollbar");

            g.ecl.register_button(420, 140 + 14 * 15, 15, 15, "v", "Scroll down", "logscreen_scrolldown");
            opengl_interface::button_assignbitmaps(g, "logscreen_scrolldown", "down.tif", "down_h.tif", "down_c.tif");
            g.ecl.register_button_callback("logscreen_scrolldown", LogScreenInterface::scroll_down_click);
        }

        // Create the close button

        g.ecl.register_button(421, 121, 13, 13, "", "Close the Log Screen", "logscreen_close");
        opengl_interface::button_assignbitmaps(g, "logscreen_close", "close.tif", "close_h.tif", "close_c.tif");
        g.ecl.register_button_callback("logscreen_close", LogScreenInterface::close_click);

        // Set the baseoffset to be the last entry in the logs (newest)

        self.baseoffset = logbank_size - 1;
    }

    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            g.ecl.remove_button("logscreen_maintitle");
            g.ecl.remove_button("logscreen_subtitle");

            g.ecl.remove_button("logscreen_datetitle");
            g.ecl.remove_button("logscreen_actiontitle");

            for i in 0..15 {
                let name = format!("logscreen_log {}", i);
                g.ecl.remove_button(&name);
            }

            g.ecl.remove_button("logscreen_scrollup");
            g.ecl.remove_button("logscreen_scrollbar");
            g.ecl.remove_button("logscreen_scrolldown");

            g.ecl.remove_button("logscreen_close");
        }
    }

    fn update(&mut self, g: &mut Ctx) {
        let Some(newnumlogs) = self.get_computer_screen(g).map(|ls| ls.get_target_log_bank(g).logs.num_used()) else {
            return;
        };

        if newnumlogs != self.previousnumlogs {
            dirty_log_rows(g);
            self.previousnumlogs = newnumlogs;
        }
    }

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("logscreen_maintitle").is_some()
    }

    fn screen_id(&self) -> i32 {
        SCREEN_LOGSCREEN
    }

    fn escape_key_pressed(&mut self, g: &mut Ctx) -> bool {
        // C++: CloseClick ( NULL ); return true;
        let nextpage = self.get_computer_screen(g).map(|ls| ls.nextpage).unwrap_or(-1);
        close_to_next_page(g, nextpage);
        true
    }

    fn set_computer_screen(&mut self, cs: ScreenRef) {
        self.cs = Some(cs);
    }

    fn computer_screen_ref(&self) -> Option<&ScreenRef> {
        self.cs.as_ref()
    }
}
