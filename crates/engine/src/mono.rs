//! Hinted, 1-bit TrueType glyphs, the way the original game got them from GLTT (FreeType 1):
//! the outline is hinted by the font's own TrueType bytecode (skrifa's interpreter, mono target)
//! and scan-converted without anti-aliasing by a port of the rules of FreeType 1's monochrome
//! rasteriser (`ttraster.c`): a pixel is set when its centre is inside the outline, plus
//! drop-out control in the vertical sweep as selected by the font's `prep` (dungeon.ttf:
//! SCANCTRL 511, SCANTYPE 1 = simple drop-outs).
//!
//! Checked pixel for pixel against the original game's screens (dungeon.ttf at 9, 11 and 16 px).

use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{DrawSettings, Engine, HintingInstance, HintingOptions, OutlinePen, Target};
use skrifa::{FontRef, MetadataProvider, OutlineGlyphCollection};

/// One rasterised glyph. `bits` is `w * h` bytes (0 or 1), rows top-down.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MonoGlyph {
    pub w: i32,
    pub h: i32,
    /// Bitmap column 0 relative to the pen x.
    pub left: i32,
    /// Bitmap row 0 is this many pixels above the baseline (FreeType's `bitmap_top`):
    /// row `r` is screen row `baseline_y - top + r`.
    pub top: i32,
    /// Hinted advance in whole pixels.
    pub advance: i32,
    pub bits: Vec<u8>,
}

/// A closed polyline in pixels, y up.
pub type Contour = Vec<(f32, f32)>;

/// FreeType 1's drop-out modes (`ras.dropOutControl`): 0 none, 1 simple, 2 simple excluding
/// stubs, 4 smart, 5 smart excluding stubs.
pub type DropoutMode = u8;

/// A TrueType face at one pixel size, hinted for monochrome output.
pub struct MonoFont {
    font: FontRef<'static>,
    outlines: OutlineGlyphCollection<'static>,
    hinter: Option<HintingInstance>,
    ppem: f32,
    dropout: DropoutMode,
}

impl MonoFont {
    /// `data` must live for the program (faces are loaded once and kept).
    /// `ppem` is pixels per em (FreeType rounds it to an integer for fonts with head flag bit 3).
    pub fn new(data: &'static [u8], ppem: f32) -> Option<MonoFont> {
        let font = FontRef::new(data).ok()?;
        let outlines = font.outline_glyphs();
        let hinter = HintingInstance::new(
            &outlines,
            Size::new(ppem),
            LocationRef::default(),
            HintingOptions { engine: Engine::Interpreter, target: Target::Mono },
        )
        .ok();
        // skrifa does not expose the graphics state left by `prep`, so read SCANCTRL/SCANTYPE
        // from it the simple way: the common "PUSHW[1] type ctrl SCANCTRL SCANTYPE" idiom.
        let dropout = if hinter.is_some() { prep_dropout_mode(&font, ppem) } else { 0 };
        Some(MonoFont { font, outlines, hinter, ppem, dropout })
    }

    /// The hinted outline of `ch` as closed polylines in pixels (y up), and its advance.
    pub fn outline(&self, ch: char) -> Option<(Vec<Contour>, i32)> {
        let gid = self.font.charmap().map(ch)?;
        let outline = self.outlines.get(gid)?;
        let mut pen = Contours::default();
        let metrics = match &self.hinter {
            Some(h) => outline.draw(DrawSettings::hinted(h, false), &mut pen).ok()?,
            None => outline.draw(DrawSettings::unhinted(Size::new(self.ppem), LocationRef::default()), &mut pen).ok()?,
        };
        pen.finish();
        let advance = match metrics.advance_width {
            Some(a) => a.round() as i32,
            None => self
                .font
                .glyph_metrics(Size::new(self.ppem), LocationRef::default())
                .advance_width(gid)
                .unwrap_or(0.0)
                .round() as i32,
        };
        Some((pen.contours, advance))
    }

    /// Rasterise `ch`; `None` when the font has no glyph for it.
    pub fn glyph(&self, ch: char) -> Option<MonoGlyph> {
        let (contours, advance) = self.outline(ch)?;
        let mut g = rasterise(&contours, self.dropout);
        g.advance = advance;
        Some(g)
    }
}

