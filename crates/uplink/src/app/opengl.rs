//! Port of source/app/opengl.h + source/app/opengl.cpp
//!
//! Generic OpenGL code: the GLUT-style input / idle / display handlers. PORT: GLUT called these
//! through `GciXxxFunc` registration; here the macroquad loop in [`opengl_run`] calls them
//! directly once per frame (`poll_input` -> `idle` -> `display` -> `next_frame`), so
//! `setcallbacks` and `resize` (perspective setup for the unused 3D view) are not ported.
//! The game window itself is created by `uplink_main::main`. Gucci's `GciTimerFunc` has no engine
//! equivalent, so it lives here as [`timer_func`] / [`run_timers`].

#![allow(unused_variables, unused_imports, unused_mut, dead_code)]

use std::cell::{Cell, RefCell};
use std::time::{Duration, Instant};

use macroquad::input::{
    get_char_pressed, is_key_down, is_key_pressed, is_mouse_button_down, is_mouse_button_pressed,
    is_mouse_button_released, KeyCode, MouseButton,
};
use uplink_engine::{gl, sound};

use crate::app::app::App;
use crate::app::globals::{cheatmodes_enabled, DEMOGAME};
use crate::app::opengl_interface::{
    button_click, button_draw, button_highlight, clear_draw, superhighlight_draw,
    textbutton_keypress, tooltip_update,
};
use crate::ctx::Ctx;
use crate::eclipse::{Button, Eclipse};
use crate::interface::interfacescreen::ScreenSlot;
use crate::interface::localinterface::irc_interface::IRCInterface;
use crate::interface::localinterface::localinterface::{
    LocalInterface, SCREEN_CHEATS, SCREEN_IRC, SCREEN_NONE,
};
use crate::interface::localinterface::sw_interface::SWInterface;
use crate::interface::scrollbox::ScrollBox;
use crate::mainmenu::mainmenu::MAINMENU_UNKNOWN;

// Gucci special keys and mouse constants.
pub const GCI_KEY_F1: i32 = 1;
pub const GCI_KEY_F12: i32 = 2;

pub const GCI_UP: i32 = 0;
pub const GCI_DOWN: i32 = 1;

pub const GCI_LEFT_BUTTON: i32 = 1;
pub const GCI_RIGHT_BUTTON: i32 = 2;
pub const GCI_MIDDLE_BUTTON: i32 = 3;

/// Callback of `GciTimerFunc` (`GciCallbackT`: `void (*)(int)`), e.g. `ScriptLibrary::run_script`.
pub type TimerFn = fn(g: &mut Ctx, value: i32);

/// Delay before a held key starts repeating (GLUT's default is about half a second).
const KEY_REPEAT_DELAY: Duration = Duration::from_millis(500);
/// Time between repeats of a held key.
const KEY_REPEAT_INTERVAL: Duration = Duration::from_millis(33);

/// Keys that reach `keyboard` with their ASCII code, and that repeat while held.
const REPEATING_KEYS: [(KeyCode, u8); 5] = [
    (KeyCode::Enter, 13),
    (KeyCode::Escape, 27),
    (KeyCode::Tab, 9),
    (KeyCode::Backspace, 8),
    (KeyCode::Delete, 127),
];

thread_local! {
    // File-scope statics of opengl.cpp.
    static LAST_IDLE_UPDATE: Cell<i32> = const { Cell::new(0) };
    static MOUSE_X: Cell<i32> = const { Cell::new(0) };
    static MOUSE_Y: Cell<i32> = const { Cell::new(0) };
    /// Pending `GciTimerFunc` calls: (due time, callback, value).
    static TIMERS: RefCell<Vec<(Instant, TimerFn, i32)>> = const { RefCell::new(Vec::new()) };
    /// Next repeat time of each key in [`REPEATING_KEYS`] while it is held.
    static KEY_REPEAT_DUE: RefCell<[Option<Instant>; 5]> = const { RefCell::new([None; 5]) };
}

/// `GciTimerFunc`: call `callback(g, value)` once, `millis` ms from now.
pub fn timer_func(millis: u32, callback: TimerFn, value: i32) {
    let due = Instant::now() + Duration::from_millis(millis as u64);
    TIMERS.with(|t| t.borrow_mut().push((due, callback, value)));
}

