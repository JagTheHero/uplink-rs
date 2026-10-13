//! Port of source/world/generator/plotgenerator.h + source/world/generator/plotgenerator.cpp

use serde::{Deserialize, Serialize};

use crate::app::globals;
use crate::ctx::Ctx;
use crate::game::data::data::{
    NAME_UPLINKINTERNALSERVICES, PAYMENT_SPECIALMISSION_BACKFIRE, PAYMENT_SPECIALMISSION_TAKEMETOYOURLEADER,
    PAYMENT_SPECIALMISSION_TRACER, TIME_ARCBUSTED_WITHOUTPLAYER, TIME_ARCBUSTED_WITHPLAYER,
    GAME_START_DATE, HACKDIFFICULTY_ARCCENTRALMAINFRAME, HACKDIFFICULTY_ARUNMORCENTRALMAINFRAME,
    PAYMENT_SPECIALMISSION_ARCINFILTRATION, PAYMENT_SPECIALMISSION_COUNTERATTACK,
    PAYMENT_SPECIALMISSION_DARWIN, PAYMENT_SPECIALMISSION_GRANDTOUR,
    PAYMENT_SPECIALMISSION_MAIDENFLIGHT, PAYMENT_SPECIALMISSION_SAVEITFORTHEJURY,
    PAYMENT_SPECIALMISSION_SHINYHAMMER, REVELATION_RELEASEUNCONTROLLED,
};
use crate::game::game::GAMESPEED_NORMAL;
use crate::interface::remoteinterface::remoteinterface::{RemoteInterface, SCREEN_FAITHSCREEN, SCREEN_FILESERVERSCREEN};
use crate::interface::taskmanager::taskmanager::TaskManager;
use crate::world::company::company::{Company, COMPANYTYPE_ACADEMIC, COMPANYTYPE_COMMERCIAL};
use crate::world::company::companyuplink::CompanyUplink;
use crate::world::company::mission::Mission;
use crate::world::company::news::{News, NEWS_TYPE_COMPUTERDESTROYED};
use crate::world::computer::computer::{
    Computer, COMPUTER_TRACEACTION_DISCONNECT, COMPUTER_TYPE_CENTRALMAINFRAME, COMPUTER_TYPE_INTERNALSERVICESMACHINE,
    COMPUTER_TYPE_PERSONALCOMPUTER, COMPUTER_TYPE_PUBLICBANKSERVER,
};
use crate::world::computer::computerscreen::computerscreen::ComputerScreen;
use crate::world::computer::computerscreen::cypherscreen::CypherScreen;
use crate::world::computer::computerscreen::dialogscreen::{
    DialogScreen, WIDGET_CAPTION, WIDGET_NEXTPAGE, WIDGET_SCRIPTBUTTON, WIDGET_TEXTBOX,
};
use crate::world::computer::computerscreen::genericscreen::GenericScreen;
use crate::world::computer::computerscreen::menuscreen::MenuScreen;
use crate::world::computer::computerscreen::messagescreen::MessageScreen;
use crate::world::computer::computerscreen::passwordscreen::PasswordScreen;
use crate::world::computer::databank::{
    Data, DATATYPE_DATA, DATATYPE_PROGRAM, SOFTWARETYPE_NONE, SOFTWARETYPE_OTHER,
};
use crate::world::computer::logbank::{
    AccessLog, LogBank, LOG_NOTSUSPICIOUS, LOG_TYPE_CONNECTIONOPENED, LOG_TYPE_TEXT,
};
use crate::world::computer::recordbank::{Record, RECORDBANK_NAME, RECORDBANK_PASSWORD, RECORDBANK_SECURITY};
use crate::world::connection::Connection;
use crate::world::date::Date;
use crate::world::generator::missiongenerator::MISSION_SPECIAL;
use crate::world::generator::namegenerator::NameGenerator;
use crate::world::generator::numbergenerator::NumberGenerator;
use crate::world::generator::worldgenerator::WorldGenerator;
use crate::world::message::Message;
use crate::world::person::{Person, PERSON_STATUS_DEAD, PERSON_STATUS_NONE};
use crate::world::rating::Rating;
use crate::world::scheduler::runplotsceneevent::RunPlotSceneEvent;
use crate::world::scheduler::uplinkevent::UplinkEvent;

pub const SPECIALMISSION_BACKFIRE: i32 = 0;
pub const SPECIALMISSION_TRACER: i32 = 1;
pub const SPECIALMISSION_TAKEMETOYOURLEADER: i32 = 2;
pub const SPECIALMISSION_ARCINFILTRATION: i32 = 3;
pub const SPECIALMISSION_COUNTERATTACK: i32 = 4;
pub const SPECIALMISSION_MAIDENFLIGHT: i32 = 5;
pub const SPECIALMISSION_DARWIN: i32 = 6;
pub const SPECIALMISSION_SAVEITFORTHEJURY: i32 = 7;
pub const SPECIALMISSION_SHINYHAMMER: i32 = 8;
pub const SPECIALMISSION_GRANDTOUR: i32 = 9;
pub const SPECIALMISSION_MOLE: i32 = 10;
pub const SPECIALMISSION_WARGAMES: i32 = 11;

/// Short names of the special missions, indexed by SPECIALMISSION_*.
const MISSION_TITLE: [&str; 16] = [
    "Backfire",
    "Tracer",
    "TakeMeToYourLeader",
    "ARCInfiltration",
    "CounterAttack",
    "MaidenFlight",
    "Darwin",
    "SaveItForTheJury",
    "ShinyHammer",
    "GrandTour",
    "Mole????",
    "Wargames????",
    "12",
    "13",
    "14",
    "15",
];

/// Mission board descriptions of the special missions, indexed by SPECIALMISSION_*.
const MISSION_DESCRIPTION: [&str; 16] = [
    "Counter attack!",
    "Perform a covert installation of a Tracer",
    "Help bring the CEO of a major company into custody",
    "Take advantage of an amazing opportunity",
    "Prevent the outbreak of Revelation",
    "Test run Revelation Version 1.0",
    "Steal vital Research into Digitial Life Forms",
    "Frame the chief tech of Arunmor Corporation",
    "Destroy all research completed by Arunmor",
    "Release the Revelation Virus into the wild",
    "Help us track down some people we wish to speak with",
    "wargames",
    "12",
    "13",
    "14",
    "15",
];

/// Only one instance exists (`World::plotgenerator`). Handles the details of running the main plot
/// (Revelation) alongside the game; generates missions, computers, people etc.
///
/// PORT: the C++ `protected` fields are all `pub` (the world model is plain data).
/// Every method that touches the world takes `g: &mut Ctx` instead of `self` (the generator lives
/// inside the world); it reaches its own state through `g.world_mut().plotgenerator`.
/// Methods that only touch these fields keep `&self` / `&mut self`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PlotGenerator {
    pub act: i32,
    pub scene: i32,

    /// The guy who first raises the issue
    pub act1scene3agent: String,
    /// The guy who is first executed
    pub act1scene4agent: String,
    /// The guy who does the Arunmor test mission
    pub act2scene1agent: String,

    /// Set to true if the player pokes around the plot
    pub playervisitsplotsites: bool,
    /// Has the player cancelled the warning mail from a1s7?
    pub playercancelsmail: bool,
    /// 0 = none, -1 = ARC, 1 = Arunmor
    pub playerloyalty: i32,

    pub completed_tracer: bool,
    pub completed_takemetoyourleader: bool,
    pub completed_arcinfiltration: bool,
    pub completed_darwin: bool,
    pub completed_saveitforthejury: bool,
    pub completed_shinyhammer: bool,

    // Special variables for special missions
    pub saveitforthejury_guytobeframed: String,
    pub saveitforthejury_targetbankip: String,
    pub tracer_lastknownip: String,

    /// Number of times Revelation has been released
    pub numuses_revelation: i32,

    pub version_revelation: f32,
    pub version_faith: f32,

    /// List of all infected IP addresses
    pub infected: Vec<String>,
    /// Has the Internet been destroyed?
    pub revelation_releaseuncontrolled: bool,
    /// Failed to reach critical mass in time
    pub revelation_releasefailed: bool,
    /// Has ARC been busted?
    pub revelation_arcbusted: bool,

    /// Bitfield of player completed special missions, see SPECIALMISSION_*
    pub specialmissionscompleted: i32,
}

impl Default for PlotGenerator {
    fn default() -> Self {
        PlotGenerator {
            act: 0,
            scene: 0,
            act1scene3agent: " ".to_string(),
            act1scene4agent: " ".to_string(),
            act2scene1agent: " ".to_string(),
            playervisitsplotsites: false,
            playercancelsmail: false,
            playerloyalty: 0,
            completed_tracer: false,
            completed_takemetoyourleader: false,
            completed_arcinfiltration: false,
            completed_darwin: false,
            completed_saveitforthejury: false,
            completed_shinyhammer: false,
            saveitforthejury_guytobeframed: "None".to_string(),
            saveitforthejury_targetbankip: "None".to_string(),
            // PORT: the trailing space is in the C++ ("None ")
            tracer_lastknownip: "None ".to_string(),
            numuses_revelation: 0,
            version_revelation: 0.0,
            version_faith: 0.0,
            infected: Vec::new(),
            revelation_releaseuncontrolled: false,
            revelation_releasefailed: false,
            revelation_arcbusted: false,
            specialmissionscompleted: 0,
        }
    }
}

// ---- helpers: the C++ reached these objects through `game->GetWorld()` ----

fn pg(g: &Ctx) -> &PlotGenerator {
    &g.world().plotgenerator
}

fn pg_mut(g: &mut Ctx) -> &mut PlotGenerator {
    &mut g.world_mut().plotgenerator
}

fn computer_mut<'a>(g: &'a mut Ctx, name: &str) -> &'a mut Computer {
    g.world_mut()
        .get_computer_mut(name)
        .expect("plotgenerator: computer not found")
}

fn computer_ip(g: &Ctx, name: &str) -> String {
    g.world()
        .get_computer(name)
        .expect("plotgenerator: computer not found")
        .ip
        .clone()
}

fn company_mut<'a>(g: &'a mut Ctx, name: &str) -> &'a mut Company {
    g.world_mut()
        .get_company_mut(name)
        .expect("plotgenerator: company not found")
}

fn person_mut<'a>(g: &'a mut Ctx, name: &str) -> &'a mut Person {
    g.world_mut()
        .get_person_mut(name)
        .expect("plotgenerator: person not found")
}

/// `((Agent *) GetPerson (name))->handle`
fn person_handle(g: &Ctx, name: &str) -> String {
    g.world()
        .get_person(name)
        .expect("plotgenerator: person not found")
        .agent()
        .handle
        .clone()
}

/// `(DialogScreen *) comp->GetComputerScreen (index)`
fn dialog_screen_mut(comp: &mut Computer, index: i32) -> &mut DialogScreen {
    comp.get_computer_screen_mut(index)
        .and_then(|s| s.as_dialog_screen_mut())
        .expect("plotgenerator: screen is not a DialogScreen")
}

/// The `RunPlotSceneEvent` scheduling pattern used throughout the C++.
fn schedule_scene(g: &mut Ctx, rundate: &Date, act: i32, scene: i32) {
    let mut rns = RunPlotSceneEvent::new();
    rns.set_act_and_scene(act, scene);
    let mut ev = UplinkEvent::RunPlotSceneEvent(rns);
    ev.set_run_date(rundate);
    g.world_mut().scheduler.schedule_event(ev);
}

impl PlotGenerator {
    pub fn new() -> Self {
        Self::default()
    }

    /// `UplinkObject::Update` (empty in C++)
    pub fn update(&mut self) {}

    // ---- trivial accessors ----

    pub fn player_visits_plot_sites(&mut self) {
        self.playervisitsplotsites = true;
    }

    pub fn player_cancels_mail(&mut self) {
        self.playercancelsmail = true;
    }

    pub fn player_completed_special_mission(&self, mission_id: i32) -> bool {
        (self.specialmissionscompleted & (1 << mission_id)) != 0
    }

    pub fn get_special_missions_completed(&self) -> i32 {
        self.specialmissionscompleted
    }

    pub fn get_version_faith(&self) -> f32 {
        self.version_faith
    }

    pub fn get_version_revelation(&self) -> f32 {
        self.version_revelation
    }

    pub fn run_revelation_tracer(&mut self, ip: &str) {
        self.tracer_lastknownip = ip.to_string();
    }

