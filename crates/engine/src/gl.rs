//! A tiny emulation of the fixed-function OpenGL 1.x immediate mode that Uplink's
//! interface code was written against, implemented on top of macroquad.
//!
//! The goal is that C++ like
//! ```text
//! glColor4f ( 0.2, 0.2, 0.6, ALPHA );
//! glBegin ( GL_QUADS );
//!     glVertex2i ( x, y ); ...
//! glEnd ();
//! ```
//! ports line-for-line to
//! ```ignore
//! gl::color4f(0.2, 0.2, 0.6, ALPHA);
//! gl::begin(gl::QUADS);
//!     gl::vertex2i(x, y); ...
//! gl::end();
//! ```
//!
//! Coordinates are in *virtual screen pixels*: (0,0) top-left, y down, size set by
//! [`set_virtual_size`] (the game's configured resolution). The virtual screen is
//! stretched to the real window. `scissor` uses GL conventions (origin bottom-left)
//! exactly like `glScissor`, because the C++ code computes its arguments that way.

use std::cell::RefCell;

use macroquad::prelude::*;

use crate::image::Image;

pub type Prim = u32;
pub const POINTS: Prim = 0x0000;
pub const LINES: Prim = 0x0001;
pub const LINE_LOOP: Prim = 0x0002;
pub const LINE_STRIP: Prim = 0x0003;
pub const TRIANGLES: Prim = 0x0004;
pub const TRIANGLE_STRIP: Prim = 0x0005;
pub const TRIANGLE_FAN: Prim = 0x0006;
pub const QUADS: Prim = 0x0007;
pub const POLYGON: Prim = 0x0009;

pub type Cap = u32;
pub const SCISSOR_TEST: Cap = 0x0C11;
pub const LINE_STIPPLE: Cap = 0x0B24;
pub const BLEND: Cap = 0x0BE2;
pub const TEXTURE_2D: Cap = 0x0DE1;
pub const DEPTH_TEST: Cap = 0x0B71;
pub const LINE_SMOOTH: Cap = 0x0B20;

#[derive(Clone, Copy)]
struct V {
    x: f32,
    y: f32,
    u: f32,
    v: f32,
    c: Color,
}

struct State {
    vw: f32,
    vh: f32,
    prim: Option<Prim>,
    verts: Vec<V>,
    colour: Color,
    texcoord: (f32, f32),
    line_width: f32,
    stipple: Option<(i32, u16)>,
    stipple_enabled: bool,
    scissor_enabled: bool,
    scissor: (i32, i32, i32, i32),
    texture_enabled: bool,
    texture: Option<Texture2D>,
    translate: Vec<(f32, f32)>,
    offset: (f32, f32),
}

impl Default for State {
    fn default() -> Self {
        State {
            vw: 1024.0,
            vh: 768.0,
            prim: None,
            verts: Vec::new(),
            colour: WHITE,
            texcoord: (0.0, 0.0),
            line_width: 1.0,
            stipple: None,
            stipple_enabled: false,
            scissor_enabled: false,
            scissor: (0, 0, 0, 0),
            texture_enabled: false,
            texture: None,
            translate: Vec::new(),
            offset: (0.0, 0.0),
        }
    }
}

thread_local! {
    static GL: RefCell<State> = RefCell::new(State::default());
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    GL.with(|s| f(&mut s.borrow_mut()))
}

// ---------------------------------------------------------------- screen setup

/// Set the size of the virtual screen (the game's configured resolution).
pub fn set_virtual_size(width: i32, height: i32) {
    with(|s| {
        s.vw = width as f32;
        s.vh = height as f32;
    });
}

pub fn virtual_width() -> i32 {
    with(|s| s.vw as i32)
}

pub fn virtual_height() -> i32 {
    with(|s| s.vh as i32)
}

/// Call once at the start of every frame, before any drawing.
/// Sets up the camera that maps virtual pixels onto the window and clears the screen.
pub fn begin_frame(clear: Color) {
    let (vw, vh) = with(|s| {
        s.scissor_enabled = false;
        s.translate.clear();
        s.offset = (0.0, 0.0);
        (s.vw, s.vh)
    });
    clear_background(clear);
    let cam = Camera2D::from_display_rect(Rect::new(0.0, 0.0, vw, vh));
    let mut cam = cam;
    // from_display_rect has a negative y zoom (right for render targets), which draws the window
    // upside down; a positive y zoom makes virtual y grow downwards on screen.
    cam.zoom.y = cam.zoom.y.abs();
    set_camera(&cam);
    apply_scissor();
}

