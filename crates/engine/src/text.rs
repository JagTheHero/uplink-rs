//! Text half of gucci: `GciDrawText`, `GciTextWidth`, font styles.
//!
//! Font style ids match the C++ `#define`s. Styles with a TrueType font loaded via
//! [`load_truetype_font`] draw it the way GLTT did: hinted, 1-bit glyphs ([`crate::mono`]) at
//! `int(size * 72 / 96 + 0.5)` points on a 96 dpi device, placed with glBitmap rules.
//!
//! Other styles (BITMAP_15 etc.) go through `GciFallbackDrawText` / `GciFallbackTextWidth`. The
//! SDL gucci (`gucci_sdl.cpp`, what the Linux makefile and the GOG 1.6 game use) redirects those to
//! the default font: the original draws the BITMAP_15 password stars in dungeon.ttf. Only when no
//! TrueType font is usable (where the C++ aborted) the real GLUT bitmap fonts
//! ([`crate::glut_fonts`], as `gucci_glut.cpp` would draw them) stand in.
//!
//! `(x, y)` is the raster position (`glRasterPos2i`): the baseline start. Glyphs are cached per
//! style in one texture atlas and drawn as textured quads in the current `gl` colour, through
//! the current translate and scissor.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use macroquad::prelude::*;

use crate::archive;
use crate::gl;
use crate::glut_fonts::{self, GlutFont};
use crate::mono::{MonoFont, MonoGlyph};

pub const BITMAP_13: i32 = 1;
pub const BITMAP_15: i32 = 2;
pub const TIMESROMAN_10: i32 = 3;
pub const TIMESROMAN_24: i32 = 4;
pub const HELVETICA_10: i32 = 5;
pub const HELVETICA_12: i32 = 6;
pub const HELVETICA_18: i32 = 7;

/// One glyph of a [`GlyphSet`]: where it sits in the atlas and how glBitmap would place it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Glyph {
    /// raster x advance
    adv: i32,
    /// first bitmap column relative to `floor(raster x)`
    x_off: i32,
    /// first (top) bitmap row relative to the raster row (`ceil(raster y) - 1`)
    y_off: i32,
    w: i32,
    h: i32,
    /// position in the atlas
    ax: i32,
    ay: i32,
}

/// `(advance, x_off, y_off, w, h, bits top-down)` of one glyph before packing.
type RawGlyph = (i32, i32, i32, i32, i32, Vec<u8>);

/// Characters 0..=255 of one font at one size, packed into a single texture.
struct GlyphSet {
    glyphs: Vec<Option<Glyph>>,
    atlas_w: i32,
    atlas_h: i32,
    /// RGBA, white with alpha = coverage (0 or 255)
    pixels: Vec<u8>,
    texture: RefCell<Option<Texture2D>>,
}

const ATLAS_W: i32 = 256;

impl GlyphSet {
    fn new(glyphs: Vec<Option<RawGlyph>>) -> GlyphSet {
        // shelf packing, 1 px gap so nearest sampling never bleeds
        let (mut x, mut y, mut row_h) = (0, 0, 0);
        let mut placed = Vec::with_capacity(glyphs.len());
        for g in &glyphs {
            placed.push(g.as_ref().map(|&(adv, x_off, y_off, w, h, _)| {
                if x + w > ATLAS_W {
                    x = 0;
                    y += row_h + 1;
                    row_h = 0;
                }
                let placed = Glyph { adv, x_off, y_off, w, h, ax: x, ay: y };
                x += w + 1;
                row_h = row_h.max(h);
                placed
            }));
        }
        let atlas_h = (y + row_h).max(1);
        let mut pixels = vec![0u8; (ATLAS_W * atlas_h * 4) as usize];
        for (g, src) in placed.iter().zip(&glyphs) {
            let (Some(g), Some((.., bits))) = (g, src) else { continue };
            for r in 0..g.h {
                for c in 0..g.w {
                    if bits[(r * g.w + c) as usize] != 0 {
                        let i = (((g.ay + r) * ATLAS_W + g.ax + c) * 4) as usize;
                        pixels[i..i + 4].copy_from_slice(&[255, 255, 255, 255]);
                    }
                }
            }
        }
        GlyphSet { glyphs: placed, atlas_w: ATLAS_W, atlas_h, pixels, texture: RefCell::new(None) }
    }

