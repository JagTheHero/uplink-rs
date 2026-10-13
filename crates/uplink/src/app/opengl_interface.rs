//! Port of source/app/opengl_interface.h + source/app/opengl_interface.cpp
//!
//! Generic rendering functions for the interface (the Eclipse draw / mouse callbacks), the
//! theme colour helper `set_colour`, scrollable text boxes, modal message boxes and the
//! window scale helpers (`SX` / `SY` macros -> [`sx`] / [`sy`]).
//!
//! PORT: the C++ file-scope statics are thread-locals below. The disabled
//! `USE_DESKTOPBACKDROP` / `USE_BUTTONBACKDROP` code (`initialise_transparency`, `backdrop`) is dropped.
//! Callbacks are plain `fn` items matching `DrawFn` / `MouseFn` / `AnimCallback` / `ClearDrawFn`.
//! `glPushAttrib` / `glPopAttrib` are dropped (PORTING.md); the state they restored is restored explicitly.
//! `GciPostRedisplay` is a no-op: the engine redraws every frame.

#![allow(unused_variables, unused_imports, unused_mut, dead_code)]

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use uplink_engine::gl;
use uplink_engine::image::Image;
use uplink_engine::sound;
use uplink_engine::text;

use crate::app::globals::ALPHA;
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::interface::scrollbox::ScrollBox;

thread_local! {
    /// Index of animation running on the tooltip button (-1 = none).
    static TOOLTIP_ANIM_INDEX: Cell<i32> = const { Cell::new(-1) };
    /// Name of the button the mouse is currently over.
    static CURRENT_BUTTON_NAME: RefCell<Option<String>> = const { RefCell::new(None) };
    /// Used to flash superhighlighted buttons (Eclipse accurate time, ms).
    static SUPERHIGHLIGHT_FLASH: Cell<i32> = const { Cell::new(0) };
    /// Used to flash the text cursor (unix seconds).
    static CURSER_FLASH: Cell<i64> = const { Cell::new(0) };
    /// Used to convert pixel space onto screen size.
    static WINDOW_SCALE: Cell<(f32, f32)> = const { Cell::new((1.0, 1.0)) };
}

/// `time(NULL)`: unix seconds.
fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// `strchr` over a NUL-terminated buffer: stops at a 0 byte like the C library.
fn c_strchr(buf: &[u8], from: usize, c: u8) -> Option<usize> {
    let mut i = from;
    while i < buf.len() {
        if buf[i] == c {
            return Some(i);
        }
        if buf[i] == 0 {
            return None;
        }
        i += 1;
    }
    None
}

/// `strrchr` over the NUL-terminated string starting at `from`.
fn c_strrchr(buf: &[u8], from: usize, c: u8) -> Option<usize> {
    let mut last = None;
    let mut i = from;
    while i < buf.len() && buf[i] != 0 {
        if buf[i] == c {
            last = Some(i);
        }
        i += 1;
    }
    last
}

/// Colour name of the top-edge (A) gradient colour for the button state.
fn gradient_colour_a(highlighted: bool, clicked: bool) -> &'static str {
    if clicked {
        "ButtonClickedA"
    } else if highlighted {
        "ButtonHighlightedA"
    } else {
        "ButtonNormalA"
    }
}

/// Colour name of the bottom-edge (B) gradient colour for the button state.
fn gradient_colour_b(highlighted: bool, clicked: bool) -> &'static str {
    if clicked {
        "ButtonClickedB"
    } else if highlighted {
        "ButtonHighlightedB"
    } else {
        "ButtonNormalB"
    }
}

/// The two-colour vertical gradient quad that most buttons draw as their background.
fn gradient_quad(g: &Ctx, button: &Button, highlighted: bool, clicked: bool) {
    gl::begin(gl::QUADS);

    set_colour(g, gradient_colour_a(highlighted, clicked));
    gl::vertex2i(button.x, button.y + button.height);

    set_colour(g, gradient_colour_b(highlighted, clicked));
    gl::vertex2i(button.x, button.y);

    set_colour(g, gradient_colour_a(highlighted, clicked));
    gl::vertex2i(button.x + button.width, button.y);

    set_colour(g, gradient_colour_b(highlighted, clicked));
    gl::vertex2i(button.x + button.width, button.y + button.height);

    gl::end();
}

