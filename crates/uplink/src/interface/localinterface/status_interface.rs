//! Port of source/interface/localinterface/status_interface.h + source/interface/localinterface/status_interface.cpp
//! Status screen: the player's Uplink rating, neuromancer rating, criminal record and special-mission awards.

use uplink_engine::text;

use crate::app::opengl_interface::{button_assignbitmaps, button_click, button_highlight, imagebutton_draw, set_colour, text_draw};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::game::data::data::NUM_UPLINKRATINGS;
use crate::impl_as_any;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::localinterface::localinterface::{LocalInterface, PANELSIZE, SCREEN_NONE, SCREEN_STATUS};
use crate::interface::localinterface::localinterfacescreen;
use crate::world::generator::plotgenerator::PlotGenerator;
use crate::world::generator::recordgenerator::RecordGenerator;

#[derive(Debug, Clone, Default)]
pub struct StatusInterface {}

impl StatusInterface {
    /// C++ `StatusInterface::TitleClick` (static button callback).
    pub fn title_click(g: &mut Ctx, _button: &Button) {
        LocalInterface::run_screen(g, SCREEN_NONE, -1);
    }

    /// C++ `StatusInterface::MiniTitleDraw` (static draw callback).
    pub fn mini_title_draw(g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        set_colour(g, "TitleText");
        text::draw_text_styled(button.x + 10, button.y + 10, &button.caption, text::HELVETICA_18);
    }
}

impl InterfaceScreen for StatusInterface {
    fn create(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            return;
        }

        localinterfacescreen::create(g);

        let screenw = g.app.options.get_option_value("graphics_screenwidth");
        let paneltop = (100.0 * ((screenw as f64 * PANELSIZE) / 188.0) + 30.0) as i32;
        let panelwidth = (screenw as f64 * PANELSIZE) as i32;

        g.ecl.register_button(screenw - panelwidth, paneltop + 3, panelwidth - 7, 15, "STATUS", "Remove the status screen", "status_title");
        g.ecl.register_button_callback("status_title", Self::title_click);

        //
        // Ratings
        //

        g.ecl.register_button(screenw - panelwidth + 50, paneltop + 40, 80, 20, "Ratings", "", "status_ratingstitle");
        g.ecl.register_button_callbacks("status_ratingstitle", Some(Self::mini_title_draw), None, None, None);

        g.ecl.register_button(screenw - panelwidth, paneltop + 70, panelwidth - 5, 30, "", "", "status_uplinkrating");
        g.ecl.register_button(screenw - panelwidth, paneltop + 120, panelwidth - 5, 30, "", "", "status_neuromancerrating");
        g.ecl.register_button_callbacks("status_uplinkrating", Some(text_draw), None, None, None);
        g.ecl.register_button_callbacks("status_neuromancerrating", Some(text_draw), None, None, None);

        let urating = format!(
            "GRADE {} UPLINK AGENT\nRating: {}",
            NUM_UPLINKRATINGS - g.player().rating.uplinkrating,
            g.player().rating.get_uplink_rating()
        );
        let nrating = format!("NEUROMANCER (Unofficial)\nRating: {}", g.player().rating.get_neuromancer_rating());
        g.ecl.register_caption_change_auto("status_uplinkrating", &urating, None);
        g.ecl.register_caption_change_auto("status_neuromancerrating", &nrating, None);

        //
        // Legal
        //

        g.ecl.register_button(screenw - panelwidth + 50, paneltop + 190, 80, 20, "Criminal", "", "status_legaltitle");
        g.ecl.register_button_callbacks("status_legaltitle", Some(Self::mini_title_draw), None, None, None);

        let crimrec = RecordGenerator::get_criminal(g, "PLAYER")
            .and_then(|rec| rec.get_field("Convictions"))
            .map(str::to_string)
            .unwrap_or_else(|| "None".to_string());

        g.ecl.register_button(screenw - panelwidth + 5, paneltop + 220, panelwidth - 15, 60, "", "", "status_legal");
        g.ecl.register_button_callbacks("status_legal", Some(text_draw), None, None, None);
        g.ecl.register_caption_change_auto("status_legal", &crimrec, None);

        //
        // Awards
        //

        let mut x = screenw - panelwidth + 2;
        let y = 405;
        for i in 0..16 {
            if g.world().plotgenerator.player_completed_special_mission(i) {
                let name = format!("status_award {i}");
                let tooltip = format!("Completed Special Mission '{}'", PlotGenerator::special_mission_title(i));
                let filename = format!("awards/award{i}.tif");

                g.ecl.register_button(x, y, 16, 16, " ", &tooltip, &name);
                button_assignbitmaps(g, &name, &filename, &filename, &filename);
                g.ecl.register_button_callbacks(&name, Some(imagebutton_draw), None, Some(button_click), Some(button_highlight));
                x += 18;
            }
        }
    }

    fn remove(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            return;
        }

        localinterfacescreen::remove(g);

        g.ecl.remove_button("status_title");
        g.ecl.remove_button("status_ratingstitle");
        g.ecl.remove_button("status_uplinkrating");
        g.ecl.remove_button("status_neuromancerrating");
        g.ecl.remove_button("status_legaltitle");
        g.ecl.remove_button("status_legal");
        for i in 0..16 {
            g.ecl.remove_button(&format!("status_award {i}"));
        }
    }

    /// The C++ body is only a comment: ratings are deliberately not re-checked every update.
    fn update(&mut self, _g: &mut Ctx) {}

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("status_title").is_some()
    }

    fn screen_id(&self) -> i32 {
        SCREEN_STATUS
    }

    impl_as_any!();
}