    pub fn special_mission_title(mission_id: i32) -> &'static str {
        debug_assert!((0..16).contains(&mission_id));
        MISSION_TITLE[mission_id as usize]
    }

    pub fn special_mission_description(mission_id: i32) -> &'static str {
        debug_assert!((0..16).contains(&mission_id));
        MISSION_DESCRIPTION[mission_id as usize]
    }

    // ---- setup and scheduling ----

    /// Set up companies/computers/people
    pub fn initialise(g: &mut Ctx) {
        Self::initialise_arc(g);
        Self::initialise_andromeda(g);
        Self::initialise_arunmor(g);
        Self::initialise_darwin(g);
        Self::run_act1scene1(g);
    }

    pub fn initialise_arc(g: &mut Ctx) {
        let companyname = "ARC";

        let mut arc = Company::new();
        arc.set_name(companyname);
        arc.set_size(50);
        arc.set_type(COMPANYTYPE_COMMERCIAL);
        arc.set_growth(10);
        arc.set_alignment(-100);
        g.world_mut().create_company_from(arc);

        let pas = WorldGenerator::generate_public_access_server(g, companyname);
        let ism = WorldGenerator::generate_internal_services_machine(g, companyname);
        let cm = WorldGenerator::generate_central_mainframe(g, companyname);
        let lan = WorldGenerator::generate_lan(g, companyname).expect("plotgenerator: no ARC LAN");

        computer_mut(g, &cm)
            .get_computer_screen_mut(0)
            .and_then(|s| s.as_high_security_screen_mut())
            .expect("plotgenerator: ARC central mainframe screen 0 is not a HighSecurityScreen")
            .add_system("Elliptic-Curve Encryption Cypher", 7);

        let mut cs = CypherScreen::new();
        cs.base.set_main_title("ARC");
        cs.base.set_sub_title("Enter elliptic-curve encryption cypher");
        cs.set_difficulty(HACKDIFFICULTY_ARCCENTRALMAINFRAME);
        cs.set_next_page(0);
        computer_mut(g, &cm).add_computer_screen(ComputerScreen::CypherScreen(cs), 7);

        let ceo = WorldGenerator::get_random_person(g);
        person_mut(g, &ceo).set_is_targetable(false);
        company_mut(g, companyname).set_boss(&ceo);

        let admin = WorldGenerator::get_random_person(g);
        person_mut(g, &admin).set_is_targetable(false);
        company_mut(g, companyname).set_admin(&admin);

        computer_mut(g, &pas).set_is_targetable(false);
        computer_mut(g, &ism).set_is_targetable(false);
        computer_mut(g, &cm).set_is_targetable(false);
        computer_mut(g, &lan).set_is_targetable(false);

        for _ in 0..8 {
            let comp = WorldGenerator::generate_computer(g, companyname);
            computer_mut(g, &comp).set_is_targetable(false);
        }

        // Generate contact addresses for this company
        let personname = format!("internal@{}.net", companyname);
        let pas_ip = computer_ip(g, &pas);
        g.world_mut()
            .create_person(&personname, &pas_ip)
            .set_is_targetable(false);

        // Set up our share prices for the start
        Company::grow(g, companyname, 0);
    }

    pub fn initialise_andromeda(g: &mut Ctx) {
        // Add a single computer system owned by ARC
        let vl_ip = WorldGenerator::generate_location(g);

        let mut comp = Computer::new();
        let name = NameGenerator::generate_access_terminal_name("Andromeda");
        comp.set_name(&name);
        comp.set_company_name("ARC");
        comp.set_trace_speed(20);
        comp.set_trace_action(COMPUTER_TRACEACTION_DISCONNECT);
        comp.set_is_targetable(false);
        comp.set_ip(&vl_ip);

        // PORT: the C++ stored the computer first and then attached its screens through the pointer.
        // Here the screens are attached before the computer is stored; the end state is the same.
        let mut ms = MessageScreen::new();
        ms.base.set_main_title("Andromeda");
        ms.base.set_sub_title("");
        ms.set_text_message("\n\n\n\n\n                            THE TIME IS NEAR");
        ms.set_button_message(".");
        ms.set_next_page(1);
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms), 0);

        let mut ms1 = MenuScreen::new();
        ms1.base.set_main_title("Andromeda");
        ms1.base.set_sub_title("The time is near");
        ms1.add_option("Our beliefs", "Our beliefs", 3, 10, -1);
        ms1.add_option("Our people", "Our people", 2, 10, -1);
        ms1.add_option("Our support", "Our support", 4, 10, -1);
        comp.add_computer_screen(ComputerScreen::MenuScreen(ms1), 1);

        let mut ms2 = MessageScreen::new();
        ms2.base.set_main_title("Andromeda");
        ms2.base.set_sub_title("Our people");
        ms2.set_text_message(concat!(
            "Removed for security purposes\n",
            "effective from 12-02-2010",
        ));
        ms2.set_button_message(".");
        ms2.set_next_page(1);
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms2), 2);

        let mut ms3 = MessageScreen::new();
        ms3.base.set_main_title("Andromeda");
        ms3.base.set_sub_title("Our Beliefs");
        ms3.set_text_message(concat!(
            "THE ANDROMEDA FOUNDATION IS BUILT ON ITS CORE FIVE BELIEFS\n\n",
            "1. The Internet, once the saviour of free speech and anonymity, has been completely ",
            "taken over by the evils of capitalism.\n\n",
            "2. The common man has been reduced to a series of numbers and his life has been made an ",
            "item of public record.\n\n",
            "3. The Government is using the Internet to catalogue and control the population.\n\n",
            "4. The destruction of the Anonymity of the Internet has removed with it the last remnants ",
            "of Human creativity.\n\n",
            "5. Fundamentally, the wide-spread usage of the Internet has led to a new class divide - ",
            "Those that know, and those that do not.",
        ));
        ms3.set_button_message(".");
        ms3.set_next_page(1);
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms3), 3);

        let mut ms4 = MessageScreen::new();
        ms4.base.set_main_title("Andromeda");
        ms4.base.set_sub_title("Our Support");
        ms4.set_text_message(concat!(
            "It is heartening to know that we are not alone in our beliefs.\n\n",
            "The Andromeda group has the support of a number of high profile ",
            "people and organisations.\n\n",
            "This puts us in a situation where we can make a real difference.\n\n",
            "The time is near.",
        ));
        ms4.set_button_message(".");
        ms4.set_next_page(1);
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms4), 4);

        g.world_mut().create_computer_from(comp);
    }

    pub fn initialise_arunmor(g: &mut Ctx) {
        let companyname = "Arunmor";

        let mut company = Company::new();
        company.set_name(companyname);
        company.set_boss("Unlisted");
        company.set_size(50);
        company.set_type(COMPANYTYPE_COMMERCIAL);
        company.set_growth(10);
        company.set_alignment(0);
        g.world_mut().create_company_from(company);

        let pas = WorldGenerator::generate_public_access_server(g, companyname);
        let ism = WorldGenerator::generate_internal_services_machine(g, companyname);
        let cm = WorldGenerator::generate_central_mainframe(g, companyname);
        let lan = WorldGenerator::generate_lan(g, companyname).expect("plotgenerator: no Arunmor LAN");

        computer_mut(g, &cm)
            .get_computer_screen_mut(0)
            .and_then(|s| s.as_high_security_screen_mut())
            .expect("plotgenerator: Arunmor central mainframe screen 0 is not a HighSecurityScreen")
            .add_system("Elliptic-Curve Encryption Cypher", 7);

        let mut cs = CypherScreen::new();
        cs.base.set_main_title("Arunmor");
        cs.base.set_sub_title("Enter elliptic-curve encryption cypher");
        cs.set_difficulty(HACKDIFFICULTY_ARUNMORCENTRALMAINFRAME);
        cs.set_next_page(0);
        computer_mut(g, &cm).add_computer_screen(ComputerScreen::CypherScreen(cs), 7);

        let ceo = WorldGenerator::get_random_person(g);
        person_mut(g, &ceo).set_is_targetable(false);
        company_mut(g, companyname).set_boss(&ceo);

        let admin = WorldGenerator::get_random_person(g);
        person_mut(g, &admin).set_is_targetable(false);
        company_mut(g, companyname).set_admin(&admin);

        computer_mut(g, &pas).set_is_targetable(false);
        computer_mut(g, &ism).set_is_targetable(false);
        computer_mut(g, &cm).set_is_targetable(false);
        computer_mut(g, &lan).set_is_targetable(false);

        for _ in 0..8 {
            let comp = WorldGenerator::generate_computer(g, companyname);
            computer_mut(g, &comp).set_is_targetable(false);
        }

        // Generate contact addresses for this company
        let personname = format!("internal@{}.net", companyname);
        let pas_ip = computer_ip(g, &pas);
        g.world_mut()
            .create_person(&personname, &pas_ip)
            .set_is_targetable(false);

        // Set up our share prices for the start
        Company::grow(g, companyname, 0);
    }

    pub fn initialise_darwin(g: &mut Ctx) {
        let companyname = "Darwin Research Associates";

        let mut company = Company::new();
        company.set_name(companyname);
        company.set_size(20);
        company.set_type(COMPANYTYPE_ACADEMIC);
        company.set_growth(10);
        company.set_alignment(0);
        g.world_mut().create_company_from(company);

        let pas = WorldGenerator::generate_public_access_server(g, companyname);
        let ism = WorldGenerator::generate_internal_services_machine(g, companyname);
        let cm = WorldGenerator::generate_central_mainframe(g, companyname);
        let lan = WorldGenerator::generate_lan(g, companyname).expect("plotgenerator: no Darwin LAN");

        computer_mut(g, &pas).set_is_targetable(false);
        computer_mut(g, &ism).set_is_targetable(false);
        computer_mut(g, &cm).set_is_targetable(false);
        computer_mut(g, &lan).set_is_targetable(false);

        for _ in 0..3 {
            let comp = WorldGenerator::generate_computer(g, companyname);
            computer_mut(g, &comp).set_is_targetable(false);
        }

        let ceo = WorldGenerator::get_random_person(g);
        let admin = WorldGenerator::get_random_person(g);
        company_mut(g, companyname).set_boss(&ceo);
        company_mut(g, companyname).set_admin(&admin);

        // Generate contact addresses for this company
        let personname = format!("internal@{}.net", companyname);
        let pas_ip = computer_ip(g, &pas);
        g.world_mut()
            .create_person(&personname, &pas_ip)
            .set_is_targetable(false);

        Company::grow(g, companyname, 0);
    }

    /// Called by the scheduler
    pub fn run_scene(g: &mut Ctx, act: i32, scene: i32) {
        {
            let p = pg_mut(g);
            p.act = act;
            p.scene = scene;
        }

        match act {
            1 => match scene {
                1 => Self::run_act1scene1(g),
                2 => Self::run_act1scene2(g),
                3 => Self::run_act1scene3(g),
                4 => Self::run_act1scene4(g),
                5 => Self::run_act1scene5(g),
                7 => Self::run_act1scene7(g),
                8 => Self::run_act1scene8(g),
                _ => println!("Invalid Scene number: Act1 Scene{}", scene),
            },

            // The acts after 1 are not compiled into the demo build
            2 if !globals::DEMOGAME => match scene {
                1 => Self::run_act2scene1(g),
                2 => Self::run_act2scene2(g),
                3 => Self::run_act2scene3(g),
                _ => println!("Invalid Scene number: Act2 Scene{}", scene),
            },
            3 if !globals::DEMOGAME => match scene {
                1 => Self::run_act3scene1(g),
                2 => Self::run_act3scene2(g),
                3 => Self::run_act3scene3(g),
                4 => Self::run_act3scene4(g),
                _ => println!("Invalid Scene number: Act3 Scene{}", scene),
            },
            4 if !globals::DEMOGAME => match scene {
                1 => Self::run_act4scene1(g),
                2 => Self::run_act4scene2(g),
                3 => Self::run_act4scene3(g),
                4 => Self::run_act4scene4(g),
                5 => Self::run_act4scene5(g),
                6 => Self::run_act4scene6(g),
                7 => Self::run_act4scene7(g),
                8 => Self::run_act4scene8(g),
                9 => Self::run_act4scene9(g),
                10 => Self::run_act4scene10(g),
                11 => Self::run_act4scene11(g),
                12 => Self::run_act4scene12(g),
                13 => Self::run_act4scene13(g),
                14 => Self::run_act4scene14(g),
                15 => Self::run_act4scene15(g),
                _ => println!("Invalid scene number: Act4 scene{}", scene),
            },
            5 if !globals::DEMOGAME => match scene {
                1 => Self::run_act5scene1(g),
                2 => Self::run_act5scene2(g),
                3 => Self::run_act5scene3(g),
                4 => Self::run_act5scene4(g),
                5 => Self::run_act5scene5(g),
                6 => Self::run_act5scene6(g),
                7 => Self::run_act5scene7(g),
                _ => println!("Invalid Scene number: Act5 Scene{}", scene),
            },
            6 if !globals::DEMOGAME => match scene {
                1 => Self::run_act6scene1(g),
                2 => Self::run_act6scene2(g),
                3 => Self::run_act6scene3(g),
                _ => println!("Invalid Scene number: Act6 Scene{}", scene),
            },

            _ => println!("Invalid Act number: Act{} Scene{}", act, scene),
        }
    }

    // ---- scenes ----

    /// ARC begin work on Revelation
    pub fn run_act1scene1(g: &mut Ctx) {
        // Scene 2 is the first noticable scene
        // It should happen a few days after a player
        // starts a game

        pg_mut(g).version_revelation = 0.2;

        let mut rundate = Date::default();
        let (second, minute, hour, day, month, year) = GAME_START_DATE;
        rundate.set_date(second, minute, hour, day, month, year);
        rundate.advance_day(1);
        rundate.advance_hour(NumberGenerator::random_number(12));

        if globals::TESTGAME {
            let darwin = Self::generate_mission_darwin(g);
            Person::give_mission(g, "PLAYER", darwin);

            let mut data = Data::new();
            data.set_title("Revelation");
            data.set_details(DATATYPE_PROGRAM, 3, 0, 0, 1.0, SOFTWARETYPE_OTHER);
            computer_mut(g, "Uplink Test Machine").databank.put_data(data);

            let mut data2 = Data::new();
            data2.set_title("Faith");
            data2.set_details(DATATYPE_PROGRAM, 3, 0, 0, 2.0, SOFTWARETYPE_OTHER);
            computer_mut(g, "Uplink Test Machine").databank.put_data(data2);

            let mut data3 = Data::new();
            data3.set_title("Revelation_Tracker");
            data3.set_details(DATATYPE_PROGRAM, 3, 0, 0, 1.0, SOFTWARETYPE_OTHER);
            computer_mut(g, "Uplink Test Machine").databank.put_data(data3);
        }

        schedule_scene(g, &rundate, 1, 2);
    }

    /// ARC are hiring all top Uplink Agents - this makes the news
    pub fn run_act1scene2(g: &mut Ctx) {
        let mut news = News::new();
        news.set_headline("Unusual hiring practices noticed by analysts");
        news.set_details(concat!(
            "Andromeda Research Corporation, otherwise known as ARC, have been extremely busy lately.\n\n",
            "The company has been hiring all of the top Uplink Agents for the past couple of weeks, ",
            "sometimes posting several missions in one day.\n\n",
            "Details are thin but it appears that the work done is very vague in nature - ARC are ",
            "keeping quiet about the ultimate goal of this work and all of the Agents involved are ",
            "sworn to secrecy.  Analysts predict that ARC has spent over fifty-thousand credits in ",
            "the last week alone on agent fees.\n\n",
            "At the time of writing, ARC was unwilling to comment.",
        ));
        CompanyUplink::create_news(g, news);

        //
        // Select the two agents who will be our characters
        // for scenes 3 and 4
        //
        // The C++ walks people.ConvertToDArray (BTree pre-order); equal scores keep that order.
        let mut sorted: Vec<(String, i32)> = Vec::new();
        for (name, p) in g.world().people.iter() {
            if p.is_agent() && p.status == PERSON_STATUS_NONE {
                let score = p.rating.uplinkscore;
                let mut pos = sorted.len();
                for (is, s) in sorted.iter().enumerate() {
                    if score >= s.1 {
                        pos = is;
                        break;
                    }
                }
                sorted.insert(pos, (name.to_string(), score));
            }
        }

        let scene3agent = sorted
            .get(1)
            .expect("plotgenerator: fewer than two agents for Act1 Scene2")
            .0
            .clone();
        let scene4agent = sorted[0].0.clone();
        pg_mut(g).act1scene3agent = scene3agent;
        pg_mut(g).act1scene4agent = scene4agent.clone();

        //
        // Set up a log in on scene4 guys computer
        //

        let computername = NameGenerator::generate_personal_computer_name(&scene4agent);
        let comp_ip = computer_ip(g, &computername);

        if globals::TESTGAME {
            Person::give_link(g, "PLAYER", &comp_ip);
        }

        // The C++ deleted the existing screen 0 here; the AddComputerScreen below replaces it.
        let comp = computer_mut(g, &computername);
        let title = comp.name.clone();

        let mut ps0 = PasswordScreen::new();
        ps0.base.set_main_title(&title);
        ps0.base.set_sub_title("Speak friend, and enter");
        ps0.set_password("MySocratesNote");
        ps0.set_difficulty(50);
        ps0.set_next_page(1);
        comp.add_computer_screen(ComputerScreen::PasswordScreen(ps0), 0);

        let mut ms1 = MenuScreen::new();
        ms1.base.set_main_title(&title);
        ms1.base.set_sub_title("Menu");
        ms1.add_option("Filez", "Mess with my files", 2, 10, -1);
        ms1.add_option("eMailz", "Edit my mail settings", 3, 10, -1);
        comp.add_computer_screen(ComputerScreen::MenuScreen(ms1), 1);

        let mut gs2 = GenericScreen::new();
        gs2.base.set_main_title(&title);
        gs2.base.set_sub_title("Filez");
        gs2.set_screen_type(SCREEN_FILESERVERSCREEN);
        gs2.set_next_page(1);
        comp.add_computer_screen(ComputerScreen::GenericScreen(gs2), 2);

        let mut ds3 = DialogScreen::new();
        ds3.base.set_main_title(&title);
        ds3.base.set_sub_title("eMailz");
        ds3.add_widget_with_data(
            "close",
            WIDGET_NEXTPAGE,
            270,
            380,
            100,
            20,
            "Close",
            "Click to close this screen",
            1,
            -1,
            None,
            None,
        );
        comp.add_computer_screen(ComputerScreen::DialogScreen(ds3), 3);

        //
        // Act1Scene3 guy raises suspicion a few days later
        //

        let mut rundate = Date::default();
        rundate.set_date_from(&g.world().date);
        rundate.advance_day(4);
        rundate.advance_hour(NumberGenerator::random_number(12));
        schedule_scene(g, &rundate, 1, 3);
    }

    /// Act1Scene3Agent raises suspicion over ARCs activity
    pub fn run_act1scene3(g: &mut Ctx) {
        let agent_name = pg(g).act1scene3agent.clone();
        assert!(agent_name != " ", "plotgenerator: Act1 Scene3 agent not chosen");
        let handle = person_handle(g, &agent_name);

        let details = format!(
            concat!(
                "One of the top ranking Uplink Agents has broken the silence over the recent ",
                "activity of ARC (Andromeda Research Corporation) and raised doubts over the ",
                "morality of the work being done.\n\n",
                "The announcement comes less than a month after ARC began recruiting all the top ",
                "agents for work on some unspecified project.\n\n",
                "The man in question, known as Agent {}, is on one of the top ",
                "ranked Agents in the world and is well respected throughout the hacker community.\n\n",
                "He states that 'We don't know anything about this company ARC - some of the work ",
                "that i've seen done for them is highly suspicious and could be used in a very ",
                "destructive manner if properly exploited.  ARC are relying on the unofficial code of ",
                "honour of Uplink Agents - that they won't discuss what they were working on.  Someone ",
                "needs to come forward and explain what this company is up to.'\n\n",
                "Once again, ARC refused to comment, but did say that a press release was imminent.",
            ),
            handle
        );

        let mut news = News::new();
        news.set_headline("High ranking Agent raises doubts over ARC");
        news.set_details(&details);
        CompanyUplink::create_news(g, news);

        //
        // Act1Scene4 guy posts story a few days later
        //

        let mut rundate = Date::default();
        rundate.set_date_from(&g.world().date);
        rundate.advance_day(2);
        rundate.advance_hour(NumberGenerator::random_number(12));
        schedule_scene(g, &rundate, 1, 4);
    }

    /// Act1Scene4Agent posts story - "everything is ok with ARC"
    pub fn run_act1scene4(g: &mut Ctx) {
        let agent_name = pg(g).act1scene4agent.clone();
        assert!(agent_name != " ", "plotgenerator: Act1 Scene4 agent not chosen");
        let agent_handle = person_handle(g, &agent_name);

        let scene3_name = pg(g).act1scene3agent.clone();
        assert!(scene3_name != " ", "plotgenerator: Act1 Scene3 agent not chosen");
        let scene3_handle = person_handle(g, &scene3_name);

        pg_mut(g).version_revelation = 0.7;

        //
        // Create the news story
        //

        let details = format!(
            concat!(
                "In a suprising development in the mystery of Andromeda Research Corporation (ARC), ",
                "the number one rated Uplink Agent has come forward and tried to re-assure the community.\n\n",
                "This comes two days after Agent {} publicly expressed concern ",
                "that ARC were hiring agents to work on some kind of weapon.\n\n",
                "The Agent, known amoungst the hacker community as {}, says in his statement ",
                "that 'I have been working for ARC on a confidential project for the past two weeks.  The project ",
                "is immensly ambitious in nature but is nothing to worry about.  ARC have been keeping the public ",
                "in the dark to protect their trade secrets.  Uplink Agents should not be afraid of accepting jobs ",
                "from this company.'\n",
                "'Some people have said they think ARC are building some kind of military weapon - I can say that I ",
                "have had full access to the ARC Central Mainframe for the past week and there is nothing of the sort ",
                "on the system.  What they are doing is being kept under wraps but it is definately not a weapon.'\n\n",
                "Agent {} is a well respected hacker with years of experience and his comments ",
                "will certainly re-assure the community.",
            ),
            scene3_handle, agent_handle, agent_handle
        );

        let mut news = News::new();
        news.set_headline("Top Uplink Agent re-assures community");
        news.set_details(&details);
        CompanyUplink::create_news(g, news);

        //
        // Put some interesting logs on the ARC central mainframe
        //

        let cm = NameGenerator::generate_central_mainframe_name("ARC");
        let date = g.world().date.clone();
        for text in [
            "Deleted file RevelationCoreV0.7",
            "Deleted file RevelationDataV0.7",
            "Deleted file RevelationSpecV0.7",
        ] {
            let ip = WorldGenerator::get_random_location(g);
            let mut al = AccessLog::new();
            al.set_properties(&date, &ip, " ", LOG_NOTSUSPICIOUS, LOG_TYPE_TEXT);
            al.set_data1(Some(text));
            computer_mut(g, &cm).logbank.add_log(al, -1);
        }

        //
        // Act1Scene5 - the agent is found dead a few days later
        //

        let mut rundate = Date::default();
        rundate.set_date_from(&g.world().date);
        rundate.advance_day(7);
        rundate.advance_hour(NumberGenerator::random_number(12));
        schedule_scene(g, &rundate, 1, 5);
    }

    /// Act1Scene4Agent is found dead
    pub fn run_act1scene5(g: &mut Ctx) {
        let agent_name = pg(g).act1scene4agent.clone();
        assert!(agent_name != " ", "plotgenerator: Act1 Scene4 agent not chosen");
        let agent_handle = person_handle(g, &agent_name);

        let cm = NameGenerator::generate_central_mainframe_name("ARC");
        let cm_ip = computer_ip(g, &cm);

        //
        // This guy is now dead
        // His last action was to connect to the central mainframe and copy the virus
        // He also sets up the delayed email
        //

        Person::establish_connection(g, &agent_name, &cm_ip);

        let date = g.world().date.clone();
        let mut al = AccessLog::new();
        al.set_properties(&date, "unknown", "unknown", LOG_NOTSUSPICIOUS, LOG_TYPE_TEXT);
        al.set_data1(Some("Copied file RevelationCoreV0.7"));
        computer_mut(g, &cm).logbank.add_log(al, -1);

        let hiscomputername = NameGenerator::generate_personal_computer_name(&agent_name);

        let mut data = Data::new();
        data.set_title("RevelationCoreV0.7");
        data.set_details(DATATYPE_DATA, 4, 5, 0, 0.7, SOFTWARETYPE_NONE);
        let hiscomp = computer_mut(g, &hiscomputername);
        hiscomp.databank.set_size(hiscomp.databank.get_size() + 5);
        hiscomp.databank.put_data(data);

        let ds = dialog_screen_mut(hiscomp, 3);
        ds.add_widget("scheduletitle", WIDGET_CAPTION, 50, 150, 200, 20, "Scheduled emails", "");
        ds.add_widget(
            "email1",
            WIDGET_TEXTBOX,
            80,
            180,
            200,
            30,
            "TO: subscriptions@hackers.com\nSUBJECT: continuation of order",
            "",
        );
        ds.add_widget(
            "email2",
            WIDGET_TEXTBOX,
            80,
            220,
            200,
            30,
            "TO: all-agents@uplink.com\nSUBJECT: Warning about ARC",
            "",
        );
        ds.add_widget_with_data(
            "view1",
            WIDGET_SCRIPTBUTTON,
            280,
            180,
            50,
            14,
            "View",
            "Click to view this email",
            62,
            -1,
            None,
            None,
        );
        ds.add_widget_with_data(
            "view2",
            WIDGET_SCRIPTBUTTON,
            280,
            220,
            50,
            14,
            "View",
            "Click to view this email",
            63,
            -1,
            None,
            None,
        );
        ds.add_widget_with_data(
            "cancel1",
            WIDGET_SCRIPTBUTTON,
            280,
            195,
            50,
            14,
            "Cancel",
            "Click to stop this mail from being sent",
            60,
            -1,
            None,
            None,
        );
        ds.add_widget_with_data(
            "cancel2",
            WIDGET_SCRIPTBUTTON,
            280,
            235,
            50,
            14,
            "Cancel",
            "Click to stop this mail from being sent",
            61,
            -1,
            None,
            None,
        );
        ds.add_widget("mailtext", WIDGET_TEXTBOX, 30, 270, 400, 100, "", "");

        person_mut(g, &agent_name).set_status(PERSON_STATUS_DEAD);

        //
        // Generate the news story
        //

        let details = format!(
            concat!(
                "Agent {} has been found dead in his house after an apparent suicide.\n",
                "Preliminary forensic analysis suggests he overdosed on sleeping pills.\n\n",
                "He was at the top of the Agent leaderboard and made the news recently after commenting on the ",
                "actions of Andromeda Research Corporation (ARC).  He had been working for them on their latest ",
                "product which is currently shrowded in mystery.  This latest development will no doubt add to ",
                "the mystery surrounding the project.\n\n",
                "ARC released a statement earlier today, stating 'He was the best programmer we had ever seen and ",
                "was a lead player in the project.  For something like this happen is a tradgedy of the greatest order.  ",
                "Our thoughts go out to this mans family and friends.'",
            ),
            agent_handle
        );

        let mut news = News::new();
        news.set_headline("Top agent working for ARC found dead");
        news.set_details(&details);
        CompanyUplink::create_news(g, news);

        //
        // Act1Scene4 guys email will go out 5 days later -
        // The player has until then to figure out what to do
        // (If he's even listening to this)
        //

        let mut rundate = Date::default();
        rundate.set_date_from(&g.world().date);
        rundate.advance_day(5);
        rundate.advance_hour(NumberGenerator::random_number(12));
        schedule_scene(g, &rundate, 1, 7);
    }

    /// Act1Scebe4Agent sends out a "delayed" warning mail about ARC
    pub fn run_act1scene7(g: &mut Ctx) {
        if !pg(g).playercancelsmail {
            let agent_name = pg(g).act1scene4agent.clone();
            assert!(agent_name != " ", "plotgenerator: Act1 Scene4 agent not chosen");
            let handle = person_handle(g, &agent_name);

            // Look up the admin password ..........................
            let cm = NameGenerator::generate_central_mainframe_name("ARC");
            let cm_ip = computer_ip(g, &cm);

            let body = concat!(
                "[Autosend - 7 day delay expired]\n",
                "[Sent to ALL-AGENTS]\n\n",
                "Agents,\n",
                "If you are reading this email then it means I am dead.  I have been working for ",
                "Andromeda for the past month on a top secret project - and until yesterday I did not ",
                "even know what exactly I was coding.\n\n",
                "This story hit the news and I decided that I should try to find out what was happening ",
                "with project Revelation - and yesterday I managed to gain access to their stand-alone ",
                "server room and copy the files that we were working on.\n\n",
                "Unfortunately they saw what I was doing and I had to leave quick, and I haven't had time ",
                "to analyse the project files yet.  If you read this then they have already caught me.\n\n",
                "ARC cannot be trusted.  We must come together as a group and find out what they are up to ",
                "before something terrible happens.  I don't know if it's already too late.",
            );

            let from = format!("Agent {}", handle);

            // Include an access code, but it shouldn't work (they've disabled it by now, of course)
            let code = Computer::generate_access_code_name(&handle, "MySocratesNote");

            let mut msg = Message::new();
            msg.set_to("PLAYER");
            msg.set_from(&from);
            msg.set_subject("Warning about ARC");
            msg.set_body(body);
            msg.give_link(&cm_ip);
            msg.give_code(&cm_ip, &code);
            msg.send(g);

            // Make sure that mail doesn't appear on his computer any more
            let hiscomputername = NameGenerator::generate_personal_computer_name(&agent_name);
            let ds = dialog_screen_mut(computer_mut(g, &hiscomputername), 3);
            ds.remove_widget("email2");
            ds.remove_widget("view2");
            ds.remove_widget("cancel2");
        }

        //
        // Uplink issue a formal warning to all Agents
        //

        let mut rundate = Date::default();
        rundate.set_date_from(&g.world().date);
        rundate.advance_hour(NumberGenerator::random_number(12) + 2);
        schedule_scene(g, &rundate, 1, 8);
    }

    /// Uplink issue a warning to all agents
    pub fn run_act1scene8(g: &mut Ctx) {
        if !pg(g).playercancelsmail {
            let mut details = String::from(concat!(
                "Uplink Corporation has issued a formal warning to all of its agents, warning them ",
                "against accepting work for Andromeda Research Corporation (ARC).\n\n",
                "In a statement released today, they say 'Uplink Corporation is currently investigating ",
                "the claim that ARC have been developing some kind of high-tech weapon, using the ",
                "expertise of freelance Uplink agents to assist them.  Federal agents have been called in ",
                "to assist and are also persuing the matter of a recent suicide, which they believe may ",
                "have been a related murder.\n\n",
                "'Until we can determine if these charges have any truth in them, we must insist that all ",
                "Uplink Agents refuse any work being offered to them from ARC or anyone working for that company.  ",
                "Anyone caught working for this company will lose their Uplink status immediately.\n\n",
            ));
            if globals::DEMOGAME {
                details.push_str(concat!(
                    "'In addition, all news and data related to ARC will be classified and restricted to those agents ",
                    "with fully registered accounts.  Those agents using the Demo Uplink client will not be able to ",
                    "view any more of this information, in the interests of our own internal security.\n\n",
                    "'Any agents using the Demo Uplink client who wish to purchase the full client can do so by ",
                    "visiting www.introversion.co.uk'",
                ));
            }

            let mut news = News::new();
            news.set_headline("Formal warning issued by Uplink Corporation");
            news.set_details(&details);
            CompanyUplink::create_news(g, news);
        }

        //
        // "Maiden Flight" is given out a few days later
        //

        if !globals::DEMOGAME {
            let mut rundate = Date::default();
            rundate.set_date_from(&g.world().date);
            rundate.advance_day(3);
            rundate.advance_hour(NumberGenerator::random_number(12));
            schedule_scene(g, &rundate, 2, 1);
        }
    }

    /// The mission "Maiden Flight" is given out
    /// ARC begin work on Revelation
    pub fn run_act2scene1(g: &mut Ctx) {
        // ARC remove act1scene4agents computer
        // And replace it with a message screen
        let act1scene4agent = g.world().plotgenerator.act1scene4agent.clone();
        let hiscomputername = NameGenerator::generate_personal_computer_name(&act1scene4agent);
        let ip = g
            .world()
            .get_computer(&hiscomputername)
            .expect("Run_Act2Scene1: agent computer not found")
            .ip
            .clone();
        // PORT: tosser BTree::RemoveData, faithfully: it does nothing when the name sorts after
        // a node on its search path, and then the old computer stays (found first by name).
        g.world_mut().computers.remove_data(&hiscomputername);

        let mut comp = Computer::new();
        comp.set_type(COMPUTER_TYPE_PERSONALCOMPUTER);
        comp.set_name(&hiscomputername);
        comp.set_company_name(&act1scene4agent);
        comp.set_trace_speed(0);
        comp.set_trace_action(COMPUTER_TRACEACTION_DISCONNECT);
        comp.set_is_targetable(false);
        comp.set_ip(&ip);

        let mut ms0 = MessageScreen::new();
        ms0.base.set_main_title("404 error");
        ms0.base.set_sub_title("Computer not found");
        ms0.set_text_message(concat!(
            "This IP address is currently unused.\n",
            "If you would like to purchase it, please contact sales@internic.net."
        ));
        comp.add_computer_screen(ComputerScreen::MessageScreen(ms0), -1);

        g.world_mut().create_computer_from(comp);

        g.world_mut().plotgenerator.version_revelation = 1.0;

        // If the player found the Agents computer
        // Or if the player has revelation
        // Or if the player visited any of the important systems involved
        // then ARC will offer this mission to the player
        // Otherwise it will go to another high ranking agent.

        let has_revelation_core = g
            .player()
            .gateway
            .databank
            .get_data_by_title("RevelationCoreV0.7")
            .is_some();
        if has_revelation_core {
            let handle = g.player().agent().handle.clone();
            let body = format!(
                concat!(
                    "Agent {},\n",
                    "We couldn't help but notice that you have been poking into our ",
                    "business recently, and if our backup logs are correct, you are now ",
                    "in possesion of something that is very important to us.\n\n",
                    "Of course, you could make use of this data.  You could probably get ",
                    "quite a lot of money for it.  Or you could come and work for us.\n",
                    "You obviously have the talent.  We were amazed when we realised what you had done.\n\n",
                    "We believe you will find that we can offer huge rewards for successfull applicants.\n\n",
                    "Reply to this email (or send one to internal@ARC.net) if you are interested in discussing this further."
                ),
                handle
            );
            let mut msg = Message::new();
            msg.set_from("internal@ARC.net");
            msg.set_subject("An offer you can't refuse");
            msg.set_to("PLAYER");
            msg.set_body(&body);
            msg.send(g);
        } else if g.world().plotgenerator.playercancelsmail {
            let act1scene4_handle = g
                .world()
                .get_person(&act1scene4agent)
                .expect("Run_Act2Scene1: act1scene4 agent not found")
                .agent()
                .handle
                .clone();
            let handle = g.player().agent().handle.clone();
            let body = format!(
                concat!(
                    "Agent {},\n",
                    "We have been tidying up some of our loose ends recently and we ",
                    "couldn't help but notice that you seem to have done us a very ",
                    "large favour.  It appears that Agent {} was quite resourceful and would have sent a warning mail to all ",
                    "agents about us, almost from beyond the grave.\n\n",
                    "We didn't find out about this until it would have been too late, ",
                    "but we were very suprised to see that you stopped this from happening ",
                    "yourself.\n\n",
                    "We're not sure on your motives but would like to offer you some more ",
                    "work if you are interested.  We will reward you if you decide to accept ",
                    "of course.\n\n",
                    "Why are we offering this to you?  You managed to trace back to his ",
                    "computer system several days before any of our guys could get there.  ",
                    "You obviously have talent.  And your motives appear to be good as well.\n\n",
                    "Reply to this email (or send one to internal@ARC.net) if you are interested in discussing this further."
                ),
                handle, act1scene4_handle
            );
            let mut msg = Message::new();
            msg.set_from("internal@ARC.net");
            msg.set_subject("An offer you can't refuse");
            msg.set_to("PLAYER");
            msg.set_body(&body);
            msg.send(g);
        } else if g.world().plotgenerator.playervisitsplotsites {
            let handle = g.player().agent().handle.clone();
            let body = format!(
                concat!(
                    "Agent {},\n",
                    "We couldn't help but notice that you have recently been poking around in ",
                    "some of our computer systems, presumably attempting to determine what is ",
                    "happening with our company at the moment. We feel we should warn you at this ",
                    "time that any attempt to gain unauthorised access to our systems will result in ",
                    "SEVERE consequences.\n\n",
                    "Pleasantries aside, You may have heard that we have been hiring a number of Uplink ",
                    "Agents recently for a yet-to-be-announced project. Your recent activities caused us to ",
                    "run a background check on you, and you seem to be the kind of agent we are looking for.\n\n",
                    "If you would be interested in doing some work for us, we'd love to hear from you. Reply ",
                    "to this email or send one to internal@ARC.net, and we will see what we can do.  We think ",
                    "you will find that ARC offer the best rates for freelance agents in the business.\n\n",
                    "We look forward to hearing from you."
                ),
                handle
            );
            let mut msg = Message::new();
            msg.set_from("internal@ARC.net");
            msg.set_subject("An offer you can't refuse");
            msg.set_to("PLAYER");
            msg.set_body(&body);
            msg.send(g);
        }

        // Schedule act 2 scene 2 for 1 weeks time
        // At this time, another agent will do the mission
        // if the player hasn't already

        let mut rundate = Date::default();
        rundate.set_date_from(&g.world().date);
        rundate.advance_day(7);
        rundate.advance_hour(NumberGenerator::random_number(12));

        let mut rns = RunPlotSceneEvent::new();
        rns.set_act_and_scene(2, 2);
        rns.base.set_run_date(&rundate);
        g.world_mut()
            .scheduler
            .schedule_event(UplinkEvent::RunPlotSceneEvent(rns));
    }

    /// Another agent completes "Maiden Flight"
    /// Another agent completes "Maiden Flight"
    pub fn run_act2scene2(g: &mut Ctx) {
        // If the player hasn't done the Arunmor test mission,
        // Someone else will now do it.
        // If his loyalty is -1, he did the mission.
        // If it is 1, he betrayed ARC.
        // If it is 0, he has done neither so someone else should.

        if g.world().plotgenerator.playerloyalty == 0 {
            // Do the mission

            let comp_name = NameGenerator::generate_internal_services_server_name("Arunmor");
            let comp_ip = g
                .world()
                .get_computer(&comp_name)
                .expect("Run_Act2Scene2: Arunmor computer not found")
                .ip
                .clone();

            g.world_mut().plotgenerator.numuses_revelation += 1;
            PlotGenerator::run_revelation_with_success(g, &comp_ip, 1.0, false, 2);

            g.world_mut().plotgenerator.version_faith = 0.2;

            // Schedule the main news story for a few days from now

            let mut rundate = Date::default();
            rundate.set_date_from(&g.world().date);
            rundate.advance_hour(NumberGenerator::random_number(12));

            let mut rns = RunPlotSceneEvent::new();
            rns.set_act_and_scene(3, 1);
            rns.base.set_run_date(&rundate);
            g.world_mut()
                .scheduler
                .schedule_event(UplinkEvent::RunPlotSceneEvent(rns));
        }
    }

    /// Arunmor get in contact and try to get you to betray ARC
    /// Arunmor get in contact and try to get you to betray ARC
    pub fn run_act2scene3(g: &mut Ctx) {
        // Arunmor get in contact with the player
        // And try to convince him to defect to them
        // By sending them the Revelation Virus

        let handle = g.player().agent().handle.clone();
        let body = format!(
            concat!(
                "Agent {}\n\n",
                "Firstly let us congratulate you on scoring such a lucrative deal with ",
                "the Andromeda Research Corporation.  Secondly, let us warn you that you ",
                "are making a big mistake.\n\n",
                "You must have read the headlines, and by now you have probably guessed what ",
                "ARC are up to.  Our sources tell us you are about to test run ARCs secret ",
                "project on OUR computer systems.  We can only pray this message reaches you ",
                "before you go ahead.\n\n",
                "Whatever ARC have offered you, we can do better.  We have a vested interest ",
                "in stopping ARC, so we would like to offer you a counter-proposal.  Send us ",
                "a copy of whatever ARC sent you, and we'll reward you with {}c.  You can ",
                "work for us full time.\n\n",
                "Simply reply to this email with the ARC data attached.\n\n",
                "Before you answer, think about the moral implications of what you are about ",
                "to do.  Can you really trust ARC, who recently made the news by murdering ",
                "two of their best Uplink Agents?"
            ),
            handle, PAYMENT_SPECIALMISSION_BACKFIRE
        );

        let mut msg = Message::new();
        msg.set_to("PLAYER");
        msg.set_from("internal@Arunmor.net");
        msg.set_subject("Counter offer from a friend");
        msg.set_body(&body);
        msg.send(g);
    }

    /// News story about Revelation attack
    /// News story about Revelation attack
    pub fn run_act3scene1(g: &mut Ctx) {
        // Remove the ARC mission from the player

        PlotGenerator::player_fails_special_mission(g, SPECIALMISSION_MAIDENFLIGHT);

        if g.world().plotgenerator.playerloyalty == 1 {
            // The player betrayed ARC and went over to Arunmor

            let mut msg = Message::new();
            msg.set_to("PLAYER");
            msg.set_from("internal@ARC.net");
            msg.set_subject("Warning");
            msg.set_body(concat!(
                "So it seems that you were working for the enemy behind our backs all this time.  You cannot imagine ",
                "how suprised we were when we heard that Arunmor had received a complete copy of Revelation, and that ",
                "it was an Uplink Agent who was the source.  Unfortunately for you, you are the only Uplink Agent ",
                "who could have sent them this copy.  This makes you our new enemy.\n\n",
                "Take this message however you want - but Andromeda Research Corporation will not let this betrayal go ",
                "unpunished.  We'll be watching you, and when we finally come after you, you're going to wish you had ",
                "never heard of Arunmor Corporation or Uplink."
            ));
            msg.send(g);

            let mut news = News::new();
            news.set_headline("Arunmor Security scare - a special report");
            news.set_details(concat!(
                "Earlier today Arunmor Corporation shut down all of its primary computer systems and disconnected ",
                "their Internal Network from the web. The disconnection lasted just under one hour, and caused a great ",
                "deal of disruption to Internet traffic flowing through their systems.\n\n",
                "Arunmor says they had received word that they were about to be attacked by a group of hackers ",
                "working for the controversial Andromeda Research Corporation. So far Arunmor has not said why they suspect ",
                "this company. It is believed they had some kind of inside information.\n\n",
                "This latest accusation casts Andromeda Research Corporation in an increasingly bad light."
            ));
            CompanyUplink::create_news(g, news);
        } else {
            // The virus was launched

            let mut news = News::new();
            news.set_headline("The Arunmor virus outbreak - a special report");
            news.set_details(concat!(
                "Arunmor Corporation recently suffered an attack from a new computer virus, dubbed by ",
                "industry experts as 'Revelation'.\n\n",
                "This virus has never been seen before and contains a payload capable of destroying ",
                "almost any computer it is used against.  Currently the virus does not appear to spread ",
                "particularly fast and it is limited to systems owned by the target company, but it is believed ",
                "that whoever is responsible is probably still working on improving it.\n\n",
                "Recently a high ranking Uplink Agent contacted us and suggested that Andromeda Research Corporation ",
                "might be connected to this viral outbreak.  Agents have been assigned the task of investigating."
            ));
            CompanyUplink::create_news(g, news);
        }

        let mut rundate = Date::default();
        rundate.set_date_from(&g.world().date);
        rundate.advance_day(1);
        rundate.advance_hour(NumberGenerator::random_number(12));

        let mut rns = RunPlotSceneEvent::new();
        rns.set_act_and_scene(3, 2);
        rns.base.set_run_date(&rundate);
        g.world_mut()
            .scheduler
            .schedule_event(UplinkEvent::RunPlotSceneEvent(rns));
    }

    /// Arunmor begin work on Faith
    /// Arunmor begin work on Faith
    pub fn run_act3scene2(g: &mut Ctx) {
        let mut news = News::new();
        news.set_headline("Arunmor : we'll retaliate");
        news.set_details(concat!(
            "Following the recent attempted attack on the Arunmor computer network, ",
            "Arunmor have announced that they have begun work on a new project - dubbed 'Faith'. ",
            "The aim of this project is to stop the release of destructive computer viruses onto the Net.\n\n",
            "A company spokesman said 'We cannot allow any virus, including the so called Revelation virus, ",
            "to roam freely around our corporate networks.  Project Faith will actively seak out ",
            "Revelation and other destructive viruses, and will totally eradicate them.  At this moment ",
            "we are seeking Government assistance as this project could help everyone.  The recent ",
            "attempted attack on our network highlighted how weak Internet security is.  We intend to put a stop to this.'\n\n",
            "Arunmor has taken the unprecented step of offering a reward of 25000 credits to anyone who ",
            "can help them with their research into Revelation.  Anyone with any data related to this ",
            "virus should send it to internal@Arunmor.net to claim their reward."
        ));
        CompanyUplink::create_news(g, news);

        // Version info now becomes available at Uplink

        let mut news2 = News::new();
        news2.set_headline("New Uplink service available");
        news2.set_details(concat!(
            "In light of the recent developments related to Revelation and Faith, Uplink Corporation ",
            "has installed a new system which will allow agents to track the progress of the two rival ",
            "viruses.\n\nA new option has appeared in the main menu, titled 'Faith Progress'.  Clicking this ",
            "option will show you the current latest versions of both virusus.  This data is based on reports ",
            "of these programs on the Web, so they may not be entirely accurate.\n\n"
        ));
        CompanyUplink::create_news(g, news2);

        let comp_name = {
            let comp = g
                .world_mut()
                .get_computer_mut(NAME_UPLINKINTERNALSERVICES)
                .expect("Run_Act3Scene2: Uplink internal services computer not found");
            let comp_name = comp.name.clone();
            let menu2 = comp
                .screens
                .get_data_mut(1)
                .and_then(|s| s.as_menu_screen_mut())
                .expect("Run_Act3Scene2: menu screen 1 not found");
            menu2.add_option(
                "Faith Progress",
                "Shows the latest known versions of Faith and Revelation",
                3,
                10,
                5,
            );
            comp_name
        };

        let mut gs3 = GenericScreen::new();
        gs3.set_screen_type(SCREEN_FAITHSCREEN);
        gs3.base.set_main_title(&comp_name);
        gs3.base.set_sub_title("Faith V Revelation");
        gs3.set_next_page(1);
        g.world_mut()
            .get_computer_mut(NAME_UPLINKINTERNALSERVICES)
            .expect("Run_Act3Scene2: Uplink internal services computer not found")
            .add_computer_screen(ComputerScreen::GenericScreen(gs3), 3);

        let mut rundate = Date::default();
        rundate.set_date_from(&g.world().date);
        rundate.advance_day(2);
        rundate.advance_hour(NumberGenerator::random_number(12));

        let mut rns = RunPlotSceneEvent::new();
        rns.set_act_and_scene(3, 3);
        rns.base.set_run_date(&rundate);
        g.world_mut()
            .scheduler
            .schedule_event(UplinkEvent::RunPlotSceneEvent(rns));
    }

    /// Arunmor release details on Revelation, announce their plan
    /// Arunmor release details on Revelation, announce their plan
    pub fn run_act3scene3(g: &mut Ctx) {
        let mut details = String::new();
        details.push_str(concat!(
            "Arunmor Corporation have been working hard on researching the recently discovered ",
            "Revelation project, and have released details of their findings.\n\n",
            "'Revelation is the most destructive computer virus we have ever seen' said the CEO of ",
            "Arunmor Corporation earlier today.  'At first we thought it was targetted specifically at us. ",
            "It has now become clear that the aim of Revelation is to totally destroy the Internet, from the inside.\n\n",
            "'Once released, it will spead like wildfire to all the main systems, then eventually to the entire Internet, ",
            "destroying everything in its path.  Its payload is extremely well designed and will take down any system ",
            "it comes up against.'\n\n"
        ));

        if g.world().plotgenerator.playerloyalty == 1 {
            details.push_str("'We have hired a small number of capable Uplink Agents to help work against this.'\n\n");
        } else {
            details.push_str(concat!(
                "We will be posting missions on the Uplink BBS mission board.  Any agents that can help us will be ",
                "well rewarded, and will be fighting for a good cause.\n\n",
                "'The offer of 25000c for any data on related to Revelation is still open, and will remain open ",
                "for the next 48 hours.'\n\n"
            ));
        }

        details.push_str("'Our Faith project is now aiming to provide a counter-virus that will stop Revelation in its tracks.'\n");
        details.push_str("Federal Agents today began an investigation into the activities of Andromeda Research Corporation.");

        let mut news = News::new();
        news.set_headline("Arunmor release details on Revelation");
        news.set_details(&details);
        CompanyUplink::create_news(g, news);

        let mut rundate = Date::default();
        rundate.set_date_from(&g.world().date);
        rundate.advance_day(2);
        rundate.advance_hour(NumberGenerator::random_number(12));

        let mut rns = RunPlotSceneEvent::new();
        rns.set_act_and_scene(3, 4);
        rns.base.set_run_date(&rundate);
        g.world_mut()
            .scheduler
            .schedule_event(UplinkEvent::RunPlotSceneEvent(rns));
    }

    /// Feds reveal that ARC is run by Andromeda
    /// Feds reveal that ARC is run by Andromeda
    pub fn run_act3scene4(g: &mut Ctx) {
        let act1scene4agent = g.world().plotgenerator.act1scene4agent.clone();
        let details = format!(
            concat!(
                "Federal Agents today made their first public announcement on their investigation into Andromeda Research Corporation (ARC).\n\n",
                "'Our investigation so far has uncovered a number of disturbing facts, however no concrete evidence has yet been discovered' ",
                "said a Federal Spokesman. 'It would appear that ARC are developing a software tool in an attempt to damage the operation of the ",
                "Internet. So far no evidence has been found and no witnesses have come forward to back this up.'\n\n",
                "'It is our belief that ARC were responsible for the murder of {} and possibly one other, but until ",
                "we aquire evidence we will be unable to act in any legal way.'\n\n",
                "Most importantly, we have determined that Andromeda Research Corporation is privately owned and controlled by the secretive group 'Andromeda', ",
                "and that the CEO and the Managing Director of ARC are both core members of this group.  Our profile of Andromeda lists them as ",
                "well funded Anti-Capitalist Techno-Anarchists, which would seem to fit the charges against them.  So far, we have not had any success in finding this group.'\n\n",
                "'Their only real-world presence that we are aware of is their Public Access Terminal, but the registered contact details of that site are all fake.'\n\n",
                "'We intend to continue our investigation and appeal for anyone with any information to come forward.'"
            ),
            act1scene4agent
        );

        let mut news = News::new();
        news.set_headline("Federal Agents release findings on ARC");
        news.set_details(&details);
        CompanyUplink::create_news(g, news);

        let mut rundate = Date::default();
        rundate.set_date_from(&g.world().date);
        rundate.advance_day(2);
        rundate.advance_hour(NumberGenerator::random_number(12));

        let mut rns = RunPlotSceneEvent::new();
        rns.set_act_and_scene(4, 1);
        rns.base.set_run_date(&rundate);
        g.world_mut()
            .scheduler
            .schedule_event(UplinkEvent::RunPlotSceneEvent(rns));
    }

    /// "Tracer" is put out
    /// "Tracer" is put out
    pub fn run_act4scene1(g: &mut Ctx) {
        // Arunmor put out "Tracer"

        let tracer = PlotGenerator::generate_mission_tracer(g);

        if g.world().plotgenerator.playerloyalty == 1 {
            Person::give_mission(g, "PLAYER", tracer);
        } else {
            CompanyUplink::create_mission(g, tracer);

            // Make sure somebody does it eventually
            schedule_plot_scene(g, 4, 13, 7, 2);
        }

        schedule_plot_scene(g, 4, 7, 4, 5);
    }

    /// "TakeMeToYourLeader" is out
    /// "TakeMeToYourLeader" is out
    pub fn run_act4scene2(g: &mut Ctx) {
        // Arunmor put out "Take me to your leader"

        let takeme = PlotGenerator::generate_mission_take_me_to_your_leader(g);

        if g.world().plotgenerator.playerloyalty == 1 {
            Person::give_mission(g, "PLAYER", takeme);
        } else {
            CompanyUplink::create_mission(g, takeme);

            // Make sure somebody does it eventually
            schedule_plot_scene(g, 4, 14, 7, 2);
        }

        schedule_plot_scene(g, 4, 8, 4, 5);
    }

    /// "ARCInfiltration" is put out
    /// "ARCInfiltration" is put out
    pub fn run_act4scene3(g: &mut Ctx) {
        // Arunmor put out "ARC Infiltration"

        let mission = PlotGenerator::generate_mission_arcinfiltration(g);

        if g.world().plotgenerator.playerloyalty == 1 {
            Person::give_mission(g, "PLAYER", mission);
        } else {
            CompanyUplink::create_mission(g, mission);

            // Make sure somebody does it eventually
            schedule_plot_scene(g, 4, 15, 7, 2);
        }

        schedule_plot_scene(g, 4, 9, 4, 5);
    }

    /// News story about Tracer
    /// News story about Tracer
    pub fn run_act4scene4(g: &mut Ctx) {
        let mut news = News::new();
        news.set_headline("ARC Internal Services System attacked");
        news.set_details(concat!(
            "The Internal Services System run by Andromeda Research Corporation (ARC) has been ",
            "penetrated by an unknown hacker.  This did not come to light until a few hours ago, when ",
            "system administrators noticed a discrepancy in the access logs on the system.\n\n",
            "'This is the latest example of the kind of abuse our company is having to deal with' said ",
            "the public relations officer of ARC. 'Ever since these ridiculous allegations about us ",
            "developing some doomsday weapon, we have had serious problems attracting new business.  Now this.'\n\n",
            "It is not known what the hacker did - he was very careful and covered his tracks. ARC are ",
            "launching a full investigation into this incident."
        ));
        CompanyUplink::create_news(g, news);
    }

    /// News story about TakeMeToYourLeader
    /// News story about TakeMeToYourLeader
    pub fn run_act4scene5(g: &mut Ctx) {
        let mut news = News::new();
        news.set_headline("CEO of Andromeda Research Corporation arrested");
        news.set_details(concat!(
            "Following an outbreak of the Revelation computer virus on Arunmor Corporations Internal Services ",
            "System, the CEO of ARC has been arrested by Federal Agents and charged with the crime.  Federal ",
            "officials say they have been watching his company for some time, and have been waiting for ",
            "an opportunity to bring him in.\n\n",
            "Federal Agents have refused to comment on the results of their questioning at this time."
        ));
        CompanyUplink::create_news(g, news);
    }

    /// News story about ARCInfiltration
    /// News story about ARCInfiltration
    pub fn run_act4scene6(g: &mut Ctx) {
        let mut news = News::new();
        news.set_headline("ARC Central Mainframe compromised");
        news.set_details(concat!(
            "Industry experts have expressed suprise at the discovery that the ARC central Mainframe was ",
            "yesteday hacked wide open, and over 50 Gigaquads of data was stolen.  The system has come under ",
            "attack many times recently because of ARC's alleged involvement in the construction of some kind ",
            "of digital weapon, but it was previously believed to be inpenetrable.\n\n",
            "The ARC central mainframe stores most of its critical data on a stand-alone file server, which is ",
            "physically disconnected from the rest of the Internet.  This ingenious system has stopped all ",
            "previous attempts at access.\n\n",
            "'We now know that somebody inside our company betrayed us' stated ARCs public relations officer in ",
            "a press release this morning. 'We have found equipment which suggests somebody temporarily connected ",
            "the file server to the outside world with a remote link, which left the file server open to access ",
            "for a few minutes.  They must have been working with somebody on the outside to pull this off.'"
        ));
        CompanyUplink::create_news(g, news);
    }

    /// "Darwin" is put out
    /// "Darwin" is put out
    pub fn run_act4scene7(g: &mut Ctx) {
        // Arc put out the Darwin special mission

        if g.world().plotgenerator.playerloyalty == -1 {
            let darwin = PlotGenerator::generate_mission_darwin(g);
            Person::give_mission(g, "PLAYER", darwin);
        } else {
            // Another agent does the mission now

            g.world_mut().plotgenerator.completed_darwin = true;
            g.world_mut().plotgenerator.version_revelation += 2.0;

            // Schedule news story
            schedule_plot_scene(g, 4, 10, 0, 6);
        }

        // Next special mission shortly
        schedule_plot_scene(g, 4, 2, 4, 12);
    }

    /// "SaveItForTheJury" is put out
    /// "SaveItForTheJury" is put out
    pub fn run_act4scene8(g: &mut Ctx) {
        // ARC put out the SaveItForTheJury special mission

        if g.world().plotgenerator.playerloyalty == -1 {
            let saveit = PlotGenerator::generate_mission_save_it_for_the_jury(g);
            Person::give_mission(g, "PLAYER", saveit);
        } else {
            // Another agent does the mission now

            g.world_mut().plotgenerator.completed_saveitforthejury = true;
            let pg = &mut g.world_mut().plotgenerator;
            pg.version_faith -= 1.0;
            if pg.version_faith < 0.2 {
                pg.version_faith = 0.2;
            }

            schedule_plot_scene(g, 4, 11, 0, 3);
        }

        schedule_plot_scene(g, 4, 3, 4, 12);
    }

    /// "ShinyHammer" is put out
    /// "ShinyHammer" is put out
    pub fn run_act4scene9(g: &mut Ctx) {
        // ARC put out the ShinyHammer special mission

        if g.world().plotgenerator.playerloyalty == -1 {
            let shiny = PlotGenerator::generate_mission_shiny_hammer(g);
            Person::give_mission(g, "PLAYER", shiny);
        } else {
            // Another agent gets the mission

            g.world_mut().plotgenerator.completed_shinyhammer = true;
            let pg = &mut g.world_mut().plotgenerator;
            pg.version_faith -= 1.0;
            if pg.version_faith < 0.2 {
                pg.version_faith = 0.2;
            }

            schedule_plot_scene(g, 4, 12, 0, 3);
        }

        // Schedule the end game soon
        schedule_plot_scene(g, 5, 1, 10, 12);
    }

    /// News story about Darwin
    /// News story about Darwin
    pub fn run_act4scene10(g: &mut Ctx) {
        let mut news = News::new();
        news.set_headline("Digital Life research stolen");
        news.set_details(concat!(
            "Earlier today a hacker penetrated the Local Area Network of Darwin Research Associates ",
            "and stole Gigaquads of data on their latest project - Darwin Digital Life.  It is not ",
            "known exactly what the hackers wanted the data for, since its use is described as purely theoretical.\n\n",
            "'The idea behind Darwin Digital Life was to create a life form that could live and reproduce on the Internet' ",
            "said the CEO of Darwin Research Associates recently.  'Obviously we are gutted that somebody has stolen what ",
            "amounts to years of research into this area.'\n\n",
            "A suspect has been named to Federal Agents, but so far no arrests have been made.  Darwin Research Associates ",
            "earlier today stated that an Uplink Agent was responsible, and that the Agent in question was working for ",
            "Andromeda Research Corporation (ARC) at the time.\n\n",
            "Once again, ARC was unwilling to comment on the issue, but this latest allegation has fueled speculation as to ",
            "the ultimate motives of the company."
        ));
        CompanyUplink::create_news(g, news);
    }

    /// News story about SaveItForTheJury
    /// News story about SaveItForTheJury
    pub fn run_act4scene11(g: &mut Ctx) {
        let mut news = News::new();
        news.set_headline("Arunmor Corporation in Financial Fraud");
        news.set_details(concat!(
            "The lead Technical director of Arunmor Corporation has been arrested by Federal Agents, ",
            "following allegations that he has been caught attempting bank fraud.  This latest development ",
            "in the Arunmor Versus ARC story has called into question the true motives behind Arunmor Corporation.\n\n",
            "Federal Agents arrested the technical director in a dawn raid, after an anonymous tip led them to discover ",
            "evidence which immediately incriminated him.\n\n",
            "So far he has protested his innocence, claiming he has been framed.\n\n",
            "This arrest will come as a severe blow to Arunmor Corporation, who relied on this man for most of their ",
            "technical leadership.  Their so called 'Faith Project' will have been set back.\n\n"
        ));
        CompanyUplink::create_news(g, news);
    }

    /// News story about ShinyHammer
    /// News story about ShinyHammer
    pub fn run_act4scene12(g: &mut Ctx) {
        let mut news = News::new();
        news.set_headline("Arunmor LAN system suffers crippling blow");
        news.set_details(concat!(
            "Arunmor Corporation today released a press statememt that said their primary computer system ",
            "had been severely damaged by the acts of a single hacker.\n\n",
            "It appears that most of their research on that system has been destroyed, as well as the computer ",
            "system itself.  Arunmor are said to be devastated by this critial error of judgement.\n\n",
            "Arunmor today claimed Andromeda Research Corporation was again responsible for this attack.\n\n",
            "It is not known what effect this will have on project Faith, but it is known that the Arunmor Local ",
            "Area Network was used to store a great deal of research on that topic.\n\n"
        ));
        CompanyUplink::create_news(g, news);
    }

    /// Somebody else attempts "Tracer"
    /// Somebody else attempts "Tracer"
    pub fn run_act4scene13(g: &mut Ctx) {
        // Is the Tracer mission still available?

        let removed = PlotGenerator::remove_special_mission(g, SPECIALMISSION_TRACER);

        if removed {
            schedule_plot_scene(g, 4, 4, 0, 6);

            g.world_mut().plotgenerator.version_faith += 1.0;
            g.world_mut().plotgenerator.completed_tracer = true;
        }
    }

    /// Somebody else attempts "TakeMeToYourLeader"
    /// Somebody else attempts "TakeMeToYourLeader"
    pub fn run_act4scene14(g: &mut Ctx) {
        // Is the TakeMeToYourLeader mission still available?

        let removed = PlotGenerator::remove_special_mission(g, SPECIALMISSION_TAKEMETOYOURLEADER);

        if removed {
            schedule_plot_scene(g, 4, 5, 0, 6);

            g.world_mut().plotgenerator.version_faith += 1.0;
            g.world_mut().plotgenerator.completed_takemetoyourleader = true;
        }
    }

    /// Somebody else attempts "ARC Infiltration"
    /// Somebody else attempts "ARC Infiltration"
    pub fn run_act4scene15(g: &mut Ctx) {
        // Is the ARC Infiltration mission still available?

        let removed = PlotGenerator::remove_special_mission(g, SPECIALMISSION_ARCINFILTRATION);

        if removed {
            schedule_plot_scene(g, 4, 6, 0, 6);

            g.world_mut().plotgenerator.version_faith += 2.0;
            g.world_mut().plotgenerator.completed_arcinfiltration = true;
        }
    }

    /// Arunmor warn everyone that virus release is imminent
    /// Arunmor warn everyone that virus release is imminent
    pub fn run_act5scene1(g: &mut Ctx) {
        let mut news = News::new();
        news.set_headline("Revelation Launch due soon?");
        news.set_details(concat!(
            "Industry experts are predicting that a launch of the destructive Revelation virus ",
            "may be due soon. If true, this could mean damage to a great deal of computer systems.\n\n",
            "Federal agents have been waiting for an opportunity to enter Andromeda Research Corporation ",
            "but have so far not had enough evidence against the company.\n\n",
            "Recent events such as the arrest of a high ranked member of the company and the theft of ",
            "program code from Darwin Research Associates has led industry experts to speculate that now ",
            "would be the perfect time for a launch of the virus.\n\n",
            "Arunmor corporation has been working hard and have a prototype of the counter virus Faith, ",
            "but it is not clear how effective this would be."
        ));
        CompanyUplink::create_news(g, news);

        schedule_plot_scene(g, 5, 2, 0, 6);
    }

    /// "TheGrandTour" and "CounterAttack" are put out
    /// "TheGrandTour" and "CounterAttack" are put out
    pub fn run_act5scene2(g: &mut Ctx) {
        // Player has had long enough to do the special missions
        // Remove any still pending

        PlotGenerator::player_fails_special_mission(g, SPECIALMISSION_TRACER);
        PlotGenerator::player_fails_special_mission(g, SPECIALMISSION_TAKEMETOYOURLEADER);
        PlotGenerator::player_fails_special_mission(g, SPECIALMISSION_ARCINFILTRATION);
        PlotGenerator::player_fails_special_mission(g, SPECIALMISSION_DARWIN);
        PlotGenerator::player_fails_special_mission(g, SPECIALMISSION_SAVEITFORTHEJURY);
        PlotGenerator::player_fails_special_mission(g, SPECIALMISSION_SHINYHAMMER);

        let playerloyalty = g.world().plotgenerator.playerloyalty;

        if playerloyalty == -1 {
            // Send the player a Tracker now

            let mut data = Data::new();
            data.set_title("Revelation_Tracker");
            data.set_details(DATATYPE_PROGRAM, 3, 0, 0, 1.0, SOFTWARETYPE_OTHER);

            let mut msg = Message::new();
            msg.set_to("PLAYER");
            msg.set_from("internal@ARC.net");
            msg.set_subject("Revelation Tracker");
            msg.set_body(concat!(
                "This message includes our Revelation Tracker Software. This will tell you ",
                "how many systems have been infected with the Virus.\n",
                "You will need this for your next mission."
            ));
            msg.attach_data(&data);
            msg.send(g);

            // Send the player Revelation

            let version_revelation = g.world().plotgenerator.version_revelation;
            let mut data2 = Data::new();
            data2.set_title("Revelation");
            data2.set_details(DATATYPE_PROGRAM, 3, 0, 0, version_revelation, SOFTWARETYPE_OTHER);

            let mut msg2 = Message::new();
            msg2.set_to("PLAYER");
            msg2.set_from("internal@ARC.net");
            msg2.set_subject("Revelation");
            msg2.set_body(concat!(
                "This message includes a copy of the Revelation virus.\n",
                "You will need this file for your next mission.\n\n",
                "DO NOT ATTEMPT TO RUN IT LOCALLY"
            ));
            msg2.attach_data(&data2);
            msg2.send(g);

            // The player is asked to do "Grand Tour"

            let mission = PlotGenerator::generate_mission_grand_tour(g);
            Person::give_mission(g, "PLAYER", mission);

            set_normal_speed_and_dirty_buttons(g);

            // Start the AI going

            let version_faith = g.world().plotgenerator.version_faith;
            let version_revelation = g.world().plotgenerator.version_revelation;
            let timebetweenlaunches = (3.0 - (version_faith - version_revelation)) as i32;

            let mut rundate = Date::default();
            rundate.set_date_from(&g.world().date);
            rundate.advance_minute(timebetweenlaunches);
            schedule_plot_scene_on(g, &rundate, 5, 5);

            // Schedule the downfall of ARC for a few minutes

            let mut rundate2 = Date::default();
            rundate2.set_date_from(&g.world().date);
            rundate2.advance_minute(TIME_ARCBUSTED_WITHPLAYER);
            schedule_plot_scene_on(g, &rundate2, 5, 6);
        } else if playerloyalty == 1 {
            // The player is asked to do "Counter attack"

            let mission = PlotGenerator::generate_mission_counter_attack(g);
            Person::give_mission(g, "PLAYER", mission);

            set_normal_speed_and_dirty_buttons(g);

            // Start the AI going

            for _ in 0..3 {
                let comp_name = WorldGenerator::get_random_computer_of_type(
                    g,
                    COMPUTER_TYPE_INTERNALSERVICESMACHINE | COMPUTER_TYPE_CENTRALMAINFRAME,
                );
                let comp_ip = g
                    .world()
                    .get_computer(&comp_name)
                    .expect("Run_Act5Scene2: random computer not found")
                    .ip
                    .clone();
                let version_revelation = g.world().plotgenerator.version_revelation;
                PlotGenerator::run_revelation_with_success(g, &comp_ip, version_revelation, false, 2);
            }

            let version_faith = g.world().plotgenerator.version_faith;
            let version_revelation = g.world().plotgenerator.version_revelation;
            let timebetweenlaunches = (3.0 - (version_revelation - version_faith)) as i32;

            let mut rundate = Date::default();
            rundate.set_date_from(&g.world().date);
            rundate.advance_minute(timebetweenlaunches);
            schedule_plot_scene_on(g, &rundate, 5, 5);

            // Schedule the downfall of ARC for a few minutes

            let mut rundate2 = Date::default();
            rundate2.set_date_from(&g.world().date);
            rundate2.advance_minute(TIME_ARCBUSTED_WITHOUTPLAYER);
            schedule_plot_scene_on(g, &rundate2, 5, 6);
        } else {
            // Grand Tour and Counter attack are done by other people

            schedule_plot_scene(g, 5, 4, 0, 2);
        }
    }

    /// Player works for Arunmor, successful with Faith
    /// Player works for Arunmor, successful with Faith
    pub fn run_act5scene3(g: &mut Ctx) {
        let mut news = News::new();
        news.set_headline("Revelation Virus Release stopped in its path");
        news.set_details(concat!(
            "Within the last couple of hours the Revelation virus was released onto the Internet ",
            "by operatives at the controversial Andromeda Research Corporation. Experts within the ",
            "industry had predicted this was about to happen, but were unprepared for the scale of the attack.\n\n",
            "However, Arunmor Corporation have been developing a counter virus known as Faith for the past few ",
            "weeks with the aid of Government assistance, and have managed to stop the Revelation Virus from ",
            "spreading to too many systems.\n\n",
            "Federal Agents busted into Andromeda Research Corporation shortly after the virus outbreak and ",
            "arrested staff members suspected of involvement.  A number of computer systems were also siezed. ",
            "This co-incided with a world wide effort to take down this rogue group within Andromeda Research ",
            "Corporation. A number of Uplink agents were also arrested in connection with the outbreak.\n\n",
            "It is not known what will become of Andromeda, but it is clear that many of the company executives ",
            "have come to the end of their career at that company.\n\n",
            "Federal Agents refused to comment at this time, but did say a statement would be released soon."
        ));
        CompanyUplink::create_news(g, news);

        schedule_plot_scene(g, 6, 1, 2, 12);
    }

    /// Player works for no-one
    /// Player works for no-one
    pub fn run_act5scene4(g: &mut Ctx) {
        let mut news = News::new();
        news.set_headline("Revelation virus outbreak leaves systems crippled");
        news.set_details(concat!(
            "The Revelation computer virus has been released into the wild and has spread rapidly through ",
            "major computer systems around the world.  A massive amount of damage has been done, and reports ",
            "are still coming in from systems that have been literally wiped clean by the virus.\n\n",
            "However, Arunmor Corporation have been developing a counter virus known as Faith for the past few ",
            "weeks with the aid of Government assistance, and have managed to stop the Revelation Virus from ",
            "spreading any further.\n\n",
            "Federal Agents busted into Andromeda Research Corporation shortly after the virus outbreak and ",
            "arrested staff members suspected of involvement.  A number of computer systems were also siezed. ",
            "This co-incided with a world wide effort to take down this rogue group within Andromeda Research ",
            "Corporation. A number of Uplink agents were also arrested in connection with the outbreak.\n\n",
            "It is not known what will become of Andromeda, but it is clear that many of the company executives ",
            "have come to the end of their career at that company.\n\n",
            "Federal Agents refused to comment at this time, but did say a statement would be released soon."
        ));
        CompanyUplink::create_news(g, news);

        schedule_plot_scene(g, 6, 1, 2, 12);
    }

    /// AI launches Revelation/Faith at a random system
    /// AI launches Revelation/Faith at a random system
    pub fn run_act5scene5(g: &mut Ctx) {
        if !g.world().plotgenerator.revelation_releasefailed {
            if g.world().plotgenerator.playerloyalty == 1 {
                // Release Revelation at a random spot

                let comp_name = WorldGenerator::get_random_computer_of_type(
                    g,
                    COMPUTER_TYPE_INTERNALSERVICESMACHINE | COMPUTER_TYPE_CENTRALMAINFRAME,
                );
                let comp_ip = g
                    .world()
                    .get_computer(&comp_name)
                    .expect("Run_Act5Scene5: random computer not found")
                    .ip
                    .clone();
                let version_revelation = g.world().plotgenerator.version_revelation;
                PlotGenerator::run_revelation_with_success(g, &comp_ip, version_revelation, false, 2);

                let version_faith = g.world().plotgenerator.version_faith;
                let version_revelation = g.world().plotgenerator.version_revelation;
                let timebetweenlaunches = (3.0 - (version_revelation - version_faith)) as i32;

                let mut rundate = Date::default();
                rundate.set_date_from(&g.world().date);
                rundate.advance_minute(timebetweenlaunches);
                schedule_plot_scene_on(g, &rundate, 5, 5);
            } else if g.world().plotgenerator.playerloyalty == -1 {
                // Run Faith on an infected system

                let infected_count = g.world().plotgenerator.infected.len();
                if infected_count > 0 {
                    let pos = NumberGenerator::random_number(infected_count as i32) as usize;
                    let ip = g.world().plotgenerator.infected[pos].clone();
                    let version_faith = g.world().plotgenerator.version_faith;
                    PlotGenerator::run_faith(g, &ip, version_faith, false);
                }

                let version_faith = g.world().plotgenerator.version_faith;
                let version_revelation = g.world().plotgenerator.version_revelation;
                let timebetweenlaunches = (3.0 - (version_faith - version_revelation)) as i32;

                let mut rundate = Date::default();
                rundate.set_date_from(&g.world().date);
                rundate.advance_minute(timebetweenlaunches);
                schedule_plot_scene_on(g, &rundate, 5, 5);
            }
        } else {
            g.world_mut().plotgenerator.revelation_releasefailed = false;
        }
    }

    /// ARC is busted
    /// ARC is busted
    pub fn run_act5scene6(g: &mut Ctx) {
        // If it gets this far, Revelation has failed
        // The player has either won or lost

        g.world_mut().plotgenerator.revelation_releasefailed = true;
        g.world_mut().plotgenerator.revelation_arcbusted = true;

        let playerloyalty = g.world().plotgenerator.playerloyalty;
        if playerloyalty == -1 {
            g.game.game_over(concat!(
                "For working for Andromeda Research Corporation\n",
                "during the Revelation crisis"
            ));
        } else if playerloyalty == 1 {
            // Send him an email about ARC getting busted

            let mut msg = Message::new();
            msg.set_to("PLAYER");
            msg.set_from("internal@Arunmor.net");
            msg.set_subject("Arunmor has been busted");
            msg.set_body(concat!(
                "We know you're busy, so we'll make this brief.\n\n",
                "We've just been informed that Federal Agents have broken into ",
                "ARCs headquarters and have arrested the staff and seized the ",
                "equipment there.\n\n",
                "This means that no further outbreaks of Revelation will occur.\n\n",
                "If you can stop the current outbreak from spreading, we will have ",
                "beaten them. This is our best chance to put an end to the ",
                "Revelation threat once and for all.\n\n",
                "Do your best and hurry.\n\n",
                "END"
            ));
            msg.send(g);
        }
    }

    /// Revelation Release Uncontrolled - GAME OVER
    /// Revelation Release Uncontrolled - GAME OVER
    pub fn run_act5scene7(g: &mut Ctx) {
        g.world_mut().plotgenerator.revelation_releaseuncontrolled = true;

        PlotGenerator::player_completes_special_mission(g, SPECIALMISSION_GRANDTOUR);

        g.game.game_over(concat!(
            "An outbreak of the Revelation virus forced\n",
            "Uplink Corporation to cease operations"
        ));
    }

    /// Leader of Andromeda releases press statement
    /// Leader of Andromeda releases press statement
    pub fn run_act6scene1(g: &mut Ctx) {
        let mut news = News::new();
        news.set_headline("First press release from Andromeda Leader");
        news.set_details(concat!(
            "The Leader of the Andromeda group today issued a statement through his lawyer, regarding his recent arrest ",
            "for criminal damage to the Web. In it he outlines why he tried to destroy the Internet that we all rely on.\n\n",
            "'Firstly, I make no apologies for what I have done. My only regret is that I did not succeed' says the man, who ",
            "has not yet been identified by Federal Agents.\n",
            "'The Internet used to be a place of amazing opportunity and promise - the last remnant of free speech and ",
            "true anonymity.  Now it is little more than an extension of the evil of Western Capitalism.'\n\n",
            "'Think of this.  Do you have any idea how much money Arunmor will make from recent events?  This is why I tried ",
            "to destroy the Internet.  It only serves to line the pockets of those who would try to control us.'\n\n",
            "'While the Web exists, you will never be safe. Your entire life is on file, waiting for somebody to tamper with it.  Your lives ",
            "are being destroyed by the Computers you desperately try to defend.'\n\n",
            "It is believed this man will spend several years in jail for this crime."
        ));
        CompanyUplink::create_news(g, news);

        schedule_plot_scene(g, 6, 2, 7, 12);
    }

    /// Leaders of ARC sentenced to jail
    /// Leaders of ARC sentenced to jail
    pub fn run_act6scene2(g: &mut Ctx) {
        let mut news = News::new();
        news.set_headline("Leader of Andromeda jailed");
        news.set_details(concat!(
            "The Federal courts today found the CEO of ARC and the leader of Andromeda Guilty of criminal destruction of data. \n",
            "The judge called the Leader of Andromeda 'an evil and vindictive man, who truly believes the world would be a better place in a state of Anarchy.'\n\n",
            "He went on to say that he had little sympathy for both the defendants.\n\n",
            "As punishment for their crimes, both members will serve eight years imprisonment and will be banned from the use of computers for life.\n\n"
        ));
        CompanyUplink::create_news(g, news);

        schedule_plot_scene(g, 6, 3, 4, 12);
    }

    /// Profits of Arunmor soar high
    /// Profits of Arunmor soar high
    pub fn run_act6scene3(g: &mut Ctx) {
        let mut news = News::new();
        news.set_headline("Revelation : A retrospective");
        news.set_details(concat!(
            "It would appear that the recent panic over Revelation was (as usual) totally unnecessary.  The Internet has suffered a number of ",
            "high profile scares since its foundation, and it has always survived.  This incident was no different, and only serves to highlight the ",
            "ease with which the public can be scared into thinking their lives are in danger.\n\n",
            "A special help line was recently set up by Federal Agents for worried members of the public, and they have so far received an average of ",
            "ten-thousand calls a day from people who are terrified their life records are now wide open due to this recent security scare.  'They just ",
            "keep calling' said a Federal Spokesman today.  'These people honestly believe that somebody could just open up their records and change ",
            "them as if they were in the public library.  Its total hysteria.'\n\n",
            "Since the collapse of the Andromeda Group, ARC has been exposed as little more than a front for their activities, and has shut down all operations. ",
            "It is a wonder that nobody noticed ARC never released any products.\n\n",
            "Arunmor Corporation yesterday launched their new Active-Virus-Guard software, titled 'Faith'.  This is an extension of the tool used to defeat ",
            "Revelation, and the makers claim it actively seeks out viruses and eradicates them.  Sales have apparently been 'very promising'.\n\n",
            "At the time of writing, shares in Arunmor Corporation had risen to a new high."
        ));
        CompanyUplink::create_news(g, news);

        // This is the end of the plot
        // A tear is forming in my eye ;|
    }

    // ---- special mission generators (PORT: return the Mission; the caller posts it) ----

    /// Arunmor #2
    /// Arunmor #2
    pub fn generate_mission_tracer(g: &mut Ctx) -> Mission {
        //
        // Generate our own fileserver

        let our_name = WorldGenerator::generate_empty_file_server(g, "Arunmor");

        // Put the tracer software on our own file server
        //

        let mut data = Data::new();
        data.set_title("RevelationTracer");
        data.set_details(DATATYPE_PROGRAM, 3, 0, 0, 1.0, SOFTWARETYPE_NONE);
        g.world_mut()
            .get_computer_mut(&our_name)
            .expect("generated file server must exist")
            .databank
            .put_data(data);

        //
        // Add in a new account for the player to use
        //

        let password = "tracer";
        let mut username = String::from("temp");
        for _ in 0..4 {
            username.push((b'a' + NumberGenerator::random_number(26) as u8) as char);
        }

        let mut record = Record::new();
        record.add_field(RECORDBANK_NAME, &username);
        record.add_field(RECORDBANK_PASSWORD, password);
        record.add_field(RECORDBANK_SECURITY, "3");
        g.world_mut()
            .get_computer_mut(&our_name)
            .expect("generated file server must exist")
            .recordbank
            .add_record(record);

        let code = Computer::generate_access_code_name(&username, password);

        // Find the target computer

        let comp = g
            .world()
            .get_computer(&NameGenerator::generate_internal_services_server_name("ARC"))
            .expect("ARC internal services must exist");
        let comp_name = comp.name.clone();
        let comp_ip = comp.ip.clone();
        let our_ip = g
            .world()
            .get_computer(&our_name)
            .expect("generated file server must exist")
            .ip
            .clone();

        let fulldetails = format!(
            concat!(
                "One of our sources within Andromeda Research Corporation has discovered that ARC ",
                "are planning on launching Revelation from three primary systems. When the time is right, ",
                "their Agents will enter these systems and begin running Revelation.\n\n",
                "We already have two of the systems covered, but the third is wide open.  We need you to ",
                "get inside that system, and install a Tracer program covertly which will send us routine ",
                "updates, and will give us early warning when Revelation is launched.\n\n",
                "First, you'll need to download the program from our File Server.  Addresses and log-in details ",
                "are shown below.\n\n",
                "Second, break into the target system (ARC Internal Services) and compromise all their security ",
                "systems.  This includes Proxies, Monitors, Firewalls etc.  You have to stop them from seeing what ",
                "you are doing.\n\n",
                "Copy the Tracer into place, then drop into a console.  You can run the tracer from there.  Once it is ",
                "running it will immediately start hiding itself. Check you can't see it from the File Server screen - this ",
                "will ensure nobody else can see it.\n\n",
                "Cover your tracks and get out.  Reply to this email when you are done.\n\n",
                "Codename: Tracer\n\n",
                "Payment will be {}\n\n",
                "Our FileServer: {}\n",
                "Target system: {}\n",
                "Target IP: {}"
            ),
            PAYMENT_SPECIALMISSION_TRACER, our_ip, comp_name, comp_ip
        );

        let details = format!(
            concat!(
                "Payment for this job is {} credits.\n",
                "This job has been assigned an Uplink difficulty of {}.\n",
                "This mission cannot be auto-accepted."
            ),
            PAYMENT_SPECIALMISSION_TRACER, 10
        );

        let mut m = Mission::new();
        m.set_type(MISSION_SPECIAL);
        m.set_employer("Arunmor");
        m.set_contact("internal@Arunmor.net");
        m.set_payment(
            PAYMENT_SPECIALMISSION_TRACER,
            (PAYMENT_SPECIALMISSION_TRACER as f64 * 1.1) as i32,
        );
        m.set_difficulty(10);
        m.set_min_rating(10);
        m.set_accept_rating(99);
        m.set_description(Self::special_mission_description(SPECIALMISSION_TRACER));
        m.set_details(&details);
        m.set_full_details(&fulldetails);
        m.set_why_so_much_money(Some("The mission is extremely important."));
        m.set_how_secure(Some("The target will be very secure."));
        m.set_who_is_the_target(Some("Andromeda Research Corporation."));
        m.give_link(&comp_ip);
        m.give_link(&our_ip);
        m.give_code(&our_ip, &code);
        m
    }

    /// Arunmor #3
    /// Arunmor #3
    pub fn generate_mission_take_me_to_your_leader(g: &mut Ctx) -> Mission {
        //
        // Look up the target systen

        let target = g
            .world()
            .get_computer(&NameGenerator::generate_internal_services_server_name("Arunmor"))
            .expect("Arunmor internal services must exist");
        let target_name = target.name.clone();
        let target_ip = target.ip.clone();

        //
        // Put Revelation on to our file server

        let our_name = NameGenerator::generate_file_server_name("Arunmor");
        let mut data = Data::new();
        data.set_title("Revelation");
        data.set_details(DATATYPE_PROGRAM, 3, 0, 0, 1.0, SOFTWARETYPE_NONE);
        g.world_mut()
            .get_computer_mut(&our_name)
            .expect("Arunmor file server must exist")
            .databank
            .put_data(data);

        //
        // Add in a new account for the player to use
        //

        let password = "takemetoyourleader";
        let mut username = String::from("temp");
        for _ in 0..4 {
            username.push((b'a' + NumberGenerator::random_number(26) as u8) as char);
        }

        let mut record = Record::new();
        record.add_field(RECORDBANK_NAME, &username);
        record.add_field(RECORDBANK_PASSWORD, password);
        record.add_field(RECORDBANK_SECURITY, "3");
        g.world_mut()
            .get_computer_mut(&our_name)
            .expect("Arunmor file server must exist")
            .recordbank
            .add_record(record);

        let code = Computer::generate_access_code_name(&username, password);

        let our_ip = g
            .world()
            .get_computer(&our_name)
            .expect("Arunmor file server must exist")
            .ip
            .clone();

        //
        // Generate the mission
        //

        let fulldetails = format!(
            concat!(
                "We have recently been co-operating with Federal Agents in an attempt to bring the CEO ",
                "of Andromeda Research Corporation into custody for questioning.  Unfortunately, for various ",
                "legal and political reasons, the Feds are refusing to arrest the man due to a lack of evidence ",
                "against him.  We are going to provide that evidence.\n\n",
                "Unfortunately for ARC, everyone now knows they are developing the Revelation virus.  Equally ",
                "unfortunately, we have a copy of an early version of Revelation, thanks to an earlier mission.\n\n",
                "Your first task is to connect to our File Server and retrieve Revelation.\n\n",
                "You must then break into OUR Internal Services System (shown below), disable the security and run ",
                "Revelation on that system.  Finally, cover your tracks to make sure the source can't be traced.\n\n",
                "This attack will give the Federal Agents a good enough reason to arrest the CEO of ARC, and we can ",
                "start questioning him.\n\n",
                "It is important to note that nobody else knows of this attack - even the system administrators at our ",
                "Internal Services System.  They will still try to trace you, and if you are caught we will not be ",
                "coming to rescue you.\n\n",
                "Codename: TakeMeToYourLeader\n\n",
                "Our FileServer: {}\n",
                "Username: {}\n",
                "Password: {}\n",
                "Filename: Revelation\n",
                "Target system: {}\n",
                "Target IP: {}"
            ),
            our_ip, username, password, target_name, target_ip
        );

        let details = format!(
            concat!(
                "Payment for this job is {} credits.\n",
                "This job has been assigned an Uplink difficulty of {}.\n",
                "This mission cannot be auto-accepted."
            ),
            PAYMENT_SPECIALMISSION_TAKEMETOYOURLEADER, 10
        );

        let mut m = Mission::new();
        m.set_type(MISSION_SPECIAL);
        m.set_employer("Arunmor");
        m.set_contact("internal@Arunmor.net");
        m.set_payment(
            PAYMENT_SPECIALMISSION_TAKEMETOYOURLEADER,
            (PAYMENT_SPECIALMISSION_TAKEMETOYOURLEADER as f64 * 1.1) as i32,
        );
        m.set_difficulty(10);
        m.set_min_rating(10);
        m.set_accept_rating(99);
        m.set_description(Self::special_mission_description(SPECIALMISSION_TAKEMETOYOURLEADER));
        m.set_details(&details);
        m.set_full_details(&fulldetails);
        m.set_why_so_much_money(Some("It involves very important people."));
        m.set_how_secure(Some("It won't be a problem."));
        m.set_who_is_the_target(Some("The CEO of Andromeda Research Corporation."));
        m.give_link(&target_ip);
        m.give_link(&our_ip);
        m.give_code(&our_ip, &code);
        m
    }

    /// Arunmor #4
    pub fn generate_mission_arcinfiltration(g: &mut Ctx) -> Mission {
        // Find the target computer
        let target_key = NameGenerator::generate_lan_name("ARC");
        let (target_name, target_ip, _) = lookup_computer(g, &target_key);

        // Get a new computer to dump the files to
        let ourcomp_key = NameGenerator::generate_file_server_name("Arunmor");
        let (ourcomp_name, ourcomp_ip, _) = lookup_computer(g, &ourcomp_key);

        // Add in a new account for the player to use
        let password = "infiltrate";
        let username = random_temp_username();
        let code = {
            let ourcomp = g
                .world_mut()
                .get_computer_mut(&ourcomp_key)
                .expect("ARCInfiltration: file server");
            add_account(ourcomp, &username, password);
            Computer::generate_access_code_name(&username, password)
        };

        // Generate around 10 files to be stolen, all fairly large
        let numfiles = 9;
        let encrypted = 5;
        let size = 10;
        for i in 0..numfiles {
            let mut file = Data::new();
            file.set_title(&format!("ARC-REVELATION {}.dat", i));
            file.set_details(DATATYPE_DATA, size, encrypted, 0, 1.0, SOFTWARETYPE_NONE);
            let target = g
                .world_mut()
                .get_computer_mut(&target_key)
                .expect("ARCInfiltration: target");
            put_file_on(target, file);
        }

        // Generate the fields of the mission
        let description =
            PlotGenerator::special_mission_description(SPECIALMISSION_ARCINFILTRATION);
        let fulldetails = format!(
            concat!(
                "As you may be aware, ARC are storing most of their Research on their Local Area Network system. ",
                "However, what is not commonly known is that the critial data (ie the research files themselves) ",
                "are actually stored on a Stand-Alone file server, which is physically disconnected from the rest ",
                "of the Internet.  When the file server is connected for use, the ARC LAN itself is ",
                "disconnected, meaning their data is always totally isolated from public access.  This has made it ",
                "impossible for us to steal their research until now.\n\n",
                "A member of ARCs team who wishes to remain anonymous has come forward and offered to help us. You ",
                "will need to break in to the system first.  Once inside, you should be able to find the restricted ",
                "file area.  Our man on the inside will then install a remote radio modem on the file server, which ",
                "will give you access.  You can then copy the files.\n\n",
                "Once done, decrypt the files and dump them onto our File Server - details below.\n\n",
                "You may also have noticed that the CEO of ARC was recently arrested and questioned by Federal Agents ",
                "in connection with Revelation. We believe ARC are planning to launch Revelation early due to this.  ",
                "This makes the success of this mission Super critical.\n\n",
                "You will be paid {}c for this mission.\n\n",
                "Codename: ARCInfiltration\n\n",
                "Target computer : {}\n",
                "IP : {}\n\n",
                "Once complete, dump the files you have to this computer:\n",
                "{}\n",
                "IP : {}\n\n",
                "USERNAME : {}\n",
                "PASSWORD : {}\n\n",
                "Reply to this mail when you have finished.\n\n",
                "END"
            ),
            PAYMENT_SPECIALMISSION_ARCINFILTRATION,
            target_name,
            target_ip,
            ourcomp_name,
            ourcomp_ip,
            username,
            password
        );
        let details = format!(
            "Payment for this job is {} credits.\nThis job has been assigned an Uplink difficulty of {}.\nThis mission cannot be auto-accepted.",
            PAYMENT_SPECIALMISSION_ARCINFILTRATION, 10
        );

        // Insert the mission
        let mut mission = Mission::new();
        mission.set_type(MISSION_SPECIAL);
        mission.set_completion(None, None, None, None, None);
        mission.set_employer("Arunmor");
        mission.set_contact("internal@Arunmor.net");
        mission.set_difficulty(10);
        mission.set_min_rating(10);
        mission.set_accept_rating(99);
        mission.set_payment(
            PAYMENT_SPECIALMISSION_ARCINFILTRATION,
            (PAYMENT_SPECIALMISSION_ARCINFILTRATION as f64 * 1.1) as i32,
        );
        mission.set_description(description);
        mission.set_details(&details);
        mission.set_full_details(&fulldetails);
        mission.set_why_so_much_money(Some("This mission will be extremely difficult"));
        mission.set_who_is_the_target(Some("Andromeda Research Corporation"));
        mission.set_how_secure(Some("The system will be extremely secure."));
        mission.give_link(&target_ip);
        mission.give_link(&ourcomp_ip);
        mission.give_code(&ourcomp_ip, &code);
        mission
    }

    /// Arunmor #5
    pub fn generate_mission_counter_attack(g: &mut Ctx) -> Mission {
        let version_faith = g.world().plotgenerator.version_faith;

        // Clear file server
        let ourcomp_key = NameGenerator::generate_file_server_name("Arunmor");
        let ourcomp = g
            .world_mut()
            .get_computer_mut(&ourcomp_key)
            .expect("CounterAttack: file server");
        ourcomp.databank.format();

        // Put Tracker on file server
        let mut data = Data::new();
        data.set_title("Revelation_Tracker");
        data.set_details(DATATYPE_PROGRAM, 3, 0, 0, 1.0, SOFTWARETYPE_OTHER);
        ourcomp.databank.put_data(data);

        // Put Faith on file server
        let mut data2 = Data::new();
        data2.set_title("Faith");
        data2.set_details(DATATYPE_PROGRAM, 3, 0, 0, version_faith, SOFTWARETYPE_OTHER);
        ourcomp.databank.put_data(data2);

        // Add in a new account for the player to use
        let password = "faithinchaos";
        let username = random_temp_username();
        add_account(ourcomp, &username, password);
        let code = Computer::generate_access_code_name(&username, password);
        let ourcomp_ip = ourcomp.ip.clone();

        let details = format!(
            concat!(
                "Our tracer programs have just informed us that Revelation has been released ",
                "into the wild by Andromeda Research Corporation.  Only a small number of systems ",
                "are currently infected but more will certainly follow.  This is what we have been planning for.\n\n",
                "Revelation will infect a system, wait for a few minutes, then spread to two other systems. The origional ",
                "infected system will then be shut down. If two many systems get infected, we won't be able to stop it.\n\n",
                "Connect to our File Server and download the Revelation Tracker program. This will keep you updated when ",
                "systems become infected with Revelation, so you can connect and disinfect them.\n\n",
                "You will also need to download Faith - our countervirus.  This is run by copying it into the databanks of ",
                "the infected systems, dropping into a command prompt, and running it.\n\n",
                "There isn't much time.  You have only minutes until Revelation spreads beyond control. Get moving.\n\n",
                "Codename: CounterAttack\n\n",
                "Our FileServer: {}\n",
                "Username: {}\n",
                "Password: {}\n"
            ),
            ourcomp_ip, username, password
        );

        // Generate the mission
        let mut m = Mission::new();
        m.set_type(MISSION_SPECIAL);
        m.set_employer("Arunmor");
        m.set_contact("internal@Arunmor.net");
        m.set_payment(PAYMENT_SPECIALMISSION_COUNTERATTACK, -1);
        m.set_difficulty(20);
        m.set_description(PlotGenerator::special_mission_description(
            SPECIALMISSION_COUNTERATTACK,
        ));
        m.set_full_details(&details);
        m.give_link(&ourcomp_ip);
        m.give_code(&ourcomp_ip, &code);
        m
    }

    /// ARC #1
    pub fn generate_mission_maiden_flight(g: &mut Ctx) -> Mission {
        // Find the target computer
        let target_key = NameGenerator::generate_internal_services_server_name("Arunmor");
        lookup_computer(g, &target_key);

        let fulldetails = format!(
            concat!(
                "Our corporation has been developing a software product known as Revelation for some time now, ",
                "and Version 1.0 is finally ready.  In order to test and refine this program, we need to test it ",
                "in the real world.  Your mission will be to install and run Revelation for us.\n\n",
                "The target system is the Arunmor Internal Services System.  You will need to first locate this computer. ",
                "No doubt Arunmor will ",
                "not be too pleased if they catch you on their system.\n\n",
                "You will find that we have sent you Version 1.0 of Revelation in an email.  Once you are inside ",
                "The Arunmor system, copy the program into their memory banks and disrupt all of their security ",
                "systems.  The more security you disable, the better this product will operate.  In order to activate ",
                "the program you will need to enter a command prompt and run it from there.\n\n",
                "DO NOT ATTEMPT TO RUN REVELATION ON YOUR COMPUTER.\n\n",
                "Once complete, contact us at internal@ARC.net and we'll arrange your debrief and payment.\n",
                "You will be paid a further {}c for successful completion of this mission.\n\n",
                "Codename: MaidenFlight\n\n",
                "[END]"
            ),
            PAYMENT_SPECIALMISSION_MAIDENFLIGHT
        );

        let mut m = Mission::new();
        m.set_type(MISSION_SPECIAL);
        m.set_employer("ARC");
        m.set_contact("internal@ARC.net");
        m.set_payment(
            PAYMENT_SPECIALMISSION_MAIDENFLIGHT,
            PAYMENT_SPECIALMISSION_MAIDENFLIGHT,
        );
        m.set_difficulty(6);
        m.set_description(PlotGenerator::special_mission_description(
            SPECIALMISSION_MAIDENFLIGHT,
        ));
        m.set_full_details(&fulldetails);
        m
    }

    /// ARC #2
    pub fn generate_mission_darwin(g: &mut Ctx) -> Mission {
        // Find the target computer
        let target_key = NameGenerator::generate_lan_name("Darwin Research Associates");
        let (target_name, target_ip, target_companyname) = lookup_computer(g, &target_key);

        // Generate a new computer to dump the files to
        let ourcomp_key = WorldGenerator::generate_empty_file_server(g, "ARC");
        let (ourcomp_name, ourcomp_ip, _) = lookup_computer(g, &ourcomp_key);

        // Add in a new account for the player to use
        let password = "darwin";
        let username = random_temp_username();
        let code = {
            let ourcomp = g
                .world_mut()
                .get_computer_mut(&ourcomp_key)
                .expect("Darwin: file server");
            add_account(ourcomp, &username, password);
            Computer::generate_access_code_name(&username, password)
        };

        // Generate around 10 files to be stolen, all fairly large
        let numfiles = 9;
        let encrypted = 5;
        let size = 10;
        let prefix = company_prefix4(&target_companyname);
        for i in 0..numfiles {
            let mut file = Data::new();
            file.set_title(&format!("{}-DARWIN {}.dat", prefix, i));
            file.set_details(DATATYPE_DATA, size, encrypted, 0, 1.0, SOFTWARETYPE_NONE);
            let target = g
                .world_mut()
                .get_computer_mut(&target_key)
                .expect("Darwin: target");
            put_file_on(target, file);
        }

        // Generate the fields of the mission
        let description = PlotGenerator::special_mission_description(SPECIALMISSION_DARWIN);
        let fulldetails = format!(
            concat!(
                "The results of our recent test have been extremely encouraging.  Revelation is easily ",
                "destructive enough to take out most of the Internet when it is finally released.  Unfortunately, ",
                "it is not particularly virulant.  In other words, it doesn't spread very fast.  We need to fix this.\n\n",
                "We've looked around and we have found a company called Darwin Research Associates.  They are currently ",
                "working on research into digital life forms that exist and breed on the Internet.  Since they have refused ",
                "to sell any of their technology to us, we have decided instead to steal it.\n\n",
                "Break into their Local Area Network system shown below, and steal any files relating to the Darwin project.  ",
                "Try to copy as much data as possible.  You will need to decrypt the data before we can make any use of it.  ",
                "We have determined that over 80Gqs of their research is stored on this system.\n\n",
                "Once you have stolen the research, copy it onto the File Server that we have set up for you, detailed below.\n\n",
                "We are confident we will be able to use their artificial life code to make Revelation spread like wildfire.",
                "You will be paid {}c for this mission.\n\n",
                "Codename: Darwin\n\n",
                "Target computer : {}\n",
                "IP : {}\n\n",
                "Once complete, dump the files you have to this computer:\n",
                "{}\n",
                "IP : {}\n\n",
                "USERNAME : {}\n",
                "PASSWORD : {}\n\n",
                "Reply to this mail when you have finished.\n\n",
                "END"
            ),
            PAYMENT_SPECIALMISSION_DARWIN,
            target_name,
            target_ip,
            ourcomp_name,
            ourcomp_ip,
            username,
            password
        );

        // Insert the mission
        let mut mission = Mission::new();
        mission.set_type(MISSION_SPECIAL);
        mission.set_completion(None, None, None, None, None);
        mission.set_employer("ARC");
        mission.set_contact("internal@ARC.net");
        mission.set_difficulty(10);
        mission.set_payment(PAYMENT_SPECIALMISSION_DARWIN, -1);
        mission.set_description(description);
        mission.set_full_details(&fulldetails);
        mission.give_link(&target_ip);
        mission.give_link(&ourcomp_ip);
        mission.give_code(&ourcomp_ip, &code);
        mission
    }

    /// ARC #3
    pub fn generate_mission_save_it_for_the_jury(g: &mut Ctx) -> Mission {
        // Find the person who is to be jailed
        let poorsod_key = WorldGenerator::get_random_person(g);
        let poorsod_name = g
            .world()
            .get_person(&poorsod_key)
            .expect("SaveItForTheJury: person")
            .name
            .clone();
        g.world_mut().plotgenerator.saveitforthejury_guytobeframed = poorsod_name.clone();

        let personal_key = NameGenerator::generate_personal_computer_name(&poorsod_name);
        let (_, personal_ip, _) = lookup_computer(g, &personal_key);

        // Choose a bank that the guy is due to hack
        let bank_key =
            WorldGenerator::get_random_computer_of_type(g, COMPUTER_TYPE_PUBLICBANKSERVER);
        let (bank_name, bank_ip, _) = lookup_computer(g, &bank_key);
        g.world_mut().plotgenerator.saveitforthejury_targetbankip = bank_ip.clone();

        let description =
            PlotGenerator::special_mission_description(SPECIALMISSION_SAVEITFORTHEJURY);
        let fulldetails = format!(
            concat!(
                "We have decided that it is time to get personal.  The chief technical director of ",
                "Arunmor Corporation, Mr {}, has recently excelled in his line of work.\n",
                "He has managed to make a number of breakthrough's in their research into Revelation, and no doubt ",
                "his presence at Arunmor has increased the power of their rival virus Faith.  We have decided that ",
                "his life should now come to an effective end.\n\n",
                "In addition, we'd like to strike a blow to Arunmor's public image as well.  As such, we'd like you ",
                "to frame their chief tech for something particularly nasty - Bank Fraud.\n\n",
                "Break in to the bank below, and start messing around.  Make sure they notice you.  Then break in ",
                "to one of your relay systems and modify the logs so that {} is incriminated.\n\n",
                "To do this, you'll need to make the log point back to his home computer - address below.\n",
                "Codename: SaveItForTheJury\n\n",
                "\n\nTARGET BANK:\n",
                "{}\n",
                "IP: {}\n",
                "\n",
                "Target individual :\n",
                "{}\n",
                "His personal computer: {}\n"
            ),
            poorsod_name, poorsod_name, bank_name, bank_ip, poorsod_name, personal_ip
        );

        // Build the mission
        let mut m = Mission::new();
        m.set_type(MISSION_SPECIAL);
        m.set_employer("ARC");
        m.set_contact("internal@ARC.net");
        m.set_payment(PAYMENT_SPECIALMISSION_SAVEITFORTHEJURY, -1);
        m.set_difficulty(10);
        m.set_description(description);
        m.set_full_details(&fulldetails);
        m.give_link(&bank_ip);
        m.give_link(&personal_ip);
        m
    }

    /// ARC #4
    pub fn generate_mission_shiny_hammer(g: &mut Ctx) -> Mission {
        // Insert some interesting data onto Arunmor's system
        let target_key = NameGenerator::generate_lan_name("Arunmor");
        let (_, _, target_companyname) = lookup_computer(g, &target_key);
        let prefix = company_prefix4(&target_companyname);

        for i in 0..10 {
            let datatitle = format!("{}-FAITH {}.dat", prefix, i);
            let size = NumberGenerator::random_number(10) + 1;
            let encrypted = NumberGenerator::random_number(5);

            let mut file = Data::new();
            file.set_title(&datatitle);
            file.set_details(DATATYPE_DATA, size, encrypted, 0, 1.0, SOFTWARETYPE_NONE);
            let target = g
                .world_mut()
                .get_computer_mut(&target_key)
                .expect("ShinyHammer: target");
            put_file_on(target, file);
        }

        // Ahhh... a nice simple one for a change
        let details = format!(
            concat!(
                "We are getting tired of racing against Arunmor Corporation, and recently ",
                "their Faith project has been performing well against Revelation.  We need ",
                "to put a stop to this.  Since the CEO of our company was arrested, Arunmor have ",
                "been catching up with our research every day.\n\n",
                "This mission is designed to be a crushing blow ",
                "to the research efforts of Arunmor Corporation.\n\n",
                "We know from our inside sources that Arunmor keep all of their research on ",
                "their Local Area Network, and that it is heavily guarded. The fact that they leave ",
                "this system open to the Internet will be their un-doing.\n\n",
                "Your task is simple.  Find their Local Area Network, break in to it, and delete ",
                "all the files you find.  Then take down the entire system.\n\n",
                "Codename: ShinyHammer\n\n",
                "You will encounter severe resistance.  We are prepared to pay you {} credits for completion of this mission.  If you succeed, we believe we will be able to ",
                "take advantage of their momentary confusion and launch Revelation successfully.\n\n",
                "Be patient...the time is near."
            ),
            PAYMENT_SPECIALMISSION_SHINYHAMMER
        );

        let mut m = Mission::new();
        m.set_type(MISSION_SPECIAL);
        m.set_employer("ARC");
        m.set_contact("internal@ARC.net");
        m.set_payment(PAYMENT_SPECIALMISSION_SHINYHAMMER, -1);
        m.set_difficulty(15);
        m.set_description(PlotGenerator::special_mission_description(
            SPECIALMISSION_SHINYHAMMER,
        ));
        m.set_full_details(&details);
        m
    }

    /// ARC #5
    pub fn generate_mission_grand_tour(g: &mut Ctx) -> Mission {
        let handle = g.world().get_player().agent().handle.clone();

        let details = format!(
            concat!(
                "In your email inbox you will see two emails from us - one containing the final version ",
                "of the Revelation virus, and the other containing a tracker which will list the systems ",
                "currently infected.\n\n",
                "This is your green light agent {} - we have ",
                "selected you as our most successful freelance agent to carry out our final mission. Your aim is ",
                "simple - release Revelation onto as many systems as you possibly can before our entire company ",
                "is shut down by Federal Agents.\n\n",
                "Arunmor have Faith ready - they will be right behind you.  If you are quick enough, you will be able ",
                "to spread the virus faster than they can stop it. Revelation will infect a system, scan for nearby systems ",
                "on the same subnet, then migrate to the nearest two of those systems. At the same time it will destroy ",
                "the host it originated from.  Once a certain number of systems have been disabled, the growth of the virus ",
                "will be unstoppable and the entire Internet will suffer a system-wide cascade failure.\n\n",
                "This is the end - if you are successfull there will be no more ",
                "Uplink Corporation, no more ARC and no more Arunmor.  The entire Internet will come down with us.  ",
                "Think about it - we can destroy all records of ownership, reset everyone back to square one, crash ",
                "the whole damn system.  Think of it!\n\n",
                "This will be our last message to you - if all goes to plan, we will never speak again.\n\n",
                "The codename for this mission is GrandTour.  Best of luck.  ARC out."
            ),
            handle
        );

        let mut m = Mission::new();
        m.set_type(MISSION_SPECIAL);
        m.set_employer("ARC");
        m.set_contact("internal@ARC.net");
        m.set_payment(PAYMENT_SPECIALMISSION_GRANDTOUR, -1);
        m.set_difficulty(20);
        m.set_description(PlotGenerator::special_mission_description(
            SPECIALMISSION_GRANDTOUR,
        ));
        m.set_full_details(&details);
        m
    }

    pub fn is_mission_complete_tracer(g: &mut Ctx) -> bool {
        // Find the target computer
        let comp_key = NameGenerator::generate_internal_services_server_name("ARC");
        let (_, comp_ip, _) = lookup_computer(g, &comp_key);

        if g.world().plotgenerator.tracer_lastknownip == comp_ip {
            // Success
            send_plot_mail(
                g,
                "internal@Arunmor.net",
                "PLAYER",
                "Congratulations",
                concat!(
                    "We just started receiving ping signals from our Tracer Software.\n",
                    "This means it is set up correctly and working. Time will tell if they managed ",
                    "to spot what you are doing.\n\n",
                    "We have made the full mission payment into your account.\n\n",
                    "When the time comes, well send you some Tracer software that you can run on ",
                    "your local system.  This will list all of the computers that have been infected ",
                    "with the Revelation virus."
                ),
            );

            {
                let pg = &mut g.world_mut().plotgenerator;
                pg.version_faith += 1.0;
                pg.completed_tracer = true;
            }

            PlotGenerator::player_completes_special_mission(g, SPECIALMISSION_TRACER);

            // Schedule news story
            let rundate = plot_date_hours_later(g, 6);
            schedule_plot_scene_p3(g, 4, 4, &rundate);

            true
        } else {
            // Failure
            send_plot_mail(
                g,
                "internal@Arunmor.net",
                "PLAYER",
                "Failure",
                "We haven't received any ping signals from our Tracer yet.  You can't have installed it right.",
            );

            false
        }
    }

    pub fn is_mission_complete_take_me_to_your_leader(g: &mut Ctx) -> bool {
        // Nice and simple
        let comp_key = NameGenerator::generate_internal_services_server_name("Arunmor");
        let isinfected = g
            .world()
            .get_computer(&comp_key)
            .expect("TakeMeToYourLeader: computer")
            .isinfected_revelation;

        if isinfected >= 0.0 {
            PlotGenerator::player_completes_special_mission(g, SPECIALMISSION_TAKEMETOYOURLEADER);

            // Send him a nice message
            send_plot_mail(
                g,
                "internal@ARC.net",
                "PLAYER",
                "Congratulations",
                concat!(
                    "That looks like a total success to us. We'll have to keep quiet for a while, but it looks like ",
                    "the Feds are getting ready to arrest the CEO of Andromeda Research Corporation tommorow.\n\n",
                    "It also looks like we are going to be able to question him about Revelation.  This is going to ",
                    "help our cause.\n\n",
                    "We believe that the CEO of Andromeda Corporation is not actually the man in charge ",
                    "of the Revelation project - since ARC is actually owned by the Andromeda group, and their leader ",
                    "is currently unknown.  However, he undoubtably has a strong impact on their progress.\n\n",
                    "If we are very lucky, this man will help us find the leader of Andromeda.\n\n",
                    "It is also likely that our funding will be increased due to this outbreak.  The Government is ",
                    "getting quite desperate at the moment - they think their entire infrastructure may be destroyed, ",
                    "and we are the only people who can offer them a solution."
                ),
            );

            {
                let pg = &mut g.world_mut().plotgenerator;
                pg.version_faith += 1.0;
                pg.completed_takemetoyourleader = true;
            }

            // Schedule the main news story for a few days from now
            let rundate = plot_date_hours_later(g, 6);
            schedule_plot_scene_p3(g, 4, 5, &rundate);

            true
        } else {
            false
        }
    }

    pub fn is_mission_complete_arcinfiltration(g: &mut Ctx) -> bool {
        // Player thinks he has completed the ARC Infiltration mission
        let ourcomp_key = NameGenerator::generate_file_server_name("Arunmor");
        let foundsize = scan_found_files(g, &ourcomp_key, "REVELATION");

        // How many files out of the 10 did he find?
        if foundsize == 0 {
            send_plot_mail(
                g,
                "internal@Arunmor.net",
                "PLAYER",
                "No files found",
                concat!(
                    "We don't see any useful files on the File Server we specified.\n",
                    "Have you copied the files onto the file server?  Have you remembered ",
                    "to decrypt the files so we can read them?"
                ),
            );

            false
        } else {
            let handle = g.world().get_player().agent().handle.clone();
            let body = format!(
                concat!(
                    "\n\nWe can see {} Gigaquads of useful data which will help us ",
                    "perfect Faith.\n",
                    "Congratulations on your success Agent {}. We are still not entirely confident that we can stop an outbreak of Revelation, ",
                    "but this data will certainly improve our chances.\n\n",
                    "We have managed to determine that the reproductive systems in the Revelation virus ",
                    "are based on data and code stolen from Darwin Research Associates a few weeks ago.  The ",
                    "virus mutates, multiplies and evolves, much like a real life form.  We believe the way ",
                    "to defeat it is to attack it with another virus that it cannot evolve against."
                ),
                foundsize, handle
            );
            send_plot_mail(
                g,
                "internal@Arunmor.net",
                "PLAYER",
                "Congratulations",
                &body,
            );

            {
                let pg = &mut g.world_mut().plotgenerator;
                pg.completed_arcinfiltration = true;
                let change = (3.0_f32 * (foundsize as f32 / 90.0_f32)) as i32;
                pg.version_faith += change as f32;
            }

            PlotGenerator::player_completes_special_mission(g, SPECIALMISSION_ARCINFILTRATION);

            // Schedule news story
            let rundate = plot_date_hours_later(g, 6);
            schedule_plot_scene_p3(g, 4, 6, &rundate);

            true
        }
    }

    pub fn is_mission_complete_maiden_flight(g: &mut Ctx) -> bool {
        let comp_key = NameGenerator::generate_internal_services_server_name("Arunmor");
        let isinfected = g
            .world()
            .get_computer(&comp_key)
            .expect("MaidenFlight: computer")
            .isinfected_revelation;

        if isinfected > 0.0 {
            PlotGenerator::player_completes_special_mission(g, SPECIALMISSION_MAIDENFLIGHT);

            // Send him a nice message
            send_plot_mail(
                g,
                "internal@ARC.net",
                "PLAYER",
                "Congratulations",
                concat!(
                    "Congratulations on your recent success agent - we are very pleased ",
                    "with the way things turned out.  We have made the appropriate payment into ",
                    "your account.\n\n",
                    "No doubt this event will manage a lot of media coverage so it ",
                    "is probably best if you keep a low profile for a short while.  We will contact ",
                    "you again soon with some more work."
                ),
            );

            // Player now works for ARC
            {
                let pg = &mut g.world_mut().plotgenerator;
                pg.playerloyalty = -1;
                pg.act = 2;
                pg.scene = 4;
                pg.version_revelation = 1.0;
                pg.version_faith = 0.1;
            }

            // Schedule the main news story for a few days from now
            let rundate = plot_date_hours_later(g, 6);
            schedule_plot_scene_p3(g, 3, 1, &rundate);

            true
        } else {
            false
        }
    }

    pub fn is_mission_complete_darwin(g: &mut Ctx) -> bool {
        // Player thinks he has completed the Darwin mission
        let ourcomp_key = NameGenerator::generate_file_server_name("ARC");
        let foundsize = scan_found_files(g, &ourcomp_key, "DARWIN");

        // How many files out of the 10 did he find?
        if foundsize == 0 {
            send_plot_mail(
                g,
                "internal@ARC.net",
                "PLAYER",
                "No files found",
                concat!(
                    "We don't see any useful files on the File Server we specified.\n",
                    "Have you copied the files onto the file server?  Have you remembered ",
                    "to decrypt the files so we can read them?"
                ),
            );

            false
        } else {
            let handle = g.world().get_player().agent().handle.clone();
            let body = format!(
                concat!(
                    "Congratulations on your success Agent {}",
                    "\n\nWe can see {} Gigaquads of useful data which will help us ",
                    "perfect Revelation.  At the moment the virus does not spread particularly well - ",
                    "if the security systems of the local machine are still running, the virus has ",
                    "trouble getting out of the system at all.  We are planning on using the research ",
                    "that Darwin has conducted to make the virus mutate and evolve, in order to ",
                    "combat any defense systems it comes up against."
                ),
                handle, foundsize
            );
            send_plot_mail(g, "internal@ARC.net", "PLAYER", "Congratulations", &body);

            {
                let pg = &mut g.world_mut().plotgenerator;
                pg.completed_darwin = true;
                let change = (2.0_f32 * (foundsize as f32 / 90.0_f32)) as i32;
                pg.version_revelation += change as f32;
            }

            PlotGenerator::player_completes_special_mission(g, SPECIALMISSION_DARWIN);

            // Schedule news story
            let rundate = plot_date_hours_later(g, 6);
            schedule_plot_scene_p3(g, 4, 10, &rundate);

            true
        }
    }

    pub fn is_mission_complete_save_it_for_the_jury(g: &mut Ctx) -> bool {
        // Look up the persons computer
        let poorsod_key = g.world().plotgenerator.saveitforthejury_guytobeframed.clone();
        let poorsod_name = g
            .world()
            .get_person(&poorsod_key)
            .expect("SaveItForTheJury: person")
            .name
            .clone();
        let personal_key = NameGenerator::generate_personal_computer_name(&poorsod_name);
        let (_, personal_ip, _) = lookup_computer(g, &personal_key);

        // Lookup the bank he was supposed to be framed for
        let target_bank_ip = g.world().plotgenerator.saveitforthejury_targetbankip.clone();
        let (bank_name, bank_ip) = {
            let vl = g
                .world()
                .get_vlocation(&target_bank_ip)
                .expect("SaveItForTheJury: bank location");
            let comp = vl
                .get_computer(g.world())
                .expect("SaveItForTheJury: bank computer");
            (comp.name.clone(), comp.ip.clone())
        };

        // Look through logs on that system
        // See if any trace back to him
        let mut success = 0;
        let numlogs = g
            .world()
            .get_computer(&bank_name)
            .expect("SaveItForTheJury: bank computer")
            .logbank
            .logs
            .size();

        for il in 0..numlogs {
            let al = {
                let comp = g
                    .world()
                    .get_computer(&bank_name)
                    .expect("SaveItForTheJury: bank computer");
                if !comp.logbank.logs.valid_index(il) {
                    continue;
                }
                comp.logbank
                    .logs
                    .get_data(il)
                    .expect("SaveItForTheJury: access log")
                    .clone()
            };

            if al.suspicious != LOG_NOTSUSPICIOUS && al.type_ == LOG_TYPE_CONNECTIONOPENED {
                let fromcomp_ip = {
                    let vlf = g
                        .world()
                        .get_vlocation(&al.fromip)
                        .expect("SaveItForTheJury: from location");
                    vlf.get_computer(g.world())
                        .expect("SaveItForTheJury: from computer")
                        .ip
                        .clone()
                };

                let traced_ip = LogBank::trace_log(g, &bank_ip, &fromcomp_ip, &al.date, 10);

                if let Some(traced_ip) = traced_ip {
                    if traced_ip == personal_ip {
                        // Now investigated - so set it back to LOG_NOTSUSPICIOUS
                        let comp = g
                            .world_mut()
                            .get_computer_mut(&bank_name)
                            .expect("SaveItForTheJury: bank computer");
                        if let Some(log) = comp.logbank.logs.get_data_mut(il) {
                            log.set_suspicious(LOG_NOTSUSPICIOUS);
                        }
                        if let Some(internal) = comp.logbank.internallogs.get_data_mut(il) {
                            internal.set_suspicious(LOG_NOTSUSPICIOUS);
                        }

                        // Success!
                        success = 2;
                        break;
                    } else {
                        success = 1;
                    }
                }
            }
        }

        if success == 2 {
            send_plot_mail(
                g,
                "internal@ARC.net",
                "PLAYER",
                "Congratulations",
                concat!(
                    "Well done Agent - we have checked the systems ourselves and have determined that ",
                    "enough evidence has been planted to convict the man.  Police will soon be receiving ",
                    "an anonymous tip, which should end his work at Arunmor.\n\n",
                    "We are planning on using this instance to cast doubt over the actions of Arunmor Corporation. ",
                    "Currently they are receiving a great deal of funding from the Government and are making ",
                    "very fast progress on their Faith countervirus.  This has become something of an arms race. ",
                    "Hopefully the Government will be less willing to fund such an operation when they hear about ",
                    "this financial scandal."
                ),
            );

            {
                let pg = &mut g.world_mut().plotgenerator;
                pg.version_faith -= 1.0;
                if pg.version_faith < 0.2 {
                    pg.version_faith = 0.2;
                }
                pg.completed_saveitforthejury = true;
            }

            PlotGenerator::player_completes_special_mission(g, SPECIALMISSION_SAVEITFORTHEJURY);

            // Schedule news story
            let rundate = plot_date_hours_later(g, 6);
            schedule_plot_scene_p3(g, 4, 11, &rundate);

            true
        } else if success == 1 {
            send_plot_mail(
                g,
                "internal@ARC.net",
                "PLAYER",
                "The wrong person has been framed",
                concat!(
                    "We've looked at the systems involved, and we can see that you have performed the hack.\n",
                    "However, it doesn't appear that the logs lead back to his home computer.  You must fix this \n",
                    "before somebody traces the wrong person."
                ),
            );

            false
        } else {
            send_plot_mail(
                g,
                "internal@ARC.net",
                "PLAYER",
                "Failure to complete mission",
                "You don't appear to have done the hack or framed the man in question.",
            );

            false
        }
    }

    pub fn is_mission_complete_shiny_hammer(g: &mut Ctx) -> bool {
        // Look up the system
        let computer_key = NameGenerator::generate_lan_name("Arunmor");
        let (isrunning, formatted) = {
            let computer = g
                .world()
                .get_computer(&computer_key)
                .expect("ShinyHammer: computer");
            (computer.isrunning, computer.databank.formatted)
        };

        let (body, faith_loss) = if !isrunning && formatted {
            // Success!
            (
                concat!(
                    "We've just heard the news about Arunmor's research - congratulations!\n\n",
                    "It looks like you performed the job with your usual style and skill.  Well done.\n\n",
                    "Faith is certain to take a major blow from this.  We are planning on releasing Revelation ",
                    "very shortly...there are just a few more things to take care of first.\n\n",
                    "It is likely that Arunmor had some backups of the countervirus somewhere, so we will have to ",
                    "move quickly to take advantage of their loss.\n",
                    "We will contact you soon."
                ),
                2.0_f32,
            )
        } else if !isrunning && !formatted {
            // Shut down the system, didn't delete the files
            (
                concat!(
                    "Well done for attempting the Arunmor crash mission.  It appears you shut down the system, ",
                    "which will cause them problems.  But it appears they have recovered the data from back-ups.\n\n",
                    "You should have deleted all the files before you shut the system down.\n\n",
                    "Still, this will hurt Arunmor.  We are planning on releasing Revelation ",
                    "very shortly...there are just a few more things to take care of first.\n\n",
                    "We will contact you soon."
                ),
                0.5_f32,
            )
        } else if isrunning && formatted {
            // Didn't shut down system, but did delete files
            (
                concat!(
                    "Congratulations on deleting the data on Arunmor's central mainframe.  It appears you didn't ",
                    "quite manage to shut down the system, but that's ok.  We've dealt them a nasty blow.  ",
                    "We are planning on releasing Revelation ",
                    "very shortly...there are just a few more things to take care of first.\n\n",
                    "We will contact you soon."
                ),
                1.0_f32,
            )
        } else {
            // Total failure
            send_plot_mail(
                g,
                "internal@ARC.net",
                "PLAYER",
                "CONGRATULATIONS",
                "You don't appear to have damaged the Arunmor Central Mainframe very much.  Try again.",
            );

            return false;
        };

        {
            let pg = &mut g.world_mut().plotgenerator;
            pg.version_faith -= faith_loss;
            if pg.version_faith < 0.2 {
                pg.version_faith = 0.2;
            }
        }

        send_plot_mail(
            g,
            "internal@ARC.net",
            "PLAYER",
            "CONGRATULATIONS",
            body,
        );

        g.world_mut().plotgenerator.completed_shinyhammer = true;

        PlotGenerator::player_completes_special_mission(g, SPECIALMISSION_SHINYHAMMER);

        // Schedule news story
        let rundate = plot_date_hours_later(g, 3);
        schedule_plot_scene_p3(g, 4, 12, &rundate);

        true
    }

    pub fn player_completes_special_mission(g: &mut Ctx, mission_id: i32) {
        // Pay him / change ratings
        // Remove the mission from his list

        let title = Self::special_mission_description(mission_id);
        let found = g
            .player()
            .person
            .agent()
            .missions
            .iter()
            .position(|m| m.description == title);
        let Some(i) = found else {
            return;
        };

        let (payment, difficulty, contact) = {
            let m = &g.player().person.agent().missions[i];
            (m.payment, m.difficulty, m.contact.clone())
        };

        Person::change_balance(g, "PLAYER", payment, "Anonymous donation");
        Rating::change_uplink_score(g, "PLAYER", difficulty);
        g.player_mut().person.agent_mut().missions.remove(i);

        pg_mut(g).specialmissionscompleted |= 1 << mission_id;

        if contact == "internal@ARC.net" {
            Rating::change_neuromancer_score(g, "PLAYER", difficulty * -1);
        } else {
            Rating::change_neuromancer_score(g, "PLAYER", difficulty);
        }
    }

    pub fn player_fails_special_mission(g: &mut Ctx, mission_id: i32) {
        // Remove the mission from his list

        let title = Self::special_mission_description(mission_id);
        let found = g
            .player()
            .person
            .agent()
            .missions
            .iter()
            .position(|m| m.description == title);
        if let Some(i) = found {
            g.player_mut().person.agent_mut().missions.remove(i);
        }
    }

    /// Remove from public BBS board
    pub fn remove_special_mission(g: &mut Ctx, mission_id: i32) -> bool {
        let title = Self::special_mission_description(mission_id);

        let cu = company_mut(g, "Uplink")
            .uplink
            .as_deref_mut()
            .expect("plotgenerator: Uplink company has no CompanyUplink");

        match cu.missions.iter().position(|m| m.description == title) {
            Some(i) => {
                cu.missions.remove(i);
                true
            }
            None => false,
        }
    }

    /// It's game over! (success for ARC)
    pub fn revelation_release_uncontrolled(g: &mut Ctx) {
        // Its all over
        let mut rundate = Date::default();
        rundate.set_date_from(&g.world().date);
        rundate.advance_second(10);

        schedule_plot_scene_p3(g, 5, 7, &rundate);

        g.world_mut().plotgenerator.revelation_releaseuncontrolled = true;

        // Run revelation locally
        let version = g.world().plotgenerator.version_revelation;
        TaskManager::run_software(g, "Revelation", version);
    }

    /// Generates a news story
    pub fn news_revelation_used(g: &mut Ctx, ip: &str, success: i32) {
        // Look up the system attacked
        let (compname, compip, companyname) = {
            let vl = g
                .world()
                .get_vlocation(ip)
                .expect("NewsRevelationUsed: location");
            let comp = vl
                .get_computer(g.world())
                .expect("NewsRevelationUsed: computer");
            (comp.name.clone(), comp.ip.clone(), comp.companyname.clone())
        };
        let numuses = g.world().plotgenerator.numuses_revelation;

        // Headline
        // Part1:       Company hit by virus
        // Part2:       Extent of damage
        // Part3:       History of virus use
        let part1 = format!(
            "{} has been attacked by a computer virus.  The virus first infected the {}.",
            companyname, compname
        );

        let part2 = match success {
            0 => "System administrators were able to contain the virus before it managed any serious damage.  It appears the virus was totally contained.".to_string(),
            1 => "The virus destroyed most of the data and logs on the computer but the company was able to stop the virus from spreading to other systems.".to_string(),
            2 => format!("The {} was totally destroyed and the virus spread to many neighbouring systems, causing a great deal of damage.", compname),
            3 => format!("It appears that the entire corporate computer network owned by {} was severely damaged by the virus.", companyname),
            _ => String::new(),
        };

        let (part3, headline) = match numuses {
            0 | 1 => (
                "This is the first time that this virus has been seen in the wild, but experts who analysed it say it has incredible power.  After analysing the virus in detail, it has been determined that the name of the virus is Revelation.".to_string(),
                format!("Unknown virus outbreak at {}", companyname),
            ),
            2 => (
                "Upon closer inspection, industry experts have been able to determine that the Revelation computer virus is responsible.  This is the second time that Revelation has been deployed on the internet.".to_string(),
                format!("Revelation seen for second time at {}", companyname),
            ),
            _ => (
                "Once again Revelation is responsible for this destruction.".to_string(),
                "Revelation outbreak yet again".to_string(),
            ),
        };

        // Concatenate each part together and post it
        let details = format!("{}\n\n{}\n\n{}\n\n", part1, part2, part3);

        let mut news = News::new();
        news.set_headline(&headline);
        news.set_details(&details);
        news.set_data(NEWS_TYPE_COMPUTERDESTROYED, Some(compip.as_str()), None);

        g.world().get_company("Uplink").expect("NewsRevelationUsed: Uplink company");
        CompanyUplink::create_news(g, news);
    }

    // ---- emails from the player ----

    /// Handles emails from the player to ARC. True if this function handled the email.
    pub fn player_contacts_arc(g: &mut Ctx, msg: &Message) -> bool {
        if !globals::DEMOGAME {
            let act = pg(g).act;
            let scene = pg(g).scene;

            if act == 2 && scene == 1 {
                // The player is responding to the offer to begin
                // working for ARC - so send him the "Maiden Flight"
                // mission and a cover note in return, with Revelation attached

                let mut revelation = Data::new();
                revelation.set_title("Revelation");
                revelation.set_details(DATATYPE_PROGRAM, 4, 0, 0, 1.0, SOFTWARETYPE_OTHER);

                let mut reply = Message::new();
                reply.set_from("internal@ARC.net");
                reply.set_to("PLAYER");
                reply.set_subject("Welcome to ARC");
                reply.set_body(concat!(
                    "Thank you for replying to our offer, agent.  We knew that you ",
                    "would make the right decision.\n\n",
                    "As it happens, we have some work for you to get started on immediately.\n",
                    "We are getting ready to test out Revelation and we want you to help.  ",
                    "Please read the attached mission description and get started as soon as possible.\n\n",
                    "No doubt you will incur some expenses for this mission, so please accept a small ",
                    "donation of 10000c to help you with any equipment you might need.\n\n",
                    "The attached file is to be used as part of your mission.  DO NOT ATTEMPT TO RUN ",
                    "THE ATTACHED PROGRAM LOCALLY.\n\n",
                    "[END]",
                ));
                reply.attach_data(&revelation);
                reply.send(g);

                Person::change_balance(g, "PLAYER", 10000, "Anonymous donation");

                let maiden_flight = Self::generate_mission_maiden_flight(g);
                Person::give_mission(g, "PLAYER", maiden_flight);

                pg_mut(g).act = 2;
                pg_mut(g).scene = 2;

                // Schedule act 2 scene 3 very soon
                // (Arunmor get in contact and offer him another job)
                let mut rundate = Date::default();
                rundate.set_date_from(&g.world().date);
                rundate.advance_minute(2);
                schedule_scene(g, &rundate, 2, 3);

                return true;
            }

            if act == 2 && (scene == 2 || scene == 3) {
                Self::is_mission_complete_maiden_flight(g);
                return true;
            }

            if act == 4 {
                // Player might have completed one of our missions
                if pg(g).playerloyalty == -1 {
                    let body = msg.get_body();

                    if body.contains(Self::special_mission_description(SPECIALMISSION_DARWIN)) {
                        if !pg(g).completed_darwin {
                            Self::is_mission_complete_darwin(g);
                        }
                    } else if body
                        .contains(Self::special_mission_description(SPECIALMISSION_SAVEITFORTHEJURY))
                    {
                        if !pg(g).completed_saveitforthejury {
                            Self::is_mission_complete_save_it_for_the_jury(g);
                        }
                    } else if body.contains(Self::special_mission_description(SPECIALMISSION_SHINYHAMMER)) {
                        if !pg(g).completed_shinyhammer {
                            Self::is_mission_complete_shiny_hammer(g);
                        }
                    }
                }

                return true;
            }
        }

        false
    }

    /// Handles emails from the player to Arunmor. True if this function handled the email.
    pub fn player_contacts_arunmor(g: &mut Ctx, msg: &Message) -> bool {
        if !globals::DEMOGAME {
            let act = pg(g).act;
            let scene = pg(g).scene;

            if act == 2 && scene == 3 {
                // Player may have sent us a copy of Revelation
                // In response to our offer
                if let Some(attached) = msg.get_data() {
                    if attached.type_ == DATATYPE_PROGRAM && attached.title == "Revelation" {
                        pg_mut(g).version_faith = attached.version;

                        Person::change_balance(g, "PLAYER", PAYMENT_SPECIALMISSION_BACKFIRE, "Arunmor [Faith project]");
                        Rating::change_uplink_score(g, "PLAYER", 10);
                        Rating::change_neuromancer_score(g, "PLAYER", 10);

                        let mut reply = Message::new();
                        reply.set_to("PLAYER");
                        reply.set_from("internal@Arunmor.net");
                        reply.set_subject("Your recent submission");
                        reply.set_body(concat!(
                            "Let us be the first to congratulate you Agent - we have been ",
                            "totally unable to find any copies of Revelation ",
                            "and we had been hoping that you would decide to work for us.\n\n",
                            "Thanks to your submission, we will now be able to catch up with the ",
                            "research of our counter virus to Revelation.  We have already deposited ",
                            "the full reward into your account.\n\n",
                            "We will have some more work for you soon regarding this project.  ",
                            "We will let you know.",
                        ));
                        reply.send(g);

                        pg_mut(g).specialmissionscompleted |= 1 << SPECIALMISSION_BACKFIRE;

                        // Player now works for Arunmor
                        pg_mut(g).playerloyalty = 1;
                        pg_mut(g).act = 2;
                        pg_mut(g).scene = 4;

                        // Schedule start of next act for a day from now
                        let mut rundate = Date::default();
                        rundate.set_date_from(&g.world().date);
                        rundate.advance_hour(NumberGenerator::random_number(12));
                        schedule_scene(g, &rundate, 3, 1);
                    }
                }
            } else if act == 3 {
                // The player might have sent a copy of Revelation - big news!
                if let Some(attached) = msg.get_data() {
                    if attached.type_ == DATATYPE_PROGRAM && attached.title == "Revelation" {
                        if pg(g).playerloyalty != 1 && attached.version > pg(g).version_faith {
                            pg_mut(g).version_faith = attached.version;

                            Person::change_balance(g, "PLAYER", 25000, "Arunmor [Faith project]");
                            Rating::change_uplink_score(g, "PLAYER", 10);
                            Rating::change_neuromancer_score(g, "PLAYER", 10);

                            let mut reply = Message::new();
                            reply.set_to("PLAYER");
                            reply.set_from("Arunmor");
                            reply.set_subject("Your recent submission");
                            reply.set_body(concat!(
                                "Let us be the first to congratulate you Agent - we have been ",
                                "totally unable to recover Revelation from our damaged machines ",
                                "and we had been hoping that somebody out there would have a copy of it.\n\n",
                                "Of course, we won't ask where you got it, nor will we let it become known who sent it too us.\n\n",
                                "Thanks to your submission, we will now be able to pull several weeks ahead of Revelation.  We have already deposited ",
                                "the full reward of 25000c into your account.\n\n",
                                "We may have some more work for you in future.\n\n",
                                "Once again thank you for your contribution to project Faith.",
                            ));
                            reply.send(g);
                        } else {
                            // He sent us an older version which is no use
                            let mut reply = Message::new();
                            reply.set_to("PLAYER");
                            reply.set_from("Arunmor");
                            reply.set_subject("Your recent submission");
                            reply.set_body(concat!(
                                "Thank you for your recent submission of the Revelation virus, and congratulations on finding ",
                                "such a rare item.  Unfortunately, we already have a copy of this version of the virus, so what ",
                                "you have sent is will not prove useful.\n\n",
                                "As such, we are unable to pay you any reward.  If you find any newer versions, please send them ",
                                "to us.",
                            ));
                            reply.send(g);
                        }
                    }
                }

                return true;
            } else if act == 4 {
                // Player might have completed one of our missions
                let body = msg.get_body();

                if body.contains(Self::special_mission_description(SPECIALMISSION_TRACER)) {
                    if !pg(g).completed_tracer {
                        Self::is_mission_complete_tracer(g);
                    }
                } else if body
                    .contains(Self::special_mission_description(SPECIALMISSION_TAKEMETOYOURLEADER))
                {
                    if !pg(g).completed_takemetoyourleader {
                        Self::is_mission_complete_take_me_to_your_leader(g);
                    }
                } else if body
                    .contains(Self::special_mission_description(SPECIALMISSION_ARCINFILTRATION))
                {
                    if !pg(g).completed_arcinfiltration {
                        Self::is_mission_complete_arcinfiltration(g);
                    }
                }

                return true;
            }
        }

        false
    }

    // ---- Revelation / Faith ----

    /// Rolls the dice for success, then calls `run_revelation_with_success`.
    pub fn run_revelation(g: &mut Ctx, ip: &str, version: f32, playerresponsible: bool) {
        if globals::DEMOGAME {
            return;
        }

        // Look up the system attacked
        let (infected, numsystems, numrunningsystems) = {
            let world = g.world();
            let vl = world
                .get_vlocation(ip)
                .expect("plotgenerator: run_revelation: no VLocation");
            let comp = vl
                .get_computer(world)
                .expect("plotgenerator: run_revelation: no computer");
            (
                comp.isinfected_revelation,
                comp.security.num_systems(),
                comp.security.num_running_systems(),
            )
        };

        if infected > 0.0 {
            return;
        }

        // Work out how successful he was
        // Based on correct target and how much security was enabled on the target
        // and if he had been spotted when he released it
        let mut success = if numrunningsystems == numsystems {
            1 // local system only
        } else if numrunningsystems == 0 {
            3 // total success
        } else {
            2 // medium success
        };

        let (traced, trace_in_progress) = {
            let conn = g.player().person.get_connection();
            (conn.traced(), conn.trace_in_progress())
        };
        if traced {
            success = 0;
        } else if trace_in_progress {
            success -= 1;
        }

        pg_mut(g).numuses_revelation += 1;

        Self::run_revelation_with_success(g, ip, version, playerresponsible, success);

        let is_remote_host = g
            .player()
            .person
            .get_remote_host(g.world())
            .map_or(false, |vl| vl.ip == ip);

        if playerresponsible && is_remote_host {
            // The player is still connected - disconnect him!
            Connection::disconnect(g, "PLAYER");
            Connection::reset(g, "PLAYER");

            RemoteInterface::run_new_location(g);
            RemoteInterface::run_screen(g, 2);
        }
    }

    /// C++ overload `RunRevelation ( ip, version, playerresponsible, success )`
    pub fn run_revelation_with_success(
        g: &mut Ctx,
        ip: &str,
        version: f32,
        _playerresponsible: bool,
        success: i32,
    ) {
        if globals::DEMOGAME {
            return;
        }

        // Look up the system attacked
        let (name, infected) = {
            let world = g.world();
            let vl = world
                .get_vlocation(ip)
                .expect("plotgenerator: run_revelation: no VLocation");
            let comp = vl
                .get_computer(world)
                .expect("plotgenerator: run_revelation: no computer");
            (comp.name.clone(), comp.isinfected_revelation)
        };

        if infected > 0.0 {
            return;
        }

        if success > 0 {
            Computer::infect_with_revelation(g, &name, version);
            Self::infected(g, ip);
        }

        if version <= 1.0 {
            Self::news_revelation_used(g, ip, success);
        }

        // The C++ has a commented-out block here (spreading the infection to related computers by
        // success level); it is disabled in the original, so there is nothing to port.
    }

    pub fn run_faith(g: &mut Ctx, ip: &str, version: f32, _playerresponsible: bool) {
        if globals::DEMOGAME {
            return;
        }

        // Look up the system attacked
        let (name, infected) = {
            let world = g.world();
            let vl = world
                .get_vlocation(ip)
                .expect("plotgenerator: run_faith: no VLocation");
            let comp = vl
                .get_computer(world)
                .expect("plotgenerator: run_faith: no computer");
            (comp.name.clone(), comp.isinfected_revelation)
        };

        if infected != 0.0 && version >= infected {
            // Works immediately
            Computer::disinfect_revelation(g, &name);
        } else {
            // Lower version of Faith - may work
            let diff: f32 = infected - version;
            let percent: f64 = 100.0 * (1.0 - f64::from(diff) / 2.0);
            let mut percentchance = percent as i32;
            if percentchance <= 33 {
                percentchance = 33;
            }

            let random = NumberGenerator::random_number(100);
            if random <= percentchance {
                Computer::disinfect_revelation(g, &name);
            }
        }
    }

    /// Adds to the list of infected systems; game over if too many.
    pub fn infected(g: &mut Ctx, ip: &str) {
        if !globals::DEMOGAME {
            let count = {
                let pg = &mut g.world_mut().plotgenerator;
                pg.infected.push(ip.to_string());
                pg.infected.len() as i32
            };

            if count >= REVELATION_RELEASEUNCONTROLLED {
                // Its game over!
                PlotGenerator::revelation_release_uncontrolled(g);
            }
        }
    }

    /// Removes from the list of infected systems.
    pub fn disinfected(g: &mut Ctx, ip: &str) {
        if !globals::DEMOGAME {
            // LList::RemoveData shrinks the list, and the C++ loop still steps past the
            // element that slides into the removed slot; the while loop reproduces that.
            {
                let pg = &mut g.world_mut().plotgenerator;
                let mut i = 0;
                while i < pg.infected.len() {
                    if pg.infected[i] == ip {
                        pg.infected.remove(i);
                    }
                    i += 1;
                }
            }

            let won = {
                let pg = &g.world().plotgenerator;
                pg.infected.is_empty()
                    && pg.revelation_arcbusted
                    && pg.playerloyalty == 1
                    && !pg.player_completed_special_mission(SPECIALMISSION_COUNTERATTACK)
            };

            if won {
                // Player was working for ARC
                // And has now won the game
                send_plot_mail(
                    g,
                    "internal@Arunmor.net",
                    "PLAYER",
                    "Revelation has been destroyed",
                    concat!(
                        "Congratulations would seem to be in order - not only have you ",
                        "stopped Revelation in its tracks, you've also wiped out any trace ",
                        "of the virus on the Internet.\n\n",
                        "As you are probably aware, Andromeda Research Corporation has been ",
                        "raided and won't be doing any more damage at any point in the future. ",
                        "The key members of the company are facing some serious charges.\n\n",
                        "Once again, a sincere thank you from Arunmor Corporation."
                    ),
                );

                PlotGenerator::player_completes_special_mission(g, SPECIALMISSION_COUNTERATTACK);

                // News story
                let mut rundate = Date::default();
                rundate.set_date_from(&g.world().date);
                rundate.advance_hour(3);
                schedule_plot_scene_p3(g, 5, 3, &rundate);
            }
        }
    }
}

