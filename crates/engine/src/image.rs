//! Port of gucci's `Image` class.
//!
//! IMPORTANT: like the original (which used libtiff's `TIFFRGBAImageGet`), `pixels`
//! is stored BOTTOM-UP: row 0 is the bottom row of the picture. Code that reads
//! pixels (`get_pixel_r` etc., e.g. the world map land mask) therefore behaves
//! exactly like the C++. Drawing handles the flip.

use std::cell::RefCell;

use macroquad::prelude::*;

use crate::archive;

pub struct Image {
    width: i32,
    height: i32,
    alpha: f32,
    /// RGBA, bottom-up rows, `width * height * 4` bytes.
    pub pixels: Vec<u8>,
    texture: RefCell<Option<Texture2D>>,
    /// The texture `draw` (opaque) uses, created once like `texture`.
    opaque: RefCell<Option<Texture2D>>,
}

impl Clone for Image {
    fn clone(&self) -> Self {
        Image {
            width: self.width,
            height: self.height,
            alpha: self.alpha,
            pixels: self.pixels.clone(),
            texture: RefCell::new(None),
            opaque: RefCell::new(None),
        }
    }
}

impl std::fmt::Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Image({}x{})", self.width, self.height)
    }
}

impl Default for Image {
    fn default() -> Self {
        Self::new()
    }
}

impl Image {
    pub fn new() -> Self {
        Image { width: 0, height: 0, alpha: 1.0, pixels: Vec::new(), texture: RefCell::new(None), opaque: RefCell::new(None) }
    }

    /// Build from top-down RGBA bytes (the usual layout of decoders).
    pub fn from_rgba_top_down(width: i32, height: i32, rgba: &[u8]) -> Self {
        let mut pixels = Vec::with_capacity(rgba.len());
        let row = (width * 4) as usize;
        for y in (0..height as usize).rev() {
            pixels.extend_from_slice(&rgba[y * row..(y + 1) * row]);
        }
        Image { width, height, alpha: 1.0, pixels, texture: RefCell::new(None), opaque: RefCell::new(None) }
    }

