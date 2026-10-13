//! Port of eclipse's `Button`.

use std::rc::Rc;

use uplink_engine::image::Image;

use crate::ctx::Ctx;

/// Draws a button: `(ctx, button snapshot, highlighted, clicked)`.
pub type DrawFn = fn(&mut Ctx, &Button, bool, bool);
/// Mouse up / down / move callback.
pub type MouseFn = fn(&mut Ctx, &Button);
/// Called when an animation finishes.
pub type AnimCallback = fn(&mut Ctx);
/// Clears a screen rectangle `(x, y, w, h)` before buttons are drawn.
pub type ClearDrawFn = fn(&mut Ctx, i32, i32, i32, i32);

#[derive(Clone, Default)]
pub struct Button {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub caption: String,
    pub name: String,
    pub tooltip: String,
    /// C++ `dirty` (renamed: `dirty()` is the method that sets it).
    pub is_dirty: bool,
    pub image_standard: Option<Rc<Image>>,
    pub image_highlighted: Option<Rc<Image>>,
    pub image_clicked: Option<Rc<Image>>,
    pub draw: Option<DrawFn>,
    pub mousedown: Option<MouseFn>,
    pub mouseup: Option<MouseFn>,
    pub mousemove: Option<MouseFn>,
}

impl std::fmt::Debug for Button {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Button({:?} at {},{} {}x{} caption={:?})", self.name, self.x, self.y, self.width, self.height, self.caption)
    }
}

impl Button {
    pub fn new(x: i32, y: i32, width: i32, height: i32, caption: &str, name: &str) -> Self {
        let mut b = Button::default();
        b.set_properties(x, y, width, height, caption, name);
        b
    }

    pub fn set_properties(&mut self, x: i32, y: i32, width: i32, height: i32, caption: &str, name: &str) {
        self.x = x;
        self.y = y;
        self.width = width;
        self.height = height;
        self.caption = caption.to_string();
        self.name = name.to_string();
        self.is_dirty = true;
    }

    pub fn set_caption(&mut self, newcaption: &str) {
        if self.caption != newcaption {
            self.dirty();
        }
        self.caption = newcaption.to_string();
    }

    pub fn set_tooltip(&mut self, newtooltip: &str) {
        self.tooltip = newtooltip.to_string();
    }

    pub fn set_standard_image(&mut self, image: Option<Rc<Image>>) {
        self.image_standard = image;
    }

    pub fn set_images(&mut self, standard: Option<Rc<Image>>, highlighted: Option<Rc<Image>>, clicked: Option<Rc<Image>>) {
        self.image_standard = standard;
        self.image_highlighted = highlighted;
        self.image_clicked = clicked;
    }

    pub fn register_draw_function(&mut self, f: Option<DrawFn>) {
        self.draw = f;
    }
    pub fn register_mouse_up_function(&mut self, f: Option<MouseFn>) {
        self.mouseup = f;
    }
    pub fn register_mouse_down_function(&mut self, f: Option<MouseFn>) {
        self.mousedown = f;
    }
    pub fn register_mouse_move_function(&mut self, f: Option<MouseFn>) {
        self.mousemove = f;
    }

    pub fn dirty(&mut self) {
        self.is_dirty = true;
    }

    pub fn debug_print(&self) {
        println!("{self:?} tooltip={:?}", self.tooltip);
    }
}
