//! Port of source/interface/remoteinterface/securityscreen_interface.h + source/interface/remoteinterface/securityscreen_interface.cpp

use std::time::{SystemTime, UNIX_EPOCH};

use uplink_engine::{gl, text};

use crate::app::opengl_interface;
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::remoteinterface::remoteinterface::{RemoteInterface, SCREEN_SECURITYSCREEN};
use crate::interface::remoteinterface::remoteinterfacescreen::{draw_main_title, draw_sub_title};
use crate::objref::ScreenRef;
use crate::world::computer::computerscreen::computerscreen::ComputerScreen;
use crate::world::computer::computerscreen::genericscreen::GenericScreen;
use crate::world::connection::Connection;

/// Remote security screen: shows the computer's security status.
#[derive(Debug, Default)]
pub struct SecurityScreenInterface {
    pub cs: Option<ScreenRef>,
    /// Last update time; C++ stored `time(NULL)` in an `int`, kept as i64 here.
    pub lastupdate: i64,
}

/// `sscanf(button->name, "%s %d", unused, &index)`: the system number at the end of the name.
fn system_index(name: &str) -> i32 {
    name.rsplit(' ').next().and_then(|s| s.parse().ok()).unwrap_or(0)
}

/// Name of the computer the security screen is showing, from the remote interface's screen.
fn screen_computer(g: &Ctx) -> Option<String> {
    g.interface()
        .ri
        .screen
        .get::<SecurityScreenInterface>()
        .and_then(|me| me.cs.as_ref())
        .map(|cs| cs.computer.clone())
}

/// `CloseClick` body: follows the screen's `nextpage` when it has one.
fn run_next_page(g: &mut Ctx, nextpage: Option<i32>) {
    if let Some(nextpage) = nextpage {
        if nextpage != -1 {
            RemoteInterface::run_screen(g, nextpage);
        }
    }
}

/// Seconds since the epoch, as C `time(NULL)`.
fn unix_time() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

impl SecurityScreenInterface {
    /// `GetComputerScreen` (typed accessor; the C++ casts `cs`).
    pub fn get_computer_screen<'a>(&self, g: &'a Ctx) -> Option<&'a GenericScreen> {
        let cs = self.cs.as_ref()?;
        match g.world().get_computer(&cs.computer)?.get_computer_screen(cs.index)? {
            ComputerScreen::GenericScreen(gs) => Some(gs),
            _ => None,
        }
    }

    /// `SystemTitleDraw` (static DrawFn)
    pub fn system_title_draw(g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        opengl_interface::clear_draw(g, button.x + 1, button.y, button.width, button.height);

        // Get the security system in question

        let index = system_index(&button.name);

        let Some(enabled) = screen_computer(g)
            .and_then(|name| g.world().get_computer(&name))
            .and_then(|comp| comp.security.get_system(index))
            .map(|ss| ss.enabled)
        else {
            return;
        };

        // Draw 2 background lines

        if enabled {
            gl::color3f(0.2, 0.2, 0.7);
        } else {
            gl::color3f(0.0, 0.0, 0.3);
        }

        gl::line_width(2.0);

        gl::begin(gl::LINES);
        gl::vertex2i(button.x, button.y + 10);
        gl::vertex2i(button.x + button.width, button.y + 10);

        gl::vertex2i(button.x, button.y + 15);
        gl::vertex2i(button.x + button.width, button.y + 15);
        gl::end();

        gl::line_width(1.0);

        // Write the text

        if enabled {
            gl::color3f(1.0, 1.0, 1.0);
        } else {
            gl::color3f(0.5, 0.5, 0.5);
        }

        text::draw_text_styled(button.x + 10, button.y + 18, &button.caption, text::HELVETICA_18);
    }

    /// `StatusClick` (static MouseFn)
    pub fn status_click(g: &mut Ctx, button: &Button) {
        let index = system_index(&button.name);

        let Some(name) = screen_computer(g) else {
            return;
        };

        let Some(enabled) = g
            .world()
            .get_computer(&name)
            .and_then(|comp| comp.security.get_system(index))
            .map(|ss| ss.enabled)
        else {
            return;
        };

        let proxy_running = g.world().get_computer(&name).map(|comp| comp.security.is_running_proxy()).unwrap_or(false);
        if proxy_running {
            opengl_interface::create_msgbox(g, "Error", "Denied access by Proxy");
            return;
        }

        if let Some(ss) = g
            .world_mut()
            .get_computer_mut(&name)
            .and_then(|comp| comp.security.get_system_mut(index))
        {
            if enabled {
                ss.disable();
            } else {
                ss.enable();
            }
        }

        let monitor_running = g.world().get_computer(&name).map(|comp| comp.security.is_running_monitor()).unwrap_or(false);
        if monitor_running {
            Connection::begin_trace(g, "PLAYER");
        }

        if let Some(me) = g.interface_mut().ri.screen.get_mut::<Self>() {
            me.lastupdate = 0; // Force update now
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
            .map(|gs| gs.nextpage);
        run_next_page(g, nextpage);
    }
}

impl InterfaceScreen for SecurityScreenInterface {
    crate::impl_as_any!();

    fn create(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            return;
        }

        let Some((maintitle, subtitle)) = self
            .get_computer_screen(g)
            .map(|gs| (gs.base.maintitle.clone(), gs.base.subtitle.clone()))
        else {
            return;
        };
        let Some(computer_name) = self.cs.as_ref().map(|cs| cs.computer.clone()) else {
            return;
        };

        // Copy the system list out of the world before building the interface.

