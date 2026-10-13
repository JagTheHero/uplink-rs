//! Port of source/interface/scrollbox.h + source/interface/scrollbox.cpp
#![allow(unused_variables, unused_imports, unused_mut, dead_code)]

use uplink_engine::gl;

use crate::app::opengl::get_mouse_y;
use crate::app::opengl_interface::{
    border_draw, button_assignbitmaps, button_click, button_highlight, set_colour,
};
use crate::ctx::Ctx;
use crate::eclipse::button::Button;

/// Callback when the scroll value changes: `(ctx, scrollbox name, new current index)`.
// PORT: C++ was `void (*)(char *, int)`; it needs `g` because the callbacks redraw game UI.
pub type ScrollCallback = fn(&mut Ctx, &str, i32);

/// C++ float-to-int conversion as done on x86: truncates, and NaN or out-of-range gives `i32::MIN`.
fn f32_to_i32(v: f32) -> i32 {
    if v.is_nan() || v >= 2147483648.0 || v < -2147483648.0 {
        i32::MIN
    } else {
        v as i32
    }
}

/// `sscanf(button->name, "%s %s", unused, name)`: the scroll box name after the button's first word.
fn button_scroll_name(button: &Button) -> Option<String> {
    button.name.split_whitespace().nth(1).map(str::to_string)
}

/// The slider's `y` and unclamped `h` in screen pixels, with the C++ float-to-int truncation.
fn slider_geometry(sb: &ScrollBox) -> (i32, i32) {
    let y = f32_to_i32(
        (sb.y + 15) as f32 + (sb.current_index as f32 / sb.num_items as f32) * (sb.h - 30) as f32,
    );
    let h = f32_to_i32((sb.window_size as f32 / sb.num_items as f32) * (sb.h - 30) as f32);
    (y, h)
}

/// A vertical scroll bar. All scroll boxes in existence live in `Interface::scrollboxes`;
/// instance methods that touch the UI are associated fns taking `(g, name)`.
#[derive(Debug, Clone)]
pub struct ScrollBox {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub num_items: i32,
    pub window_size: i32,
    pub current_index: i32,
    callback: Option<ScrollCallback>,
}

impl Default for ScrollBox {
    fn default() -> Self {
        ScrollBox {
            name: "NewScrollBox".to_string(),
            x: 0,
            y: 0,
            w: 0,
            h: 0,
            num_items: 0,
            window_size: 0,
            current_index: 0,
            callback: None,
        }
    }
}

impl ScrollBox {
    pub fn new() -> Self {
        ScrollBox::default()
    }

    pub fn set_name(&mut self, newname: &str) {
        self.name = newname.to_string();
    }

    pub fn set_dimensions(&mut self, newx: i32, newy: i32, neww: i32, newh: i32) {
        self.x = newx;
        self.y = newy;
        self.w = neww;
        self.h = newh;
    }

    pub fn set_callback(&mut self, newcallback: Option<ScrollCallback>) {
        self.callback = newcallback;
    }

    /// `SetNumItems`: clamps the index (calling the callback if it changed) and redraws.
    pub fn set_num_items(g: &mut Ctx, name: &str, new_num_items: i32) {
        let Some(sb) = Self::get_scroll_box_mut(g, name) else {
            return;
        };
        sb.num_items = new_num_items;

        let mut clamped = None;
        if sb.current_index > sb.num_items - sb.window_size {
            sb.current_index = (sb.num_items - sb.window_size).max(0);
            clamped = sb.callback.map(|cb| (cb, sb.current_index));
        }
        if let Some((cb, index)) = clamped {
            cb(g, name, index);
        }

        Self::update_interface(g, name);
    }

    /// `SetWindowSize`: stores and redraws.
    pub fn set_window_size(g: &mut Ctx, name: &str, new_window_size: i32) {
        let Some(sb) = Self::get_scroll_box_mut(g, name) else {
            return;
        };
        sb.window_size = new_window_size;
        Self::update_interface(g, name);
    }

    /// `SetCurrentIndex`: stores and redraws (does not call the callback).
    pub fn set_current_index(g: &mut Ctx, name: &str, new_current_index: i32) {
        let Some(sb) = Self::get_scroll_box_mut(g, name) else {
            return;
        };
        sb.current_index = new_current_index;
        Self::update_interface(g, name);
    }

    // --- Interface related stuff (button names: "scrollup NAME", "scrollbar NAME", "scrolldown NAME") ---

