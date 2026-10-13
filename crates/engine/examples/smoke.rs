//! Visual smoke test: `UPLINK_DATA=~/Games/uplink-gog/app cargo run --example smoke -- out.png`
use uplink_engine::{archive, gl, image::Image, text};
use macroquad::prelude::*;

fn conf() -> Conf {
    Conf { window_title: "smoke".into(), window_width: 1024, window_height: 768, ..Default::default() }
}

#[macroquad::main(conf)]
async fn main() {
    let dir = archive::find_data_dir().expect("data dir");
    println!("mounted {}", archive::init(&dir));
    text::enable_truetype_support();
    println!("font {}", text::load_truetype_font(text::HELVETICA_12, "Dungeon", "fonts/dungeon.ttf", 11));
    let img = Image::load("graphics/mainmenu/theme.tif");
    let close = Image::load("graphics/close.tif");
    let out = std::env::args().nth(1);
    let frames: i32 = std::env::var("SMOKE_FRAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(5);
    for frame in 0..frames {
        gl::set_virtual_size(1024, 768);
        gl::begin_frame(BLACK);
        gl::begin(gl::QUADS);
        gl::color4f(0.2, 0.2, 0.6, 0.85); gl::vertex2i(10, 10); gl::vertex2i(300, 10);
        gl::color4f(0.0, 0.0, 0.2, 0.85); gl::vertex2i(300, 60); gl::vertex2i(10, 60);
        gl::end();
        gl::color3f(1.0, 1.0, 1.0);
        gl::rect_outline(10, 10, 290, 50);
        text::draw_text_styled(20, 40, "Uplink Rust port - HELVETICA_12", text::HELVETICA_12);
        text::draw_text_styled(20, 90, "fallback BITMAP_15 text", text::BITMAP_15);
        img.draw(20, 120);
        close.draw_blend(400, 20);
        gl::enable(gl::LINE_STIPPLE); gl::line_stipple(2, 0x00FF);
        gl::begin(gl::LINES); gl::vertex2i(10, 700); gl::vertex2i(1000, 700); gl::end();
        gl::disable(gl::LINE_STIPPLE);
        gl::scissor(500, 768 - 400, 100, 100); gl::enable(gl::SCISSOR_TEST);
        gl::color3f(0.0, 1.0, 0.0); gl::rect(450, 250, 300, 300);
        gl::disable(gl::SCISSOR_TEST);
        if frame == 4 { if let Some(p) = &out { gl::screenshot(p); } }
        next_frame().await;
    }
    // Leave without the libc/thread-local teardown that races macroquad's audio thread.
    uplink_engine::exit_now(0);
}
