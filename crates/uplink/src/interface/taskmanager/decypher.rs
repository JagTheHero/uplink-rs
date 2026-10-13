//! Port of source/interface/taskmanager/decypher.h + source/interface/taskmanager/decypher.cpp

use uplink_engine::{gl, text};

use crate::app::globals::ALPHA;
use crate::app::opengl_interface as ogl;
use crate::ctx::Ctx;
use crate::eclipse::{Button, MOVE_STRAIGHTLINE};
use crate::game::data::data::{HACKDIFFICULTY_VARIANCE, TICKSREQUIRED_BYPASSCYPHER};
use crate::interface::remoteinterface::cypherscreen_interface::{
    CypherScreenInterface, CYPHER_HEIGHT, CYPHER_WIDTH,
};
use crate::interface::remoteinterface::remoteinterface::SCREEN_CYPHERSCREEN;
use crate::interface::taskmanager::uplinktask::{UplinkTask, UplinkTaskBase};
use crate::objref::ObjRef;
use crate::vanbakel::Vanbakel;
use crate::world::computer::computerscreen::computerscreen::ComputerScreen;
use crate::world::connection::Connection;
use crate::world::generator::numbergenerator::NumberGenerator;

pub const DECYPHER_OFF: i32 = 0;
pub const DECYPHER_INPROGRESS: i32 = 1;
pub const DECYPHER_FINISHED: i32 = 2;

#[derive(Debug)]
pub struct Decypher {
    pub base: UplinkTaskBase,
    pub status: i32,
    pub numticksrequired: i32,
    /// 0 .. numticksrequired
    pub progress: i32,
}

/// PID from a button name of the form `"<word> <pid>"` (C++ `sscanf(name, "%s %d", bname, &pid)`).
/// A name without a readable PID gives -1, which matches no task (C++ left `pid` uninitialised).
fn pid_of(name: &str) -> i32 {
    name.split_whitespace().nth(1).and_then(|t| t.parse().ok()).unwrap_or(-1)
}

/// Names of the four buttons the Decypher window is made of: title, border, progress, close.
fn button_names(pid: i32) -> [String; 4] {
    [
        format!("decypher_title {pid}"),
        format!("decypher_border {pid}"),
        format!("decypher_progress {pid}"),
        format!("decypher_close {pid}"),
    ]
}

/// The gradient background of `Decypher::BorderDraw`.
fn draw_border_background(button: &Button, highlighted: bool, clicked: bool) {
    // Corners top-left and bottom-right use `dark`, top-right and bottom-left use `light`.
    let (dark, light): ([f32; 3], [f32; 3]) = if clicked {
        ([0.5, 0.5, 0.6], [0.7, 0.7, 0.6])
    } else if highlighted {
        ([0.2, 0.2, 0.5], [0.5, 0.5, 0.6])
    } else {
        ([0.2, 0.2, 0.4], [0.3, 0.3, 0.5])
    };
    let (x, y, w, h) = (button.x, button.y, button.width, button.height);

    gl::begin(gl::QUADS);
    gl::color4f(dark[0], dark[1], dark[2], ALPHA);
    gl::vertex2i(x, y);
    gl::color4f(light[0], light[1], light[2], ALPHA);
    gl::vertex2i(x + w, y);
    gl::color4f(dark[0], dark[1], dark[2], ALPHA);
    gl::vertex2i(x + w, y + h);
    gl::color4f(light[0], light[1], light[2], ALPHA);
    gl::vertex2i(x, y + h);
    gl::end();
}

impl Decypher {
    pub fn new() -> Self {
        Decypher {
            base: UplinkTaskBase {
                version: 1.0,
                targetint: -1,
                ..Default::default()
            },
            status: DECYPHER_OFF,
            numticksrequired: 0,
            progress: 0,
        }
    }

