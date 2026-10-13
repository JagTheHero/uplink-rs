//! Port of `lib/eclipse`: Uplink's button-based interface library.
//!
//! The C++ kept everything in file-scope globals; here it is one [`Eclipse`] value stored in
//! `Ctx::ecl`. Plain bookkeeping is `&mut self` methods (`g.ecl.register_button(..)`); anything that
//! calls back into game code (drawing, animations finishing, mouse events) is an associated
//! function taking the whole context (`Eclipse::draw_all_buttons(g)`), because callbacks need `&mut Ctx`.
//!
//! Buttons are ordered front-to-back: index 0 is the front-most button. Drawing goes back to front.
//!
//! The original only redrew "dirty" buttons. This port redraws every button every frame, so the
//! dirty-tracking calls are kept for API compatibility but only record state.

pub mod animation;
pub mod button;

use std::collections::BTreeSet;

pub use animation::Animation;
pub use button::{AnimCallback, Button, ClearDrawFn, DrawFn, MouseFn};

use crate::ctx::Ctx;
use crate::tosser::DArray;

pub const MOVE_STRAIGHTLINE: i32 = 1;
pub const MOVE_XFIRST: i32 = 2;
pub const MOVE_YFIRST: i32 = 3;

#[derive(Default)]
pub struct Eclipse {
    buttons: Vec<Button>,
    currenthighlight: Option<String>,
    currentclick: Option<String>,
    anims: DArray<Animation>,
    animsdisabled: bool,
    superhighlightedbuttons: BTreeSet<String>,
    superhighlight_borderwidth: i32,
    editablebuttons: Vec<String>,
    dirtyrectangles: Vec<(i32, i32, i32, i32)>,
    default_draw: Option<DrawFn>,
    default_mouseup: Option<MouseFn>,
    default_mousedown: Option<MouseFn>,
    default_mousemove: Option<MouseFn>,
    clear_draw: Option<ClearDrawFn>,
    superhighlight_draw: Option<DrawFn>,
    start: Option<std::time::Instant>,
}

fn superhighlight_name(name: &str) -> String {
    format!("Ecl_superhighlight {name}")
}

fn rectangle_overlap(x1: i32, y1: i32, w1: i32, h1: i32, x2: i32, y2: i32, w2: i32, h2: i32) -> bool {
    let maxleft = x1.max(x2);
    let maxtop = y1.max(y2);
    let minright = (x1 + w1).min(x2 + w2);
    let minbottom = (y1 + h1).min(y2 + h2);
    maxtop < minbottom && maxleft < minright
}

impl Eclipse {
    pub fn new() -> Self {
        Self::default()
    }

    /// `EclReset`: remove every button (animations survive, as in the C++).
    pub fn reset(&mut self, width: i32, height: i32) {
        self.currenthighlight = None;
        self.currentclick = None;
        self.buttons.clear();
        self.editablebuttons.clear();
        self.superhighlight_borderwidth = 0;
        self.dirty_clear();
        self.dirty_rectangle(0, 0, width, height);
    }

    // ------------------------------------------------------------------ buttons

    /// `EclRegisterButton(x, y, w, h, caption, tooltip, name)`. Re-registering an existing name
    /// is ignored (with a warning in debug builds), exactly like the C++.
    pub fn register_button(&mut self, x: i32, y: i32, width: i32, height: i32, caption: &str, tooltip: &str, name: &str) {
        if self.get_button(name).is_some() {
            if cfg!(debug_assertions) {
                eprintln!("ECL WARNING : EclRegisterButton called, Button name not unique : {name}");
            }
            return;
        }
        let mut button = Button::new(x, y, width, height, caption, name);
        button.set_tooltip(tooltip);
        self.buttons.insert(0, button);
        let (d, u, dn, m) = (self.default_draw, self.default_mouseup, self.default_mousedown, self.default_mousemove);
        self.register_button_callbacks(name, d, u, dn, m);
        self.dirty_button(name);
    }

