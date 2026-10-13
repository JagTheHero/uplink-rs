//! Port of source/interface/remoteinterface/cypherscreen_interface.h + source/interface/remoteinterface/cypherscreen_interface.cpp

use uplink_engine::{gl, text};

use crate::app::globals::cheatmodes_enabled;
use crate::app::opengl::screen_h;
use crate::app::opengl_interface::{
    border_draw, button_assignbitmaps, button_click, button_highlight, clear_draw, textbutton_draw,
};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::remoteinterface::remoteinterface::{RemoteInterface, SCREEN_CYPHERSCREEN};
use crate::interface::remoteinterface::remoteinterfacescreen::{draw_main_title, draw_sub_title};
use crate::interface::taskmanager::taskmanager::TaskManager;
use crate::objref::{ObjRef, ScreenRef};
use crate::world::computer::computerscreen::cypherscreen::CypherScreen;
use crate::world::generator::numbergenerator::NumberGenerator;

/// Grid width in cells (C++ `CYPHER_WIDTH`).
pub const CYPHER_WIDTH: usize = 30;
/// Grid height in cells (C++ `CYPHER_HEIGHT`).
pub const CYPHER_HEIGHT: usize = 14;

/// Cypher screen: a grid of digits that cycles until the player locks every cell by software.
#[derive(Debug, Clone)]
pub struct CypherScreenInterface {
    pub cs: Option<ScreenRef>,
    /// Current digit of each cell, as an ASCII char '0'..='9'.
    pub cypher: [[char; CYPHER_HEIGHT]; CYPHER_WIDTH],
    /// True for cells that have been locked (C++ `cypherlock`).
    pub cypherlock: [[bool; CYPHER_HEIGHT]; CYPHER_WIDTH],
    pub numlocked: i32,
    pub lastupdate: i32,
}

impl Default for CypherScreenInterface {
    /// The C++ constructor: every cell gets a random digit and nothing is locked.
    fn default() -> Self {
        let mut cypher = [['0'; CYPHER_HEIGHT]; CYPHER_WIDTH];
        for i in 0..CYPHER_WIDTH {
            for j in 0..CYPHER_HEIGHT {
                cypher[i][j] = random_digit();
            }
        }
        CypherScreenInterface {
            cs: None,
            cypher,
            cypherlock: [[false; CYPHER_HEIGHT]; CYPHER_WIDTH],
            numlocked: 0,
            lastupdate: 0,
        }
    }
}

/// `'0' + RandomNumber(10)`
fn random_digit() -> char {
    (b'0' + NumberGenerator::random_number(10) as u8) as char
}

/// Shared tail of `BypassClick` and `ProceedClick`: grants the Admin security level, then runs the
/// next page of the computer screen.
fn grant_admin_and_continue(g: &mut Ctx, this: &CypherScreenInterface) {
    let nextpage = this.get_computer_screen(g).map(|cs| cs.nextpage);
    g.interface_mut().ri.set_security("Admin", 1);
    if let Some(nextpage) = nextpage.filter(|&np| np != -1) {
        RemoteInterface::run_screen(g, nextpage);
    }
}

impl CypherScreenInterface {
    /// `NumLocked`
    pub fn num_locked(&self) -> i32 {
        self.numlocked
    }

    /// `NumUnLocked`: once they are all locked the screen can be bypassed.
    pub fn num_unlocked(&self) -> i32 {
        (CYPHER_WIDTH * CYPHER_HEIGHT) as i32 - self.numlocked
    }

    /// `IsLocked(x, y)`
    pub fn is_locked(&self, x: i32, y: i32) -> bool {
        self.cypherlock[x as usize][y as usize]
    }

    /// `CypherLock(x, y)`: locks one cell (sets it to '9').
    pub fn cypher_lock(&mut self, x: i32, y: i32) {
        let (x, y) = (x as usize, y as usize);
        if !self.cypherlock[x][y] {
            self.numlocked += 1;
            self.cypher[x][y] = '9';
            self.cypherlock[x][y] = true;
        }
    }

    /// `CypherLock()`: picks an unlocked cell at random and locks it.
    pub fn cypher_lock_random(&mut self) {
        if self.num_unlocked() > 0 {
            loop {
                let x = NumberGenerator::random_number(CYPHER_WIDTH as i32);
                let y = NumberGenerator::random_number(CYPHER_HEIGHT as i32);
                if !self.is_locked(x, y) {
                    self.cypher_lock(x, y);
                    break;
                }
            }
        }
    }

