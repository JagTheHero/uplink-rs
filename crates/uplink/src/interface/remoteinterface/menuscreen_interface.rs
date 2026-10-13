//! Port of source/interface/remoteinterface/menuscreen_interface.h + source/interface/remoteinterface/menuscreen_interface.cpp

use std::rc::Rc;

use uplink_engine::{sound, text};

use crate::app::globals::ALPHA_DISABLED;
use crate::app::opengl_interface;
use crate::ctx::Ctx;
use crate::eclipse::{Button, MOVE_STRAIGHTLINE};
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::remoteinterface::remoteinterface::{RemoteInterface, SCREEN_MENUSCREEN};
use crate::interface::remoteinterface::remoteinterfacescreen::{draw_main_title, draw_sub_title};
use crate::objref::ScreenRef;
use crate::world::computer::computerscreen::menuscreen::MenuScreen;

/// Remote menu screen: the list of options a computer offers.
#[derive(Debug, Default)]
pub struct MenuScreenInterface {
    pub cs: Option<ScreenRef>,
}

impl MenuScreenInterface {
    /// `GetComputerScreen` (typed accessor; the C++ casts `cs`).
    pub fn get_computer_screen<'a>(&self, g: &'a Ctx) -> Option<&'a MenuScreen> {
        let cs = self.cs.as_ref()?;
        g.world().get_computer(&cs.computer)?.get_computer_screen(cs.index)?.as_menu_screen()
    }

    /// `DrawMenuOption` (static DrawFn)
    pub fn draw_menu_option(g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        opengl_interface::set_colour(g, "MenuText");
        text::draw_text_styled(button.x, button.y + 20, &button.caption, text::HELVETICA_18);
    }

    /// `DrawMenuOptionDimmed` (static DrawFn)
    pub fn draw_menu_option_dimmed(g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        opengl_interface::set_colour(g, "DimmedText");
        text::draw_text_styled(button.x, button.y + 20, &button.caption, text::HELVETICA_18);
    }

    /// `ClickMenuScreenOption` (static MouseFn)
    pub fn click_menu_screen_option(g: &mut Ctx, button: &Button) {
        sound::play_sound("sounds/menuitemclick.wav", None, false);

        // sscanf ( button->name, "%s %d %d", text, &unused, &nextpage )
        let Some(nextpage) = button.name.split_whitespace().nth(2).and_then(|s| s.parse::<i32>().ok()) else {
            return;
        };
        if nextpage != -1 {
            RemoteInterface::run_screen(g, nextpage);
        }
    }
}

impl InterfaceScreen for MenuScreenInterface {
    crate::impl_as_any!();