    /// `EclRegisterImageButton`: same as a normal button; images are attached separately.
    pub fn register_image_button(&mut self, x: i32, y: i32, width: i32, height: i32, caption: &str, tooltip: &str, name: &str) {
        self.register_button(x, y, width, height, caption, tooltip, name);
    }

    pub fn remove_button(&mut self, name: &str) {
        let index = self.lookup_index(name);
        if index < 0 {
            if cfg!(debug_assertions) {
                eprintln!("ECL WARNING : EclRemoveButton called, button does not exist : {name}");
            }
            return;
        }
        if self.is_highlighted(name) {
            self.currenthighlight = None;
        }
        if self.is_clicked(name) {
            self.currentclick = None;
        }
        if self.is_super_highlighted(name) {
            self.super_unhighlight(name);
        }
        if self.is_button_editable(name) {
            self.make_button_uneditable(name);
        }
        let index = self.lookup_index(name);
        if index >= 0 {
            let b = self.buttons.remove(index as usize);
            self.dirty_rectangle(b.x, b.y, b.width, b.height);
        }
    }

    pub fn button_bring_to_front(&mut self, name: &str) {
        let index = self.lookup_index(name);
        if index >= 0 {
            let b = self.buttons.remove(index as usize);
            self.buttons.insert(0, b);
            self.dirty_button(name);
        }
    }

    pub fn button_send_to_back(&mut self, name: &str) {
        let index = self.lookup_index(name);
        if index >= 0 {
            let b = self.buttons.remove(index as usize);
            self.buttons.push(b);
            self.dirty_button(name);
        }
    }

    pub fn register_default_button_callbacks(
        &mut self,
        draw: Option<DrawFn>,
        mouseup: Option<MouseFn>,
        mousedown: Option<MouseFn>,
        mousemove: Option<MouseFn>,
    ) {
        self.default_draw = draw;
        self.default_mouseup = mouseup;
        self.default_mousedown = mousedown;
        self.default_mousemove = mousemove;
    }

    pub fn register_button_callbacks(
        &mut self,
        name: &str,
        draw: Option<DrawFn>,
        mouseup: Option<MouseFn>,
        mousedown: Option<MouseFn>,
        mousemove: Option<MouseFn>,
    ) {
        match self.get_button_mut(name) {
            Some(b) => {
                b.register_draw_function(draw);
                b.register_mouse_up_function(mouseup);
                b.register_mouse_down_function(mousedown);
                b.register_mouse_move_function(mousemove);
            }
            None => {
                if cfg!(debug_assertions) {
                    eprintln!("ECL WARNING : EclRegisterButtonCallbacks called, button does not exist : {name}");
                }
            }
        }
    }

    pub fn register_button_callback(&mut self, name: &str, mouseup: MouseFn) {
        if let Some(b) = self.get_button_mut(name) {
            b.register_mouse_up_function(Some(mouseup));
        }
    }

    pub fn make_button_editable(&mut self, name: &str) {
        if self.get_button(name).is_some() {
            self.editablebuttons.push(name.to_string());
        }
    }

    pub fn make_button_uneditable(&mut self, name: &str) {
        if let Some(i) = self.editablebuttons.iter().position(|b| b == name) {
            self.editablebuttons.remove(i);
        }
    }

    pub fn is_button_editable(&self, name: &str) -> bool {
        self.get_button(name).is_some() && self.editablebuttons.iter().any(|b| b == name)
    }

    pub fn highlight_next_editable_button(&mut self) {
        if let Some(current) = self.currenthighlight.clone() {
            if let Some(i) = self.editablebuttons.iter().position(|b| *b == current) {
                let next = if i + 1 < self.editablebuttons.len() { i + 1 } else { 0 };
                let name = self.editablebuttons[next].clone();
                self.highlight_button(&name);
                return;
            }
        }
        if let Some(first) = self.editablebuttons.first().cloned() {
            self.highlight_button(&first);
        }
    }

