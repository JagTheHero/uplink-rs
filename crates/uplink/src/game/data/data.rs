//! Port of source/game/data/data.h + source/game/data/data.cpp
//!
//! Game balance data: starting conditions, tick costs, trace speeds, mission
//! probabilities and the static upgrade / rating / gateway-location tables.
//! All `#define`d values are `pub const`s, all tables are `pub static` arrays.
//! Time constants are in game minutes (`7 DAYS` = `7 * 24 * 60`).

#![allow(unused_variables, unused_imports, unused_mut, dead_code)]

/// Hardware / software upgrade table entry (C++ `ComputerUpgrade`; `TYPE` is `type_`).
#[derive(Debug, Clone, Copy)]
pub struct ComputerUpgrade {
    pub name: &'static str,
    pub type_: i32, // unused in HW
    pub cost: i32,
    pub size: i32,
    pub data: i32, // SW version, HW speed or HW capacity or HW bandwidth etc
    pub description: &'static str,
}

/// Rating name / score pair.
#[derive(Debug, Clone, Copy)]
pub struct UplinkRating {
    pub name: &'static str,
    pub score: i32,
}

/// A place the player can pick for the physical gateway.
#[derive(Debug, Clone, Copy)]
pub struct PhysicalGatewayLocation {
    pub city: &'static str,
    pub country: &'static str,
    pub x: i32,
    pub y: i32,
}