    fn create(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            let Some(ms) = self.get_computer_screen(g) else {
                debug_assert!(false, "MenuScreenInterface::create, no MenuScreen");
                return;
            };
            let maintitle = ms.base.maintitle.clone();
            let subtitle = ms.base.subtitle.clone();
            let numoptions = ms.num_options();
            // (caption, tooltip, nextpage, security) for each option
            let options: Vec<(String, String, i32, i32)> = (0..numoptions)
                .map(|i| {
                    (
                        ms.get_caption(i).to_string(),
                        ms.get_tooltip(i).to_string(),
                        ms.get_next_page(i),
                        ms.get_security(i),
                    )
                })
                .collect();

            g.ecl.register_button(0, 0, 0, 0, "", "", "menuscreeninterface");
            g.ecl.register_button_callbacks("menuscreeninterface", None, None, None, None);

            g.ecl.register_button(80, 60, 350, 25, &maintitle, "", "menuscreen_maintitle");
            g.ecl.register_button_callbacks("menuscreen_maintitle", Some(draw_main_title), None, None, None);
            g.ecl.register_button(80, 80, 350, 20, &subtitle, "", "menuscreen_subtitle");
            g.ecl.register_button_callbacks("menuscreen_subtitle", Some(draw_sub_title), None, None, None);

            // C++: int timeinterval = 1000.0 / NumOptions () (division by zero gives inf -> garbage;
            // with no options the loop below never uses it)
            let timeinterval = if numoptions != 0 { (1000.0 / numoptions as f64) as i32 } else { 0 };
            let security_level = g.interface().ri.security_level;

            for (i, (caption, tooltip, nextpage, security)) in options.iter().enumerate() {
                let i = i as i32;

                let bname1 = format!("menuscreenimage {} {}", i, nextpage);
                let bname2 = format!("menuscreentext {} {}", i, nextpage);

                if security_level <= *security {
                    // You have clearance to view this option

                    g.ecl.register_button(-350, 120 + i * 30, 16, 16, "", tooltip, &bname1);
                    opengl_interface::button_assignbitmaps(
                        g,
                        &bname1,
                        "menuscreenoption.tif",
                        "menuscreenoption_h.tif",
                        "menuscreenoption_c.tif",
                    );
                    g.ecl.register_button_callback(&bname1, Self::click_menu_screen_option);

                    g.ecl.register_button(-300, 120 + i * 30 - 5, 300, 32, caption, tooltip, &bname2);
                    g.ecl.register_button_callbacks(
                        &bname2,
                        Some(Self::draw_menu_option),
                        Some(Self::click_menu_screen_option),
                        Some(opengl_interface::button_click),
                        Some(opengl_interface::button_highlight),
                    );

                    g.ecl.register_movement(&bname1, 100, 130 + i * 30, (i + 1) * timeinterval, MOVE_STRAIGHTLINE, None);
                    g.ecl.register_movement(&bname2, 140, 130 + i * 30 - 5, (i + 1) * timeinterval, MOVE_STRAIGHTLINE, None);
                } else {
                    // You don't have clearance to view this option

                    g.ecl.register_button(-350, 120 + i * 30, 16, 16, "", tooltip, &bname1);
                    opengl_interface::button_assignbitmap(g, &bname1, "menuscreenoption.tif");
                    if let Some(img) = g.ecl.get_button_mut(&bname1).and_then(|b| b.image_standard.as_mut()) {
                        Rc::make_mut(img).set_alpha(ALPHA_DISABLED);
                    }
                    g.ecl.register_button_callbacks(
                        &bname1,
                        Some(opengl_interface::imagebutton_draw),
                        None,
                        None,
                        None,
                    );

                    g.ecl.register_button(-300, 120 + i * 30 - 5, 300, 32, caption, tooltip, &bname2);
                    g.ecl.register_button_callbacks(&bname2, Some(Self::draw_menu_option_dimmed), None, None, None);

                    g.ecl.register_movement(&bname1, 100, 130 + i * 30, (i + 1) * timeinterval, MOVE_STRAIGHTLINE, None);
                    g.ecl.register_movement(&bname2, 140, 130 + i * 30 - 5, (i + 1) * timeinterval, MOVE_STRAIGHTLINE, None);
                }
            }
        }
    }

    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            g.ecl.remove_button("menuscreeninterface");

            g.ecl.remove_button("menuscreen_maintitle");
            g.ecl.remove_button("menuscreen_subtitle");

            let nextpages: Vec<i32> = match self.get_computer_screen(g) {
                Some(ms) => (0..ms.num_options()).map(|i| ms.get_next_page(i)).collect(),
                None => {
                    debug_assert!(false, "MenuScreenInterface::remove, no MenuScreen");
                    Vec::new()
                }
            };

            for (i, nextpage) in nextpages.iter().enumerate() {
                let bname1 = format!("menuscreenimage {} {}", i, nextpage);
                let bname2 = format!("menuscreentext {} {}", i, nextpage);

                g.ecl.remove_button(&bname1);
                g.ecl.remove_button(&bname2);
            }
        }
    }

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("menuscreeninterface").is_some()
    }

    fn screen_id(&self) -> i32 {
        SCREEN_MENUSCREEN
    }

    fn set_computer_screen(&mut self, cs: ScreenRef) {
        self.cs = Some(cs);
    }

    fn computer_screen_ref(&self) -> Option<&ScreenRef> {
        self.cs.as_ref()
    }
}