/// Schedules RunPlotScene `act`/`scene` for `days` days (skipped when 0) plus a random
/// 0..`hours_random` hours after the current game date (the C++ `Date rundate` pattern).
fn schedule_plot_scene(g: &mut Ctx, act: i32, scene: i32, days: i32, hours_random: i32) {
    let mut rundate = Date::default();
    rundate.set_date_from(&g.world().date);
    if days != 0 {
        rundate.advance_day(days);
    }
    rundate.advance_hour(NumberGenerator::random_number(hours_random));
    schedule_plot_scene_on(g, &rundate, act, scene);
}

/// Schedules RunPlotScene `act`/`scene` to run on `rundate`.
fn schedule_plot_scene_on(g: &mut Ctx, rundate: &Date, act: i32, scene: i32) {
    let mut rns = RunPlotSceneEvent::new();
    rns.set_act_and_scene(act, scene);
    rns.base.set_run_date(rundate);
    g.world_mut().scheduler.schedule_event(UplinkEvent::RunPlotSceneEvent(rns));
}

/// `SetGameSpeed(GAMESPEED_NORMAL)` followed by redrawing the speed buttons.
fn set_normal_speed_and_dirty_buttons(g: &mut Ctx) {
    g.game.set_game_speed(GAMESPEED_NORMAL);
    g.ecl.dirty_button("hud_speed 0");
    g.ecl.dirty_button("hud_speed 1");
    g.ecl.dirty_button("hud_speed 2");
    g.ecl.dirty_button("hud_speed 3");
}

