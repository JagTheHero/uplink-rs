//! Port of source/interface/remoteinterface/highsecurityscreen_interface.h + source/interface/remoteinterface/highsecurityscreen_interface.cpp

use uplink_engine::{gl, text};

use crate::app::globals::cheatmodes_enabled;
use crate::app::opengl_interface::{
    button_assignbitmap, button_assignbitmaps, button_click, button_highlight, clear_draw,
    textbutton_draw,
};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::remoteinterface::remoteinterface::{RemoteInterface, SCREEN_HIGHSECURITYSCREEN};
use crate::interface::remoteinterface::remoteinterfacescreen::{draw_main_title, draw_sub_title};
use crate::objref::ScreenRef;
use crate::world::computer::computerscreen::highsecurityscreen::HighSecurityScreen;
use crate::world::computer::computerscreen::menuscreen::MenuScreenOption;
use crate::world::connection::Connection;

/// High security screen: a menu of login systems that must each be bypassed before access is
/// granted. Security level 10 means the system is still locked.
#[derive(Debug, Default)]
pub struct HighSecurityScreenInterface {
    pub cs: Option<ScreenRef>,
}

/// The system index from a button name "prefix N" (`sscanf(name, "%s %d", unused, &index)`).
/// A missing or bad number leaves the C++ variable uninitialised; -1 (no such system) is used here.
fn system_index(name: &str) -> i32 {
    name.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(-1)
}

/// `hs->systems.GetData(index)` on the current high security screen (a copy of the option).
fn system_option(g: &Ctx, index: i32) -> Option<MenuScreenOption> {
    let hs = RemoteInterface::get_computer_screen(g)?.as_high_security_screen()?;
    usize::try_from(index).ok().and_then(|i| hs.systems.get(i)).cloned()
}

impl HighSecurityScreenInterface {
    /// `SystemTitleDraw` (static DrawFn): row background and caption, coloured by lock state.
    /// The system index is parsed from the button name ("highsecurity_systemtitle N").
    pub fn system_title_draw(g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        let index = system_index(&button.name);

        let Some(mso) = system_option(g, index) else { return };

        clear_draw(g, button.x + 1, button.y, button.width, button.height);

        // Draw 2 background lines

        if mso.security == 10 {
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

        if mso.security == 10 {
            gl::color3f(1.0, 1.0, 1.0);
        } else {
            gl::color3f(0.5, 0.5, 0.5);
        }

        text::draw_text_styled(button.x + 10, button.y + 18, &button.caption, text::HELVETICA_18);
    }

    /// `SystemClick` (static MouseFn): runs the chosen system's bypass page.
    pub fn system_click(g: &mut Ctx, button: &Button) {
        let index = system_index(&button.name);

        let Some(mso) = system_option(g, index) else { return };

        let nextpage = mso.nextpage;
        if nextpage != -1 {
            RemoteInterface::run_screen(g, nextpage);
        }
    }

    /// `BypassClick` (static MouseFn, cheat mode only): grants admin access and runs the next screen.
    pub fn bypass_click(g: &mut Ctx, _button: &Button) {
        // Give admin access

        g.interface_mut().ri.set_security("Admin", 1);

        // Run the next screen

        let Some(nextpage) = RemoteInterface::get_computer_screen(g)
            .and_then(|s| s.as_high_security_screen())
            .map(|hs| hs.nextpage)
        else {
            return;
        };

        if nextpage != -1 {
            RemoteInterface::run_screen(g, nextpage);
        }
    }

    /// `ProceedClick` (static MouseFn): once every system is bypassed, grants valid access at the
    /// worst level seen and runs the next screen.
    pub fn proceed_click(g: &mut Ctx, _button: &Button) {
        let Some(hs) = RemoteInterface::get_computer_screen(g)
            .and_then(|s| s.as_high_security_screen())
            .cloned()
        else {
            return;
        };

        let mut worstlevel = 1;

        for mso in &hs.systems {
            if mso.security > worstlevel {
                worstlevel = mso.security;
            }
        }

        if worstlevel < 10 && hs.nextpage != -1 {
            // The screen this interface shows; captured before RunScreen replaces the interface,
            // so the same screen can be reset afterwards (C++ `hs->ResetSecurity()`).
            let screen_ref = g
                .interface()
                .ri
                .screen
                .get::<HighSecurityScreenInterface>()
                .and_then(|s| s.cs.clone());

            g.interface_mut().ri.set_security("Valid", worstlevel);
            RemoteInterface::run_screen(g, hs.nextpage);

            if let Some(r) = screen_ref {
                if let Some(screen) = g
                    .world_mut()
                    .get_computer_mut(&r.computer)
                    .and_then(|c| c.get_computer_screen_mut(r.index))
                    .and_then(|s| s.as_high_security_screen_mut())
                {
                    screen.reset_security();
                }
            }

            g.player_mut().score_highsecurityhacks += 1;
        }
    }

    /// `GetComputerScreen` (typed accessor; the C++ casts `cs`).
    pub fn get_computer_screen<'a>(&self, g: &'a Ctx) -> Option<&'a HighSecurityScreen> {
        let r = self.cs.as_ref()?;
        g.world().get_computer(&r.computer)?.get_computer_screen(r.index)?.as_high_security_screen()
    }

    /// Sets the security of one system on the world's copy of this screen (`mso->security = ..`).
    fn set_system_security(&self, g: &mut Ctx, index: usize, security: i32) {
        let Some(r) = self.cs.clone() else { return };
        if let Some(screen) = g
            .world_mut()
            .get_computer_mut(&r.computer)
            .and_then(|c| c.get_computer_screen_mut(r.index))
            .and_then(|s| s.as_high_security_screen_mut())
        {
            if let Some(mso) = screen.systems.get_mut(index) {
                mso.security = security;
            }
        }
    }
}