/// The image shown for the button state: clicked, then highlighted, then standard.
fn pick_image(button: &Button, highlighted: bool, clicked: bool) -> Option<&Rc<Image>> {
    if clicked && button.image_clicked.is_some() {
        button.image_clicked.as_ref()
    } else if highlighted && button.image_highlighted.is_some() {
        button.image_highlighted.as_ref()
    } else {
        button.image_standard.as_ref()
    }
}

/// Loads a theme image (`ThemeFilename` + `LoadTIF` + `SetAlpha(ALPHA)`).
fn load_themed_image(g: &Ctx, filename: &str) -> Rc<Image> {
    let fullfilename = g.app.options.theme_filename(filename);
    let mut image = Image::new();
    image.load_tif(&fullfilename);
    image.set_alpha(ALPHA);
    Rc::new(image)
}

// ============================================================================
// Colours

/// `SetColour`: `glColor3f` of the named theme colour (black plus a warning if unknown).
pub fn set_colour(g: &Ctx, name: &str) {
    let c = g.app.options.get_colour(name);
    gl::color3f(c.r, c.g, c.b);
}

// ============================================================================
// Eclipse default drawing

/// ClearDrawFn: fills the rectangle with the "Background" colour.
pub fn clear_draw(g: &mut Ctx, x: i32, y: i32, w: i32, h: i32) {
    set_colour(g, "Background");

    gl::begin(gl::QUADS);
    gl::vertex2i(x, y);
    gl::vertex2i(x + w, y);
    gl::vertex2i(x + w, y + h);
    gl::vertex2i(x, y + h);
    gl::end();
}

/// Default button: gradient quad in the clicked / highlighted / normal colours plus centred caption.
pub fn button_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
    let screenheight = g.app.options.get_option_value("graphics_screenheight");
    gl::scissor(button.x, screenheight - (button.y + button.height), button.width, button.height);
    gl::enable(gl::SCISSOR_TEST);

    // Draw the button
    gradient_quad(g, button, highlighted, clicked);

    // Draw the text
    let xpos = (button.x + button.width / 2) - (text::text_width_default(&button.caption) / 2);
    let ypos = (button.y + button.height / 2) + 2;

    set_colour(g, "DefaultText");
    text::draw_text_default(xpos, ypos, &button.caption);

    gl::disable(gl::SCISSOR_TEST);
}

/// Default mouse-move callback: highlights the button and updates the tooltip.
pub fn button_highlight(g: &mut Ctx, button: &Button) {
    g.ecl.highlight_button(&button.name);
    tooltip_update(g, &button.tooltip);

    // (The C++ mouseover sound is commented out there and stays out here.)
    CURRENT_BUTTON_NAME.with(|c| {
        let mut current = c.borrow_mut();
        if current.as_deref() != Some(button.name.as_str()) {
            *current = Some(button.name.clone());
        }
    });
}

/// Default mouse-down callback: clicks the button and plays `sounds/mouseclick.wav`.
pub fn button_click(g: &mut Ctx, button: &Button) {
    g.ecl.click_button(&button.name);

    // SgPlaySound's defaults in C++: id = filename, synchronised = true.
    sound::play_sound("sounds/mouseclick.wav", Some("sounds/mouseclick.wav"), true);
}

/// Draws the button's standard / highlighted / clicked image.
pub fn imagebutton_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
    let screenheight = g.app.options.get_option_value("graphics_screenheight");
    gl::scissor(button.x, screenheight - (button.y + button.height), button.width, button.height);
    gl::enable(gl::SCISSOR_TEST);

    match pick_image(button, highlighted, clicked) {
        Some(image) => image.draw(button.x, button.y),
        None => debug_assert!(false, "imagebutton_draw: button {} has no image", button.name),
    }

    gl::disable(gl::SCISSOR_TEST);
}

