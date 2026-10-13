//! Port of source/mainmenu/obituary_interface.h + source/mainmenu/obituary_interface.cpp

use uplink_engine::{gl, sound, text};

use crate::app::globals::{ALPHA, DEMOGAME, FULLGAME};
use crate::app::opengl_interface::{
    border_draw, button_assignbitmaps, button_click, button_highlight, clear_draw, imagebutton_draw,
    textbutton_draw,
};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::mainmenu::mainmenu::{MainMenu, MAINMENU_LOGIN, MAINMENU_OBITUARY, MAINMENU_THETEAM};
use crate::mainmenu::mainmenuscreen;
use crate::world::generator::plotgenerator::PlotGenerator;
use crate::world::rating::Rating;

/// Obituary interface: lets the player view the final condition of a fallen agent.
#[derive(Debug, Default)]
pub struct ObituaryInterface {}

impl ObituaryInterface {
    pub fn new() -> Self {
        Self::default()
    }

    /// Static `MediumTextDraw` (DrawFn).
    pub fn medium_text_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        let screenheight = g.app.options.get_option_value("graphics_screenheight");
        gl::scissor(button.x, screenheight - (button.y + button.height), button.width, button.height);
        gl::enable(gl::SCISSOR_TEST);

        clear_draw(g, button.x, button.y, button.width, button.height);

        gl::color4f(1.0, 1.0, 1.0, ALPHA);
        let xpos = button.x + 10;
        let ypos = (button.y + button.height / 2) + 3;
        text::draw_text_styled(xpos, ypos, &button.caption, text::HELVETICA_12);

        if highlighted || clicked {
            border_draw(button);
        }

        gl::disable(gl::SCISSOR_TEST);
    }

    /// Static `LargeTextDraw` (DrawFn).
    pub fn large_text_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        let screenheight = g.app.options.get_option_value("graphics_screenheight");
        gl::scissor(button.x, screenheight - (button.y + button.height), button.width, button.height);
        gl::enable(gl::SCISSOR_TEST);

        clear_draw(g, button.x, button.y, button.width, button.height);

        gl::color4f(1.0, 1.0, 1.0, ALPHA);
        let xpos = button.x + 10;
        let ypos = (button.y + button.height / 2) + 3;
        text::draw_text_styled(xpos, ypos, &button.caption, text::HELVETICA_18);

        if highlighted || clicked {
            border_draw(button);
        }

        gl::disable(gl::SCISSOR_TEST);
    }

    /// Static `ReturnToMainMenuClick` (MouseFn).
    pub fn return_to_main_menu_click(g: &mut Ctx, _button: &Button) {
        MainMenu::run_screen(g, MAINMENU_LOGIN);
    }

    /// Static `PiClick` (MouseFn).
    pub fn pi_click(g: &mut Ctx, _button: &Button) {
        MainMenu::run_screen(g, MAINMENU_THETEAM);
    }
}

impl InterfaceScreen for ObituaryInterface {
    fn create(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            return;
        }

        if g.app.options.is_option_equal_to("sound_musicenabled", 1) {
            //SgPlaylist_Play ("sad");
            sound::playlist_play("main");
        }

        mainmenuscreen::create(g);

        let screenw = g.app.options.get_option_value("graphics_screenwidth");
        let screenh = g.app.options.get_option_value("graphics_screenheight");

        // Copy the obituary out of the game so the buttons can be registered (and awards assigned) freely.
        let gob = g.game.get_obituary().clone();

        let finances = format!("{} credits", gob.money);
        let name = format!("Agent {}", gob.name);

        g.ecl.register_button(20, 200, 200, 40, &name, "", "obituary_name");
        g.ecl.register_button_callbacks("obituary_name", Some(Self::large_text_draw), None, None, None);

        let reason = gob.game_over_reason().unwrap_or("").to_string();
        g.ecl.register_button(20, 240, 400, 50, &reason, "", "obituary_text");
        g.ecl.register_button_callbacks("obituary_text", Some(textbutton_draw), None, None, None);

        //
        // Left column

        g.ecl.register_button(20, 300, 140, 15, "Finances", "", "obituary_financestext");
        g.ecl.register_button(20, 320, 140, 15, "Uplink Rating", "", "obituary_uratingtext");
        g.ecl.register_button(20, 340, 140, 15, "Neuromancer Rating", "", "obituary_nratingtext");

        g.ecl.register_button(190, 300, 100, 15, &finances, "", "obituary_finances");
        g.ecl.register_button(
            190,
            320,
            100,
            15,
            &Rating::get_uplink_rating_string(gob.uplinkrating),
            "",
            "obituary_urating",
        );
        g.ecl.register_button(
            190,
            340,
            100,
            15,
            &Rating::get_neuromancer_rating_string(gob.neuromancerrating),
            "",
            "obituary_nrating",
        );