/// `0, 0, 0, 24, 2, 2010`: `Date::set_date(second, minute, hour, day, month, year)` arguments.
pub const WORLD_START_DATE: (i32, i32, i32, i32, i32, i32) = (0, 0, 0, 24, 2, 2010);
/// `0, 0, 0, 24, 3, 2010`: `Date::set_date(second, minute, hour, day, month, year)` arguments.
pub const GAME_START_DATE: (i32, i32, i32, i32, i32, i32) = (0, 0, 0, 24, 3, 2010);
pub const COMPANYSIZE_AVERAGE: i32 = 20;
pub const COMPANYSIZE_RANGE: i32 = 20;
pub const COMPANYGROWTH_AVERAGE: i32 = 10; // percent
pub const COMPANYGROWTH_RANGE: i32 = 20; // percent
pub const COMPANYALIGNMENT_AVERAGE: i32 = 0;
pub const COMPANYALIGNMENT_RANGE: i32 = 50;
pub const PLAYER_START_BALANCE: i32 = 3000;
pub const PLAYER_START_CREDITRATING: i32 = 10;
pub const PLAYER_START_UPLINKRATING: i32 = 0;
pub const PLAYER_START_NEUROMANCERRATING: i32 = 5;
pub const PLAYER_START_GATEWAYTYPE: i32 = 0;
pub const PLAYER_START_CPUTYPE: &str = "CPU ( 60 Ghz )";
pub const PLAYER_START_MODEMSPEED: i32 = 1;
pub const PLAYER_START_MEMORYSIZE: i32 = 24;
pub const NUM_STARTING_COMPANIES: i32 = 30;
pub const NUM_STARTING_BANKS: i32 = 7;
pub const NUM_STARTING_PEOPLE: i32 = 50;
pub const NUM_STARTING_AGENTS: i32 = 30;
pub const NUM_STARTING_MISSIONS: i32 = 20;
pub const NUM_STARTING_PHOTOS: i32 = 20;
pub const NUM_STARTING_VOICES: i32 = 4;
pub const TICKSREQUIRED_COPY: i32 = 45;
pub const TICKSREQUIRED_DELETE: i32 = 9;
pub const TICKSREQUIRED_DECRYPT: i32 = 90;
pub const TICKSREQUIRED_DEFRAG: i32 = 3; // (Per memory slot)
pub const TICKSREQUIRED_DICTIONARYHACKER: f32 = 0.2; // (Num ticks required to test 1 word)
pub const TICKSREQUIRED_LOGDELETER: i32 = 60; // (for entire log)
pub const TICKSREQUIRED_LOGUNDELETER: i32 = 60; // (for entire log)
pub const TICKSREQUIRED_LOGMODIFIER: i32 = 50; // (for entire log)
pub const TICKSREQUIRED_ANALYSEPROXY: i32 = 50;
pub const TICKSREQUIRED_DISABLEPROXY: i32 = 100;
pub const TICKSREQUIRED_ANALYSEFIREWALL: i32 = 40;
pub const TICKSREQUIRED_DISABLEFIREWALL: i32 = 80;
pub const TICKSREQUIRED_BYPASSCYPHER: f32 = 0.1; // (Per element in a widthXheight element array)
pub const TICKSREQUIRED_SCANLANSYSTEM: i32 = 70; // (To scan one LAN system)
pub const TICKSREQUIRED_SCANLANLINKS: i32 = 100; // (For all 1024 ports to be scanned)
pub const TICKSREQUIRED_SPOOFLANSYSTEM: i32 = 100;
pub const TICKSREQUIRED_FORCELANLOCK: i32 = 100;
pub const TICKSREQUIRED_LANSCAN: i32 = 300; // (To scan all LAN systems)
pub const TIMEREQUIRED_DELETEONELOG: i32 = 300; // Used in ConsoleScreen
pub const TIMEREQUIRED_DELETEONEGIGAQUAD: i32 = 150; //
pub const TRACESPEED_VARIANCE: f32 = 0.1; // (ie 10%)
pub const TRACESPEED_PUBLICACCESSSERVER: i32 = -1;
pub const TRACESPEED_INTERNALSERVICESMACHINE: i32 = 15;
pub const TRACESPEED_CENTRALMAINFRAME: i32 = 5;
pub const TRACESPEED_PUBLICBANKSERVER: i32 = 5;
pub const TRACESPEED_LAN: i32 = 5;
pub const TRACESPEED_UPLINK_INTERNALSERVICESMACHINE: i32 = 5;
pub const TRACESPEED_UPLINK_TESTMACHINE: i32 = 30;
pub const TRACESPEED_UPLINK_PUBLICACCESSSERVER: i32 = -1;
pub const TRACESPEED_GLOBALCRIMINALDATABASE: i32 = 10;
pub const TRACESPEED_INTERNATIONALSOCIALSECURITYDATABASE: i32 = 15;
pub const TRACESPEED_CENTRALMEDICALDATABASE: i32 = 25;
pub const TRACESPEED_GLOBALINTELLIGENCEAGENCY: i32 = 5;
pub const TRACESPEED_INTERNATIONALACADEMICDATABASE: i32 = 35;
pub const TRACESPEED_INTERNIC: i32 = 15;
pub const TRACESPEED_STOCKMARKET: i32 = 20;
pub const TRACESPEED_PROTOVISION: i32 = 30;
pub const TRACESPEED_MODIFIER_NOACCOUNT: f32 = 0.1; // Player does not have an account on bounce system
pub const TRACESPEED_MODIFIER_HASACCOUNT: f32 = 0.7; // Player has an account on the bounce system
pub const TRACESPEED_MODIFIER_ADMINACCESS: f32 = 1.0; // Player has admin access on bounce system
pub const TRACESPEED_MODIFIER_CENTRALMAINFRAME: f32 = 1.3; // Player is routing through a central mainframe
pub const TRACESPEED_MODIFIER_PUBLICBANKSERVERADMIN: f32 = 1.6; // Player has routing through a bank with admin
pub const HACKDIFFICULTY_VARIANCE: f32 = 0.15; // (ie 15%)
pub const HACKDIFFICULTY_PUBLICACCESSSERVER: i32 = 6;
pub const HACKDIFFICULTY_INTERNALSERVICESMACHINE: i32 = 45;
pub const HACKDIFFICULTY_CENTRALMAINFRAME: i32 = 80;
pub const HACKDIFFICULTY_PUBLICBANKSERVER: i32 = 100;
pub const HACKDIFFICULTY_PUBLICBANKSERVER_ADMIN: i32 = 300;
pub const HACKDIFFICULTY_LOCALMACHINE: i32 = 20;
pub const HACKDIFFICULTY_LANTERMINAL: i32 = 75;
pub const HACKDIFFICULTY_LANAUTHENTICATIONSERVER: i32 = 150;
pub const HACKDIFFICULTY_LANLOGSERVER: i32 = 300;
pub const HACKDIFFICULTY_LANMODEM: i32 = 200;
pub const HACKDIFFICULTY_LANMAINSERVER: i32 = 500;
pub const HACKDIFFICULTY_UPLINK_INTERNALSERVICESMACHINE: i32 = 300;
pub const HACKDIFFICULTY_UPLINK_TESTMACHINE: i32 = 30;
pub const HACKDIFFICULTY_UPLINK_PUBLICACCESSSERVER: i32 = 30;
pub const HACKDIFFICULTY_ARCCENTRALMAINFRAME: i32 = 600;
pub const HACKDIFFICULTY_ARUNMORCENTRALMAINFRAME: i32 = 600;
pub const HACKDIFFICULTY_GLOBALCRIMINALDATABASE: i32 = 180;
pub const HACKDIFFICULTY_INTERNATIONALSOCIALSECURITYDATABASE: i32 = 120;
pub const HACKDIFFICULTY_CENTRALMEDICALDATABASE: i32 = 120;
pub const HACKDIFFICULTY_GLOBALINTELLIGENCEAGENCY: i32 = 450;
pub const HACKDIFFICULTY_INTERNATIONALACADEMICDATABASE: i32 = 90;
pub const HACKDIFFICULTY_INTERNIC: i32 = 70;
pub const HACKDIFFICULTY_STOCKMARKET: i32 = 120;
pub const HACKDIFFICULTY_PROTOVISION: i32 = -1; // Unhackable
pub const MINCOMPANYSIZE_MONITOR: i32 = 1;
pub const MINCOMPANYSIZE_PROXY: i32 = 8;
pub const MINCOMPANYSIZE_FIREWALL: i32 = 10;
pub const MAXCOMPANYSIZE_WARNINGMAIL: i32 = 5;
pub const MAXCOMPANYSIZE_FINE: i32 = 12;
pub const PROB_GENERATETRACEHACKERMISSION: i32 = 30; // ie X% of crimes result in a trace hacker mission
pub const PAYMENT_MISSION_VARIANCE: f32 = 0.3; // ie 30% of payment
pub const PAYMENT_MISSION_STEALFILE: i32 = 900;
pub const PAYMENT_MISSION_STEALALLFILES: i32 = 1500;
pub const PAYMENT_MISSION_DESTROYFILE: i32 = 800;
pub const PAYMENT_MISSION_DESTROYALLFILES: i32 = 1400;
pub const PAYMENT_MISSION_FINDDATA: i32 = 1000;
pub const PAYMENT_MISSION_FINDDATA_FINANCIAL: i32 = 1200;
pub const PAYMENT_MISSION_CHANGEDATA: i32 = 1000;
pub const PAYMENT_MISSION_CHANGEDATA_ACADEMIC: i32 = 1000;
pub const PAYMENT_MISSION_CHANGEDATA_SOCIAL: i32 = 1200;
pub const PAYMENT_MISSION_CHANGEDATA_CRIMINAL: i32 = 1500;
pub const PAYMENT_MISSION_FRAMEUSER: i32 = 2200;
pub const PAYMENT_MISSION_TRACEUSER: i32 = 1800;
pub const PAYMENT_MISSION_CHANGEACCOUNT: i32 = 1700;
pub const PAYMENT_MISSION_REMOVECOMPUTER: i32 = 1600;
pub const PAYMENT_MISSION_REMOVECOMPANY: i32 = 2000;
pub const PAYMENT_MISSION_REMOVEUSER: i32 = 1900;
pub const PAYMENT_SPECIALMISSION_BACKFIRE: i32 = 15000;
pub const PAYMENT_SPECIALMISSION_TRACER: i32 = 10000;
pub const PAYMENT_SPECIALMISSION_TAKEMETOYOURLEADER: i32 = 30000;
pub const PAYMENT_SPECIALMISSION_ARCINFILTRATION: i32 = 30000;
pub const PAYMENT_SPECIALMISSION_COUNTERATTACK: i32 = 50000;
pub const PAYMENT_SPECIALMISSION_MAIDENFLIGHT: i32 = 10000;
pub const PAYMENT_SPECIALMISSION_DARWIN: i32 = 15000;
pub const PAYMENT_SPECIALMISSION_SAVEITFORTHEJURY: i32 = 20000;
pub const PAYMENT_SPECIALMISSION_SHINYHAMMER: i32 = 30000;
pub const PAYMENT_SPECIALMISSION_GRANDTOUR: i32 = 50000;
pub const PAYMENT_SPECIALMISSION_MOLE: i32 = 50000;
pub const DIFFICULTY_MISSION_VARIANCE: i32 = 2; // Random Normal Number variance
pub const MINDIFFICULTY_MISSION_STEALFILE: i32 = 2;
pub const MINDIFFICULTY_MISSION_STEALALLFILES: i32 = 5;
pub const MINDIFFICULTY_MISSION_DESTROYFILE: i32 = 2;
pub const MINDIFFICULTY_MISSION_DESTROYALLFILES: i32 = 5;
pub const MINDIFFICULTY_MISSION_FINDDATA: i32 = 2;
pub const MINDIFFICULTY_MISSION_FINDDATA_FINANCIAL: i32 = 5;
pub const MINDIFFICULTY_MISSION_CHANGEDATA: i32 = 3;
pub const MINDIFFICULTY_MISSION_CHANGEDATA_ACADEMIC: i32 = 3;
pub const MINDIFFICULTY_MISSION_CHANGEDATA_SOCIAL: i32 = 4;
pub const MINDIFFICULTY_MISSION_CHANGEDATA_CRIMINAL: i32 = 5;
pub const MINDIFFICULTY_MISSION_FRAMEUSER: i32 = 9;
pub const MINDIFFICULTY_MISSION_TRACEUSER: i32 = 7;
pub const MINDIFFICULTY_MISSION_TRACEUSER_BANKFRAUD: i32 = 8;
pub const MINDIFFICULTY_MISSION_CHANGEACCOUNT: i32 = 7;
pub const MINDIFFICULTY_MISSION_REMOVECOMPUTER: i32 = 6;
pub const MINDIFFICULTY_MISSION_REMOVECOMPANY: i32 = 8;
pub const MINDIFFICULTY_MISSION_REMOVEUSER: i32 = 8;
pub const AGENT_UPLINKRATINGAVERAGE: i32 = 7;
pub const AGENT_UPLINKRATINGVARIANCE: i32 = 7;
pub const MINREQUIREDRATING_DELETELOGLEVEL1: i32 = 2;
pub const MINREQUIREDRATING_DELETELOGLEVEL2: i32 = 3;
pub const MINREQUIREDRATING_DELETELOGLEVEL3: i32 = 4;
pub const MINREQUIREDRATING_UNDELETELOGLEVEL1: i32 = 7;
pub const MINREQUIREDRATING_UNDELETELOGLEVEL2: i32 = 7;
pub const MINREQUIREDRATING_UNDELETELOGLEVEL3: i32 = 9;
pub const MINREQUIREDRATING_HACKBANKSERVER: i32 = 6;
pub const MINREQUIREDRATING_HACKGOVERNMENTCOMPUTER: i32 = 8;
pub const TIME_TOPAYLEGALFINE: i32 = 7 * 24 * 60; // You must pay before this time or face legal action
pub const TIME_LEGALACTION: i32 = 3 * 60; // They come to arrest you after this time
pub const TIME_LEGALACTION_WARNING: i32 = 2 * 1; // You are given warning this many minutes before hand
pub const TIME_TACTICALACTION: i32 = 5 * 1; // They send in the heavies to pick you up.  Nasty.
pub const TIME_TACTICALACTION_WARNING: i32 = 1 * 1; // You hear them at the door
pub const TIME_TOINSTALLHARDWARE: i32 = 14 * 60; // Time for engineer to visit your gateway
pub const TIME_TOINSTALLHARDWARE_WARNING: i32 = 30 * 1; // He mails you 30 minutes before finishing.
pub const TIME_TOCHANGEGATEWAY: i32 = 24 * 60; // No warnings.  New gateway is now ready.
pub const TIME_TOEXPIRELOGS: i32 = 40 * 24 * 60; // After this time logs will be deleted
pub const TIME_TOEXPIRENEWS: i32 = 30 * 24 * 60; // After this time news will be deleted
pub const TIME_TOEXPIREMISSIONS: i32 = 30 * 24 * 60; // After this time mission will be withdrawn
pub const TIME_TODEMOGAMEOVER: i32 = 0 * 1; // Player gets this long AFTER reaching the max demo rating
pub const TIME_TOCOVERBANKROBBERY: i32 = 2 * 1; // Player has this long to cover his tracks
pub const TIME_REVELATIONREPRODUCE: i32 = 3 * 1; // Doubles in size this often
pub const TIME_ARCBUSTED_WITHPLAYER: i32 = 15 * 1; // Player has this long to plant Revelation
pub const TIME_ARCBUSTED_WITHOUTPLAYER: i32 = 10 * 1; // Player must defend against Revelation for this long
pub const FREQUENCY_GENERATENEWMISSION: i32 = 12 * 60;
pub const FREQUENCY_CHECKFORSECURITYBREACHES: i32 = 8 * 60;
pub const FREQUENCY_CHECKMISSIONDUEDATES: i32 = 1 * 24 * 60;
pub const FREQUENCY_GIVEMISSIONTONPC: i32 = 8 * 60;
pub const FREQUENCY_EXPIREOLDSTUFF: i32 = 7 * 24 * 60;
pub const FREQUENCY_ADDINTERESTONLOANS: i32 = 30 * 24 * 60;
pub const FREQUENCY_DEMOGENERATENEWMISSION: i32 = 4 * 60;
pub const NUM_STARTINGSOFTWAREUPGRADES: i32 = 72;
pub const NUM_STARTINGHARDWAREUPGRADES: i32 = 19;
pub const COST_UPLINK_PERMONTH: i32 = 300;
pub const COST_UPLINK_NEWGATEWAY: i32 = 1000;
pub const GATEWAY_PARTEXCHANGEVALUE: f32 = 0.75; // Percentage of purchase value
pub const SMALLLOAN_MAX: i32 = 3000;
pub const SMALLLOAN_INTEREST: f32 = 0.2;
pub const MEDIUMLOAN_MAX: i32 = 7000;
pub const MEDIUMLOAN_INTEREST: f32 = 0.4;
pub const LARGELOAN_MAX: i32 = 10000;
pub const LARGELOAN_INTEREST: f32 = 0.7;
pub const MAXLOAN_INTEREST: f32 = 1.0;
pub const NUM_UPLINKRATINGS: i32 = 17;
pub const NUM_NEUROMANCERRATINGS: i32 = 11;
pub const NEUROMANCERCHANGE_GATEWAYNEARMISS: i32 = 150; // Feds sieze your old gateway and you get away scott free
pub const UPLINKCHANGE_GATEWAYNEARMISS: i32 = -30;
pub const NEUROMANCERCHANGE_ROBBANK: i32 = 0;
pub const UPLINKCHANGE_ROBBANK: i32 = 150;
pub const DEMO_MAXUPLINKRATING: i32 = 4;
pub const DEMO_MAXGATEWAY: i32 = 2;
pub const WAREZ_MAXUPLINKRATING: i32 = 5;
pub const TIME_TOWAREZGAMEOVER: i32 = 15 * 1;
pub const IP_LOCALHOST: &str = "127.0.0.1";
pub const IP_INTERNIC: &str = "458.615.48.651";
pub const IP_ACADEMICDATABASE: &str = "443.65.765.2";
pub const IP_GLOBALCRIMINALDATABASE: &str = "785.234.87.124";
pub const IP_SOCIALSECURITYDATABASE: &str = "653.76.235.432";
pub const IP_CENTRALMEDICALDATABASE: &str = "432.543.12.544";
pub const IP_STOCKMARKETSYSTEM: &str = "456.789.159.459";
pub const IP_PROTOVISION: &str = "284.345.42.283";
pub const IP_OCP: &str = "265.125.767.1";
pub const IP_SJGAMES: &str = "849.23.459.24";
pub const IP_INTROVERSION: &str = "128.128.128.128";
pub const IP_UPLINKPUBLICACCESSSERVER: &str = "234.773.0.666";
pub const IP_UPLINKCREDITSMACHINE: &str = "128.185.0.2";
pub const IP_UPLINKINTERNALSERVICES: &str = "128.185.0.8";
pub const IP_UPLINKTESTMACHINE: &str = "128.185.0.4";
pub const NAME_UPLINKINTERNALSERVICES: &str = "Uplink Internal Services System"; // Mistake here, everyone else is
pub const NUM_PHYSICALGATEWAYLOCATIONS: i32 = 8;
pub const PERCENTAGE_PEOPLEWITHCONVICTIONS: i32 = 20;
pub const PERCENTAGE_AGENTSWITHCONVICTIONS: i32 = 40;
pub const REVELATION_RELEASEUNCONTROLLED: i32 = 20;
pub const LAN_SUBNETRANGE: i32 = 1024;
pub const LAN_LINKPORTRANGE: i32 = 1024;
pub const RADIOTRANSMITTER_MINRANGE: i32 = 140; // Ghz
pub const RADIOTRANSMITTER_MAXRANGE: i32 = 180; // Ghz
pub const WAREZ_MAXPLAYTIME: i32 = 60 * 60 * 1000; // 60 mins in milliseconds