    /// `Cycle`: advances every unlocked cell one step and dirties the grid button.
    pub fn cycle(&mut self, g: &mut Ctx) {
        for i in 0..CYPHER_WIDTH {
            for j in 0..CYPHER_HEIGHT {
                if !self.cypherlock[i][j] {
                    // The C++ test `i < CYPHER_WIDTH+1` is always true, so for the last column it
                    // reads cypher[CYPHER_WIDTH][j], one past the array. In the class layout that
                    // byte is cypherlock[0][j] (a bool, never '1'), so the C++ always takes the
                    // random digit there; the port skips the out-of-bounds read with the same result.
                    let copy_from_right = i + 1 < CYPHER_WIDTH
                        && self.cypher[i + 1][j] == '1'
                        && self.cypher[i][j] != '1';
                    if copy_from_right {
                        self.cypher[i][j] = '1';
                    } else {
                        self.cypher[i][j] = random_digit();
                    }
                } else if self.cypher[i][j] > '0' {
                    self.cypher[i][j] = (self.cypher[i][j] as u8 - 1) as char;
                }
            }
        }

        g.ecl.dirty_button("cypherscreen_cypher");
    }

    /// `GetComputerScreen` (typed accessor; the C++ casts `cs`).
    pub fn get_computer_screen<'a>(&self, g: &'a Ctx) -> Option<&'a CypherScreen> {
        let cs = self.cs.as_ref()?;
        g.world()
            .get_computer(&cs.computer)?
            .get_computer_screen(cs.index)?
            .as_cypher_screen()
    }

    /// `DrawCypher` (static DrawFn)
    pub fn draw_cypher(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        // PORT: the C++ reads the grid through GetInterfaceScreen. The grid is copied out here so the
        // drawing below can borrow `g` mutably.
        let Some((cypher, cypherlock)) = g
            .interface()
            .ri
            .screen
            .get::<CypherScreenInterface>()
            .map(|s| (s.cypher, s.cypherlock))
        else {
            return;
        };

        let screenheight = screen_h();
        gl::scissor(
            button.x,
            screenheight - (button.y + button.height),
            button.width,
            button.height,
        );
        gl::enable(gl::SCISSOR_TEST);

        clear_draw(g, button.x, button.y, button.width, button.height);

        for i in 0..CYPHER_WIDTH {
            for j in 0..CYPHER_HEIGHT {
                // C++: `int xpos = x + width * ((float) i / (float) CYPHER_WIDTH)`, computed in float
                // and truncated to int when stored.
                let xpos = (button.x as f32
                    + button.width as f32 * (i as f32 / CYPHER_WIDTH as f32)) as i32;
                let ypos = ((button.y + 12) as f32
                    + button.height as f32 * (j as f32 / CYPHER_HEIGHT as f32)) as i32;

                let digit = cypher[i][j];
                let label = digit.to_string();

                if cypherlock[i][j] {
                    // C++: `0.2 + (float)(digit - '0') / 10.0` (double arithmetic, stored as float).
                    let shade = (0.2 + f64::from(digit as u8 - b'0') / 10.0) as f32;
                    gl::color4f(shade, shade, shade, 1.0);

                    let cube_w = button.width / CYPHER_WIDTH as i32 + 1;
                    let cube_h = button.height / CYPHER_HEIGHT as i32 + 1;

                    gl::begin(gl::QUADS);
                    gl::vertex2i(xpos, ypos - 10);
                    gl::vertex2i(xpos + cube_w, ypos - 10);
                    gl::vertex2i(xpos + cube_w, ypos + cube_h - 10);
                    gl::vertex2i(xpos, ypos + cube_h - 10);
                    gl::end();

                    gl::color4f(1.0, 1.0, 1.0, 1.0);
                } else {
                    gl::color4f(0.6, 0.6, 0.6, 1.0);
                }

                text::draw_text_styled(xpos, ypos, &label, text::HELVETICA_12);
            }
        }

        if clicked || highlighted {
            gl::color4f(1.0, 1.0, 1.0, 1.0);
            border_draw(button);
        }

        gl::disable(gl::SCISSOR_TEST);
    }

    /// `ClickCypher` (static MouseFn): sets the clicked cell as a program target.
    pub fn click_cypher(g: &mut Ctx, button: &Button) {
        let Some(cs) = g
            .interface()
            .ri
            .screen
            .get::<CypherScreenInterface>()
            .and_then(|s| s.cs.clone())
        else {
            return;
        };

        let targetname = button.name.clone();
        TaskManager::set_program_target(g, ObjRef::ComputerScreen(cs), &targetname, -1);
    }

    /// `BypassClick` (static MouseFn, cheat mode only)
    pub fn bypass_click(g: &mut Ctx, _button: &Button) {
        let Some(this) = g.interface().ri.screen.get::<CypherScreenInterface>().cloned() else {
            return;
        };
        grant_admin_and_continue(g, &this);
    }

    /// `ProceedClick` (static MouseFn): passes the screen once every cell is locked.
    pub fn proceed_click(g: &mut Ctx, _button: &Button) {
        let Some(this) = g.interface().ri.screen.get::<CypherScreenInterface>().cloned() else {
            return;
        };
        if this.num_unlocked() == 0 {
            grant_admin_and_continue(g, &this);
        }
    }
}

