//! Port of source/interface/localinterface/cheat_interface.h + source/interface/localinterface/cheat_interface.cpp
//! Cheat-codes screen (debug/test builds only: every callback body is guarded by `globals::TESTGAME`).
//! PORT: also enabled at run time by `UPLINK_CHEATS=1` (play-test harness, docs/PLAYTEST.md).

use crate::app::app::App;
use crate::app::globals;
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::game::data::data::NUM_UPLINKRATINGS;
use crate::impl_as_any;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::localinterface::localinterface::{LocalInterface, PANELSIZE, SCREEN_CHEATS, SCREEN_EVTQUEUE, SCREEN_NONE};
use crate::interface::localinterface::localinterfacescreen;
use crate::world::computer::lancomputer::{LANLINKVISIBLE_AWARE, LANSYSTEMVISIBLE_FULL};
use crate::world::date::Date;
use crate::world::generator::plotgenerator::PlotGenerator;
use crate::world::generator::worldgenerator::WorldGenerator;
use crate::world::person::Person;
use crate::world::rating::Rating;
use crate::world::scheduler::shotbyfedsevent::ShotByFedsEvent;
use crate::world::scheduler::uplinkevent::UplinkEvent;

#[derive(Debug, Clone, Default)]
pub struct CheatInterface {}

impl CheatInterface {
    /// C++ `CheatInterface::TitleClick` (static button callback).
    pub fn title_click(g: &mut Ctx, _button: &Button) {
        if globals::TESTGAME || globals::cheatmodes_enabled() {
            LocalInterface::run_screen(g, SCREEN_NONE, -1);
        }
    }

    /// C++ `CheatInterface::AllLinksClick`
    pub fn all_links_click(g: &mut Ctx, _button: &Button) {
        if globals::TESTGAME || globals::cheatmodes_enabled() {
            crate::world::player::Player::give_all_links(g);
        }
    }

    /// C++ `CheatInterface::AllSoftwareClick`
    pub fn all_software_click(g: &mut Ctx, _button: &Button) {
        if globals::TESTGAME || globals::cheatmodes_enabled() {
            g.player_mut().gateway.set_memory_size(256);
            g.player_mut().gateway.give_all_software();
        }
    }

    /// C++ `CheatInterface::AllHardwareClick`
    pub fn all_hardware_click(g: &mut Ctx, _button: &Button) {
        if globals::TESTGAME || globals::cheatmodes_enabled() {
            crate::world::computer::gateway::Gateway::give_all_hardware(g);
        }
    }

    /// C++ `CheatInterface::LotsOfMoneyClick`
    pub fn lots_of_money_click(g: &mut Ctx, _button: &Button) {
        if globals::TESTGAME || globals::cheatmodes_enabled() {
            Person::change_balance(g, "PLAYER", 10000, "You cheating motherfucker!");
        }
    }

    /// C++ `CheatInterface::MaxRatingsClick`
    pub fn max_ratings_click(g: &mut Ctx, _button: &Button) {
        if globals::TESTGAME || globals::cheatmodes_enabled() {
            g.player_mut().rating.set_uplink_rating(NUM_UPLINKRATINGS - 1);
            // C++ SetNeuromancerRating(NUM_NEUROMANCERRATINGS-1) was commented out.
        }
    }

    /// C++ `CheatInterface::NextRatingClick`
    pub fn next_rating_click(g: &mut Ctx, _button: &Button) {
        if globals::TESTGAME || globals::cheatmodes_enabled() {
            let next = g.player().rating.uplinkrating + 1;
            g.player_mut().rating.set_uplink_rating(next);
            Rating::change_uplink_score(g, "PLAYER", 1);
        }
    }

    /// C++ `CheatInterface::DebugPrintClick`
    pub fn debug_print_click(g: &mut Ctx, _button: &Button) {
        if globals::TESTGAME || globals::cheatmodes_enabled() {
            App::print(g);
            g.ecl.debug_print();
            g.svb.debug_print();
        }
    }