pub static HACKDIFFICULTY_SECURITYMODIFIER: [f32; 11] =
    [0.0, 2.0, 1.5, 1.2, 1.0, 1.0, 0.9, 0.7, 0.5, 0.5, 0.5];
pub static PROB_MISSION_STEALFILE: [i32; 17] =
    [30, 40, 30, 15, 5, 30, 15, 10, 0, 0, 0, 5, 5, 5, 5, 5, 5];
pub static PROB_MISSION_DESTROYFILE: [i32; 17] =
    [30, 40, 30, 15, 40, 10, 15, 5, 0, 0, 0, 5, 5, 5, 5, 5, 5];
pub static PROB_MISSION_CHANGEDATA: [i32; 17] =
    [25, 15, 25, 60, 40, 30, 20, 15, 15, 5, 0, 5, 5, 5, 5, 5, 5];
pub static PROB_MISSION_FINDDATA: [i32; 17] =
    [15, 5, 15, 10, 15, 30, 15, 15, 5, 0, 0, 5, 5, 5, 5, 5, 5];
pub static PROB_MISSION_REMOVECOMPUTER: [i32; 17] =
    [0, 0, 0, 0, 0, 0, 35, 25, 20, 5, 10, 20, 20, 20, 20, 20, 20];
pub static PROB_MISSION_CHANGEACCOUNT: [i32; 17] =
    [0, 0, 0, 0, 0, 0, 0, 30, 20, 25, 30, 20, 20, 20, 20, 20, 20];