/// Runs every timer that is due (called once per frame from [`opengl_run`]).
pub fn run_timers(g: &mut Ctx) {
    let now = Instant::now();
    let due: Vec<(TimerFn, i32)> = TIMERS.with(|t| {
        let mut t = t.borrow_mut();
        let (due, later): (Vec<_>, Vec<_>) = t.drain(..).partition(|(when, _, _)| *when <= now);
        *t = later;
        due.into_iter().map(|(_, f, v)| (f, v)).collect()
    });
    for (f, v) in due {
        f(g, v);
    }
}

/// `app->GetOptions()->GetOptionValue("graphics_screenwidth")`, taken from the engine's virtual screen.
pub fn screen_w() -> i32 {
    gl::virtual_width()
}

/// `app->GetOptions()->GetOptionValue("graphics_screenheight")`, taken from the engine's virtual screen.
pub fn screen_h() -> i32 {
    gl::virtual_height()
}

pub fn get_mouse_x() -> i32 {
    MOUSE_X.with(|m| m.get())
}

pub fn get_mouse_y() -> i32 {
    MOUSE_Y.with(|m| m.get())
}

/// `opengl_initialise` (without argc / argv): reads the graphics options, sets the engine's
/// virtual screen size and calls [`init`].
pub fn opengl_initialise(g: &mut Ctx) {
    let debugging = g.app.options.is_option_equal_to("game_debugstart", 1);

    let screen_width = g.app.options.get_option_value("graphics_screenwidth");
    let screen_height = g.app.options.get_option_value("graphics_screenheight");

    // PORT-TODO: GciInitGraphics also took fullscreen, depth and refresh rate. The window is created
    // by uplink_main, which does not apply those options yet.
    if debugging {
        println!("Initialising OpenGL...");
    }
    gl::set_virtual_size(screen_width, screen_height);
    init(g);
    if debugging {
        println!("Finished initialising OpenGL.");
    }

    // PORT: setcallbacks() is not ported; opengl_run calls the handlers in this module directly.
}

/// Shortest frame `opengl_run` allows.
const MIN_FRAME_TIME: Duration = Duration::from_millis(10);

/// The main loop that replaces `GciMainLoop`: input, timers, idle, display, present, deferred actions.
pub async fn opengl_run(g: &mut Ctx) {
    while !g.app.closed {
        let frame_start = Instant::now();
        // Camera (virtual screen -> window) and clear; anything drawn in idle or display lands on it.
        gl::begin_frame(macroquad::color::BLACK);
        poll_input(g);
        run_timers(g);
        idle(g);
        display(g);
        macroquad::window::next_frame().await;
        g.run_deferred();
        if let Some((x, y, number)) = g.pending_dial.take() {
            crate::interface::localinterface::phonedialler::play_dial(g, x, y, &number).await;
        }
        // Without vsync (e.g. a virtual X server) the loop would spin flat out; cap it near 100 fps.
        let frame = Instant::now().duration_since(frame_start);
        if frame < MIN_FRAME_TIME {
            std::thread::sleep(MIN_FRAME_TIME - frame);
        }
    }
}

/// `GciRestoreScreenSize` has nothing to restore under macroquad.
pub fn opengl_close(_g: &mut Ctx) {}

