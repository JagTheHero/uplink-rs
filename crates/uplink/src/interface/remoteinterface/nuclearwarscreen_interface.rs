//! Port of source/interface/remoteinterface/nuclearwarscreen_interface.h + source/interface/remoteinterface/nuclearwarscreen_interface.cpp

use uplink_engine::{gl, sound, text};

use crate::app::opengl::{screen_h, screen_w};
use crate::app::opengl_interface::{
    button_assignbitmap, button_click, button_highlight, clear_draw, imagebutton_drawtextured, sx, sy,
    textbutton_draw,
};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::game::data::data::{PhysicalGatewayLocation, PHYSICALGATEWAYLOCATIONS};
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::remoteinterface::remoteinterface::{RemoteInterface, SCREEN_NUCLEARWAR};
use crate::interface::remoteinterface::remoteinterfacescreen::draw_main_title;
use crate::objref::ScreenRef;
use crate::world::company::companyuplink::CompanyUplink;
use crate::world::company::news::News;
use crate::world::computer::computerscreen::genericscreen::GenericScreen;
use crate::world::connection::Connection;
use crate::world::generator::numbergenerator::NumberGenerator;
use crate::world::generator::plotgenerator::SPECIALMISSION_WARGAMES;

/// The gateway location named by a "nuclearwar_location N" button (C++ `sscanf(name, "%s %d", ...)`).
fn location_of(name: &str) -> Option<&'static PhysicalGatewayLocation> {
    let index = name.split_whitespace().nth(1)?.parse::<usize>().ok()?;
    PHYSICALGATEWAYLOCATIONS.get(index)
}

/// One nuke on the nuclear war map (C++ `NuclearWar_Nuke`).
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Default)]
pub struct NuclearWar_Nuke {
    pub sx: i32,
    pub sy: i32,
    pub x: i32,
    pub y: i32,
    pub time: i32,
    pub sound: bool,
}

/// Remote nuclear war screen: animates nukes over a world map (C++ `LList<NuclearWar_Nuke *>`).
#[derive(Debug, Default)]
pub struct NuclearWarScreenInterface {
    pub cs: Option<ScreenRef>,
    pub nukes: Vec<NuclearWar_Nuke>,
}