impl InterfaceScreen for HighSecurityScreenInterface {
    crate::impl_as_any!();

    /// `Create(ComputerScreen *newcs)` body (the `cs` is already set by `set_computer_screen`).
    /// Also records bypass results from the remote interface and starts a trace if a monitor is running.
    fn create(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            return;
        }

        // A snapshot of the screen: the loop registers buttons on `g` while it runs.
        let Some(hs) = self.get_computer_screen(g).cloned() else { return };

        g.ecl.register_button(80, 60, 350, 25, &hs.base.maintitle, "", "highsecurity_maintitle");
        g.ecl.register_button_callbacks("highsecurity_maintitle", Some(draw_main_title), None, None, None);
        g.ecl.register_button(80, 80, 350, 20, &hs.base.subtitle, "", "highsecurity_subtitle");
        g.ecl.register_button_callbacks("highsecurity_subtitle", Some(draw_sub_title), None, None, None);

        for (i, mso) in hs.systems.iter().enumerate() {
            let mut security = mso.security;

            // Have we just finished bypassing this screen?
            // If so, store the result of the bypass
            // ie the security level gained

            let (prev, level) = {
                let ri = &g.interface().ri;
                (ri.previousscreenindex, ri.security_level)
            };
            if prev == mso.nextpage && level < 10 {
                security = level;
                self.set_system_security(g, i, security);
            }

            // Start a trace going if one of the systems is disabled

            if security != 10 {
                let monitor_running = g
                    .world()
                    .get_computer(&hs.base.computer)
                    .is_some_and(|comp| comp.security.is_running_monitor());

                if monitor_running {
                    Connection::begin_trace(g, "PLAYER");
                }
            }

            let title = format!("highsecurity_systemtitle {}", i);
            let status = format!("highsecurity_systemstatus {}", i);

            let y = 140 + (i as i32) * 40;

            g.ecl.register_button(30, y, 300, 25, &mso.caption, "Click here to access this system", &title);
            g.ecl.register_button_callbacks(
                &title,
                Some(Self::system_title_draw),
                Some(Self::system_click),
                Some(button_click),
                Some(button_highlight),
            );

            g.ecl.register_button(335, y, 100, 20, "", "Click here to access this system", &status);

            if security == 10 {
                button_assignbitmap(g, &status, "accessdenied.tif");
            } else {
                button_assignbitmap(g, &status, "accessgranted.tif");
            }

            g.ecl.register_button_callback(&status, Self::system_click);

            g.ecl.register_button(335, 350, 80, 15, " ", "Click here when done", "highsecurity_proceed");
            button_assignbitmaps(g, "highsecurity_proceed", "proceed.tif", "proceed_h.tif", "proceed_c.tif");
            g.ecl.register_button_callback("highsecurity_proceed", Self::proceed_click);

            if cheatmodes_enabled() {
                // Create a symbol for quick entry into this screen
                g.ecl.register_button(
                    3,
                    20,
                    30,
                    15,
                    "#",
                    "Click here to bypass this screen (DEBUG MODE ONLY)",
                    "highsecurity_bypass",
                );
                g.ecl.register_button_callbacks(
                    "highsecurity_bypass",
                    Some(textbutton_draw),
                    Some(Self::bypass_click),
                    Some(button_click),
                    Some(button_highlight),
                );
            }
        }
    }

    fn remove(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            return;
        }

        g.ecl.remove_button("highsecurity_maintitle");
        g.ecl.remove_button("highsecurity_subtitle");

        let count = self.get_computer_screen(g).map_or(0, |hs| hs.systems.len());

        for i in 0..count {
            let title = format!("highsecurity_systemtitle {}", i);
            let button = format!("highsecurity_systembutton {}", i);
            let status = format!("highsecurity_systemstatus {}", i);

            g.ecl.remove_button(&title);
            g.ecl.remove_button(&button);
            g.ecl.remove_button(&status);
        }

        g.ecl.remove_button("highsecurity_proceed");
        g.ecl.remove_button("highsecurity_bypass");
    }

    fn update(&mut self, _g: &mut Ctx) {}

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("highsecurity_maintitle").is_some()
    }

    /// `ReturnKeyPressed`: this screen does not handle the return key.
    fn return_key_pressed(&mut self, _g: &mut Ctx) -> bool {
        false
    }

    fn screen_id(&self) -> i32 {
        SCREEN_HIGHSECURITYSCREEN
    }

    fn set_computer_screen(&mut self, cs: ScreenRef) {
        self.cs = Some(cs);
    }

    fn computer_screen_ref(&self) -> Option<&ScreenRef> {
        self.cs.as_ref()
    }
}
