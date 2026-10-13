//! Port of source/game/scriptlibrary.h + source/game/scriptlibrary.cpp
//!
//! Script library static class: a collection of functions identified only by an ID number.
//! They encode specific game events that happen only once in a game of Uplink, and always under
//! the same situation (storyline, start sequence, special events). PORT: `ScriptLibrary` is a unit
//! struct of associated fns taking `g`; `run_script` has the `TimerFn` signature of
//! `app::opengl::timer_func` (the C++ scheduled most scripts with `GciTimerFunc`).

#![allow(unused_variables, unused_imports, unused_mut, dead_code)]

use uplink_engine::{gl, sound, text};

use crate::app::app::App;
use crate::app::globals::TESTGAME;
use crate::app::opengl::timer_func;
use crate::app::opengl_interface::{
    button_assignbitmap, button_assignbitmaps, button_click, button_highlight, create_msgbox,
    imagebutton_draw, textbutton_draw,
};
use crate::ctx::Ctx;
use crate::eclipse::{Button, MOVE_STRAIGHTLINE};
use crate::game::data::data::{
    COST_UPLINK_NEWGATEWAY, IP_UPLINKINTERNALSERVICES, IP_UPLINKPUBLICACCESSSERVER,
    IP_UPLINKTESTMACHINE, NAME_UPLINKINTERNALSERVICES, PAYMENT_SPECIALMISSION_MOLE,
    PLAYER_START_BALANCE,
};
use crate::game::game::{Game, GAMESPEED_PAUSED};
use crate::interface::interfacescreen::{InterfaceScreen, ScreenSlot};
use crate::interface::localinterface::localinterface::{LocalInterface, SCREEN_LAN, SCREEN_NONE};
use crate::interface::localinterface::phonedialler::PhoneDialler;
use crate::interface::remoteinterface::academicscreen_interface::AcademicScreenInterface;
use crate::interface::remoteinterface::criminalscreen_interface::CriminalScreenInterface;
use crate::interface::remoteinterface::radiotransmitterscreen_interface::RadioTransmitterScreenInterface;
use crate::interface::remoteinterface::remoteinterface::RemoteInterface;
use crate::interface::remoteinterface::socialsecurityscreen_interface::SocialSecurityScreenInterface;
use crate::interface::taskmanager::taskmanager::TaskManager;
use crate::mainmenu::mainmenu::{MainMenu, MAINMENU_LOGIN};
use crate::world::company::companyuplink::CompanyUplink;
use crate::world::company::mission::Mission;
use crate::world::company::news::News;
use crate::world::computer::bankaccount::BankAccount;
use crate::world::computer::computer::{Computer, COMPUTER_TYPE_LAN};
use crate::world::computer::computerscreen::computerscreen::ComputerScreen;
use crate::world::computer::lancomputer::{
    LanComputerSystem, LANSYSTEM_AUTHENTICATION, LANSYSTEM_ISOLATIONBRIDGE,
};
use crate::world::computer::lanmonitor::LanMonitor;
use crate::world::computer::recordbank::{
    Record, RECORDBANK_ACCNO, RECORDBANK_NAME, RECORDBANK_PASSWORD, RECORDBANK_SECURITY,
};
use crate::world::connection::Connection;
use crate::world::date::Date;
use crate::world::generator::missiongenerator::{MISSION_SPECIAL, MISSION_STEALFILE};
use crate::world::generator::namegenerator::NameGenerator;
use crate::world::generator::numbergenerator::NumberGenerator;
use crate::world::generator::plotgenerator::{PlotGenerator, SPECIALMISSION_MOLE};
use crate::world::generator::recordgenerator::RecordGenerator;
use crate::world::generator::worldgenerator::WorldGenerator;
use crate::world::message::Message;
use crate::world::person::{Person, PERSON_STATUS_DEAD};
use crate::world::scheduler::notificationevent::{
    NotificationEvent, NOTIFICATIONEVENT_TYPE_BUYAGENTLIST,
};
use crate::world::scheduler::uplinkevent::UplinkEvent;

pub struct ScriptLibrary;

impl ScriptLibrary {
    /// Misc drawing function for use in these scripts: the connection animation line (DrawFn).
    pub fn draw_connection(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        let screenheight = g.app.options.get_option_value("graphics_screenheight");
        gl::scissor(
            button.x,
            screenheight - (button.y + button.height),
            button.width,
            button.height,
        );
        gl::enable(gl::SCISSOR_TEST);

        gl::color4f(0.3, 0.5, 1.0, 1.0);
        gl::line_width(2.0);

        if button.height == 4 {
            // Horizontal button
            gl::begin(gl::LINES);
            gl::vertex2i(button.x + 1, button.y + 1);
            gl::vertex2i(button.x + 1 + button.width - 1, button.y + 1);
            gl::end();
        } else if button.width == 4 {
            // Vertical button
            gl::begin(gl::LINES);
            gl::vertex2i(button.x + 1, button.y + 1);
            gl::vertex2i(button.x + 1, button.y + button.height - 1);
            gl::end();
        } else {
            // Diagonal button
            gl::begin(gl::LINES);
            gl::vertex2i(button.x + 1, button.y + 1);
            gl::vertex2i(button.x + button.width - 1, button.y + button.height - 1);
            gl::end();
        }

        gl::line_width(1.0);
        gl::disable(gl::SCISSOR_TEST);
    }

    /// Runs script number `scriptindex`; an unknown index only warns.
    pub fn run_script(g: &mut Ctx, scriptindex: i32) {
        match scriptindex {
            1 => Self::script1(g),
            10 => Self::script10(g),
            11 => Self::script11(g),
            12 => Self::script12(g),
            13 => Self::script13(g),
            15 => Self::script15(g),
            16 => Self::script16(g),
            17 => Self::script17(g),
            30 => Self::script30(g),
            31 => Self::script31(g),
            32 => Self::script32(g),
            33 => Self::script33(g),
            34 => Self::script34(g),
            35 => Self::script35(g),
            36 => Self::script36(g),
            40 => Self::script40(g),
            41 => Self::script41(g),
            42 => Self::script42(g),
            43 => Self::script43(g),
            45 => Self::script45(g),
            46 => Self::script46(g),
            47 => Self::script47(g),
            50 => Self::script50(g),
            51 => Self::script51(g),
            60 => Self::script60(g),
            61 => Self::script61(g),
            62 => Self::script62(g),
            63 => Self::script63(g),
            70 => Self::script70(g),
            71 => Self::script71(g),
            72 => Self::script72(g),
            80 => Self::script80(g),
            81 => Self::script81(g),
            82 => Self::script82(g),
            83 => Self::script83(g),
            85 => Self::script85(g),
            _ => eprintln!("WARNING: Unknown script index {scriptindex}"),
        }
    }

    /// Test script
    pub fn script1(g: &mut Ctx) {
        // Removed from the game. Sends an email to the player.
        let mut msg = Message::new();
        msg.set_to("PLAYER");
        msg.set_from("Script Library");
        msg.set_subject("Test message from Script Library");
        msg.set_body("Body of message.  Looks like it works!");
        msg.send(g);
    }

    /// Bank money transfers
    pub fn script10(g: &mut Ctx) {
        // Get the source bank computer
        let source_comp = remote_computer_name(g).expect("Script10: no remote computer");
        let (source_ip, proxy_running) = {
            let comp = g.world().get_computer(&source_comp).expect("Script10: remote computer");
            (comp.ip.clone(), comp.security.is_running_proxy())
        };

        // Get the source and target values
        let target_ip = caption_of(g, "t_ip_value 0 0");
        let source_accno = caption_of(g, "s_accno_value -1 -1");
        let target_accno = caption_of(g, "t_accno_value 0 0");
        let value = caption_of(g, "amount_value 0 0");

        let i_value = scan_int(&value);

        // Check the player has filled in the correct values
        if target_accno == "Fill this in"
            || target_accno == "Fill this in"
            || value == "Fill this in"
        {
            return;
        }

        // Prevent if it's not his account and a proxy is running
        if !g.player().is_player_account(&source_ip, &source_accno) && proxy_running {
            create_msgbox(g, "Error", "Denied access by Proxy server");
            return;
        }

        // Do the transfer
        let success = BankAccount::transfer_money(
            g,
            &source_ip,
            &source_accno,
            &target_ip,
            &target_accno,
            i_value,
            "PLAYER",
        );

        if success {
            // Change to a success screen
            RemoteInterface::run_screen(g, 8);
        } else {
            // Transfer failed
            RemoteInterface::run_screen(g, 9);
        }
    }