impl NuclearWarScreenInterface {
    /// `GetComputerScreen` (typed accessor; the C++ casts `cs`).
    pub fn get_computer_screen<'a>(&self, g: &'a Ctx) -> Option<&'a GenericScreen> {
        let cs = self.cs.as_ref()?;
        g.world().get_computer(&cs.computer)?.get_computer_screen(cs.index)?.as_generic_screen()
    }

    /// `DrawBlack` (static DrawFn)
    pub fn draw_black(g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        clear_draw(g, button.x, button.y, button.width, button.height);
    }

    /// `DrawLocation` (static DrawFn)
    pub fn draw_location(_g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        gl::color4f(1.0, 1.0, 1.0, 1.0);
        gl::begin(gl::QUADS);
        gl::vertex2i(button.x, button.y);
        gl::vertex2i(button.x + 7, button.y);
        gl::vertex2i(button.x + 7, button.y + 7);
        gl::vertex2i(button.x, button.y + 7);
        gl::end();

        // Write some text
        let Some(pgl) = location_of(&button.name) else { return };
        text::draw_text_default(button.x - 10, button.y + 20, pgl.city);
    }

    /// `ClickLocation` (static MouseFn)
    pub fn click_location(g: &mut Ctx, button: &Button) {
        // Launch a nuke at the players physical location
        let Some(pgl) = location_of(&button.name) else { return };

        let mut nuke = NuclearWar_Nuke::default();
        nuke.sx = NumberGenerator::random_number(640);
        nuke.sy = 0;
        nuke.x = NumberGenerator::random_normal_number(sx(pgl.x) as f32, 20.0) as i32;
        nuke.y = NumberGenerator::random_normal_number(sy(pgl.y) as f32, 20.0) as i32;
        nuke.time = g.ecl.get_accurate_time();
        nuke.sound = false;
        if let Some(nwsi) = g.interface_mut().ri.screen.get_mut::<NuclearWarScreenInterface>() {
            nwsi.nukes.push(nuke);
        }

        sound::play_sound("sounds/zap.wav", None, false);
    }

    /// `DrawMainMap` (static DrawFn)
    pub fn draw_main_map(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        imagebutton_drawtextured(g, button, highlighted, clicked);

        // The nukes are taken out of the screen while they are drawn, so the eclipse clock can be read.
        let Some(mut nukes) = g
            .interface_mut()
            .ri
            .screen
            .get_mut::<NuclearWarScreenInterface>()
            .map(|nwsi| std::mem::take(&mut nwsi.nukes))
        else {
            return;
        };

        nukes.retain_mut(|nuke| {
            let timediff = g.ecl.get_accurate_time() - nuke.time;

            if timediff > 10000 {
                // The nuke has finished: drop it
                return false;
            }

            if timediff > 3000 {
                let d = (timediff - 3000) as f32 / 7000.0;

                let dx = nuke.x + button.x;
                let dy = nuke.y + button.y;

                let sx_pos = (nuke.sx as f32 + ((nuke.x + button.x) - nuke.sx) as f32 * d) as i32;
                let sy_pos = (nuke.sy as f32 + ((nuke.y + button.y) - nuke.sy) as f32 * d) as i32;

                let col = 1.0 - d;
                gl::color3f(col, 0.0, 0.0);

                gl::begin(gl::LINE_LOOP);
                gl::vertex2i(sx_pos, sy_pos);
                gl::vertex2i(dx, dy);
                gl::end();

                // Draw the explosion
                let centre_x = dx + 3;
                let centre_y = dy + 3;
                let width = (50.0 - (50.0 * d)) as i32;
                let height = (50.0 - (50.0 * d)) as i32;

                if timediff < 3200 {
                    gl::color3f(1.0, 0.8, 0.0);
                } else {
                    gl::color3f(col, 0.0, 0.0);
                }

                gl::begin(gl::QUADS);
                gl::vertex2i(centre_x, centre_y - height / 2);
                gl::vertex2i(centre_x + width / 2, centre_y);
                gl::vertex2i(centre_x, centre_y + height / 2);
                gl::vertex2i(centre_x - width / 2, centre_y);
                gl::end();

                if !nuke.sound {
                    let explosion = format!("sounds/explosion{}.wav", NumberGenerator::random_number(3) + 1);
                    sound::play_sound(&explosion, None, false);
                    nuke.sound = true;
                }
            } else {
                // Still in flight
                let d = timediff as f32 / 3000.0;
                let dx = (nuke.sx as f32 + ((nuke.x + button.x) - nuke.sx) as f32 * d) as i32;
                let dy = (nuke.sy as f32 + ((nuke.y + button.y) - nuke.sy) as f32 * d) as i32;

                let col = timediff as f32 / 3000.0;
                gl::color3f(col, col, col);
                gl::begin(gl::LINE_LOOP);
                gl::vertex2i(nuke.sx, nuke.sy);
                gl::vertex2i(dx, dy);
                gl::end();
            }

            true
        });

        if let Some(nwsi) = g.interface_mut().ri.screen.get_mut::<NuclearWarScreenInterface>() {
            nwsi.nukes = nukes;
        }
    }

    /// `CloseClick` (static MouseFn)
    pub fn close_click(g: &mut Ctx, _button: &Button) {
        if !g.world().plotgenerator.player_completed_special_mission(SPECIALMISSION_WARGAMES) {
            g.world_mut().plotgenerator.specialmissionscompleted |= 1 << SPECIALMISSION_WARGAMES;

            let mut news = News::new();
            news.set_headline("Government run Nuclear Defense system compromised");
            news.set_details(concat!(
                "It emerged today that a high-profile experimental Government computer system ",
                "was very nearly fooled into starting World War III.  The system began receiving ",
                "information that major cities were being attacked by enemy Nuclear Weapons.  The ",
                "system raised the defense status to Defcon2 before somebody pulled the plug.\n\n",
                "'Its not yet clear how this happened' commented a senior member of the military. ",
                "'However we do know that a hacker was involved, probably playing some kind of prank ",
                "on the world.  Let me tell you this - that prank nearly wiped out the Western Hemisphere.\n\n",
            ));
            CompanyUplink::create_news(g, news);
        }

        Connection::disconnect(g, "PLAYER");
        Connection::reset(g, "PLAYER");

        RemoteInterface::run_new_location(g);
        RemoteInterface::run_screen(g, 1);
    }
}