        let (num_systems, systems) = match g.world().get_computer(&computer_name) {
            Some(comp) => {
                let num_systems = comp.security.num_systems();
                let systems: Vec<(i32, String, i32)> = (0..num_systems)
                    .filter_map(|i| {
                        comp.security
                            .get_system(i)
                            .map(|ss| (i, ss.get_name().to_string(), ss.level))
                    })
                    .collect();
                (num_systems, systems)
            }
            None => return,
        };

        g.ecl.register_button(80, 60, 350, 25, &maintitle, "", "securityscreen_maintitle");
        g.ecl.register_button_callbacks("securityscreen_maintitle", Some(draw_main_title), None, None, None);
        g.ecl.register_button(80, 80, 350, 20, &subtitle, "", "securityscreen_subtitle");
        g.ecl.register_button_callbacks("securityscreen_subtitle", Some(draw_sub_title), None, None, None);

        if num_systems == 0 {
            g.ecl.register_button(200, 100, 200, 20, "No security systems installed", "", "securityscreen_none");
            g.ecl.register_button_callbacks(
                "securityscreen_none",
                Some(opengl_interface::textbutton_draw),
                None,
                None,
                None,
            );
        } else {
            for (i, name, level) in systems {
                let title = format!("securityscreen_systemtitle {}", i);
                let level_name = format!("securityscreen_systemlevel {}", i);
                let status = format!("securityscreen_systemstatus {}", i);

                let leveltext = format!("level {}", level);

                let y = 140 + i * 40;
                g.ecl.register_button(50, y, 150, 25, &name, "Click here to toggle its status", &title);
                g.ecl.register_button(200, y, 100, 25, &leveltext, "Click here to toggle its status", &level_name);
                g.ecl.register_button(300, y, 100, 25, "", "Click here to toggle its status", &status);

                g.ecl.register_button_callbacks(
                    &title,
                    Some(Self::system_title_draw),
                    Some(Self::status_click),
                    Some(opengl_interface::button_click),
                    Some(opengl_interface::button_highlight),
                );
                g.ecl.register_button_callbacks(
                    &level_name,
                    Some(Self::system_title_draw),
                    Some(Self::status_click),
                    Some(opengl_interface::button_click),
                    Some(opengl_interface::button_highlight),
                );
                g.ecl.register_button_callbacks(
                    &status,
                    Some(Self::system_title_draw),
                    Some(Self::status_click),
                    Some(opengl_interface::button_click),
                    Some(opengl_interface::button_highlight),
                );
            }
        }

        // Create the close button

        g.ecl.register_button(320, 370, 80, 20, "Close", "Close the security screen", "securityscreen_close");
        g.ecl.register_button_callback("securityscreen_close", Self::close_click);
    }

    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            g.ecl.remove_button("securityscreen_maintitle");
            g.ecl.remove_button("securityscreen_subtitle");
            g.ecl.remove_button("securityscreen_none");

            let num_systems = self
                .cs
                .as_ref()
                .and_then(|cs| g.world().get_computer(&cs.computer))
                .map(|comp| comp.security.num_systems())
                .unwrap_or(0);

            for i in 0..num_systems {
                let title = format!("securityscreen_systemtitle {}", i);
                let level = format!("securityscreen_systemlevel {}", i);
                let status = format!("securityscreen_systemstatus {}", i);

                g.ecl.remove_button(&title);
                g.ecl.remove_button(&level);
                g.ecl.remove_button(&status);
            }

            g.ecl.remove_button("securityscreen_close");
        }
    }

    fn update(&mut self, g: &mut Ctx) {
        let now = unix_time();

        if now > self.lastupdate + 5 {
            if self.is_visible(g) {
                let (num_systems, statuses) = match self
                    .cs
                    .as_ref()
                    .and_then(|cs| g.world().get_computer(&cs.computer))
                {
                    Some(comp) => {
                        let num_systems = comp.security.num_systems();
                        let statuses: Vec<(i32, bool)> = (0..num_systems)
                            .filter_map(|i| comp.security.get_system(i).map(|ss| (i, ss.enabled)))
                            .collect();
                        (num_systems, statuses)
                    }
                    None => (0, Vec::new()),
                };

                for (i, enabled) in statuses {
                    let status = format!("securityscreen_systemstatus {}", i);
                    if let Some(button) = g.ecl.get_button_mut(&status) {
                        button.set_caption(if enabled { "Enabled" } else { "Disabled" });
                    }
                }

                for i in 0..num_systems {
                    let title = format!("securityscreen_systemtitle {}", i);
                    let level = format!("securityscreen_systemlevel {}", i);
                    let status = format!("securityscreen_systemstatus {}", i);

                    g.ecl.dirty_button(&title);
                    g.ecl.dirty_button(&level);
                    g.ecl.dirty_button(&status);
                }
            }

            self.lastupdate = now;
        }
    }

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("securityscreen_maintitle").is_some()
    }

    fn screen_id(&self) -> i32 {
        SCREEN_SECURITYSCREEN
    }

    fn escape_key_pressed(&mut self, g: &mut Ctx) -> bool {
        // C++: CloseClick ( NULL ); return true;
        let nextpage = self.get_computer_screen(g).map(|gs| gs.nextpage);
        run_next_page(g, nextpage);
        true
    }

    fn set_computer_screen(&mut self, cs: ScreenRef) {
        self.cs = Some(cs);
    }

    fn computer_screen_ref(&self) -> Option<&ScreenRef> {
        self.cs.as_ref()
    }
}