/// Translates macroquad input into the handlers below, GLUT style: mouse movement goes to
/// `passivemouse` (no button held) or `mousemove` (a button held) in virtual-screen coordinates
/// (`gl::mouse_virtual`); left / middle / right presses and releases go to `mouse`; typed
/// characters go to `keyboard` (Latin-1 only), plus Enter (13), Escape (27), Tab (9),
/// Backspace (8) and Delete (127) with key repeat; F1 / F12 go to `specialkeyboard`.
pub fn poll_input(g: &mut Ctx) {
    let (x, y) = gl::mouse_virtual();

    // Motion: with a button held it is a drag, otherwise passive motion.
    if (x, y) != (get_mouse_x(), get_mouse_y()) {
        let held = is_mouse_button_down(MouseButton::Left)
            || is_mouse_button_down(MouseButton::Right)
            || is_mouse_button_down(MouseButton::Middle);
        if held {
            mousemove(g, x, y);
        } else {
            passivemouse(g, x, y);
        }
    }

    for (button, code) in [
        (MouseButton::Left, GCI_LEFT_BUTTON),
        (MouseButton::Right, GCI_RIGHT_BUTTON),
        (MouseButton::Middle, GCI_MIDDLE_BUTTON),
    ] {
        if is_mouse_button_pressed(button) {
            mouse(g, code, GCI_DOWN, x, y);
        }
        if is_mouse_button_released(button) {
            mouse(g, code, GCI_UP, x, y);
        }
    }

    // Typed characters. Control characters are left out: Enter, Tab, Backspace, Escape and Delete
    // arrive through the key table below.
    while let Some(c) = get_char_pressed() {
        let code = c as u32;
        if (32..=255).contains(&code) && code != 127 {
            keyboard(g, code as u8, x, y);
        }
    }

    let now = Instant::now();
    for (i, (key, code)) in REPEATING_KEYS.iter().enumerate() {
        let fire = KEY_REPEAT_DUE.with(|due| {
            let mut due = due.borrow_mut();
            if is_key_pressed(*key) {
                due[i] = Some(now + KEY_REPEAT_DELAY);
                true
            } else if is_key_down(*key) {
                match due[i] {
                    Some(t) if now >= t => {
                        due[i] = Some(now + KEY_REPEAT_INTERVAL);
                        true
                    }
                    _ => false,
                }
            } else {
                due[i] = None;
                false
            }
        });
        if fire {
            keyboard(g, *code, x, y);
        }
    }

    if is_key_pressed(KeyCode::F1) {
        specialkeyboard(g, GCI_KEY_F1, x, y);
    }
    if is_key_pressed(KeyCode::F12) {
        specialkeyboard(g, GCI_KEY_F12, x, y);
    }
    // Port debugging aid: with UPLINK_DEBUG set, F11 dumps every Eclipse button to stdout.
    if is_key_pressed(KeyCode::F11) && std::env::var_os("UPLINK_DEBUG").is_some() {
        g.ecl.debug_print();
    }
}

/// C++ `init` (private): GL state setup, `EclReset` to the configured size, registers the
/// clear / default button / superhighlight draw functions and disables animations if the
/// `graphics_buttonanimations` option is 0.
pub fn init(g: &mut Ctx) {
    // PORT: the C++ GL state setup (clear colour, depth / alpha / fog / lighting / texture / blend
    // switches, pixel transfer, texture parameters and hints) is not ported: the engine's gl layer
    // keeps its own state, and the macroquad renderer ignores these calls.

    let width = g.app.options.get_option_value("graphics_screenwidth");
    let height = g.app.options.get_option_value("graphics_screenheight");
    g.ecl.reset(width, height);
    g.ecl.register_clear_draw_function(Some(clear_draw));
    g.ecl.register_default_button_callbacks(
        Some(button_draw),
        None,
        Some(button_click),
        Some(button_highlight),
    );
    g.ecl.register_super_highlight_function(3, Some(superhighlight_draw));

    if g.app
        .options
        .get_option("graphics_buttonanimations")
        .is_some_and(|o| o.value == 0)
    {
        g.ecl.disable_animations();
    }
}

/// Draws all Eclipse buttons (the C++ drew into both buffers, then cleared the dirty areas).
pub fn display(g: &mut Ctx) {
    if g.app.closed() {
        return;
    }

    // The C++ only dirtied the whole screen when the layer was damaged or in safe mode, because its
    // back buffer kept the previous frame. The macroquad frame is cleared every frame
    // (`gl::begin_frame` in `opengl_run`), so the whole screen is repainted every frame.
    let width = g.app.options.get_option_value("graphics_screenwidth");
    let height = g.app.options.get_option_value("graphics_screenheight");
    g.ecl.dirty_rectangle(0, 0, width, height);

    // Draw the Eclipse buttons. The C++ did this twice, once per GLUT buffer; there is one buffer now.
    gl::push_translate(0.375, 0.375);
    Eclipse::draw_all_buttons(g);
    gl::pop_translate();

    // Finished drawing - clear all dirty areas
    g.ecl.dirty_clear();
}