    /// Decode an image file (TIF, PNG) from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if let Ok(img) = ::image::load_from_memory(bytes) {
            let img = img.to_rgba8();
            let (w, h) = img.dimensions();
            return Some(Self::from_rgba_top_down(w as i32, h as i32, img.as_raw()));
        }
        // The image crate rejects 8-bit palette TIFFs (several loading screens use them);
        // libtiff's TIFFRGBAImage expanded them through the colour map.
        Self::from_palette_tiff(bytes)
    }

    /// Uncompressed 8-bit palette TIFF (the tiff crate does not decode palette images): the strips
    /// are read straight from the file and expanded through the colour map.
    fn from_palette_tiff(bytes: &[u8]) -> Option<Self> {
        use tiff::decoder::Decoder;
        use tiff::tags::Tag;
        let mut dec = Decoder::new(std::io::Cursor::new(bytes)).ok()?;
        let (w, h) = dec.dimensions().ok()?;
        let tag = |dec: &mut Decoder<_>, t| dec.get_tag_u32(t).ok();
        if tag(&mut dec, Tag::PhotometricInterpretation)? != 3
            || tag(&mut dec, Tag::BitsPerSample)? != 8
            || tag(&mut dec, Tag::Compression).unwrap_or(1) != 1
        {
            return None;
        }
        // ColorMap: all reds, then all greens, then all blues (16 bits each).
        let map = dec.get_tag_u16_vec(Tag::ColorMap).ok()?;
        let offsets = dec.get_tag_u32_vec(Tag::StripOffsets).ok()?;
        let counts = dec.get_tag_u32_vec(Tag::StripByteCounts).ok()?;
        let n = map.len() / 3;
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for (&off, &len) in offsets.iter().zip(&counts) {
            for &i in bytes.get(off as usize..(off + len) as usize)? {
                let i = (i as usize).min(n.checked_sub(1)?);
                rgba.extend_from_slice(&[(map[i] >> 8) as u8, (map[n + i] >> 8) as u8, (map[2 * n + i] >> 8) as u8, 255]);
            }
        }
        rgba.truncate((w * h * 4) as usize);
        (rgba.len() == (w * h * 4) as usize).then(|| Self::from_rgba_top_down(w as i32, h as i32, &rgba))
    }

    /// `LoadTIF`: load from the archive VFS (e.g. `"graphics/close.tif"`).
    /// On failure the image becomes the error cross bitmap, like the original.
    pub fn load_tif(&mut self, filename: &str) {
        match archive::read(filename).and_then(|b| Self::from_bytes(&b)) {
            Some(img) => *self = img,
            None => {
                eprintln!("GUCCI Error - failed to load TIF {filename}");
                self.create_error_bitmap();
            }
        }
    }

    /// Convenience constructor wrapping [`Image::load_tif`].
    pub fn load(filename: &str) -> Self {
        let mut i = Image::new();
        i.load_tif(filename);
        i
    }

    /// `LoadRAW`: raw RGB bytes.
    pub fn load_raw(&mut self, filename: &str, sizex: i32, sizey: i32) {
        match archive::read(filename) {
            Some(b) if b.len() >= (sizex * sizey * 3) as usize => {
                let mut rgba = Vec::with_capacity((sizex * sizey * 4) as usize);
                for px in b.chunks_exact(3).take((sizex * sizey) as usize) {
                    rgba.extend_from_slice(&[px[0], px[1], px[2], 255]);
                }
                *self = Self::from_rgba_top_down(sizex, sizey, &rgba);
            }
            _ => self.create_error_bitmap(),
        }
    }

    pub fn width(&self) -> i32 {
        self.width
    }
    pub fn height(&self) -> i32 {
        self.height
    }

    pub fn set_alpha(&mut self, newalpha: f32) {
        self.alpha = newalpha;
        let a = (newalpha * 256.0).clamp(0.0, 255.0) as u8;
        for px in self.pixels.chunks_exact_mut(4) {
            px[3] = a;
        }
        self.drop_textures();
    }

    pub fn get_alpha(&self) -> f32 {
        self.alpha
    }

    pub fn flip_around_h(&mut self) {
        let row = (self.width * 4) as usize;
        let mut out = Vec::with_capacity(self.pixels.len());
        for y in (0..self.height as usize).rev() {
            out.extend_from_slice(&self.pixels[y * row..(y + 1) * row]);
        }
        self.pixels = out;
        self.drop_textures();
    }

    /// Resample to a new size (bilinear, like gluScaleImage).
    pub fn scale(&mut self, newwidth: i32, newheight: i32) {
        if newwidth <= 0 || newheight <= 0 || self.width <= 0 || self.height <= 0 {
            return;
        }
        let src = ::image::RgbaImage::from_raw(self.width as u32, self.height as u32, self.pixels.clone());
        if let Some(src) = src {
            let dst = ::image::imageops::resize(
                &src,
                newwidth as u32,
                newheight as u32,
                ::image::imageops::FilterType::Triangle,
            );
            self.pixels = dst.into_raw();
            self.width = newwidth;
            self.height = newheight;
            self.drop_textures();
        }
    }

    /// No-op today: modern GL has no power-of-two restriction.
    pub fn scale_to_opengl(&mut self) {}

    /// Draw opaque at (x, y) = top-left, in virtual screen pixels.
    pub fn draw(&self, x: i32, y: i32) {
        self.draw_inner(x, y, false);
    }

    /// Draw alpha-blended at (x, y) = top-left.
    pub fn draw_blend(&self, x: i32, y: i32) {
        self.draw_inner(x, y, true);
    }

    /// Draw stretched into the given rectangle (alpha-blended).
    pub fn draw_scaled(&self, x: i32, y: i32, w: i32, h: i32) {
        if self.width == 0 {
            return;
        }
        let tex = self.texture();
        draw_texture_ex(
            &tex,
            x as f32,
            y as f32,
            WHITE,
            DrawTextureParams { dest_size: Some(vec2(w as f32, h as f32)), flip_y: false, ..Default::default() },
        );
    }

    fn draw_inner(&self, x: i32, y: i32, blend: bool) {
        if self.width == 0 {
            return;
        }
        let tex = if blend { self.texture() } else { self.opaque_texture() };
        draw_texture(&tex, x as f32, y as f32, WHITE);
    }

    /// GPU texture (top-down orientation), created lazily and cached.
    pub fn texture(&self) -> Texture2D {
        if let Some(t) = self.texture.borrow().as_ref() {
            return t.clone();
        }
        let t = Texture2D::from_rgba8(self.width.max(1) as u16, self.height.max(1) as u16, &self.top_down(false));
        t.set_filter(FilterMode::Nearest);
        self.texture.replace(Some(t.clone()));
        t
    }

    fn opaque_texture(&self) -> Texture2D {
        if let Some(t) = self.opaque.borrow().as_ref() {
            return t.clone();
        }
        // Opaque drawing ignores alpha. Most Uplink images are fully opaque anyway, so
        // only build a separate texture if some pixel is translucent.
        let t = if self.pixels.chunks_exact(4).all(|p| p[3] == 255) {
            self.texture()
        } else {
            let t = Texture2D::from_rgba8(self.width.max(1) as u16, self.height.max(1) as u16, &self.top_down(true));
            t.set_filter(FilterMode::Nearest);
            t
        };
        self.opaque.replace(Some(t.clone()));
        t
    }

    /// Forget the cached textures after the pixels changed. (Every texture macroquad creates
    /// stays in its bookkeeping for good, so textures must never be built per frame.)
    fn drop_textures(&self) {
        self.texture.replace(None);
        self.opaque.replace(None);
    }

    fn top_down(&self, force_opaque: bool) -> Vec<u8> {
        let row = (self.width * 4) as usize;
        let mut out = Vec::with_capacity(self.pixels.len());
        for y in (0..self.height as usize).rev() {
            out.extend_from_slice(&self.pixels[y * row..(y + 1) * row]);
        }
        if force_opaque {
            for px in out.chunks_exact_mut(4) {
                px[3] = 255;
            }
        }
        out
    }

    pub fn create_error_bitmap(&mut self) {
        let (w, h) = (32, 32);
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let white = x == 0 || y == 0 || x == w - 1 || y == h - 1 || x == y || x + y == w;
                let i = ((y * w + x) * 4) as usize;
                let c = if white { 255 } else { 0 };
                rgba[i..i + 4].copy_from_slice(&[c, c, c, 255]);
            }
        }
        *self = Self::from_rgba_top_down(w, h, &rgba);
    }

    fn pixel(&self, x: i32, y: i32, channel: usize) -> u8 {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return 0;
        }
        self.pixels[((y * self.width + x) * 4) as usize + channel]
    }

    /// Bottom-up coordinates, like the original.
    pub fn get_pixel_r(&self, x: i32, y: i32) -> u8 {
        self.pixel(x, y, 0)
    }
    pub fn get_pixel_g(&self, x: i32, y: i32) -> u8 {
        self.pixel(x, y, 1)
    }
    pub fn get_pixel_b(&self, x: i32, y: i32) -> u8 {
        self.pixel(x, y, 2)
    }

    /// RGB bytes, bottom-up (`GetRGBPixels`).
    pub fn get_rgb_pixels(&self) -> Vec<u8> {
        self.pixels.chunks_exact(4).flat_map(|p| [p[0], p[1], p[2]]).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Needs the original data (UPLINK_DATA or a GOG install next to the binary); skipped otherwise.
    #[test]
    fn palette_tiff_from_original_data() {
        let Some(dir) = archive::find_data_dir() else { return };
        archive::init(&dir);
        let bytes = archive::read("graphics/loading/kris.tif").expect("kris.tif in loading.dat");
        let img = Image::from_bytes(&bytes).expect("palette TIFF decodes");
        assert_eq!((img.width, img.height), (256, 256));
        assert!(img.pixels.chunks_exact(4).any(|p| p[0] != p[1] || p[1] != p[2]), "colour, not grey");
    }
}