    /// C++ `Decypher::BorderDraw` (static button callback).
    pub fn border_draw(_g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        draw_border_background(button, highlighted, clicked);

        if highlighted || clicked {
            gl::color4f(0.3, 0.3, 0.7, 1.0);
            ogl::border_draw(button);
        }
    }

    /// C++ `Decypher::ProgressDraw` (static button callback).
    pub fn progress_draw(_g: &mut Ctx, button: &Button, highlighted: bool, _clicked: bool) {
        let mut scale: f32 = button.width as f32 / 100.0;
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

    /// C++ `Decypher::CloseClick` (static button callback).
    pub fn close_click(g: &mut Ctx, button: &Button) {
        let pid = pid_of(&button.name);
        Vanbakel::remove_task(g, pid);
    }

    /// C++ `Decypher::BorderClick` (static button callback).
    pub fn border_click(g: &mut Ctx, button: &Button) {
        let pid = pid_of(&button.name);
        g.interface_mut().tm.set_target_program(pid);
    }
}

impl Default for Decypher {
    fn default() -> Self {
        Self::new()
    }
}

impl UplinkTask for Decypher {
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
        let sprogress = format!("decypher_progress {pid}");

        if self.status == DECYPHER_OFF {
            // Not decyphering - look for a new target
        } else if self.status == DECYPHER_INPROGRESS {
            // Make sure we are still looking at the same interface screen
            let screen_id = match g.interface_mut().ri.screen.get_dyn_mut() {
                Some(ris) => ris.screen_id(),
                None => {
                    debug_assert!(false, "Decypher::tick: no remote interface screen");
                    return;
                }
            };
            if screen_id != SCREEN_CYPHERSCREEN {
                Vanbakel::remove_task(g, pid);
                // WARNING - this has now been deleted
                return;
            }

            // progress += (n * version)  -- C++ int += float: done in float, truncated back to int
            self.progress = (self.progress as f32 + n as f32 * self.base.version) as i32;

            debug_assert!(g.ecl.get_button(&sprogress).is_some());
            if let Some(b) = g.ecl.get_button_mut(&sprogress) {
                b.width = (120.0 * (self.progress as f32 / self.numticksrequired as f32)) as i32;
            }
            g.ecl.dirty_button(&sprogress);

            // Borrow only the game's interface here, so the Eclipse borrow above is not held.
            let Some(csi) = g
                .game
                .ui
                .as_deref_mut()
                .and_then(|ui| ui.ri.screen.get_mut::<CypherScreenInterface>())
            else {
                debug_assert!(false, "Decypher::tick: screen is not a CypherScreenInterface");
                return;
            };

            if self.progress >= self.numticksrequired {
                // Finished
                while csi.num_unlocked() > 0 {
                    csi.cypher_lock_random();
                }

                self.status = DECYPHER_FINISHED;
            } else {
                // C++: int percentagedone = 100.0 * (float) progress / (float) numticksrequired;
                let percentagedone =
                    (100.0_f64 * (self.progress as f32 as f64) / (self.numticksrequired as f32 as f64)) as i32;

                // C++: 100.0 * (float) NumLocked / (float) (W*H) <= percentagedone
                while 100.0_f64 * (csi.num_locked() as f32 as f64) / ((CYPHER_WIDTH * CYPHER_HEIGHT) as f32 as f64)
                    <= percentagedone as f64
                {
                    csi.cypher_lock_random();
                }
            }
        } else if self.status == DECYPHER_FINISHED {
            Vanbakel::remove_task(g, pid);
            return;
        }
    }