pub static PROB_MISSION_REMOVEUSER: [i32; 17] =
    [0, 0, 0, 0, 0, 0, 0, 0, 40, 25, 30, 20, 20, 20, 20, 20, 20];
pub static PROB_MISSION_FRAMEUSER: [i32; 17] =
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 40, 30, 20, 20, 20, 20, 20, 20];
pub static PROB_MISSION_REMOVECOMPANY: [i32; 17] =
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
pub static PROB_MISSION_SPECIAL: [i32; 17] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
pub static PROB_MISSION_TRACEUSER: [i32; 17] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

pub static UPLINKRATING: [UplinkRating; 17] = [
    UplinkRating {
        name: "Unregistered",
        score: 0,
    },
    UplinkRating {
        name: "Registered",
        score: 1,
    },
    UplinkRating {
        name: "Beginner",
        score: 2,
    },
    UplinkRating {
        name: "Novice",
        score: 6,
    },
    UplinkRating {
        name: "Confident",
        score: 15,
    },
    UplinkRating {
        name: "Intermediate",
        score: 35,
    },
    UplinkRating {
        name: "Skilled",
        score: 60,
    },
    UplinkRating {
        name: "Experienced",
        score: 90,
    },
    UplinkRating {
        name: "Knowledgeable",
        score: 125,
    },
    UplinkRating {
        name: "Uber-Skilled",
        score: 160,
    },
    UplinkRating {
        name: "Professional",
        score: 220,
    },
    UplinkRating {
        name: "Elite",
        score: 300,
    },
    UplinkRating {
        name: "Mage",
        score: 400,
    },
    UplinkRating {
        name: "Expert",
        score: 600,
    },
    UplinkRating {
        name: "Veteren",
        score: 1000,
    },
    UplinkRating {
        name: "Techno-mage",
        score: 1500,
    },
    UplinkRating {
        name: "TERMINAL",
        score: 2500,
    },
];

pub static NEUROMANCERRATING: [UplinkRating; 11] = [
    UplinkRating {
        name: "Morally bankrupt",
        score: -1024,
    },
    UplinkRating {
        name: "Sociopathic",
        score: -512,
    },
    UplinkRating {
        name: "Indiscriminate",
        score: -128,
    },
    UplinkRating {
        name: "Notorious",
        score: -32,
    },
    UplinkRating {
        name: "Untrustworthy",
        score: -16,
    },
    UplinkRating {
        name: "Neutral",
        score: 0,
    },
    UplinkRating {
        name: "Agressive",
        score: 32,
    },
    UplinkRating {
        name: "Single minded",
        score: 64,
    },
    UplinkRating {
        name: "Activist",
        score: 256,
    },
    UplinkRating {
        name: "Anarchic",
        score: 1024,
    },
    UplinkRating {
        name: "Revolutionary",
        score: 2048,
    },
];

pub static NEUROMANCERCHANGE: [i32; 11] = [0, 0, 5, 0, -5, -20, -30, -10, 30, 60, -60];