// ---- helpers for the missions and scenes above (private to this file) ----

/// `(name, ip, companyname)` of the computer keyed `name` (C++ `GetComputer` + `UplinkAssert`).
fn lookup_computer(g: &Ctx, name: &str) -> (String, String, String) {
    let comp = g
        .world()
        .get_computer(name)
        .expect("PlotGenerator: computer not found");
    (comp.name.clone(), comp.ip.clone(), comp.companyname.clone())
}

/// C++ `sprintf(username, "temp%c%c%c%c", 'a' + RandomNumber(26), ...)`.
fn random_temp_username() -> String {
    let mut username = String::from("temp");
    for _ in 0..4 {
        username.push((b'a' + NumberGenerator::random_number(26) as u8) as char);
    }
    username
}

/// C++ `Record *record = new Record(); AddField(NAME..); AddField(PASSWORD..);
/// AddField(SECURITY, "3"); recordbank.AddRecord(record);`
fn add_account(comp: &mut Computer, username: &str, password: &str) {
    let mut record = Record::new();
    record.add_field(RECORDBANK_NAME, username);
    record.add_field(RECORDBANK_PASSWORD, password);
    record.add_field(RECORDBANK_SECURITY, "3");
    comp.recordbank.add_record(record);
}

/// C++ `while (databank.FindValidPlacement(file) == -1) databank.SetSize(GetSize() + file->size);`
/// followed by `databank.PutData(file)`.
fn put_file_on(comp: &mut Computer, file: Data) {
    while comp.databank.find_valid_placement(&file) == -1 {
        let newsize = comp.databank.get_size() + file.size;
        comp.databank.set_size(newsize);
    }
    comp.databank.put_data(file);
}