/// Like `imagebutton_draw` but scaled to the button's size.
pub fn imagebutton_drawtextured(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
    // glPushAttrib(GL_ALL_ATTRIB_BITS) / glPopAttrib(): dropped; the colour and texture state
    // they would have restored are restored by hand at the end.
    let previous_colour = gl::current_colour();

    let screenheight = g.app.options.get_option_value("graphics_screenheight");
    gl::scissor(button.x, screenheight - (button.y + button.height), button.width, button.height);
    gl::enable(gl::SCISSOR_TEST);

    gl::color4f(1.0, 1.0, 1.0, 1.0);
    gl::enable(gl::TEXTURE_2D);

    let image = pick_image(button, highlighted, clicked);
    debug_assert!(image.is_some(), "imagebutton_drawtextured: button {} has no image", button.name);
    gl::bind_texture(image.map(|i| &**i));

    // Scale the image to fit the button size
    if image.is_some() {
        let (x, y) = (button.x as f32, button.y as f32);
        let (x2, y2) = ((button.x + button.width) as f32, (button.y + button.height) as f32);
        gl::begin(gl::QUADS);
        gl::tex_coord2f(0.0, 1.0);
        gl::vertex2f(x, y);
        gl::tex_coord2f(1.0, 1.0);
        gl::vertex2f(x2, y);
        gl::tex_coord2f(1.0, 0.0);
        gl::vertex2f(x2, y2);
        gl::tex_coord2f(0.0, 0.0);
        gl::vertex2f(x, y2);
        gl::end();
    }

    gl::disable(gl::SCISSOR_TEST);
    gl::disable(gl::TEXTURE_2D);
    gl::color(previous_colour);
}

/// Outline around the button in the current colour.
pub fn border_draw(button: &Button) {
    gl::begin(gl::LINE_LOOP);
    gl::vertex2i(button.x, button.y);
    gl::vertex2i(button.x + button.width - 1, button.y);
    gl::vertex2i(button.x + button.width - 1, button.y + button.height - 1);
    gl::vertex2i(button.x, button.y + button.height - 1);
    gl::end();
}

// ============================================================================
// Text

/// Divides `string` into lines, taking newlines and the pixel width `linesize` into account.
/// An empty vector replaces the C++ NULL result for a NULL string.
// PORT: the C++ cuts the text in place with NUL bytes and hands out pointers into the copy; the
// port keeps the same byte buffer and line start offsets, then copies each line out as a String.
// A line with no break opportunity is cut at the line width and the rest of that word is dropped,
// as in the C++. Lines are cut on byte offsets like the C++ chars; a multi-byte UTF-8 character
// (never in the game's own ASCII text) that gets split is shown as U+FFFD.
pub fn wordwraptext(string: &str, linesize: i32) -> Vec<String> {
    // Calculate the maximum width in characters for 1 line
    let averagecharsize =
        text::text_width_default("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ") as f32 / 52.0;
    let linewidth = (linesize as f32 / averagecharsize) as i32;

    // A width below one character would make the C++ loop spin forever on an unbreakable line
    // (and read before the buffer); this port just splits on the newlines instead.
    // PORT: deliberate guard; the C++ has none (it never terminates for linewidth 0 with a long word).
    if linewidth < 1 {
        return string.split('\n').map(|s| s.to_string()).collect();
    }
    let linewidth = linewidth as usize;

    // Copy of the string with a newline on the end (so a newline is always found) and a NUL terminator
    let mut buf: Vec<u8> = Vec::with_capacity(string.len() + 2);
    buf.extend_from_slice(string.as_bytes());
    buf.push(b'\n');
    buf.push(0);

    // Each line is a start offset into buf (the first is the beginning of the string)
    let mut lines: Vec<usize> = vec![0];
    let mut currentpos: usize = 0;

    loop {
        let Some(nextnewline) = c_strchr(&buf, currentpos, b'\n') else {
            break;
        };

        if nextnewline - currentpos > linewidth {
            // This line is too long and needs trimming down
            // Ignore the newline char we found - it will be dealt with
            // as part of the next line
            // Place a terminater in the space between the last 2 words
            currentpos += linewidth;
            let oldchar = buf[currentpos - 1];
            buf[currentpos - 1] = 0;

            match c_strrchr(&buf, currentpos - linewidth, b' ') {
                Some(space) => {
                    buf[currentpos - 1] = oldchar;
                    currentpos = space + 1;
                    buf[space] = 0;
                    lines.push(currentpos);
                }
                None => {
                    // We cannot wrap this line - the word is longer than the max line width
                }
            }
        } else {
            // Found a newline char - replace with a terminator
            // then add this position in as the next line
            // then continue from this position
            currentpos = nextnewline + 1;
            buf[nextnewline] = 0;
            lines.push(currentpos);
        }
    }

    lines
        .iter()
        .map(|&start| {
            let end = buf[start..].iter().position(|&b| b == 0).map_or(buf.len(), |p| start + p);
            String::from_utf8_lossy(&buf[start..end]).into_owned()
        })
        .collect()
}