/// Find `SCANCTRL`/`SCANTYPE` in `prep` and map them to FreeType 1's drop-out mode.
fn prep_dropout_mode(font: &FontRef, ppem: f32) -> DropoutMode {
    let Some(prep) = font.table_data(skrifa::Tag::new(b"prep")) else { return 0 };
    let code = prep.as_bytes();
    let ppem = ppem.round() as u16;
    // PUSHW[1] (0xB9) type(2 bytes) ctrl(2 bytes) SCANCTRL (0x85) SCANTYPE (0x8D)
    for i in 0..code.len().saturating_sub(6) {
        if code[i] == 0xB9 && code[i + 5] == 0x85 && code[i + 6] == 0x8D {
            let scan_type = u16::from_be_bytes([code[i + 1], code[i + 2]]);
            let ctrl = u16::from_be_bytes([code[i + 3], code[i + 4]]);
            // SCANCTRL: low byte = ppem threshold (0xFF = every size), bit 8 = on at or below it,
            // bit 11 = off above it
            let threshold = ctrl & 0xFF;
            let mut on = threshold == 0xFF || (ctrl & 0x100 != 0 && ppem <= threshold);
            if ctrl & 0x800 != 0 && threshold != 0xFF && ppem > threshold {
                on = false;
            }
            // FreeType 1 used SCANTYPE as its drop-out mode as is (FreeType 2 later remapped it
            // to the spec's meaning: 1 = simple *excluding* stubs). For dungeon.ttf the two only
            // differ in '<', '>', '@' (9 and 16 px) and '&', 'Q' (11 px).
            return if on && matches!(scan_type, 1 | 2 | 4 | 5) { scan_type as DropoutMode } else { 0 };
        }
    }
    0
}

/// Outline as closed polylines, in pixels, y up.
#[derive(Default)]
struct Contours {
    contours: Vec<Vec<(f32, f32)>>,
    cur: Vec<(f32, f32)>,
}

impl Contours {
    fn point(&mut self, x: f32, y: f32) {
        if self.cur.last() != Some(&(x, y)) {
            self.cur.push((x, y));
        }
    }
    fn finish(&mut self) {
        let mut c = std::mem::take(&mut self.cur);
        if c.len() > 1 && c.first() == c.last() {
            c.pop();
        }
        if c.len() > 1 {
            self.contours.push(c);
        }
    }
    fn last(&self) -> (f32, f32) {
        self.cur.last().copied().unwrap_or((0.0, 0.0))
    }
}

/// Flattening steps per Bézier (glyphs are ~10 px tall, so this is far below a 1/64 px error).
const CURVE_STEPS: usize = 16;

impl OutlinePen for Contours {
    fn move_to(&mut self, x: f32, y: f32) {
        self.finish();
        self.point(x, y);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.point(x, y);
    }
    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        let (x0, y0) = self.last();
        for i in 1..=CURVE_STEPS {
            let t = i as f32 / CURVE_STEPS as f32;
            let u = 1.0 - t;
            self.point(u * u * x0 + 2.0 * u * t * cx + t * t * x, u * u * y0 + 2.0 * u * t * cy + t * t * y);
        }
    }
    fn curve_to(&mut self, c0x: f32, c0y: f32, c1x: f32, c1y: f32, x: f32, y: f32) {
        let (x0, y0) = self.last();
        for i in 1..=CURVE_STEPS {
            let t = i as f32 / CURVE_STEPS as f32;
            let u = 1.0 - t;
            let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
            self.point(a * x0 + b * c0x + c * c1x + d * x, a * y0 + b * c0y + c * c1y + d * y);
        }
    }
    fn close(&mut self) {
        self.finish();
    }
}

// ------------------------------------------------------------------ the scan converter
//
// Coordinates inside the converter are "centre coordinates": pixel centres sit on integers
// (FreeType's SCALED() subtracts half a pixel the same way). Pixel column k covers centre x = k.

/// A y-monotonic run of a contour (FreeType's profile).
struct Profile {
    up: bool,
    /// points in ascending y
    pts: Vec<(f32, f32)>,
    /// first and last scanline (inclusive)
    bot: i32,
    top: i32,
    /// next non-empty profile of the same contour (cyclic)
    next: usize,
}

impl Profile {
    fn x_at(&self, y: f32) -> f32 {
        for w in self.pts.windows(2) {
            let ((x0, y0), (x1, y1)) = (w[0], w[1]);
            if y >= y0 && y <= y1 && y1 > y0 {
                return x0 + (x1 - x0) * (y - y0) / (y1 - y0);
            }
        }
        self.pts.last().map_or(0.0, |p| p.0)
    }
}

