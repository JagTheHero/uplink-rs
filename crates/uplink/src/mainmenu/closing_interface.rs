//! Port of source/mainmenu/closing_interface.h + source/mainmenu/closing_interface.cpp

use uplink_engine::{gl, sound, text};

use crate::app::app::App;
use crate::app::opengl_interface::{clear_draw, textbutton_draw};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::mainmenu::mainmenu::MAINMENU_CLOSING;
use crate::world::generator::numbergenerator::NumberGenerator;

/// Shows an advert as Uplink closes down.
#[derive(Debug, Default)]
pub struct ClosingInterface {
    pub timesync: i32,
    pub status: i32,
    pub count: i32,
}

impl ClosingInterface {
    pub fn new() -> Self {
        Self::default()
    }

    /// Protected static `LargeTextDraw` (DrawFn).
    pub fn large_text_draw(g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        clear_draw(g, button.x, button.y, button.width, button.height);

        gl::color4f(1.0, 1.0, 1.0, 1.0);
        let ypos = (button.y + button.height / 2) + 5;
        text::draw_text_styled(button.x, ypos, &button.caption, text::HELVETICA_18);
    }
}

impl InterfaceScreen for ClosingInterface {
    fn create(&mut self, g: &mut Ctx) {
        g.ecl.register_button(260, 200, 125, 20, " ", " ", "closing_text");
        g.ecl.register_button_callbacks("closing_text", Some(Self::large_text_draw), None, None, None);
    }

    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            g.ecl.remove_button("closing_text");
            g.ecl.remove_button("closing_www");
            g.ecl.remove_button("closing_couk");
            g.ecl.remove_button("closing_time");
        }
    }

    fn update(&mut self, g: &mut Ctx) {
        if g.ecl.get_accurate_time() > self.timesync {
            g.ecl.get_button("closing_text").expect("closing_text button missing");

            let captions = [
                "Judgement",
                "Perception",
                "Thinking",
                "Feeling",
                "Sensation",
                "Intuition",
                "Extraversion",
            ];

            match self.status {
                0 => {
                    self.timesync = g.ecl.get_accurate_time() + 1000;
                    self.status = 1;
                }
                1 => {
                    let caption_number = (self.count % 7) as usize;
                    set_closing_text_caption(g, captions[caption_number]);
                    let filename = format!("sounds/{}.wav", NumberGenerator::random_number(9) + 1);
                    sound::play_sound(&filename, Some(&filename), true);
                    self.timesync = g.ecl.get_accurate_time() + 160;
                    self.count += 1;
                    if self.count > 12 {
                        self.status = 2;
                    }
                }
                2 => {
                    set_closing_text_caption(g, "INTROVERSION");
                    let filename = format!("sounds/{}.wav", NumberGenerator::random_number(9) + 1);
                    sound::play_sound(&filename, Some(&filename), true);
                    self.timesync = g.ecl.get_accurate_time() + 1000;
                    self.status += 1;
                }
                3 => {
                    g.ecl.register_button(220, 205, 40, 15, "www.", " ", "closing_www");
                    g.ecl.register_button_callbacks("closing_www", Some(textbutton_draw), None, None, None);

                    g.ecl.register_button(375, 205, 40, 15, ".co.uk", " ", "closing_couk");
                    g.ecl.register_button_callbacks("closing_couk", Some(textbutton_draw), None, None, None);

                    g.ecl.register_button(120, 460, 500, 15, " ", " ", "closing_time");
                    g.ecl.register_button_callbacks("closing_time", Some(textbutton_draw), None, None, None);
                    let elapsed_ms = g.ecl.get_accurate_time() - g.app.starttime;
                    let playtime = (((elapsed_ms as f32) as f64) / 1000.0 / 60.0) as i32;
                    let closingtime = format!(
                        "You've been using Uplink for {playtime} minutes. Visit our web site to buy the full product."
                    );
                    g.ecl.register_caption_change("closing_time", &closingtime, 1500, None);

                    g.ecl.button_bring_to_front("closing_text");

                    sound::play_sound("sounds/ringout.wav", Some("sounds/ringout.wav"), true);

                    self.timesync = g.ecl.get_accurate_time() + 3000;
                    self.status += 1;
                }
                4 => {
                    sound::play_sound("sounds/ringout.wav", Some("sounds/ringout.wav"), true);
                    self.timesync = g.ecl.get_accurate_time() + 3000;
                    self.status += 1;
                }
                5 => {
                    self.status += 1;
                    App::close(g);
                }
                _ => {}
            }
        }
    }

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("closing_text").is_some()
    }
    fn screen_id(&self) -> i32 {
        MAINMENU_CLOSING
    }
    crate::impl_as_any!();
}

/// `button->SetCaption(caption)` on the "closing_text" button (done from update, so the real button is changed).
fn set_closing_text_caption(g: &mut Ctx, caption: &str) {
    g.ecl
        .get_button_mut("closing_text")
        .expect("closing_text button missing")
        .set_caption(caption);
}