/// Slot of the remote interface's screen (`GetRemoteInterface()->GetInterfaceScreen()`).
fn remote_screen_slot(g: &mut Ctx) -> &mut ScreenSlot {
    &mut g.game.get_interface_mut().ri.screen
}

/// Slot of the main menu's screen (`GetMainMenu()->GetMenuScreen()`).
fn mainmenu_screen_slot(g: &mut Ctx) -> &mut ScreenSlot {
    &mut g.app.mainmenu.screen
}

/// The highlighted button's name, if that button is editable (`EclIsButtonEditable`).
fn editable_highlighted(g: &Ctx) -> Option<String> {
    g.ecl
        .get_highlighted_button()
        .filter(|name| g.ecl.is_button_editable(name))
}

/// `textbutton_keypress(EclGetButton(name), key)`.
fn press_editable_button(g: &mut Ctx, name: &str, key: u8) {
    if let Some(button) = g.ecl.get_button(name).cloned() {
        textbutton_keypress(g, &button, key);
    }
}

/// Return / Esc / Tab / other keys: forwards to the current screen, the highlighted editable button
/// (`textbutton_keypress`) or `EclHighlightNextEditableButton`.
pub fn keyboard(g: &mut Ctx, key: u8, x: i32, y: i32) {
    if key == 13 {
        // ======== Return key
        let mut returned = false;

        if g.game.is_running() {
            if g.game.get_interface().li.in_screen() != SCREEN_NONE
                && g.game.get_interface().li.screen.screen_id() == SCREEN_IRC
            {
                returned = IRCInterface::return_key_pressed(g);
            } else {
                returned = ScreenSlot::call(g, remote_screen_slot, |s, g| s.return_key_pressed(g))
                    .unwrap_or(false);
            }
        } else if g.app.mainmenu.in_screen() != MAINMENU_UNKNOWN {
            returned = ScreenSlot::call(g, mainmenu_screen_slot, |s, g| s.return_key_pressed(g))
                .unwrap_or(false);
        }

        if !returned {
            // Pass the key press on to the box under the mouse
            if let Some(name) = editable_highlighted(g) {
                press_editable_button(g, &name, key);
            }
        }
    } else if key == 27 {
        // ======== Esc key
        if let Some(name) = editable_highlighted(g) {
            press_editable_button(g, &name, key);
        } else if g.game.is_running() {
            // The C++ kept the result in an unused `escaped` variable.
            ScreenSlot::call(g, remote_screen_slot, |s, g| s.escape_key_pressed(g));
        }
    } else if key == 9 {
        // ========= Tab key
        g.ecl.highlight_next_editable_button();
    } else if let Some(name) = editable_highlighted(g) {
        press_editable_button(g, &name, key);
    }
}

/// F1 = cheat menu (if `globals::cheatmodes_enabled()`), F12 = save and exit.
pub fn specialkeyboard(g: &mut Ctx, key: i32, x: i32, y: i32) {
    match key {
        GCI_KEY_F1 => {
            // Cheat menu. The C++ only had this with CHEATMODES_ENABLED; the other variants were
            // commented out there.
            if cheatmodes_enabled() {
                LocalInterface::run_screen(g, SCREEN_CHEATS, -1);
            }
        }
        GCI_KEY_F12 => {
            // Exit (the C++ had no such case in the demo game)
            if !DEMOGAME {
                if g.game.is_running() {
                    let handle = g.player().agent().handle.clone();
                    App::save_game(g, &handle);
                }
                App::close(g);
            }
        }
        _ => {}
    }
}

/// Updates Eclipse animations and sound, then runs `App::update`; records the idle time.
pub fn idle(g: &mut Ctx) {
    if g.app.closed() {
        return;
    }

    Eclipse::update_all_animations(g);
    sound::update();
    App::update(g);

    let now = g.ecl.get_accurate_time();
    LAST_IDLE_UPDATE.with(|c| c.set(now));
}