/// C++ overload `text_draw(x, y, string, linesize)`: draws word-wrapped text, 15 px per line.
pub fn text_draw_xy(x: i32, y: i32, string: &str, linesize: i32) {
    for (i, line) in wordwraptext(string, linesize).iter().enumerate() {
        let xpos = x;
        let ypos = y + i as i32 * 15;
        text::draw_text_default(xpos, ypos, line);
    }
}

/// C++ overload `text_draw(Button*, ...)` (a DrawFn): the button's caption, wrapped, with a blinking cursor when editable.
pub fn text_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
    let screenheight = g.app.options.get_option_value("graphics_screenheight");
    gl::scissor(button.x, screenheight - (button.y + button.height), button.width, button.height);
    gl::enable(gl::SCISSOR_TEST);

    set_colour(g, "DefaultText");

    // Print the text
    let wrappedtext: Vec<String> = if !highlighted || !g.ecl.is_button_editable(&button.name) {
        wordwraptext(&button.caption, button.width)
    } else {
        // Highlighted - put a symbol at the end of every other second
        let wrapped = if unix_now() >= CURSER_FLASH.get() - 1 {
            let newcaption = format!("{}_", button.caption);
            let wrapped = wordwraptext(&newcaption, button.width);

            if unix_now() >= CURSER_FLASH.get() {
                CURSER_FLASH.set(unix_now() + 2);
            }
            wrapped
        } else {
            wordwraptext(&button.caption, button.width)
        };

        // Must dirty this button so the curser is redrawn
        g.ecl.dirty_button(&button.name);
        wrapped
    };

    for (i, line) in wrappedtext.iter().enumerate() {
        let xpos = button.x + 10;
        let ypos = button.y + 10 + i as i32 * 15;
        text::draw_text_default(xpos, ypos, line);
    }

    gl::disable(gl::SCISSOR_TEST);
}

/// Black background, text, and a border when highlighted.
pub fn textbutton_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
    let screenheight = g.app.options.get_option_value("graphics_screenheight");
    gl::scissor(button.x, screenheight - (button.y + button.height), button.width, button.height);
    gl::enable(gl::SCISSOR_TEST);

    // Black out the background
    clear_draw(g, button.x, button.y, button.width, button.height);

    // Draw the text
    text_draw(g, button, highlighted, clicked);

    // Draw a box around the text if highlighted
    if highlighted || clicked {
        set_colour(g, "TextBorder");
        border_draw(button);
    }

    gl::disable(gl::SCISSOR_TEST);
}

/// Edits the button's caption: Backspace deletes, Esc clears, Enter appends a newline, anything else appends `key`.
pub fn textbutton_keypress(g: &mut Ctx, button: &Button, key: u8) {
    let name = button.name.clone();
    let Some(real) = g.ecl.get_button_mut(&name) else {
        return;
    };

    let mut caption = real.caption.clone();
    if key == 8 {
        // Backspace key
        caption.pop();
    } else if key == 27 {
        // Escape key
        caption.clear();
    } else if key == 13 {
        // Enter key
        caption.push('\n');
    } else {
        caption.push(key as char);
    }
    real.set_caption(&caption);

    g.ecl.dirty_button(&name);
}

