//! Port of source/world/generator/worldgenerator.h + source/world/generator/worldgenerator.cpp

use crate::app::globals;
use crate::app::globals::{CODECARD_ENABLED, DEMOGAME, FULLGAME, TESTGAME};
use crate::app::miscutils::list_directory;
use crate::ctx::Ctx;
use crate::game::data::data::{
    ComputerUpgrade, AGENT_UPLINKRATINGAVERAGE, AGENT_UPLINKRATINGVARIANCE,
    COMPANYALIGNMENT_AVERAGE, COMPANYALIGNMENT_RANGE, COMPANYGROWTH_AVERAGE, COMPANYGROWTH_RANGE,
    COMPANYSIZE_AVERAGE, COMPANYSIZE_RANGE, COST_UPLINK_PERMONTH, HACKDIFFICULTY_CENTRALMAINFRAME,
    HACKDIFFICULTY_CENTRALMEDICALDATABASE, HACKDIFFICULTY_GLOBALCRIMINALDATABASE,
    HACKDIFFICULTY_INTERNALSERVICESMACHINE, HACKDIFFICULTY_INTERNATIONALACADEMICDATABASE,
    HACKDIFFICULTY_INTERNATIONALSOCIALSECURITYDATABASE, HACKDIFFICULTY_INTERNIC,
    HACKDIFFICULTY_PROTOVISION, HACKDIFFICULTY_PUBLICBANKSERVER,
    HACKDIFFICULTY_PUBLICBANKSERVER_ADMIN, HACKDIFFICULTY_STOCKMARKET,
    HACKDIFFICULTY_UPLINK_INTERNALSERVICESMACHINE, HACKDIFFICULTY_UPLINK_TESTMACHINE,
    HACKDIFFICULTY_VARIANCE, HARDWARE_UPGRADES, IP_ACADEMICDATABASE, IP_CENTRALMEDICALDATABASE,
    IP_GLOBALCRIMINALDATABASE, IP_INTERNIC, IP_INTROVERSION, IP_LOCALHOST, IP_OCP, IP_PROTOVISION,
    IP_SJGAMES, IP_SOCIALSECURITYDATABASE, IP_STOCKMARKETSYSTEM, IP_UPLINKCREDITSMACHINE,
    IP_UPLINKINTERNALSERVICES, IP_UPLINKPUBLICACCESSSERVER, IP_UPLINKTESTMACHINE,
    MAXCOMPANYSIZE_FINE, MAXCOMPANYSIZE_WARNINGMAIL, MINCOMPANYSIZE_FIREWALL,
    MINCOMPANYSIZE_MONITOR, MINCOMPANYSIZE_PROXY, NAME_UPLINKINTERNALSERVICES,
    NUM_STARTINGHARDWAREUPGRADES, NUM_STARTINGSOFTWAREUPGRADES, NUM_STARTING_AGENTS,
    NUM_STARTING_BANKS, NUM_STARTING_COMPANIES, NUM_STARTING_MISSIONS, NUM_STARTING_PEOPLE,
    NUM_STARTING_PHOTOS, NUM_STARTING_VOICES, PLAYER_START_BALANCE, PLAYER_START_CREDITRATING,
    PLAYER_START_NEUROMANCERRATING, PLAYER_START_UPLINKRATING, SOFTWARE_UPGRADES,
    TRACESPEED_CENTRALMAINFRAME, TRACESPEED_CENTRALMEDICALDATABASE,
    TRACESPEED_GLOBALCRIMINALDATABASE, TRACESPEED_INTERNALSERVICESMACHINE,
    TRACESPEED_INTERNATIONALACADEMICDATABASE, TRACESPEED_INTERNATIONALSOCIALSECURITYDATABASE,
    TRACESPEED_INTERNIC, TRACESPEED_PROTOVISION, TRACESPEED_PUBLICACCESSSERVER,
    TRACESPEED_PUBLICBANKSERVER, TRACESPEED_STOCKMARKET, TRACESPEED_UPLINK_INTERNALSERVICESMACHINE,
    TRACESPEED_UPLINK_PUBLICACCESSSERVER, TRACESPEED_UPLINK_TESTMACHINE, TRACESPEED_VARIANCE,
};
use crate::interface::remoteinterface::remoteinterface::{
    SCREEN_ACADEMICSCREEN, SCREEN_ACCOUNTSCREEN, SCREEN_CHANGEGATEWAY, SCREEN_CODECARD,
    SCREEN_COMPANYINFO, SCREEN_CONSOLESCREEN, SCREEN_CONTACTSCREEN, SCREEN_CRIMINALSCREEN,
    SCREEN_FILESERVERSCREEN, SCREEN_HWSALESSCREEN, SCREEN_LOANSSCREEN, SCREEN_NEARESTGATEWAY,
    SCREEN_NEWSSCREEN, SCREEN_NUCLEARWAR, SCREEN_PROTOVISION, SCREEN_RANKINGSCREEN,
    SCREEN_RECORDSCREEN, SCREEN_SECURITYSCREEN, SCREEN_SHARESVIEWSCREEN, SCREEN_SOCSECSCREEN,
    SCREEN_SWSALESSCREEN, SCREEN_VOICEANALYSIS, SCREEN_VOICEPHONE,
};
use crate::world::company::company::{
    Company, COMPANYTYPE_COMMERCIAL, COMPANYTYPE_FINANCIAL, COMPANYTYPE_UNKNOWN,
};
use crate::world::company::companyuplink::CompanyUplink;
use crate::world::company::mission::Mission;
use crate::world::company::sale::{Sale, SALETYPE_HARDWARE, SALETYPE_SOFTWARE};
use crate::world::computer::computer::{
    Computer, COMPUTER_TRACEACTION_DISCONNECT, COMPUTER_TRACEACTION_FINE,
    COMPUTER_TRACEACTION_LEGAL, COMPUTER_TRACEACTION_WARNINGMAIL, COMPUTER_TYPE_ANY,
    COMPUTER_TYPE_CENTRALMAINFRAME, COMPUTER_TYPE_INTERNALSERVICESMACHINE, COMPUTER_TYPE_LAN,
    COMPUTER_TYPE_PERSONALCOMPUTER, COMPUTER_TYPE_PUBLICACCESSSERVER,
    COMPUTER_TYPE_PUBLICBANKSERVER, COMPUTER_TYPE_UNKNOWN, COMPUTER_TYPE_VOICEPHONESYSTEM,
};
use crate::world::computer::computerscreen::bbsscreen::BBSScreen;
use crate::world::computer::computerscreen::computerscreen::ComputerScreen;
use crate::world::computer::computerscreen::cypherscreen::CypherScreen;
use crate::world::computer::computerscreen::dialogscreen::{
    DialogScreen, WIDGET_BASIC, WIDGET_CAPTION, WIDGET_FIELDVALUE, WIDGET_NEXTPAGE,
    WIDGET_PASSWORDBOX, WIDGET_SCRIPTBUTTON, WIDGET_TEXTBOX,
};
use crate::world::computer::computerscreen::disconnectedscreen::DisconnectedScreen;
use crate::world::computer::computerscreen::genericscreen::GenericScreen;
use crate::world::computer::computerscreen::highsecurityscreen::HighSecurityScreen;
use crate::world::computer::computerscreen::linksscreen::{
    LinksScreen, LINKSSCREENTYPE_ALLLINKS, LINKSSCREENTYPE_LOCALLINKS, LINKSSCREENTYPE_PLAYERLINKS,
};
use crate::world::computer::computerscreen::logscreen::{
    LogScreen, LOGSCREEN_TARGET_ACCESSLOGS, LOGSCREEN_TARGET_ACCOUNTLOGS,
};
use crate::world::computer::computerscreen::menuscreen::MenuScreen;
use crate::world::computer::computerscreen::messagescreen::MessageScreen;
use crate::world::computer::computerscreen::passwordscreen::PasswordScreen;
use crate::world::computer::computerscreen::shareslistscreen::{
    SharesListScreen, SHARESLISTSCREENTYPE_ALLSHARES,
};
use crate::world::computer::computerscreen::useridscreen::UserIDScreen;
use crate::world::computer::databank::{
    Data, DATATYPE_DATA, DATATYPE_PROGRAM, SOFTWARETYPE_NONE, SOFTWARETYPE_OTHER,
};
use crate::world::computer::gateway::Gateway;
use crate::world::computer::gatewaydef::GatewayDef;
use crate::world::computer::logbank::{AccessLog, LOG_NOTSUSPICIOUS, LOG_TYPE_TEXT};
use crate::world::computer::recordbank::{
    Record, RECORDBANK_ADMIN, RECORDBANK_NAME, RECORDBANK_PASSWORD, RECORDBANK_READONLY,
    RECORDBANK_READWRITE, RECORDBANK_SECURITY,
};
use crate::world::computer::securitysystem::{
    SECURITY_TYPE_FIREWALL, SECURITY_TYPE_MONITOR, SECURITY_TYPE_PROXY,
};
use crate::world::connection::Connection;
use crate::world::generator::langenerator::LanGenerator;
use crate::world::generator::missiongenerator::MissionGenerator;
use crate::world::generator::namegenerator::NameGenerator;
use crate::world::generator::numbergenerator::NumberGenerator;
use crate::world::generator::recordgenerator::RecordGenerator;
use crate::world::person::{Person, PERSON_STATUS_NONE};
use crate::world::player::Player;
use crate::world::vlocation::{VIRTUAL_HEIGHT, VIRTUAL_WIDTH};
use std::cell::RefCell;
use uplink_engine::archive;
use uplink_engine::image::Image;

// PORT: conventions used by every function here (the C++ added everything to `game->GetWorld()`
// and returned raw pointers into it):
//  - everything touches the world, so it takes `g: &mut Ctx` (`&Ctx` when it only reads);
//  - "returns the object" becomes "returns the object's key": VLocation -> ip, Company /
//    Computer / Person / Agent -> name (Agent = a `Person` with `agent: Some(..)`),
//    Mission -> index in the Uplink company's `missions`; look the object up again with the key;
//  - C++ overloads get distinct names (`generate_company` / `generate_company_named`,
//    `generate_computer` / `generate_computer_of_type`, `get_random_computer` /
//    `get_random_computer_of_type`);
//  - functions that can return NULL return `Option`.
//  - a computer is built completely as a local `Computer` and stored with
//    `create_computer_from` at the end (the C++ stored the pointer first and filled it in
//    afterwards; the stored result is the same).

thread_local! {
    /// `worldmapmask`: used to determine legal computer positions.
    static WORLDMAPMASK: RefCell<Option<Image>> = const { RefCell::new(None) };
}

/// C++ `int RandomNormalNumber ( mean, range )`: the C++ returned an int, so the f32 result is
/// truncated the same way.
fn random_normal_int(mean: f32, range: f32) -> i32 {
    NumberGenerator::random_normal_number(mean, range) as i32
}

pub struct WorldGenerator;

impl WorldGenerator {
    /// Sets up the worldmapmask (theme file "worldmaplarge_mask.tif").
    pub fn initialise(g: &Ctx) {
        let mut mask = Image::new();
        let filename = g.app.options.theme_filename("worldmaplarge_mask.tif");
        mask.load_tif(&filename);
        mask.scale(VIRTUAL_WIDTH, VIRTUAL_HEIGHT);
        mask.flip_around_h();
        WORLDMAPMASK.with(|m| *m.borrow_mut() = Some(mask));
    }

    /// Clears up memory used
    pub fn shutdown() {
        WORLDMAPMASK.with(|m| *m.borrow_mut() = None);
    }

    // Top level functions for generating groups of data

    /// This is the main entry function, called in a NewGame
    pub fn generate_all(g: &mut Ctx) {
        Self::generate_specifics(g);
        Self::generate_player(g, "NEWAGENT");
        Self::generate_random_world(g); // Must come after GeneratePlayer
        Self::load_dynamics(g);
    }

    pub fn generate_random_world(g: &mut Ctx) {
        // Generate some companies
        for _ in 0..NUM_STARTING_COMPANIES {
            Self::generate_company(g);
        }

        // Generate some banks
        for _ in 0..NUM_STARTING_BANKS {
            Self::generate_company_bank(g);
        }

        // Generate some people
        for _ in 0..NUM_STARTING_PEOPLE {
            Self::generate_person(g);
        }

        // Generate some agents
        for _ in 0..NUM_STARTING_AGENTS {
            Self::generate_agent(g);
        }

        // Generate some missions
        for _ in 0..NUM_STARTING_MISSIONS {
            MissionGenerator::generate_mission(g);
        }

        // Generate employment data
        // C++ walks companies.ConvertToDArray (BTree pre-order). The snapshot of names matches the
        // C++ snapshot (PORT: a duplicate name finds its first entry again).
        let names: Vec<String> = g.world().companies.keys_all();
        for name in names {
            if name != "Government" && name != "Uplink" && name != "Player" {
                let ceo = Self::get_random_person(g);
                let admin = Self::get_random_person(g);

                let company = g
                    .world_mut()
                    .get_company_mut(&name)
                    .expect("company listed a moment ago");
                company.set_boss(&ceo);
                company.set_admin(&admin);
            }
        }
    }

    pub fn generate_specifics(g: &mut Ctx) {
        // Create yourself as a company
        g.world_mut().create_company("Player");

        // Generate some specific companies and computers
        Self::generate_local_machine(g);

        Self::generate_company_government(g);
        Self::generate_global_criminal_database(g);
        Self::generate_international_social_security_database(g);
        // Self::generate_central_medical_database(g);
        Self::generate_global_intelligence_agency(g);
        Self::generate_international_academic_database(g);
        Self::generate_inter_nic(g);
        Self::generate_stock_market(g);
        Self::generate_proto_vision(g);

        if !DEMOGAME {
            Self::generate_ocp(g);
            Self::generate_sj_games(g);
            Self::generate_introversion(g);
        }

        Self::generate_company_uplink(g);
    }

    /// Loads things that change every time - LANs etc
    pub fn load_dynamics(g: &mut Ctx) {
        //
        // Load additional gateways from the directory
        //

        let numgateways = g.world().gatewaydefs.size();
        let extra_gateways = list_directory("data/gateways/", ".txt");
        for (j, gateway_filename) in extra_gateways.iter().enumerate() {
            let bytes = archive::read(gateway_filename)
                .unwrap_or_else(|| panic!("UplinkAssert(rsFilename): {gateway_filename}"));
            // idos2unixstream: the file is read with every '\r' removed
            let text: Vec<u8> = bytes.into_iter().filter(|&b| b != b'\r').collect();
            let mut this_file = std::io::Cursor::new(text);

            let mut def = GatewayDef::new();
            def.load_gateway_definition(&mut this_file);
            def.set_index(numgateways + j as i32 + 1);

            // "gateway/<file name>", cut at its first '.', then ".tif" and "_t.tif" are added
            let base = gateway_filename
                .rsplit('/')
                .next()
                .unwrap_or(gateway_filename);
            let full = format!("gateway/{base}");
            let stem = full.split('.').next().unwrap_or(&full);

            def.set_filename(&format!("{stem}.tif"));
            def.set_thumbnail(&format!("{stem}_t.tif"));

            g.world_mut().create_gateway_def(def);
        }

        //
        // Load all LANs from file
        //

        let files = list_directory("data/lans/", ".txt");
        for filename in &files {
            LanGenerator::load_lan(g, filename);
        }
    }

    // Middle level functions for generating specific items of data

    /// Creates the player and stores it in `World::player` (nothing to return: use `g.player()`).
    pub fn generate_player(g: &mut Ctx, handle: &str) {
        // Initialise the player
        let mut player = Player::new();

        player.set_name("PLAYER");
        player.set_age(21);
        player.set_handle(handle);
        player.set_local_host(IP_LOCALHOST);
        player.set_is_targetable(false);
        player.rating.set_uplink_rating(PLAYER_START_UPLINKRATING);
        player
            .rating
            .set_neuromancer_rating(PLAYER_START_NEUROMANCERRATING);
        player.rating.set_credit_rating(PLAYER_START_CREDITRATING);
        player.set_current_account(0);

        // PORT: the C++ filled the gateway before CreatePerson. The Gateway hardware helper works
        // on `g`, so the player is stored first; the order has no other effect.
        g.world_mut().set_player(player);

        Gateway::give_starting_hardware(g);
        g.player_mut().gateway.give_starting_software();

        Connection::reset(g, "PLAYER");
        g.player_mut()
            .get_connection_mut()
            .add_vlocation(IP_UPLINKPUBLICACCESSSERVER);
        Connection::connect(g, "PLAYER");

        Person::give_link(g, "PLAYER", IP_UPLINKPUBLICACCESSSERVER);
        Person::give_link(g, "PLAYER", IP_INTERNIC);

        if TESTGAME {
            Person::give_link(g, "PLAYER", IP_GLOBALCRIMINALDATABASE);
            Person::give_link(g, "PLAYER", IP_SOCIALSECURITYDATABASE);
            Person::give_link(g, "PLAYER", IP_ACADEMICDATABASE);
            // Person::give_link(g, "PLAYER", IP_CENTRALMEDICALDATABASE);
            Person::give_link(g, "PLAYER", IP_STOCKMARKETSYSTEM);

            let comp_name = Self::get_random_computer_of_type(g, COMPUTER_TYPE_LAN);
            let comp_ip = g
                .world()
                .get_computer(&comp_name)
                .expect("UplinkAssert(comp)")
                .ip
                .clone();
            Person::give_link(g, "PLAYER", &comp_ip);
        }
    }

    pub fn generate_local_machine(g: &mut Ctx) {
        // Create a local machine for players to fall back to
        // Which can also act as a links screen
        //

        let world = g.world_mut();
        world
            .create_vlocation(IP_LOCALHOST, 284, 73)
            .set_listed(false);
        let localmachine = world.create_computer("Gateway", "Player", IP_LOCALHOST);
        localmachine.set_is_targetable(false);

        // Screen 0			:			Links

        let mut links = LinksScreen::new();
        links.set_screen_type(LINKSSCREENTYPE_PLAYERLINKS);
        links.base.set_main_title("Gateway");
        links
            .base
            .set_sub_title("Click on a link to quick-connect to that site");
        localmachine.add_computer_screen(ComputerScreen::LinksScreen(links), 0);

        // Screen 1			:			Connection disconnected locally

        let mut msg1 = DisconnectedScreen::new();
        msg1.base.set_main_title("");
        msg1.base.set_sub_title("");
        msg1.set_text_message("Connection terminated by Gateway");
        msg1.set_next_page(0);
        localmachine.add_computer_screen(ComputerScreen::DisconnectedScreen(msg1), 1);

        // Screen 2			:			Connection disconnected remotely (naughty!!)

        let mut msg2 = DisconnectedScreen::new();
        msg2.base.set_main_title("");
        msg2.base.set_sub_title("");
        msg2.set_text_message("Connection terminated by remote host");
        msg2.set_next_page(0);
        localmachine.add_computer_screen(ComputerScreen::DisconnectedScreen(msg2), 2);

        // Screen 3			:			Welcome to Gateway - time since last log in

        let mut dlg3 = DialogScreen::new();
        dlg3.base.set_main_title("Gateway");
        dlg3.base.set_sub_title("Log on complete");
        dlg3.add_widget(
            "text2",
            WIDGET_CAPTION,
            170,
            210,
            270,
            15,
            "Welcome back to your Gateway",
            "",
        );
        dlg3.add_widget_with_data(
            "okbutton",
            WIDGET_NEXTPAGE,
            300,
            350,
            50,
            20,
            "OK",
            "Click to continue",
            0,
            0,
            None,
            None,
        );
        localmachine.add_computer_screen(ComputerScreen::DialogScreen(dlg3), 3);

        // Screen 5			:			Welcome to Gateway

        let dlg5 = DialogScreen::new();
        localmachine.add_computer_screen(ComputerScreen::DialogScreen(dlg5), 5);

        // Screen 6						Successfully started

        let mut dlg6 = DialogScreen::new();
        dlg6.add_widget(
            "text2",
            WIDGET_CAPTION,
            100,
            210,
            340,
            15,
            "UPLINK Operating System successfully started on Gateway",
            "",
        );
        dlg6.add_widget_with_data(
            "okbutton",
            WIDGET_NEXTPAGE,
            300,
            350,
            50,
            20,
            "OK",
            "Click to continue",
            7,
            0,
            None,
            None,
        );
        localmachine.add_computer_screen(ComputerScreen::DialogScreen(dlg6), 6);

        // Screen 7						All done!

        let mut dlg7 = DialogScreen::new();
        dlg7.base.set_main_title("Gateway");
        dlg7.base.set_sub_title("Set up complete");
        dlg7.add_widget(
            "caption",
            WIDGET_CAPTION,
            50,
            120,
            400,
            280,
            concat!(
                "Your Gateway computer has been successfully set up and will accept only your username and password ",
                "in the future.  When you next start Uplink on your home computer, you will be connected automatically ",
                "to this Gateway.\n\n",
                "In your email you will find a message from Uplink Corporation, with some useful IP addresses.\n",
                "You will also find the Uplink Test Mission, which will allow you to practice hacking.\n\n",
                "WOULD YOU LIKE TO RUN THE TUTORIAL?\n",
                "(Recommended for first time users)"
            ),
            "",
        );
        dlg7.add_widget_with_data(
            "runtutorial",
            WIDGET_SCRIPTBUTTON,
            300,
            350,
            50,
            20,
            "Yes",
            "Run the tutorial",
            43,
            0,
            None,
            None,
        );
        dlg7.add_widget_with_data(
            "dontruntutorial",
            WIDGET_NEXTPAGE,
            200,
            350,
            50,
            20,
            "No",
            "Skip the tutorial and return to the main screen",
            0,
            0,
            None,
            None,
        );
        localmachine.add_computer_screen(ComputerScreen::DialogScreen(dlg7), 7);

        // Screen 8						404 page not found error ;)

        let mut ms8 = MessageScreen::new();
        ms8.base.set_main_title("Error 404");
        ms8.base.set_sub_title("System not found");
        ms8.set_text_message(concat!(
            "The IP address you specified did not \nhave a valid computer host.\n\n",
            "The system may be temporarily out of action\nand may return shortly."
        ));
        ms8.set_button_message("OK");
        ms8.set_next_page(0);
        localmachine.add_computer_screen(ComputerScreen::MessageScreen(ms8), 8);

        // Screen 9						Cannot connect to remotehost - they are not externally targetable

        let mut ms9 = MessageScreen::new();
        ms9.base.set_main_title("Failed to establish connection");
        ms9.base.set_sub_title("External connections not permitted");
        ms9.set_text_message(concat!(
            "The computer you tried to access will not accept external connections.\n",
            "Only connections from trusted internal systems will be allowed to connect.\n\n",
            "If you wish to establish a connection to this system,\n",
            "you must first route your connection through a trusted system owned by the same company."
        ));
        ms9.set_button_message("OK");
        ms9.set_next_page(0);
        localmachine.add_computer_screen(ComputerScreen::MessageScreen(ms9), 9);

        // Screen 10					Welcome to your new gateway

        let mut ms10 = MessageScreen::new();
        ms10.base.set_main_title("Gateway");
        ms10.base.set_sub_title("Upgrade complete");
        ms10.set_text_message(concat!(
            "Congratulations agent - your are now using your new gateway and your old one is already ",
            "being dismantled.\n\n",
            "If you view your hardware profile you should see the new gateway in place.\n",
            "If you experience any problems with this new system, please get in contact with us at\n",
            "Problems@UplinkCorporation.net."
        ));
        ms10.set_button_message("OK");
        ms10.set_next_page(0);
        localmachine.add_computer_screen(ComputerScreen::MessageScreen(ms10), 10);
    }