    /// Create new bank account
    pub fn script11(g: &mut Ctx) {
        // Get the inputted properties of the new account
        let name = caption_of(g, "nametext 0 0");
        let passwd = caption_of(g, "passwordtext 0 0");
        let passwd2 = caption_of(g, "passwordtext2 0 0");

        if passwd != passwd2 {
            create_msgbox(g, "Error", "The two passwords must be identical");
            return;
        }

        if name == "Fill this in" {
            create_msgbox(g, "Error", "You must enter a username");
            return;
        }

        // Get the bank computer
        let comp_name = remote_computer_name(g).expect("Script11: no remote computer");
        let comp_ip = g.world().get_computer(&comp_name).expect("Script11: bank computer").ip.clone();

        // Create the account
        let accno = Person::create_new_account(g, "PLAYER", &comp_ip, &name, &passwd, 0, 0);
        Person::give_link(g, "PLAYER", &comp_ip);

        // Log the player in as this person
        g.interface_mut().ri.set_security(&accno.to_string(), 3);

        // Change to success screen
        RemoteInterface::run_screen(g, 5);
    }

    /// Create new stock market account
    pub fn script12(g: &mut Ctx) {
        // Get the inputted properties of the new account
        let name = caption_of(g, "nametext 0 0");
        let passwd = caption_of(g, "passwordtext 0 0");
        let passwd2 = caption_of(g, "passwordtext2 0 0");

        if passwd != passwd2 {
            create_msgbox(g, "Error", "The two passwords must be identical");
            return;
        }

        // Get the stock market
        let comp_name = remote_computer_name(g).expect("Script12: no remote computer");
        let comp_ip = g.world().get_computer(&comp_name).expect("Script12: market computer").ip.clone();

        // Create the account
        let mut record = Record::new();
        record.add_field(RECORDBANK_NAME, &name);
        record.add_field(RECORDBANK_PASSWORD, &passwd);
        record.add_field(RECORDBANK_SECURITY, "3");
        g.world_mut()
            .get_computer_mut(&comp_name)
            .expect("Script12: market computer")
            .recordbank
            .add_record(record);

        // Give the player his new access codes
        let code = Computer::generate_access_code_name(&name, &passwd);

        Person::give_link(g, "PLAYER", &comp_ip);
        g.player_mut().give_code(&comp_ip, &code);

        // Log the player in as this person
        g.interface_mut().ri.set_security(&name, 3);

        // Change to success screen
        RemoteInterface::run_screen(g, 3);
    }

    /// Close your bank account
    pub fn script13(g: &mut Ctx) {
        // Look up the BankAccount object and store it
        let (bank_name, accno, balance, loan) = {
            let world = g.world();
            let vl = g
                .player()
                .get_remote_host(world)
                .expect("Script13: no remote host");
            let comp = world.get_computer(&vl.computer).expect("Script13: remote host computer");
            let bank = comp.bank().expect("Script13: remote host is not a bank");

            // Look up the account number based on the log in name
            let security_name = &g.interface().ri.security_name;
            let rec = comp
                .recordbank
                .get_record_from_name(security_name)
                .expect("Script13: no record for login");
            let accno = rec
                .get_field(RECORDBANK_ACCNO)
                .expect("Script13: record without account number")
                .to_string();

            // Now look up the account based on that account number
            let account = bank.accounts.get(&accno).expect("Script13: account");

            // Only allow close if empty
            // TODO finish this in the C++ too
            (vl.computer.clone(), accno, account.balance, account.loan)
        };

        if balance != 0 {
            create_msgbox(
                g,
                "Error",
                concat!(
                    "Your account cannot be closed.\n",
                    "The balance must be zero."
                ),
            );
            RemoteInterface::run_screen(g, 3);
            return;
        } else if loan != 0 {
            create_msgbox(
                g,
                "Error",
                concat!("Your account cannot be closed.\n", "The loan must be zero."),
            );
            RemoteInterface::run_screen(g, 3);
            return;
        } else {
            let iaccno = scan_int(&accno);
            g.world_mut()
                .get_computer_mut(&bank_name)
                .expect("Script13: bank computer")
                .bank_mut()
                .expect("Script13: bank computer is not a bank")
                .close_account(iaccno);

            g.interface_mut().ri.set_security("Guest", 10);
            RemoteInterface::run_screen(g, 3);
        }
    }

    /// Search for Criminal Record
    pub fn script15(g: &mut Ctx) {
        // Get the search string
        let name = caption_of(g, "name 0 0");

        // Run the Criminal Record Screen
        RemoteInterface::run_screen(g, 8);

        // Start the search going
        let csi = g
            .interface_mut()
            .ri
            .screen
            .get_mut::<CriminalScreenInterface>()
            .expect("Script15: not a criminal screen");
        csi.set_search_name(&name);
    }

    /// Search for Academic Record
    pub fn script16(g: &mut Ctx) {
        // Get the search string
        let name = caption_of(g, "name 0 0");

        // Run the Academic Record Screen
        RemoteInterface::run_screen(g, 6);

        // Start the search going
        let asi = g
            .interface_mut()
            .ri
            .screen
            .get_mut::<AcademicScreenInterface>()
            .expect("Script16: not an academic screen");
        asi.set_search_name(&name);
    }

    /// Search for Social Security
    pub fn script17(g: &mut Ctx) {
        // Get the search string
        let name = caption_of(g, "name 0 0");

        // Run the Record Screen
        RemoteInterface::run_screen(g, 6);

        // Start the search going
        let sssi = g
            .interface_mut()
            .ri
            .screen
            .get_mut::<SocialSecurityScreenInterface>()
            .expect("Script17: not a social security screen");
        sssi.set_search_name(&name);
    }

    /// Opening caption
    pub fn script30(g: &mut Ctx) {
        // Bring up first caption
        g.ecl.register_caption_change(
            "firsttimeloading_text",
            "Establishing connection to UPLINK public access server...",
            1500,
            None,
        );
        timer_func(1500, Self::run_script, 31);
    }

    /// Connection anim
    pub fn script31(g: &mut Ctx) {
        if g.ecl.get_button("start_localhost").is_none() {
            // First time through
            g.ecl.register_button(230, 200, 64, 78, " ", " ", "start_localhost");
            button_assignbitmap(g, "start_localhost", localhost_bitmap());

            g.ecl.register_button(330, 220, 32, 32, " ", " ", "start_key");
            button_assignbitmap(g, "start_key", "analyser/nonsecure.tif");

            let (kx, ky, kw, kh) = button_rect(g, "start_key");
            g.ecl.register_button(kx - 43, ky + kh / 2 - 1, 42, 4, " ", " ", "start_keylink");
            g.ecl.register_button_callbacks(
                "start_keylink",
                Some(Self::draw_connection),
                None,
                None,
                None,
            );

            timer_func(800, Self::run_script, 31);
        } else if g.ecl.get_button("start_target").is_none() {
            // Second time through
            g.ecl.register_button(690, 320, 100, 100, " ", " ", "start_target");
            button_assignbitmap(g, "start_target", "start/publicaccessserver.tif");

            g.ecl.register_button(650, 340, 32, 32, " ", " ", "start_lock");
            button_assignbitmap(g, "start_lock", "analyser/secure.tif");

            let (_, _, lock_w, lock_h) = button_rect(g, "start_lock");
            let lock_x = 450 + lock_w;
            let lock_y = 340 + lock_h / 2 - 1;

            g.ecl.register_button(lock_x + 200, lock_y, 31, 4, " ", " ", "start_locklink");
            g.ecl.register_button_callbacks(
                "start_locklink",
                Some(Self::draw_connection),
                None,
                None,
                None,
            );

            g.ecl.register_movement("start_target", 490, 320, 500, MOVE_STRAIGHTLINE, None);
            g.ecl.register_movement("start_lock", 450, 340, 500, MOVE_STRAIGHTLINE, None);
            g.ecl.register_movement("start_locklink", lock_x, lock_y, 500, MOVE_STRAIGHTLINE, None);

            sound::play_sound("sounds/newmail.wav", Some("sounds/newmail.wav"), true);

            timer_func(800, Self::run_script, 31);
        } else {
            // Third time through
            let mut pd = PhoneDialler {};
            pd.create_at(g, 400, 170);
            pd.dial_number(g, IP_UPLINKPUBLICACCESSSERVER);
            pd.remove(g);

            let (kx, ky, kw, kh) = button_rect(g, "start_key");
            let (lx, _, _, _) = button_rect(g, "start_lock");

            g.ecl.register_button(
                kx + kw / 2 + 8,
                ky + kh / 2 + 8,
                lx - kx - 16,
                lx - kx - 16,
                " ",
                " ",
                "start_link",
            );
            g.ecl.register_button_callbacks(
                "start_link",
                Some(Self::draw_connection),
                None,
                None,
                None,
            );

            g.ecl.register_caption_change(
                "firsttimeloading_text",
                "Awaiting connection acknowledgment from UPLINK...",
                800,
                None,
            );

            sound::play_sound("sounds/ringout.wav", Some("sounds/ringout.wav"), false);

            timer_func(900, Self::run_script, 32);
        }
    }