impl InterfaceScreen for NuclearWarScreenInterface {
    crate::impl_as_any!();

    fn create(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            let Some(gs) = self.get_computer_screen(g) else { return };
            let subtitle = gs.base.subtitle.clone();

            g.ecl.register_button(40, 10, 350, 25, &subtitle, "", "nuclearwar_maintitle");
            g.ecl.register_button_callbacks("nuclearwar_maintitle", Some(draw_main_title), None, None, None);

            // Work out the size/ratios of the map
            let screenw = screen_w();
            let screenh = screen_h();

            let x1 = 23;
            let y1 = 50;
            let fullsize_x = screenw - x1 * 2;
            let fullsize_y = (316.0 * (fullsize_x as f64 / 595.0)) as i32;

            // Black out the background
            g.ecl.register_button(0, 0, screenw, screenh, " ", " ", "nuclearwar_black");
            g.ecl.register_button_callbacks("nuclearwar_black", Some(Self::draw_black), None, None, None);

            // Create the large map graphic
            g.ecl.register_button(x1, y1, fullsize_x, fullsize_y, "", "", "nuclearwar_largemap");
            button_assignbitmap(g, "nuclearwar_largemap", "worldmaplarge.tif");
            g.ecl.register_button_callbacks("nuclearwar_largemap", Some(Self::draw_main_map), None, None, None);

            // Create a button for each gateway
            for (i, pgl) in PHYSICALGATEWAYLOCATIONS.iter().enumerate() {
                let bname = format!("nuclearwar_location {i}");
                g.ecl.register_button(
                    x1 + sx(pgl.x),
                    y1 + sy(pgl.y),
                    7,
                    7,
                    pgl.city,
                    "Simulate attack on this site",
                    &bname,
                );
                g.ecl.register_button_callbacks(
                    &bname,
                    Some(Self::draw_location),
                    Some(Self::click_location),
                    Some(button_click),
                    Some(button_highlight),
                );
            }

            // Create some helpfull text
            g.ecl.register_button(50, y1 + fullsize_y + 20, 400, 70, " ", " ", "nuclearwar_helptext");
            g.ecl.register_button_callbacks("nuclearwar_helptext", Some(textbutton_draw), None, None, None);
            g.ecl.register_caption_change(
                "nuclearwar_helptext",
                "Good Evening, Dr Falken.\nWould you like to play a game?",
                2000,
                None,
            );

            // Exit button
            g.ecl.register_button(540, 465, 100, 15, "Close", "How about a nice game of chess?", "nuclearwar_close");
            g.ecl.register_button_callback("nuclearwar_close", Self::close_click);

            // Bring some buttons forward
            g.ecl.button_bring_to_front("hud_date");
        }
    }

    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            g.ecl.remove_button("nuclearwar_maintitle");
            g.ecl.remove_button("nuclearwar_black");
            g.ecl.remove_button("nuclearwar_largemap");

            for i in 0..PHYSICALGATEWAYLOCATIONS.len() {
                g.ecl.remove_button(&format!("nuclearwar_location {i}"));
            }

            g.ecl.remove_button("nuclearwar_helptext");
            g.ecl.remove_button("nuclearwar_close");
        }
    }

    fn update(&mut self, g: &mut Ctx) {
        g.ecl.dirty_button("nuclearwar_largemap");
    }

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("nuclearwar_largemap").is_some()
    }

    fn screen_id(&self) -> i32 {
        SCREEN_NUCLEARWAR
    }

    fn return_key_pressed(&mut self, _g: &mut Ctx) -> bool {
        false
    }

    fn set_computer_screen(&mut self, cs: ScreenRef) {
        self.cs = Some(cs);
    }

    fn computer_screen_ref(&self) -> Option<&ScreenRef> {
        self.cs.as_ref()
    }
}