/// Mouse button press / release (`state` = `GCI_DOWN` / `GCI_UP`).
pub fn mouse(g: &mut Ctx, button: i32, state: i32, x: i32, y: i32) {
    if button == GCI_LEFT_BUTTON && state == GCI_DOWN {
        if let Some(name) = g.ecl.get_button_at_coord(x, y) {
            Eclipse::button_mouse_down(g, &name);
        }
    } else if button == GCI_LEFT_BUTTON && state == GCI_UP {
        g.ecl.unclick_button();
        ScrollBox::un_grab_scroll_bar(g);

        if let Some(name) = g.ecl.get_button_at_coord(x, y) {
            Eclipse::button_mouse_up(g, &name);
        } else if g.game.is_running() && SWInterface::is_visible_software_menu(g) {
            // Clicked on nothing - remove the software menu
            SWInterface::toggle_software_menu(g);
        }
    } else if button == GCI_RIGHT_BUTTON && state == GCI_UP {
        // Put down any programs that are linked to the cursor
        if g.game.is_running() {
            g.game.get_interface_mut().tm.set_target_program(-1);
        }
    }
}

/// DrawFn of the software mouse cursor button named "mouse".
pub fn mousedraw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
    gl::color4f(1.0, 1.0, 1.0, 1.0);

    gl::begin(gl::LINES);

    gl::line_width(1.0);
    gl::vertex2i(button.x, button.y);
    gl::vertex2i(button.x + button.width - 1, button.y + button.height - 1);

    gl::vertex2i(button.x, button.y);
    gl::vertex2i(
        (button.x as f64 + button.width as f64 / 1.5) as i32,
        button.y,
    );

    gl::vertex2i(button.x, button.y);
    gl::vertex2i(
        button.x,
        (button.y as f64 + button.height as f64 / 1.5) as i32,
    );

    gl::end();

    gl::line_width(1.0);
}

/// Moves the software mouse cursor (the "mouse" button, drawn at `x + 1`, `y + 1`) if the
/// `graphics_softwaremouse` option is 1. The button is registered on first use.
fn move_software_mouse(g: &mut Ctx, x: i32, y: i32) {
    if g.ecl.get_button("mouse").is_none() {
        g.ecl.register_button(0, 0, 8, 8, "", "", "mouse");
        g.ecl
            .register_button_callbacks("mouse", Some(mousedraw), None, None, None);
    }

    g.ecl.button_bring_to_front("mouse");
    let (old_x, old_y, width, height) = {
        let mouse = g.ecl.get_button("mouse").expect("mouse button was just registered");
        (mouse.x, mouse.y, mouse.width, mouse.height)
    };
    g.ecl.dirty_rectangle(old_x, old_y, width, height);
    if let Some(mouse) = g.ecl.get_button_mut("mouse") {
        mouse.x = x + 1;
        mouse.y = y + 1;
    }
    g.ecl.dirty_rectangle(x + 1, y + 1, width, height);
}

/// Mouse moved with no button held.
pub fn passivemouse(g: &mut Ctx, x: i32, y: i32) {
    if g.app.closed() {
        return;
    }

    MOUSE_X.with(|m| m.set(x));
    MOUSE_Y.with(|m| m.set(y));

    if g.app
        .options
        .is_option_equal_to("graphics_softwaremouse", 1)
    {
        move_software_mouse(g, x, y);
    }

    // Look for any buttons under the mouse
    if let Some(name) = g.ecl.get_button_at_coord(x, y) {
        Eclipse::button_mouse_move(g, &name);
    } else {
        g.ecl.unhighlight_button();
        tooltip_update(g, " ");
    }
}

/// Mouse moved with a button held (scroll bar dragging, software mouse).
pub fn mousemove(g: &mut Ctx, x: i32, y: i32) {
    if g.app.closed() {
        return;
    }

    MOUSE_X.with(|m| m.set(x));
    MOUSE_Y.with(|m| m.set(y));

    // Update scroll bars
    if ScrollBox::is_grab_in_progress(g) {
        ScrollBox::update_grab_scroll(g);
    }

    if g.app
        .options
        .is_option_equal_to("graphics_softwaremouse", 1)
    {
        move_software_mouse(g, x, y);
    }
}