    /// C++ `CheatInterface::EndGameClick`
    pub fn end_game_click(g: &mut Ctx, _button: &Button) {
        if globals::TESTGAME || globals::cheatmodes_enabled() {
            // Simulate a "shot by feds" event in 5 seconds

            let mut duedate = Date::default();
            duedate.set_date_from(&g.world().date);
            duedate.advance_second(5);

            let mut event = ShotByFedsEvent::new();
            event.set_name("PLAYER");
            event.set_reason("clicking on the END GAME button you fucking wanker");
            event.base.set_run_date(&duedate);

            g.world_mut().scheduler.schedule_event(UplinkEvent::ShotByFedsEvent(event));
        }
    }

    /// C++ `CheatInterface::CancelTraceClick`
    pub fn cancel_trace_click(g: &mut Ctx, _button: &Button) {
        if globals::TESTGAME || globals::cheatmodes_enabled() {
            g.player_mut().get_connection_mut().end_trace();
        }
    }

    /// C++ `CheatInterface::EventQueueClick`
    pub fn event_queue_click(g: &mut Ctx, _button: &Button) {
        if globals::TESTGAME || globals::cheatmodes_enabled() {
            LocalInterface::run_screen(g, SCREEN_EVTQUEUE, -1);
        }
    }

    /// C++ `CheatInterface::RevelationClick`
    pub fn revelation_click(g: &mut Ctx, _button: &Button) {
        if globals::TESTGAME || globals::cheatmodes_enabled() {
            for _i in 0..5 {
                let comp_name = WorldGenerator::get_random_computer(g);
                let ip = g.world().get_computer(&comp_name).expect("UplinkAssert(comp)").ip.clone();
                PlotGenerator::run_revelation(g, &ip, 3.0, true);
            }
        }
    }

    /// C++ `CheatInterface::ShowLANClick`
    pub fn show_lan_click(g: &mut Ctx, _button: &Button) {
        if globals::TESTGAME || globals::cheatmodes_enabled() {
            let remote_name = {
                let world = g.world();
                let vl = g.player().get_remote_host(world).expect("UplinkAssert(vl)");
                vl.get_computer(world).expect("UplinkAssert(comp)").name.clone()
            };

            // C++ returns early unless the computer is a LAN computer.
            let Some(comp) = g.world_mut().get_computer_mut(&remote_name) else { return };
            let Some(lan) = comp.lan_mut() else { return };

            for i in 0..lan.systems.size() {
                if lan.systems.valid_index(i) {
                    let system = lan.systems.get_data_mut(i).expect("UplinkAssert(system)");
                    system.visible = LANSYSTEMVISIBLE_FULL;
                }
            }

            for j in 0..lan.links.size() {
                if lan.links.valid_index(j) {
                    let link = lan.links.get_data_mut(j).expect("UplinkAssert(link)");
                    link.visible = LANLINKVISIBLE_AWARE;
                }
            }
        }
    }
}

impl InterfaceScreen for CheatInterface {
    fn create(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            return;
        }

        localinterfacescreen::create(g);

        let screenw = g.app.options.get_option_value("graphics_screenwidth");
        let _screenh = g.app.options.get_option_value("graphics_screenheight");
        let paneltop = (100.0 * ((screenw as f64 * PANELSIZE) / 188.0) + 30.0) as i32;
        let panelwidth = (screenw as f64 * PANELSIZE) as i32;

        // Title

        g.ecl.register_button(screenw - panelwidth, paneltop + 3, panelwidth - 7, 15, "CHEATS", "Remove the cheats screen", "cheat_title");
        g.ecl.register_button_callback("cheat_title", Self::title_click);

        // Give all links

        g.ecl.register_button(screenw - panelwidth, paneltop + 50, panelwidth - 7, 15, "Give all links", " ", "cheat_alllinks");
        g.ecl.register_button_callback("cheat_alllinks", Self::all_links_click);