/// Split contours into profiles the way the rasteriser's `Decompose_Curve` + `End_Profile` do.
fn build_profiles(contours: &[Vec<(f32, f32)>]) -> Vec<Profile> {
    let mut all = Vec::new();
    for c in contours {
        // runs of strictly rising / falling y; flat segments join the current run
        let mut runs: Vec<(bool, Vec<(f32, f32)>)> = Vec::new();
        let n = c.len();
        let mut state: Option<bool> = None;
        let mut cur: Vec<(f32, f32)> = vec![c[0]];
        for i in 1..=n {
            let p = c[i % n];
            let last = *cur.last().unwrap();
            let dir = if p.1 > last.1 {
                Some(true)
            } else if p.1 < last.1 {
                Some(false)
            } else {
                None
            };
            if let (Some(d), Some(s)) = (dir, state) {
                if d != s {
                    runs.push((s, std::mem::replace(&mut cur, vec![last])));
                }
            }
            if dir.is_some() {
                state = dir;
            }
            cur.push(p);
        }
        let Some(s) = state else { continue };
        runs.push((s, cur));
        let start_y = c[0].1;
        let first_dir = runs[0].0;
        let nruns = runs.len();
        let base = all.len();
        for (i, (up, mut pts)) in runs.into_iter().enumerate() {
            if !up {
                pts.reverse();
            }
            let (ylo, yhi) = (pts[0].1, pts[pts.len() - 1].1);
            let mut bot = ylo.ceil() as i32;
            let mut top = yhi.floor() as i32;
            // the extreme arcs of a contour join: drop the duplicated scanline (Convert_Glyph)
            if i == nruns - 1 && nruns > 1 && up == first_dir && start_y.fract() == 0.0 {
                if up {
                    top -= 1;
                } else {
                    bot += 1;
                }
            }
            all.push(Profile { up, pts, bot, top, next: 0 });
        }
        // keep non-empty profiles only and link them cyclically in contour order
        let kept: Vec<usize> = (base..all.len()).filter(|&i| all[i].bot <= all[i].top).collect();
        for (k, &i) in kept.iter().enumerate() {
            all[i].next = kept[(k + 1) % kept.len()];
        }
        // empty profiles stay in the vector but never become active
    }
    all
}

/// A bitmap in centre coordinates: column x in [x0, x0+w), row y in [y0, y0+h), y up.
struct Grid {
    x0: i32,
    y0: i32,
    w: i32,
    h: i32,
    bits: Vec<u8>,
}

impl Grid {
    fn get(&self, x: i32, y: i32) -> bool {
        let (i, j) = (x - self.x0, y - self.y0);
        i >= 0 && j >= 0 && i < self.w && j < self.h && self.bits[(j * self.w + i) as usize] != 0
    }
    fn set(&mut self, x: i32, y: i32) {
        let (i, j) = (x - self.x0, y - self.y0);
        if i >= 0 && j >= 0 && i < self.w && j < self.h {
            self.bits[(j * self.w + i) as usize] = 1;
        }
    }
}

/// The vertical sweep of FreeType 1's `ttraster.c` (`Draw_Sweep` with `Vertical_Sweep_Span` and
/// `Vertical_Sweep_Drop`). The original showed no drop-outs from the horizontal second pass, so
/// there is none here.
fn sweep(profiles: &[Profile], mode: DropoutMode, lo: i32, hi: i32, grid: &mut Grid) {
    let mut left: Vec<(f32, usize)> = Vec::new();
    let mut right: Vec<(f32, usize)> = Vec::new();
    let mut drops: Vec<(f32, f32, usize, usize)> = Vec::new();
    for y in lo..=hi {
        left.clear();
        right.clear();
        drops.clear();
        for (i, p) in profiles.iter().enumerate() {
            if y >= p.bot && y <= p.top {
                let x = p.x_at(y as f32);
                if p.up { left.push((x, i)) } else { right.push((x, i)) }
            }
        }
        left.sort_by(|a, b| a.0.total_cmp(&b.0));
        right.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (&(mut x1, mut l), &(mut x2, mut r)) in left.iter().zip(right.iter()) {
            if x1 > x2 {
                std::mem::swap(&mut x1, &mut x2);
                std::mem::swap(&mut l, &mut r);
            }
            if x2 - x1 <= 1.0 {
                let (f1, c2) = (x1.floor(), x2.ceil());
                if mode != 0 && (f1 > c2 || c2 == f1 + 1.0) {
                    drops.push((x1, x2, l, r));
                    continue;
                }
            }
            // Vertical_Sweep_Span: pixel centres in [x1, x2]; a span up to 1 px wide sets one
            let e1 = x1.ceil() as i32;
            let e2 = if x2 - x1 <= 1.0 { e1 } else { x2.floor() as i32 };
            for x in e1..=e2 {
                grid.set(x, y);
            }
        }
        // drop-outs are handled after the scanline's spans
        for &(x1, x2, l, r) in &drops {
            let (e1, e2) = (x1.ceil(), x2.floor());
            let mut pxl = e1;
            if e1 > e2 {
                if e1 != e2 + 1.0 {
                    continue;
                }
                match mode {
                    1 => pxl = e2,
                    4 => pxl = ((x1 + x2) / 2.0).ceil(),
                    2 | 5 => {
                        // Drop-out Control Rule #4: no stubs
                        let (lp, rp) = (&profiles[l], &profiles[r]);
                        if lp.next == r && y == lp.top {
                            continue;
                        }
                        if rp.next == l && y == lp.bot {
                            continue;
                        }
                        // the rightmost pixel is already set
                        if grid.get(e1 as i32, y) {
                            continue;
                        }
                        pxl = if mode == 2 { e2 } else { ((x1 + x2) / 2.0).ceil() };
                    }
                    _ => continue,
                }
            }
            grid.set(pxl as i32, y);
        }
    }
}