    pub fn generate_company_uplink(g: &mut Ctx) {
        // PORT: the C++ created the company `cu` first and then filled in its sales. The sales are
        // built first here (they touch no world data) and the company is stored with them.
        let cu = Self::generate_uplink_sales();
        let mut company = Company::new_uplink();
        company.uplink = Some(Box::new(cu));
        g.world_mut().create_company_from(company);

        Self::generate_uplink_public_access_server(g);
        Self::generate_uplink_internal_services_system(g);
        Self::generate_uplink_test_machine(g);
        Self::generate_uplink_credits_machine(g);

        let ubank_name = Self::generate_public_bank_server(g, "Uplink");
        g.world_mut()
            .get_computer_mut(&ubank_name)
            .expect("UplinkAssert(ubank)")
            .set_is_targetable(false);

        g.world_mut()
            .create_person("internal@Uplink.net", IP_UPLINKINTERNALSERVICES)
            .set_is_targetable(false);
    }

    pub fn generate_uplink_public_access_server(g: &mut Ctx) {
        let (x, y) = Self::generate_valid_map_pos();
        let comp_name = NameGenerator::generate_public_access_server_name("Uplink");
        let screenw = g.app.options.get_option_value("graphics_screenwidth");
        let screenh = g.app.options.get_option_value("graphics_screenheight");

        g.world_mut()
            .create_vlocation(IP_UPLINKPUBLICACCESSSERVER, x, y);
        let comp = g
            .world_mut()
            .create_computer(&comp_name, "Uplink", IP_UPLINKPUBLICACCESSSERVER);
        comp.set_is_targetable(false);
        comp.set_trace_speed(TRACESPEED_UPLINK_PUBLICACCESSSERVER);

        // Screen 0						Intro
        let mut msg1 = MessageScreen::new();
        msg1.base.set_main_title("Uplink");
        msg1.base.set_sub_title("About us");
        msg1.set_next_page(1);
        msg1.set_button_message("OK");
        msg1.set_text_message(concat!(
            "Welcome to the Uplink Public Access Server.\n\n",
            "Uplink Corporation maintains the largest list of freelance agents in the world, and we ",
            "have operated for the last decade with a flawless record of satisfied customers and ",
            "successful agents.  Our company acts as an anonymous job centre, bringing corporations ",
            "together with agents who can work for them.  Our company also provides rental of essential ",
            "gateway computers to all agents, which allow unparalleled security in a high risk environment.",
            "\n\nYou are here because you wish to join this company."
        ));
        comp.add_computer_screen(ComputerScreen::MessageScreen(msg1), 0);

        // Screen 1						Main menu

        let mut menu0 = MenuScreen::new();
        menu0.base.set_main_title("Uplink");
        menu0.base.set_sub_title("Public server Main Menu");
        menu0.add_option("About Us", "Find out who we are and what we do", 0, 10, -1);
        menu0.add_option(
            "Register as an Agent",
            "Click to register yourself as a new Uplink Agent",
            2,
            10,
            -1,
        );
        comp.add_computer_screen(ComputerScreen::MenuScreen(menu0), 1);

        // Screen 2						Registration info

        let mut dlg2 = DialogScreen::new();
        dlg2.base.set_main_title("Uplink");
        dlg2.base.set_sub_title("Registration");
        let body2 = format!(
            concat!(
                "Your Uplink membership package includes : \n\n",
                "- A Gateway computer at a secure location.  You will connect to this machine from your home ",
                "computer when you are working for Uplink.  You can have it upgraded at a later stage if necessary.\n\n",
                "- A low interest loan of {} credits with Uplink International Bank, ",
                "to get you started.\n\n",
                "- Access to our Bulletin Board system - the usual place for Uplink Agents to find work.\n\n",
                "- You will be officially rated as an Uplink Agent, and we will monitor your progress.  As your ",
                "rating increases you will find new avenues of work become available."
            ),
            PLAYER_START_BALANCE
        );

        dlg2.add_widget("caption1", WIDGET_CAPTION, 100, 110, 380, 270, &body2, "");
        dlg2.add_widget_with_data(
            "cancel",
            WIDGET_NEXTPAGE,
            160,
            380,
            100,
            20,
            "Cancel",
            "Cancel registration",
            1,
            0,
            None,
            None,
        );
        dlg2.add_widget_with_data(
            "continue",
            WIDGET_NEXTPAGE,
            270,
            380,
            100,
            20,
            "Continue",
            "Continue registration",
            3,
            0,
            None,
            None,
        );
        dlg2.set_return_key_button("continue");
        comp.add_computer_screen(ComputerScreen::DialogScreen(dlg2), 2);

        // Screen 3						Username / password

        let mut dlg3 = DialogScreen::new();
        dlg3.base.set_main_title("Uplink Registration");
        dlg3.base.set_sub_title("Create your agent profile");
        dlg3.add_widget(
            "name",
            WIDGET_BASIC,
            80,
            140,
            170,
            20,
            "Enter your name",
            "Enter the name you wish to use",
        );
        dlg3.add_widget(
            "password",
            WIDGET_BASIC,
            80,
            170,
            170,
            20,
            "Enter your password",
            "You will need this to access the Uplink Services Machine",
        );
        dlg3.add_widget(
            "password2",
            WIDGET_BASIC,
            80,
            200,
            170,
            20,
            "Re-type your password",
            "For verification purposes",
        );

        dlg3.add_widget(
            "nametext",
            WIDGET_TEXTBOX,
            270,
            140,
            170,
            20,
            "Fill this in",
            "Enter your name here",
        );
        dlg3.add_widget(
            "passwordtext",
            WIDGET_PASSWORDBOX,
            270,
            170,
            170,
            20,
            "",
            "Enter your password here",
        );
        dlg3.add_widget(
            "passwordtext2",
            WIDGET_PASSWORDBOX,
            270,
            200,
            170,
            20,
            "",
            "Re-type your password here",
        );

        dlg3.add_widget(
            "moretext",
            WIDGET_CAPTION,
            80,
            270,
            360,
            100,
            concat!(
                "Uplink Corporation will not ask for any more personal details.  ",
                "In the event of you being charged with illegal operations, our ",
                "corporation will be forced to disavow all knowledge of your actions, ",
                "however you will be safe from arrest as your real world address will never be stored."
            ),
            "",
        );

        dlg3.add_widget_with_data(
            "continue",
            WIDGET_SCRIPTBUTTON,
            270,
            400,
            100,
            20,
            "Done",
            "Click here when finished",
            33,
            -1,
            None,
            None,
        );
        dlg3.set_return_key_button("continue");
        comp.add_computer_screen(ComputerScreen::DialogScreen(dlg3), 3);

        // Screen 4						World map - select nearby gateway

        let mut gs4 = GenericScreen::new();
        gs4.base.set_main_title("Uplink");
        gs4.base.set_sub_title("Local gateway selection");
        gs4.set_screen_type(SCREEN_NEARESTGATEWAY);
        // C++: `#ifdef FULLGAME` / `#ifdef CODECARD_ENABLED` (both compile-time, here runtime consts)
        gs4.set_next_page(if FULLGAME && CODECARD_ENABLED { 9 } else { 5 }); // code card id, else all done
        comp.add_computer_screen(ComputerScreen::GenericScreen(gs4), 4);

        // Screen 5						Introduction to your Gateway

        let mut dlg5 = DialogScreen::new();
        dlg5.base.set_main_title("Uplink Registration");
        dlg5.base.set_sub_title("Your Gateway computer");
        let body3 = format!(
            concat!(
                "Registration is now taking place.\n\n",
                "As part of your membership, we will assign you a Gateway computer system in your chosen ",
                "server room.  This will act as your jumping off point to the rest of the Net.  When you next ",
                "log in you will connect directly from your home computer to this gateway machine, and from there ",
                "to the rest of the world.\n\n",
                "Should any of your actions be traced back to your Gateway, Uplink Corporation will disavow any ",
                "knowledge of your actions and will destroy your account.  Your Gateway machine will also be destroyed.\n\n",
                "Rental of your Gateway computer will cost {} credits a month."
            ),
            COST_UPLINK_PERMONTH
        );
        dlg5.add_widget("caption1", WIDGET_CAPTION, 100, 130, 380, 270, &body3, "");

        let continue_page = if TESTGAME { 36 } else { 34 };
        dlg5.add_widget_with_data(
            "continue",
            WIDGET_SCRIPTBUTTON,
            270,
            380,
            100,
            20,
            "Done",
            "Click here when finished",
            continue_page,
            6,
            None,
            None,
        );

        dlg5.set_return_key_button("continue");
        comp.add_computer_screen(ComputerScreen::DialogScreen(dlg5), 5);

        // Screen 6						Connecting to Gateway

        let mut dlg6 = DialogScreen::new();
        dlg6.add_widget(
            "connecting",
            WIDGET_CAPTION,
            screenw - 370,
            screenh - 30,
            370,
            20,
            "",
            "",
        );
        comp.add_computer_screen(ComputerScreen::DialogScreen(dlg6), 6);

        // Screen 9                     Code card verification

        let mut gs9 = GenericScreen::new();
        gs9.base.set_main_title("Uplink");
        gs9.base.set_sub_title("Code card verification");
        gs9.set_screen_type(SCREEN_CODECARD);
        gs9.set_next_page(5);
        comp.add_computer_screen(ComputerScreen::GenericScreen(gs9), 9);

        // Screen 10					Your Gateway is Gone!

        let mut dlg10 = DialogScreen::new();
        dlg10.base.set_main_title("Uplink");
        dlg10.base.set_sub_title("Gateway connection failed");
        dlg10.add_widget_with_data(
            "caption1",
            WIDGET_CAPTION,
            100,
            130,
            400,
            270,
            concat!(
                "We have been unable to connect you to your Gateway computer.\n\n",
                "This may be due to a fault in out communications lines, and if this is the case then we ",
                "apologise and ask you to try again later.\n\n",
                "We are aware that Uplink Agents occasionally suffer 'difficulties' with their Gateway ",
                "systems and sometimes require a new one as a result.  If this is the case, click YES below ",
                "and we will assign you a new gateway.  (There will be an administrative charge of 1000 credits ",
                "if you take this action.)  Otherwise, click NO to log off and try again later.\n\n",
                "Rent a new Gateway computer?"
            ),
            "",
            0,
            0,
            None,
            None,
        );
        dlg10.add_widget_with_data(
            "yes",
            WIDGET_SCRIPTBUTTON,
            170,
            380,
            100,
            20,
            "YES",
            "Click here to rent a new Gateway",
            50,
            4,
            None,
            None,
        );
        dlg10.add_widget_with_data(
            "no",
            WIDGET_SCRIPTBUTTON,
            300,
            380,
            100,
            20,
            "NO",
            "Click here to log out",
            51,
            -1,
            None,
            None,
        );
        comp.add_computer_screen(ComputerScreen::DialogScreen(dlg10), 10);
    }