/// First four letters of the company name, as the C++ `companyname[0..3]` gives them
/// (`"%c%c%c%c-DARWIN %d.dat"`).
fn company_prefix4(companyname: &str) -> String {
    let bytes = companyname.as_bytes();
    String::from_utf8_lossy(&bytes[..bytes.len().min(4)]).into_owned()
}

/// C++ `sscanf(title, "%s %d.dat", unused, &thisfileindex)`: skip the first word, then read the
/// integer that follows it. `None` where the C++ would leave the index uninitialised; such a file
/// never matches (the index test rejects it).
fn index_from_data_title(title: &str) -> Option<i32> {
    let after_word = title
        .trim_start()
        .trim_start_matches(|c: char| !c.is_whitespace());
    let rest = after_word.trim_start();
    let (negative, digits_src) = match rest.as_bytes().first() {
        Some(b'-') => (true, &rest[1..]),
        Some(b'+') => (false, &rest[1..]),
        _ => (false, rest),
    };
    let digits: String = digits_src
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    if digits.is_empty() {
        return None;
    }
    let value: i32 = digits.parse().ok()?;
    Some(if negative { -value } else { value })
}

/// The file scan shared by IsMissionComplete_ARCInfiltration and IsMissionComplete_Darwin.
/// Sums the size of each unencrypted data file whose title contains `tag` and whose index
/// (0..10) has not been counted yet.
fn scan_found_files(g: &Ctx, computer_key: &str, tag: &str) -> i32 {
    let numfiles: i32 = 10;
    let mut filefound = [false; 10];
    let mut foundsize = 0;

    let ourcomp = g
        .world()
        .get_computer(computer_key)
        .expect("PlotGenerator: file server");
    for i in 0..ourcomp.databank.get_data_size() {
        if let Some(thisfile) = ourcomp.databank.get_data_file(i) {
            if thisfile.encrypted == 0 && thisfile.title.contains(tag) {
                if let Some(thisfileindex) = index_from_data_title(&thisfile.title) {
                    if thisfileindex >= 0
                        && thisfileindex < numfiles
                        && !filefound[thisfileindex as usize]
                    {
                        foundsize += thisfile.size;
                        filefound[thisfileindex as usize] = true;
                    }
                }
            }
        }
    }

    foundsize
}