    fn from_truetype(font: &MonoFont) -> GlyphSet {
        let glyphs = (0..256u32)
            .map(|code| {
                let MonoGlyph { w, h, left, top, advance, bits } = font.glyph(char::from_u32(code)?)?;
                // glBitmap(w, h, -left, h - top, advance, 0, ...): the row just above the
                // baseline lands on the raster row
                Some((advance, left, 1 - top, w, h, bits))
            })
            .collect();
        GlyphSet::new(glyphs)
    }

    fn from_glut(font: &GlutFont) -> GlyphSet {
        let mut glyphs = Vec::with_capacity(256);
        let mut d = font.data;
        for _ in 0..256 {
            let (adv, bottom, rows) = (d[0] as i32, d[1] as i32, d[2] as i32);
            let bpr = ((adv + 7) / 8) as usize;
            let body = &d[3..3 + bpr * rows as usize];
            d = &d[3 + body.len()..];
            if adv == 0 && rows == 0 {
                glyphs.push(None);
                continue;
            }
            // rows are stored bottom-up; flip to top-down
            let mut bits = vec![0u8; (adv * rows) as usize];
            for r in 0..rows {
                let row = &body[(rows - 1 - r) as usize * bpr..][..bpr];
                for c in 0..adv {
                    bits[(r * adv + c) as usize] = (row[c as usize / 8] >> (7 - c % 8)) & 1;
                }
            }
            // glBitmap(adv, height, xorig, yorig, adv, 0, ...): cell row j (counted from the
            // bottom) lands on raster row + yorig - j
            let y_off = font.yorig as i32 - bottom - rows + 1;
            glyphs.push(Some((adv, -(font.xorig as i32), y_off, adv, rows, bits)));
        }
        GlyphSet::new(glyphs)
    }

    fn glyph(&self, ch: char) -> Option<&Glyph> {
        self.glyphs.get(ch as usize)?.as_ref()
    }

    fn width(&self, text: &str) -> i32 {
        text.chars().filter_map(|c| self.glyph(c)).map(|g| g.adv).sum()
    }

    fn texture(&self) -> Texture2D {
        self.texture
            .borrow_mut()
            .get_or_insert_with(|| {
                let t = Texture2D::from_rgba8(self.atlas_w as u16, self.atlas_h as u16, &self.pixels);
                t.set_filter(FilterMode::Nearest);
                t
            })
            .clone()
    }
}

#[derive(Default)]
struct TextState {
    default_style: i32,
    truetype_enabled: bool,
    fonts: HashMap<i32, Rc<GlyphSet>>,
    /// font name -> file contents (kept for the program's lifetime, like the C++ face registry)
    faces: HashMap<String, &'static [u8]>,
    glut: HashMap<i32, Rc<GlyphSet>>,
}

thread_local! {
    static TEXT: RefCell<TextState> = RefCell::new(TextState { default_style: HELVETICA_10, ..Default::default() });
}

/// `gucciStyleToGlut`
fn glut_font(style: i32) -> &'static GlutFont {
    match style {
        BITMAP_15 => &glut_fonts::FIXED_9X15,
        TIMESROMAN_10 => &glut_fonts::TIMES_ROMAN_10,
        TIMESROMAN_24 => &glut_fonts::TIMES_ROMAN_24,
        HELVETICA_10 => &glut_fonts::HELVETICA_10,
        HELVETICA_12 => &glut_fonts::HELVETICA_12,
        HELVETICA_18 => &glut_fonts::HELVETICA_18,
        _ => &glut_fonts::FIXED_8X13,
    }
}

fn glut_set(t: &mut TextState, style: i32) -> Rc<GlyphSet> {
    t.glut.entry(style).or_insert_with(|| Rc::new(GlyphSet::from_glut(glut_font(style)))).clone()
}