        g.ecl.register_button_callbacks("obituary_financestext", Some(Self::medium_text_draw), None, None, None);
        g.ecl.register_button_callbacks("obituary_uratingtext", Some(Self::medium_text_draw), None, None, None);
        g.ecl.register_button_callbacks("obituary_nratingtext", Some(Self::medium_text_draw), None, None, None);
        g.ecl.register_button_callbacks("obituary_finances", Some(textbutton_draw), None, None, None);
        g.ecl.register_button_callbacks("obituary_urating", Some(textbutton_draw), None, None, None);
        g.ecl.register_button_callbacks("obituary_nrating", Some(textbutton_draw), None, None, None);

        //
        // Special missions buttons

        g.ecl.register_button(20, 400, 120, 15, "Special Awards", "", "obituary_specialmissions");
        g.ecl.register_button_callbacks("obituary_specialmissions", Some(Self::medium_text_draw), None, None, None);

        let mut x = 30;
        let y = 420;
        let mut numcompleted = 0;

        for i in 0..16 {
            if (gob.specialmissionscompleted & (1 << i)) != 0 {
                let name = format!("obituary_award {i}");
                let tooltip = format!(
                    "Completed Special Mission '{}'",
                    PlotGenerator::special_mission_title(i)
                );
                let filename = format!("awards/award{i}.tif");

                g.ecl.register_button(x, y, 16, 16, " ", &tooltip, &name);
                button_assignbitmaps(g, &name, &filename, &filename, &filename);
                g.ecl.register_button_callbacks(&name, Some(imagebutton_draw), None, Some(button_click), Some(button_highlight));

                x += 18;
                numcompleted += 1;
            }
        }

        let numberoutof = format!("({numcompleted} out of {})", 12);
        g.ecl.register_button(190, 400, 120, 15, &numberoutof, "", "obituary_numberoutof");
        g.ecl.register_button_callbacks("obituary_numberoutof", Some(textbutton_draw), None, None, None);

        //
        // Right column

        let highsecurityhacks = format!(
            "You compromised {} high security systems",
            gob.score_highsecurityhacks
        );
        let livesruined = format!("You ruined the lives of {} people", gob.score_peoplefucked);
        let systemsdestroyed = format!("You destroyed {} computers", gob.score_systemsfucked);

        g.ecl.register_button(350, 300, 300, 15, &highsecurityhacks, "", "obituary_highsecurityhacks");
        g.ecl.register_button(350, 320, 300, 15, &systemsdestroyed, "", "obituary_systemsdestroyed");
        g.ecl.register_button(350, 340, 300, 15, &livesruined, "", "obituary_livesruined");

        g.ecl.register_button_callbacks("obituary_highsecurityhacks", Some(textbutton_draw), None, None, None);
        g.ecl.register_button_callbacks("obituary_livesruined", Some(textbutton_draw), None, None, None);
        g.ecl.register_button_callbacks("obituary_systemsdestroyed", Some(textbutton_draw), None, None, None);

        let score = format!("Final Score    {}", gob.score);

        g.ecl.register_button(360, 410, 200, 25, &score, "", "obituary_score");
        g.ecl.register_button_callbacks("obituary_score", Some(Self::large_text_draw), None, None, None);

        // Exit button

        g.ecl.register_button(screenw - 40, screenh - 40, 32, 32, "", "Return to Main Menu", "obituary_mainmenu");
        g.ecl.register_button_callback("obituary_mainmenu", Self::return_to_main_menu_click);
        button_assignbitmaps(g, "obituary_mainmenu", "mainmenu/exitgame.tif", "mainmenu/exitgame_h.tif", "mainmenu/exitgame_c.tif");

        // Secret "PI" button

        if !DEMOGAME {
            g.ecl.register_button(5, 5, 16, 16, " ", "???????????Meet the makers", "pi");
            button_assignbitmaps(g, "pi", "mainmenu/pi.tif", "mainmenu/pi_h.tif", "mainmenu/pi_c.tif");
            g.ecl.register_button_callback("pi", Self::pi_click);
        }
    }

    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            mainmenuscreen::remove(g);

            g.ecl.remove_button("obituary_name");
            g.ecl.remove_button("obituary_text");

            g.ecl.remove_button("obituary_financestext");
            g.ecl.remove_button("obituary_uratingtext");
            g.ecl.remove_button("obituary_nratingtext");
            g.ecl.remove_button("obituary_finances");
            g.ecl.remove_button("obituary_urating");
            g.ecl.remove_button("obituary_nrating");

            g.ecl.remove_button("obituary_specialmissions");
            g.ecl.remove_button("obituary_numberoutof");
            for i in 0..16 {
                g.ecl.remove_button(&format!("obituary_award {i}"));
            }

            g.ecl.remove_button("obituary_highsecurityhacks");
            g.ecl.remove_button("obituary_livesruined");
            g.ecl.remove_button("obituary_systemsdestroyed");
            g.ecl.remove_button("obituary_score");

            g.ecl.remove_button("obituary_mainmenu");

            if FULLGAME {
                g.ecl.remove_button("pi");
            }
        }
    }

    fn update(&mut self, _g: &mut Ctx) {}
    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("obituary_text").is_some()
    }
    fn screen_id(&self) -> i32 {
        MAINMENU_OBITUARY
    }
    crate::impl_as_any!();
}