    /// Begin registration
    pub fn script32(g: &mut Ctx) {
        // Load world
        // then start game running
        Game::new_game(g);

        sound::play_sound("sounds/phonepickup.wav", Some("sounds/phonepickup.wav"), true);

        // Remove the connection buttons
        g.ecl.remove_button("start_localhost");
        g.ecl.remove_button("start_key");
        g.ecl.remove_button("start_keylink");
        // g.ecl.remove_button("start_target"); (kept by the C++)
        g.ecl.remove_button("start_lock");
        g.ecl.remove_button("start_locklink");
        g.ecl.remove_button("start_link");

        MainMenu::remove(g);
    }

    /// Registers player as Agent
    pub fn script33(g: &mut Ctx) {
        // To register the player as an Uplink Agent after he has filled in all of his details.
        // OCCURS: When clicking on the OK button during Uplink Registration.
        let name = caption_of(g, "nametext 0 0");
        let password = caption_of(g, "passwordtext 0 0");
        let password2 = caption_of(g, "passwordtext2 0 0");

        let accesscode = Computer::generate_access_code_name(&name, &password);

        if player_handle(g) != "NEWAGENT" {
            create_msgbox(
                g,
                "Error",
                concat!(
                    "Our records show you are already\n",
                    "registered as an Uplink agent."
                ),
            );
            return;
        }

        if name == "Fill this in" {
            create_msgbox(g, "Error", "You must first enter a username");
            return;
        }

        if name.contains(|c| {
            matches!(c, ':' | '/' | '\\' | '?' | '.' | ',' | '"' | '<' | '>' | '|')
        }) {
            create_msgbox(
                g,
                "Error",
                concat!(
                    "Usernames cannot contain\n",
                    "any of these characters:\n",
                    " : / \\ ? . , \" < > | "
                ),
            );
            return;
        }

        if password.is_empty() {
            create_msgbox(g, "Error", "You must first enter a password");
            return;
        }

        if password != password2 {
            create_msgbox(g, "Error", "The two passwords must be identical");
            return;
        }

        g.player_mut().set_handle(&name);

        // Open a new account with Uplink International Bank
        let bank_name = NameGenerator::generate_international_bank_name("Uplink");
        let bank_ip = g.world().get_computer(&bank_name).expect("Script33: bank").ip.clone();
        let accno = Person::create_new_account(
            g,
            "PLAYER",
            &bank_ip,
            &name,
            &password,
            0,
            PLAYER_START_BALANCE,
        );
        Person::give_link(g, "PLAYER", &bank_ip);

        // Store the new player's account on Uplink's computer
        let mut record = Record::new();
        record.add_field(RECORDBANK_NAME, &name);
        record.add_field_int(RECORDBANK_ACCNO, accno);
        record.add_field(RECORDBANK_PASSWORD, &password);
        record.add_field(RECORDBANK_SECURITY, "5");
        record.add_field("Created", &g.world().date.get_long_string());
        g.world_mut()
            .get_computer_mut(NAME_UPLINKINTERNALSERVICES)
            .expect("Script33: internal services")
            .recordbank
            .add_record(record);

        // Create the rest of your records
        RecordGenerator::generate_records_player(g, &name);

        // Change his uplink rating to beginner
        g.player_mut().rating.set_uplink_rating(1);

        // Send notification to the player
        let mut msg = Message::new();
        msg.set_to("PLAYER");
        msg.set_from("Uplink public access system");
        msg.set_subject("Welcome to Uplink");
        msg.set_body(concat!(
            "Your details have been entered into our account.  Congratulations, you are now ",
            "an officially rated Uplink Agent.\n",
            "We recommend you complete the Uplink Test mission before attempting any real work.\n",
            "Your first stopping off point should be the Uplink Internal Services System.  Once there, ",
            "log in and click on 'help', followed by 'getting started'.\n",
            "Good luck."
        ));
        msg.give_link(IP_UPLINKINTERNALSERVICES);
        msg.give_code(IP_UPLINKINTERNALSERVICES, &accesscode);
        msg.send(g);

        // Send the player a test mission as a tutorial
        let mut mission = Mission::new();
        mission.set_type(MISSION_STEALFILE);
        mission.set_completion(
            Some(IP_UPLINKINTERNALSERVICES),
            Some("Uplink test data"),
            None,
            None,
            None,
        );
        mission.set_employer("Uplink");
        mission.set_contact("internal@Uplink.net");
        mission.set_payment(0, -1);
        mission.set_difficulty(1);
        mission.set_description("Uplink Test Mission -\nSteal data from a file server");
        mission.set_full_details(concat!(
            "Gain access to the Uplink Test Machine by breaking the outer security layer.\n",
            "Access the fileserver and download the target file 'Uplink Test Data'.\n",
            "Remove any traces of your hack.\n",
            "Return the data to us at address internal@Uplink.net"
        ));
        mission.give_link(IP_UPLINKTESTMACHINE);

        Person::give_mission(g, "PLAYER", mission);

        // Run the next screen
        if TESTGAME {
            RemoteInterface::run_screen(g, 5);
        } else {
            RemoteInterface::run_screen(g, 4);
        }
    }

    /// Connecting to Gateway anim (1)
    pub fn script34(g: &mut Ctx) {
        // Animation of opening connection to gateway
        if g.ecl.get_button("start_localhost").is_none() {
            // First time through - rebuild the connection graphics
            g.ecl.register_button(230, 200, 64, 78, " ", " ", "start_localhost");
            button_assignbitmap(g, "start_localhost", localhost_bitmap());

            g.ecl.register_button(330, 220, 32, 32, " ", " ", "start_key");
            button_assignbitmap(g, "start_key", "analyser/nonsecure.tif");

            let (kx, ky, kw, kh) = button_rect(g, "start_key");
            g.ecl.register_button(kx - 43, ky + kh / 2 - 1, 42, 4, " ", " ", "start_keylink");
            g.ecl.register_button_callbacks(
                "start_keylink",
                Some(Self::draw_connection),
                None,
                None,
                None,
            );

            g.ecl.register_button(450, 340, 32, 32, " ", " ", "start_lock");
            button_assignbitmap(g, "start_lock", "analyser/secure.tif");

            let (lx, ly, lw, lh) = button_rect(g, "start_lock");
            let lock_x = lx + lw;
            let lock_y = ly + lh / 2 - 1;

            g.ecl.register_button(lock_x, lock_y, 31, 4, " ", " ", "start_locklink");
            g.ecl.register_button_callbacks(
                "start_locklink",
                Some(Self::draw_connection),
                None,
                None,
                None,
            );

            g.ecl.register_button(
                kx + kw / 2 + 8,
                ky + kh / 2 + 8,
                lx - kx - 16,
                lx - kx - 16,
                " ",
                " ",
                "start_link",
            );
            g.ecl.register_button_callbacks(
                "start_link",
                Some(Self::draw_connection),
                None,
                None,
                None,
            );

            g.ecl.register_caption_change(
                "connecting 0 0",
                "Disconnecting from Uplink Public access server...",
                700,
                None,
            );

            timer_func(800, Self::run_script, 34);
        } else if g.ecl.get_button("start_link").is_some() {
            // Second time through - remove connection line
            g.ecl.remove_button("start_link");

            sound::play_sound("sounds/menuitemclick.wav", Some("sounds/menuitemclick.wav"), true);

            timer_func(600, Self::run_script, 35);
        }
    }