/// Convert a window-space mouse position into virtual screen pixels.
pub fn mouse_virtual() -> (i32, i32) {
    let (mx, my) = mouse_position();
    let (vw, vh) = with(|s| (s.vw, s.vh));
    let sx = vw / screen_width().max(1.0);
    let sy = vh / screen_height().max(1.0);
    ((mx * sx) as i32, (my * sy) as i32)
}

// ---------------------------------------------------------------- state

pub fn enable(cap: Cap) {
    match cap {
        SCISSOR_TEST => {
            with(|s| s.scissor_enabled = true);
            apply_scissor();
        }
        LINE_STIPPLE => with(|s| s.stipple_enabled = true),
        TEXTURE_2D => with(|s| s.texture_enabled = true),
        _ => {}
    }
}

pub fn disable(cap: Cap) {
    match cap {
        SCISSOR_TEST => {
            with(|s| s.scissor_enabled = false);
            apply_scissor();
        }
        LINE_STIPPLE => with(|s| s.stipple_enabled = false),
        TEXTURE_2D => with(|s| s.texture_enabled = false),
        _ => {}
    }
}

pub fn is_enabled(cap: Cap) -> bool {
    with(|s| match cap {
        SCISSOR_TEST => s.scissor_enabled,
        LINE_STIPPLE => s.stipple_enabled,
        TEXTURE_2D => s.texture_enabled,
        _ => false,
    })
}

/// `glScissor`: x, y measured from the BOTTOM-left of the virtual screen, like GL.
pub fn scissor(x: i32, y: i32, w: i32, h: i32) {
    with(|s| s.scissor = (x, y, w, h));
    apply_scissor();
}

fn apply_scissor() {
    flush_quad_gl();
    let (enabled, (x, y, w, h), vw, vh) = with(|s| (s.scissor_enabled, s.scissor, s.vw, s.vh));
    let gl = unsafe { get_internal_gl() };
    if !enabled {
        gl.quad_gl.scissor(None);
        return;
    }
    // GL scissor boxes start at the bottom-left; quad_gl's start at the top-left (checked against an
    // X capture of the window), so flip y, then scale to window pixels.
    let dpi = screen_dpi_scale();
    let sx = screen_width() * dpi / vw;
    let sy = screen_height() * dpi / vh;
    let top = vh as i32 - y - h.max(0);
    gl.quad_gl.scissor(Some((
        (x as f32 * sx) as i32,
        (top as f32 * sy) as i32,
        (w.max(0) as f32 * sx) as i32,
        (h.max(0) as f32 * sy) as i32,
    )));
}

fn flush_quad_gl() {
    // macroquad batches draw calls; scissor changes apply per draw call, so nothing to do.
}

pub fn line_width(w: f32) {
    with(|s| s.line_width = w.max(1.0));
}

/// `glLineStipple(factor, pattern)`
pub fn line_stipple(factor: i32, pattern: u16) {
    with(|s| s.stipple = Some((factor.max(1), pattern)));
}

pub fn color3ub(r: u8, g: u8, b: u8) {
    with(|s| s.colour = Color::from_rgba(r, g, b, 255));
}
pub fn color4ub(r: u8, g: u8, b: u8, a: u8) {
    with(|s| s.colour = Color::from_rgba(r, g, b, a));
}
pub fn color3f(r: f32, g: f32, b: f32) {
    with(|s| s.colour = Color::new(r, g, b, 1.0));
}
pub fn color4f(r: f32, g: f32, b: f32, a: f32) {
    with(|s| s.colour = Color::new(r, g, b, a));
}
pub fn color3d(r: f64, g: f64, b: f64) {
    color3f(r as f32, g as f32, b as f32);
}
pub fn color(c: Color) {
    with(|s| s.colour = c);
}
pub fn current_colour() -> Color {
    with(|s| s.colour)
}