        // All software

        g.ecl.register_button(screenw - panelwidth, paneltop + 70, panelwidth - 7, 15, "Give all software", " ", "cheat_allsoftware");
        g.ecl.register_button_callback("cheat_allsoftware", Self::all_software_click);

        // All hardware

        g.ecl.register_button(screenw - panelwidth, paneltop + 90, panelwidth - 7, 15, "Give all hardware", " ", "cheat_allhardware");
        g.ecl.register_button_callback("cheat_allhardware", Self::all_hardware_click);

        // Lots of money

        g.ecl.register_button(screenw - panelwidth, paneltop + 110, panelwidth - 7, 15, "Give lots of money", " ", "cheat_money");
        g.ecl.register_button_callback("cheat_money", Self::lots_of_money_click);

        // Next rating

        g.ecl.register_button(screenw - panelwidth, paneltop + 130, panelwidth - 7, 15, "Give next rating", " ", "cheat_nextrating");
        g.ecl.register_button_callback("cheat_nextrating", Self::next_rating_click);

        // Maximum ratings

        g.ecl.register_button(screenw - panelwidth, paneltop + 150, panelwidth - 7, 15, "Give max ratings", " ", "cheat_maxratings");
        g.ecl.register_button_callback("cheat_maxratings", Self::max_ratings_click);

        // Cancel trace

        g.ecl.register_button(screenw - panelwidth, paneltop + 170, panelwidth - 7, 15, "Cancel current trace", " ", "cheat_canceltrace");
        g.ecl.register_button_callback("cheat_canceltrace", Self::cancel_trace_click);

        // End game

        g.ecl.register_button(screenw - panelwidth, paneltop + 190, panelwidth - 7, 15, "End Game", " ", "cheat_endgame");
        g.ecl.register_button_callback("cheat_endgame", Self::end_game_click);

        // Debug Print

        g.ecl.register_button(screenw - panelwidth, paneltop + 210, panelwidth - 7, 15, "Debug Print", " ", "cheat_debugprint");
        g.ecl.register_button_callback("cheat_debugprint", Self::debug_print_click);

        // Event Queue

        g.ecl.register_button(screenw - panelwidth, paneltop + 230, panelwidth - 7, 15, "Event Queue", " ", "cheat_eventqueue");
        g.ecl.register_button_callback("cheat_eventqueue", Self::event_queue_click);

        // Run Revelation

        g.ecl.register_button(screenw - panelwidth, paneltop + 250, panelwidth - 7, 15, "Run Revelation", " ", "cheat_revelation");
        g.ecl.register_button_callback("cheat_revelation", Self::revelation_click);

        // Show LAN

        g.ecl.register_button(screenw - panelwidth, paneltop + 270, panelwidth - 7, 15, "Show Local Area Network", " ", "cheat_showlan");
        g.ecl.register_button_callback("cheat_showlan", Self::show_lan_click);
    }

    fn remove(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            return;
        }

        localinterfacescreen::remove(g);

        g.ecl.remove_button("cheat_title");

        g.ecl.remove_button("cheat_alllinks");
        g.ecl.remove_button("cheat_allsoftware");
        g.ecl.remove_button("cheat_allhardware");
        g.ecl.remove_button("cheat_money");
        g.ecl.remove_button("cheat_nextrating");
        g.ecl.remove_button("cheat_maxratings");
        g.ecl.remove_button("cheat_endgame");
        g.ecl.remove_button("cheat_canceltrace");
        g.ecl.remove_button("cheat_debugprint");
        g.ecl.remove_button("cheat_eventqueue");
        g.ecl.remove_button("cheat_revelation");
        g.ecl.remove_button("cheat_showlan");
    }

    fn update(&mut self, _g: &mut Ctx) {}

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("cheat_title").is_some()
    }

    fn screen_id(&self) -> i32 {
        SCREEN_CHEATS
    }

    impl_as_any!();
}