    /// Connecting to Gateway anim (2)
    pub fn script35(g: &mut Ctx) {
        let target_x = g.ecl.get_button("start_target").map(|b| b.x);

        match target_x {
            Some(x) if x < 640 => {
                // First time through - scroll target off screen
                let (lx, ly, lw, lh) = button_rect(g, "start_lock");
                let lock_x = lx + lw;
                let lock_y = ly + lh / 2 - 1;

                g.ecl.register_movement("start_target", 690, 320, 500, MOVE_STRAIGHTLINE, None);
                g.ecl.register_movement("start_lock", 650, 340, 500, MOVE_STRAIGHTLINE, None);
                g.ecl.register_movement(
                    "start_locklink",
                    lock_x + 200,
                    lock_y,
                    500,
                    MOVE_STRAIGHTLINE,
                    None,
                );

                sound::play_sound(
                    "sounds/newmailreverse.wav",
                    Some("sounds/newmailreverse.wav"),
                    true,
                );

                g.ecl.register_caption_change(
                    "connecting 0 0",
                    "Establishing connection to your GATEWAY computer...",
                    600,
                    None,
                );

                timer_func(1000, Self::run_script, 35);
            }
            Some(x) if x > 640 => {
                // Second time through - replace target with Gateway, scroll back
                g.ecl.remove_button("start_target");

                g.ecl.register_button(706, 320, 64, 80, " ", " ", "start_target2");
                button_assignbitmap(g, "start_target2", "start/gateway.tif");
                g.ecl.button_send_to_back("start_target2");

                let (_, _, lock_w, lock_h) = button_rect(g, "start_lock");
                let lock_x = 450 + lock_w;
                let lock_y = 340 + lock_h / 2 - 1;

                g.ecl.register_movement("start_target2", 506, 320, 500, MOVE_STRAIGHTLINE, None);
                g.ecl.register_movement("start_lock", 450, 340, 500, MOVE_STRAIGHTLINE, None);
                g.ecl.register_movement("start_locklink", lock_x, lock_y, 500, MOVE_STRAIGHTLINE, None);

                sound::play_sound("sounds/newmail.wav", Some("sounds/newmail.wav"), true);

                timer_func(1000, Self::run_script, 35);
            }
            None => {
                // Third time through - connect to gateway
                let mut pd = PhoneDialler {};
                pd.create_at(g, 400, 170);
                pd.dial_number(g, IP_UPLINKPUBLICACCESSSERVER);
                pd.remove(g);

                let (kx, ky, kw, kh) = button_rect(g, "start_key");
                let (lx, _, _, _) = button_rect(g, "start_lock");

                g.ecl.register_button(
                    kx + kw / 2 + 8,
                    ky + kh / 2 + 8,
                    lx - kx - 16,
                    lx - kx - 16,
                    " ",
                    " ",
                    "start_link",
                );
                g.ecl.register_button_callbacks(
                    "start_link",
                    Some(Self::draw_connection),
                    None,
                    None,
                    None,
                );

                g.ecl.register_caption_change(
                    "connecting 0 0",
                    "Awaiting connection acknowledgment from GATEWAY...",
                    700,
                    None,
                );
                timer_func(3500, Self::run_script, 36);
            }
            _ => {}
        }
    }

    /// Connects you to your gateway
    pub fn script36(g: &mut Ctx) {
        // Connects the user to his Gateway for the first time. OCCURS: Right after registration
        g.ecl.remove_button("start_localhost");
        g.ecl.remove_button("start_key");
        g.ecl.remove_button("start_keylink");
        g.ecl.remove_button("start_target2");
        g.ecl.remove_button("start_lock");
        g.ecl.remove_button("start_locklink");
        g.ecl.remove_button("start_link");

        Connection::disconnect(g, "PLAYER");
        Connection::reset(g, "PLAYER");

        RemoteInterface::run_new_location(g);
        RemoteInterface::run_screen(g, 5);

        if TESTGAME {
            timer_func(100, Self::run_script, 42);
        } else {
            timer_func(100, Self::run_script, 40);
        }
    }

    /// Download Uplink OS
    pub fn script40(g: &mut Ctx) {
        // Simulates the downloading of the Uplink Operating System. OCCURS: First-time start sequence
        if g.ecl.get_button("downloadingUOS").is_none() {
            // Button doesn't exist yet
            g.ecl.register_button(
                180,
                180,
                300,
                20,
                "Downloading UPLINK Operating System...",
                "",
                "downloadingUOS_title",
            );
            g.ecl.register_button_callbacks(
                "downloadingUOS_title",
                Some(textbutton_draw),
                None,
                None,
                None,
            );

            g.ecl.register_button(190, 210, 0, 15, "", "Shows the progress of the download", "downloadingUOS");
            g.ecl.register_resize("downloadingUOS", 200, 15, 5000, None);

            g.ecl.register_button(150, 210, 40, 15, "", "0 %", "downloadingUOS_prog");
            g.ecl.register_button_callbacks("downloadingUOS_prog", Some(textbutton_draw), None, None, None);

            g.ecl.register_button(190, 230, 300, 15, " ", " ", "downloadingUOS_current");
            g.ecl.register_button_callbacks("downloadingUOS_current", Some(textbutton_draw), None, None, None);

            timer_func(100, Self::run_script, 40);
        } else {
            // Download is occuring
            // Set the percentage done so far
            let width = button_rect(g, "downloadingUOS").2;
            let progress = (100.0 * width as f64 / 200.0) as i32;

            let caption = format!("{progress} %");
            g.ecl
                .get_button_mut("downloadingUOS_prog")
                .expect("Script40: progress button")
                .set_caption(&caption);

            // Put down some description of what we're downloading...
            let text = match progress {
                p if p < 5 => Some("Downloading Core services"),
                p if p < 10 => Some("Downloading Kernel"),
                p if p < 15 => Some("Downloading Loader package"),
                p if p < 20 => Some("Downloading Gateway package"),
                p if p < 25 => Some("Downloading Memory package"),
                p if p < 30 => Some("Downloading Status package"),
                p if p < 35 => Some("Downloading Finance package"),
                p if p < 40 => Some("Downloading Mail package"),
                p if p < 45 => Some("Downloading Mission package"),
                p if p < 50 => Some("Downloading Clock package"),
                p if p < 55 => Some("Downloading Multitasking package"),
                p if p < 60 => Some("Downloading Map package"),
                p if p < 75 => Some("Downloading Global communications package"),
                p if p < 80 => Some("Downloading Security package"),
                p if p < 85 => Some("Downloading File utilities"),
                p if p < 90 => Some("Downloading Tutorial daemon"),
                p if p < 95 => Some("Downloading Remote renderer package"),
                p if p < 100 => Some("Downloading Site connect package"),
                _ => None,
            };
            if let Some(text) = text {
                g.ecl
                    .get_button_mut("downloadingUOS_current")
                    .expect("Script40: current button")
                    .set_caption(text);
            }

            if progress < 100 {
                timer_func(100, Self::run_script, 40);
            } else {
                timer_func(500, Self::run_script, 41);
            }
        }
    }