    /// `EclLookupIndex`: position in the front-to-back list, or -1.
    pub fn lookup_index(&self, name: &str) -> i32 {
        self.buttons.iter().position(|b| b.name == name).map(|i| i as i32).unwrap_or(-1)
    }

    pub fn is_occupied(&self, x: i32, y: i32, w: i32, h: i32) -> bool {
        self.buttons.iter().any(|b| rectangle_overlap(b.x, b.y, b.width, b.height, x, y, w, h))
    }

    pub fn dirty_button(&mut self, name: &str) {
        if let Some(b) = self.get_button_mut(name) {
            b.dirty();
        }
    }

    pub fn dirty_rectangle(&mut self, x: i32, y: i32, w: i32, h: i32) {
        self.dirtyrectangles.push((x, y, w, h));
        for b in &mut self.buttons {
            if rectangle_overlap(x, y, w, h, b.x, b.y, b.width, b.height) {
                b.dirty();
            }
        }
    }

    pub fn dirty_clear(&mut self) {
        self.dirtyrectangles.clear();
        for b in &mut self.buttons {
            b.is_dirty = false;
        }
    }

    pub fn highlight_button(&mut self, name: &str) {
        if self.currenthighlight.as_deref() != Some(name) {
            self.unhighlight_button();
            self.currenthighlight = Some(name.to_string());
            self.dirty_button(name);
        }
    }

    pub fn click_button(&mut self, name: &str) {
        if self.currentclick.as_deref() != Some(name) {
            self.unclick_button();
            self.currentclick = Some(name.to_string());
            self.dirty_button(name);
        }
    }

    pub fn is_highlighted(&self, name: &str) -> bool {
        self.currenthighlight.as_deref() == Some(name)
    }

    pub fn is_clicked(&self, name: &str) -> bool {
        self.currentclick.as_deref() == Some(name)
    }

    pub fn unhighlight_button(&mut self) {
        if let Some(h) = self.currenthighlight.take() {
            self.dirty_button(&h);
        }
    }

    pub fn unclick_button(&mut self) {
        if let Some(c) = self.currentclick.take() {
            self.dirty_button(&c);
        }
    }

    pub fn super_highlight(&mut self, name: &str) {
        let Some(b) = self.get_button(name) else { return };
        if self.superhighlightedbuttons.contains(name) {
            return;
        }
        let bw = self.superhighlight_borderwidth;
        let (x, y, w, h) = (b.x - bw, b.y - bw, b.width + bw * 2, b.height + bw * 2);
        self.superhighlightedbuttons.insert(name.to_string());
        let shname = superhighlight_name(name);
        self.register_button(x, y, w, h, "", "", &shname);
        let draw = self.superhighlight_draw;
        self.register_button_callbacks(&shname, draw, None, None, None);
        self.button_send_to_back(&shname);
    }

    pub fn super_unhighlight(&mut self, name: &str) {
        if self.superhighlightedbuttons.remove(name) {
            self.remove_button(&superhighlight_name(name));
        }
    }

    pub fn super_unhighlight_all(&mut self) {
        let all: Vec<String> = self.superhighlightedbuttons.iter().cloned().collect();
        for n in all {
            self.super_unhighlight(&n);
        }
    }

    pub fn is_super_highlighted(&self, name: &str) -> bool {
        self.superhighlightedbuttons.contains(name)
    }

    pub fn register_clear_draw_function(&mut self, draw: Option<ClearDrawFn>) {
        self.clear_draw = draw;
    }

    pub fn register_super_highlight_function(&mut self, borderwidth: i32, draw: Option<DrawFn>) {
        self.superhighlight_borderwidth = borderwidth;
        self.superhighlight_draw = draw;
    }