/// `glPushMatrix` + `glTranslatef` replacement: offsets every following vertex.
pub fn push_translate(dx: f32, dy: f32) {
    with(|s| {
        s.translate.push(s.offset);
        s.offset = (s.offset.0 + dx, s.offset.1 + dy);
    });
}
/// `glPopMatrix` counterpart of [`push_translate`].
pub fn pop_translate() {
    with(|s| {
        if let Some(o) = s.translate.pop() {
            s.offset = o;
        }
    });
}

/// `glBindTexture`: the image whose texture is used by textured primitives.
pub fn bind_texture(image: Option<&Image>) {
    let tex = image.map(|i| i.texture());
    with(|s| s.texture = tex);
}

/// GL texture coordinates: t = 0 is the image's bottom row (images are uploaded bottom-up in GL).
/// Textures here are top-down, so t is flipped.
pub fn tex_coord2f(u: f32, v: f32) {
    with(|s| s.texcoord = (u, 1.0 - v));
}

// ---------------------------------------------------------------- begin / end

pub fn begin(prim: Prim) {
    with(|s| {
        s.prim = Some(prim);
        s.verts.clear();
    });
}

pub fn vertex2f(x: f32, y: f32) {
    with(|s| {
        let v = V { x: x + s.offset.0, y: y + s.offset.1, u: s.texcoord.0, v: s.texcoord.1, c: s.colour };
        s.verts.push(v);
    });
}
pub fn vertex2i(x: i32, y: i32) {
    vertex2f(x as f32, y as f32);
}
pub fn vertex2d(x: f64, y: f64) {
    vertex2f(x as f32, y as f32);
}
pub fn vertex3i(x: i32, y: i32, _z: i32) {
    vertex2f(x as f32, y as f32);
}

pub fn end() {
    let (prim, verts, width, stipple, textured) = with(|s| {
        let tex = if s.texture_enabled { s.texture.clone() } else { None };
        (
            s.prim.take(),
            std::mem::take(&mut s.verts),
            s.line_width,
            if s.stipple_enabled { s.stipple } else { None },
            tex,
        )
    });
    let Some(prim) = prim else { return };
    match prim {
        QUADS => {
            for q in verts.chunks_exact(4) {
                fill(&[q[0], q[1], q[2], q[3]], &[0, 1, 2, 0, 2, 3], textured.as_ref());
            }
        }
        TRIANGLES => {
            for t in verts.chunks_exact(3) {
                fill(t, &[0, 1, 2], textured.as_ref());
            }
        }
        TRIANGLE_FAN | POLYGON => {
            if verts.len() >= 3 {
                let mut idx = Vec::new();
                for i in 1..verts.len() as u16 - 1 {
                    idx.extend_from_slice(&[0, i, i + 1]);
                }
                fill(&verts, &idx, textured.as_ref());
            }
        }
        TRIANGLE_STRIP => {
            let mut idx = Vec::new();
            for i in 0..verts.len().saturating_sub(2) as u16 {
                idx.extend_from_slice(&[i, i + 1, i + 2]);
            }
            fill(&verts, &idx, textured.as_ref());
        }
        LINES => {
            for l in verts.chunks_exact(2) {
                line(l[0], l[1], width, stipple);
            }
        }
        LINE_STRIP | LINE_LOOP => {
            for w in verts.windows(2) {
                line(w[0], w[1], width, stipple);
            }
            if prim == LINE_LOOP && verts.len() > 2 {
                line(verts[verts.len() - 1], verts[0], width, stipple);
            }
        }
        POINTS => {
            for v in verts {
                draw_rectangle(v.x, v.y, width, width, v.c);
            }
        }
        _ => {}
    }
}

/// `glRasterPos2f(x, y)` for the text code: the raster position in virtual pixels (top-down) after
/// the current translate, or `None` when GL would mark it invalid (outside the window), in which
/// case the following glBitmap calls draw nothing.
pub(crate) fn raster_pos(x: f32, y: f32) -> Option<(f32, f32)> {
    with(|s| {
        let (rx, ry) = (x + s.offset.0, y + s.offset.1);
        (rx >= 0.0 && rx <= s.vw && ry >= 0.0 && ry <= s.vh).then_some((rx, ry))
    })
}