pub fn enable_truetype_support() {
    TEXT.with(|t| t.borrow_mut().truetype_enabled = true);
}
pub fn disable_truetype_support() {
    TEXT.with(|t| t.borrow_mut().truetype_enabled = false);
}

pub fn set_default_font(style: i32) {
    TEXT.with(|t| t.borrow_mut().default_style = style);
}

/// GLTT's size: `GciLoadTrueTypeFont` asks for `int(size * 72 / 96 + 0.5)` points at 96 dpi.
/// (dungeon.ttf sets head flag 3, so the hinter then rounds this to whole pixels: 9, 11, 16.)
fn pixels_per_em(size: i32) -> f32 {
    let points = (size as f64 * 72.0 / 96.0 + 0.5) as i32;
    (points as f64 * 96.0 / 72.0) as f32
}

/// `GciLoadTrueTypeFont(index, fontname, filename, size)`: `filename` is an archive path
/// (e.g. `"fonts/dungeon.ttf"`).
pub fn load_truetype_font(index: i32, fontname: &str, filename: &str, size: i32) -> bool {
    TEXT.with(|t| {
        let mut t = t.borrow_mut();
        let data = match t.faces.get(fontname) {
            Some(d) => *d,
            None => {
                let Some(bytes) = archive::read(filename) else { return false };
                let data: &'static [u8] = Box::leak(bytes.into_boxed_slice());
                t.faces.insert(fontname.to_string(), data);
                data
            }
        };
        let Some(font) = MonoFont::new(data, pixels_per_em(size)) else { return false };
        t.fonts.insert(index, Rc::new(GlyphSet::from_truetype(&font)));
        true
    })
}

pub fn delete_truetype_font(index: i32) {
    TEXT.with(|t| t.borrow_mut().fonts.remove(&index));
}

pub fn delete_all_truetype_fonts() {
    TEXT.with(|t| t.borrow_mut().fonts.clear());
}

/// `GciDrawText(x, y, text)` with the default style.
pub fn draw_text_default(x: i32, y: i32, text: &str) {
    let style = TEXT.with(|t| t.borrow().default_style);
    draw_text_styled(x, y, text, style);
}

/// `GciDrawText(x, y, text, STYLE)` in the current `gl` colour.
pub fn draw_text_styled(x: i32, y: i32, text: &str, style: i32) {
    if text.is_empty() {
        return;
    }
    // the style's TrueType font, else (GciFallbackDrawText) the default style's
    let set = TEXT.with(|t| {
        let mut t = t.borrow_mut();
        let truetype = if t.truetype_enabled {
            t.fonts.get(&style).or_else(|| t.fonts.get(&t.default_style)).cloned()
        } else {
            None
        };
        truetype.unwrap_or_else(|| glut_set(&mut t, style))
    });
    // glRasterPos2i(x, y): an invalid (off-window) raster position draws nothing
    let Some((mut rx, ry)) = gl::raster_pos(x as f32, y as f32) else { return };
    let tex = set.texture();
    for g in text.chars().filter_map(|c| set.glyph(c)) {
        if g.w > 0 && g.h > 0 {
            let px = rx.floor() as i32 + g.x_off;
            let py = ry.ceil() as i32 - 1 + g.y_off;
            gl::draw_bitmap(&tex, px, py, (g.ax, g.ay, g.w, g.h));
        }
        rx += g.adv as f32;
    }
}

/// `GciTextWidth(text, STYLE)` in virtual pixels: GLTT's sum of hinted advances of the style's
/// TrueType font (loaded, enabled or not, like the C++), else (GciFallbackTextWidth) the default
/// style's, else `glutBitmapLength`.
pub fn text_width(text: &str, style: i32) -> i32 {
    let set = TEXT.with(|t| {
        let mut t = t.borrow_mut();
        let truetype = t.fonts.get(&style).or_else(|| t.fonts.get(&t.default_style)).cloned();
        truetype.unwrap_or_else(|| glut_set(&mut t, style))
    });
    set.width(text)
}