/// A text button with a gradient background.
pub fn textbox_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
    // A text button with a background
    let screenheight = g.app.options.get_option_value("graphics_screenheight");
    gl::scissor(button.x, screenheight - (button.y + button.height), button.width, button.height);
    gl::enable(gl::SCISSOR_TEST);

    // Draw the background
    gradient_quad(g, button, highlighted, clicked);

    // Draw the text
    text_draw(g, button, highlighted, clicked);

    // Draw a box around the text if highlighted
    if highlighted || clicked {
        border_draw(button);
    }

    gl::disable(gl::SCISSOR_TEST);
}

pub fn buttonborder_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
    gl::begin(gl::QUADS);

    gl::color4f(0.2, 0.2, 0.4, ALPHA);
    gl::vertex2i(button.x, button.y);
    gl::color4f(0.3, 0.3, 0.5, ALPHA);
    gl::vertex2i(button.x + button.width, button.y);
    gl::color4f(0.2, 0.2, 0.4, ALPHA);
    gl::vertex2i(button.x + button.width, button.y + button.height);
    gl::color4f(0.3, 0.3, 0.5, ALPHA);
    gl::vertex2i(button.x, button.y + button.height);

    gl::end();
}

/// Flashing frame drawn around superhighlighted buttons.
pub fn superhighlight_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
    gl::begin(gl::QUADS);

    let border = 3;

    let timediff = (SUPERHIGHLIGHT_FLASH.get() - g.ecl.get_accurate_time()) as f32 / 1000.0;
    let fraction: f32 = if timediff <= 1.0 {
        timediff
    } else if timediff <= 2.0 {
        2.0 - timediff
    } else {
        0.0
    };

    let dark = || gl::color4f(0.0, 0.0, 0.0, ALPHA);
    let lit = || gl::color4f(fraction, fraction, fraction / 2.0, ALPHA);
    let (x, y, w, h) = (button.x, button.y, button.width, button.height);

    // Top
    dark();
    gl::vertex2i(x + border, y);
    dark();
    gl::vertex2i(x + w - border, y);
    lit();
    gl::vertex2i(x + w - border, y + border);
    lit();
    gl::vertex2i(x + border, y + border);

    // Right
    dark();
    gl::vertex2i(x + w, y + border);
    dark();
    gl::vertex2i(x + w, y + h - border);
    lit();
    gl::vertex2i(x + w - border, y + h - border);
    lit();
    gl::vertex2i(x + w - border, y + border);

    // Bottom
    dark();
    gl::vertex2i(x + border, y + h);
    dark();
    gl::vertex2i(x + w - border, y + h);
    lit();
    gl::vertex2i(x + w - border, y + h - border);
    lit();
    gl::vertex2i(x + border, y + h - border);

    // Left
    dark();
    gl::vertex2i(x, y + border);
    dark();
    gl::vertex2i(x, y + h - border);
    lit();
    gl::vertex2i(x + border, y + h - border);
    lit();
    gl::vertex2i(x + border, y + border);

    // Top left
    dark();
    gl::vertex2i(x + border / 2, y + border / 2);
    dark();
    gl::vertex2i(x + border, y);
    lit();
    gl::vertex2i(x + border, y + border);
    dark();
    gl::vertex2i(x, y + border);

    // Top right
    lit();
    gl::vertex2i(x + w - border, y + border);
    dark();
    gl::vertex2i(x + w - border, y);
    dark();
    gl::vertex2i(x + w - border / 2, y + border / 2);
    dark();
    gl::vertex2i(x + w, y + border);

    // Bottom right
    lit();
    gl::vertex2i(x + w - border, y + h - border);
    dark();
    gl::vertex2i(x + w, y + h - border);
    dark();
    gl::vertex2i(x + w - border / 2, y + h - border / 2);
    dark();
    gl::vertex2i(x + w - border, y + h);

    // Bottom left
    lit();
    gl::vertex2i(x + border, y + h - border);
    dark();
    gl::vertex2i(x + border, y + h);
    dark();
    gl::vertex2i(x + border / 2, y + h - border / 2);
    dark();
    gl::vertex2i(x, y + h - border);

    gl::end();

    if g.ecl.get_accurate_time() >= SUPERHIGHLIGHT_FLASH.get() {
        let next = g.ecl.get_accurate_time() + 2000;
        SUPERHIGHLIGHT_FLASH.set(next);
    }
}