/// Scan-convert closed polylines (pixel coordinates, y up) into a 1-bit glyph.
fn rasterise(contours: &[Vec<(f32, f32)>], mode: DropoutMode) -> MonoGlyph {
    let (mut xmin, mut ymin, mut xmax, mut ymax) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for &(x, y) in contours.iter().flatten() {
        xmin = xmin.min(x);
        xmax = xmax.max(x);
        ymin = ymin.min(y);
        ymax = ymax.max(y);
    }
    if xmin > xmax {
        return MonoGlyph::default();
    }
    // the bitmap GLTT allocated: the outline's box grown to whole pixels
    let (px0, px1) = (xmin.floor() as i32, (xmax.ceil() as i32).max(xmin.floor() as i32 + 1));
    let (py0, py1) = (ymin.floor() as i32, (ymax.ceil() as i32).max(ymin.floor() as i32 + 1));
    let mut grid = Grid { x0: px0, y0: py0, w: px1 - px0, h: py1 - py0, bits: vec![0; ((px1 - px0) * (py1 - py0)) as usize] };

    let centred: Vec<Vec<(f32, f32)>> =
        contours.iter().map(|c| c.iter().map(|&(x, y)| (x - 0.5, y - 0.5)).collect()).collect();
    let profiles = build_profiles(&centred);
    sweep(&profiles, mode, (ymin - 0.5).ceil() as i32, (ymax - 0.5).floor() as i32, &mut grid);

    // trim to the set pixels and flip to rows top-down
    let on: Vec<(i32, i32)> = (0..grid.h)
        .flat_map(|j| (0..grid.w).map(move |i| (i, j)))
        .filter(|&(i, j)| grid.bits[(j * grid.w + i) as usize] != 0)
        .collect();
    if on.is_empty() {
        return MonoGlyph::default();
    }
    let i0 = on.iter().map(|p| p.0).min().unwrap();
    let i1 = on.iter().map(|p| p.0).max().unwrap();
    let j0 = on.iter().map(|p| p.1).min().unwrap();
    let j1 = on.iter().map(|p| p.1).max().unwrap();
    let (w, h) = (i1 - i0 + 1, j1 - j0 + 1);
    let mut bits = vec![0u8; (w * h) as usize];
    for (i, j) in on {
        bits[((j1 - j) * w + (i - i0)) as usize] = 1;
    }
    MonoGlyph { w, h, left: grid.x0 + i0, top: grid.y0 + j1 + 1, advance: 0, bits }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<(f32, f32)> {
        // clockwise in y-up, like TrueType outer contours
        vec![(x0, y0), (x0, y1), (x1, y1), (x1, y0)]
    }

    #[test]
    fn box_covers_pixel_centres() {
        let g = rasterise(&[rect(1.0, 0.0, 3.0, 3.0)], 1);
        assert_eq!((g.w, g.h, g.left, g.top), (2, 3, 1, 3));
        assert!(g.bits.iter().all(|&b| b == 1));
    }

    #[test]
    fn thin_stem_needs_dropout_control() {
        // a 0.4 px wide vertical stem between pixel centres
        let stem = rect(1.55, 0.0, 1.95, 4.0);
        assert_eq!(rasterise(&[stem.clone()], 0), MonoGlyph::default());
        // simple drop-outs pick the left pixel (centre 1.5 lies left of the stem)
        let g = rasterise(&[stem.clone()], 1);
        assert_eq!((g.w, g.h, g.left, g.top), (1, 4, 1, 4));
        // excluding stubs drops the stem's ends (first and last scanline)
        let g = rasterise(&[stem], 2);
        assert_eq!((g.w, g.h, g.left, g.top), (1, 2, 1, 3));
    }

    #[test]
    fn hole_stays_empty() {
        // outer clockwise, inner counter-clockwise
        let outer = rect(0.0, 0.0, 5.0, 5.0);
        let mut inner = rect(1.0, 1.0, 4.0, 4.0);
        inner.reverse();
        let g = rasterise(&[outer, inner], 1);
        assert_eq!((g.w, g.h), (5, 5));
        let filled: usize = g.bits.iter().map(|&b| b as usize).sum();
        assert_eq!(filled, 25 - 9);
    }
}