    pub fn update_super_highlights(&mut self, name: &str) {
        let Some(src) = self.get_button(name).map(|b| (b.x, b.y)) else {
            self.super_unhighlight(name);
            return;
        };
        if !self.is_super_highlighted(name) {
            return;
        }
        let bw = self.superhighlight_borderwidth;
        let shname = superhighlight_name(name);
        if let Some(hb) = self.get_button_mut(&shname) {
            hb.x = src.0 - bw;
            hb.y = src.1 - bw;
            hb.dirty();
        }
    }

    /// `EclGetButtonAtCoord`: front-most button containing the point (edges inclusive).
    pub fn get_button_at_coord(&self, x: i32, y: i32) -> Option<String> {
        self.buttons
            .iter()
            .find(|b| x >= b.x && x <= b.x + b.width && y >= b.y && y <= b.y + b.height)
            .map(|b| b.name.clone())
    }

    pub fn get_highlighted_button(&self) -> Option<String> {
        self.currenthighlight.clone()
    }

    pub fn get_clicked_button(&self) -> Option<String> {
        self.currentclick.clone()
    }

    pub fn get_button(&self, name: &str) -> Option<&Button> {
        self.buttons.iter().find(|b| b.name == name)
    }

    pub fn get_button_mut(&mut self, name: &str) -> Option<&mut Button> {
        self.buttons.iter_mut().find(|b| b.name == name)
    }

    /// All buttons, front-most first.
    pub fn buttons(&self) -> &[Button] {
        &self.buttons
    }

    /// Names of all buttons whose name starts with `prefix` (handy for bulk removal).
    pub fn button_names_with_prefix(&self, prefix: &str) -> Vec<String> {
        self.buttons.iter().filter(|b| b.name.starts_with(prefix)).map(|b| b.name.clone()).collect()
    }

    // ------------------------------------------------------------------ animations

    pub fn enable_animations(&mut self) {
        self.animsdisabled = false;
    }

    pub fn disable_animations(&mut self) {
        self.animsdisabled = true;
    }

    pub fn register_movement(&mut self, bname: &str, target_x: i32, target_y: i32, time_ms: i32, movetype: i32, callback: Option<AnimCallback>) -> i32 {
        let Some(b) = self.get_button(bname) else { return -1 };
        let (w, h) = (b.width, b.height);
        self.register_animation_full(bname, target_x, target_y, movetype, w, h, None, time_ms, callback)
    }

    pub fn register_resize(&mut self, bname: &str, target_w: i32, target_h: i32, time_ms: i32, callback: Option<AnimCallback>) -> i32 {
        let Some(b) = self.get_button(bname) else { return -1 };
        let (x, y) = (b.x, b.y);
        self.register_animation_full(bname, x, y, MOVE_STRAIGHTLINE, target_w, target_h, None, time_ms, callback)
    }

    pub fn register_animation(
        &mut self,
        bname: &str,
        target_x: i32,
        target_y: i32,
        target_w: i32,
        target_h: i32,
        time_ms: i32,
        callback: Option<AnimCallback>,
    ) -> i32 {
        self.register_animation_full(bname, target_x, target_y, MOVE_STRAIGHTLINE, target_w, target_h, None, time_ms, callback)
    }

    /// `EclRegisterCaptionChange(name, caption, time_ms, callback)`
    pub fn register_caption_change(&mut self, bname: &str, target_c: &str, time_ms: i32, callback: Option<AnimCallback>) -> i32 {
        let Some(b) = self.get_button(bname) else {
            if cfg!(debug_assertions) {
                eprintln!("ECL WARNING : Tried to register a caption change, could not find button {bname}");
            }
            return -1;
        };
        let (x, y, w, h) = (b.x, b.y, b.width, b.height);
        self.register_animation_full(bname, x, y, MOVE_STRAIGHTLINE, w, h, Some(target_c), time_ms, callback)
    }