// ============================================================================
// Button bitmaps and tooltip

/// Loads `standard_f` through the theme (`Options::theme_filename`), sets alpha `ALPHA`, and makes the
/// named button an image button.
pub fn button_assignbitmap(g: &mut Ctx, name: &str, standard_f: &str) {
    if g.ecl.get_button(name).is_none() {
        debug_assert!(false, "button_assignbitmap: no button {name}");
        return;
    }

    let standard = load_themed_image(g, standard_f);

    let Some(button) = g.ecl.get_button_mut(name) else {
        return;
    };
    button.set_standard_image(Some(standard));
    button.register_draw_function(Some(imagebutton_draw));

    g.ecl.dirty_button(name);
}

/// As `button_assignbitmap` with separate highlighted and clicked images.
pub fn button_assignbitmaps(
    g: &mut Ctx,
    name: &str,
    standard_f: &str,
    highlighted_f: &str,
    clicked_f: &str,
) {
    if g.ecl.get_button(name).is_none() {
        debug_assert!(false, "button_assignbitmaps: no button {name}");
        return;
    }

    let standard_i = load_themed_image(g, standard_f);
    let highlighted_i = load_themed_image(g, highlighted_f);
    let clicked_i = load_themed_image(g, clicked_f);

    let Some(button) = g.ecl.get_button_mut(name) else {
        return;
    };
    button.set_images(Some(standard_i), Some(highlighted_i), Some(clicked_i));
    button.register_draw_function(Some(imagebutton_draw));

    g.ecl.dirty_button(name);
}

/// AnimCallback: the tooltip caption animation has finished.
pub fn tooltip_callback(_g: &mut Ctx) {
    TOOLTIP_ANIM_INDEX.with(|t| t.set(-1));
}

/// Creates the "tooltip" button if needed, and animates its caption to `newtooltip` (" " = none).
pub fn tooltip_update(g: &mut Ctx, newtooltip: &str) {
    if g.ecl.get_button("tooltip").is_none() {
        // The tooltip button has been removed / never created
        // So create it now
        let screenheight = g.app.options.get_option_value("graphics_screenheight");
        g.ecl.register_button(0, screenheight - 15, 500, 15, "", "", "tooltip");
        g.ecl.register_button_callbacks("tooltip", Some(textbutton_draw), None, None, None);
        g.ecl.button_send_to_back("tooltip");
    }

    let current = g.ecl.get_button("tooltip").map(|b| b.tooltip.clone()).unwrap_or_default();
    if current != newtooltip {
        // Moved mouse over a different button
        if let Some(tooltip) = g.ecl.get_button_mut("tooltip") {
            tooltip.set_tooltip(newtooltip); // Store target here
        }

        // Remove any currently running animation
        let running = TOOLTIP_ANIM_INDEX.get();
        if running != -1 {
            g.ecl.remove_animation(running);
            TOOLTIP_ANIM_INDEX.set(-1);
        }

        if newtooltip == " " {
            // No button under mouse
            if let Some(tooltip) = g.ecl.get_button_mut("tooltip") {
                tooltip.set_caption(" ");
            }
            CURRENT_BUTTON_NAME.with(|c| *c.borrow_mut() = None);
        } else {
            let anim = g.ecl.register_caption_change_auto("tooltip", newtooltip, Some(tooltip_callback));
            TOOLTIP_ANIM_INDEX.set(anim);
        }
    }
}

/// Returns the button under the mouse, or None.
pub fn getcurrentbutton(g: &Ctx) -> Option<&Button> {
    let name = CURRENT_BUTTON_NAME.with(|c| c.borrow().clone())?;
    g.ecl.get_button(&name)
}

// ============================================================================
// Scrollable text box

/// Creates `"<name> box"` plus a scroll bar (`ScrollBox::create_scroll_box`) on its right.
pub fn create_stextbox(
    g: &mut Ctx,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    caption: &str,
    name: &str,
) {
    let name_box = format!("{name} box");
    g.ecl.register_button(x, y, width - 16, height, caption, "", &name_box);
    g.ecl.register_button_callbacks(&name_box, Some(draw_stextbox), None, None, None);

    let num_items = if caption.len() < 5 {
        15
    } else {
        let wrapped_text = wordwraptext(caption, width);
        let n = wrapped_text.len() as i32 + 2;
        if n < 4 {
            15
        } else {
            n
        }
    };

    let scroll_left = x + width - 15;
    let window_size = height / 15;
    ScrollBox::create_scroll_box(
        g,
        name,
        scroll_left,
        y,
        15,
        height,
        num_items,
        window_size,
        0,
        Some(stextbox_scroll),
    );
}