    fn create_interface(&mut self, g: &mut Ctx) {
        if self.is_interface_visible(g) {
            return;
        }

        let pid = self.base.pid;
        let [stitle, sborder, sprogress, sclose] = button_names(pid);

        g.ecl.register_button(265, 450, 20, 15, "", "Decypher", &stitle);
        ogl::button_assignbitmap(g, &stitle, "software/dec.tif");

        g.ecl.register_button(285, 450, 120, 15, "", "", &sborder);
        g.ecl.register_button_callbacks(
            &sborder,
            Some(Self::border_draw),
            None,
            Some(ogl::button_click),
            Some(ogl::button_highlight),
        );
        g.ecl.register_button_callback(&sborder, Self::border_click);

        g.ecl.register_button(285, 450, 120, 13, "Select target", "Shows the progress of the decypher", &sprogress);
        g.ecl.register_button_callbacks(
            &sprogress,
            Some(Self::progress_draw),
            None,
            Some(ogl::button_click),
            Some(ogl::button_highlight),
        );

        g.ecl.register_button(405, 450, 13, 13, "", "Close the Decypher (and stop)", &sclose);
        ogl::button_assignbitmaps(g, &sclose, "close.tif", "close_h.tif", "close_c.tif");
        g.ecl.register_button_callback(&sclose, Self::close_click);
    }

    fn remove_interface(&mut self, g: &mut Ctx) {
        if !self.is_interface_visible(g) {
            return;
        }

        for name in button_names(self.base.pid) {
            g.ecl.remove_button(&name);
        }
    }

    fn show_interface(&mut self, g: &mut Ctx) {
        if !self.is_interface_visible(g) {
            self.create_interface(g);
        }

        for name in button_names(self.base.pid) {
            g.ecl.button_bring_to_front(&name);
        }
    }

    fn is_interface_visible(&self, g: &Ctx) -> bool {
        let border = format!("decypher_border {}", self.base.pid);
        g.ecl.get_button(&border).is_some()
    }

    fn set_target(&mut self, g: &mut Ctx, target: ObjRef, targetstring: &str, _targetint: i32) {
        // target : CypherScreen object (ie the screen interface object)
        // targetstring : name of button representing data
        // _targetint : unused

        if self.status != DECYPHER_OFF {
            return;
        }

        // uo->GetOBJECTID() == OID_CYPHERSCREEN
        let ObjRef::ComputerScreen(sr) = target else {
            return;
        };
        let (difficulty, running_monitor) = {
            let Some(comp) = g.world().get_computer(&sr.computer) else {
                debug_assert!(false, "Decypher::set_target: no computer {}", sr.computer);
                return;
            };
            let Some(ComputerScreen::CypherScreen(cs)) = comp.get_computer_screen(sr.index) else {
                return;
            };
            (cs.difficulty, comp.security.is_running_monitor())
        };

        let Some((bx, by, bw, bh)) = g.ecl.get_button(targetstring).map(|b| (b.x, b.y, b.width, b.height)) else {
            debug_assert!(false, "Decypher::set_target: no button {targetstring}");
            return;
        };
        self.move_to(g, bx + bw - 300, by + bh, 1000);

        // C++ also read game->GetInterface()->GetRemoteInterface()->security_level into `security`,
        // which was never used, so it is not read here.
        let ticks = (CYPHER_WIDTH * CYPHER_HEIGHT) as f64 * TICKSREQUIRED_BYPASSCYPHER as f64 * difficulty as f64;
        let difficulty = ticks as i32;
        self.numticksrequired = NumberGenerator::apply_variance(difficulty, (HACKDIFFICULTY_VARIANCE * 100.0) as i32);
        self.progress = 0;

        if running_monitor {
            // The player's connection (owner "PLAYER")
            Connection::begin_trace(g, "PLAYER");
        }

        let pid = self.base.pid;
        let sprogress = format!("decypher_progress {pid}");
        g.ecl.register_caption_change_auto(&sprogress, "Decyphering...", None);

        self.status = DECYPHER_INPROGRESS;
    }

    fn move_to(&mut self, g: &mut Ctx, x: i32, y: i32, time_ms: i32) {
        // UplinkTask::MoveTo is empty in C++.
        let pid = self.base.pid;
        let [stitle, sborder, sprogress, sclose] = button_names(pid);

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