    /// `EclRegisterCaptionChange(name, caption, callback)`: duration = 50ms per character.
    pub fn register_caption_change_auto(&mut self, bname: &str, target_c: &str, callback: Option<AnimCallback>) -> i32 {
        let time = target_c.len() as i32 * 50;
        self.register_caption_change(bname, target_c, time, callback)
    }

    /// The full `EclRegisterAnimation(name, x, y, MOVETYPE, w, h, caption, time, callback)`.
    #[allow(clippy::too_many_arguments)]
    pub fn register_animation_full(
        &mut self,
        bname: &str,
        target_x: i32,
        target_y: i32,
        movetype: i32,
        target_w: i32,
        target_h: i32,
        target_c: Option<&str>,
        time_ms: i32,
        callback: Option<AnimCallback>,
    ) -> i32 {
        let now = self.get_accurate_time();
        let Some(b) = self.get_button(bname) else { return -1 };
        let mut anim = Animation {
            buttonname: bname.to_string(),
            movetype,
            source_x: b.x,
            source_y: b.y,
            target_x,
            target_y,
            source_w: b.width,
            source_h: b.height,
            target_w,
            target_h,
            callback,
            time: time_ms,
            starttime: now,
            finishtime: now + time_ms,
            ..Default::default()
        };
        if anim.finishtime - anim.starttime == 0 {
            anim.finishtime = anim.starttime + 1;
        }
        let span = (anim.finishtime - anim.starttime) as f32;
        if anim.target_x != anim.source_x || anim.target_y != anim.source_y {
            anim.d_x = (anim.target_x - anim.source_x) as f32 / span;
            anim.d_y = (anim.target_y - anim.source_y) as f32 / span;
        }
        if anim.target_w != anim.source_w || anim.target_h != anim.source_h {
            anim.d_w = (anim.target_w - anim.source_w) as f32 / span;
            anim.d_h = (anim.target_h - anim.source_h) as f32 / span;
        }
        if let Some(tc) = target_c {
            loop {
                let ca = self.is_caption_change_active(bname);
                if ca == -1 {
                    break;
                }
                self.remove_animation(ca);
            }
            anim.target_c = Some(tc.to_string());
            anim.d_c = tc.len() as f32 / span;
        }
        self.anims.put_data(anim)
    }

    /// Returns the id of an animation on this button, or -1.
    pub fn is_animation_active(&self, bname: &str) -> i32 {
        self.anims.iter().find(|(_, a)| a.buttonname == bname).map(|(i, _)| i).unwrap_or(-1)
    }

    pub fn is_caption_change_active(&self, bname: &str) -> i32 {
        self.anims
            .iter()
            .find(|(_, a)| a.buttonname == bname && a.target_c.is_some())
            .map(|(i, _)| i)
            .unwrap_or(-1)
    }

    pub fn remove_animation(&mut self, idnumber: i32) {
        self.anims.remove_data(idnumber);
    }

    /// Milliseconds since the first call (`EclGetAccurateTime`).
    pub fn get_accurate_time(&mut self) -> i32 {
        let start = *self.start.get_or_insert_with(std::time::Instant::now);
        start.elapsed().as_millis() as i32
    }