pub static PHYSICALGATEWAYLOCATIONS: [PhysicalGatewayLocation; 8] = [
    PhysicalGatewayLocation {
        city: "London",
        country: "United Kingdom",
        x: 282,
        y: 69,
    },
    PhysicalGatewayLocation {
        city: "Tokyo",
        country: "Japan",
        x: 514,
        y: 104,
    },
    PhysicalGatewayLocation {
        city: "Los Angeles",
        country: "USA",
        x: 91,
        y: 125,
    },
    PhysicalGatewayLocation {
        city: "New York",
        country: "USA",
        x: 171,
        y: 98,
    },
    PhysicalGatewayLocation {
        city: "Chicago",
        country: "USA",
        x: 138,
        y: 88,
    },
    PhysicalGatewayLocation {
        city: "Moscow",
        country: "Russia",
        x: 331,
        y: 70,
    },
    PhysicalGatewayLocation {
        city: "Hong Kong",
        country: "China",
        x: 485,
        y: 133,
    },
    PhysicalGatewayLocation {
        city: "Sydney",
        country: "Australia",
        x: 547,
        y: 244,
    },
];

pub static SOFTWARE_UPGRADES: [ComputerUpgrade; 72] = [
    ComputerUpgrade { name: "Decrypter", type_: 4, cost: 800, size: 2, data: 1, description: "DECRYPTER V1.0\nTYPE: software.Cracker\n\nDecrypts any file encrypted with an Encrypter V1.0" },
    ComputerUpgrade { name: "Decrypter", type_: 4, cost: 1200, size: 2, data: 2, description: "DECRYPTER V2.0\nTYPE: software.Cracker\n\nDecrypts any file encrypted with an Encrypter V2.0 or V1.0" },
    ComputerUpgrade { name: "Decrypter", type_: 4, cost: 1600, size: 2, data: 3, description: "DECRYPTER V3.0\nTYPE: software.Cracker\n\nDecrypts any file encrypted with an Encrypter V3.0\nor below" },
    ComputerUpgrade { name: "Decrypter", type_: 4, cost: 2200, size: 2, data: 4, description: "DECRYPTER V4.0\nTYPE: software.Cracker\n\nDecrypts any file encrypted with an Encrypter V4.0\nor below" },
    ComputerUpgrade { name: "Decrypter", type_: 4, cost: 3000, size: 3, data: 5, description: "DECRYPTER V5.0\nTYPE: software.Cracker\n\nDecrypts any file encrypted with an Encrypter V5.0\nor below" },
    ComputerUpgrade { name: "Decrypter", type_: 4, cost: 7000, size: 4, data: 6, description: "DECRYPTER V6.0\nTYPE: software.Cracker\n\nDecrypts any file encrypted with an Encrypter V6.0\nor below" },
    ComputerUpgrade { name: "Decrypter", type_: 4, cost: 15000, size: 5, data: 7, description: "DECRYPTER V7.0\nTYPE: software.Cracker\n\nDecrypts any file encrypted with an Encrypter V7.0\nor below" },
    ComputerUpgrade { name: "Decypher", type_: 4, cost: 3000, size: 2, data: 1, description: "DECYPHER V1.0\nTYPE: software.Cracker\n\nBreaks through Encryption Cyphers\nUses a Quadratic Decryption Algorithm to force crack an Elliptic-curve Encryption Cypher" },
    ComputerUpgrade { name: "Decypher", type_: 4, cost: 5000, size: 2, data: 2, description: "DECYPHER V2.0\nTYPE: software.Cracker\n\nBreaks through Encryption Cyphers\nThis is a faster implementation of the Quadratic Decryption Algorithm" },
    ComputerUpgrade { name: "Decypher", type_: 4, cost: 8000, size: 2, data: 3, description: "DECYPHER V3.0\nTYPE: software.Cracker\n\nBreaks through Encryption Cyphers\nThis is the fastest version of this item of Software" },
    ComputerUpgrade { name: "Dictionary_Hacker", type_: 4, cost: 1000, size: 4, data: 1, description: "DICTIONARY HACKER V1.0\nTYPE: software.Cracker\n\nA cheap and fast method of breaking passwords\nThis software tool tries every word in a ten-thousand word dictionary as a password. It is faster than a Password Breaker, but not guaranteed to find an answer." },
    ComputerUpgrade { name: "File_Copier", type_: 1, cost: 100, size: 1, data: 1, description: "FILE COPIER V1.0\nTYPE: software.FileUtil\n\nCopies programs and data from one databank to another\nStandard software tool given free to all Agents" },
    ComputerUpgrade { name: "File_Deleter", type_: 1, cost: 100, size: 1, data: 1, description: "FILE DELETER V1.0\nTYPE: software.FileUtil\n\nDeletes programs and data from a databank\nStandard software tool given free to all Agents" },
    ComputerUpgrade { name: "Defrag", type_: 1, cost: 5000, size: 2, data: 1, description: "DEFRAG\nTYPE: software.FileUtil\n\nPushes all of your files together at the top of your memory banks\nand frees up large blocks of space for use" },
    ComputerUpgrade { name: "Firewall_Bypass", type_: 5, cost: 3000, size: 1, data: 1, description: "FIREWALL BYPASS V1.0\nTYPE: software.Bypasser\n\nActively bypasses firewall systems with a security level of 1\nAll bypassers require a 'HUD:ConnectionAnalysis' upgrade first" },
    ComputerUpgrade { name: "Firewall_Bypass", type_: 5, cost: 4000, size: 1, data: 2, description: "FIREWALL BYPASS V2.0\nTYPE: software.Bypasser\n\nActively bypasses firewall systems with a security level of 2 or below\nAll bypassers require a 'HUD:ConnectionAnalysis' upgrade first" },
    ComputerUpgrade { name: "Firewall_Bypass", type_: 5, cost: 6000, size: 1, data: 3, description: "FIREWALL BYPASS V3.0\nTYPE: software.Bypasser\n\nActively bypasses firewall systems with a security level of 3 or below\nAll bypassers require a 'HUD:ConnectionAnalysis' upgrade first" },
    ComputerUpgrade { name: "Firewall_Bypass", type_: 5, cost: 8000, size: 1, data: 4, description: "FIREWALL BYPASS V4.0\nTYPE: software.Bypasser\n\nActively bypasses firewall systems with a security level of 4 or below\nAll bypassers require a 'HUD:ConnectionAnalysis' upgrade first" },
    ComputerUpgrade { name: "Firewall_Bypass", type_: 5, cost: 10000, size: 1, data: 5, description: "FIREWALL BYPASS V5.0\nTYPE: software.Bypasser\n\nActively bypasses firewall systems with a security level of 5 or below\nAll bypassers require a 'HUD:ConnectionAnalysis' upgrade first" },
    ComputerUpgrade { name: "Firewall_Disable", type_: 3, cost: 2000, size: 1, data: 1, description: "FIREWALL DISABLE V1.0\nTYPE: software.Security\n\nDisables a firewall with a security level of 1\nThis action will be detected immediately" },
    ComputerUpgrade { name: "Firewall_Disable", type_: 3, cost: 3000, size: 1, data: 2, description: "FIREWALL DISABLE V2.0\nTYPE: software.Security\n\nDisables a firewall with a security level of 2 or below\nThis action will be detected immediately" },
    ComputerUpgrade { name: "Firewall_Disable", type_: 3, cost: 4000, size: 1, data: 3, description: "FIREWALL DISABLE V3.0\nTYPE: software.Security\n\nDisables a firewall with a security level of 3 or below\nThis action will be detected immediately" },
    ComputerUpgrade { name: "Firewall_Disable", type_: 3, cost: 6000, size: 2, data: 4, description: "FIREWALL DISABLE V4.0\nTYPE: software.Security\n\nDisables a firewall with a security level of 4 or below\nThis action will be detected immediately" },
    ComputerUpgrade { name: "Firewall_Disable", type_: 3, cost: 8000, size: 3, data: 5, description: "FIREWALL DISABLE V5.0\nTYPE: software.Security\n\nDisables a firewall with a security level of 5 or below\nThis action will be detected immediately" },
    ComputerUpgrade { name: "HUD_ConnectionAnalysis", type_: 9, cost: 20000, size: 3, data: 1, description: "CONNECTION ANALYSIS V1.0\nTYPE: software.HUDUpgrade\n\nUpgrades your HUD so that you can see the security systems installed on your target computer, and launch Bypass software at them" },
    ComputerUpgrade { name: "HUD_IRC-Client", type_: 9, cost: 4000, size: 2, data: 1, description: "IRC CLIENT V1.0\nTYPE: software.HUDUpgrade\n\nUpgrades your HUD with a new screen, in which you can participate in Internet Relay Chat. This allows you to talk to other hackers for help.\nYou will also need an Internet Connection on your local machine." },
    ComputerUpgrade { name: "HUD_MapShowTrace", type_: 9, cost: 5000, size: 1, data: 1, description: "MAP SHOW TRACE V1.0\nTYPE: software.HUDUpgrade\n\nUpgrades your HUD so that your map shows the progress of any trace on your connection.\nReplaces the Trace Tracker." },
    ComputerUpgrade { name: "HUD_LANView", type_: 9, cost: 50000, size: 5, data: 1, description: "LAN VIEW V1.0\nTYPE: software.HUDUpgrade\n\nUpgrades your HUD so that you can analyse and connect to Local Area Network (LAN) systems.\nThis upgrade is required before you can install and run any LAN software." },
    ComputerUpgrade { name: "IP_Lookup", type_: 10, cost: 500, size: 1, data: 1, description: "IP LOOKUP V1.0\nTYPE: software.Other\n\nConverts a raw IP number into an Internet address and adds it to your links" },
    ComputerUpgrade { name: "IP_Probe", type_: 10, cost: 2000, size: 3, data: 1, description: "IP PROBE V1.0\nTYPE: software.Other\n\nScans a given IP address for security systems and reports the types in use" },
    ComputerUpgrade { name: "IP_Probe", type_: 10, cost: 4000, size: 3, data: 2, description: "IP PROBE V2.0\nTYPE: software.Other\n\nScans a given IP address for security systems and reports the types in use and their version information" },
    ComputerUpgrade { name: "IP_Probe", type_: 10, cost: 5000, size: 3, data: 3, description: "IP PROBE V3.0\nTYPE: software.Other\n\nScans a given IP address for security systems and reports the types in use, their version information and their status" },
    ComputerUpgrade { name: "LAN_Probe", type_: 6, cost: 15000, size: 3, data: 1, description: "LAN PROBE V1.0\nTYPE: Software.LAN\n\nScans a single system on a LAN, and the links running from it." },
    ComputerUpgrade { name: "LAN_Probe", type_: 6, cost: 20000, size: 3, data: 2, description: "LAN PROBE V2.0\nTYPE: Software.LAN\n\nScans a single system on a LAN, and the links running from it." },
    ComputerUpgrade { name: "LAN_Probe", type_: 6, cost: 30000, size: 4, data: 3, description: "LAN PROBE V3.0\nTYPE: Software.LAN\n\nScans a single system on a LAN, and the links running from it." },
    ComputerUpgrade { name: "LAN_Scan", type_: 6, cost: 10000, size: 2, data: 1, description: "LAN SCAN V1.0\nTYPE: Software.LAN\n\nScans an entire LAN, looking for systems" },
    ComputerUpgrade { name: "LAN_Scan", type_: 6, cost: 15000, size: 2, data: 2, description: "LAN SCAN V2.0\nTYPE: Software.LAN\n\nScans an entire LAN, looking for systems" },
    ComputerUpgrade { name: "LAN_Scan", type_: 6, cost: 25000, size: 2, data: 3, description: "LAN SCAN V3.0\nTYPE: Software.LAN\n\nScans an entire LAN, looking for systems" },
    ComputerUpgrade { name: "LAN_Spoof", type_: 6, cost: 20000, size: 2, data: 1, description: "LAN SPOOF V1.0\nTYPE: Software.LAN\n\nSpoofs a system on the network, fooling the network itself\ninto believing you are that system.\nWorks on level 1 systems only." },
    ComputerUpgrade { name: "LAN_Spoof", type_: 6, cost: 30000, size: 3, data: 2, description: "LAN SPOOF V2.0\nTYPE: Software.LAN\n\nSpoofs a system on the network, fooling the network itself\ninto believing you are that system.\nWorks on level 1 and 2 systems." },
    ComputerUpgrade { name: "LAN_Spoof", type_: 6, cost: 45000, size: 5, data: 3, description: "LAN SPOOF V3.0\nTYPE: Software.LAN\n\nSpoofs a system on the network, fooling the network itself\ninto believing you are that system.\nWorks on all LAN systems." },
    ComputerUpgrade { name: "LAN_Force", type_: 6, cost: 15000, size: 2, data: 1, description: "LAN FORCE V1.0\nTYPE: Software.LAN\n\nThis tool will force open any lock system you might\nencounter on a LAN system.\nThe sys Admin will probably notice this." },
    ComputerUpgrade { name: "LAN_Force", type_: 6, cost: 20000, size: 3, data: 2, description: "LAN FORCE V2.0\nTYPE: Software.LAN\n\nThis tool will force open any lock system you might\nencounter on a LAN system.\nThe sys Admin will probably notice this." },
    ComputerUpgrade { name: "LAN_Force", type_: 6, cost: 25000, size: 4, data: 3, description: "LAN FORCE V3.0\nTYPE: Software.LAN\n\nThis tool will force open any lock system you might\nencounter on a LAN system.\nThe sys Admin will probably notice this." },
    ComputerUpgrade { name: "Log_Deleter", type_: 3, cost: 500, size: 1, data: 1, description: "LOG DELETER V1.0\nTYPE: software.Security\n\nDeletes the contents of an access log from a computer\nLeaves an empty log behind" },
    ComputerUpgrade { name: "Log_Deleter", type_: 3, cost: 1000, size: 1, data: 2, description: "LOG DELETER V2.0\nTYPE: software.Security\n\nDeletes an access-log from a computer\nLeaves a distinctive blank space behind" },
    ComputerUpgrade { name: "Log_Deleter", type_: 3, cost: 2000, size: 1, data: 3, description: "LOG DELETER V3.0\nTYPE: software.Security\n\nCopies another user's legitimate access log over the top of of your own.\nVery difficult to detect." },
    ComputerUpgrade { name: "Log_Deleter", type_: 3, cost: 4000, size: 1, data: 4, description: "LOG DELETER V4.0\nTYPE: software.Security\n\nDeletes an access-log from a computer\nMoves other logs up to cover empty space.\nImpossible to detect." },
    ComputerUpgrade { name: "Log_Modifier", type_: 3, cost: 4000, size: 2, data: 1, description: "LOG MODIFIER V1.0\nTYPE: software.Security\n\nAllows you to make modifications to an existing log" },
    ComputerUpgrade { name: "Log_Modifier", type_: 3, cost: 6000, size: 2, data: 2, description: "LOG MODIFIER V2.0\nTYPE: software.Security\n\nAllows you to make modifications to an existing log, and create new logs in blank spaces" },
    ComputerUpgrade { name: "Log_UnDeleter", type_: 3, cost: 5000, size: 1, data: 1, description: "LOG UNDELETER V1.0\nTYPE: software.Security\n\nRecovers previously deleted logs" },
    ComputerUpgrade { name: "Monitor_Bypass", type_: 5, cost: 10000, size: 1, data: 1, description: "MONITOR BYPASS V1.0\nTYPE: software.Bypasser\n\nActively bypasses a monitor with a security level of 1\nAll bypassers require a 'HUD:ConnectionAnalysis' upgrade first" },
    ComputerUpgrade { name: "Monitor_Bypass", type_: 5, cost: 12000, size: 1, data: 2, description: "MONITOR BYPASS V2.0\nTYPE: software.Bypasser\n\nActively bypasses a monitor with a security level of 2 or below\nAll bypassers require a 'HUD:ConnectionAnalysis' upgrade first" },
    ComputerUpgrade { name: "Monitor_Bypass", type_: 5, cost: 16000, size: 1, data: 3, description: "MONITOR BYPASS V3.0\nTYPE: software.Bypasser\n\nActively bypasses a monitor with a security level of 3 or below\nAll bypassers require a 'HUD:ConnectionAnalysis' upgrade first" },
    ComputerUpgrade { name: "Monitor_Bypass", type_: 5, cost: 20000, size: 1, data: 4, description: "MONITOR BYPASS V4.0\nTYPE: software.Bypasser\n\nActively bypasses a monitor with a security level of 4 or below\nAll bypassers require a 'HUD:ConnectionAnalysis' upgrade first" },
    ComputerUpgrade { name: "Monitor_Bypass", type_: 5, cost: 25000, size: 1, data: 5, description: "MONITOR BYPASS V5.0\nTYPE: software.Bypasser\n\nActively bypasses a monitor with a security level of 5 or below\nAll bypassers require a 'HUD:ConnectionAnalysis' upgrade first" },
    ComputerUpgrade { name: "Password_Breaker", type_: 4, cost: 1500, size: 2, data: 1, description: "PASSWORD BREAKER V1.0\nTYPE: software.Cracker\n\nHacks past password protection screens\nNot as fast as a Dictionary Hacker but is guaranteed to find the password eventually" },
    ComputerUpgrade { name: "Proxy_Bypass", type_: 5, cost: 6000, size: 1, data: 1, description: "PROXY BYPASS V1.0\nTYPE: software.Bypasser\n\nActively bypasses a proxy server with a security level of 1\nAll bypassers require a 'HUD:ConnectionAnalysis' upgrade first" },
    ComputerUpgrade { name: "Proxy_Bypass", type_: 5, cost: 8000, size: 1, data: 2, description: "PROXY BYPASS V2.0\nTYPE: software.Bypasser\n\nActively bypasses a proxy server with a security level of 2 or below\nAll bypassers require a 'HUD:ConnectionAnalysis' upgrade first" },
    ComputerUpgrade { name: "Proxy_Bypass", type_: 5, cost: 12000, size: 1, data: 3, description: "PROXY BYPASS V3.0\nTYPE: software.Bypasser\n\nActively bypasses a proxy server with a security level of 3 or below\nAll bypassers require a 'HUD:ConnectionAnalysis' upgrade first" },
    ComputerUpgrade { name: "Proxy_Bypass", type_: 5, cost: 16000, size: 1, data: 4, description: "PROXY BYPASS V4.0\nTYPE: software.Bypasser\n\nActively bypasses a proxy server with a security level of 4 or below\nAll bypassers require a 'HUD:ConnectionAnalysis' upgrade first" },
    ComputerUpgrade { name: "Proxy_Bypass", type_: 5, cost: 20000, size: 1, data: 5, description: "PROXY BYPASS V5.0\nTYPE: software.Bypasser\n\nActively bypasses a proxy server with a security level of 5 or below\nAll bypassers require a 'HUD:ConnectionAnalysis' upgrade first" },
    ComputerUpgrade { name: "Proxy_Disable", type_: 3, cost: 3000, size: 1, data: 1, description: "PROXY DISABLE V1.0\nTYPE: software.Security\n\nDisables a proxy with a security level of 1\nThis action will be detected immediately" },
    ComputerUpgrade { name: "Proxy_Disable", type_: 3, cost: 4000, size: 1, data: 2, description: "PROXY DISABLE V2.0\nTYPE: software.Security\n\nDisables a proxy with a security level of 2 or below\nThis action will be detected immediately" },
    ComputerUpgrade { name: "Proxy_Disable", type_: 3, cost: 6000, size: 1, data: 3, description: "PROXY DISABLE V3.0\nTYPE: software.Security\n\nDisables a proxy with a security level of 3 or below\nThis action will be detected immediately" },
    ComputerUpgrade { name: "Proxy_Disable", type_: 3, cost: 8000, size: 2, data: 4, description: "PROXY DISABLE V4.0\nTYPE: software.Security\n\nDisables a proxy with a security level of 4 or below\nThis action will be detected immediately" },
    ComputerUpgrade { name: "Proxy_Disable", type_: 3, cost: 10000, size: 3, data: 5, description: "PROXY DISABLE V5.0\nTYPE: software.Security\n\nDisables a proxy with a security level of 5 or below\nThis action will be detected immediately" },
    ComputerUpgrade { name: "Trace_Tracker", type_: 3, cost: 300, size: 1, data: 1, description: "TRACE TRACKER V1.0\nTYPE: software.Security\n\nInforms you on the progress of any traces on your connection\n(No trace detected, Trace in progress, Trace complete)" },
    ComputerUpgrade { name: "Trace_Tracker", type_: 3, cost: 600, size: 1, data: 2, description: "TRACE TRACKER V2.0\nTYPE: software.Security\n\nInforms you on the progress of any traces on your connection\n(No trace detected, Percentage of trace complete, Trace complete)" },
    ComputerUpgrade { name: "Trace_Tracker", type_: 3, cost: 1400, size: 2, data: 3, description: "TRACE TRACKER V3.0\nTYPE: software.Security\n\nInforms you on the progress of any traces on your connection\n(No trace detected, Time remaining before trace, Trace complete)" },
    ComputerUpgrade { name: "Trace_Tracker", type_: 3, cost: 2500, size: 3, data: 4, description: "TRACE TRACKER V4.0\nTYPE: software.Security\n\nInforms you on the progress of any traces on your connection\n(No trace detected, Time remaining before trace, Trace complete)\nIncluding an estimate of the exact time remaining" },
    ComputerUpgrade { name: "Voice_Analyser", type_: 10, cost: 5000, size: 4, data: 1, description: "VOICE ANALYSER V1.0\nTYPE: software.Other\n\nRecords and analyses voice patterns for reproduction.\nCan be used to simulate voice authorisations." },
];