/// glBitmap stand-in for the text code: draw the `(u, v, w, h)` texel rectangle of `atlas` with
/// its top-left at virtual pixel `(x, y)` (no translate applied), one texel per pixel, in the
/// current colour.
pub(crate) fn draw_bitmap(atlas: &Texture2D, x: i32, y: i32, src: (i32, i32, i32, i32)) {
    let c = current_colour();
    let (aw, ah) = (atlas.width(), atlas.height());
    let (u, v, w, h) = (src.0 as f32, src.1 as f32, src.2 as f32, src.3 as f32);
    let (x, y) = (x as f32, y as f32);
    let q = |px: f32, py: f32, tu: f32, tv: f32| V { x: px, y: py, u: tu / aw, v: tv / ah, c };
    let quad = [q(x, y, u, v), q(x + w, y, u + w, v), q(x + w, y + h, u + w, v + h), q(x, y + h, u, v + h)];
    fill(&quad, &[0, 1, 2, 0, 2, 3], Some(atlas));
}

fn fill(verts: &[V], indices: &[u16], texture: Option<&Texture2D>) {
    let gl = unsafe { get_internal_gl() };
    gl.quad_gl.texture(texture);
    gl.quad_gl.draw_mode(DrawMode::Triangles);
    let vs: Vec<Vertex> = verts.iter().map(|v| Vertex::new(v.x, v.y, 0.0, v.u, v.v, v.c)).collect();
    gl.quad_gl.geometry(&vs, indices);
}

fn line(a: V, b: V, width: f32, stipple: Option<(i32, u16)>) {
    match stipple {
        None => solid_line(a, b, width),
        Some((factor, pattern)) => {
            let len = ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt();
            if len < 0.5 {
                return;
            }
            let step = factor as f32;
            let mut t = 0.0f32;
            let mut bit = 0u32;
            while t < len {
                let t2 = (t + step).min(len);
                if pattern & (1 << (bit % 16)) != 0 {
                    let p = lerp(a, b, t / len);
                    let q = lerp(a, b, t2 / len);
                    solid_line(p, q, width);
                }
                t = t2;
                bit += 1;
            }
        }
    }
}

fn lerp(a: V, b: V, t: f32) -> V {
    V {
        x: a.x + (b.x - a.x) * t,
        y: a.y + (b.y - a.y) * t,
        u: a.u,
        v: a.v,
        c: Color::new(
            a.c.r + (b.c.r - a.c.r) * t,
            a.c.g + (b.c.g - a.c.g) * t,
            a.c.b + (b.c.b - a.c.b) * t,
            a.c.a + (b.c.a - a.c.a) * t,
        ),
    }
}

fn solid_line(a: V, b: V, width: f32) {
    // Pixel-centre the line like GL's diamond-exit rule does for 1px lines.
    let (ax, ay, bx, by) = (a.x + 0.5, a.y + 0.5, b.x + 0.5, b.y + 0.5);
    let dx = bx - ax;
    let dy = by - ay;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 0.001 {
        draw_rectangle(a.x, a.y, width, width, a.c);
        return;
    }
    let (nx, ny) = (-dy / len * width * 0.5, dx / len * width * 0.5);
    // extend half a pixel at both ends so axis-aligned boxes close their corners
    let (ex, ey) = (dx / len * 0.5, dy / len * 0.5);
    let p = [
        V { x: ax - ex + nx, y: ay - ey + ny, ..a },
        V { x: bx + ex + nx, y: by + ey + ny, ..b },
        V { x: bx + ex - nx, y: by + ey - ny, ..b },
        V { x: ax - ex - nx, y: ay - ey - ny, ..a },
    ];
    fill(&p, &[0, 1, 2, 0, 2, 3], None);
}

// ---------------------------------------------------------------- helpers

/// Convenience: filled rectangle in the current colour.
pub fn rect(x: i32, y: i32, w: i32, h: i32) {
    begin(QUADS);
    vertex2i(x, y);
    vertex2i(x + w, y);
    vertex2i(x + w, y + h);
    vertex2i(x, y + h);
    end();
}

/// Convenience: rectangle outline in the current colour.
pub fn rect_outline(x: i32, y: i32, w: i32, h: i32) {
    begin(LINE_LOOP);
    vertex2i(x, y);
    vertex2i(x + w, y);
    vertex2i(x + w, y + h);
    vertex2i(x, y + h);
    end();
}

/// Save the current window contents as a PNG (top-down, unlike raw GL readback).
pub fn screenshot(path: &str) {
    // get_screen_data already returns rows top-down (checked against an X capture of the window).
    get_screen_data().export_png(path);
}