    /// `EclUpdateAllAnimations`: advance every animation; runs finished callbacks.
    pub fn update_all_animations(g: &mut Ctx) {
        let names: Vec<String> = g.ecl.superhighlightedbuttons.iter().cloned().collect();
        for n in names {
            g.ecl.update_super_highlights(&n);
        }

        let size = g.ecl.anims.size();
        for i in 0..size {
            let Some(anim) = g.ecl.anims.get_data(i).cloned() else { continue };
            let now = g.ecl.get_accurate_time();
            let disabled = g.ecl.animsdisabled;
            let Some(button) = g.ecl.get_button_mut(&anim.buttonname) else {
                g.ecl.remove_animation(i);
                continue;
            };
            button.dirty();
            if now >= anim.finishtime || disabled {
                if anim.d_x != 0.0 || anim.d_y != 0.0 {
                    button.x = anim.target_x;
                    button.y = anim.target_y;
                }
                if anim.d_w != 0.0 || anim.d_h != 0.0 {
                    button.width = anim.target_w;
                    button.height = anim.target_h;
                }
                if anim.d_c != 0.0 {
                    if let Some(tc) = &anim.target_c {
                        button.set_caption(tc);
                    }
                }
                if g.ecl.is_super_highlighted(&anim.buttonname) {
                    g.ecl.update_super_highlights(&anim.buttonname);
                }
                // Remove before the callback: callbacks often register a new animation on the
                // same button (e.g. a blinking cursor), which may reuse this slot.
                g.ecl.remove_animation(i);
                if let Some(cb) = anim.callback {
                    cb(g);
                }
            } else {
                let elapsed = (now - anim.starttime) as f32;
                match anim.movetype {
                    MOVE_XFIRST | MOVE_YFIRST if anim.d_x != 0.0 || anim.d_y != 0.0 => {
                        let xfirst = anim.movetype == MOVE_XFIRST;
                        let ratio = if xfirst { anim.d_x / (anim.d_x + anim.d_y) } else { anim.d_y / (anim.d_x + anim.d_y) };
                        let halftime = anim.starttime + (ratio * (anim.finishtime - anim.starttime) as f32) as i32;
                        if now < halftime {
                            let firsthalf = (now - anim.starttime) as f32 / (halftime - anim.starttime).max(1) as f32;
                            if xfirst {
                                button.x = anim.source_x + ((anim.target_x - anim.source_x) as f32 * firsthalf) as i32;
                            } else {
                                button.y = anim.source_y + ((anim.target_y - anim.source_y) as f32 * firsthalf) as i32;
                            }
                        } else {
                            let secondhalf = (now - halftime) as f32 / (anim.finishtime - halftime).max(1) as f32;
                            if xfirst {
                                button.x = anim.target_x;
                                button.y = anim.source_y + ((anim.target_y - anim.source_y) as f32 * secondhalf) as i32;
                            } else {
                                button.y = anim.target_y;
                                button.x = anim.source_x + ((anim.target_x - anim.source_x) as f32 * secondhalf) as i32;
                            }
                        }
                    }
                    _ => {
                        if anim.d_x != 0.0 || anim.d_y != 0.0 {
                            button.x = (anim.source_x as f32 + elapsed * anim.d_x) as i32;
                            button.y = (anim.source_y as f32 + elapsed * anim.d_y) as i32;
                        }
                    }
                }
                if anim.d_w != 0.0 || anim.d_h != 0.0 {
                    button.width = (anim.source_w as f32 + elapsed * anim.d_w) as i32;
                    button.height = (anim.source_h as f32 + elapsed * anim.d_h) as i32;
                }
                if anim.d_c != 0.0 {
                    if let Some(tc) = &anim.target_c {
                        let showlength = (elapsed * anim.d_c) as usize;
                        let caption: String = tc.chars().take(showlength).collect();
                        button.set_caption(&caption);
                    }
                }
                if g.ecl.is_super_highlighted(&anim.buttonname) {
                    g.ecl.update_super_highlights(&anim.buttonname);
                }
            }
        }
    }

    // ------------------------------------------------------------------ callbacks into the game