    /// Runs new OS
    pub fn script41(g: &mut Ctx) {
        // Runs the newly downloaded Uplink OS (pretends to)
        if g.ecl.get_button("syscheckUOS_13").is_some() {
            // All finished - create the hud
            timer_func(250, Self::run_script, 42);
        } else {
            // Run each system check on the new Operating System
            for i in 0..14 {
                let buttonname = format!("syscheckUOS_{i}");

                if g.ecl.get_button(&buttonname).is_none() {
                    // We are running test #i
                    if i == 0 {
                        // First time round - remove previous screen
                        g.ecl.remove_button("downloadingUOS_title");
                        g.ecl.remove_button("downloadingUOS_prog");
                        g.ecl.remove_button("downloadingUOS_current");
                        g.ecl.remove_button("downloadingUOS");
                    } else {
                        // Not first time round - create 'OK' button
                        let okname = format!("syscheckUOS_OK_{}", i - 1);
                        g.ecl.register_button(450, 130 + 20 * (i - 1), 30, 15, "[OK]", "", &okname);
                        g.ecl.register_button_callbacks(&okname, Some(textbutton_draw), None, None, None);
                    }

                    // Now create current test button
                    let caption = match i {
                        0 => "Checking gateway HARDWARE : Trinity Ax086 2GQs cpu...",
                        1 => "Checking gateway HARDWARE : K256 512KQds modem...",
                        2 => "Checking gateway HARDWARE : 24 GQds DR-RAM memory...",
                        3 => "Verifying integrity of Uplink Operating System...",
                        4 => "Verifying security of Uplink Operating System...",
                        5 => "Installing UPLINK Operating system files...",
                        6 => "Initialising TSR programs in memory...",
                        7 => "Sending RUN signal to core Uplink Kernel...",
                        8 => "Checking Kernel Initilisation...",
                        9 => "Establishing Network communications protocols...",
                        10 => "Installing File-Copier...",
                        11 => "Installing File-Deleter...",
                        12 => "Installing Task-Manager you can read fast...",
                        13 => "Running Graphical User Interface...",
                        _ => "",
                    };

                    g.ecl.register_button(20, 130 + 20 * i, 400, 15, caption, "", &buttonname);
                    g.ecl.register_button_callbacks(&buttonname, Some(textbutton_draw), None, None, None);

                    sound::play_sound("sounds/done.wav", Some("sounds/done.wav"), true);

                    timer_func(250, Self::run_script, 41);

                    // End now (to avoid creating other buttons)
                    return;
                }
            }
        }
    }

    /// Creates LocalInterface HUD
    pub fn script42(g: &mut Ctx) {
        // Removes start-up sequence + Creates the HUD
        if g.ecl.get_button("syscheckUOS_0").is_some() {
            for i in 0..14 {
                // Remove existing start-up sequence
                g.ecl.remove_button(&format!("syscheckUOS_{i}"));
                g.ecl.remove_button(&format!("syscheckUOS_OK_{i}"));
            }
        }

        // Create the HUD
        if !LocalInterface::is_visible(g) {
            g.interface_mut().li.reset();
            LocalInterface::create(g);
        }

        // Put this line in to disable "first time" after one game
        g.app.options.set_option_value("game_firsttime", 0);

        RemoteInterface::run_screen(g, 6);
    }

    /// Starts the tutorial software
    pub fn script43(g: &mut Ctx) {
        // To start the tutorial program running
        TaskManager::run_software(g, "Tutorial", 1.0);
    }

    /// Installs a new patch
    pub fn script45(g: &mut Ctx) {
        // Installs a new patch. OCCURS: When a new version EXE is detected for the first time
        if g.ecl.get_button("newpatch_3").is_some() {
            // All finished - create the hud
            timer_func(250, Self::run_script, 46);
        } else {
            // Run a bunch of fake tests
            for i in 0..4 {
                let buttonname = format!("newpatch_{i}");

                if g.ecl.get_button(&buttonname).is_none() {
                    if i == 0 {
                        g.ecl.register_button(120, 40, 405, 52, " ", " ", "newpatch_title");
                        button_assignbitmap(g, "newpatch_title", "mainmenu/newpatch.tif");

                        g.ecl.register_button(150, 90, 330, 12, " ", " ", "newpatch_version");
                        button_assignbitmap(g, "newpatch_version", "mainmenu/versionnew.tif");
                    } else {
                        let okname = format!("newpatch_OK_{}", i - 1);
                        g.ecl.register_button(500, 140 + 20 * (i - 1), 30, 15, "[OK]", "", &okname);
                        g.ecl.register_button_callbacks(&okname, Some(textbutton_draw), None, None, None);
                    }

                    // Now create current test button
                    let caption = match i {
                        0 => "Reading new patch data...",
                        1 => "Modifying Uplink executable...",
                        2 => "Modifying Uplink runtime data...",
                        3 => "Validating patch authorisation with Uplink Corporation...",
                        _ => "",
                    };

                    g.ecl.register_button(100, 140 + 20 * i, 400, 15, caption, "", &buttonname);
                    g.ecl.register_button_callbacks(&buttonname, Some(textbutton_draw), None, None, None);

                    sound::play_sound("sounds/done.wav", Some("sounds/done.wav"), true);

                    timer_func(400, Self::run_script, 45);

                    // End now (to avoid creating other buttons)
                    return;
                }
            }
        }
    }

    /// Shows release notes
    pub fn script46(g: &mut Ctx) {
        let okname = "newpatch_OK_3";
        g.ecl.register_button(500, 140 + 20 * 3, 30, 15, "[OK]", "", okname);
        g.ecl.register_button_callbacks(okname, Some(textbutton_draw), None, None, None);

        g.ecl.register_button(50, 270, 550, 190, " ", " ", "newpatch_text");
        g.ecl.register_button_callbacks("newpatch_text", Some(textbutton_draw), None, None, None);
        g.ecl.register_caption_change(
            "newpatch_text",
            concat!(
                "UPLINK VERSION 1.3 PATCH\n",
                "Release notes\n\n",
                "Any problems with this patch should be reported to ",
                "Introversion Software at www.introversion.co.uk.\n\n",
                "This patch is backwards-compatible with your user files, however many ",
                "of the new features will not become active until you create a ",
                "new user from scratch.\n\n",
                "Uplink Corporation recommends you create a new user ",
                "as soon as possible."
            ),
            6000,
            None,
        );

        g.ecl.register_button(450, 440, 80, 15, "OK", "Click once you have read the information", "newpatch_ok");
        g.ecl.register_button_callback("newpatch_ok", show_recommendations_click);
    }

    /// Shows recommendations
    pub fn script47(g: &mut Ctx) {
        let width = g.app.options.get_option_value("graphics_screenwidth");
        let height = g.app.options.get_option_value("graphics_screenheight");
        g.ecl.reset(width, height);

        g.ecl.register_button(120, 40, 405, 52, " ", " ", "newpatch_title");
        button_assignbitmap(g, "newpatch_title", "mainmenu/newpatch.tif");

        g.ecl.register_button(150, 90, 330, 12, " ", " ", "newpatch_version");
        button_assignbitmap(g, "newpatch_version", "mainmenu/versionnew.tif");

        // Show recommendations to the player
        g.ecl.register_button(100, 200, 300, 20, "Recommendations", " ", "newpatch_recommendtitle");
        g.ecl.register_button_callbacks("newpatch_recommendtitle", Some(draw_main_title), None, None, None);

        // Screen resolution to 800x600
        g.ecl.register_button(
            150,
            250,
            300,
            15,
            "Set the in-game screen resolution to 800x600",
            "This is now the recommended in-game resolution.",
            "newpatch_screenres",
        );
        g.ecl.register_button_callbacks(
            "newpatch_screenres",
            Some(textbutton_draw),
            None,
            Some(button_click),
            Some(button_highlight),
        );

        g.ecl.register_button(450, 250, 15, 15, "1", "click to Enable or Disable this option", "newpatch_screenrestoggle");
        button_assignbitmaps(
            g,
            "newpatch_screenrestoggle",
            "mainmenu/optionno.tif",
            "mainmenu/optionyes.tif",
            "mainmenu/optionyes.tif",
        );
        g.ecl.register_button_callbacks(
            "newpatch_screenrestoggle",
            Some(toggle_box_draw),
            Some(toggle_box_click),
            Some(button_click),
            Some(button_highlight),
        );

        // New user profile
        g.ecl.register_button(
            150,
            280,
            300,
            15,
            "Start a new Agent profile",
            "Your old profiles will work but many new features won't be available",
            "newpatch_newuser",
        );
        g.ecl.register_button_callbacks(
            "newpatch_newuser",
            Some(textbutton_draw),
            None,
            Some(button_click),
            Some(button_highlight),
        );

        g.ecl.register_button(450, 280, 15, 15, "1", "click to Enable or Disable this option", "newpatch_newusertoggle");
        button_assignbitmaps(
            g,
            "newpatch_newusertoggle",
            "mainmenu/optionno.tif",
            "mainmenu/optionyes.tif",
            "mainmenu/optionyes.tif",
        );
        g.ecl.register_button_callbacks(
            "newpatch_newusertoggle",
            Some(toggle_box_draw),
            Some(toggle_box_click),
            Some(button_click),
            Some(button_highlight),
        );

        // OK button
        g.ecl.register_button(450, 440, 80, 15, "OK", "Click once you have chosen your options", "newpatch_ok");
        g.ecl.register_button_callback("newpatch_ok", new_patch_ok_click);
    }