/// `GciTextWidth(text)` with the default style.
pub fn text_width_default(text: &str) -> i32 {
    let style = TEXT.with(|t| t.borrow().default_style);
    text_width(text, style)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_dungeon() -> bool {
        let dir = archive::find_data_dir().filter(|d| d.join("fonts.dat").exists()).or_else(|| {
            let d = std::path::PathBuf::from(std::env::var("HOME").ok()?).join("Games/uplink-gog/app");
            d.join("fonts.dat").exists().then_some(d)
        });
        let Some(dir) = dir else { return false };
        archive::init(&dir);
        enable_truetype_support();
        load_truetype_font(HELVETICA_10, "Dungeon", "fonts/dungeon.ttf", 9)
            && load_truetype_font(HELVETICA_12, "Dungeon", "fonts/dungeon.ttf", 11)
            && load_truetype_font(HELVETICA_18, "Dungeon", "fonts/dungeon.ttf", 16)
    }

    /// Render `text` with `set` into ASCII rows (top-down) with the raster row at `base`.
    fn ascii(set: &GlyphSet, text: &str, rows: i32, base: i32) -> Vec<String> {
        let width = set.width(text) as usize + 4;
        let mut grid = vec![vec!['.'; width]; rows as usize];
        let mut x = 0;
        for ch in text.chars() {
            let Some(g) = set.glyph(ch) else { continue };
            for r in 0..g.h {
                for c in 0..g.w {
                    let i = (((g.ay + r) * set.atlas_w + g.ax + c) * 4 + 3) as usize;
                    if set.pixels[i] != 0 {
                        grid[(base + g.y_off + r) as usize][(x + g.x_off + c) as usize] = '#';
                    }
                }
            }
            x += g.adv;
        }
        grid.into_iter().map(|r| r.into_iter().collect::<String>().trim_end_matches('.').to_string()).collect()
    }

    /// Strip the empty rows and the common empty left margin, like the extracts below.
    fn trim_ink(rows: Vec<String>) -> Vec<String> {
        let inked: Vec<&String> = rows.iter().filter(|r| r.contains('#')).collect();
        let lead = inked.iter().map(|r| r.len() - r.trim_start_matches('.').len()).min().unwrap_or(0);
        let first = rows.iter().position(|r| r.contains('#')).unwrap_or(0);
        let last = rows.iter().rposition(|r| r.contains('#')).unwrap_or(0);
        rows[first..=last].iter().map(|r| r.get(lead..).unwrap_or("").to_string()).collect()
    }

    #[test]
    fn point_sizes_match_gltt() {
        assert_eq!(pixels_per_em(9), (7.0f64 * 96.0 / 72.0) as f32);
        assert_eq!(pixels_per_em(11), (8.0f64 * 96.0 / 72.0) as f32);
        assert_eq!(pixels_per_em(16), 16.0);
    }

    #[test]
    fn glut_9x15_metrics() {
        let set = GlyphSet::from_glut(&glut_fonts::FIXED_9X15);
        assert_eq!(set.width("Hello world"), 99);
        assert!(set.glyph('\n').is_none());
        // 'A' of 9x15: 10 rows, bottom row on the baseline (yorig 4 -> 4 rows of descent)
        let a = set.glyph('A').unwrap();
        assert_eq!((a.adv, a.h, a.y_off), (9, 10, -9));
    }

    /// Pixels and advances checked against the original game (GOG 1.6 under Wine): the
    /// "Welcome back to your Gateway" message after logging in (HELVETICA_10, raster row 220,
    /// screenshot rows 215..=222).
    #[test]
    fn dungeon_matches_original_pixels() {
        if !load_dungeon() {
            return;
        }
        let text = "Welcome back to your Gateway";
        assert_eq!(text_width(text, HELVETICA_10), 155);
        let set = TEXT.with(|t| t.borrow().fonts[&HELVETICA_10].clone());
        let rows = ascii(&set, text, 13, 8);
        let original = [
            "#...#...#......#...............................#................#.........#......................................####........#",
            ".#.#.#.#..###..#..###..###..####.###...###.....####..####...###.#...#....####..###.....#...#..###..#...#.###....#.....####..####..###.#...#...#.####..#...#",
            ".#.#.#.#.#...#.#.#....#...#.#...#...#.#...#....#...#.....#.#....#..#......#...#...#....#...#.#...#.#...#.#......#.###.....#..#...#...#.#.#.#.#......#.#...#",
            ".#.#.#.#.#####.#.#....#...#.#...#...#.#####....#...#..####.#....###.......#...#...#....#...#.#...#.#...#.#......#...#..####..#...#####.#.#.#.#...####.#...#",
            ".#.#.#.#.#.....#.#....#...#.#...#...#.#........#...#.#...#.#....#..#......#...#...#....#...#.#...#.#...#.#......#...#.#...#..#...#.....#.#.#.#..#...#.#...#",
            "..#...#...####.#..###..###..#...#...#..####....####...####..###.#...#......##..###......####..###...####.#.......####..####...##..####..#...#....####..####",
            "...........................................................................................#..............................................................#",
            ".......................................................................................####...........................................................####",
        ];
        assert_eq!(&rows[3..11], &original);
        assert!(rows[..3].iter().chain(&rows[11..]).all(|r| r.is_empty()));
    }

    /// More original pixels: the login screen's help text (HELVETICA_10, has a drop-out-sensitive
    /// comma), the password stars and the "Exit" button caption (HELVETICA_18, 16 px).
    #[test]
    fn dungeon_matches_original_login_screen() {
        if !load_dungeon() {
            return;
        }
        let set = TEXT.with(|t| t.borrow().fonts[&HELVETICA_10].clone());
        let rows = trim_ink(ascii(&set, "Otherwise, click on the New User button.", 14, 9));
        let original = [
            "....................................#..........................#",
            ".###...#...#.................................................#........#........................#...#..............#...#...................#...#....................#............#....#",
            "#...#.####.####...###..####...#...#.#..####..###.........###.#.#..###.#...#.....###..####.....####.####...###.....##..#..###.#...#...#....#...#..####..###..###....####..#...#.####.####..###..####",
            "#...#..#...#...#.#...#.#...#.#.#.#..#.#.....#...#.......#....#.#.#....#..#.....#...#.#...#.....#...#...#.#...#....##..#.#...#.#.#.#.#.....#...#.#.....#...#.#......#...#.#...#..#....#...#...#.#...#",
            "#...#..#...#...#.#####.#...#.#.#.#..#..###..#####.......#....#.#.#....###......#...#.#...#.....#...#...#.#####....#.#.#.#####.#.#.#.#.....#...#..###..#####.#......#...#.#...#..#....#...#...#.#...#",
            "#...#..#...#...#.#.....#...#.#.#.#..#.....#.#...........#....#.#.#....#..#.....#...#.#...#.....#...#...#.#........#..##.#.....#.#.#.#.....#...#.....#.#.....#......#...#.#...#..#....#...#...#.#...#",
            ".###....##.#...#..####.#....#...#...#.####...####.##.....###.#.#..###.#...#.....###..#...#......##.#...#..####....#...#..####..#...#.......###..####...####.#......####...####...##...##..###..#...#.##",
            "...................................................#",
        ];
        assert_eq!(rows, original);

        // BITMAP_15 (the password field) falls back to the default font: 8 dungeon stars
        assert_eq!(text_width("********", BITMAP_15), 40);

        let set = TEXT.with(|t| t.borrow().fonts[&HELVETICA_18].clone());
        let rows = trim_ink(ascii(&set, "Exit", 20, 14));
        let original = [
            "########............##",
            "##......................##",
            "##......................##",
            "##........##.....#..##.#######",
            "##.........##...#...##..##",
            "########....##.#....##..##",
            "##...........##.....##..##",
            "##...........###....##..##",
            "##..........#..##...##..##",
            "##.........#....##..##..##",
            "########..#......##.##...#####",
        ];
        assert_eq!(rows, original);
    }
}