    pub fn create_interface(g: &mut Ctx, name: &str) {
        let Some(sb) = Self::get_scroll_box(g, name) else {
            return;
        };
        let (x, y, h) = (sb.x, sb.y, sb.h);

        let up_name = format!("scrollup {name}");
        let down_name = format!("scrolldown {name}");
        let bar_name = format!("scrollbar {name}");

        g.ecl.register_button(x, y, 15, 15, "^", "", &up_name);
        g.ecl
            .register_button(x, y + 15, 15, h - 30, " ", "", &bar_name);
        g.ecl
            .register_button(x, (y + h) - 15, 15, 15, "v", "", &down_name);

        button_assignbitmaps(g, &up_name, "up.tif", "up_h.tif", "up_c.tif");
        button_assignbitmaps(g, &down_name, "down.tif", "down_h.tif", "down_c.tif");

        g.ecl
            .register_button_callback(&up_name, Self::click_scroll_up);
        g.ecl
            .register_button_callback(&down_name, Self::click_scroll_down);
        g.ecl.register_button_callbacks(
            &bar_name,
            Some(Self::draw_scroll_bar),
            Some(Self::click_scroll_bar),
            Some(Self::mouse_down_scroll_bar),
            Some(Self::mouse_move_scroll_bar),
        );
    }

    pub fn remove_interface(g: &mut Ctx, name: &str) {
        g.ecl.remove_button(&format!("scrollup {name}"));
        g.ecl.remove_button(&format!("scrollbar {name}"));
        g.ecl.remove_button(&format!("scrolldown {name}"));
    }

    /// `UpdateInterface`: dirties the scroll bar button.
    pub fn update_interface(g: &mut Ctx, name: &str) {
        g.ecl.dirty_button(&format!("scrollbar {name}"));
    }

    pub fn is_interface_visible(&self) -> bool {
        true
    }

    pub fn scroll_up(g: &mut Ctx, name: &str) {
        let Some(sb) = Self::get_scroll_box_mut(g, name) else {
            return;
        };
        if sb.current_index > 0 {
            sb.current_index -= 1;
            let (callback, index) = (sb.callback, sb.current_index);
            Self::update_interface(g, name);
            if let Some(cb) = callback {
                cb(g, name, index);
            }
        }
    }

    pub fn scroll_down(g: &mut Ctx, name: &str) {
        let Some(sb) = Self::get_scroll_box_mut(g, name) else {
            return;
        };
        if sb.current_index < sb.num_items - sb.window_size {
            sb.current_index += 1;
            let (callback, index) = (sb.callback, sb.current_index);
            Self::update_interface(g, name);
            if let Some(cb) = callback {
                cb(g, name, index);
            }
        }
    }

    // --- Eclipse callbacks ---

    pub fn click_scroll_up(g: &mut Ctx, button: &Button) {
        if let Some(name) = button_scroll_name(button) {
            Self::scroll_up(g, &name);
        }
    }

    pub fn click_scroll_down(g: &mut Ctx, button: &Button) {
        if let Some(name) = button_scroll_name(button) {
            Self::scroll_down(g, &name);
        }
    }

    pub fn click_scroll_bar(g: &mut Ctx, button: &Button) {
        let Some(name) = button_scroll_name(button) else {
            return;
        };
        let mouse_y = get_mouse_y();
        let Some(sb) = Self::get_scroll_box_mut(g, &name) else {
            return;
        };

        let (y, h) = slider_geometry(sb);
        let moved = if mouse_y < y {
            sb.current_index = (sb.current_index - sb.window_size).max(0);
            true
        } else if mouse_y > y + h {
            sb.current_index += sb.window_size;
            if sb.current_index > sb.num_items - sb.window_size {
                sb.current_index = (sb.num_items - sb.window_size).max(0);
            }
            true
        } else {
            false
        };
        if !moved {
            return;
        }

        let (callback, index) = (sb.callback, sb.current_index);
        if let Some(cb) = callback {
            cb(g, &name, index);
        }
        Self::update_interface(g, &name);
    }

    pub fn mouse_down_scroll_bar(g: &mut Ctx, button: &Button) {
        button_click(g, button);
        if let Some(name) = button_scroll_name(button) {
            Self::grab_scroll_bar(g, &name);
        }
    }

    pub fn mouse_move_scroll_bar(g: &mut Ctx, button: &Button) {
        button_highlight(g, button);
    }