    /// Bill the player for renting another gateway
    pub fn script50(g: &mut Ctx) {
        // To bill the player because he blew his last Gateway up, and now wants another.
        // OCCURS: After Gateway Nuke, log in again, select "Yes" to hire new gateway
        Person::change_balance(
            g,
            "PLAYER",
            COST_UPLINK_NEWGATEWAY * -1,
            "Uplink Corporation : New gateway",
        );

        // Recreate some of the missing interface
        g.ecl.register_button(490, 320, 100, 100, " ", " ", "start_target");
        button_assignbitmap(g, "start_target", "start/publicaccessserver.tif");
    }

    /// Player decides not to rent a new Gateway
    pub fn script51(g: &mut Ctx) {
        // To allow a player to say "NO" to hiring a new Gateway by logging him out, remembering
        // that his Gateway is still Nuked for next time.
        // OCCURS: After Gateway Nuke, log in again, select "NO" to hire a new gateway
        g.player_mut().gateway.nuked = true;

        let handle = player_handle(g);
        App::save_game(g, &handle);
        g.game.set_game_speed(GAMESPEED_PAUSED);

        let width = g.app.options.get_option_value("graphics_screenwidth");
        let height = g.app.options.get_option_value("graphics_screenheight");
        g.ecl.reset(width, height);

        MainMenu::run_screen(g, MAINMENU_LOGIN);
    }

    /// Stop Act1Scene4agents first email from being sent (subscription)
    pub fn script60(g: &mut Ctx) {
        // to remove the first email from Act1Scene4agents computer
        // OCCURS: When the player is on that agents computer and clicks to remove it
        let ds = remote_computer_screen_mut(g)
            .and_then(|cs| cs.as_dialog_screen_mut())
            .expect("Script60: not a dialog screen");

        ds.remove_widget("email1");
        ds.remove_widget("view1");
        ds.remove_widget("cancel1");

        g.ecl.remove_button("email1 0 0");
        g.ecl.remove_button("view1 62 -1");
        g.ecl.remove_button("cancel1 60 -1");
    }

    /// Stop Act1Scene4agents second email from being sent (ARC warning)
    pub fn script61(g: &mut Ctx) {
        // to remove the second email from Act1Scene4agents computer
        // OCCURS: When the player is on that agents computer and clicks to remove it
        let ds = remote_computer_screen_mut(g)
            .and_then(|cs| cs.as_dialog_screen_mut())
            .expect("Script61: not a dialog screen");

        ds.remove_widget("email2");
        ds.remove_widget("view2");
        ds.remove_widget("cancel2");

        g.ecl.remove_button("email2 0 0");
        g.ecl.remove_button("view2 63 -1");
        g.ecl.remove_button("cancel2 61 -1");

        g.world_mut().plotgenerator.player_cancels_mail();
    }

    /// View Act1Scene4agents first email (subscription)
    pub fn script62(g: &mut Ctx) {
        // To view an email on Act1Scene4agents computer
        // OCCURS: When he clicks on the view button for the first scheduled email (subscription)
        g.ecl.register_caption_change(
            "mailtext 0 0",
            concat!(
                "I would very much like to continue my subscription to ",
                "Hacker monthly.  Please continue to bill my account and ",
                "send me each issue for the next year."
            ),
            2000,
            None,
        );
    }

    /// View Act1Scene4agents second email (ARC warning)
    pub fn script63(g: &mut Ctx) {
        // To view an email on Act1Scene4agents computer
        // OCCURS: When he clicks on the view button for the second scheduled email (ARC warning)
        // We need to resize the button so the whole text will fit in as well
        let b = g.ecl.get_button_mut("mailtext 0 0").expect("Script63: mailtext button");
        b.y = 150;
        b.height = 220;

        g.ecl.register_caption_change(
            "mailtext 0 0",
            concat!(
                "Agents,\n",
                "If you are reading this email then it means I am dead.  I have been working for ",
                "Andromeda for the past month on a top secret project - and until yesterday I did not ",
                "even know what exactly I was coding.\n",
                "This story hit the news and I decided that I should try to find out what was happening ",
                "with project Revelation - and yesterday I managed to gain access to their stand-alone ",
                "server room and copy the files that we were working on.\n",
                "Unfortunately they saw what I was doing and I had to leave quick, and I haven't had time ",
                "to analyse the project files yet.  If you read this then they have already caught me.\n",
                "ARC cannot be trusted.  We must come together as a group and find out what they are up to ",
                "before something terrible happens.  I don't know if it's already too late."
            ),
            2000,
            None,
        );
    }

    /// The player just ran the Uplink_Agent_List program
    pub fn script70(g: &mut Ctx) {
        // To make a company offer the player money for his agent list.
        // OCCURS: After the player has downloaded the agent list from Uplink Internal Services

        // First - has the player already done this?
        if g.world().plotgenerator.player_completed_special_mission(SPECIALMISSION_MOLE) {
            return;
        }

        // No he hasn't.
        let mut rundate = Date::default();
        rundate.set_date_from(&g.world().date);
        rundate.advance_day(7);

        let mut ne = NotificationEvent::new();
        ne.set_type(NOTIFICATIONEVENT_TYPE_BUYAGENTLIST);
        ne.base.set_run_date(&rundate);
        g.world_mut()
            .scheduler
            .schedule_event(UplinkEvent::NotificationEvent(ne));
    }

    /// Somebody offers the player money for the Agent list
    pub fn script71(g: &mut Ctx) {
        // First - has the player already done this?
        if g.world().plotgenerator.player_completed_special_mission(SPECIALMISSION_MOLE) {
            return;
        }

        // Does the player already have this offer?
        let description = PlotGenerator::special_mission_description(SPECIALMISSION_MOLE);
        let has_offer = g
            .player()
            .agent
            .as_deref()
            .is_some_and(|a| a.missions.iter().any(|m| m.description == description));
        if has_offer {
            return;
        }

        // No he hasn't.
        let employer = WorldGenerator::get_random_company(g);
        if g.world().get_company(&employer).is_none() {
            return;
        }

        // Infer the internal services contact address
        let personname = format!("internal@{employer}.net");

        // Generate a new computer to dump the files to
        let ourcomp_name = WorldGenerator::generate_empty_file_server(g, &employer);
        let ourcomp_ip = g
            .world()
            .get_computer(&ourcomp_name)
            .expect("Script71: file server")
            .ip
            .clone();

        // Add in a new account for the player to use
        let password = NameGenerator::generate_password(g);
        let letters: String = (0..4)
            .map(|_| (b'a' + NumberGenerator::random_number(26) as u8) as char)
            .collect();
        let username = format!("temp{letters}");

        let mut record = Record::new();
        record.add_field(RECORDBANK_NAME, &username);
        record.add_field(RECORDBANK_PASSWORD, &password);
        record.add_field(RECORDBANK_SECURITY, "3");
        g.world_mut()
            .get_computer_mut(&ourcomp_name)
            .expect("Script71: file server")
            .recordbank
            .add_record(record);

        let code = Computer::generate_access_code_name(&username, &password);

        // Generate the text
        let description = PlotGenerator::special_mission_description(SPECIALMISSION_MOLE);
        let details = format!(
            concat!(
                "We have recently heard a rumour going around that says the complete Uplink Agent ",
                "List was stolen from their very own Internal Services System, and that this list includes ",
                "a mapping of agent Handles to real-world name.  If true, this data would be of extreme ",
                "interest to us and our operations.\n\n",
                "We have also heard a rumour, totally unsubstantiated of course, that you might be the Agent who stole ",
                "the data.  Certainly there are only a handful of agents capable, and you are indeed one.\n\n",
                "We have set up a file Server at the address below, and supplied you with a login.  If the rumour is ",
                "true, we will be willing to pay you {} for all of the data files and the Agent List program.  ",
                "Payment will be up to the amount specified - if we ",
                "dont receive all of the data we will reduce payment accordingly.\n\n",
                "If we have made a mistake and you are not the person with whom we should be talking, then we apologise.  ",
                "Your ratings will not change if you choose to abandon this mission - it is totally optional.\n",
            ),
            PAYMENT_SPECIALMISSION_MOLE
        ) + &format!("Our fileserver: \n{ourcomp_ip}\nLogin:\n{code}\n\nEND");

        let payment = 10;

        // Generate the mission
        let mut mission = Mission::new();
        mission.set_type(MISSION_SPECIAL);
        mission.set_completion(Some(&ourcomp_ip), None, None, None, None);
        mission.set_employer(&employer);
        mission.set_contact(&personname);
        mission.set_payment(payment, payment);
        mission.set_description(description);
        mission.set_full_details(&details);
        mission.give_link(&ourcomp_ip);
        mission.give_code(&ourcomp_ip, &code);

        Person::give_mission(g, "PLAYER", mission);
    }