pub fn remove_stextbox(g: &mut Ctx, name: &str) {
    let name_box = format!("{name} box");
    g.ecl.remove_button(&name_box);

    ScrollBox::remove_scroll_box(g, name);
}

pub fn draw_stextbox(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
    let screenheight = g.app.options.get_option_value("graphics_screenheight");
    gl::scissor(button.x, screenheight - (button.y + button.height), button.width, button.height);
    gl::enable(gl::SCISSOR_TEST);

    // Get the offset
    // sscanf(button->name, "%s"): the first whitespace-delimited word of the name
    let name_base = button.name.split_whitespace().next().unwrap_or("");
    // PORT: the C++ returns here without glDisable(GL_SCISSOR_TEST); kept as it was.
    let Some(scroll_box) = ScrollBox::get_scroll_box(g, name_base) else {
        return;
    };
    let offset = scroll_box.current_index;

    // Draw the button
    gradient_quad(g, button, highlighted, clicked);

    // Draw a border if highlighted
    if highlighted || clicked {
        border_draw(button);
    }

    // Draw the text
    let maxnumlines = (button.height - 10) / 15;

    set_colour(g, "DefaultText");

    let wrappedtext = wordwraptext(&button.caption, button.width);

    let mut i = offset;
    while i < wrappedtext.len() as i32 {
        if i > maxnumlines + offset {
            break;
        }
        if let Some(line) = usize::try_from(i).ok().and_then(|k| wrappedtext.get(k)) {
            let xpos = button.x + 10;
            let ypos = button.y + 10 + (i - offset) * 15;
            text::draw_text_styled(xpos, ypos, line, text::HELVETICA_10);
        }
        i += 1;
    }

    gl::disable(gl::SCISSOR_TEST);
}

pub fn draw_scrollbox(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
    gradient_quad(g, button, highlighted, clicked);
}

/// ScrollCallback of the scrollable text box: dirties `"<name> box"`.
pub fn stextbox_scroll(g: &mut Ctx, name: &str, new_value: i32) {
    let name_box = format!("{name} box");
    g.ecl.dirty_button(&name_box);
}

// ============================================================================
// Modal message boxes

/// The background overlay is drawn by nothing (the C++ had its transparency code commented out).
pub fn draw_msgboxbackground(_g: &mut Ctx, _button: &Button, _highlighted: bool, _clicked: bool) {}

pub fn draw_msgboxbox(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
    gl::begin(gl::QUADS);
    set_colour(g, "PanelBackgroundA");
    gl::vertex2i(button.x, button.y + button.height);
    set_colour(g, "PanelBackgroundB");
    gl::vertex2i(button.x, button.y);
    set_colour(g, "PanelBackgroundA");
    gl::vertex2i(button.x + button.width, button.y);
    set_colour(g, "PanelBackgroundB");
    gl::vertex2i(button.x + button.width, button.y + button.height);
    gl::end();

    set_colour(g, "PanelBorder");
    border_draw(button);
}

pub fn closeclick_msgbox(g: &mut Ctx, button: &Button) {
    remove_msgbox(g);
}

