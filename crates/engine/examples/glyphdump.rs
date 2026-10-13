//! Print a string rendered by the mono TrueType rasteriser as ASCII art (for comparing against
//! screenshots of the original). Usage: glyphdump FONT.ttf PPEM "text"
use uplink_engine::mono::MonoFont;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let data: &'static [u8] = Box::leak(std::fs::read(&args[1]).expect("font").into_boxed_slice());
    let ppem: f32 = args[2].parse().unwrap();
    let font = MonoFont::new(data, ppem).expect("parse");
    if std::env::var("OUTLINE").is_ok() {
        for c in args[3].chars() {
            println!("{c:?}: {:?}", font.outline(c));
        }
    }
    let glyphs: Vec<_> = args[3].chars().map(|c| font.glyph(c).unwrap_or_default()).collect();
    let (mut top, mut bottom, mut width, mut pen) = (0, 0, 0, 0);
    let shift = -glyphs.iter().map(|g| g.left).min().unwrap_or(0).min(0);
    for g in &glyphs {
        top = top.max(g.top);
        bottom = bottom.min(g.top - g.h);
        width = width.max(shift + pen + g.left + g.w);
        pen += g.advance;
    }
    let width = width.max(pen) as usize;
    let rows = (top - bottom) as usize;
    let mut grid = vec![vec!['.'; width]; rows];
    let mut pen = 0;
    for g in &glyphs {
        for r in 0..g.h {
            for c in 0..g.w {
                if g.bits[(r * g.w + c) as usize] != 0 {
                    grid[(top - g.top + r) as usize][(shift + pen + g.left + c) as usize] = '#';
                }
            }
        }
        pen += g.advance;
    }
    for (i, row) in grid.iter().enumerate() {
        let y = top - i as i32 - 1; // pixel row above-baseline index (0 = row just above baseline)
        println!("{:3} {}", y, row.iter().collect::<String>());
    }
    println!("advances {:?} total {}", glyphs.iter().map(|g| g.advance).collect::<Vec<_>>(), pen);
}