    /// Some agents are killed due to your selling of the agent list
    pub fn script72(g: &mut Ctx) {
        // Kill a few agents
        for _ in 0..5 {
            let agent = WorldGenerator::get_random_agent(g);
            g.world_mut()
                .get_person_mut(&agent)
                .expect("Script72: agent")
                .set_status(PERSON_STATUS_DEAD);
        }

        // Write a news story
        let mut news = News::new();
        news.set_headline("Multiple Uplink Agent murders reported");
        news.set_details(concat!(
            "Federal Agents today announced they were investigating a series of murders and ",
            "suicides, all involving active or retired Uplink Agents.  Within the last two days, ",
            "eight Uplink Agents of various skill levels have been executed or have committed suicide.\n\n",
            "Federal agents are keeping quiet about the identities of those agents killed, and have not ",
            "suggested any connecting factor other than the fact that they all worked freelance for Uplink Corporation.\n\n",
            "Murders of Uplink Agents are rare - Uplink Corporation is very careful at making sure the agent handles ",
            "are never converted into real-world people, effectively providing anonymity for their agents."
        ));

        CompanyUplink::create_news(g, news);
    }

    /// Enable LAN lock systems
    pub fn script80(g: &mut Ctx) {
        // Are we on a LAN?
        let Some(lan_name) = player_lan_name(g) else {
            return;
        };

        // Are we connected to an Authentication Server?
        // If so, do the locking
        let current = lan_current_system(g);
        let Some((sys_type, unlock1, unlock2, unlock3)) = lan_system_fields(g, &lan_name, current)
        else {
            return;
        };
        if sys_type != LANSYSTEM_AUTHENTICATION {
            return;
        }

        lan_edit_system(g, &lan_name, unlock1, |s| s.data1 = 1);
        lan_edit_system(g, &lan_name, unlock2, |s| s.data1 = 1);
        lan_edit_system(g, &lan_name, unlock3, |s| s.data3 = 1); // sic: C++ sets data3 here

        if lan_is_in_connection(g, unlock1)
            || lan_is_in_connection(g, unlock2)
            || lan_is_in_connection(g, unlock3)
        {
            show_current_lan_system(g);
        }

        create_msgbox(g, "Success", "Lock System ENABLED");

        // Awaken the sys admin
        // LanMonitor::SysAdminAwaken(); (commented out in the C++ too)
    }

    /// Disable LAN lock systems
    pub fn script81(g: &mut Ctx) {
        // Are we on a LAN?
        let Some(lan_name) = player_lan_name(g) else {
            return;
        };

        // Are we connected to an Authentication Server?
        // If so, do the unlocking
        let current = lan_current_system(g);
        let Some((sys_type, unlock1, unlock2, unlock3)) = lan_system_fields(g, &lan_name, current)
        else {
            return;
        };
        if sys_type != LANSYSTEM_AUTHENTICATION {
            return;
        }

        lan_edit_system(g, &lan_name, unlock1, |s| s.data1 = 0);
        lan_edit_system(g, &lan_name, unlock2, |s| s.data1 = 0);
        lan_edit_system(g, &lan_name, unlock3, |s| s.data1 = 0);

        create_msgbox(g, "Success", "Lock System DISABLED");

        // Awaken the sys admin
        // LanMonitor::SysAdminAwaken(); (commented out in the C++ too)
    }

    /// Enable LAN Isolation bridge
    pub fn script82(g: &mut Ctx) {
        // Are we on a LAN?
        let Some(lan_name) = player_lan_name(g) else {
            return;
        };

        // Are we connected to an Isolation bridge
        // If so, do the locking
        let current = lan_current_system(g);
        let Some((sys_type, unlock1, unlock2, _)) = lan_system_fields(g, &lan_name, current)
        else {
            return;
        };
        if sys_type != LANSYSTEM_ISOLATIONBRIDGE {
            return;
        }

        lan_edit_system(g, &lan_name, unlock1, |s| s.data1 = 1);
        lan_edit_system(g, &lan_name, unlock2, |s| s.data1 = 0);

        // If our connection ran through the newly locked system
        // Then reset the bastard
        if lan_is_in_connection(g, unlock1) {
            LanMonitor::begin_attack(g);
            show_current_lan_system(g);
        }

        create_msgbox(g, "Success", "Isolation Bridge ENABLED");

        // Awaken the sys admin
        // LanMonitor::SysAdminAwaken(); (commented out in the C++ too)
    }

    /// Disable LAN Isolation bridge
    pub fn script83(g: &mut Ctx) {
        // Are we on a LAN?
        let Some(lan_name) = player_lan_name(g) else {
            return;
        };

        // Are we connected to an Isolation bridge
        // If so, do the unlocking
        let current = lan_current_system(g);
        let Some((sys_type, unlock1, unlock2, _)) = lan_system_fields(g, &lan_name, current)
        else {
            return;
        };
        if sys_type != LANSYSTEM_ISOLATIONBRIDGE {
            return;
        }

        lan_edit_system(g, &lan_name, unlock1, |s| s.data1 = 0);
        lan_edit_system(g, &lan_name, unlock2, |s| s.data1 = 1);

        // If our connection ran through the newly locked system
        // Then reset the bastard
        if lan_is_in_connection(g, unlock2) {
            LanMonitor::begin_attack(g);
            show_current_lan_system(g);
        }

        create_msgbox(g, "Success", "Isolation Bridge DISABLED");

        // Awaken the sys admin
        // LanMonitor::SysAdminAwaken(); (commented out in the C++ too)
    }

    /// Do a RadioScreen connection
    pub fn script85(g: &mut Ctx) {
        const NUMCAPTIONS: usize = 14;

        const TEXT: [&str; NUMCAPTIONS] = [
            "Not Connected",
            "Aligning radio transmitter",
            "Complete",
            "Starting remote connect services",
            "Powering up transmitter",
            "Verifying transmitter power output",
            "Verified",
            "Preparing ping message",
            "Transmitting ping signal",
            "Sent",
            "Listening for echo reply",
            "Listening for echo reply .",
            "Listening for echo reply ..",
            "Listening for echo reply ...",
        ];

        const TIME: [u32; NUMCAPTIONS] = [0, 700, 200, 100, 100, 100, 100, 100, 500, 200, 250, 250, 250, 250];

        let Some(status) = g
            .ecl
            .get_button("radioscreen_statustext")
            .map(|b| b.caption.clone())
        else {
            return;
        };

        // Which caption is currently up?
        let Some(caption_index) = TEXT.iter().position(|t| *t == status) else {
            return; // Something went wrong
        };

        // Show the next caption
        if caption_index == NUMCAPTIONS - 1 {
            ScreenSlot::call(g, remote_screen_slot, |s, g| {
                if let Some(rsi) = s
                    .as_any_mut()
                    .downcast_mut::<RadioTransmitterScreenInterface>()
                {
                    rsi.connect(g);
                }
            });
        } else {
            g.ecl
                .get_button_mut("radioscreen_statustext")
                .expect("Script85: status button")
                .set_caption(TEXT[caption_index + 1]);
            timer_func(TIME[caption_index + 1], Self::run_script, 85);
        }
    }
}