pub static HARDWARE_UPGRADES: [ComputerUpgrade; 19] = [
    ComputerUpgrade { name: "CPU ( 20 Ghz )", type_: 1, cost: 250, size: 0, data: 20, description: "CPU ( 20 Ghz )\n\nA slow and simple processor designed for use in parallel with many others,in large scale super-computers" },
    ComputerUpgrade { name: "CPU ( 60 Ghz )", type_: 1, cost: 1000, size: 0, data: 60, description: "CPU ( 60 Ghz )\n\nAn entry level processor, fitted as standard to all agent computers" },
    ComputerUpgrade { name: "CPU ( 80 Ghz )", type_: 1, cost: 1300, size: 0, data: 80, description: "CPU ( 80 Ghz )\n\nA low budget, low speed processor" },
    ComputerUpgrade { name: "CPU ( 100 Ghz )", type_: 1, cost: 3000, size: 0, data: 100, description: "CPU ( 100 Ghz )\n\nA reasonably fast processor, and a good upgrade choice for new agents" },
    ComputerUpgrade { name: "CPU ( 120 Ghz )", type_: 1, cost: 5000, size: 0, data: 120, description: "CPU ( 120 Ghz )\n\nA faster processor with onboard MPU instruction handling for faster instruction execution" },
    ComputerUpgrade { name: "CPU ( 150 Ghz )", type_: 1, cost: 8000, size: 0, data: 150, description: "CPU ( 150 Ghz )\n\nA very fast processor, ideal for cpu-intensive activity" },
    ComputerUpgrade { name: "CPU ( Turbo 200 Ghz )", type_: 1, cost: 12000, size: 0, data: 200, description: "CPU ( Turbo 200 Ghz )\n\nA high-speed accelerated processor with unique pre-emptive instruction handling, for use in high-priority critical systems" },
    ComputerUpgrade { name: "Modem ( 1 Gq / s )", type_: 2, cost: 1000, size: 0, data: 1, description: "Modem ( 1 Gq / s )\n\nAn entry level modem with a low transfer rate, fitted as standard in all new Gateway systems" },
    ComputerUpgrade { name: "Modem ( 2 Gq / s )", type_: 2, cost: 2000, size: 0, data: 2, description: "Modem ( 2 Gq / s )\n\nAn low speed modem with a fairly low transfer rate" },
    ComputerUpgrade { name: "Modem ( 4 Gq / s )", type_: 2, cost: 4000, size: 0, data: 4, description: "Modem ( 4 Gq / s )\n\nA reasonable speed modem at a good price" },
    ComputerUpgrade { name: "Modem ( 6 Gq / s )", type_: 2, cost: 6000, size: 0, data: 6, description: "Modem ( 6 Gq / s )\n\nA fast modem, normally sufficient for all but the highest bandwidth requirements" },
    ComputerUpgrade { name: "Modem ( 8 Gq / s )", type_: 2, cost: 8000, size: 0, data: 8, description: "Modem ( 8 Gq / s )\n\nA very fast modem for large files and fast access times" },
    ComputerUpgrade { name: "Modem ( 10 Gq / s )", type_: 2, cost: 10000, size: 0, data: 10, description: "Modem ( 10 Gq / s )\n\nA blindingly fast modem which will take full advantage of the network bandwidth available" },
    ComputerUpgrade { name: "Memory ( 8 Gq )", type_: 4, cost: 3000, size: 0, data: 8, description: "Memory ( 8 Gq )\n\n8 Gigaquads of storage space" },
    ComputerUpgrade { name: "Memory ( 16 Gq )", type_: 4, cost: 5500, size: 0, data: 16, description: "Memory ( 16 Gq )\n\n16 Gigaquads of storage space" },
    ComputerUpgrade { name: "Memory ( 24 Gq )", type_: 4, cost: 8000, size: 0, data: 24, description: "Memory ( 24 Gq )\n\n24 Gigaquads of storage space" },
    ComputerUpgrade { name: "Memory ( 32 Gq )", type_: 4, cost: 11000, size: 0, data: 32, description: "Memory ( 32 Gq )\n\n32 Gigaquads of storage space" },
    ComputerUpgrade { name: "Gateway Self Destruct", type_: 5, cost: 20000, size: 1, data: 0, description: "Gateway Self Destruct\n\nExplosives are attached to your Gateway computer, designed to destroy any evidence.\nThis is a doomsday device that has saved many agents from jail in the past." },
    ComputerUpgrade { name: "Gateway Motion Sensor", type_: 5, cost: 10000, size: 1, data: 0, description: "Gateway Motion Sensor\n\nAn ultra-sonic based motion sensor, which can detect when people are near your gateway. Often used together with a Self Destruct device." },
];

/// C++ `GetSoftwareUpgrade`: look up a software upgrade by exact name.
pub fn get_software_upgrade(name: &str) -> Option<&'static ComputerUpgrade> {
    SOFTWARE_UPGRADES.iter().find(|u| u.name == name)
}

/// C++ `GetHardwareUpgrade`: look up a hardware upgrade by exact name.
pub fn get_hardware_upgrade(name: &str) -> Option<&'static ComputerUpgrade> {
    HARDWARE_UPGRADES.iter().find(|u| u.name == name)
}