/// Shows a modal message box (does nothing if one is already visible).
pub fn create_msgbox(g: &mut Ctx, title: &str, message: &str) {
    if !isvisible_msgbox(g) {
        // Blank out the background
        let screenw = g.app.options.get_option_value("graphics_screenwidth");
        let screenh = g.app.options.get_option_value("graphics_screenheight");

        g.ecl.register_button(0, 0, screenw, screenh, "", "", "msgbox_background");
        g.ecl.register_button_callbacks("msgbox_background", Some(draw_msgboxbackground), None, None, None);

        // Create the message box
        g.ecl.register_button(screenw / 2 - 100, screenh / 3, 200, 140, "", "", "msgbox_box");
        g.ecl.register_button_callbacks("msgbox_box", Some(draw_msgboxbox), None, None, None);

        // Title bar
        g.ecl.register_button(
            screenw / 2 - 100 + 2,
            screenh / 3 + 2,
            195,
            15,
            title,
            "Close the message box",
            "msgbox_title",
        );
        g.ecl.register_button_callback("msgbox_title", closeclick_msgbox);

        // Message box
        g.ecl.register_button(screenw / 2 - 100 + 2, screenh / 3 + 20, 195, 80, "", "", "msgbox_text");
        g.ecl.register_button_callbacks("msgbox_text", Some(text_draw), None, None, None);
        // C++ EclRegisterCaptionChange(name, caption): no callback, time from the caption length
        g.ecl.register_caption_change_auto("msgbox_text", message, None);

        // OK button
        g.ecl.register_button(
            screenw / 2 - 50,
            screenh / 3 + 110,
            100,
            15,
            "Close",
            "Close the message box",
            "msgbox_close",
        );
        g.ecl.register_button_callback("msgbox_close", closeclick_msgbox);
    }
}

pub fn remove_msgbox(g: &mut Ctx) {
    if isvisible_msgbox(g) {
        g.ecl.remove_button("msgbox_background");
        g.ecl.remove_button("msgbox_box");
        g.ecl.remove_button("msgbox_title");
        g.ecl.remove_button("msgbox_text");
        g.ecl.remove_button("msgbox_close");
    }
}

pub fn isvisible_msgbox(g: &Ctx) -> bool {
    g.ecl.get_button("msgbox_background").is_some()
}

// ============================================================================
// Window scale

pub fn set_window_scale_factor(scale_x: f32, scale_y: f32) {
    WINDOW_SCALE.with(|s| s.set((scale_x, scale_y)));
}

/// `GetScaledXPosition` (the `SX` macro).
pub fn sx(x_pos: i32) -> i32 {
    (x_pos as f32 * WINDOW_SCALE.with(|s| s.get().0)) as i32
}

/// `GetScaledYPosition` (the `SY` macro).
pub fn sy(y_pos: i32) -> i32 {
    (y_pos as f32 * WINDOW_SCALE.with(|s| s.get().1)) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_text_stays_on_one_line() {
        let lines = wordwraptext("Hello world", 1000);
        assert_eq!(lines.first().map(String::as_str), Some("Hello world"));
        assert!(lines[1..].iter().all(|l| l.is_empty()), "{lines:?}");
    }

    #[test]
    fn newlines_always_break_lines() {
        let lines = wordwraptext("one\ntwo\nthree", 1000);
        let text: Vec<&str> = lines.iter().map(String::as_str).filter(|l| !l.is_empty()).collect();
        assert_eq!(text, ["one", "two", "three"]);
        assert_eq!(&lines[..3], ["one", "two", "three"]);
    }

    #[test]
    fn long_text_wraps_between_words_and_loses_nothing() {
        let text = "The quick brown fox jumps over the lazy dog and keeps on running through the field";
        let narrow = wordwraptext(text, 120);
        assert!(narrow.iter().filter(|l| !l.is_empty()).count() > 2, "{narrow:?}");
        let words: Vec<&str> = narrow.iter().flat_map(|l| l.split_whitespace()).collect();
        assert_eq!(words.join(" "), text);

        // a narrower box never gives fewer lines than a wider one
        let wide = wordwraptext(text, 400);
        let count = |v: &Vec<String>| v.iter().filter(|l| !l.is_empty()).count();
        assert!(count(&narrow) > count(&wide));
    }

    #[test]
    fn a_degenerate_width_just_splits_on_newlines() {
        assert_eq!(wordwraptext("a b\nc", 0), ["a b", "c"]);
    }

    #[test]
    fn window_scale_factor_scales_positions() {
        set_window_scale_factor(2.0, 0.5);
        assert_eq!((sx(10), sy(10)), (20, 5));
        set_window_scale_factor(1.0, 1.0);
        assert_eq!((sx(10), sy(10)), (10, 10));
    }
}