// Free functions of scriptlibrary.cpp (button callbacks of the patch screens).

/// Mouse-up of "newpatch_ok" on the release notes: runs script 47.
pub fn show_recommendations_click(g: &mut Ctx, button: &Button) {
    ScriptLibrary::run_script(g, 47);
}

/// Mouse-up that closes the app.
pub fn exit_game_click(g: &mut Ctx, button: &Button) {
    App::close(g);
}

/// Applies the patch screen's toggles (screen resolution, new user) and shows the restart message box.
pub fn new_patch_ok_click(g: &mut Ctx, button: &Button) {
    // The toggles' captions say whether each option is on ('1') or off
    let set_screen_res = caption_of(g, "newpatch_screenrestoggle").starts_with('1');
    let new_user = caption_of(g, "newpatch_newusertoggle").starts_with('1');

    let width = g.app.options.get_option_value("graphics_screenwidth");
    let height = g.app.options.get_option_value("graphics_screenheight");
    g.ecl.reset(width, height);

    create_msgbox(g, "Shutdown", "Patch changes complete.\nUplink must now be restarted.");
    g.ecl.register_button_callback("msgbox_close", exit_game_click);
    g.ecl.register_button_callback("msgbox_title", exit_game_click);

    if set_screen_res {
        g.app.options.request_shutdown_change("graphics_screenwidth", 800);
        g.app.options.request_shutdown_change("graphics_screenheight", 600);
    }

    if new_user {
        g.app.options.set_option_value("game_firsttime", 1);
    }
}

/// DrawFn: the caption in Helvetica 18.
pub fn draw_main_title(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
    text::draw_text_styled(button.x, button.y, &button.caption, text::HELVETICA_18);
}

/// DrawFn of a toggle box: caption "0" draws the normal image, anything else the clicked image.
pub fn toggle_box_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
    if button.caption.starts_with('0') {
        imagebutton_draw(g, button, false, false);
    } else {
        imagebutton_draw(g, button, true, true);
    }
}

/// Mouse-up of a toggle box: flips the caption between "0" and "1".
pub fn toggle_box_click(g: &mut Ctx, button: &Button) {
    let new_caption = if button.caption.starts_with('0') { "1" } else { "0" };
    let name = button.name.clone();
    if let Some(b) = g.ecl.get_button_mut(&name) {
        b.set_caption(new_caption);
    }
}

// Private helpers shared by the scripts above.

/// The start-sequence localhost bitmap for this platform (the C++ picked it with `#ifdef WIN32`).
fn localhost_bitmap() -> &'static str {
    if cfg!(windows) {
        "start/localhost_win32.tif"
    } else {
        "start/localhost_linux.tif"
    }
}

/// `EclGetButton(name)->caption`; the C++ asserted that the button exists.
fn caption_of(g: &Ctx, name: &str) -> String {
    g.ecl
        .get_button(name)
        .unwrap_or_else(|| panic!("ScriptLibrary: missing button {name}"))
        .caption
        .clone()
}

/// `(x, y, width, height)` of a registered button; the C++ asserted that the button exists.
fn button_rect(g: &Ctx, name: &str) -> (i32, i32, i32, i32) {
    let b = g
        .ecl
        .get_button(name)
        .unwrap_or_else(|| panic!("ScriptLibrary: missing button {name}"));
    (b.x, b.y, b.width, b.height)
}

/// `sscanf(s, "%d", &i)`: the leading integer of `s`, or 0 when there is none.
fn scan_int(s: &str) -> i32 {
    let t = s.trim_start();
    let bytes = t.as_bytes();
    let mut end = 0;
    if end < bytes.len() && (bytes[end] == b'-' || bytes[end] == b'+') {
        end += 1;
    }
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    t[..end].parse().unwrap_or(0)
}

/// `Player::handle`: the agent handle of the player ("NEWAGENT" before registration).
fn player_handle(g: &Ctx) -> String {
    g.player()
        .agent
        .as_deref()
        .map(|a| a.handle.clone())
        .unwrap_or_default()
}

/// `game->GetInterface()->GetRemoteInterface()->GetComputerScreen()->GetComputer()`: the name of
/// the computer the remote interface is showing.
fn remote_computer_name(g: &Ctx) -> Option<String> {
    g.interface()
        .ri
        .screen
        .get_dyn()?
        .computer_screen_ref()
        .map(|cs| cs.computer.clone())
}

/// `GetRemoteInterface()->GetComputerScreen()`, mutable: resolved from the remote screen's `ScreenRef`.
fn remote_computer_screen_mut(g: &mut Ctx) -> Option<&mut ComputerScreen> {
    let cs = g.interface().ri.screen.get_dyn()?.computer_screen_ref()?.clone();
    g.world_mut()
        .get_computer_mut(&cs.computer)?
        .get_computer_screen_mut(cs.index)
}

/// The remote interface's screen slot (for `ScreenSlot::call`).
fn remote_screen_slot(g: &mut Ctx) -> &mut ScreenSlot {
    &mut g.interface_mut().ri.screen
}

/// `LanMonitor::currentSystem`
fn lan_current_system(g: &Ctx) -> i32 {
    g.lan_monitor.current_system
}

/// `LanMonitor::IsInConnection(node)`
fn lan_is_in_connection(g: &Ctx, node: i32) -> bool {
    g.lan_monitor.is_in_connection(node)
}

/// `GetRemoteHost()` as a LAN computer: the name of the LAN computer the player is attacking, or
/// None when the remote host is not a LAN (the C++ returned early in that case).
fn player_lan_name(g: &Ctx) -> Option<String> {
    let vl = g.player().get_remote_host(g.world())?;
    let comp = g.world().get_computer(&vl.computer)?;
    comp.lan().map(|_| vl.computer.clone())
}

/// `lan->systems.GetData(index)`: (TYPE, data1, data2, data3) of that LAN system, or None when
/// the slot is empty.
fn lan_system_fields(g: &Ctx, lan_name: &str, index: i32) -> Option<(i32, i32, i32, i32)> {
    let lan = g.world().get_computer(lan_name)?.lan()?;
    let s = lan.systems.get_data(index)?;
    Some((s.type_, s.data1, s.data2, s.data3))
}

/// Edits LAN system `index` when that slot is occupied (the C++ `ValidIndex` + `GetData` pair).
fn lan_edit_system(
    g: &mut Ctx,
    lan_name: &str,
    index: i32,
    edit: impl FnOnce(&mut LanComputerSystem),
) {
    let Some(lan) = g
        .world_mut()
        .get_computer_mut(lan_name)
        .and_then(|c| c.lan_mut())
    else {
        return;
    };
    if let Some(s) = lan.systems.get_data_mut(index) {
        edit(s);
    }
}

/// The C++ block of scripts 80-83 that reopens the screen of the system the player is attacking.
fn show_current_lan_system(g: &mut Ctx) {
    let current_system = lan_current_system(g);

    let remotehost = g.player().remotehost.clone();
    let vl_computer = g
        .world()
        .get_vlocation(&remotehost)
        .expect("ScriptLibrary: remote host")
        .computer
        .clone();
    let comp = g
        .world()
        .get_computer(&vl_computer)
        .expect("ScriptLibrary: remote host computer");
    if comp.type_ != COMPUTER_TYPE_LAN {
        return;
    }

    let screen_index = comp
        .lan()
        .and_then(|lc| lc.systems.get_data(current_system))
        .expect("ScriptLibrary: current LAN system")
        .screen_index;

    RemoteInterface::run_screen(g, screen_index);
    LocalInterface::run_screen(g, SCREEN_NONE, -1);
    LocalInterface::run_screen(g, SCREEN_LAN, -1);
}