    pub fn generate_uplink_internal_services_system(g: &mut Ctx) {
        let (x, y) = Self::generate_valid_map_pos();
        // The admin password needs `g`, so it is made before the computer is borrowed. No other
        // random number is drawn between here and the C++ point where it was made, so the random
        // sequence is unchanged.
        let admin_password = NameGenerator::generate_complex_password(g);

        g.world_mut()
            .create_vlocation(IP_UPLINKINTERNALSERVICES, x, y);
        let comp = g.world_mut().create_computer(
            NAME_UPLINKINTERNALSERVICES,
            "Uplink",
            IP_UPLINKINTERNALSERVICES,
        );

        comp.set_trace_speed(TRACESPEED_UPLINK_INTERNALSERVICESMACHINE);
        comp.set_trace_action(COMPUTER_TRACEACTION_DISCONNECT | COMPUTER_TRACEACTION_LEGAL);
        comp.security.add_system_new(SECURITY_TYPE_PROXY, 5, -1);
        comp.security.add_system_new(SECURITY_TYPE_FIREWALL, 5, -1);
        comp.security.add_system_new(SECURITY_TYPE_MONITOR, 5, -1);
        comp.set_is_targetable(false);

        // Screen 0			( ID )

        let mut uid = UserIDScreen::new();
        uid.base.set_main_title("Uplink");
        uid.base.set_sub_title("Internal Services System");
        uid.set_next_page(1);
        uid.set_difficulty(HACKDIFFICULTY_UPLINK_INTERNALSERVICESMACHINE);
        comp.add_computer_screen(ComputerScreen::UserIDScreen(uid), 0);

        // Screen 1			( Main menu )

        let mut menu2 = MenuScreen::new();
        menu2.base.set_main_title("Uplink");
        menu2.base.set_sub_title("Internal Services Main Menu");
        menu2.add_option("News Server", "Read the latest news", 11, 10, -1);
        menu2.add_option(
            "Mission list",
            "View a list of available missions",
            4,
            10,
            -1,
        );
        menu2.add_option("Software upgrades", "Buy new Software", 6, 10, -1);
        menu2.add_option("Hardware upgrades", "Buy new Hardware", 7, 10, -1);
        menu2.add_option(
            "Gateway upgrades",
            "Exchange your gateway for a better model",
            8,
            10,
            -1,
        );
        menu2.add_option(
            "Rankings",
            "View the scores of the finest Uplink agents",
            12,
            10,
            -1,
        );
        menu2.add_option(
            "Help",
            "Read documents on a wide range of skills",
            20,
            10,
            -1,
        );
        menu2.add_option(
            "Admin",
            "Access Administrator services (security level 1 required)",
            13,
            1,
            -1,
        );
        comp.add_computer_screen(ComputerScreen::MenuScreen(menu2), 1);

        // Screen 2			( Admin menu )

        let mut menu3 = MenuScreen::new();
        menu3.base.set_main_title("Uplink");
        menu3.base.set_sub_title("Administrator services");
        menu3.add_option("Access fileserver", "Upload or Download files", 14, 1, -1);
        menu3.add_option(
            "Access security",
            "View settings for security systems",
            9,
            1,
            -1,
        );
        menu3.add_option(
            "Access logs",
            "View all activity logs for this computer",
            10,
            1,
            -1,
        );
        menu3.add_option("Main Menu", "Return to the Main Menu", 1, 10, -1);
        comp.add_computer_screen(ComputerScreen::MenuScreen(menu3), 2);

        // Screen 4			( Bulletin board )

        let mut bbs = BBSScreen::new();
        bbs.set_next_page(1);
        bbs.set_contact_page(5);
        comp.add_computer_screen(ComputerScreen::BBSScreen(bbs), 4);

        // Screen 5			( Contact screen )

        let mut gs5 = GenericScreen::new();
        gs5.set_screen_type(SCREEN_CONTACTSCREEN);
        gs5.set_next_page(4);
        comp.add_computer_screen(ComputerScreen::GenericScreen(gs5), 5);

        // Screen 6			( Software market )

        let mut sss = GenericScreen::new();
        sss.set_screen_type(SCREEN_SWSALESSCREEN);
        sss.base.set_main_title("Uplink");
        sss.base.set_sub_title("Software sales");
        sss.set_next_page(1);
        comp.add_computer_screen(ComputerScreen::GenericScreen(sss), 6);

        // Screen 7			( Hardware market )

        let mut hss = GenericScreen::new();
        hss.set_screen_type(SCREEN_HWSALESSCREEN);
        hss.base.set_main_title("Uplink");
        hss.base.set_sub_title("Hardware sales");
        hss.set_next_page(1);
        comp.add_computer_screen(ComputerScreen::GenericScreen(hss), 7);

        // Screen 8			( Change Gateway )

        let mut gs8 = GenericScreen::new();
        gs8.set_screen_type(SCREEN_CHANGEGATEWAY);
        gs8.base.set_main_title("Uplink");
        gs8.base.set_sub_title("Exchange gateway");
        gs8.set_next_page(1);
        comp.add_computer_screen(ComputerScreen::GenericScreen(gs8), 8);

        // Screen 9			( Access security )

        let mut rs = GenericScreen::new();
        rs.set_screen_type(SCREEN_SECURITYSCREEN);
        rs.base.set_main_title("Uplink");
        rs.base.set_sub_title("Security Systems");
        rs.set_next_page(2);
        comp.add_computer_screen(ComputerScreen::GenericScreen(rs), 9);

        // Screen 10			( Access logs )

        let mut ls = LogScreen::new();
        ls.set_target(LOGSCREEN_TARGET_ACCESSLOGS);
        ls.base.set_main_title("Uplink");
        ls.base.set_sub_title("Access Logs");
        ls.set_next_page(2);
        comp.add_computer_screen(ComputerScreen::LogScreen(ls), 10);

        // Screen 11		( News server )

        let mut gs11 = GenericScreen::new();
        gs11.base.set_main_title("Uplink");
        gs11.base.set_sub_title("News server");
        gs11.set_screen_type(SCREEN_NEWSSCREEN);
        gs11.set_next_page(1);
        comp.add_computer_screen(ComputerScreen::GenericScreen(gs11), 11);

        // Screen 12		( Rankings screen )

        let mut gs12 = GenericScreen::new();
        gs12.base.set_main_title("Uplink");
        gs12.base.set_sub_title("Agent Rankings");
        gs12.set_screen_type(SCREEN_RANKINGSCREEN);
        gs12.set_next_page(1);
        comp.add_computer_screen(ComputerScreen::GenericScreen(gs12), 12);

        // Screen 13        ( Decypher )

        let mut cs13 = CypherScreen::new();
        cs13.base.set_main_title("Uplink");
        cs13.base
            .set_sub_title("Enter Elliptic-Curve encryption cypher");
        cs13.set_difficulty(HACKDIFFICULTY_UPLINK_INTERNALSERVICESMACHINE);
        cs13.set_next_page(2);
        comp.add_computer_screen(ComputerScreen::CypherScreen(cs13), 13);

        // Screen 14        ( File Server )

        let mut fss14 = GenericScreen::new();
        fss14.set_screen_type(SCREEN_FILESERVERSCREEN);
        fss14.base.set_main_title("Uplink");
        fss14.base.set_sub_title("File server");
        fss14.set_next_page(2);
        comp.add_computer_screen(ComputerScreen::GenericScreen(fss14), 14);

        // Screen 20		( Help menu )

        let mut ms20 = MenuScreen::new();
        ms20.base.set_main_title("Uplink");
        ms20.base.set_sub_title("Help/");
        ms20.add_option(
            "Getting Started",
            "If you're new to this, you should read this section",
            40,
            10,
            -1,
        );
        ms20.add_option(
            "Secure Connections",
            "Info on establishing secure, untraceable connections",
            21,
            10,
            -1,
        );
        ms20.add_option(
            "Gateway Computers",
            "Info on the usage of your Gateway computer",
            28,
            10,
            -1,
        );
        ms20.add_option(
            "Security Systems",
            "Info on the various security systems in use",
            35,
            10,
            -1,
        );
        ms20.add_option("Main Menu", "Return to main menu", 1, 10, -1);
        comp.add_computer_screen(ComputerScreen::MenuScreen(ms20), 20);

        // Screen 21		( Secure connections help menu )

        let mut ms21 = MenuScreen::new();
        ms21.base.set_main_title("Uplink Help");
        ms21.base.set_sub_title("Help/Secure Connections");
        ms21.add_option("Introduction", "Introduction", 22, 10, -1);
        ms21.add_option(
            "Call bouncing",
            "Info on routing your call through multiple servers",
            23,
            10,
            -1,
        );
        ms21.add_option(
            "Active Traces",
            "Info on the Active Traces that can be employed to track you",
            24,
            10,
            -1,
        );
        ms21.add_option(
            "Passive Traces",
            "Info on the Passive Traces that can be employed to track you",
            25,
            10,
            -1,
        );
        ms21.add_option(
            "Trace tracking",
            "Info on ways to monitor traces on your connection",
            26,
            10,
            -1,
        );
        ms21.add_option(
            "Active Tips",
            "How to avoid being traced by active traces",
            48,
            10,
            -1,
        );
        ms21.add_option(
            "Passive Tips",
            "How to avoid being traced by passive traces",
            27,
            10,
            -1,
        );
        ms21.add_option("Help Menu", "Return to help menu", 20, 10, -1);
        comp.add_computer_screen(ComputerScreen::MenuScreen(ms21), 21);

        // Screen 22		( Intro to secure connections )

        let ms22 = return_message(
            "Secure connections",
            "Help/Secure Connections/Introduction",
            concat!(
                "When you connect to a computer from your links screen, a direct connection is established between  ",
                "your Gateway and the computer.  This type of connection is very easy to trace back to your Gateway.  ",
                "If a connection is traced back to your Gateway, you will receive blame for the crime and risk legal action.  ",
                "\n\nThis section of the help files will teach you to establish safe connections that cannot be traced back to you."
            ),
            21,
        );
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms22), 22);

        // Screen 23		( Call bouncing help )

        let ms23 = return_message(
            "Secure connections",
            "Help/Secure Connections/Call bouncing",
            concat!(
                "The first step in establishing a secure untracable connection is to route your connection through ",
                "multiple servers all over the world.  In order to trace you, the agents on the target machine will ",
                "be forced to trace each link individually, which takes time.  They will get you eventually, but it ",
                "will take much longer and you will have warning before they finally get back to you.  ",
                "\n\nCall bouncing can be set up from the main map screen - simply click on each location to route the ",
                "call through, clicking finally on the target as the final node in the chain.  Then click on the connect ",
                "button to establish the connection."
            ),
            21,
        );
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms23), 23);

        // screen 24		( Active traces )

        let ms24 = return_message(
            "Secure connections",
            "Help/Secure Connections/Active traces",
            concat!(
                "Whenever you begin to act suspiciously, the target machine will begin to trace you.  Many computers ",
                "are running security software which begins a trace on certain key actions - such as trying hundreds ",
                "of passwords very fast (eg a Dictionary Hacker), deleting logs or use of the admin account.  ",
                "\n\nWhilst you are connected you will be traced with an Active Trace - the agents on the target machine ",
                "trace the physical lines that you are using.  This method can result in a trace back to your Gateway in ",
                "as little as one minute.  You can see an active trace as it happens on your mini-map, which should provide ",
                "ample warning before the trace completes.  You are strongly advised to disconnect before an active trace is completed."
            ),
            21,
        );
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms24), 24);

        // Screen 25		( Passive traces )

        let ms25 = return_message(
            "Secure connections",
            "Help/Secure Connections/Passive traces",
            concat!(
                "Whenever you use call bouncing to connect to a computer, you will leave logs behind on each computer ",
                "used.  Normally this is not a problem, but this information can be used against you.  If you disconnect ",
                "before an active trace has completed, the company that owned that computer may begin a Passive Trace. ",
                "\n\nThey will dig up any logs left by you and trace you that way.  It can take longer - usually several hours, ",
                "but the results can be just as severe.  Because of this, you should make every effort to delete the logs you leave.",
                "\n\nYou never need to delete all the logs on every system you bounce through.  Deleting the logs on one system ",
                "will be sufficient to break the chain and prevent traces from proceding."
            ),
            21,
        );
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms25), 25);

        // Screen 26		( Trace tracking )

        let ms26 = return_message(
            "Secure connections",
            "Help/Secure Connections/Trace tracking",
            concat!(
                "Whilst you are hacking, it is inevitable that people will attempt to trace you.  It is very important ",
                "that you avoid being traced - this of course requires that you know how close you are to being traced ",
                "at any point.",
                "\n\nIf you purchase the relevent HUD upgrade (from the software sales screen), the mini-map will show you ",
                "the progress of any Active Traces visually.  You can also purchase software ",
                "such as the Trace Tracker which will sit in the corner of your screen and will report on the progress ",
                "of any active traces.  Some later versions even report the time you have remaining before you are traced. "
            ),
            21,
        );
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms26), 26);

        // Screen 27		( Tips on secure connections )

        let ms27 = return_message(
            "Secure connections",
            "Help/Secure Connections/Passive Tips",
            concat!(
                "There are several methods of preventing Passive traces from coming back to you.  Here are some suggestions. \n",
                "First, it is best to delete any logs left behind on the target machine.  This is not always possible though, ",
                "(you may be forced to disconnect quickly) and it is not always successful.\n\n",
                "If you can establish one link in the chain on a computer that you can access easily, you can erase the logs ",
                "on that machine after every call bounce.  This requires admin access on a low security system.  Academic ",
                "systems are ideal for this due to the low consequence if caught.\n\n",
                "Another common trick is to route through a financial or government system.  These companies are typically very ",
                "resistive to opening their logs to other agents, due to the sensitivity of their data. "
            ),
            21,
        );
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms27), 27);

        // Screen 28		( Gateway info )

        let mut ms28 = MenuScreen::new();
        ms28.base.set_main_title("Uplink help");
        ms28.base.set_sub_title("Help/Gateway Computers");
        ms28.add_option(
            "Introduction",
            "Introduction to the use of Gateway computers",
            29,
            10,
            -1,
        );
        ms28.add_option(
            "Why use Gateways",
            "Info on the purpose of the Gateway",
            30,
            10,
            -1,
        );
        ms28.add_option(
            "Physical specs",
            "Info on the physical specifications of a Gateway computer",
            31,
            10,
            -1,
        );
        ms28.add_option(
            "Upgrading your Gateway",
            "Info on how to maintain your Gateway computer",
            32,
            10,
            -1,
        );
        ms28.add_option("Return to Help Menu", "Return to the help menu", 20, 10, -1);
        comp.add_computer_screen(ComputerScreen::MenuScreen(ms28), 28);

        // Screen 29		( Intro to gateway )

        let ms29 = return_message(
            "Gateway computers",
            "Help/Gateway Computers/Introduction",
            concat!(
                "When an Uplink agent logs on to his local machine, the Uplink software on his computer establishes a direct ",
                "connection to his Gateway computer and runs the Uplink Operating System on that Gateway.  This allows each ",
                "agent to retain a safe distance from his jumping off point onto the Web.  This gives Uplink Agents virtual ",
                "anonymity in the world - there is no way to trace an agent from a Gateway.\n\n",
                "A gateway also provides the processor power, storage and high bandwidth connection necessary for an Uplink ",
                "agent to complete his tasks.\n",
                "Gateway computers are rented to Uplink Agents as part of the contract and are stored in secure server rooms ",
                "around the world."
            ),
            28,
        );
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms29), 29);

        // Screen 30		( Why use a gateway )

        let ms30 = return_message(
            "Gateway computers",
            "Help/Gateway Computers/Why use Gateways",
            concat!(
                "A gateway computer gives total physical anonymity to all Uplink Agents who use them.  A gateway computer is ",
                "programmed to automatically disconnect any connection that has been traced back to the Gateway, thus ensuring ",
                "that an agents physical location can never be determined.  Gateway computers keep no logs of access.  Further more, ",
                "there is no record of which real person uses which username.  This means that, in a worse case scenario when a connection ",
                "is traced back to a Gateway, any federal agents inspecting the Gateway will be able to determine the username but ",
                "not the real person who uses that username.\n",
                "Unfortunately, when this happens it is often necessary for Uplink Corporation to disavow the agent in question, ",
                "in order to avoid any legal consequences.  That user's account and rating will be deleted, and his gateway will be ",
                "given to another agent."
            ),
            28,
        );
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms30), 30);

        // Screen 31		( Physical specs of gateway )

        let ms31 = return_message(
            "Gateway computers",
            "Help/Gateway Computers/Physical specifications",
            concat!(
                "Uplink Corporation owns several large offices around the world, which are used to house hundreds of Gateway computers.  ",
                "These computers are openly connected to the phone network, allowing Agents to connect to them from any local computer.  ",
                "The buildings are under 24 hour guard.  Engineering teams from Uplink Corporation routinely service the computers inside, ",
                "installing new hardware when requested.\n\n",
                "As such, the physical specifications of each machine can vary drastically.  With enough money, an Uplink Agent could upgrade ",
                "his Gateway to enormous power.  Typically a Gateway contains an inbound modem (to allow the agent to dial in), a high speed ",
                "processor and large quantities of memory storage, and a high speed broadband connection to the Net."
            ),
            28,
        );
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms31), 31);

        // Screen 32		( Upgrading your gateway )

        let ms32 = return_message(
            "Gateway Computers",
            "Help/Gateway Computers/Upgrading your gateway",
            concat!(
                "You can order new hardware upgrades for your Gateway from the Hardware Menu on the Uplink Internal Services Machine.  ",
                "All hardware will be installed within 24 hours by a certified Uplink technician.  Available upgrades include faster ",
                "processors, more memory capacity, higher speed connections to the Net, and many more.\n\n",
                "You can also add extra security to your Gateway by buying security camera's, laser trip wires etc, which will inform you ",
                "if your Gateway is about to be seized by Federal Agents.  This can give you warning, allowing you to destroy any incriminating ",
                "evidence.  You can even fit your Gateway to self-destruct on your command.  In this case, Federal Agents have no way of ",
                "finding out who was using the Gateway - allowing you to keep your username and established reputation.  "
            ),
            28,
        );
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms32), 32);

        // Screen 35		( Security systems )

        let mut ms35 = MenuScreen::new();
        ms35.base.set_main_title("Uplink Help");
        ms35.base.set_sub_title("Help/Security Systems");
        ms35.add_option(
            "Introduction",
            "Introduction to security systems",
            36,
            10,
            -1,
        );
        ms35.add_option(
            "the Proxy",
            "Information on the Proxy security system",
            37,
            10,
            -1,
        );
        ms35.add_option(
            "the Firewall",
            "Information on the Firewall security system",
            38,
            10,
            -1,
        );
        ms35.add_option(
            "the Monitor",
            "Information on the Monitor security system",
            49,
            10,
            -1,
        );
        ms35.add_option(
            "Voice print ID Systems",
            "Information on voice print security",
            50,
            10,
            -1,
        );
        ms35.add_option(
            "Elliptic Curve Systems",
            "Information on the Elliptic Curve system",
            51,
            10,
            -1,
        );
        ms35.add_option("Help Menu", "Return to the help menu", 20, 10, -1);
        comp.add_computer_screen(ComputerScreen::MenuScreen(ms35), 35);

        // Screen 36		( Introduction to security systems )

        let ms36 = return_message(
            "Security Systems Help",
            "Help/Security Systems/Introduction",
            concat!(
                "As you accept more work from Uplink Corporation you will encounter more and more security systems of different types and strengths.  ",
                "All security systems are designed with one primary goal - to keep you out, and as an Agent you must learn to bypass them.\n\n",
                "Most systems can be simply disabled once Administrative access has been gained, but this method is considered unprofessional and is ",
                "'noisy' - most systems will detect this act and will begin tracing your connection immediately.\n",
                "In nearly all cases a better method exists - most systems can be bypassed or fooled in some way."
            ),
            35,
        );
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms36), 36);

        // Screen 37		( Proxy systems )

        let ms37 = return_message(
            "Security Systems Help",
            "Help/Security Systems/the Proxy",
            concat!(
                "The primary purpose of the Proxy server is to stop unauthorised users from modifying records or logs.  A system with a good Proxy server ",
                "will permit you to examine records and logs but will not allow you to change them.  As such, you will not be able to remove any logs of ",
                "your actions, nor will you be able to make any alterations to any records on the target system.\n\n",
                "Proxy servers are extremely popular on government data systems (such as the social security database) and other systems were the data ",
                "itself is sensitive.  They can be bypassed simply by disabling them on the target computer (which will typically require admin access) ",
                "or by running a Proxy_Disable tool.  Both of these methods will cause you to get noticed.  You can also bypass them silently with more ",
                "advanced software tools and a Connection Analyser."
            ),
            35,
        );
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms37), 37);

        // Screen 38		( Firewall systems )

        let ms38 = return_message(
            "Security Systems Help",
            "Help/Security Systems/the Firewall",
            concat!(
                "The primary purpose of the firewall is to prevent unauthorised usage of files on the target system.  A good Firewall will stop you from ",
                "copying, deleting or downloading any files in existence on the target computer.\n\n",
                "Firewalls are common on File Servers or any system where the contents of the files are sensitive or valuable. They can be bypassed ",
                "simply by disabling them on the target computer (which will typically require admin access) or by running a Firewall_Disable tool.  ",
                "Both of these methods will cause you to get noticed.  You can also bypass them silently with more advanced software tools and a ",
                "Connection Analyser."
            ),
            35,
        );
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms38), 38);

        // Screen 40		( Getting started menu )

        let mut ms40 = MenuScreen::new();
        ms40.base.set_main_title("Uplink Help");
        ms40.base.set_sub_title("Help/Getting Started");
        ms40.add_option("Introduction", "What this section is about", 41, 10, -1);
        ms40.add_option(
            "Basic tools",
            "Information on the first software tools to buy",
            42,
            10,
            -1,
        );
        ms40.add_option(
            "the Connection",
            "Information on opening a connection to a target",
            43,
            10,
            -1,
        );
        ms40.add_option(
            "Password Breaking",
            "Information on breaking the opening password",
            44,
            10,
            -1,
        );
        ms40.add_option(
            "Stealing",
            "Information on stealing the files you need",
            45,
            10,
            -1,
        );
        ms40.add_option("Hide", "Information on covering your tracks", 46, 10, -1);
        ms40.add_option(
            "Missions",
            "Information on getting new mission contracts",
            47,
            10,
            -1,
        );
        ms40.add_option("Help Menu", "Return to the help menu", 20, 10, -1);
        comp.add_computer_screen(ComputerScreen::MenuScreen(ms40), 40);

        // Screen 41		( Intro to getting started )

        let ms41 = return_message(
            "Getting Started",
            "Help/Getting Started/Introduction",
            concat!(
                "This section of the help documents is designed to help new Uplink Agents with their first few hours online.\n\n",
                "We will start with the Uplink Test Mission.  You will notice this mission has already been given to you (it is given to all new agents) ",
                "and you can read the details by clicking on its icon at the bottom right of the screen.  This mission is the perfect opportunity to ",
                "test yourself against real computer defenses, without risking jail time.\n\n",
                "We will go over the basic equipment needed, how to do the mission, and how to aquire more missions afterwards."
            ),
            40,
        );
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms41), 41);

        // Screen 42		( Basic tools )

        let mut ms42 = return_message(
            "Getting Started",
            "Help/Getting Started/Basic Tools",
            concat!(
                "The basic tools that you will require for the Uplink Test mission will also be required for almost every other type of mission, so you ",
                "will not be wasting your money when you buy these.  We recommend you purchase these tools immediately as they will be essential.\n\n",
                "Password Breaker (V1.0) - You will need this to break through password protection screens.\n",
                "Log Deleter (V1.0) - For removing traces of your actions.\n",
                "Trace Tracker (V2.0) - Will allow you to monitor your communication lines, so you will know when you are about to be traced.  ",
                "We recommend version 2.0 for beginners as it will show you the percentage of your connection that has ",
                "been traced.\n\n",
                "You can purchase all of these items with your starting loan, from the 'Software Sales' section of this site."
            ),
            40,
        );
        ms42.set_mail_this_to_me(true);
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms42), 42);

        // Screen 43		( the Connection )

        let mut ms43 = return_message(
            "Getting Started",
            "Help/Getting Started/The Connection",
            concat!(
                "The Uplink Test Mission requires you to connect to the Uplink Test Machine and steal a file.  You should not connect directly to it - ",
                "they would be able to trace you very quickly.  Instead, you should bounce our call around the world.\n",
                "Open the Communications map by clicking on the world map, top right.  Click on a few other nodes to bounce through them.  Bounce through ",
                "at least 3 other nodes before the Uplink Test Machine.  Click on the connect button to connect to the target computer.\n\n",
                "Now would be a good time to start up your TraceTracker  (click the software menu bottom left).  This tool will tell you if you are being traced.  ",
                "Once you start hacking, you'll have a couple of minutes before they trace you and cut you off.  In the real world, this would get you in trouble."
            ),
            40,
        );
        ms43.set_mail_this_to_me(true);
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms43), 43);

        // Screen 44		( The hack )

        let mut ms44 = return_message(
            "Getting Started",
            "Help/Getting Started/Password breaking",
            concat!(
                "Once connected to the Uplink Test Machine, you will see the main log in screen.  Since you do not have security clearence to use this machine, ",
                "you will need to hack through this screen.  Passwords can be broken in one of two ways - either using a dictionary hacker (which tries every english ",
                "word from a large dictionary) or a password cracker, which takes longer but can break non-english-word passwords.  \n\n",
                "Run your Password Breaker and target the password box.  It will take around 10 seconds to break this password (security is low) and they ",
                "will begin tracing you immediately."
            ),
            40,
        );
        ms44.set_mail_this_to_me(true);
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms44), 44);

        // Screen 45		( Stealing the file )

        let mut ms45 = return_message(
            "Getting Started",
            "Help/Getting Started/Stealing the file",
            concat!(
                "Once you are in the system, you should have no trouble accessing the file server and downloading the file you need.  (Use the file copier and download ",
                "the file 'uplink test data' into your memory banks.)  In many cases this might not be so easy - you may find that Firewall or Proxy systems are running, ",
                "which will prevent you from just copying files easily.  You will learn to bypass these in time.\n\n",
                "Once you've got the file, disconnect from the system as soon as possible.\n",
                "You can now send it to the employer (by clicking on 'reply' in the mission screen) and receive your payment, which in this case ",
                "will be zero (it was only a test after all)."
            ),
            40,
        );
        ms45.set_mail_this_to_me(true);
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms45), 45);

        // Screen 46		( Hide )

        let mut ms46 = return_message(
            "Getting Started",
            "Help/Getting Started/Covering your tracks",
            concat!(
                "The final step in any hack is to cover your tracks.  ",
                "Access the log banks and run your log deleter.  Remember to keep an eye on the trace tracker.  Locate the logs that you have created from ",
                "your actions (especially the ones that detail where your connection came from) and target your log deleter on them.  The logs will be ",
                "deleted but will leave a clear marker behind.  More advanced versions of the log deleter can cover your tracks better, but this will be ",
                "sufficient for now.\n\n",
                "In the real world of hacking, you would probably not delete the logs on the target machine - you would not have time.  Instead, you would ",
                "bounce your connection through a system on which you have admin access.  After the hack, you would connect to that system and remove those ",
                "logs in safety - breaking the chain of logs between yourself and the target."
            ),
            40,
        );
        ms46.set_mail_this_to_me(true);
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms46), 46);

        // Screen 47		( Missions )

        let mut ms47 = return_message(
            "Getting Started",
            "Help/Getting Started/Future missions",
            concat!(
                "The Uplink test mission is the easiest mission you will ever attempt.  Once you feel good enough, connect to the Uplink Internal Services ",
                "Machine and go to the Missions section.  You will see some missions that are already available.  You may be able to auto-accept some of them, ",
                "and you will need to contact the employer to accept others.  As your rating increases, more and more missions will appear here.  You will find ",
                "that you will need to upgrade your hardware and software before the more complex missions can be attempted."
            ),
            40,
        );
        ms47.set_mail_this_to_me(true);
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms47), 47);

        // Screen 48		( Tips on secure connections (active) )

        let ms48 = return_message(
            "Secure connections",
            "Help/Secure Connections/Active Tips",
            concat!(
                "There are several important points to keep in mind when setting up connections. If done correctly, you can give yourself the extra few ",
                "seconds need to get the job done without being caught.\n\n",
                "Firstly, bouncing through a random computer will not work against any reasonably secure system. They will be able to trace through that link in seconds.\n",
                "If you bounce through a system that you have already compromised, and have an access account available, the trace will take much longer.\n",
                "If you bounce through a system on which you have Admin access, the trace will take longer still.\n",
                "In general, the higher security the system you bounce through, the longer it will take to trace through that link.\n\n",
                "It is worth keeping in mind that you can lose your username and password login on a system if you route through it too many times."
            ),
            21,
        );
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms48), 48);

        // Screen 49		( Monitor systems )

        let ms49 = return_message(
            "Security Systems Help",
            "Help/Security Systems/the Monitor",
            concat!(
                "Monitor systems are designed to detect unauthorised activity. They are usually triggered when a user attempts to hack a password, ",
                "or makes use of the Admin account, or starts deleting files. Once they have detected illegal activity they will sound the alarm and ",
                "start an Active Trace on your connection. They will also flag your logs as suspicious.\n\n",
                "Monitor systems will also periodically check the other security systems running locally, to see if they have been disabled.\n\n",
                "You can bypass a monitor system with a connection Analyser HUD upgrade and a Monitor Bypass.  This will allow you to hack without being noticed."
            ),
            35,
        );
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms49), 49);

        // Screen 50        ( Voice print ID systems )

        let ms50 = return_message(
            "Security Systems Help",
            "Help/Security Systems/Voice Print ID",
            concat!(
                "Many systems that require a specific person to log in use Voice Print technology.  The users voice is first sampled and analysed ",
                "by the company, then digitised and encrypted onto their system.  When that person logs in, he reproduces a certain phrase or set ",
                "of words.  These are then analysed and compared.\n\n",
                "Recent advances in this area have made it very difficult to crack.  For many years Digital Approximation software was capable of ",
                "defeating these systems, but this is no longer the case.  In most modern systems, the only real way is to obtain a sample of the users ",
                "voice directly.  Agents can then use Voice Analysis software to reproduce the phrase digitally from that sample."
            ),
            35,
        );
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms50), 50);

        // Screen 51        ( Elliptic Curve systems )

        let ms51 = return_message(
            "Security Systems Help",
            "Help/Security Systems/Elliptic Curve Security",
            concat!(
                "Faced with the growing threat of hackers, many companies have begun using Elliptic Curve Cryptography.  A user on a system must ",
                "input a large block of data in sequence, synchronised with the system.  This cannot be done by hand - the timings must be perfect. ",
                "Elliptic Curve Cryptography is popular due to the large amount of CPU power needed to force crack it.\n\n",
                "However, as with all cryptography, even this system can be broken given enough time and CPU cycles.  Agents can use the Decypher ",
                "software tool to force crack one of these systems, but a fast CPU rig will be required."
            ),
            35,
        );
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms51), 51);

        // Add in the admin account

        let mut record = Record::new();
        record.add_field(RECORDBANK_NAME, RECORDBANK_ADMIN);
        record.add_field(RECORDBANK_PASSWORD, &admin_password);
        record.add_field(RECORDBANK_SECURITY, "1");
        comp.recordbank.add_record(record);

        // Add in some interesting data

        comp.databank.set_size(100);

        let mut program = Data::new();
        program.set_title("Uplink_Agent_List");
        program.set_details(DATATYPE_PROGRAM, 2, 0, 0, 1.0, SOFTWARETYPE_OTHER);
        comp.databank.put_data(program);

        for i in 0..10 {
            let datatitle = format!("Uplink_Agent_Data {i}.dat");
            let size = 5;
            let encrypted = 7;

            if i != 4 {
                // Miss out 4 to make some of the names Unknown
                let mut file = Data::new();
                file.set_title(&datatitle);
                file.set_details(DATATYPE_DATA, size, encrypted, 0, 1.0, SOFTWARETYPE_NONE);
                comp.databank.put_data(file);
            }
        }
    }

    pub fn generate_uplink_test_machine(g: &mut Ctx) {
        let (x, y) = Self::generate_valid_map_pos();
        g.world_mut().create_vlocation(IP_UPLINKTESTMACHINE, x, y);

        // Generate a computer with some data on it

        let name = "Uplink Test Machine";
        let mut comp = Computer::new();
        comp.set_name(name);
        comp.set_company_name("Uplink");
        comp.set_ip(IP_UPLINKTESTMACHINE);
        comp.set_is_targetable(false);

        comp.set_trace_speed(TRACESPEED_UPLINK_TESTMACHINE);
        comp.set_trace_action(COMPUTER_TRACEACTION_DISCONNECT);
        comp.security.add_system_new(SECURITY_TYPE_MONITOR, 1, -1);
        comp.databank.set_size(20);

        let mut data1 = Data::new();
        data1.set_title("Uplink test data");
        data1.set_details(DATATYPE_DATA, 6, 0, 0, 1.0, SOFTWARETYPE_NONE);
        comp.databank.put_data(data1);

        let mut data2 = Data::new();
        data2.set_title("Random file");
        data2.set_details(DATATYPE_DATA, 3, 4, 1, 1.0, SOFTWARETYPE_NONE);
        comp.databank.put_data(data2);

        let mut data3 = Data::new();
        data3.set_title("Sample program");
        data3.set_details(DATATYPE_PROGRAM, 2, 0, 0, 1.0, SOFTWARETYPE_NONE);
        comp.databank.put_data(data3);

        // Generate an admin log on

        let mut record = Record::new();
        record.add_field(RECORDBANK_NAME, RECORDBANK_ADMIN);
        record.add_field(RECORDBANK_PASSWORD, "rosebud");
        record.add_field(RECORDBANK_SECURITY, "1");
        comp.recordbank.add_record(record);

        // Generate an interface

        // Screen 0			( Opening message )

        let mut ms = MessageScreen::new();
        ms.base.set_main_title("Uplink");
        ms.base.set_sub_title("Welcome to the Uplink Test Machine");
        ms.set_text_message(concat!(
            "This will be your first opportunity to hack into a real computer system with real defenses. ",
            "There is no point in trying to hack a real computer system until you can do the Uplink Test in your sleep.\n",
            "\nYou will be traced during your attempts, so remember to monitor the communications lines, however no action will be taken against you should you be traced.\n",
            "\nHINT : Never connect directly to your target server.  Bounce your connection around the world - ",
            "it will make you much harder to track and give you longer to complete your work.\n",
            "Remember that you will leave logs on the computers you use as relays (which can be traced back to you) - ideally your connection should use a computer which allows you easy and risk free log access, ",
            "so you can erase your tracks without fear of punishment.\n",
        ));
        ms.set_button_message("Start the test");
        ms.set_next_page(1);
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms), 0);

        // Screen 1			( User ID )

        let mut uid = UserIDScreen::new();
        uid.base.set_main_title("Uplink");
        uid.base.set_sub_title("Test machine");
        uid.set_difficulty(HACKDIFFICULTY_UPLINK_TESTMACHINE);
        uid.set_next_page(2);
        comp.add_computer_screen(ComputerScreen::UserIDScreen(uid), 1);

        // Screen 2			( Main menu )

        let mut menu = MenuScreen::new();
        menu.base.set_main_title("Uplink");
        menu.base.set_sub_title("Test machine Main Menu");
        menu.add_option("Access fileserver", "", 4, 10, -1);
        menu.add_option("View Logs", "", 3, 10, -1);
        menu.add_option("Console", "", 5, 10, -1);
        comp.add_computer_screen(ComputerScreen::MenuScreen(menu), 2);

        // Screen 3			( Logs )

        let mut ls = LogScreen::new();
        ls.set_target(LOGSCREEN_TARGET_ACCESSLOGS);
        ls.base.set_main_title("Uplink Test Machine");
        ls.base.set_sub_title("Access logs");
        ls.set_next_page(2);
        comp.add_computer_screen(ComputerScreen::LogScreen(ls), 3);

        // Screen 4			( file server )

        let mut gs4 = GenericScreen::new();
        gs4.set_screen_type(SCREEN_FILESERVERSCREEN);
        gs4.base.set_main_title("Uplink Test Machine");
        gs4.base.set_sub_title("File Server");
        gs4.set_next_page(2);
        comp.add_computer_screen(ComputerScreen::GenericScreen(gs4), 4);

        // Screen 5			( Console )

        let mut gs5 = GenericScreen::new();
        gs5.set_screen_type(SCREEN_CONSOLESCREEN);
        gs5.base.set_main_title("Uplink Test Machine");
        gs5.base.set_sub_title("Console");
        gs5.set_next_page(2);
        comp.add_computer_screen(ComputerScreen::GenericScreen(gs5), 5);

        g.world_mut().create_computer_from(comp);
    }

    pub fn generate_uplink_credits_machine(g: &mut Ctx) {
        let (x, y) = Self::generate_valid_map_pos();
        g.world_mut()
            .create_vlocation(IP_UPLINKCREDITSMACHINE, x, y);

        // Generate a computer with some data on it

        let mut comp = Computer::new();
        comp.set_name("Uplink Credits Machine");
        comp.set_company_name("Uplink");
        comp.set_ip(IP_UPLINKCREDITSMACHINE);
        comp.set_is_targetable(false);

        let mut ms = MessageScreen::new();
        ms.base.set_main_title("Uplink");
        ms.base.set_sub_title("The Credits Machine");
        ms.set_text_message(concat!(
            "Produced by INTROVERSION SOFTWARE\n\n",
            "Chris 'Abandon' Delay\n",
            "Mark 'Monkey' Morris\n",
            "Tom 'Blissfully-ignorant' Arundel\n\n",
            "WITH HELP FROM\n",
            "John 'Rotton-smell' Knottenbelt'\n",
            "Gareth 'D-V-D' Sidebottom'\n\n",
            "DEVELOPMENT TIME : 816 days\n",
            "SOURCE CODE SIZE : 93102 lines\n\n",
            "A-N-D  C-O-U-N-T-I-N-G",
        ));
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms), 0);

        g.world_mut().create_computer_from(comp);
    }

    pub fn generate_company_government(g: &mut Ctx) {
        Self::generate_company_named(g, "Government", 5, 1, 10, -20);
    }

    pub fn generate_global_criminal_database(g: &mut Ctx) {
        // Create the VLocation

        let (x, y) = Self::generate_valid_map_pos();
        g.world_mut()
            .create_vlocation(IP_GLOBALCRIMINALDATABASE, x, y);

        // Create the computer

        let mut comp = Computer::new();
        comp.set_type(COMPUTER_TYPE_UNKNOWN);
        comp.set_name("Global Criminal Database");
        comp.set_company_name("Government");
        comp.set_ip(IP_GLOBALCRIMINALDATABASE);
        comp.set_trace_speed(TRACESPEED_GLOBALCRIMINALDATABASE);
        comp.set_trace_action(COMPUTER_TRACEACTION_DISCONNECT | COMPUTER_TRACEACTION_LEGAL);
        comp.set_is_targetable(false);
        comp.security.add_system_new(SECURITY_TYPE_PROXY, 1, -1);
        comp.security.add_system_new(SECURITY_TYPE_MONITOR, 1, -1);

        // Screen 0					( Opening message screen )

        let mut ms = MessageScreen::new();
        ms.base.set_main_title("Global Criminal Database");
        ms.base
            .set_sub_title("Unauthorised Access will be punished");
        ms.set_text_message(concat!(
            "This is a Government owned computer system - all attempts at\n",
            "illegal access will be severely punished to the full extent of the law.\n\n",
            "An Elliptic-Curve encryption cypher will be required for full system access.\n\n",
            "Valid User Accounts\n",
            "====================\n\n",
            "admin     - Complete system wide access\n",
            "readwrite  - Read/Write access to all records\n",
            "readonly   - Read only access to all records\n",
        ));
        ms.set_button_message("Log in");
        ms.set_next_page(1);
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms), 0);

        // Screen 1					( High Security Screen )

        let mut hs = HighSecurityScreen::new();
        hs.base.set_main_title("Global Criminal Database");
        hs.base.set_sub_title("Authorisation required");
        hs.add_system("UserID / password verification", 2);
        hs.add_system("Elliptic-Curve Encryption Cypher", 3);
        hs.set_next_page(4);
        comp.add_computer_screen(ComputerScreen::HighSecurityScreen(hs), 1);

        // Screen 2					( User ID Screen )

        let mut uid = UserIDScreen::new();
        uid.base.set_main_title("Global Criminal Database");
        uid.base.set_sub_title("Log in");
        uid.set_difficulty(HACKDIFFICULTY_GLOBALCRIMINALDATABASE);
        uid.set_next_page(1);
        comp.add_computer_screen(ComputerScreen::UserIDScreen(uid), 2);

        // Screen 3					( Cypher screen )

        let mut cs = CypherScreen::new();
        cs.base.set_main_title("Global Criminal Database");
        cs.base
            .set_sub_title("Enter elliptic-curve encryption cypher");
        cs.set_difficulty(HACKDIFFICULTY_GLOBALCRIMINALDATABASE);
        cs.set_next_page(1);
        comp.add_computer_screen(ComputerScreen::CypherScreen(cs), 3);

        // Screen 4					( Main Menu )

        let mut menu = MenuScreen::new();
        menu.base.set_main_title("Global Criminal Database");
        menu.base.set_sub_title("Main Menu");
        menu.add_option("Search Records", "", 7, 10, -1);
        if globals::TESTGAME {
            menu.add_option("Edit Records", "", 6, 1, -1);
        }
        menu.add_option("View Logs", "", 6, 1, -1);
        menu.add_option("Security", "", 9, 1, -1);
        comp.add_computer_screen(ComputerScreen::MenuScreen(menu), 4);

        // Screen 5					( Records )

        let mut rs = GenericScreen::new();
        rs.set_screen_type(SCREEN_RECORDSCREEN);
        rs.base.set_main_title("Global Criminal Database");
        rs.base.set_sub_title("Criminal Records");
        rs.set_next_page(4);
        comp.add_computer_screen(ComputerScreen::GenericScreen(rs), 5);

        // Screen 6					( Logs )

        let mut ls = LogScreen::new();
        ls.set_target(LOGSCREEN_TARGET_ACCESSLOGS);
        ls.base.set_main_title("Global Criminal Database");
        ls.base.set_sub_title("Access logs");
        ls.set_next_page(4);
        comp.add_computer_screen(ComputerScreen::LogScreen(ls), 6);

        // Screen 7					( Search )

        let mut ds7 = DialogScreen::new();
        ds7.base.set_main_title("Global Criminal Database");
        ds7.base.set_sub_title("Search for Criminal Record");
        ds7.add_widget(
            "name",
            WIDGET_TEXTBOX,
            150,
            200,
            200,
            15,
            "Enter name here",
            "Type the name of the person to search for here",
        );
        ds7.add_widget_with_data(
            "OK",
            WIDGET_SCRIPTBUTTON,
            150,
            240,
            80,
            15,
            "OK",
            "Click here when done",
            15,
            -1,
            None,
            None,
        );
        ds7.add_widget_with_data(
            "Cancel",
            WIDGET_NEXTPAGE,
            270,
            240,
            80,
            15,
            "Cancel",
            "Click here to return to the menu",
            4,
            -1,
            None,
            None,
        );

        ds7.add_widget("border1", WIDGET_BASIC, 120, 170, 260, 4, "", "");
        ds7.add_widget("border2", WIDGET_BASIC, 120, 266, 260, 4, "", "");
        ds7.add_widget("border3", WIDGET_BASIC, 120, 170, 4, 100, "", "");
        ds7.add_widget("border4", WIDGET_BASIC, 380, 170, 4, 100, "", "");

        ds7.set_return_key_button("OK");
        ds7.set_escape_key_button("Cancel");

        comp.add_computer_screen(ComputerScreen::DialogScreen(ds7), 7);

        // Screen 8					( Search )

        let mut gs8 = GenericScreen::new();
        gs8.set_screen_type(SCREEN_CRIMINALSCREEN);
        gs8.base.set_main_title("Global Criminal Database");
        gs8.base.set_sub_title("View Criminal Records");
        gs8.set_next_page(4);
        comp.add_computer_screen(ComputerScreen::GenericScreen(gs8), 8);

        // Screen 9					( Security )

        let mut gs9 = GenericScreen::new();
        gs9.set_screen_type(SCREEN_SECURITYSCREEN);
        gs9.base.set_main_title("Global Criminal Database");
        gs9.base.set_sub_title("Security settings");
        gs9.set_next_page(4);
        comp.add_computer_screen(ComputerScreen::GenericScreen(gs9), 9);

        // Add in the user accounts

        let mut record = Record::new();
        record.add_field(RECORDBANK_NAME, RECORDBANK_ADMIN);
        record.add_field(
            RECORDBANK_PASSWORD,
            &NameGenerator::generate_complex_password(g),
        );
        record.add_field(RECORDBANK_SECURITY, "1");
        comp.recordbank.add_record(record);

        let mut record2 = Record::new();
        record2.add_field(RECORDBANK_NAME, RECORDBANK_READWRITE);
        record2.add_field(
            RECORDBANK_PASSWORD,
            &NameGenerator::generate_complex_password(g),
        );
        record2.add_field(RECORDBANK_SECURITY, "2");
        comp.recordbank.add_record(record2);

        let mut record3 = Record::new();
        record3.add_field(RECORDBANK_NAME, RECORDBANK_READONLY);
        record3.add_field(RECORDBANK_PASSWORD, &NameGenerator::generate_password(g));
        record3.add_field(RECORDBANK_SECURITY, "4");
        comp.recordbank.add_record(record3);

        g.world_mut().create_computer_from(comp);
    }

    pub fn generate_international_social_security_database(g: &mut Ctx) {
        // Create the VLocation

        let (x, y) = Self::generate_valid_map_pos();
        g.world_mut()
            .create_vlocation(IP_SOCIALSECURITYDATABASE, x, y);

        // Create the computer

        let mut comp = Computer::new();
        comp.set_type(COMPUTER_TYPE_UNKNOWN);
        comp.set_name("International Social Security Database");
        comp.set_company_name("Government");
        comp.set_ip(IP_SOCIALSECURITYDATABASE);
        comp.set_trace_speed(TRACESPEED_INTERNATIONALSOCIALSECURITYDATABASE);
        comp.set_trace_action(COMPUTER_TRACEACTION_DISCONNECT | COMPUTER_TRACEACTION_FINE);
        comp.set_is_targetable(false);
        comp.security.add_system_new(SECURITY_TYPE_PROXY, 1, -1);
        comp.security.add_system_new(SECURITY_TYPE_MONITOR, 1, -1);

        // Screen 0					( Opening message screen )

        let mut ms = MessageScreen::new();
        ms.base.set_main_title("Social Security Database");
        ms.base
            .set_sub_title("Unauthorised Access will be punished");
        ms.set_text_message(concat!(
            "This is a Government owned computer system - all attempts at\n",
            "illegal access will be severely punished to the full extent of the law.\n\n",
            "Valid User Accounts\n",
            "====================\n\n",
            "admin    - Complete system wide access\n",
            "readwrite  - Read/Write access to all records\n",
            "readonly  - Read only access to all records\n",
        ));
        ms.set_button_message("Log in");
        ms.set_next_page(1);
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms), 0);

        // Screen 1					( User ID Screen )

        let mut uid = UserIDScreen::new();
        uid.base.set_main_title("Social Security Database");
        uid.base.set_sub_title("Log in");
        uid.set_difficulty(HACKDIFFICULTY_INTERNATIONALSOCIALSECURITYDATABASE);
        uid.set_next_page(2);
        comp.add_computer_screen(ComputerScreen::UserIDScreen(uid), 1);

        // Screen 2					( Main Menu )

        let mut menu = MenuScreen::new();
        menu.base.set_main_title("Social Security Database");
        menu.base.set_sub_title("Main Menu");
        menu.add_option("Search Records", "", 5, 3, -1);
        if globals::TESTGAME {
            menu.add_option("View Records", "", 3, 10, -1);
        }
        menu.add_option("View Logs", "", 4, 1, -1);
        comp.add_computer_screen(ComputerScreen::MenuScreen(menu), 2);

        // Screen 3					( Records )

        let mut rs = GenericScreen::new();
        rs.set_screen_type(SCREEN_RECORDSCREEN);
        rs.base.set_main_title("Social Security Database");
        rs.base.set_sub_title("Records");
        rs.set_next_page(2);
        comp.add_computer_screen(ComputerScreen::GenericScreen(rs), 3);

        // Screen 4					( Logs )

        let mut ls = LogScreen::new();
        ls.set_target(LOGSCREEN_TARGET_ACCESSLOGS);
        ls.base.set_main_title("Social Security Database");
        ls.base.set_sub_title("Access logs");
        ls.set_next_page(2);
        comp.add_computer_screen(ComputerScreen::LogScreen(ls), 4);

        // Screen 5					( Search )

        let mut ds5 = DialogScreen::new();
        ds5.base.set_main_title("Social Security Database");
        ds5.base.set_sub_title("Search for social security record");
        ds5.add_widget(
            "name",
            WIDGET_TEXTBOX,
            150,
            200,
            200,
            15,
            "Enter name here",
            "Type the name of the person to search for here",
        );
        ds5.add_widget_with_data(
            "OK",
            WIDGET_SCRIPTBUTTON,
            150,
            240,
            80,
            15,
            "OK",
            "Click here when done",
            17,
            -1,
            None,
            None,
        );
        ds5.add_widget_with_data(
            "Cancel",
            WIDGET_NEXTPAGE,
            270,
            240,
            80,
            15,
            "Cancel",
            "Click here to return to the menu",
            2,
            -1,
            None,
            None,
        );

        ds5.add_widget("border1", WIDGET_BASIC, 120, 170, 260, 4, "", "");
        ds5.add_widget("border2", WIDGET_BASIC, 120, 266, 260, 4, "", "");
        ds5.add_widget("border3", WIDGET_BASIC, 120, 170, 4, 100, "", "");
        ds5.add_widget("border4", WIDGET_BASIC, 380, 170, 4, 100, "", "");

        ds5.set_return_key_button("OK");
        ds5.set_escape_key_button("Cancel");

        comp.add_computer_screen(ComputerScreen::DialogScreen(ds5), 5);

        // Screen 6					( View record )

        let mut gs6 = GenericScreen::new();
        gs6.base.set_main_title("Social Security Database");
        gs6.base.set_sub_title("View record");
        gs6.set_next_page(2);
        gs6.set_screen_type(SCREEN_SOCSECSCREEN);
        comp.add_computer_screen(ComputerScreen::GenericScreen(gs6), 6);

        // Add in the accounts

        let mut record = Record::new();
        record.add_field(RECORDBANK_NAME, RECORDBANK_ADMIN);
        record.add_field(RECORDBANK_PASSWORD, &NameGenerator::generate_password(g));
        record.add_field(RECORDBANK_SECURITY, "1");
        record.add_field_int("Social Security", -1);
        comp.recordbank.add_record(record);

        let mut record2 = Record::new();
        record2.add_field(RECORDBANK_NAME, RECORDBANK_READWRITE);
        record2.add_field(RECORDBANK_PASSWORD, &NameGenerator::generate_password(g));
        record2.add_field(RECORDBANK_SECURITY, "2");
        record2.add_field_int("Social Security", -1);
        comp.recordbank.add_record(record2);

        let mut record3 = Record::new();
        record3.add_field(RECORDBANK_NAME, RECORDBANK_READONLY);
        record3.add_field(RECORDBANK_PASSWORD, &NameGenerator::generate_password(g));
        record3.add_field(RECORDBANK_SECURITY, "4");
        record3.add_field_int("Social Security", -1);
        comp.recordbank.add_record(record3);

        g.world_mut().create_computer_from(comp);
    }

    pub fn generate_central_medical_database(g: &mut Ctx) {
        // Create the VLocation

        let (x, y) = Self::generate_valid_map_pos();
        g.world_mut()
            .create_vlocation(IP_CENTRALMEDICALDATABASE, x, y);

        // Create the computer

        let mut comp = Computer::new();
        comp.set_type(COMPUTER_TYPE_UNKNOWN);
        comp.set_name("Central Medical Database");
        comp.set_company_name("Government");
        comp.set_ip(IP_CENTRALMEDICALDATABASE);
        comp.set_trace_speed(TRACESPEED_CENTRALMEDICALDATABASE);
        comp.set_trace_action(COMPUTER_TRACEACTION_DISCONNECT | COMPUTER_TRACEACTION_WARNINGMAIL);
        comp.set_is_targetable(false);
        comp.security.add_system_new(SECURITY_TYPE_PROXY, 1, -1);
        comp.security.add_system_new(SECURITY_TYPE_MONITOR, 1, -1);

        // Screen 0					( Opening message screen )

        let mut ms = MessageScreen::new();
        ms.base.set_main_title("Central Medical Database");
        ms.base.set_sub_title("No Unauthorised Access");
        ms.set_text_message("admin - Complete read/write/delete access on all records");
        ms.set_button_message("Log in");
        ms.set_next_page(1);
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms), 0);

        // Screen 1					( User ID Screen )

        let mut uid = UserIDScreen::new();
        uid.base.set_main_title("Central Medical Database");
        uid.base.set_sub_title("Log in");
        uid.set_difficulty(HACKDIFFICULTY_CENTRALMEDICALDATABASE);
        uid.set_next_page(2);
        comp.add_computer_screen(ComputerScreen::UserIDScreen(uid), 1);

        // Screen 2					( Main Menu )

        let mut menu = MenuScreen::new();
        menu.base.set_main_title("Central Medical Database");
        menu.base.set_sub_title("Main Menu");
        menu.add_option("View Records", "", 3, 10, -1);
        menu.add_option("View Logs", "", 4, 3, -1);
        comp.add_computer_screen(ComputerScreen::MenuScreen(menu), 2);

        // Screen 3					( Records )

        let mut rs = GenericScreen::new();
        rs.set_screen_type(SCREEN_RECORDSCREEN);
        rs.base.set_main_title("Central Medical Database");
        rs.base.set_sub_title("Medical Records");
        rs.set_next_page(2);
        comp.add_computer_screen(ComputerScreen::GenericScreen(rs), 3);

        // Screen 4					( Logs )

        let mut ls = LogScreen::new();
        ls.set_target(LOGSCREEN_TARGET_ACCESSLOGS);
        ls.base.set_main_title("Central Medical Database");
        ls.base.set_sub_title("Access logs");
        ls.set_next_page(2);
        comp.add_computer_screen(ComputerScreen::LogScreen(ls), 4);

        // Add in the admin account

        let mut record = Record::new();
        record.add_field(RECORDBANK_NAME, RECORDBANK_ADMIN);
        record.add_field("Handle", RECORDBANK_ADMIN);
        record.add_field(
            RECORDBANK_PASSWORD,
            &NameGenerator::generate_complex_password(g),
        );
        record.add_field(RECORDBANK_SECURITY, "1");
        comp.recordbank.add_record(record);

        g.world_mut().create_computer_from(comp);
    }

    pub fn generate_global_intelligence_agency(g: &mut Ctx) {
        // C++ body is empty
        let _ = g;
    }

    pub fn generate_international_academic_database(g: &mut Ctx) {
        // Create the VLocation

        let (x, y) = Self::generate_valid_map_pos();
        g.world_mut().create_vlocation(IP_ACADEMICDATABASE, x, y);

        // Create the computer

        let mut comp = Computer::new();
        comp.set_type(COMPUTER_TYPE_UNKNOWN);
        comp.set_name("International Academic Database");
        comp.set_company_name("Government");
        comp.set_ip(IP_ACADEMICDATABASE);
        comp.set_trace_speed(TRACESPEED_INTERNATIONALACADEMICDATABASE);
        comp.set_trace_action(COMPUTER_TRACEACTION_DISCONNECT | COMPUTER_TRACEACTION_WARNINGMAIL);
        comp.set_is_targetable(false);
        comp.security.add_system_new(SECURITY_TYPE_MONITOR, 1, -1);

        // Screen 0					( Opening message screen )

        let mut ms = MessageScreen::new();
        ms.base.set_main_title("International Academic Database");
        ms.base
            .set_sub_title("Unauthorised Access will be punished");
        ms.set_text_message(concat!(
            "This is a Government owned computer system - all attempts at\n",
            "illegal access will be severely punished to the full extent of the law.\n\n",
            "Valid User Accounts\n",
            "====================\n\n",
            "admin    - Complete system wide access\n",
            "readwrite  - Read/Write access to all records\n",
            "readonly  - Read only access to all records\n",
        ));
        ms.set_button_message("Log in");
        ms.set_next_page(1);
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms), 0);

        // Screen 1					( User ID Screen )

        let mut uid = UserIDScreen::new();
        uid.base.set_main_title("International Academic Database");
        uid.base.set_sub_title("Log in");
        uid.set_difficulty(HACKDIFFICULTY_INTERNATIONALACADEMICDATABASE);
        uid.set_next_page(2);
        comp.add_computer_screen(ComputerScreen::UserIDScreen(uid), 1);

        // Screen 2					( Main Menu )

        let mut menu = MenuScreen::new();
        menu.base.set_main_title("International Academic Database");
        menu.base.set_sub_title("Main Menu");
        menu.add_option("Search Records", "", 5, 3, -1);
        if globals::TESTGAME {
            menu.add_option("View Records", "", 3, 10, -1);
        }
        menu.add_option("View Logs", "", 4, 1, -1);
        comp.add_computer_screen(ComputerScreen::MenuScreen(menu), 2);

        // Screen 3					( Records )

        let mut rs = GenericScreen::new();
        rs.set_screen_type(SCREEN_RECORDSCREEN);
        rs.base.set_main_title("International Academic Database");
        rs.base.set_sub_title("Academic Records");
        rs.set_next_page(2);
        comp.add_computer_screen(ComputerScreen::GenericScreen(rs), 3);

        // Screen 4					( Logs )

        let mut ls = LogScreen::new();
        ls.set_target(LOGSCREEN_TARGET_ACCESSLOGS);
        ls.base.set_main_title("International Academic Database");
        ls.base.set_sub_title("Access logs");
        ls.set_next_page(2);
        comp.add_computer_screen(ComputerScreen::LogScreen(ls), 4);

        // Screen 5					( Search )

        let mut ds5 = DialogScreen::new();
        ds5.base.set_main_title("International Academic Database");
        ds5.base.set_sub_title("Search for Academic record");
        ds5.add_widget(
            "name",
            WIDGET_TEXTBOX,
            150,
            200,
            200,
            15,
            "Enter name here",
            "Type the name of the person to search for here",
        );
        ds5.add_widget_with_data(
            "OK",
            WIDGET_SCRIPTBUTTON,
            150,
            240,
            80,
            15,
            "OK",
            "Click here when done",
            16,
            -1,
            None,
            None,
        );
        ds5.add_widget_with_data(
            "Cancel",
            WIDGET_NEXTPAGE,
            270,
            240,
            80,
            15,
            "Cancel",
            "Click here to return to the menu",
            2,
            -1,
            None,
            None,
        );

        ds5.add_widget("border1", WIDGET_BASIC, 120, 170, 260, 4, "", "");
        ds5.add_widget("border2", WIDGET_BASIC, 120, 266, 260, 4, "", "");
        ds5.add_widget("border3", WIDGET_BASIC, 120, 170, 4, 100, "", "");
        ds5.add_widget("border4", WIDGET_BASIC, 380, 170, 4, 100, "", "");

        ds5.set_return_key_button("OK");
        ds5.set_escape_key_button("Cancel");

        comp.add_computer_screen(ComputerScreen::DialogScreen(ds5), 5);

        // Screen 6					( View record )

        let mut gs6 = GenericScreen::new();
        gs6.base.set_main_title("International Academic Database");
        gs6.base.set_sub_title("View record");
        gs6.set_next_page(2);
        gs6.set_screen_type(SCREEN_ACADEMICSCREEN);
        comp.add_computer_screen(ComputerScreen::GenericScreen(gs6), 6);

        // Add in the accounts

        let mut record = Record::new();
        record.add_field(RECORDBANK_NAME, RECORDBANK_ADMIN);
        record.add_field(RECORDBANK_PASSWORD, &NameGenerator::generate_password(g));
        record.add_field(RECORDBANK_SECURITY, "1");
        comp.recordbank.add_record(record);

        let mut record2 = Record::new();
        record2.add_field(RECORDBANK_NAME, RECORDBANK_READWRITE);
        record2.add_field(RECORDBANK_PASSWORD, &NameGenerator::generate_password(g));
        record2.add_field(RECORDBANK_SECURITY, "2");
        comp.recordbank.add_record(record2);

        let mut record3 = Record::new();
        record3.add_field(RECORDBANK_NAME, RECORDBANK_READONLY);
        record3.add_field(RECORDBANK_PASSWORD, &NameGenerator::generate_password(g));
        record3.add_field(RECORDBANK_SECURITY, "4");
        comp.recordbank.add_record(record3);

        g.world_mut().create_computer_from(comp);
    }

    pub fn generate_inter_nic(g: &mut Ctx) {
        // Create the VLocation

        let (x, y) = Self::generate_valid_map_pos();
        g.world_mut().create_vlocation(IP_INTERNIC, x, y);

        // Create the computer

        let mut comp = Computer::new();
        comp.set_type(COMPUTER_TYPE_UNKNOWN);
        comp.set_name("InterNIC");
        comp.set_company_name("Government");
        comp.set_ip(IP_INTERNIC);
        comp.set_trace_speed(TRACESPEED_INTERNIC);
        comp.set_trace_action(COMPUTER_TRACEACTION_DISCONNECT | COMPUTER_TRACEACTION_WARNINGMAIL);
        comp.set_is_targetable(false);

        // Screen 0						-			Intro screen

        let mut ms0 = MessageScreen::new();
        ms0.base.set_main_title("InterNIC");
        ms0.base.set_sub_title("InterNet Information Center");
        ms0.set_next_page(1);
        ms0.set_text_message(concat!(
            "Welcome to the Internet Information Center.\n\n",
            "Use of this service is free and open to all.",
        ));
        ms0.set_button_message("OK");
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms0), 0);

        // Screen 1						-			Main menu

        let mut ms1 = MenuScreen::new();
        ms1.base.set_main_title("InterNIC");
        ms1.base.set_sub_title("Main menu");
        ms1.add_option(
            "Browse/Search",
            "Click here to use a list of all known IP's",
            2,
            10,
            -1,
        );
        ms1.add_option("Admin", "Click here to modify the database", 3, 10, -1);
        comp.add_computer_screen(ComputerScreen::MenuScreen(ms1), 1);

        // Screen 2						-			Full list

        let mut ls2 = LinksScreen::new();
        ls2.base.set_main_title("InterNIC");
        ls2.base.set_sub_title("Search list");
        ls2.set_screen_type(LINKSSCREENTYPE_ALLLINKS);
        ls2.set_next_page(1);
        comp.add_computer_screen(ComputerScreen::LinksScreen(ls2), 2);

        // Screen 3						-			Admin password

        let mut ps3 = PasswordScreen::new();
        ps3.base.set_main_title("InterNIC");
        ps3.base.set_sub_title("Password authorisation required");
        ps3.set_next_page(4);
        ps3.set_password(&NameGenerator::generate_password(g));
        ps3.set_difficulty(HACKDIFFICULTY_INTERNIC);
        comp.add_computer_screen(ComputerScreen::PasswordScreen(ps3), 3);

        // Screen 4						-			Admin menu

        let mut ms4 = MenuScreen::new();
        ms4.base.set_main_title("InterNIC");
        ms4.base.set_sub_title("Admin menu");
        ms4.add_option("Access Logs", "Click here to view access logs", 5, 10, -1);
        ms4.add_option("Exit", "Return to the main menu", 1, 10, -1);
        comp.add_computer_screen(ComputerScreen::MenuScreen(ms4), 4);

        // Screen 5						-			Logs

        let mut ls5 = LogScreen::new();
        ls5.base.set_main_title("InterNIC");
        ls5.base.set_sub_title("Log Screen");
        ls5.set_target(LOGSCREEN_TARGET_ACCESSLOGS);
        ls5.set_next_page(4);
        comp.add_computer_screen(ComputerScreen::LogScreen(ls5), 5);

        g.world_mut().create_computer_from(comp);
    }

    pub fn generate_stock_market(g: &mut Ctx) {
        // Create the VLocation

        let (x, y) = Self::generate_valid_map_pos();
        g.world_mut().create_vlocation(IP_STOCKMARKETSYSTEM, x, y);

        // Create the computer

        let name = "International Stock Market System";
        let mut comp = Computer::new();
        comp.set_type(COMPUTER_TYPE_UNKNOWN);
        comp.set_name(name);
        comp.set_company_name("Government");
        comp.set_ip(IP_STOCKMARKETSYSTEM);
        comp.set_trace_speed(TRACESPEED_STOCKMARKET);
        comp.set_trace_action(COMPUTER_TRACEACTION_DISCONNECT | COMPUTER_TRACEACTION_LEGAL);
        comp.set_is_targetable(false);
        comp.security.add_system_new(SECURITY_TYPE_PROXY, 1, -1);
        comp.security.add_system_new(SECURITY_TYPE_MONITOR, 1, -1);

        // Screen 0									( Intro )

        let mut txtmsg = String::from(concat!(
            "Welcome to the International Stock Market trading System.\n",
            "This system is open to all, however you will need an access ",
            "account if you wish to trade in shares.\n\n",
            "Select 'new user' from the menu to create a new account.\n",
            "Log in as 'guest', password 'guest' to browse share prices.\n",
            "Otherwise, use your provided log in name and password.\n\n",
        ));
        if globals::DEMOGAME {
            txtmsg.push_str("Demo Uplink clients will not be permitted to trade shares.");
        }

        let mut ms0 = MessageScreen::new();
        ms0.base.set_main_title(name);
        ms0.base.set_sub_title("Welcome");
        ms0.set_text_message(&txtmsg);
        ms0.set_button_message("OK");
        ms0.set_next_page(1);
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms0), 0);

        // Screen 1									( Main Menu )

        let mut ms1 = MenuScreen::new();
        ms1.base.set_main_title(name);
        ms1.base.set_sub_title("Main Menu");
        ms1.add_option(
            "Log in",
            "Log in using an existing username and password",
            4,
            10,
            -1,
        );

        if !globals::DEMOGAME {
            ms1.add_option(
                "New User",
                "Create a new user account to trade in shares",
                2,
                10,
                -1,
            );
        }

        ms1.add_option(
            "Information",
            "Click for help information about this computer",
            0,
            10,
            -1,
        );
        comp.add_computer_screen(ComputerScreen::MenuScreen(ms1), 1);

        // Screen 2									( New user )

        let mut ds2 = DialogScreen::new();
        ds2.base.set_main_title(name);
        ds2.base.set_sub_title("New User");

        ds2.add_widget(
            "name",
            WIDGET_BASIC,
            80,
            180,
            170,
            20,
            "Enter your name",
            "This is your name",
        );
        ds2.add_widget(
            "password",
            WIDGET_BASIC,
            80,
            210,
            170,
            20,
            "Enter your password",
            "You'll need this to access your account",
        );
        ds2.add_widget(
            "password2",
            WIDGET_BASIC,
            80,
            240,
            170,
            20,
            "Re-type your password",
            "For verification purposes",
        );

        ds2.add_widget(
            "nametext",
            WIDGET_TEXTBOX,
            270,
            180,
            170,
            20,
            "Fill this in",
            "Enter your name here",
        );
        ds2.add_widget(
            "passwordtext",
            WIDGET_PASSWORDBOX,
            270,
            210,
            170,
            20,
            "",
            "Enter your password here",
        );
        ds2.add_widget(
            "passwordtext2",
            WIDGET_PASSWORDBOX,
            270,
            240,
            170,
            20,
            "",
            "Re-type your password here",
        );

        ds2.add_widget_with_data(
            "create",
            WIDGET_SCRIPTBUTTON,
            250,
            330,
            80,
            20,
            "Create",
            "Create this account",
            12,
            -1,
            Some(""),
            Some(""),
        );
        ds2.add_widget_with_data(
            "cancel",
            WIDGET_NEXTPAGE,
            350,
            330,
            80,
            20,
            "Cancel",
            "Cancel",
            1,
            -1,
            Some(""),
            Some(""),
        );

        comp.add_computer_screen(ComputerScreen::DialogScreen(ds2), 2);

        // Screen 3									( New user created )

        let mut ms3 = MessageScreen::new();
        ms3.base.set_main_title(name);
        ms3.base.set_sub_title("New user created");
        ms3.set_text_message(concat!(
            "Your new user account has been created successfully.\n",
            "You are now authorised to trade shares on this system.",
        ));
        ms3.set_button_message("OK");
        ms3.set_next_page(5);
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms3), 3);

        // Screen 4									( Log in )

        let mut uid4 = UserIDScreen::new();
        uid4.base.set_main_title(name);
        uid4.base.set_sub_title("Enter username and password");
        uid4.set_difficulty(HACKDIFFICULTY_STOCKMARKET);
        uid4.set_next_page(5);
        comp.add_computer_screen(ComputerScreen::UserIDScreen(uid4), 4);

        // Screen 5									( User menu )

        let mut ms5 = MenuScreen::new();
        ms5.base.set_main_title(name);
        ms5.base.set_sub_title("User Menu");
        ms5.add_option(
            "Browse shares",
            "Browse a list of all shares available",
            7,
            10,
            -1,
        );
        ms5.add_option("Admin", "View the admin menu", 8, 1, -1);
        comp.add_computer_screen(ComputerScreen::MenuScreen(ms5), 5);

        // Screen 6									( View my portfolio )

        // Screen 7									( Browse shares )

        let mut sls7 = SharesListScreen::new();
        sls7.base.set_main_title(name);
        sls7.base.set_sub_title("Browse Shares");
        sls7.set_screen_type(SHARESLISTSCREENTYPE_ALLSHARES);
        sls7.set_next_page(5);
        sls7.set_view_page(11);
        comp.add_computer_screen(ComputerScreen::SharesListScreen(sls7), 7);

        // Screen 8									( Admin menu )

        let mut ms8 = MenuScreen::new();
        ms8.base.set_main_title(name);
        ms8.base.set_sub_title("Admin menu");
        if globals::TESTGAME {
            ms8.add_option("View records", "View all records on this system", 9, 10, -1);
        }
        ms8.add_option(
            "View logs",
            "View all access logs on this system",
            10,
            10,
            -1,
        );
        ms8.add_option("Done", "Return to the user menu", 5, 10, -1);
        comp.add_computer_screen(ComputerScreen::MenuScreen(ms8), 8);

        // Screen 9									( Records )

        let mut rs9 = GenericScreen::new();
        rs9.set_screen_type(SCREEN_RECORDSCREEN);
        rs9.base.set_main_title(name);
        rs9.base.set_sub_title("Records");
        rs9.set_next_page(8);
        comp.add_computer_screen(ComputerScreen::GenericScreen(rs9), 9);

        // Screen 10								( Logs )

        let mut ls10 = LogScreen::new();
        ls10.set_target(LOGSCREEN_TARGET_ACCESSLOGS);
        ls10.base.set_main_title(name);
        ls10.base.set_sub_title("Access logs");
        ls10.set_next_page(8);
        comp.add_computer_screen(ComputerScreen::LogScreen(ls10), 10);

        // Screen 11								( View a share )

        let mut gs11 = GenericScreen::new();
        gs11.set_screen_type(SCREEN_SHARESVIEWSCREEN);
        gs11.base.set_main_title(name);
        gs11.base.set_sub_title("View share details");
        gs11.set_next_page(7);
        comp.add_computer_screen(ComputerScreen::GenericScreen(gs11), 11);

        // Create some logins

        let mut record = Record::new();
        record.add_field(RECORDBANK_NAME, RECORDBANK_ADMIN);
        record.add_field(RECORDBANK_PASSWORD, &NameGenerator::generate_password(g));
        record.add_field(RECORDBANK_SECURITY, "1");
        comp.recordbank.add_record(record);

        let mut record2 = Record::new();
        record2.add_field(RECORDBANK_NAME, "guest");
        record2.add_field(RECORDBANK_PASSWORD, "guest");
        record2.add_field(RECORDBANK_SECURITY, "10");
        comp.recordbank.add_record(record2);

        g.world_mut().create_computer_from(comp);
    }

    pub fn generate_proto_vision(g: &mut Ctx) {
        // Create the VLocation

        let (x, y) = Self::generate_valid_map_pos();
        g.world_mut().create_vlocation(IP_PROTOVISION, x, y);

        // Create the company

        let company_name =
            Self::generate_company_named(g, "Protovision", 20, COMPANYTYPE_COMMERCIAL, 10, 0);
        g.world_mut()
            .get_company_mut(&company_name)
            .expect("Protovision company was just generated")
            .set_boss("Dr. Falken");

        // Create the computer

        let mut comp = Computer::new();
        comp.set_type(COMPUTER_TYPE_UNKNOWN);
        comp.set_name("Protovision Game Server");
        comp.set_company_name("Protovision");
        comp.set_ip(IP_PROTOVISION);
        comp.set_trace_speed(TRACESPEED_PROTOVISION);
        comp.set_trace_action(COMPUTER_TRACEACTION_DISCONNECT | COMPUTER_TRACEACTION_LEGAL);
        comp.set_is_targetable(false);

        // Screen 0					Password ID

        let mut ps0 = PasswordScreen::new();
        ps0.base.set_main_title("Protovision");
        ps0.base.set_sub_title("Game Server");
        ps0.set_difficulty(HACKDIFFICULTY_PROTOVISION);
        ps0.set_password("joshua");
        ps0.set_next_page(1);
        comp.add_computer_screen(ComputerScreen::PasswordScreen(ps0), -1);

        // Screen 1					Main menu

        let mut ms1 = MenuScreen::new();
        ms1.base.set_main_title("Protovision");
        ms1.base.set_sub_title("Main Menu");
        ms1.add_option("WarGames", " ", 2, 1, -1);
        ms1.add_option("Hints", " ", 3, 1, -1);
        comp.add_computer_screen(ComputerScreen::MenuScreen(ms1), -1);

        // Screen 2					WarGames

        let mut ms2 = GenericScreen::new();
        ms2.set_screen_type(SCREEN_PROTOVISION);
        ms2.base.set_main_title("Protovision");
        ms2.base.set_sub_title("WarGames");
        ms2.set_next_page(1);
        comp.add_computer_screen(ComputerScreen::GenericScreen(ms2), -1);

        // Screen 3					New Games

        let mut ms3 = MessageScreen::new();
        ms3.base.set_main_title("Protovision");
        ms3.base.set_sub_title("Hints");
        ms3.set_text_message(concat!(
            "Getting warmmmmm ;)\n",
            "What was the IP of Introversion Software again?",
        ));
        ms3.set_button_message("OK");
        ms3.set_next_page(1);
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms3), -1);

        // Screen 4                 Global Thermonuclear War

        let mut gs4 = GenericScreen::new();
        gs4.set_screen_type(SCREEN_NUCLEARWAR);
        gs4.base.set_main_title("Protovision");
        gs4.base.set_sub_title("Global Thermonuclear War");
        gs4.set_next_page(1);
        comp.add_computer_screen(ComputerScreen::GenericScreen(gs4), -1);

        g.world_mut().create_computer_from(comp);
    }

    pub fn generate_ocp(g: &mut Ctx) {
        // Create the VLocation

        let (x, y) = Self::generate_valid_map_pos();
        g.world_mut().create_vlocation(IP_OCP, x, y);

        // Create the company

        let mut company = Company::new();
        company.set_name("OCP");
        company.set_size(20);
        company.set_type(COMPANYTYPE_COMMERCIAL);
        company.set_growth(10);
        company.set_alignment(-30);
        g.world_mut().create_company_from(company);
        Company::grow(g, "OCP", 0);

        // Create the computer

        let mut comp = Computer::new();
        comp.set_type(COMPUTER_TYPE_UNKNOWN);
        comp.set_name("OCP remote monitoring system");
        comp.set_company_name("OCP");
        comp.set_ip(IP_OCP);
        comp.set_is_targetable(false);

        // Screen 1					Message

        let mut ms0 = MessageScreen::new();
        ms0.base.set_main_title("OCP");
        ms0.base.set_sub_title("Remote Monitoring System");
        ms0.set_text_message(concat!(
            "\n\n\n\n\n\n\nServe the public trust\n",
            "Protect the innocent\n",
            "Uphold the law\n",
            "(Classified)",
        ));

        comp.add_computer_screen(ComputerScreen::MessageScreen(ms0), -1);

        g.world_mut().create_computer_from(comp);

        // Generate contact addresses for this company

        generate_internal_contact(g, "OCP remote monitoring system", "internal@OCP.net");
    }

    pub fn generate_sj_games(g: &mut Ctx) {
        // Create the VLocation

        let (x, y) = Self::generate_valid_map_pos();
        g.world_mut().create_vlocation(IP_SJGAMES, x, y);

        // Create the company

        let mut company = Company::new();
        company.set_name("Steve Jackson Games");
        company.set_size(3);
        company.set_type(COMPANYTYPE_COMMERCIAL);
        company.set_growth(1);
        company.set_alignment(0);
        g.world_mut().create_company_from(company);
        Company::grow(g, "Steve Jackson Games", 0);

        // Create the computer

        let mut comp = Computer::new();
        comp.set_type(COMPUTER_TYPE_UNKNOWN);
        comp.set_name("Steve Jackson Games Server");
        comp.set_company_name("Steve Jackson Games");
        comp.set_ip(IP_SJGAMES);
        comp.set_is_targetable(false);

        // Screen 1					Message

        let mut ms0 = MessageScreen::new();
        ms0.base.set_main_title("Steve Jackson Games");
        ms0.base.set_sub_title("Public Access Server");
        ms0.set_text_message(concat!(
            "ATTENTION\n\n",
            "This computer system has been seized\n",
            "by the United States Secret Service\n",
            "in the interests of National Security.\n\n\n",
            "Your IP has been logged.",
        ));

        comp.add_computer_screen(ComputerScreen::MessageScreen(ms0), -1);

        g.world_mut().create_computer_from(comp);

        // Generate contact addresses for this company

        generate_internal_contact(
            g,
            "Steve Jackson Games Server",
            "internal@Steve Jackson Games.net",
        );
    }

    pub fn generate_introversion(g: &mut Ctx) {
        // Create the VLocation

        let (x, y) = Self::generate_valid_map_pos();
        g.world_mut()
            .create_vlocation(IP_INTROVERSION, x, y)
            .set_listed(false);

        // Create the company

        let mut company = Company::new();
        company.set_name("Introversion Software");
        company.set_size(10);
        company.set_type(COMPANYTYPE_COMMERCIAL);
        company.set_growth(50);
        company.set_alignment(0);
        g.world_mut().create_company_from(company);
        Company::grow(g, "Introversion Software", 0);

        // Create the computer

        let mut comp = Computer::new();
        comp.set_type(COMPUTER_TYPE_UNKNOWN);
        comp.set_name("Introversion Software");
        comp.set_company_name("Introversion Software");
        comp.set_ip(IP_INTROVERSION);
        comp.set_is_targetable(false);

        // Screen 0					Welcome Message

        let mut ms0 = MessageScreen::new();
        ms0.base.set_main_title("Introversion Software");
        ms0.base.set_sub_title("Internal Services");
        ms0.set_text_message(concat!(
            "Welcome, employee\n",
            "All design documents relating to the forthcoming Introversion title\n",
            "codenamed 'Uplink2: TERMINAL' are to be copied to this site\n",
            "for easy access by all employees.\n\n",
            "All data on this site is strictly confidential.",
        ));
        ms0.set_button_message("OK");
        ms0.set_next_page(1);
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms0), 0);

        // Screen 1                 Password

        let mut ps1 = PasswordScreen::new();
        ps1.base.set_main_title("Introversion Software");
        ps1.base.set_sub_title("Enter password");
        ps1.set_password(&NameGenerator::generate_complex_password(g));
        ps1.set_difficulty(HACKDIFFICULTY_UPLINK_TESTMACHINE);
        ps1.set_next_page(2);
        comp.add_computer_screen(ComputerScreen::PasswordScreen(ps1), 1);

        // Screen 2                 Menu

        let mut ms2 = MenuScreen::new();
        ms2.base.set_main_title("Introversion Software");
        ms2.base.set_sub_title("Project Uplink2 : TERMINAL");
        ms2.add_option("Introduction", "Introduction", 3, 10, -1);
        ms2.add_option("Uplink", "Uplink", 4, 10, -1);
        ms2.add_option("Mission: Impossible", "Mission: Impossible", 5, 10, -1);
        ms2.add_option("Johnny Mnemonic", "Johnny Mnemonic", 6, 10, -1);
        ms2.add_option("The Holy Grail", "The Holy Grail", 8, 10, -1);
        ms2.add_option("Other", "Other", 7, 10, -1);
        comp.add_computer_screen(ComputerScreen::MenuScreen(ms2), 2);

        // Screen 3                 Introduction

        let mut ms3 = MessageScreen::new();
        ms3.base.set_main_title("Introversion Software");
        ms3.base.set_sub_title("Introduction");
        ms3.set_text_message(concat!(
            "The aim of this site is to serve as a repository for ",
            "all ideas and documents relating to the design of the next ",
            "Uplink product, to be released by Introversion Software.\n\n",
            "Currently nothing is finalised, other than the sequel to Uplink ",
            "must be a hacking tool of a similar nature to Uplink itself.\n\n",
            "If any employees have any good ideas, please email them to the ",
            "company designer.  Please use his real-world address, as his ",
            "Uplink email address is under almost constant surveillance from ",
            "federal agents and hackers, and any good ideas will no doubt be stolen.\n\n",
            "Contact Chris at chris@introversion.co.uk",
        ));
        ms3.set_button_message("OK");
        ms3.set_next_page(2);
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms3), 3);

        // Screen 4                 Uplink

        let mut ms4 = MessageScreen::new();
        ms4.base.set_main_title("Introversion Software");
        ms4.base.set_sub_title("Uplink");
        ms4.set_text_message(concat!(
            "At some point I would like one of our members to perform a ",
            "postmortem on Uplink itself, and come up with a list of possible improvements.\n\n",
            "<todo>",
        ));
        ms4.set_button_message("OK");
        ms4.set_next_page(2);
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms4), 4);

        // Screen 5                  Mission: Impossible

        let mut ms5 = MessageScreen::new();
        ms5.base.set_main_title("Introversion Software");
        ms5.base.set_sub_title("Mission: Impossible");
        ms5.set_text_message(concat!(
            "One possible direction for Uplink to go is shown in the film Mission:Impossible, ",
            "starring Tom Cruise when he was a much younger actor and long before his terrible accident.\n\n",
            "This film came out in the late 20th century, but the basic idea was that a squad of real people ",
            "would enter the target site (eg a bank, corporation HQ etc) and they would be assisted by ",
            "a tech guy on the outside.  The tech guy would be running Uplink2, and he would be able to see ",
            "the agents moving around the target site.  He would have voice control over his squad.  He would ",
            "need to open doors, block elevator shafts, override building security, distract guards etc.\n\n",
            "Success would attract agents with better skills who could rob more advanced buildings and sites. ",
            "Failure would lead to the arrest of the Agents under the guys command.",
        ));
        ms5.set_button_message("OK");
        ms5.set_next_page(2);
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms5), 5);

        // Screen 6                 Johnny Mnemonic

        let mut ms6 = MessageScreen::new();
        ms6.base.set_main_title("Introversion Software");
        ms6.base.set_sub_title("Johnny Mnemonic");
        ms6.set_text_message(concat!(
            "Around 15 years ago a film was released in what was then called Hollywood, called Johnny Mnemonic. ",
            "The film was almost universally hated, however it did contain some fantastic visulisations of the ",
            "Internet of 2010.  It is frequently hilarious to see how inaccurate those predictions were.\n\n",
            "Nevertheless, Uplink2 would look fantastic with a similar look. The Internet would be rendered in 3D, ",
            "with major corporate computer systems appearing as huge 3D sky-scrapers.  Every tool and action would ",
            "be extremely visual.\n\n",
            "It is thought that this feel would give Uplink2 a broader appeal, but would alienate the hard-core Uplink ",
            "player who values its (semi) realism. In addition, it would require a team of artists to produce the 3d models.",
        ));
        ms6.set_button_message("OK");
        ms6.set_next_page(2);
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms6), 6);

        // Screen 7                 Other

        let mut ms7 = MessageScreen::new();
        ms7.base.set_main_title("Introversion Software");
        ms7.base.set_sub_title("Other ideas");
        ms7.set_text_message(concat!(
            "Assasination mission. The target is flying on a chartered flight. Agent has to hack into airport ",
            "system and find out which flight the target is on. Then hack into air-traffic control and cause ",
            "the plane to crash.\n\n",
            "Power grids. A terrorist group is planning to attack a bank, and will cut you in for 10 percent ",
            "if you can cut the power to the bank vault at just the right moment.\n\n",
            "Remote control security drone.  The agent takes control of a security drone, viewing the world ",
            "through a black-and-white CCTV camera in the room. \n\n",
            "The agent takes control of a government owned satellite weapon, and uses it to destroy a target.",
        ));
        ms7.set_button_message("OK ");
        ms7.set_next_page(2);
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms7), 7);

        // Screen 8                 The Holy Grail

        let mut ms8 = MessageScreen::new();
        ms8.base.set_main_title("Introversion Software");
        ms8.base.set_sub_title("The Holy Grail");
        ms8.set_text_message(concat!(
            "The founder of Introversion Software believes the definitive version of Uplink could only be achieved ",
            "as an online massively-muliplayer universe.  When a player launches Uplink2, it would connect to a global ",
            "network of Uplink2 clients.  Each agent would be able to attack other Agent computer systems and steal ",
            "things from them.  A number of 'core' systems would be run by Introversion Software, including most of the ",
            "Government systems.  Agents would need to work in teams to properly hack the well guarded systems.\n\n",
            "Agents would also need to defend their own systems from hackers.  In effect, it would be similar in design ",
            "to Napster - everyone can see everyone else on the Uplink Network.\n\n",
            "It is not clear what the legal situation would be - could anyone actually be prosecuted under this system?",
        ));
        ms8.set_button_message("Dream on");
        ms8.set_next_page(2);
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms8), 8);

        g.world_mut().create_computer_from(comp);

        // Generate contact addresses for this company

        generate_internal_contact(
            g,
            "Introversion Software",
            "internal@Introversion Software.net",
        );
    }

    pub fn generate_simple_starting_mission_a(g: &mut Ctx) {
        // Generate two very easy missions for the player at the start
        let (employer1, target1) = Self::pick_starting_employer_and_target(g);
        if let Some(mission1) =
            MissionGenerator::generate_steal_single_file(g, &employer1, &target1)
        {
            Self::set_min_rating_on_posted(g, &mission1, 2);
        }
    }

    pub fn generate_simple_starting_mission_b(g: &mut Ctx) {
        // Generate a delete file mission
        let (employer2, target2) = Self::pick_starting_employer_and_target(g);
        if let Some(mission2) =
            MissionGenerator::generate_destroy_single_file(g, &employer2, &target2)
        {
            Self::set_min_rating_on_posted(g, &mission2, 2);
        }
    }

    // Low level functions for generating random items of data
    // They all add the data into the World object automatically, and then return its key

    /// Returns the new location's ip.
    pub fn generate_location(g: &mut Ctx) -> String {
        let (x, y) = Self::generate_valid_map_pos();

        let a = NumberGenerator::random_number(1000);
        let b = NumberGenerator::random_number(1000);
        let c = NumberGenerator::random_number(1000);
        let d = NumberGenerator::random_number(1000);
        let ip = format!("{}.{}.{}.{}", a, b, c, d);

        g.world_mut().create_vlocation(&ip, x, y);
        ip
    }

    /// Returns the new company's name.
    pub fn generate_company(g: &mut Ctx) -> String {
        let companyname = NameGenerator::generate_company_name();

        let size = random_normal_int(COMPANYSIZE_AVERAGE as f32, COMPANYSIZE_RANGE as f32);
        let growth = random_normal_int(COMPANYGROWTH_AVERAGE as f32, COMPANYGROWTH_RANGE as f32);
        let alignment = random_normal_int(
            COMPANYALIGNMENT_AVERAGE as f32,
            COMPANYALIGNMENT_RANGE as f32,
        );

        Self::generate_company_named(
            g,
            &companyname,
            size,
            COMPANYTYPE_COMMERCIAL,
            growth,
            alignment,
        )
    }

    pub fn generate_company_bank(g: &mut Ctx) -> String {
        let companyname = NameGenerator::generate_bank_name();

        let size = random_normal_int(4.0, 4.0);
        let growth = random_normal_int(10.0, 20.0);
        let alignment = random_normal_int(0.0, 50.0);

        let name = Self::generate_company_named(
            g,
            &companyname,
            size,
            COMPANYTYPE_FINANCIAL,
            growth,
            alignment,
        );

        Self::generate_public_bank_server(g, &name);

        name
    }

    /// `GenerateComputer ( char *companyname )`: an access terminal. Returns the computer's name.
    pub fn generate_computer(g: &mut Ctx, companyname: &str) -> String {
        let computername = NameGenerator::generate_access_terminal_name(companyname);

        let vl_ip = Self::generate_location(g);

        let mut comp = Computer::new();
        comp.set_name(&computername);
        comp.set_company_name(companyname);
        comp.set_trace_speed(20);
        comp.set_trace_action(COMPUTER_TRACEACTION_DISCONNECT);
        comp.set_is_targetable(false);
        comp.set_ip(&vl_ip);

        let mut ms = MessageScreen::new();
        ms.base.set_main_title(companyname);
        ms.base.set_sub_title("Access Terminal");
        ms.set_text_message("No public log-ons permitted");
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms), -1);

        g.world_mut().create_computer_from(comp);

        computername
    }

    /// Returns the new person's name.
    pub fn generate_person(g: &mut Ctx) -> String {
        let name = NameGenerator::generate_person_name();

        let comp_name = Self::generate_personal_computer(g, &name);
        let comp_ip = g
            .world()
            .get_computer(&comp_name)
            .expect("personal computer was just generated")
            .ip
            .clone();

        let phone_name = Self::generate_voice_phone_system(g, &name);
        let phone_ip = g
            .world()
            .get_computer(&phone_name)
            .expect("phone system was just generated")
            .ip
            .clone();

        let age = random_normal_int(30.0, 20.0); // 10 - 50
        let photo = NumberGenerator::random_number(NUM_STARTING_PHOTOS + 1);

        let mut voice = 1;
        if !DEMOGAME {
            voice = NumberGenerator::random_number(NUM_STARTING_VOICES) + 1;
        }

        let mut person = Person::new();
        person.set_name(&name);
        person.set_age(age);
        person.set_photo_index(photo);
        person.set_voice_index(voice);
        person.set_local_host(&comp_ip);
        person.set_remote_host(&comp_ip);
        person.set_phone_number(&phone_ip);

        g.world_mut().create_person_from(person);

        Connection::reset(g, &name);

        // Create some records for this person, spread around the world-wide databases

        RecordGenerator::generate_records(g, &name);

        name
    }

    /// Returns the new agent's (person's) name.
    pub fn generate_agent(g: &mut Ctx) -> String {
        let name = NameGenerator::generate_person_name();
        let handle = NameGenerator::generate_agent_alias();

        let comp_name = Self::generate_personal_computer(g, &name);
        let comp_ip = g
            .world()
            .get_computer(&comp_name)
            .expect("personal computer was just generated")
            .ip
            .clone();

        let phone_name = Self::generate_voice_phone_system(g, &name);
        let phone_ip = g
            .world()
            .get_computer(&phone_name)
            .expect("phone system was just generated")
            .ip
            .clone();

        let age = random_normal_int(30.0, 10.0); // 20 - 40
        let photo = NumberGenerator::random_number(NUM_STARTING_PHOTOS + 1);

        let mut voice = 1;
        if !DEMOGAME {
            voice = NumberGenerator::random_number(NUM_STARTING_VOICES) + 1;
        }

        let mut person = Person::new_agent();
        person.set_name(&name);
        person.set_handle(&handle);
        person.set_age(age);
        person.set_photo_index(photo);
        person.set_voice_index(voice);
        person.set_local_host(&comp_ip);
        person.set_remote_host(&comp_ip);
        person.set_phone_number(&phone_ip);
        person.rating.set_uplink_rating(random_normal_int(
            AGENT_UPLINKRATINGAVERAGE as f32,
            AGENT_UPLINKRATINGVARIANCE as f32,
        ));

        g.world_mut().create_person_from(person);

        // Create some records for this person, spread around the world-wide databases

        RecordGenerator::generate_records(g, &name);

        name
    }

    /// `GenerateComputer ( char *companyName, int TYPE )`: `None` if TYPE matches no generator.
    pub fn generate_computer_of_type(
        g: &mut Ctx,
        company_name: &str,
        type_: i32,
    ) -> Option<String> {
        if (type_ & COMPUTER_TYPE_PUBLICACCESSSERVER) != 0 {
            Some(Self::generate_public_access_server(g, company_name))
        } else if (type_ & COMPUTER_TYPE_INTERNALSERVICESMACHINE) != 0 {
            Some(Self::generate_internal_services_machine(g, company_name))
        } else if (type_ & COMPUTER_TYPE_CENTRALMAINFRAME) != 0 {
            Some(Self::generate_central_mainframe(g, company_name))
        } else if (type_ & COMPUTER_TYPE_PUBLICBANKSERVER) != 0 {
            Some(Self::generate_public_bank_server(g, company_name))
        } else if (type_ & COMPUTER_TYPE_PERSONALCOMPUTER) != 0 {
            Some(Self::generate_personal_computer(g, company_name))
        } else if (type_ & COMPUTER_TYPE_VOICEPHONESYSTEM) != 0 {
            Some(Self::generate_voice_phone_system(g, company_name))
        } else if (type_ & COMPUTER_TYPE_LAN) != 0 {
            Self::generate_lan(g, company_name)
        } else {
            None
        }
    }

    pub fn generate_public_access_server(g: &mut Ctx, companyname: &str) -> String {
        let computername = NameGenerator::generate_public_access_server_name(companyname);

        let vl_ip = Self::generate_location(g);

        let mut comp = Computer::new();
        comp.set_type(COMPUTER_TYPE_PUBLICACCESSSERVER);
        comp.set_name(&computername);
        comp.set_company_name(companyname);
        comp.set_trace_speed(random_normal_int(
            TRACESPEED_PUBLICACCESSSERVER as f32,
            TRACESPEED_PUBLICACCESSSERVER as f32 * TRACESPEED_VARIANCE,
        ));
        comp.set_ip(&vl_ip);

        let mut ms = MessageScreen::new();
        ms.base.set_main_title(companyname);
        ms.base.set_sub_title("Public Access Server");
        ms.set_text_message(&computername);
        ms.set_button_message("OK");
        ms.set_next_page(1);
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms), 0);

        let mut gs = GenericScreen::new();
        gs.base.set_main_title(companyname);
        gs.base.set_sub_title("Company data");
        gs.set_screen_type(SCREEN_COMPANYINFO);
        comp.add_computer_screen(ComputerScreen::GenericScreen(gs), 1);

        g.world_mut().create_computer_from(comp);

        computername
    }

    pub fn generate_internal_services_machine(g: &mut Ctx, companyname: &str) -> String {
        let computername = NameGenerator::generate_internal_services_server_name(companyname);
        let company_size = g
            .world()
            .get_company(companyname)
            .expect("company must exist")
            .size;

        // Create the location

        let vl_ip = Self::generate_location(g);

        // Create the computer

        let mut comp = Computer::new();
        comp.set_type(COMPUTER_TYPE_INTERNALSERVICESMACHINE);
        comp.set_name(&computername);
        comp.set_company_name(companyname);
        comp.set_trace_speed(random_normal_int(
            TRACESPEED_INTERNALSERVICESMACHINE as f32,
            TRACESPEED_INTERNALSERVICESMACHINE as f32 * TRACESPEED_VARIANCE,
        ));
        comp.set_ip(&vl_ip);

        if company_size > MINCOMPANYSIZE_MONITOR {
            let monitorstrength = ((company_size - MINCOMPANYSIZE_MONITOR) / 3).clamp(1, 5);
            comp.security
                .add_system_new(SECURITY_TYPE_MONITOR, monitorstrength, -1);
        }

        if company_size > MINCOMPANYSIZE_PROXY {
            let proxystrength = ((company_size - MINCOMPANYSIZE_PROXY) / 3).clamp(1, 5);
            comp.security
                .add_system_new(SECURITY_TYPE_PROXY, proxystrength, -1);
        }

        if company_size > MINCOMPANYSIZE_FIREWALL {
            let firewallstrength = ((company_size - MINCOMPANYSIZE_FIREWALL) / 3).clamp(1, 5);
            comp.security
                .add_system_new(SECURITY_TYPE_FIREWALL, firewallstrength, -1);
        }

        if company_size < MAXCOMPANYSIZE_WARNINGMAIL {
            comp.set_trace_action(
                COMPUTER_TRACEACTION_DISCONNECT | COMPUTER_TRACEACTION_WARNINGMAIL,
            );
        } else if company_size < MAXCOMPANYSIZE_FINE {
            comp.set_trace_action(COMPUTER_TRACEACTION_DISCONNECT | COMPUTER_TRACEACTION_FINE);
        } else {
            comp.set_trace_action(COMPUTER_TRACEACTION_DISCONNECT | COMPUTER_TRACEACTION_LEGAL);
        }

        if companyname == "Government" {
            comp.set_is_targetable(false);
        }

        // Create the screens

        let mut uid = UserIDScreen::new();
        uid.base.set_main_title(companyname);
        uid.base.set_sub_title("Log in");
        uid.set_difficulty(random_normal_int(
            HACKDIFFICULTY_INTERNALSERVICESMACHINE as f32,
            HACKDIFFICULTY_INTERNALSERVICESMACHINE as f32 * HACKDIFFICULTY_VARIANCE,
        ));
        uid.set_next_page(1);
        comp.add_computer_screen(ComputerScreen::UserIDScreen(uid), 0);

        let mut ms = MenuScreen::new();
        ms.base.set_main_title(companyname);
        ms.base.set_sub_title("Internal Services Main Menu");
        ms.add_option("File Server", "Access the file server", 2, 10, -1);
        ms.add_option(
            "View records",
            "View the records stored on this system",
            4,
            10,
            -1,
        );
        ms.add_option(
            "View links",
            "View all links available on this system",
            6,
            10,
            -1,
        );
        ms.add_option("Admin", "Enter administrative mode", 7, 1, -1);
        comp.add_computer_screen(ComputerScreen::MenuScreen(ms), 1);

        let mut fss = GenericScreen::new();
        fss.set_screen_type(SCREEN_FILESERVERSCREEN);
        fss.base.set_main_title(companyname);
        fss.base.set_sub_title("File server");
        fss.set_next_page(1);
        comp.add_computer_screen(ComputerScreen::GenericScreen(fss), 2);

        let mut ls = LogScreen::new();
        ls.set_target(LOGSCREEN_TARGET_ACCESSLOGS);
        ls.base.set_main_title(companyname);
        ls.base.set_sub_title("Access Logs");
        ls.set_next_page(7);
        comp.add_computer_screen(ComputerScreen::LogScreen(ls), 3);

        let mut rs = GenericScreen::new();
        rs.set_screen_type(SCREEN_RECORDSCREEN);
        rs.base.set_main_title(companyname);
        rs.base.set_sub_title("Records");
        rs.set_next_page(1);
        comp.add_computer_screen(ComputerScreen::GenericScreen(rs), 4);

        let mut gs5 = GenericScreen::new();
        gs5.set_screen_type(SCREEN_SECURITYSCREEN);
        gs5.base.set_main_title(companyname);
        gs5.base.set_sub_title("Security");
        gs5.set_next_page(7);
        comp.add_computer_screen(ComputerScreen::GenericScreen(gs5), 5);

        let mut ls6 = LinksScreen::new();
        ls6.base.set_main_title(companyname);
        ls6.base.set_sub_title("Links");
        ls6.set_next_page(1);
        ls6.set_screen_type(LINKSSCREENTYPE_LOCALLINKS);
        comp.add_computer_screen(ComputerScreen::LinksScreen(ls6), 6);

        let mut ms7 = MenuScreen::new();
        ms7.base.set_main_title(companyname);
        ms7.base.set_sub_title("Admin menu");
        ms7.add_option("View logs", "View the access logs on this system", 3, 1, -1);
        ms7.add_option("Security", "Enable and disable security systems", 5, 1, -1);
        ms7.add_option("Console", "Run a console", 8, 1, -1);
        ms7.add_option("Exit", "Return to the main menu", 1, 10, -1);
        comp.add_computer_screen(ComputerScreen::MenuScreen(ms7), 7);

        let mut gs8 = GenericScreen::new();
        gs8.set_screen_type(SCREEN_CONSOLESCREEN);
        gs8.base.set_main_title(companyname);
        gs8.base.set_sub_title("Console");
        gs8.set_next_page(7);
        comp.add_computer_screen(ComputerScreen::GenericScreen(gs8), 8);

        // Create the files

        comp.databank.set_size(random_normal_int(100.0, 40.0));

        let numfiles = random_normal_int(10.0, 5.0);

        for _ in 0..numfiles {
            let type_ = NumberGenerator::random_number(2) + 1;
            let size = random_normal_int(6.0, 6.0);
            let encrypted = NumberGenerator::random_number(2);
            let compressed = NumberGenerator::random_number(2);
            let datatitle = NameGenerator::generate_data_name(companyname, type_);

            // PORT: the C++ evaluated these two conditional RandomNumber calls in unspecified
            // argument order; they are made left to right here.
            let encrypted_level = if encrypted != 0 {
                NumberGenerator::random_number(5)
            } else {
                0
            };
            let compressed_level = if compressed != 0 {
                NumberGenerator::random_number(5)
            } else {
                0
            };

            let mut data = Data::new();
            data.set_title(&datatitle);
            data.set_details(
                type_,
                size,
                encrypted_level,
                compressed_level,
                1.0,
                SOFTWARETYPE_NONE,
            );
            comp.databank.put_data(data);
        }

        // Create some logs

        let numlogs = NumberGenerator::random_number(10);

        for _ in 0..numlogs {
            let fromip = Self::get_random_location(g);

            let mut al = AccessLog::new();
            al.set_properties(
                &g.world().date,
                &fromip,
                " ",
                LOG_NOTSUSPICIOUS,
                LOG_TYPE_TEXT,
            );
            al.set_data1(Some("Accessed File"));
            comp.logbank.add_log(al, -1);
        }

        // Create some log-in's

        let mut record = Record::new();
        record.add_field(RECORDBANK_NAME, RECORDBANK_ADMIN);
        record.add_field(RECORDBANK_PASSWORD, &NameGenerator::generate_password(g));
        record.add_field(RECORDBANK_SECURITY, "1");
        comp.recordbank.add_record(record);

        g.world_mut().create_computer_from(comp);

        computername
    }

    pub fn generate_central_mainframe(g: &mut Ctx, companyname: &str) -> String {
        let computername = NameGenerator::generate_central_mainframe_name(companyname);
        let company_size = g
            .world()
            .get_company(companyname)
            .expect("company must exist")
            .size;

        let vl_ip = Self::generate_location(g);
        g.world_mut()
            .get_vlocation_mut(&vl_ip)
            .expect("location was just generated")
            .set_listed(false);

        let mut comp = Computer::new();
        comp.set_type(COMPUTER_TYPE_CENTRALMAINFRAME);
        comp.set_name(&computername);
        comp.set_company_name(companyname);
        comp.set_is_externally_open(false);
        comp.set_trace_speed(random_normal_int(
            TRACESPEED_CENTRALMAINFRAME as f32,
            TRACESPEED_CENTRALMAINFRAME as f32 * TRACESPEED_VARIANCE,
        ));
        comp.set_trace_action(COMPUTER_TRACEACTION_DISCONNECT | COMPUTER_TRACEACTION_LEGAL);
        comp.set_ip(&vl_ip);

        let monitorstrength = ((company_size - MINCOMPANYSIZE_MONITOR) / 3).clamp(1, 5);
        comp.security
            .add_system_new(SECURITY_TYPE_MONITOR, monitorstrength, -1);

        let proxystrength = ((company_size - MINCOMPANYSIZE_PROXY) / 3).clamp(1, 5);
        comp.security
            .add_system_new(SECURITY_TYPE_PROXY, proxystrength, -1);

        let firewallstrength = ((company_size - MINCOMPANYSIZE_FIREWALL) / 3).clamp(1, 5);
        comp.security
            .add_system_new(SECURITY_TYPE_FIREWALL, firewallstrength, -1);

        if companyname == "Government" {
            comp.set_is_targetable(false);
        }

        // Create the screens

        let mut hs = HighSecurityScreen::new();
        hs.base.set_main_title(companyname);
        hs.base.set_sub_title("Authorisation required");
        hs.add_system("UserID / password verification", 1);
        hs.add_system("Voice Print Identification", 2);
        hs.set_next_page(3);
        comp.add_computer_screen(ComputerScreen::HighSecurityScreen(hs), 0);

        let mut uid = UserIDScreen::new();
        uid.base.set_main_title(companyname);
        uid.base.set_sub_title("Log in");
        uid.set_difficulty(random_normal_int(
            HACKDIFFICULTY_CENTRALMAINFRAME as f32,
            HACKDIFFICULTY_CENTRALMAINFRAME as f32 * HACKDIFFICULTY_VARIANCE,
        ));
        uid.set_next_page(0);
        comp.add_computer_screen(ComputerScreen::UserIDScreen(uid), 1);

        let mut gs2 = GenericScreen::new();
        gs2.base.set_main_title(companyname);
        gs2.base.set_sub_title("Voice print analysis required");
        gs2.set_screen_type(SCREEN_VOICEANALYSIS);
        gs2.set_next_page(0);
        comp.add_computer_screen(ComputerScreen::GenericScreen(gs2), 2);

        let mut ms = MenuScreen::new();
        ms.base.set_main_title(companyname);
        ms.base.set_sub_title("Central Mainframe Main Menu");
        ms.add_option(" File Server", "Access the file server", 4, 3, -1);
        ms.add_option("View logs", "View the access logs on this system", 5, 1, -1);
        ms.add_option("Console", "Use a console on this system", 6, 1, -1);
        comp.add_computer_screen(ComputerScreen::MenuScreen(ms), 3);

        let mut fss = GenericScreen::new();
        fss.set_screen_type(SCREEN_FILESERVERSCREEN);
        fss.base.set_main_title(companyname);
        fss.base.set_sub_title("File server");
        fss.set_next_page(3);
        comp.add_computer_screen(ComputerScreen::GenericScreen(fss), 4);

        let mut ls = LogScreen::new();
        ls.set_target(LOGSCREEN_TARGET_ACCESSLOGS);
        ls.base.set_main_title(companyname);
        ls.base.set_sub_title("Access Logs");
        ls.set_next_page(3);
        comp.add_computer_screen(ComputerScreen::LogScreen(ls), 5);

        let mut gs6 = GenericScreen::new();
        gs6.set_screen_type(SCREEN_CONSOLESCREEN);
        gs6.base.set_main_title(companyname);
        gs6.base.set_sub_title("Console");
        gs6.set_next_page(3);
        comp.add_computer_screen(ComputerScreen::GenericScreen(gs6), 6);

        // 7 is taken up on some companies such as ARC and Arunmor

        // Create the files

        comp.databank.set_size(random_normal_int(100.0, 40.0));

        let numfiles = random_normal_int(10.0, 5.0);

        for _ in 0..numfiles {
            let type_ = NumberGenerator::random_number(2) + 1;
            let size = random_normal_int(6.0, 4.0);
            let encrypted = NumberGenerator::random_number(2);
            let compressed = NumberGenerator::random_number(2);
            let datatitle = NameGenerator::generate_data_name(companyname, type_);

            // PORT: see generate_internal_services_machine for the argument-order note.
            let encrypted_level = if encrypted != 0 {
                NumberGenerator::random_number(5)
            } else {
                0
            };
            let compressed_level = if compressed != 0 {
                NumberGenerator::random_number(5)
            } else {
                0
            };

            let mut data = Data::new();
            data.set_title(&datatitle);
            data.set_details(
                type_,
                size,
                encrypted_level,
                compressed_level,
                1.0,
                SOFTWARETYPE_NONE,
            );
            comp.databank.put_data(data);
        }

        // Create some logs

        let numlogs = NumberGenerator::random_number(10);

        for _ in 0..numlogs {
            let fromip = Self::get_random_location(g);

            let mut al = AccessLog::new();
            al.set_properties(
                &g.world().date,
                &fromip,
                " ",
                LOG_NOTSUSPICIOUS,
                LOG_TYPE_TEXT,
            );
            al.set_data1(Some("Accessed File"));
            comp.logbank.add_log(al, -1);
        }

        // Create some log-in's

        let mut record = Record::new();
        record.add_field(RECORDBANK_NAME, RECORDBANK_ADMIN);
        record.add_field(
            RECORDBANK_PASSWORD,
            &NameGenerator::generate_complex_password(g),
        );
        record.add_field(RECORDBANK_SECURITY, "1");
        comp.recordbank.add_record(record);

        g.world_mut().create_computer_from(comp);

        computername
    }

    pub fn generate_public_bank_server(g: &mut Ctx, companyname: &str) -> String {
        let computername = NameGenerator::generate_international_bank_name(companyname);
        let vl_ip = Self::generate_location(g);

        let mut comp = Computer::new_bank_computer();
        comp.set_type(COMPUTER_TYPE_PUBLICBANKSERVER);
        comp.set_name(&computername);
        comp.set_company_name(companyname);
        comp.set_trace_speed(TRACESPEED_PUBLICBANKSERVER);
        comp.set_trace_action(COMPUTER_TRACEACTION_DISCONNECT | COMPUTER_TRACEACTION_LEGAL);
        comp.security.add_system_new(SECURITY_TYPE_PROXY, 5, -1);
        comp.security.add_system_new(SECURITY_TYPE_MONITOR, 5, -1);
        comp.set_ip(&vl_ip);

        // Screen 0										(Main Menu)

        let mut s0 = MenuScreen::new();
        s0.base.set_main_title(companyname);
        s0.base
            .set_sub_title("Welcome to our Online Banking Server");
        s0.add_option("About Us", "About us", 1, 10, -1);
        s0.add_option("Create New Account", "Create new account", 4, 10, -1);
        s0.add_option(
            "Manage Existing Account",
            "Manage existing account",
            2,
            10,
            -1,
        );
        s0.add_option("Administration", "Access admin features", 6, 10, -1);
        comp.add_computer_screen(ComputerScreen::MenuScreen(s0), 0);

        // Screen 1										(About us)

        let mut gs = GenericScreen::new();
        gs.base.set_main_title(companyname);
        gs.base.set_sub_title("Company data");
        gs.set_screen_type(SCREEN_COMPANYINFO);
        gs.set_next_page(0);
        comp.add_computer_screen(ComputerScreen::GenericScreen(gs), 1);

        // Screen 2										(Login)

        let mut s2 = UserIDScreen::new();
        s2.base.set_main_title(companyname);
        s2.base.set_sub_title("Log in");
        s2.set_next_page(3);
        s2.set_difficulty(HACKDIFFICULTY_PUBLICBANKSERVER);
        comp.add_computer_screen(ComputerScreen::UserIDScreen(s2), 2);

        // Screen 3										(Manage existing account)

        let mut s3 = MenuScreen::new();
        s3.base.set_main_title(companyname);
        s3.base.set_sub_title("Manage Existing Account");
        s3.add_option("View Account", "View account", 12, 10, -1);
        s3.add_option("View Statement", "View Statement", 11, 10, -1);
        s3.add_option("Transfer money", "Transfer money", 7, 10, -1);
        s3.add_option("Loans", "Manage money loans", 10, 10, -1);
        // Close account (19) is commented out in the C++ source.
        s3.add_option("Done", "Return to the main menu", 0, 10, -1);
        comp.add_computer_screen(ComputerScreen::MenuScreen(s3), 3);

        // Screen 4										(Create new account)

        let mut ds4 = DialogScreen::new();
        ds4.base.set_main_title(companyname);
        ds4.base.set_sub_title("Create new account");
        ds4.add_widget(
            "name",
            WIDGET_BASIC,
            80,
            180,
            170,
            20,
            "Enter your name",
            "This is your name",
        );
        ds4.add_widget(
            "password",
            WIDGET_BASIC,
            80,
            210,
            170,
            20,
            "Enter your password",
            "You'll need this to access your account",
        );
        ds4.add_widget(
            "password2",
            WIDGET_BASIC,
            80,
            240,
            170,
            20,
            "Re-type your password",
            "For verification purposes",
        );

        ds4.add_widget(
            "nametext",
            WIDGET_TEXTBOX,
            270,
            180,
            170,
            20,
            "Fill this in",
            "Enter your name here",
        );
        ds4.add_widget(
            "passwordtext",
            WIDGET_PASSWORDBOX,
            270,
            210,
            170,
            20,
            "",
            "Enter your password here",
        );
        ds4.add_widget(
            "passwordtext2",
            WIDGET_PASSWORDBOX,
            270,
            240,
            170,
            20,
            "",
            "Re-type your password here",
        );

        ds4.add_widget_with_data(
            "create",
            WIDGET_SCRIPTBUTTON,
            250,
            330,
            80,
            20,
            "Create",
            "Create this account",
            11,
            -1,
            Some(""),
            Some(""),
        );
        ds4.add_widget_with_data(
            "cancel",
            WIDGET_NEXTPAGE,
            350,
            330,
            80,
            20,
            "Cancel",
            "Cancel",
            0,
            -1,
            Some(""),
            Some(""),
        );

        ds4.set_return_key_button("create");
        comp.add_computer_screen(ComputerScreen::DialogScreen(ds4), 4);

        // Screen 5										(New account created)

        let mut ms5 = MessageScreen::new();
        ms5.base.set_main_title(companyname);
        ms5.base.set_sub_title("Create new account");
        ms5.set_text_message("Your new account has been created.");
        ms5.set_button_message("OK");
        ms5.set_next_page(12);
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms5), 5);

        // Screen 6										(Admin security verification)

        let mut hs = HighSecurityScreen::new();
        hs.base.set_main_title(companyname);
        hs.base.set_sub_title("Admin Authorisation required");
        hs.add_system("Admin Password verification", 13);
        hs.add_system("Voice print Identification", 14);
        hs.add_system("Elliptic-Curve Encryption Cypher", 15);
        hs.set_next_page(16);
        comp.add_computer_screen(ComputerScreen::HighSecurityScreen(hs), 6);

        // Screen 7										(Money transfer)

        let mut ds7 = DialogScreen::new();
        ds7.base.set_main_title(companyname);
        ds7.base.set_sub_title("Transfer money");

        ds7.add_widget(
            "s_accno",
            WIDGET_BASIC,
            100,
            150,
            100,
            15,
            "Acc No. (From)",
            "Shows the source account number",
        );
        ds7.add_widget(
            "t_ip",
            WIDGET_BASIC,
            100,
            200,
            100,
            15,
            "Bank IP (To)",
            "Enter the IP of the target bank here",
        );
        ds7.add_widget(
            "t_accno",
            WIDGET_BASIC,
            100,
            230,
            100,
            15,
            "Acc No. (To)",
            "Enter the target account number here",
        );
        ds7.add_widget(
            "amount",
            WIDGET_BASIC,
            100,
            280,
            100,
            15,
            "Amount",
            "Enter the value (in credits) to transfer",
        );

        ds7.add_widget_with_data(
            "s_accno_value",
            WIDGET_FIELDVALUE,
            250,
            150,
            200,
            15,
            "",
            "",
            -1,
            -1,
            Some("Account Number"),
            Some(""),
        );
        ds7.add_widget(
            "t_ip_value",
            WIDGET_TEXTBOX,
            250,
            200,
            200,
            15,
            "Fill this in",
            "Fill this in",
        );
        ds7.add_widget(
            "t_accno_value",
            WIDGET_TEXTBOX,
            250,
            230,
            200,
            15,
            "Fill this in",
            "Fill this in",
        );
        ds7.add_widget(
            "amount_value",
            WIDGET_TEXTBOX,
            250,
            280,
            200,
            15,
            "Fill this in",
            "Fill this in",
        );

        ds7.add_widget_with_data(
            "transfer",
            WIDGET_SCRIPTBUTTON,
            250,
            330,
            80,
            15,
            "Transfer",
            "Perform the transaction",
            10,
            -1,
            Some(""),
            Some(""),
        );
        ds7.add_widget_with_data(
            "cancel",
            WIDGET_NEXTPAGE,
            350,
            330,
            80,
            15,
            "Cancel",
            "Cancel the transfer",
            3,
            -1,
            Some(""),
            Some(""),
        );

        comp.add_computer_screen(ComputerScreen::DialogScreen(ds7), 7);

        // Screen 8										(Transfer successfull)

        let mut ms8 = MessageScreen::new();
        ms8.base.set_main_title(companyname);
        ms8.base.set_sub_title("Transfer completed");
        ms8.set_text_message("The transfer was completed successfully.");
        ms8.set_button_message("OK");
        ms8.set_next_page(3);
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms8), 8);

        // Screen 9										(Transfer not successfull)

        let mut ms9 = MessageScreen::new();
        ms9.base.set_main_title(companyname);
        ms9.base.set_sub_title("Transfer failed");
        ms9.set_text_message(
            "The transfer was not completed successfully.\nPlease contact the bank administrator.",
        );
        ms9.set_button_message("OK");
        ms9.set_next_page(3);
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms9), 9);

        // Screen 10

        let mut gs10 = GenericScreen::new();
        gs10.set_screen_type(SCREEN_LOANSSCREEN);
        gs10.base.set_main_title(companyname);
        gs10.base.set_sub_title("Loans Management");
        gs10.set_next_page(3);
        comp.add_computer_screen(ComputerScreen::GenericScreen(gs10), 10);

        // Screen 11

        let mut log11 = LogScreen::new();
        log11.set_target(LOGSCREEN_TARGET_ACCOUNTLOGS);
        log11.base.set_main_title(companyname);
        log11.base.set_sub_title("Account Statement");
        log11.set_next_page(3);
        comp.add_computer_screen(ComputerScreen::LogScreen(log11), 11);

        // Screen 12									(View account)

        let mut view12 = GenericScreen::new();
        view12.set_screen_type(SCREEN_ACCOUNTSCREEN);
        view12.base.set_main_title(companyname);
        view12.base.set_sub_title("View account");
        view12.set_next_page(3);
        comp.add_computer_screen(ComputerScreen::GenericScreen(view12), 12);

        // Screen 13									(Admin password screen)

        let mut ps13 = PasswordScreen::new();
        ps13.base.set_main_title(companyname);
        ps13.base.set_sub_title("Access code required");
        ps13.set_password(&NameGenerator::generate_complex_password(g));
        ps13.set_difficulty(HACKDIFFICULTY_PUBLICBANKSERVER_ADMIN);
        ps13.set_next_page(6);
        comp.add_computer_screen(ComputerScreen::PasswordScreen(ps13), 13);

        // Screen 14									(Voice print analysis)

        let mut gs14 = GenericScreen::new();
        gs14.base.set_main_title(companyname);
        gs14.base.set_sub_title("Voice print analysis required");
        gs14.set_screen_type(SCREEN_VOICEANALYSIS);
        gs14.set_next_page(6);
        comp.add_computer_screen(ComputerScreen::GenericScreen(gs14), 14);

        // Screen 15									(Elliptic-Curve encryption cypher)

        let mut cs = CypherScreen::new();
        cs.base.set_main_title(companyname);
        cs.base
            .set_sub_title("Enter Elliptic-Curve encryption cypher");
        cs.set_difficulty(HACKDIFFICULTY_PUBLICBANKSERVER_ADMIN);
        cs.set_next_page(6);
        comp.add_computer_screen(ComputerScreen::CypherScreen(cs), 15);

        // Screen 16									(Admin menu)

        let mut ms16 = MenuScreen::new();
        ms16.base.set_main_title(companyname);
        ms16.base.set_sub_title("Admin Menu");
        ms16.add_option("View All Accounts", "View all accounts", 17, 10, -1);
        ms16.add_option("View access logs", "View activity logs", 18, 10, -1);
        ms16.add_option("Done", "Close admin options menu", 0, 10, -1);
        comp.add_computer_screen(ComputerScreen::MenuScreen(ms16), 16);

        // Screen 17									(View all records)

        let mut rs17 = GenericScreen::new();
        rs17.set_screen_type(SCREEN_RECORDSCREEN);
        rs17.base.set_main_title(companyname);
        rs17.base.set_sub_title("Bank accounts");
        rs17.set_next_page(16);
        comp.add_computer_screen(ComputerScreen::GenericScreen(rs17), 17);

        // Screen 18									(Access logs)

        let mut ls18 = LogScreen::new();
        ls18.set_target(LOGSCREEN_TARGET_ACCESSLOGS);
        ls18.base.set_main_title(companyname);
        ls18.base.set_sub_title("Access logs");
        ls18.set_next_page(16);
        comp.add_computer_screen(ComputerScreen::LogScreen(ls18), 18);

        // Screen 19                                    (Close account)

        let mut ds19 = DialogScreen::new();
        ds19.base.set_main_title(companyname);
        ds19.base.set_sub_title("Close account");
        ds19.add_widget(
            "text",
            WIDGET_CAPTION,
            100,
            150,
            200,
            50,
            "Are you sure you wish to close your account?",
            "Shows the source account number",
        );
        ds19.add_widget_with_data(
            "close",
            WIDGET_SCRIPTBUTTON,
            250,
            330,
            80,
            15,
            "Close",
            "Close your account",
            13,
            -1,
            Some(""),
            Some(""),
        );
        ds19.add_widget_with_data(
            "cancel",
            WIDGET_NEXTPAGE,
            350,
            330,
            80,
            15,
            "Cancel",
            "Do not close the account",
            3,
            -1,
            Some(""),
            Some(""),
        );

        comp.add_computer_screen(ComputerScreen::DialogScreen(ds19), 19);

        // Screen 20                                    (Account closed)

        let mut ms20 = MessageScreen::new();
        ms20.base.set_main_title(companyname);
        ms20.base.set_sub_title("Account closed");
        ms20.set_text_message(
            "Your account has been closed.\nClick OK to return to the main menu.",
        );
        ms20.set_button_message("OK");
        ms20.set_next_page(0);
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms20), 20);

        g.world_mut().create_computer_from(comp);

        computername
    }

    /// Used in StealResearch missions
    pub fn generate_empty_file_server(g: &mut Ctx, companyname: &str) -> String {
        let computername = NameGenerator::generate_file_server_name(companyname);
        if g.world().get_computer(&computername).is_some() {
            return computername;
        }

        let vl_ip = Self::generate_location(g);
        g.world_mut()
            .get_vlocation_mut(&vl_ip)
            .expect("location was just generated")
            .set_listed(false);

        let mut comp = Computer::new();
        comp.set_type(COMPUTER_TYPE_UNKNOWN);
        comp.set_name(&computername);
        comp.set_company_name(companyname);
        comp.set_trace_speed(0);
        comp.set_trace_action(COMPUTER_TRACEACTION_DISCONNECT);
        comp.set_is_targetable(false);
        comp.set_ip(&vl_ip);
        comp.databank.set_size(100);

        // Screen 0			User ID

        let mut uid0 = UserIDScreen::new();
        uid0.base.set_main_title(companyname);
        uid0.base.set_sub_title("File Server");
        uid0.set_next_page(1);
        comp.add_computer_screen(ComputerScreen::UserIDScreen(uid0), 0);

        // Screen 1			File Server

        let mut gs1 = GenericScreen::new();
        gs1.base.set_main_title(companyname);
        gs1.base.set_sub_title("File Server");
        gs1.set_screen_type(SCREEN_FILESERVERSCREEN);
        comp.add_computer_screen(ComputerScreen::GenericScreen(gs1), 1);

        g.world_mut().create_computer_from(comp);

        computername
    }

    /// `None` if the company does not exist.
    pub fn generate_lan(g: &mut Ctx, companyname: &str) -> Option<String> {
        let size = g.world().get_company(companyname)?.size;

        let difficulty = if size > 40 { 4 } else { size / 10 };

        Some(LanGenerator::generate_lan(g, companyname, difficulty))
    }

    pub fn generate_personal_computer(g: &mut Ctx, personname: &str) -> String {
        let computername = NameGenerator::generate_personal_computer_name(personname);
        let vl_ip = Self::generate_location(g);
        g.world_mut()
            .get_vlocation_mut(&vl_ip)
            .expect("location was just generated")
            .set_listed(false);

        // C++ `World::CreateCompany ( char *name )`: a new default company called personname.
        let mut company = Company::new();
        company.set_name(personname);
        g.world_mut().create_company_from(company);

        let mut comp = Computer::new();
        comp.set_type(COMPUTER_TYPE_PERSONALCOMPUTER);
        comp.set_name(&computername);
        comp.set_company_name(personname);
        comp.set_trace_speed(0);
        comp.set_trace_action(COMPUTER_TRACEACTION_DISCONNECT);
        comp.set_is_targetable(false);
        comp.set_ip(&vl_ip);

        let mut ms0 = MessageScreen::new();
        ms0.base.set_main_title(personname);
        ms0.base.set_sub_title("Personal Computer");
        ms0.set_text_message("No external log-ins permitted.\nPlease leave now.");
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms0), -1);

        g.world_mut().create_computer_from(comp);

        computername
    }

    pub fn generate_voice_phone_system(g: &mut Ctx, personname: &str) -> String {
        let computername = NameGenerator::generate_voice_phone_system_name(personname);
        let (x, y) = Self::generate_valid_map_pos();

        // PORT: sprintf argument order is left to right here (C++ left it unspecified).
        let d: [i32; 10] = std::array::from_fn(|_| NumberGenerator::random_number(10));
        let ip = format!(
            "(0{}{}{}{}){}{}{}{}{}{}",
            d[0], d[1], d[2], d[3], d[4], d[5], d[6], d[7], d[8], d[9]
        );

        let vl = g.world_mut().create_vlocation(&ip, x, y);
        vl.set_listed(false);

        let mut comp = Computer::new();
        comp.set_type(COMPUTER_TYPE_VOICEPHONESYSTEM);
        comp.set_name(&computername);
        comp.set_company_name(personname);
        comp.set_trace_speed(0);
        comp.set_trace_action(COMPUTER_TRACEACTION_DISCONNECT);
        comp.set_is_targetable(false);
        comp.set_ip(&ip);

        let mut gs0 = GenericScreen::new();
        gs0.base.set_main_title(personname);
        gs0.base.set_sub_title(&ip);
        gs0.set_screen_type(SCREEN_VOICEPHONE);
        comp.add_computer_screen(ComputerScreen::GenericScreen(gs0), 0);

        g.world_mut().create_computer_from(comp);

        computername
    }

    // Low level functions for "fleshing out" specific items of data

    /// `GenerateCompany ( companyname, size, TYPE, growth, alignment )`. Returns the company's name.
    pub fn generate_company_named(
        g: &mut Ctx,
        companyname: &str,
        size: i32,
        type_: i32,
        growth: i32,
        alignment: i32,
    ) -> String {
        let mut company = Company::new();
        company.set_name(companyname);
        company.set_size(size);
        company.set_type(type_);
        company.set_growth(growth);
        company.set_alignment(alignment);

        g.world_mut().create_company_from(company);

        // Generate the computers owned by this company

        Self::generate_public_access_server(g, companyname);
        Self::generate_internal_services_machine(g, companyname);
        Self::generate_central_mainframe(g, companyname);
        Self::generate_lan(g, companyname);

        for _ in 0..(size / 20) {
            Self::generate_computer(g, companyname);
        }

        // Generate contact addresses for this company

        let contactcomputer = NameGenerator::generate_public_access_server_name(companyname);
        let comp_ip = g
            .world()
            .get_computer(&contactcomputer)
            .expect("contact computer was just generated")
            .ip
            .clone();

        let personname = format!("internal@{}.net", companyname);
        let pinternal = g.world_mut().create_person(&personname, &comp_ip);
        pinternal.set_is_targetable(false);

        // Set up our share prices for the start

        Company::grow(g, companyname, 0);

        companyname.to_string()
    }

    // Functions which return a random entry (from the BTree keys in pre-order, like the C++
    // ConvertIndexToDArray, each looked up again by key)

    /// Returns a location ip.
    pub fn get_random_location(g: &Ctx) -> String {
        let ips: Vec<String> = g.world().locations.keys_all();
        debug_assert!(!ips.is_empty());

        let index = NumberGenerator::random_number(ips.len() as i32) as usize;
        ips[index].clone()
    }

    /// Returns a company name.
    pub fn get_random_company(g: &Ctx) -> String {
        let world = g.world();
        let comps: Vec<String> = world.companies.keys_all();
        debug_assert!(!comps.is_empty());

        loop {
            let index = NumberGenerator::random_number(comps.len() as i32) as usize;
            let comp: &Company = world.get_company(&comps[index]).expect("UplinkAssert: company");

            // Make sure we haven't picked up the player's computer
            // Or a persons private company
            // Or a Government computer
            // Or an Uplink computer

            if comp.name != "Player"
                && comp.name != "Government"
                && comp.name != "Uplink"
                && comp.name != "Sample Company"
                && comp.type_ != COMPANYTYPE_UNKNOWN
            {
                return comp.name.clone();
            }
        }
    }

    /// Returns a computer name.
    pub fn get_random_computer(g: &mut Ctx) -> String {
        Self::get_random_computer_of_type(g, COMPUTER_TYPE_ANY)
    }

    /// TYPE is a bitfield. May generate a new computer if none matches. Returns a computer name.
    pub fn get_random_computer_of_type(g: &mut Ctx, type_: i32) -> String {
        debug_assert!(!g.world().computers.is_empty());

        // Create a new array of all valid computers

        let world = g.world();
        let typecomps: Vec<String> = world
            .computers
            .keys_all()
            .into_iter()
            .filter(|name| {
                let comp = world.get_computer(name).expect("UplinkAssert: computer");
                (comp.type_ & type_) != 0
                    && comp.istargetable
                    && comp.isrunning
                    && comp.companyname != "Government"
            })
            .collect();

        if typecomps.is_empty() {
            // We couldn't find a valid computer
            // Generate one now

            println!("WorldGenerator WARNING: Couldn't find any computers - generated a new one");

            let company = Self::get_random_company(g);
            // PORT: the C++ returned NULL when TYPE matches no generator; here that is an error.
            return Self::generate_computer_of_type(g, &company, type_)
                .expect("TYPE matches no computer generator");
        }

        let index = NumberGenerator::random_number(typecomps.len() as i32) as usize;
        typecomps[index].clone()
    }

    /// May generate a new person if none qualifies. Returns a person name.
    pub fn get_random_person(g: &mut Ctx) -> String {
        // Build a list of all valid People
        // (the "PLAYER" key gives the player, which is not targetable)

        let world = g.world();
        let valid: Vec<String> = world
            .people
            .keys_all()
            .iter()
            .map(|name| world.get_person(name).expect("UplinkAssert: person"))
            .filter(|p| p.istargetable && p.status == PERSON_STATUS_NONE)
            .map(|p| p.name.clone())
            .collect();

        if valid.is_empty() {
            // No person fits the criteria
            // Create a new one and return him

            return Self::generate_person(g);
        }

        let index = NumberGenerator::random_number(valid.len() as i32) as usize;
        valid[index].clone()
    }

    /// May generate a new agent if none qualifies. Returns a person name.
    pub fn get_random_agent(g: &mut Ctx) -> String {
        // Build a list of all valid Agents

        let world = g.world();
        let valid: Vec<String> = world
            .people
            .keys_all()
            .iter()
            .map(|name| world.get_person(name).expect("UplinkAssert: person"))
            .filter(|p| {
                p.istargetable && p.rating.uplinkrating > 0 && p.status == PERSON_STATUS_NONE
            })
            .map(|p| p.name.clone())
            .collect();

        if valid.is_empty() {
            // No agent fits the criteria
            // Create a new one and return him

            return Self::generate_agent(g);
        }

        let index = NumberGenerator::random_number(valid.len() as i32) as usize;
        valid[index].clone()
    }

    /// Returns an index into the Uplink company's `missions`.
    pub fn get_random_mission(g: &Ctx) -> i32 {
        let cu = g
            .world()
            .get_company("Uplink")
            .and_then(|c| c.uplink.as_deref())
            .expect("Uplink company must exist");

        debug_assert!(!cu.missions.is_empty());

        let index = NumberGenerator::random_number(cu.missions.len() as i32);
        debug_assert!(index >= 0 && (index as usize) < cu.missions.len());
        index
    }

    /// No proxys, no firewalls. Returns a computer name.
    pub fn get_random_low_security_computer(g: &mut Ctx, type_: i32) -> String {
        loop {
            let result = Self::get_random_computer_of_type(g, type_);
            let comp = g
                .world()
                .get_computer(&result)
                .expect("computer must exist");

            if !comp.security.is_running_proxy()
                && !comp.security.is_running_firewall()
                && comp.traceaction < COMPUTER_TRACEACTION_LEGAL
            {
                return result;
            }
        }
    }

    // Misc

    /// Generates map co-ordinates inland: returns `(x, y)`.
    pub fn generate_valid_map_pos() -> (i32, i32) {
        WORLDMAPMASK.with(|m| {
            let mask = m.borrow();
            let mask = mask.as_ref().expect("UplinkAssert(worldmapmask)");
            loop {
                let t_x = NumberGenerator::random_number(VIRTUAL_WIDTH - 1);
                let t_y = NumberGenerator::random_number(VIRTUAL_HEIGHT - 1);

                assert!(t_x >= 0 && t_x < VIRTUAL_WIDTH);
                assert!(t_y >= 0 && t_y < VIRTUAL_HEIGHT);

                if mask.get_pixel_r(t_x, t_y) != 0 {
                    return (t_x, t_y);
                }
            }
        })
    }

    /// The software and hardware for sale (the two sales loops of `GenerateCompanyUplink`).
    fn generate_uplink_sales() -> CompanyUplink {
        let mut cu = CompanyUplink::new();
        // C++ `previoussale`: the sale most recently created, as (is hardware, index in its list)
        let mut previous: Option<(bool, usize)> = None;

        for is in 0..NUM_STARTINGSOFTWAREUPGRADES as usize {
            Self::add_upgrade_sale(&mut cu, &mut previous, false, &SOFTWARE_UPGRADES[is]);
        }

        for ih in 0..NUM_STARTINGHARDWAREUPGRADES as usize {
            Self::add_upgrade_sale(&mut cu, &mut previous, true, &HARDWARE_UPGRADES[ih]);
        }

        cu
    }

    /// One item of the sales loops: a new version of the previous sale when it has the same title,
    /// otherwise a new sale.
    fn add_upgrade_sale(
        cu: &mut CompanyUplink,
        previous: &mut Option<(bool, usize)>,
        hardware: bool,
        upgrade: &ComputerUpgrade,
    ) {
        if let Some((prev_hardware, index)) = *previous {
            let list = if prev_hardware {
                &mut cu.hw_sales
            } else {
                &mut cu.sw_sales
            };
            if list[index].title == upgrade.name {
                // New version of an existing item
                list[index].add_version(
                    upgrade.description,
                    upgrade.cost,
                    upgrade.size,
                    upgrade.data,
                );
                return;
            }
        }

        // New sale item
        let mut sale = Sale::new();
        sale.set_title(upgrade.name);
        sale.set_sale_type(if hardware {
            SALETYPE_HARDWARE
        } else {
            SALETYPE_SOFTWARE
        });
        sale.set_swhw_type(upgrade.type_);

        sale.add_version(" ", -1, -1, -1);
        sale.add_version(
            upgrade.description,
            upgrade.cost,
            upgrade.size,
            upgrade.data,
        );

        if hardware {
            cu.create_hw_sale(sale);
            *previous = Some((true, cu.hw_sales.len() - 1));
        } else {
            cu.create_sw_sale(sale);
            *previous = Some((false, cu.sw_sales.len() - 1));
        }
    }

    /// The `while` loop of the two starting missions: an employer, and a low security internal
    /// services machine that does not belong to that employer.
    fn pick_starting_employer_and_target(g: &mut Ctx) -> (String, String) {
        loop {
            let employer = Self::get_random_company(g);
            assert!(!employer.is_empty(), "UplinkAssert(employer)");
            let target =
                Self::get_random_low_security_computer(g, COMPUTER_TYPE_INTERNALSERVICESMACHINE);
            let target_company = g
                .world()
                .get_computer(&target)
                .expect("UplinkAssert(target)")
                .companyname
                .clone();

            if employer != target_company {
                return (employer, target);
            }
        }
    }

    /// C++ `mission->SetMinRating` on the pointer the generator returned. The Rust generators hand
    /// back a copy of the mission they posted, so the posted copy is found again by its fields.
    fn set_min_rating_on_posted(g: &mut Ctx, mission: &Mission, rating: i32) {
        let missions = g
            .world_mut()
            .get_company_mut("Uplink")
            .and_then(|c| c.uplink.as_deref_mut())
            .map(|u| &mut u.missions)
            .expect("UplinkAssert(Uplink company)");

        let posted = missions.iter_mut().rev().find(|m| {
            m.employer == mission.employer
                && m.description == mission.description
                && m.payment == mission.payment
                && m.details == mission.details
        });
        debug_assert!(posted.is_some(), "generated mission was not posted");
        if let Some(posted) = posted {
            posted.set_min_rating(rating);
        }
    }
}

/// A help or message screen whose button says "Return".
fn return_message(main_title: &str, sub_title: &str, text: &str, next_page: i32) -> MessageScreen {
    let mut ms = MessageScreen::new();
    ms.base.set_main_title(main_title);
    ms.base.set_sub_title(sub_title);
    ms.set_text_message(text);
    ms.set_button_message("Return");
    ms.set_next_page(next_page);
    ms
}

/// C++ "Generate contact addresses for this company": a non-targetable person living at the
/// ip of the computer called `computer_name`.
fn generate_internal_contact(g: &mut Ctx, computer_name: &str, person_name: &str) {
    let contact_ip = g
        .world()
        .get_computer(computer_name)
        .expect("contact computer was just created")
        .ip
        .clone();
    g.world_mut()
        .create_person(person_name, &contact_ip)
        .set_is_targetable(false);
}