    /// `EclDrawAllButtons`. Redraws everything (back to front) every frame.
    pub fn draw_all_buttons(g: &mut Ctx) {
        if let Some(clear) = g.ecl.clear_draw {
            let rects = std::mem::take(&mut g.ecl.dirtyrectangles);
            for (x, y, w, h) in rects {
                clear(g, x, y, w, h);
            }
        }
        // PORT: the C++ only redrew dirty buttons over the last frame. A super highlight is sent to the
        // back but dirtied every frame, while the panels behind it are not, so it showed on top of
        // them. Every button is redrawn every frame here, so each super highlight is drawn just
        // before the button it surrounds instead of at the back, where the panels would cover it.
        let snapshot: Vec<Button> = g.ecl.buttons.iter().rev().filter(|b| b.x >= 0 && b.y >= 0).cloned().collect();
        for b in snapshot {
            // The button may have been removed by an earlier draw callback.
            if g.ecl.get_button(&b.name).is_none() || b.name.starts_with("Ecl_superhighlight ") {
                continue;
            }
            if g.ecl.is_super_highlighted(&b.name) {
                if let Some(h) = g.ecl.get_button(&superhighlight_name(&b.name)).filter(|h| h.x >= 0 && h.y >= 0).cloned() {
                    Self::draw_button_snapshot(g, &h);
                }
            }
            Self::draw_button_snapshot(g, &b);
        }
    }

    /// `EclDrawButton(name)`
    pub fn draw_button(g: &mut Ctx, name: &str) {
        if let Some(b) = g.ecl.get_button(name).cloned() {
            Self::draw_button_snapshot(g, &b);
        }
    }

    fn draw_button_snapshot(g: &mut Ctx, b: &Button) {
        let Some(draw) = b.draw else { return };
        if g.ecl.is_clicked(&b.name) {
            draw(g, b, false, true);
        } else if g.ecl.is_highlighted(&b.name) {
            draw(g, b, true, false);
        } else {
            draw(g, b, false, false);
        }
    }

    /// `Button::MouseUp()` for the named button.
    pub fn button_mouse_up(g: &mut Ctx, name: &str) {
        if let Some(b) = g.ecl.get_button(name).cloned() {
            if let Some(f) = b.mouseup {
                f(g, &b);
            }
        }
    }

    /// `Button::MouseDown()` for the named button.
    pub fn button_mouse_down(g: &mut Ctx, name: &str) {
        if let Some(b) = g.ecl.get_button(name).cloned() {
            if let Some(f) = b.mousedown {
                f(g, &b);
            }
        }
    }

    /// `Button::MouseMove()` for the named button.
    pub fn button_mouse_move(g: &mut Ctx, name: &str) {
        if let Some(b) = g.ecl.get_button(name).cloned() {
            if let Some(f) = b.mousemove {
                f(g, &b);
            }
        }
    }

    pub fn debug_print(&self) {
        println!("============== E C L I P S E ===============================");
        for b in &self.buttons {
            b.debug_print();
        }
        println!("Current Highlight = {:?}", self.currenthighlight);
        println!("Current Click     = {:?}", self.currentclick);
        for (i, a) in self.anims.iter() {
            println!("ANIM {i}: {a:?}");
        }
        println!("EDITABLE BUTTONS : {:?}", self.editablebuttons);
        println!("============== E N D  O F  E C L I P S E ===================");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn front_to_back_order_and_hit_test() {
        let mut e = Eclipse::new();
        e.register_button(0, 0, 100, 100, "", "", "back");
        e.register_button(10, 10, 20, 20, "", "", "front");
        assert_eq!(e.get_button_at_coord(15, 15).as_deref(), Some("front"));
        e.button_send_to_back("front");
        assert_eq!(e.get_button_at_coord(15, 15).as_deref(), Some("back"));
        e.remove_button("back");
        assert_eq!(e.get_button_at_coord(15, 15).as_deref(), Some("front"));
        assert_eq!(e.get_button_at_coord(500, 500), None);
    }

    #[test]
    fn caption_change_replaces_previous() {
        let mut e = Eclipse::new();
        e.register_button(0, 0, 10, 10, "", "", "b");
        let a = e.register_caption_change("b", "hello", 1000, None);
        let b = e.register_caption_change("b", "world", 1000, None);
        assert_eq!(a, b, "old caption animation removed so its slot is reused");
        assert_eq!(e.is_caption_change_active("b"), b);
    }
}