impl InterfaceScreen for CypherScreenInterface {
    crate::impl_as_any!();

    /// `Create(ComputerScreen *newcs)` body (the `cs` is already set by `set_computer_screen`).
    fn create(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            return;
        }

        let (maintitle, subtitle) = self
            .get_computer_screen(g)
            .map(|gs| (gs.base.maintitle.clone(), gs.base.subtitle.clone()))
            .unwrap_or_default();

        g.ecl.register_button(80, 60, 350, 25, &maintitle, "", "cypherscreen_maintitle");
        g.ecl.register_button_callbacks("cypherscreen_maintitle", Some(draw_main_title), None, None, None);
        g.ecl.register_button(80, 80, 350, 20, &subtitle, "", "cypherscreen_subtitle");
        g.ecl.register_button_callbacks("cypherscreen_subtitle", Some(draw_sub_title), None, None, None);

        g.ecl.register_button(50, 130, 380, 250, " ", " ", "cypherscreen_cypher");
        g.ecl.register_button_callbacks(
            "cypherscreen_cypher",
            Some(Self::draw_cypher),
            Some(Self::click_cypher),
            Some(button_click),
            Some(button_highlight),
        );

        g.ecl.register_button(350, 380, 80, 15, "Proceed", "Click here when done", "cypherscreen_proceed");
        button_assignbitmaps(g, "cypherscreen_proceed", "proceed.tif", "proceed_h.tif", "proceed_c.tif");
        g.ecl.register_button_callback("cypherscreen_proceed", Self::proceed_click);

        if cheatmodes_enabled() {
            // Create a symbol for quick entry into this screen
            g.ecl.register_button(
                3,
                20,
                30,
                15,
                "#",
                "Click here to bypass this screen (DEBUG MODE ONLY)",
                "cypherscreen_bypass",
            );
            g.ecl.register_button_callbacks(
                "cypherscreen_bypass",
                Some(textbutton_draw),
                Some(Self::bypass_click),
                Some(button_click),
                Some(button_highlight),
            );
        }
    }

    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            for name in [
                "cypherscreen_maintitle",
                "cypherscreen_subtitle",
                "cypherscreen_cypher",
                "cypherscreen_bypass",
                "cypherscreen_proceed",
            ] {
                g.ecl.remove_button(name);
            }
        }
    }

    fn update(&mut self, g: &mut Ctx) {
        if g.ecl.get_accurate_time() > self.lastupdate + 75 {
            self.cycle(g);
            self.lastupdate = g.ecl.get_accurate_time();
        }
    }

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("cypherscreen_maintitle").is_some()
    }

    fn return_key_pressed(&mut self, _g: &mut Ctx) -> bool {
        false
    }

    fn screen_id(&self) -> i32 {
        SCREEN_CYPHERSCREEN
    }

    fn set_computer_screen(&mut self, cs: ScreenRef) {
        self.cs = Some(cs);
    }

    fn computer_screen_ref(&self) -> Option<&ScreenRef> {
        self.cs.as_ref()
    }
}