/// C++ `Date rundate; rundate.SetDate(&world->date); rundate.AdvanceHour(RandomNumber(span) + 2);`
fn plot_date_hours_later(g: &Ctx, span: i32) -> Date {
    let mut rundate = Date::default();
    rundate.set_date_from(&g.world().date);
    rundate.advance_hour(NumberGenerator::random_number(span) + 2);
    rundate
}

/// C++ `RunPlotSceneEvent *rns = new RunPlotSceneEvent(); SetActAndScene(act, scene);
/// SetRunDate(&rundate); game->GetWorld()->scheduler.ScheduleEvent(rns);`
fn schedule_plot_scene_p3(g: &mut Ctx, act: i32, scene: i32, rundate: &Date) {
    let mut rns = RunPlotSceneEvent::new();
    rns.set_act_and_scene(act, scene);
    rns.base.set_run_date(rundate);
    g.world_mut()
        .scheduler
        .schedule_event(UplinkEvent::RunPlotSceneEvent(rns));
}

/// C++ `Message *msg = new Message(); SetFrom; SetTo; SetSubject; SetBody; Send();`
fn send_plot_mail(g: &mut Ctx, from: &str, to: &str, subject: &str, body: &str) {
    let mut msg = Message::new();
    msg.set_from(from);
    msg.set_to(to);
    msg.set_subject(subject);
    msg.set_body(body);
    msg.send(g);
}