    pub fn draw_scroll_bar(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        // Draw the background
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

        // Draw the selecter, when the crowd say bo
        if let Some(name) = button_scroll_name(button) {
            if let Some(this_box) = Self::get_scroll_box(g, &name) {
                let (x, w) = (this_box.x, this_box.w);
                let (y, mut h) = slider_geometry(this_box);
                if h > button.height {
                    h = button.height;
                }

                gl::begin(gl::QUADS);
                set_colour(g, "ButtonNormalA");
                // PORT: with no items (0/0) y and h are both i32::MIN; C++ int addition wraps there.
                gl::vertex2i(x, y.wrapping_add(h));
                set_colour(g, "ButtonNormalB");
                gl::vertex2i(x, y);
                set_colour(g, "ButtonNormalA");
                gl::vertex2i(x + w, y);
                set_colour(g, "ButtonNormalB");
                gl::vertex2i(x + w, y.wrapping_add(h));
                gl::end();
            }
        }

        // Draw the border
        set_colour(g, "PanelBorder");
        border_draw(button);
    }

    // --- These manage the set of all scroll boxes in existence ---

    /// `CreateScrollBox` (callback default NULL = `None`)
    #[allow(clippy::too_many_arguments)]
    pub fn create_scroll_box(
        g: &mut Ctx,
        name: &str,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        num_items: i32,
        window_size: i32,
        current_index: i32,
        callback: Option<ScrollCallback>,
    ) {
        // The C++ setters run before the callback is set (so no callback fires) and before the
        // box is in the list (so no button is dirtied): assign the fields directly.
        let mut sb = ScrollBox::new();
        sb.set_name(name);
        sb.set_dimensions(x, y, w, h);
        sb.num_items = num_items;
        sb.window_size = window_size;
        sb.current_index = current_index;
        sb.set_callback(callback);
        g.interface_mut().scrollboxes.push(sb);

        Self::create_interface(g, name);
    }

    pub fn remove_scroll_box(g: &mut Ctx, name: &str) {
        if Self::get_scroll_box(g, name).is_none() {
            return;
        }
        Self::remove_interface(g, name);
        let scrollboxes = &mut g.interface_mut().scrollboxes;
        if let Some(i) = scrollboxes.iter().position(|sb| sb.name == name) {
            scrollboxes.remove(i);
        }
    }

    /// `GetScrollBox` (NULL = `None`)
    pub fn get_scroll_box<'a>(g: &'a Ctx, name: &str) -> Option<&'a ScrollBox> {
        g.interface().scrollboxes.iter().find(|sb| sb.name == name)
    }

    pub fn get_scroll_box_mut<'a>(g: &'a mut Ctx, name: &str) -> Option<&'a mut ScrollBox> {
        g.interface_mut()
            .scrollboxes
            .iter_mut()
            .find(|sb| sb.name == name)
    }

    // --- Grabbing stuff ---

    /// `GrabScrollBar`
    pub fn grab_scroll_bar(g: &mut Ctx, name: &str) {
        g.interface_mut().scroll_grab = Some(name.to_string());
    }

    /// `UnGrabScrollBar` (also called when no game is running: then it does nothing)
    pub fn un_grab_scroll_bar(g: &mut Ctx) {
        if let Some(ui) = g.game.ui.as_deref_mut() {
            ui.scroll_grab = None;
        }
    }

    /// `IsGrabInProgress`
    pub fn is_grab_in_progress(g: &Ctx) -> bool {
        g.game
            .ui
            .as_deref()
            .is_some_and(|ui| ui.scroll_grab.is_some())
    }

    /// `UpdateGrabScroll`
    pub fn update_grab_scroll(g: &mut Ctx) {
        // The C++ grab state is a static, so this is also safe to call with no game running.
        let Some(name) = g.game.ui.as_deref().and_then(|ui| ui.scroll_grab.clone()) else {
            return;
        };
        let Some(this_box) = Self::get_scroll_box(g, &name) else {
            Self::un_grab_scroll_bar(g);
            return;
        };
        if this_box.window_size >= this_box.num_items {
            return;
        }

        let mouse_y = get_mouse_y();
        let mut fraction_through = (mouse_y - this_box.y) as f32 / this_box.h as f32;
        let mut bar_h = f32_to_i32(
            (this_box.window_size as f32 / this_box.num_items as f32) * (this_box.h - 30) as f32,
        );
        if bar_h > this_box.h {
            bar_h = this_box.h;
        }
        let bar_fraction = bar_h as f32 / this_box.h as f32;
        fraction_through -= bar_fraction / 2.0;

        if fraction_through < 0.0 {
            fraction_through = 0.0;
        }
        if fraction_through > 1.0 {
            fraction_through = 1.0;
        }

        let (num_items, window_size) = (this_box.num_items, this_box.window_size);
        let scaled = num_items as f32 * fraction_through;
        if scaled <= (num_items - window_size) as f32 {
            Self::set_current_index(g, &name, f32_to_i32(scaled));
        } else {
            Self::set_current_index(g, &name, num_items - window_size);
        }

        if let Some(this_box) = Self::get_scroll_box(g, &name) {
            let (callback, index) = (this_box.callback, this_box.current_index);
            if let Some(cb) = callback {
                cb(g, &name, index);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interface::interface::Interface;
    use crate::testutil::bare_ctx;

    thread_local! {
        static CALLS: std::cell::RefCell<Vec<i32>> = const { std::cell::RefCell::new(Vec::new()) };
    }

    fn record(_g: &mut Ctx, name: &str, index: i32) {
        assert_eq!(name, "list");
        CALLS.with(|c| c.borrow_mut().push(index));
    }

    fn calls() -> Vec<i32> {
        CALLS.with(|c| std::mem::take(&mut *c.borrow_mut()))
    }

    /// A context holding one scroll box "list": 10 items, 4 visible, at the top.
    fn ctx_with_box() -> Ctx {
        let mut g = bare_ctx();
        g.game.ui = Some(Box::new(Interface::new()));
        let mut sb = ScrollBox::new();
        sb.set_name("list");
        sb.set_dimensions(0, 0, 15, 100);
        sb.num_items = 10;
        sb.window_size = 4;
        sb.set_callback(Some(record));
        g.interface_mut().scrollboxes.push(sb);
        calls();
        g
    }

    fn index(g: &Ctx) -> i32 {
        ScrollBox::get_scroll_box(g, "list").unwrap().current_index
    }

    #[test]
    fn scrolling_stops_at_both_ends_and_reports_each_step() {
        let mut g = ctx_with_box();
        ScrollBox::scroll_up(&mut g, "list");
        assert_eq!(index(&g), 0);
        assert!(calls().is_empty());

        for _ in 0..20 {
            ScrollBox::scroll_down(&mut g, "list");
        }
        // 10 items, 4 visible: the last position that still fills the window is 6
        assert_eq!(index(&g), 6);
        assert_eq!(calls(), [1, 2, 3, 4, 5, 6]);

        ScrollBox::scroll_up(&mut g, "list");
        assert_eq!(index(&g), 5);
        assert_eq!(calls(), [5]);
    }

    #[test]
    fn shrinking_the_list_clamps_the_index_and_tells_the_owner() {
        let mut g = ctx_with_box();
        ScrollBox::set_current_index(&mut g, "list", 6);
        // setting the index directly does not call back
        assert!(calls().is_empty());

        ScrollBox::set_num_items(&mut g, "list", 8);
        assert_eq!(index(&g), 4);
        assert_eq!(calls(), [4]);

        // fewer items than the window: back to the top
        ScrollBox::set_num_items(&mut g, "list", 2);
        assert_eq!(index(&g), 0);
        assert_eq!(calls(), [0]);

        // growing the list leaves the index alone
        ScrollBox::set_num_items(&mut g, "list", 50);
        assert_eq!(index(&g), 0);
        assert!(calls().is_empty());
    }

    #[test]
    fn unknown_scroll_boxes_are_ignored() {
        let mut g = ctx_with_box();
        ScrollBox::scroll_down(&mut g, "nope");
        ScrollBox::set_num_items(&mut g, "nope", 3);
        ScrollBox::set_window_size(&mut g, "nope", 3);
        assert!(ScrollBox::get_scroll_box(&g, "nope").is_none());
        assert_eq!(index(&g), 0);
    }

    #[test]
    fn grabbing_a_bar_is_remembered() {
        let mut g = ctx_with_box();
        assert_eq!(g.interface().scroll_grab, None);
        ScrollBox::grab_scroll_bar(&mut g, "list");
        assert_eq!(g.interface().scroll_grab.as_deref(), Some("list"));
    }

    #[test]
    fn float_to_int_conversion_matches_x86() {
        assert_eq!(f32_to_i32(3.9), 3);
        assert_eq!(f32_to_i32(-3.9), -3);
        assert_eq!(f32_to_i32(f32::NAN), i32::MIN);
        assert_eq!(f32_to_i32(1e20), i32::MIN);
        assert_eq!(f32_to_i32(-1e20), i32::MIN);
    }

    #[test]
    fn the_slider_fills_the_bar_in_proportion_to_the_window() {
        let g = ctx_with_box();
        let sb = ScrollBox::get_scroll_box(&g, "list").unwrap();
        // bar runs from y+15 over h-30 = 70 px; 4 of 10 items visible
        assert_eq!(slider_geometry(sb), (15, 28));
    }

    #[test]
    fn the_scroll_name_is_the_second_word_of_the_button_name() {
        let mut b = Button::default();
        b.name = "scrollbar list".to_string();
        assert_eq!(button_scroll_name(&b).as_deref(), Some("list"));
        b.name = "scrollbar".to_string();
        assert_eq!(button_scroll_name(&b), None);
    }
}
