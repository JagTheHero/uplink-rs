//! Port of source/world/generator/missiongenerator.h + source/world/generator/missiongenerator.cpp

use std::cell::RefCell;

use uplink_engine::sound;

use crate::app::globals;
use crate::app::miscutils::lower_case_string;
use crate::app::probability::Probability;
use crate::ctx::Ctx;
use crate::game::data::data::{
    DIFFICULTY_MISSION_VARIANCE, MINDIFFICULTY_MISSION_CHANGEACCOUNT,
    MINDIFFICULTY_MISSION_CHANGEDATA_ACADEMIC, MINDIFFICULTY_MISSION_CHANGEDATA_CRIMINAL,
    MINDIFFICULTY_MISSION_CHANGEDATA_SOCIAL, MINDIFFICULTY_MISSION_DESTROYALLFILES,
    MINDIFFICULTY_MISSION_DESTROYFILE, MINDIFFICULTY_MISSION_FINDDATA_FINANCIAL,
    MINDIFFICULTY_MISSION_FRAMEUSER, MINDIFFICULTY_MISSION_REMOVECOMPUTER,
    MINDIFFICULTY_MISSION_REMOVEUSER, MINDIFFICULTY_MISSION_STEALALLFILES,
    MINDIFFICULTY_MISSION_STEALFILE, MINDIFFICULTY_MISSION_TRACEUSER,
    MINDIFFICULTY_MISSION_TRACEUSER_BANKFRAUD, NEUROMANCERCHANGE, NUM_UPLINKRATINGS,
    PAYMENT_MISSION_CHANGEACCOUNT, PAYMENT_MISSION_CHANGEDATA_ACADEMIC,
    PAYMENT_MISSION_CHANGEDATA_CRIMINAL, PAYMENT_MISSION_CHANGEDATA_SOCIAL,
    PAYMENT_MISSION_DESTROYALLFILES, PAYMENT_MISSION_DESTROYFILE,
    PAYMENT_MISSION_FINDDATA_FINANCIAL, PAYMENT_MISSION_FRAMEUSER, PAYMENT_MISSION_REMOVECOMPUTER,
    PAYMENT_MISSION_REMOVEUSER, PAYMENT_MISSION_STEALALLFILES, PAYMENT_MISSION_STEALFILE,
    PAYMENT_MISSION_TRACEUSER, PAYMENT_MISSION_VARIANCE, PAYMENT_SPECIALMISSION_MOLE,
    PROB_MISSION_CHANGEACCOUNT, PROB_MISSION_CHANGEDATA, PROB_MISSION_DESTROYFILE,
    PROB_MISSION_FINDDATA, PROB_MISSION_FRAMEUSER, PROB_MISSION_REMOVECOMPANY,
    PROB_MISSION_REMOVECOMPUTER, PROB_MISSION_REMOVEUSER, PROB_MISSION_SPECIAL,
    PROB_MISSION_STEALFILE, PROB_MISSION_TRACEUSER,
};
use crate::world::company::companyuplink::CompanyUplink;
use crate::world::company::mission::{
    Mission, MISSIONPAYMENT_AFTERCOMPLETION, MISSIONPAYMENT_HALFATSTART,
};
use crate::world::company::news::{
    NEWS_TYPE_ARREST, NEWS_TYPE_COMPUTERDESTROYED, NEWS_TYPE_COMPUTERSHUTDOWN, NEWS_TYPE_DELETED,
    NEWS_TYPE_HACKED,
};
use crate::world::computer::bankaccount::BankAccount;
use crate::world::computer::computer::{
    Computer, COMPUTER_TYPE_CENTRALMAINFRAME, COMPUTER_TYPE_INTERNALSERVICESMACHINE,
    COMPUTER_TYPE_LAN, COMPUTER_TYPE_PUBLICBANKSERVER,
};
use crate::world::computer::databank::{Data, DATATYPE_DATA, SOFTWARETYPE_NONE};
use crate::world::computer::recordbank::{
    Record, RECORDBANK_NAME, RECORDBANK_PASSWORD, RECORDBANK_SECURITY,
};
use crate::world::date::Date;
use crate::world::generator::consequencegenerator::ConsequenceGenerator;
use crate::world::generator::demoplotgenerator::DemoPlotGenerator;
use crate::world::generator::namegenerator::NameGenerator;
use crate::world::generator::numbergenerator::NumberGenerator;
use crate::world::generator::plotgenerator::{PlotGenerator, SPECIALMISSION_MOLE};
use crate::world::generator::worldgenerator::WorldGenerator;
use crate::world::message::Message;
use crate::world::person::Person;
use crate::world::rating::Rating;
use crate::world::scheduler::notificationevent::{
    NotificationEvent, NOTIFICATIONEVENT_TYPE_AGENTSONLISTDIE,
};
use crate::world::scheduler::uplinkevent::UplinkEvent;
use crate::world::world::World;

// Mission types

pub const MISSION_NONE: i32 = -1;
pub const MISSION_SPECIAL: i32 = 0;

pub const MISSION_STEALFILE: i32 = 1;
pub const MISSION_DESTROYFILE: i32 = 2;
pub const MISSION_FINDDATA: i32 = 3;
pub const MISSION_CHANGEDATA: i32 = 4;
pub const MISSION_FRAMEUSER: i32 = 5;
pub const MISSION_TRACEUSER: i32 = 6;
pub const MISSION_CHANGEACCOUNT: i32 = 7;
pub const MISSION_REMOVECOMPUTER: i32 = 8;
pub const MISSION_REMOVECOMPANY: i32 = 9;
pub const MISSION_REMOVEUSER: i32 = 10;

pub const MISSION_PAYFINE: i32 = 20;

thread_local! {
    /// C++ `static Probability *MissionGenerator::prob_missiontype [NUM_UPLINKRATINGS]`.
    /// PORT: a static member needs a home that is not `static mut`; it is pure constant tables
    /// built by `initialise()` (never saved), so it lives here as a thread-local instead of on `Ctx`.
    /// Index = player's uplink rating (`0..NUM_UPLINKRATINGS`).
    pub static PROB_MISSIONTYPE: RefCell<Vec<Probability>> = const { RefCell::new(Vec::new()) };
}

/// Generates and judges missions.
///
/// All `generate_*` functions create the mission, post it to Uplink's mission board
/// (`CompanyUplink::create_mission` on the "Uplink" company) where the C++ did, and return a
/// CLONE of it for convenience (`None` where the C++ returned NULL). Only `generate_pay_fine`
/// does not post: the caller owns the returned mission (it goes into the person's list).
///
/// Companies, computers and people are passed by key (Company name, Computer name, Person
/// name -- "PLAYER" for the player). `mission` / `message` / `date` parameters are snapshots
/// (`&Mission` etc.): callers clone them out of the world before calling.
pub struct MissionGenerator;

impl MissionGenerator {
    pub fn initialise() {
        let mut probs = Vec::with_capacity(NUM_UPLINKRATINGS as usize);
        for i in 0..NUM_UPLINKRATINGS as usize {
            let mut p = Probability::new();
            p.input_probability(MISSION_SPECIAL, PROB_MISSION_SPECIAL[i]);
            p.input_probability(MISSION_STEALFILE, PROB_MISSION_STEALFILE[i]);
            p.input_probability(MISSION_DESTROYFILE, PROB_MISSION_DESTROYFILE[i]);
            p.input_probability(MISSION_FINDDATA, PROB_MISSION_FINDDATA[i]);
            p.input_probability(MISSION_CHANGEDATA, PROB_MISSION_CHANGEDATA[i]);
            p.input_probability(MISSION_FRAMEUSER, PROB_MISSION_FRAMEUSER[i]);
            p.input_probability(MISSION_TRACEUSER, PROB_MISSION_TRACEUSER[i]);
            p.input_probability(MISSION_CHANGEACCOUNT, PROB_MISSION_CHANGEACCOUNT[i]);
            p.input_probability(MISSION_REMOVECOMPUTER, PROB_MISSION_REMOVECOMPUTER[i]);
            p.input_probability(MISSION_REMOVECOMPANY, PROB_MISSION_REMOVECOMPANY[i]);
            p.input_probability(MISSION_REMOVEUSER, PROB_MISSION_REMOVEUSER[i]);
            debug_assert!(p.validate());
            probs.push(p);
        }
        PROB_MISSIONTYPE.with(|cell| *cell.borrow_mut() = probs);
    }

    pub fn shutdown() {
        PROB_MISSIONTYPE.with(|p| p.borrow_mut().clear());
    }

    /// Generates a random mission (type chosen from the player's rating).
    pub fn generate_mission(g: &mut Ctx) -> Option<Mission> {
        let player_rating = g.player().person.rating.uplinkrating;
        debug_assert!(player_rating >= 0);
        debug_assert!(player_rating < NUM_UPLINKRATINGS);
        let mission_type =
            PROB_MISSIONTYPE.with(|p| p.borrow()[player_rating as usize].get_value());
        Self::generate_mission_of_type(g, mission_type)
    }

    /// `GenerateMission (int type)`: random employer.
    pub fn generate_mission_of_type(g: &mut Ctx, type_: i32) -> Option<Mission> {
        let employer = WorldGenerator::get_random_company(g);
        Self::generate_mission_for(g, type_, &employer)
    }

    /// `GenerateMission (int type, Company *employer)`
    pub fn generate_mission_for(g: &mut Ctx, type_: i32, employer: &str) -> Option<Mission> {
        match type_ {
            MISSION_STEALFILE => Self::generate_steal_file(g, employer),
            MISSION_DESTROYFILE => Self::generate_destroy_file(g, employer),
            MISSION_FINDDATA => Self::generate_find_data(g, employer),
            MISSION_CHANGEDATA => Self::generate_change_data(g, employer),
            MISSION_CHANGEACCOUNT if !globals::DEMOGAME => {
                let source =
                    WorldGenerator::get_random_computer_of_type(g, COMPUTER_TYPE_PUBLICBANKSERVER);
                let target =
                    WorldGenerator::get_random_computer_of_type(g, COMPUTER_TYPE_PUBLICBANKSERVER);
                Self::generate_change_account(g, employer, &source, &target)
            }
            MISSION_FRAMEUSER if !globals::DEMOGAME => Self::generate_frame_user(g, employer),
            MISSION_TRACEUSER if !globals::DEMOGAME => Self::generate_trace_user(g, employer),
            MISSION_REMOVECOMPUTER if !globals::DEMOGAME => {
                let target =
                    WorldGenerator::get_random_computer_of_type(g, COMPUTER_TYPE_CENTRALMAINFRAME);
                Self::generate_remove_computer(g, employer, &target)
            }
            MISSION_REMOVEUSER if !globals::DEMOGAME => Self::generate_remove_user(g, employer),
            _ => None,
        }
    }

    pub fn generate_steal_file(g: &mut Ctx, employer: &str) -> Option<Mission> {
        let mission_type = NumberGenerator::random_number(2) + 1;

        match mission_type {
            1 => {
                let target = WorldGenerator::get_random_low_security_computer(
                    g,
                    COMPUTER_TYPE_INTERNALSERVICESMACHINE,
                );
                if target_info(g, &target).companyname == employer {
                    return None;
                }
                Self::generate_steal_single_file(g, employer, &target)
            }
            2 => {
                let target = WorldGenerator::get_random_computer_of_type(
                    g,
                    COMPUTER_TYPE_INTERNALSERVICESMACHINE
                        | COMPUTER_TYPE_CENTRALMAINFRAME
                        | COMPUTER_TYPE_LAN,
                );
                if target_info(g, &target).companyname == employer {
                    return None;
                }
                Self::generate_steal_all_files(g, employer, &target)
            }
            _ => None,
        }
    }

    pub fn generate_steal_single_file(
        g: &mut Ctx,
        employer: &str,
        target: &str,
    ) -> Option<Mission> {
        let t = target_info(g, target);
        if t.companyname == employer {
            return None;
        }

        let mut difficulty = normal_int(
            MINDIFFICULTY_MISSION_STEALFILE,
            DIFFICULTY_MISSION_VARIANCE as f32,
        );
        if difficulty < MINDIFFICULTY_MISSION_STEALFILE {
            difficulty = MINDIFFICULTY_MISSION_STEALFILE;
        }

        // Set up the basic variables of the mission

        let mut payment = normal_int(
            difficulty * PAYMENT_MISSION_STEALFILE,
            (difficulty * PAYMENT_MISSION_STEALFILE) as f32 * PAYMENT_MISSION_VARIANCE,
        );
        let mut minrating = difficulty;
        let mut acceptrating = minrating + NumberGenerator::random_number(2);
        let datasize = (NumberGenerator::random_normal_number(
            difficulty as f32,
            (difficulty as f64 / 3.0) as f32,
        ) + 1.0) as i32;

        if minrating > 10 {
            minrating = 10;
        }
        if acceptrating > 10 {
            acceptrating = 10;
        }

        payment = (payment / 100) * 100; // Rounds payment to the nearest 100

        // Create the data to be stolen and add it to the target computer's memory

        let datatitle = NameGenerator::generate_data_name(&t.name, DATATYPE_DATA);

        let mut encrypted = if difficulty < 3 {
            0
        } else {
            normal_int(difficulty, (difficulty / 2) as f32)
        };
        if encrypted > 7 {
            encrypted = 7;
        }

        let compressed = 0;

        let mut data = Data::new();
        data.set_title(&datatitle);
        data.set_details(
            DATATYPE_DATA,
            datasize,
            encrypted,
            compressed,
            1.0,
            SOFTWARETYPE_NONE,
        );
        add_data_to_computer(g, target, data);

        // Infer the internal services contact address

        let personname = format!("internal@{}.net", employer);

        // Generate the fields of the mission

        let description = match NumberGenerator::random_number(3) + 1 {
            1 => "Steal important data from a rival company's file server",
            2 => "Access a remote system and copy important data files",
            3 => "Gain access to a rival system and steal research files",
            _ => unreachable!("RandomNumber(3) + 1 is 1..=3"),
        };

        let details = format!(
            "Payment for this job is {} credits.\n\
             You will require {} GigaQuads of storage space.\n\
             This job has been assigned an Uplink difficulty of {}.\n\
             An UplinkRating of {} or above will be sufficient for automatic acceptance.\n\n",
            payment,
            datasize,
            difficulty,
            Rating::get_uplink_rating_string(acceptrating)
        );

        let mut fulldetails = format!("Thankyou for working for {}.\n\n", employer);
        fulldetails.push_str("TARGET COMPUTER DATA :\n");
        fulldetails.push_str(&format!("   LOCATION: {}\n", t.name));
        fulldetails.push_str(&format!("   IP      : {}\n", t.ip));
        fulldetails.push_str(&format!("   FILENAME: {}\n", datatitle));
        fulldetails.push_str(&format!("   FILESIZE: {} GigaQuads\n", datasize));
        fulldetails.push_str("\n");

        if encrypted > 0 && encrypted < 5 {
            fulldetails.push_str("\nThe file will be encrypted.\n");
        } else if encrypted >= 5 {
            fulldetails.push_str("\nThe file will be heavily encrypted.\n");
        }

        if compressed > 0 && compressed < 5 {
            fulldetails.push_str("\nThe file will be compressed.\n");
        } else if compressed >= 5 {
            fulldetails.push_str("\nThe file will be heavily compressed.\n");
        }

        fulldetails.push_str(&format!(
            "\nReturn the file via email to\n{}\n\nEND",
            personname
        ));

        let postdate = random_post_date(g);

        // Insert the mission
        let mut mission = Mission::new();
        mission.set_type(MISSION_STEALFILE);
        mission.set_completion(
            Some(t.ip.as_str()),
            Some(datatitle.as_str()),
            None,
            None,
            None,
        );
        mission.set_employer(employer);
        mission.set_contact(&personname);
        mission.set_payment(payment, max_payment(payment));
        mission.set_difficulty(difficulty);
        mission.set_min_rating(minrating);
        mission.set_accept_rating(acceptrating);
        mission.set_description(description);
        mission.set_details(&details);
        mission.set_full_details(&fulldetails);
        if difficulty > 3 {
            mission.set_why_so_much_money(Some("The files contain sensitive data."));
        }
        set_create_date_unless_running(g, &mut mission, &postdate);
        mission.give_link(&t.ip);

        Some(post_mission(g, mission))
    }

    pub fn generate_steal_all_files(g: &mut Ctx, employer: &str, target: &str) -> Option<Mission> {
        let t = target_info(g, target);

        let mut difficulty = normal_int(
            MINDIFFICULTY_MISSION_STEALALLFILES,
            DIFFICULTY_MISSION_VARIANCE as f32,
        );
        if difficulty < MINDIFFICULTY_MISSION_STEALALLFILES {
            difficulty = MINDIFFICULTY_MISSION_STEALALLFILES;
        }

        if t.type_ == COMPUTER_TYPE_CENTRALMAINFRAME {
            difficulty += 1;
        }
        if t.type_ == COMPUTER_TYPE_LAN {
            difficulty += 2;
        }

        // Set up the basic variables of the mission

        let mut payment = normal_int(
            difficulty * PAYMENT_MISSION_STEALALLFILES,
            (difficulty * PAYMENT_MISSION_STEALALLFILES) as f32 * PAYMENT_MISSION_VARIANCE,
        );

        let mut minrating = difficulty;

        let mut acceptrating = minrating + NumberGenerator::random_number(2);

        if minrating > 10 {
            minrating = 10;
        }
        if acceptrating > 10 {
            acceptrating = 10;
        }

        payment = (payment / 100) * 100; // Rounds payment to the nearest 100

        // Infer the internal services contact address

        let personname = format!("internal@{}.net", employer);

        // Generate a new computer to dump the files to

        let ourcomp = WorldGenerator::generate_empty_file_server(g, employer);
        let ourip = target_info(g, &ourcomp).ip;

        // Add in a new account for the player to use

        let password = NameGenerator::generate_password(g);
        let username = format!(
            "temp{}{}{}{}",
            random_letter(),
            random_letter(),
            random_letter(),
            random_letter()
        );

        let mut record = Record::new();
        record.add_field(RECORDBANK_NAME, &username);
        record.add_field(RECORDBANK_PASSWORD, &password);
        record.add_field(RECORDBANK_SECURITY, "3");
        computer_mut(g, &ourcomp).recordbank.add_record(record);

        let code = Computer::generate_access_code_name(&username, &password);

        let missiontype = NumberGenerator::random_number(4) + 1;

        /*  ===================================================
            type 1		:		Steal all scientific research
            type 2		:		Steal all corporate data
            type 3		:		Steal customer records
            type 4		:		Steal software in development
            ===================================================
        */

        const MISSIONTYPESTRING: [&str; 4] = ["rsrch", "corp", "custmr", "softw"];

        //
        // Generate around 10 files to be stolen,
        // All fairly large
        //

        let numfiles = normal_int(10, 5.0);
        let mut encrypted = normal_int(difficulty, difficulty as f32);
        if encrypted > 7 {
            encrypted = 7;
        }
        let mut totalsize = 0;

        for i in 0..numfiles {
            let datatitle = format!(
                "{}-{} {}.dat",
                company_prefix(&t.companyname),
                MISSIONTYPESTRING[(missiontype - 1) as usize],
                i
            );

            let size = normal_int(10, 5.0);
            totalsize += size;

            let mut file = Data::new();
            file.set_title(&datatitle);
            file.set_details(DATATYPE_DATA, size, encrypted, 0, 1.0, SOFTWARETYPE_NONE);
            put_file(g, target, file);
        }

        //
        // Generate the fields of the mission
        //

        let mut details = format!(
            "Payment for this job is {} credits.\n\
             This job has been assigned an Uplink difficulty of {}.\n\
             An UplinkRating of {} or above will be sufficient for automatic acceptance.\n\
             You will need around {} gigaquads of space.",
            payment,
            difficulty,
            Rating::get_uplink_rating_string(acceptrating),
            10 * (totalsize / 10)
        );

        if t.type_ == COMPUTER_TYPE_LAN {
            details.push_str("\nThe data is stored on a LAN.\n");
        }

        let whoisthetarget = format!("The data is owned by {}", t.companyname);

        let (description, intro, completion_e) = match missiontype {
            1 => (
                "Steal valuable scientific research documents",
                concat!(
                    "Due to a lack of enthusiasm on the part of our recruitment department, ",
                    "one of our market competitors has pulled several months ahead with ",
                    "their research program.  Given our current situation, we have concluded ",
                    "that the best way to close this gap is for us to steal the research data ",
                    "they have gathered.  This will be your mission.\n\n",
                    "Their research files are stored on the system below, and will be ",
                    "heavily protected.  You will need to copy the files, break the security ",
                    "on them in any way you see fit, then copy the files onto our own fileserver.\n\n",
                ),
                "SCIENTIFIC",
            ),
            2 => (
                "Copy large and secure corporate database",
                concat!(
                    "Our establishment is very anxious to find out more about the inner workings ",
                    "of one of our rivals, and we have determined after several weeks of investigation ",
                    "that their primary corporate database is stored at the location shown below.  We ",
                    "want you to enter this system and steal all of the data files.\n\n",
                    "These files will be well protected and encrypted - you will need to break this ",
                    "security, before copying the stolen database to our own fileserver, location below.\n\n",
                ),
                "CORPORATE",
            ),
            3 => (
                "Break into High Security System and steal customer records",
                concat!(
                    "It has come to our attention that one of our competitors has amassed an extensive ",
                    "customer database which would prove to be of great help to our market research team.  ",
                    "We want you to compromise the security of the system below, copy all of the customer data ",
                    "and decrypt it.  Then copy the files onto our own temporary file server.\n\n",
                ),
                "CUSTOMER",
            ),
            4 => (
                "Copy proprietary source code database",
                concat!(
                    "As you may be aware we are currently in direct competition with another rival corporation ",
                    "to develop a software product whcih will no doubt dominate the market once released.  ",
                    "Unfortunately, due to circumstances beyond our control, they have managed to pull several months ",
                    "ahead.  We believe the best course of action is for us to copy their program source code and use ",
                    "it to help our product provide adequate competition.\n",
                    "Break into the computer system below, copy anything that looks like source code and break any ",
                    "security systems in place on the data files.  Then copy those files onto our own file server.\n\n",
                ),
                "SOFTWARE",
            ),
            _ => unreachable!("RandomNumber(4) + 1 is 1..=4"),
        };

        let mut fulldetails = String::from(intro);
        fulldetails.push_str(&format!("Target computer : {}\nIP : {}\n\n", t.name, t.ip));
        fulldetails.push_str(&format!(
            "Once complete, dump the files you have to this computer:\n{}\nIP : {}\n\n",
            ourcomp, ourip
        ));
        fulldetails.push_str(&format!(
            "USERNAME : {}\nPASSWORD : {}\n\nEND",
            username, password
        ));

        let completion_d = format!("{} {}", numfiles, totalsize);

        let postdate = random_post_date(g);

        // Insert the mission
        let mut mission = Mission::new();
        mission.set_type(MISSION_STEALFILE);
        mission.set_completion(
            Some(t.ip.as_str()),
            Some("ALL"),
            Some(ourip.as_str()),
            Some(completion_d.as_str()),
            Some(completion_e),
        );
        mission.set_employer(employer);
        mission.set_contact(&personname);
        mission.set_payment(payment, max_payment(payment));
        mission.set_difficulty(difficulty);
        mission.set_min_rating(minrating);
        mission.set_accept_rating(acceptrating);
        mission.set_description(description);
        mission.set_details(&details);
        mission.set_full_details(&fulldetails);
        mission.set_why_so_much_money(Some("There will be a lot of data to copy"));
        set_create_date_unless_running(g, &mut mission, &postdate);
        mission.set_who_is_the_target(Some(whoisthetarget.as_str()));
        mission.give_link(&t.ip);
        mission.give_link(&ourip);
        mission.give_code(&ourip, &code);

        Some(post_mission(g, mission))
    }

    pub fn generate_destroy_file(g: &mut Ctx, employer: &str) -> Option<Mission> {
        let mission_type = NumberGenerator::random_number(2) + 1;

        match mission_type {
            1 => {
                let target = WorldGenerator::get_random_low_security_computer(
                    g,
                    COMPUTER_TYPE_INTERNALSERVICESMACHINE,
                );
                if target_info(g, &target).companyname == employer {
                    return None;
                }
                Self::generate_destroy_single_file(g, employer, &target)
            }
            2 => {
                let target = WorldGenerator::get_random_computer_of_type(
                    g,
                    COMPUTER_TYPE_INTERNALSERVICESMACHINE
                        | COMPUTER_TYPE_CENTRALMAINFRAME
                        | COMPUTER_TYPE_LAN,
                );
                if target_info(g, &target).companyname == employer {
                    return None;
                }
                Self::generate_destroy_all_files(g, employer, &target)
            }
            _ => None,
        }
    }

    pub fn generate_destroy_single_file(
        g: &mut Ctx,
        employer: &str,
        target: &str,
    ) -> Option<Mission> {
        let t = target_info(g, target);

        let mut difficulty = normal_int(
            MINDIFFICULTY_MISSION_DESTROYFILE,
            DIFFICULTY_MISSION_VARIANCE as f32,
        );
        if difficulty < MINDIFFICULTY_MISSION_DESTROYFILE {
            difficulty = MINDIFFICULTY_MISSION_DESTROYFILE;
        }

        if t.type_ == COMPUTER_TYPE_CENTRALMAINFRAME {
            difficulty += 1;
        }

        // Set up the basic variables of the mission

        let mut payment = normal_int(
            difficulty * PAYMENT_MISSION_DESTROYFILE,
            (difficulty * PAYMENT_MISSION_DESTROYFILE) as f32 * PAYMENT_MISSION_VARIANCE,
        );

        let mut minrating = difficulty;
        let mut acceptrating = minrating + NumberGenerator::random_number(2);
        let datasize = (NumberGenerator::random_normal_number(
            difficulty as f32,
            (difficulty as f64 / 3.0) as f32,
        ) + 1.0) as i32;

        if minrating > 10 {
            minrating = 10;
        }
        if acceptrating > 10 {
            acceptrating = 10;
        }

        payment = (payment / 100) * 100; // Rounds payment to the nearest 100

        // Create the data to be stolen and add it to the target computer's memory

        let datatitle = NameGenerator::generate_data_name(&t.name, DATATYPE_DATA);

        let mut data = Data::new();
        data.set_title(&datatitle);
        data.set_details(DATATYPE_DATA, datasize, 0, 0, 1.0, SOFTWARETYPE_NONE);
        add_data_to_computer(g, target, data);

        // Infer the internal services contact address

        let personname = format!("internal@{}.net", employer);

        // Generate the fields of the mission

        let description = match NumberGenerator::random_number(3) + 1 {
            1 => "Find and destroy crucial data on a mainframe",
            2 => "Break into a rival computer system and sabotage files",
            3 => "Hack into a computer and delete key files",
            _ => unreachable!("RandomNumber(3) + 1 is 1..=3"),
        };

        let details = format!(
            "Payment for this job is {} credits.\n\
             This job has been assigned an Uplink difficulty of {}.\n\
             An UplinkRating of {} or above will be sufficient for automatic acceptance.\n\n",
            payment,
            difficulty,
            Rating::get_uplink_rating_string(acceptrating)
        );

        let mut fulldetails = format!("Thankyou for working for {}.\n", employer);
        fulldetails.push_str("\n");
        fulldetails.push_str("TARGET COMPUTER DATA :\n");
        fulldetails.push_str(&format!("   LOCATION: {}\n", t.name));
        fulldetails.push_str(&format!("   IP      : {}\n", t.ip));
        fulldetails.push_str(&format!("   FILENAME: {}\n", datatitle));
        fulldetails.push_str(&format!("   FILESIZE: {} GigaQuads\n", datasize));
        fulldetails.push_str("\n");
        fulldetails.push_str("Destroy this data and all backups.\n");
        fulldetails.push_str("Send a notice of completion to\n");
        fulldetails.push_str(&format!("{}\n", personname));
        fulldetails.push_str("\n");
        fulldetails.push_str("END");

        let postdate = random_post_date(g);

        // Insert the mission
        let mut mission = Mission::new();
        mission.set_type(MISSION_DESTROYFILE);
        mission.set_completion(
            Some(t.ip.as_str()),
            Some(datatitle.as_str()),
            None,
            None,
            None,
        );
        mission.set_employer(employer);
        mission.set_contact(&personname);
        mission.set_payment(payment, max_payment(payment));
        mission.set_difficulty(difficulty);
        mission.set_min_rating(minrating);
        mission.set_accept_rating(acceptrating);
        mission.set_description(description);
        mission.set_details(&details);
        mission.set_full_details(&fulldetails);
        if difficulty > 4 {
            mission.set_why_so_much_money(Some("The data will be well protected."));
        }
        set_create_date_unless_running(g, &mut mission, &postdate);
        mission.give_link(&t.ip);

        Some(post_mission(g, mission))
    }

    pub fn generate_destroy_all_files(
        g: &mut Ctx,
        employer: &str,
        target: &str,
    ) -> Option<Mission> {
        let t = target_info(g, target);

        let mut difficulty = normal_int(
            MINDIFFICULTY_MISSION_DESTROYALLFILES,
            DIFFICULTY_MISSION_VARIANCE as f32,
        );
        if difficulty < MINDIFFICULTY_MISSION_DESTROYALLFILES {
            difficulty = MINDIFFICULTY_MISSION_DESTROYALLFILES;
        }

        if t.type_ == COMPUTER_TYPE_CENTRALMAINFRAME {
            difficulty += 1;
        }
        if t.type_ == COMPUTER_TYPE_LAN {
            difficulty += 2;
        }

        // Set up the basic variables of the mission

        let mut payment = normal_int(
            difficulty * PAYMENT_MISSION_DESTROYALLFILES,
            (difficulty * PAYMENT_MISSION_DESTROYALLFILES) as f32 * PAYMENT_MISSION_VARIANCE,
        );

        let mut minrating = difficulty;

        let mut acceptrating = minrating + NumberGenerator::random_number(2);

        if minrating > 10 {
            minrating = 10;
        }
        if acceptrating > 10 {
            acceptrating = 10;
        }

        payment = (payment / 100) * 100; // Rounds payment to the nearest 100

        // Infer the internal services contact address

        let personname = format!("internal@{}.net", employer);

        // Generate the fields of the mission

        let mut details = format!(
            "Payment for this job is {} credits.\n\
             This job has been assigned an Uplink difficulty of {}.\n\
             An UplinkRating of {} or above will be sufficient for automatic acceptance.\n\n",
            payment,
            difficulty,
            Rating::get_uplink_rating_string(acceptrating)
        );

        if t.type_ == COMPUTER_TYPE_LAN {
            details.push_str("\nThe data is stored on a LAN.\n");
        }

        let whoisthetarget = format!("The data is owned by {}", t.companyname);

        let missiontype = NumberGenerator::random_number(4) + 1;

        //
        // Generate around 10 files to be destroyed,
        // All fairly large
        //

        const MISSIONTYPESTRING: [&str; 4] = ["rsrch", "corp", "custmr", "softw"];
        let numfiles = normal_int(10, 5.0);
        let encrypted = normal_int(difficulty, difficulty as f32);

        for i in 0..numfiles {
            let datatitle = format!(
                "{}-{} {}.dat",
                company_prefix(&t.companyname),
                MISSIONTYPESTRING[(missiontype - 1) as usize],
                i
            );

            let size = normal_int(10, 5.0); // C++ also summed this into an unused totalsize

            let mut file = Data::new();
            file.set_title(&datatitle);
            file.set_details(DATATYPE_DATA, size, encrypted, 0, 1.0, SOFTWARETYPE_NONE);
            put_file(g, target, file);
        }

        /*
            type 1		:		Destroy all scientific research
            type 2		:		Destroy all corporate data
            type 3		:		Destroy customer records
            type 4		:		Destroy software in development
        */

        let (description, intro, completion_c) = match missiontype {
            1 => (
                "Destroy scientific research currently in progress",
                concat!(
                    "One of our competitors has unfortunately gained a lead ",
                    "and has nearly completed some scientific research which will ",
                    "be damaging to our corporate agenda.\n",
                    "Break into the computer system below and destroy ALL files ",
                    "you find on their file server.  You must also ensure all backups ",
                    "are totally wiped out.\n",
                ),
                "SCIENTIFIC",
            ),
            2 => (
                "Destroy corporate data of a rival company",
                concat!(
                    "We are becoming tired of the competition provided by a ",
                    "rival firm and we have decided to put an end to their activities.\n",
                    "Break into their main computer system (ip below) and destroy ALL files ",
                    "on their system.\n",
                ),
                "CORPORATE",
            ),
            3 => (
                "Destroy customer records on a sales system",
                concat!(
                    "We are in direct competition with another rival sales system and ",
                    "wish to bring our conflict to a quick end.  Break into their primary ",
                    "sales system shown below and delete all files on their computer.\n",
                ),
                "CUSTOMER",
            ),
            4 => (
                "Destroy rival software in development",
                concat!(
                    "As you may be aware we are trying to promote our new software system ",
                    "but another corporation has developed a rival tool which we feel may ",
                    "harm our sales.  Gain access to their main development server (below) ",
                    "and destroy ALL files you find.  You must also ensure all backups are ",
                    "totally destroyed.\n",
                ),
                "SOFTWARE",
            ),
            _ => unreachable!("RandomNumber(4) + 1 is 1..=4"),
        };

        let mut fulldetails = String::from(intro);
        fulldetails.push_str(concat!(
            "It would be in your interests to remove any traces of your actions, ",
            "as the company will not be pleased when they discover what you have done.",
            "\n\n",
            "TARGET COMPUTER : \n",
        ));
        fulldetails.push_str(&format!("LOCATION: {}\n", t.name));
        fulldetails.push_str(&format!("IP      : {}\n", t.ip));
        fulldetails.push_str("\n");
        fulldetails.push_str("Send a notice of completion to\n");
        fulldetails.push_str(&format!("{}\n", personname));
        fulldetails.push_str("\n");
        fulldetails.push_str("END");

        let postdate = random_post_date(g);

        // Insert the mission
        let mut mission = Mission::new();
        mission.set_type(MISSION_DESTROYFILE);
        mission.set_completion(
            Some(t.ip.as_str()),
            Some("ALL"),
            Some(completion_c),
            None,
            None,
        );
        mission.set_employer(employer);
        mission.set_contact(&personname);
        mission.set_payment(payment, max_payment(payment));
        mission.set_difficulty(difficulty);
        mission.set_min_rating(minrating);
        mission.set_accept_rating(acceptrating);
        mission.set_description(description);
        mission.set_details(&details);
        mission.set_full_details(&fulldetails);
        mission.set_why_so_much_money(Some("The data will be well protected."));
        set_create_date_unless_running(g, &mut mission, &postdate);
        mission.set_who_is_the_target(Some(whoisthetarget.as_str()));
        mission.give_link(&t.ip);

        Some(post_mission(g, mission))
    }

    pub fn generate_find_data(g: &mut Ctx, employer: &str) -> Option<Mission> {
        let missiontype = 0;

        match missiontype {
            0 => {
                let target =
                    WorldGenerator::get_random_computer_of_type(g, COMPUTER_TYPE_PUBLICBANKSERVER);
                Self::generate_find_data_financial_record(g, employer, &target)
            }
            _ => None,
        }
    }

    pub fn generate_change_data(g: &mut Ctx, employer: &str) -> Option<Mission> {
        // Decide which type of changedata mission to make this

        let missiontype = NumberGenerator::random_number(3);

        match missiontype {
            0 => Self::generate_change_data_academic_record(g, employer),
            1 => Self::generate_change_data_social_security(g, employer),
            2 => Self::generate_change_data_criminal_record(g, employer),
            _ => None,
        }
    }

    /// `Generate_FrameUser ( Company *employer )`: picks a random victim.
    pub fn generate_frame_user(g: &mut Ctx, employer: &str) -> Option<Mission> {
        //
        // Choose a random person to frame
        //

        let poorsod = WorldGenerator::get_random_person(g);

        Self::generate_frame_user_person(g, employer, &poorsod)
    }

    pub fn generate_trace_user(_g: &mut Ctx, _employer: &str) -> Option<Mission> {
        //
        // This mission is never randomly generated.
        // It always occurs as a consequence of somebody doing something like
        // hacking a bank etc.
        //
        None
    }

    /// `source` / `target` = bank Computer names.
    pub fn generate_change_account(
        g: &mut Ctx,
        employer: &str,
        source: &str,
        target: &str,
    ) -> Option<Mission> {
        let difficulty = roll_difficulty(MINDIFFICULTY_MISSION_CHANGEACCOUNT);

        //
        // The employer wants you to transfer money between the two bank computers;
        // source and target.  Both account numbers are random, but obviously can't be the same.
        //

        //
        // Set up the basic variables of the mission
        //

        let payment = roll_payment(difficulty, PAYMENT_MISSION_CHANGEACCOUNT);
        let (minrating, acceptrating) = ratings(difficulty, 3);
        let payment = (payment / 100) * 100; // Rounds payment to the nearest 100

        let contact = internal_contact(employer); // Person to send completion email to

        //
        // Choose random bank accounts on the
        // source and target machines
        //

        let (source_name, source_ip) = computer_name_ip(g, source);
        let source_account = bank_random_account(g, source);
        let target_account = bank_random_account(g, target);

        //
        // Make sure they aren't player accounts
        //

        let handle = g.player().agent().handle.clone();
        if source_account.name == handle {
            return None;
        }
        if target_account.name == handle {
            return None;
        }

        let mut amount_to_transfer = random_normal_int(
            (source_account.balance / 2) as f32,
            (source_account.balance as f64 * 0.4) as f32,
        );
        amount_to_transfer = (amount_to_transfer / 500) * 500;

        if amount_to_transfer <= 0 {
            return None;
        }
        if amount_to_transfer > source_account.balance {
            amount_to_transfer = (source_account.balance as f64 * 0.9) as i32;
        }

        let (target_name, target_ip) = computer_name_ip(g, target);

        //
        // Fill in the fields of the mission
        //

        let completion_a = format!("{} {}", source_ip, source_account.accountnumber);
        let completion_b = format!("{} {}", target_ip, target_account.accountnumber);
        let completion_c = amount_to_transfer.to_string();

        let description = "Our esteemed colleague wishes to make a donation.";
        let details = standard_details(payment, difficulty, acceptrating);

        let mut fulldetails = String::new();
        fulldetails.push_str(&format!(
            "Break into the following well known financial institute:\nTARGET COMPUTER DATA :\n   LOCATION: {}\n   IP      : {}\n\n\n",
            source_name, source_ip
        ));
        fulldetails.push_str(&format!(
            "Gain access to account number {},\nwhich is in the name of {}.\nTransfer {} credits to the following account:\n",
            source_account.accountnumber, source_account.name, amount_to_transfer
        ));
        fulldetails.push_str(&format!(
            "   LOCATION: {}\n   IP      : {}\n   ACCNO   : {}\n\n\n",
            target_name, target_ip, target_account.accountnumber
        ));
        fulldetails.push_str(&format!(
            "Remove any and all traces of this transfer.\nSend a notice of completion to\n{}\n\nEND",
            contact
        ));

        let postdate = random_post_date(g);

        //
        // Insert the mission
        //

        let mut mission = Mission::new();
        mission.set_type(MISSION_CHANGEACCOUNT);
        mission.set_completion(
            Some(completion_a.as_str()),
            Some(completion_b.as_str()),
            Some(completion_c.as_str()),
            None,
            None,
        );
        mission.set_employer(employer);
        mission.set_contact(&contact);
        mission.set_payment(payment, max_payment(payment));
        mission.set_difficulty(difficulty);
        mission.set_min_rating(minrating);
        mission.set_accept_rating(acceptrating);
        mission.set_description(description);
        mission.set_details(&details);
        mission.set_full_details(&fulldetails);
        mission.set_why_so_much_money(Some("Financial systems are always well guarded."));
        mission.set_how_secure(Some("The bank will be using Proxys and Monitors."));
        if !g.game.is_running() {
            mission.set_create_date(&postdate);
        }
        mission.give_link(&source_ip);
        mission.set_who_is_the_target(Some(source_name.as_str()));

        post_mission_p2(g, mission)
    }

    pub fn generate_remove_computer(g: &mut Ctx, employer: &str, target: &str) -> Option<Mission> {
        let difficulty = roll_difficulty(MINDIFFICULTY_MISSION_REMOVECOMPUTER);

        //
        // Set up the basic variables of the mission
        //

        let payment = roll_payment(difficulty, PAYMENT_MISSION_REMOVECOMPUTER);
        let (minrating, acceptrating) = ratings(difficulty, 3);
        let payment = (payment / 100) * 100; // Rounds payment to the nearest 100

        let contact = internal_contact(employer); // Person to send completion email to

        //
        // Fill in the fields
        //

        let description = "Elite agent required for destruction of computer system";
        let details = standard_details(payment, difficulty, acceptrating);

        let (target_name, target_ip) = computer_name_ip(g, target);
        let target_company = g
            .world()
            .get_computer(target)
            .expect("UplinkAssert(target)")
            .companyname
            .clone();

        let fulldetails = format!(
            "One of our competitors has made the critical mistake of becoming over dependent \
on one of their primary computer systems, and we intend to take advantage of this oversight.\n\
We want you to put the computer system below out of action, permenantly.\n\
Use any means necessary, including virus attacks or system resets, but all data must be \
destroyed and the system itself must be shut down.\n\n\
Target : {}\nIP : {}\n\
Once you have completed the task, send a mail to {} and we will determine if you have \
been successful.\nEND",
            target_name, target_ip, contact
        );

        let postdate = random_post_date(g);

        let whoisthetarget = format!("The system is owned by {}", target_company);

        //
        // Insert the mission
        //

        let mut mission = Mission::new();
        mission.set_type(MISSION_REMOVECOMPUTER);
        mission.set_completion(Some(target_ip.as_str()), None, None, None, None);
        mission.set_employer(employer);
        mission.set_contact(&contact);
        mission.set_payment(payment, max_payment(payment));
        mission.set_difficulty(difficulty);
        mission.set_min_rating(minrating);
        mission.set_accept_rating(acceptrating);
        mission.set_description(description);
        mission.set_details(&details);
        mission.set_full_details(&fulldetails);
        mission.set_why_so_much_money(Some("The computer system will be heavily defended."));
        mission.give_link(&target_ip);
        mission.set_who_is_the_target(Some(whoisthetarget.as_str()));
        if !g.game.is_running() {
            mission.set_create_date(&postdate);
        }

        post_mission_p2(g, mission)
    }

    pub fn generate_remove_company(_g: &mut Ctx, _employer: &str) -> Option<Mission> {
        None
    }

    pub fn generate_remove_user(g: &mut Ctx, employer: &str) -> Option<Mission> {
        let mut difficulty = NumberGenerator::random_normal_number(
            MINDIFFICULTY_MISSION_REMOVEUSER as f32,
            DIFFICULTY_MISSION_VARIANCE as f32,
        ) as i32;
        if difficulty < MINDIFFICULTY_MISSION_REMOVEUSER {
            difficulty = MINDIFFICULTY_MISSION_REMOVEUSER;
        }

        // Set up the basic variables of the mission
        let payment_base = (difficulty * PAYMENT_MISSION_REMOVEUSER) as f32;
        let mut payment = NumberGenerator::random_normal_number(
            payment_base,
            payment_base * PAYMENT_MISSION_VARIANCE,
        ) as i32;

        let mut minrating = difficulty;
        let mut acceptrating = minrating + NumberGenerator::random_number(3);

        if minrating > 10 {
            minrating = 10;
        }
        if acceptrating > 10 {
            acceptrating = 10;
        }

        payment = (payment / 100) * 100; // Rounds payment to the nearest 100

        let personname = format!("internal@{}.net", employer); // Person to send completion email to

        // Find the target person (never the player)
        let target = WorldGenerator::get_random_person(g);
        if target == "PLAYER" {
            return None;
        }

        // Fill in the fields
        let description = "Highly skilled Agent required for removal job";

        let details = format!(
            "Payment for this job is {} credits.\n\
             This job has been assigned an Uplink difficulty of {}.\n\
             An UplinkRating of {} or above will be sufficient for automatic acceptance.\n\n",
            payment,
            difficulty,
            Rating::get_uplink_rating_string(acceptrating)
        );

        let fulldetails = format!(
            "Recently this man has become quite an annoyance and \
             we would like him removed.  We do not care how you do it, \
             as long as his life is destroyed by the time you have \
             finished.  \n\n\
             TARGET : {}\n\n\
             Once you have completed the task, send a mail to \
             {} and we will determine if you have \
             been successful.\n\
             END",
            target, personname
        );

        let mut postdate = Date::default();
        postdate.set_date_from(&g.world().date);
        postdate.advance_hour(NumberGenerator::random_number(96) * -1);
        postdate.advance_minute(NumberGenerator::random_number(60) * -1);

        // Insert the mission
        let mut mission = Mission::new();
        mission.set_type(MISSION_REMOVEUSER);
        mission.set_completion(Some(target.as_str()), None, None, None, None);
        mission.set_employer(employer);
        mission.set_contact(&personname);
        mission.set_payment(payment, (payment as f64 * 1.1) as i32);
        mission.set_difficulty(difficulty);
        mission.set_min_rating(minrating);
        mission.set_accept_rating(acceptrating);
        mission.set_description(description);
        mission.set_details(&details);
        mission.set_full_details(&fulldetails);
        mission.set_why_so_much_money(Some("A great deal of skill will be required."));
        mission.set_how_secure(Some("That depends on the method you use."));
        if !g.game.is_running() {
            mission.set_create_date(&postdate);
        }

        CompanyUplink::create_mission(g, mission.clone());
        Some(mission)
    }

    pub fn generate_change_data_academic_record(g: &mut Ctx, employer: &str) -> Option<Mission> {
        let difficulty = roll_difficulty(MINDIFFICULTY_MISSION_CHANGEDATA_ACADEMIC);

        let target_key = "International Academic Database";
        let (target_name, target_ip) = computer_name_ip(g, target_key);

        //
        // Set up the basic variables of the mission
        //

        let payment = roll_payment(difficulty, PAYMENT_MISSION_CHANGEDATA_ACADEMIC);
        let (minrating, acceptrating) = ratings(difficulty, 2);
        let payment = (payment / 100) * 100; // Rounds payment to the nearest 100

        let contact = internal_contact(employer); // Person to send completion email to
        let details = standard_details(payment, difficulty, acceptrating);

        let mut fulldetails = format!(
            "Gain access to the International Academic Database:\nTARGET COMPUTER DATA :\n   LOCATION: {}\n   IP      : {}\n\n\n",
            target_name, target_ip
        );

        /* =========================================================================
         type 1		:		Add a degree
         type 2		:		Improve a degree rating
         type 3		:		Decrease a degree rating
         type 4		:		Add a random qualification
        ========================================================================= */

        let missiontype = NumberGenerator::random_number(4) + 1;

        let (description, person_name, completion_c, completion_d, completion_e, whysomuchmoney) =
            match missiontype {
                1 => {
                    // Add a degree =====================================================

                    // Find someone without a degree
                    let Some(person_name) = pick_target_person(g, target_key, "University = None")
                    else {
                        return None;
                    };

                    // Generate a University degree that this person wants
                    let degreelevel = 1;
                    let degreesubject = match NumberGenerator::random_number(10) {
                        0 => "Computing",
                        1 => "Physics",
                        2 => "Electrical Engineering",
                        3 => "Mechanical Engineering",
                        4 => "Software Engineering",
                        5 => "Systems Engineering",
                        6 => "Management",
                        7 => "Artificial Intelligence",
                        8 => "Philosophy",
                        9 => "Media studies",
                        _ => panic!(
                            "MissionGenerator::Generate_ChangeData_EmploymentRecord, error in Degree Type"
                        ),
                    };

                    fulldetails.push_str(&format!(
                        "Provide this person with the following University Degree:\nUNIVERSITY DEGREE :\n   SUBJECT : {}\n   CLASS   : {}\n",
                        degreesubject, degreelevel
                    ));

                    (
                        "Generate a University Degree for a friend".to_string(),
                        person_name,
                        "University".to_string(),
                        degreesubject.to_string(),
                        degreelevel.to_string(),
                        "We really need this qualification".to_string(),
                    )
                }
                2 => {
                    // Improve a degree rating ==========================================

                    // Find someone with a degree that is not a 1st
                    let Some(person_name) =
                        pick_target_person(g, target_key, "University ! None ; University - 1")
                    else {
                        return None;
                    };

                    fulldetails.push_str("This person requires a first class degree.\n");

                    (
                        "Improve our associates' academic standing".to_string(),
                        person_name,
                        "University".to_string(),
                        "1".to_string(),
                        "1".to_string(), // Always needs a 1st
                        "This qualification will be very useful to us".to_string(),
                    )
                }
                3 => {
                    // Decrease a degree rating ==========================================

                    // Find someone with a 1st
                    let Some(person_name) =
                        pick_target_person(g, target_key, "University ! None ; University + 1")
                    else {
                        return None;
                    };

                    fulldetails.push_str(
                        "We feel this person scored slightly too highly at his degree.Give him a 3rd class replacement.\n",
                    );

                    (
                        "Help us prove a fellow employee is over-rated".to_string(),
                        person_name,
                        "University".to_string(),
                        "3".to_string(), // Always needs a 3rd
                        "3".to_string(),
                        "We really dont like the target".to_string(),
                    )
                }
                4 => {
                    // Add a random qualification =======================================

                    let qualification = match NumberGenerator::random_number(5) {
                        0 => "Diploma in Computing",
                        1 => "IEEE Accreditation",
                        2 => "Certified systems engineer",
                        3 => "Btec in IT",
                        4 => "Registered Uplink Agent",
                        _ => panic!(
                            "MissionGenerator::Generate_ChangeData_EmploymentRecord, error in Qualification Type"
                        ),
                    };

                    // Find someone who doesn't have this qualification
                    let query = format!("Other - {}", qualification);
                    let Some(person_name) = pick_target_person(g, target_key, &query) else {
                        return None;
                    };

                    fulldetails.push_str(&format!(
                        "This individual urgently requires a \n'{}' Qualification adding to his record.",
                        qualification
                    ));

                    (
                        "Qualifications required for wealthy professional".to_string(),
                        person_name,
                        "Other".to_string(),
                        qualification.to_string(),
                        qualification.to_string(),
                        "This qualification is needed to gain entry".to_string(),
                    )
                }
                _ => panic!("Unrecognised mission type"),
            };

        //
        // Generate the remaining fields of the mission
        //

        let target_person = person_name_of(g, &person_name);

        fulldetails.push_str(&format!(
            "\nTARGET INDIVIDUAL :\n   NAME    : {}\n\n\nSend a notice of completion to\n{}\n\nEND",
            target_person, contact
        ));

        let whoisthetarget = "He's just a friend of ours.";

        let postdate = random_post_date(g);

        //
        // Insert the mission
        //

        let mut mission = Mission::new();
        mission.set_type(MISSION_CHANGEDATA);
        mission.set_completion(
            Some(target_ip.as_str()),
            Some(target_person.as_str()),
            Some(completion_c.as_str()),
            Some(completion_d.as_str()),
            Some(completion_e.as_str()),
        );
        mission.set_employer(employer);
        mission.set_contact(&contact);
        mission.set_payment(payment, max_payment(payment));
        mission.set_difficulty(difficulty);
        mission.set_min_rating(minrating);
        mission.set_accept_rating(acceptrating);
        mission.set_description(&description);
        mission.set_details(&details);
        mission.set_full_details(&fulldetails);
        mission.set_why_so_much_money(Some(whysomuchmoney.as_str()));
        if !g.game.is_running() {
            mission.set_create_date(&postdate);
        }
        mission.set_who_is_the_target(Some(whoisthetarget));
        mission.give_link(&target_ip);

        post_mission_p2(g, mission)
    }

    pub fn generate_change_data_social_security(g: &mut Ctx, employer: &str) -> Option<Mission> {
        let difficulty = roll_difficulty(MINDIFFICULTY_MISSION_CHANGEDATA_SOCIAL);

        let target_key = "International Social Security Database";
        let (target_name, target_ip) = computer_name_ip(g, target_key);

        //
        // Set up the basic variables of the mission
        //

        let payment = roll_payment(difficulty, PAYMENT_MISSION_CHANGEDATA_SOCIAL);
        let (minrating, acceptrating) = ratings(difficulty, 2);
        let payment = (payment / 100) * 100; // Rounds payment to the nearest 100

        let contact = internal_contact(employer); // Person to send completion email to
        let details = standard_details(payment, difficulty, acceptrating);

        let mut fulldetails = format!(
            "Break into the International Social Security Database:\nTARGET COMPUTER DATA :\n   LOCATION: {}\n   IP      : {}\n\n\n",
            target_name, target_ip
        );

        /* =========================================================================
         type 1		:		Make someone deceased
         type 2		:		Resurrect a dead person
        ========================================================================= */

        let missiontype = NumberGenerator::random_number(2) + 1;

        let (description, person_name, completion_c, completion_d, completion_e) = match missiontype
        {
            1 => {
                // Find someone who isn't dead
                let Some(person_name) =
                    pick_target_person(g, target_key, "Personal Status ! Deceased")
                else {
                    return None;
                };

                fulldetails.push_str(
                        "The man below is living too easily.  Change his Personal Status to read 'Deceased'.\n",
                    );

                (
                    "Falsify a Social Security document".to_string(),
                    person_name,
                    "Personal Status".to_string(),
                    "Deceased".to_string(),
                    "Deceased".to_string(),
                )
            }
            _ => {
                // Find someone who is dead (missiontype == 2; RandomNumber(2) + 1 is only 1 or 2)
                let Some(person_name) =
                    pick_target_person(g, target_key, "Personal Status = Deceased")
                else {
                    return None;
                };

                fulldetails.push_str(
                        "The man below recently passed away and we wish to use his identity ourselves. Change his Personal Status to 'Employed'.\n",
                    );

                (
                    "Create part of a new identity".to_string(),
                    person_name,
                    "Personal Status".to_string(),
                    "Employed".to_string(),
                    "Employed".to_string(),
                )
            }
        };

        //
        // Generate the remaining fields of the mission
        //

        let target_person = person_name_of(g, &person_name);

        fulldetails.push_str(&format!(
            "\nTARGET INDIVIDUAL :\n   NAME    : {}\n\n\nSend a notice of completion to\n{}\n\nEND",
            target_person, contact
        ));

        let whoisthetarget = "You wouldn't recognise the name.";

        let postdate = random_post_date(g);

        //
        // Insert the mission
        //

        let mut mission = Mission::new();
        mission.set_type(MISSION_CHANGEDATA);
        mission.set_completion(
            Some(target_ip.as_str()),
            Some(target_person.as_str()),
            Some(completion_c.as_str()),
            Some(completion_d.as_str()),
            Some(completion_e.as_str()),
        );
        mission.set_employer(employer);
        mission.set_contact(&contact);
        mission.set_payment(payment, max_payment(payment));
        mission.set_difficulty(difficulty);
        mission.set_min_rating(minrating);
        mission.set_accept_rating(acceptrating);
        mission.set_description(&description);
        mission.set_details(&details);
        mission.set_full_details(&fulldetails);
        mission.set_why_so_much_money(Some("We need to use the Social Security record very soon"));
        mission.set_how_secure(Some("You will need to bypass the Proxy server."));
        if !g.game.is_running() {
            mission.set_create_date(&postdate);
        }
        mission.set_who_is_the_target(Some(whoisthetarget));
        mission.give_link(&target_ip);

        post_mission_p2(g, mission)
    }

    pub fn generate_change_data_criminal_record(g: &mut Ctx, employer: &str) -> Option<Mission> {
        let difficulty = roll_difficulty(MINDIFFICULTY_MISSION_CHANGEDATA_CRIMINAL);

        let target_key = "Global Criminal Database";
        let (target_name, target_ip) = computer_name_ip(g, target_key);

        //
        // Set up the basic variables of the mission
        //

        let payment = roll_payment(difficulty, PAYMENT_MISSION_CHANGEDATA_CRIMINAL);
        let (minrating, acceptrating) = ratings(difficulty, 2);
        let payment = (payment / 100) * 100; // Rounds payment to the nearest 100

        let contact = internal_contact(employer); // Person to send completion email to
        let details = standard_details(payment, difficulty, acceptrating);

        let mut fulldetails = format!(
            "Break into the Global Criminal Database:\nTARGET COMPUTER DATA :\n   LOCATION: {}\n   IP      : {}\n\n\n",
            target_name, target_ip
        );

        let missiontype = NumberGenerator::random_number(3) + 1;

        /* =========================================================================
         type 1		:		Give standard conviction
         type 2		:		Give computer-related conviction
         type 3		:		Remove a computer-related conviction
        ========================================================================= */

        let (description, person_name, completion_c, completion_d, completion_e, whoisthetarget) =
            match missiontype {
                1 => {
                    let Some(person_name) = pick_target_person(g, target_key, "Convictions = None")
                    else {
                        return None;
                    };

                    fulldetails.push_str(&format!(
                        "We need {} discrediting so that any future checks on his background will come up negative.\nOpen his Criminal Record and give him a conviction for Robbery.",
                        person_name
                    ));

                    (
                        "Help to discredit one of our rivals".to_string(),
                        person_name,
                        "Convictions".to_string(),
                        "Robbery".to_string(),
                        "Robbery".to_string(),
                        "We wouldn't want to give his name out just yet.",
                    )
                }
                2 => {
                    let Some(person_name) = pick_target_person(g, target_key, "Convictions = None")
                    else {
                        return None;
                    };

                    fulldetails.push_str(&format!(
                        "We have suspected that {} was guilty of hacking one of our systems for some time now, but have been unable to aquire sufficient proof.\nWe wish to discredit him by giving him a computer related criminal record, preventing him from ever working in the industry again.\nOpen his Criminal Record and give him a new conviction for High tech fraud.",
                        person_name
                    ));

                    (
                        "Help to stop a hacker from ever working again".to_string(),
                        person_name,
                        "Convictions".to_string(),
                        "High tech fraud".to_string(),
                        "High tech fraud".to_string(),
                        "A hacker who has attacked us before.",
                    )
                }
                _ => {
                    let Some(person_name) = pick_target_person(g, target_key, "Convictions ! None")
                    else {
                        return None;
                    };

                    fulldetails.push_str(&format!(
                        "One of our friends in the industry is having some trouble finding employment, due to his criminal record.  We would be grateful if you could remove all record of his criminal past \n\nTarget individual : {}",
                        person_name
                    ));

                    (
                        "Clear a criminal record".to_string(),
                        person_name,
                        "Convictions".to_string(),
                        "None".to_string(),
                        "None".to_string(),
                        "Just a friend of ours who needs help.",
                    )
                }
            };

        //
        // Generate the remaining fields of the mission
        //

        let target_person = person_name_of(g, &person_name);

        fulldetails.push_str(&format!(
            "\nSend a notice of completion to\n{}\n\nEND",
            contact
        ));

        let postdate = random_post_date(g);

        let whysomuchmoney = if missiontype == 3 {
            "The police may come after you."
        } else {
            "The target may have friends."
        };

        //
        // Insert the mission
        //

        let mut mission = Mission::new();
        mission.set_type(MISSION_CHANGEDATA);
        mission.set_completion(
            Some(target_ip.as_str()),
            Some(target_person.as_str()),
            Some(completion_c.as_str()),
            Some(completion_d.as_str()),
            Some(completion_e.as_str()),
        );
        mission.set_employer(employer);
        mission.set_contact(&contact);
        mission.set_payment(payment, max_payment(payment));
        mission.set_difficulty(difficulty);
        mission.set_min_rating(minrating);
        mission.set_accept_rating(acceptrating);
        mission.set_description(&description);
        mission.set_details(&details);
        mission.set_full_details(&fulldetails);
        mission.set_why_so_much_money(Some(whysomuchmoney));
        mission.set_how_secure(Some("The GCD uses a Proxy security system."));
        if !g.game.is_running() {
            mission.set_create_date(&postdate);
        }
        mission.set_who_is_the_target(Some(whoisthetarget));
        mission.give_link(&target_ip);

        post_mission_p2(g, mission)
    }

    pub fn generate_find_data_financial_record(
        g: &mut Ctx,
        employer: &str,
        target: &str,
    ) -> Option<Mission> {
        let t = target_info(g, target);
        debug_assert!(g
            .world()
            .get_computer(target)
            .and_then(|c| c.bank())
            .is_some());

        let mut difficulty = normal_int(
            MINDIFFICULTY_MISSION_FINDDATA_FINANCIAL,
            DIFFICULTY_MISSION_VARIANCE as f32,
        );
        if difficulty < MINDIFFICULTY_MISSION_FINDDATA_FINANCIAL {
            difficulty = MINDIFFICULTY_MISSION_FINDDATA_FINANCIAL;
        }

        //
        // Set up the basic variables of the mission
        //

        let mut payment = normal_int(
            difficulty * PAYMENT_MISSION_FINDDATA_FINANCIAL,
            (difficulty * PAYMENT_MISSION_FINDDATA_FINANCIAL) as f32 * PAYMENT_MISSION_VARIANCE,
        );

        let mut minrating = difficulty;
        let mut acceptrating = minrating + NumberGenerator::random_number(2);

        if minrating > 10 {
            minrating = 10;
        }
        if acceptrating > 10 {
            acceptrating = 10;
        }

        payment = (payment / 100) * 100; // Rounds payment to the nearest 100

        let personname = format!("internal@{}.net", employer); // Person to send completion email to

        //
        // Get the account of a person at this bank computer
        //

        let (taccount_name, taccount_no) = random_account_of(g, target);

        // Make sure we don't target the player's Uplink bank account

        if taccount_name == player_handle(g) {
            return None;
        }

        //
        // Fill in the fields of the mission
        //

        let completion_a = format!("{} {}", t.ip, taccount_no); // bankIP accountNum
        let mut completion_c = String::from("Unused"); // Name of person responsible (only type3)

        let details = format!(
            "Payment for this job is {} credits.\n\
             This job has been assigned an Uplink difficulty of {}.\n\
             An UplinkRating of {} or above will be sufficient for automatic acceptance.\n\n",
            payment,
            difficulty,
            Rating::get_uplink_rating_string(acceptrating)
        );

        let mut fulldetails = format!(
            "Break into the following well known financial institute:\n\
             TARGET COMPUTER DATA :\n\
             \x20  LOCATION: {}\n\
             \x20  IP      : {}\n\
             \n\n",
            t.name, t.ip
        );

        let missiontype = NumberGenerator::random_number(3) + 1;

        /* =========================================================================
             type 1		:		Find bank balance
             type 2		:		Find size of loan
             type 3		:		Find receiver of recent payment
        ========================================================================= */

        let (description, completion_b) = match missiontype {
            1 => {
                fulldetails.push_str(&format!(
                    "We are interested in this man's bank balance.\n\
                     Open his account and send his balance to us.\n\
                     NAME : {}\n\
                     ACCNO: {}\n",
                    taccount_name, taccount_no
                ));
                ("Find financial details of one of our associates", "Balance")
            }
            2 => {
                fulldetails.push_str(&format!(
                    "We are interested in this man's financial situation.\n\
                     Open his account and determine the size of his loan. Send this information to us.\n\
                     NAME : {}\n\
                     ACCNO: {}\n",
                    taccount_name, taccount_no
                ));
                ("Client interested in financial data on an enemy", "Loan")
            }
            3 => {
                // Perform the transfer
                let receiver =
                    WorldGenerator::get_random_computer_of_type(g, COMPUTER_TYPE_PUBLICBANKSERVER);
                let (raccount_name, raccount_no) = random_account_of(g, &receiver);

                if receiver == target && raccount_no == taccount_no {
                    return None; // That would be silly ;)
                }

                let receiver_ip = target_info(g, &receiver).ip;
                let taccountno = taccount_no.to_string();
                let raccountno = raccount_no.to_string();

                if g.player().is_player_account(&receiver_ip, &raccountno) {
                    return None;
                }
                if g.player().is_player_account(&t.ip, &taccountno) {
                    return None;
                }

                let mut amount = normal_int(1000000, 700000.0);
                amount = 100000 * (amount / 100000);
                BankAccount::get_account_mut(g.world_mut(), &t.ip, &taccountno)
                    .expect("taccount exists")
                    .balance += amount;

                let transferer = taccount_name.clone();
                debug_assert!(g.world().get_person(&transferer).is_some());

                let _success = BankAccount::transfer_money(
                    g,
                    &t.ip,
                    &taccountno,
                    &receiver_ip,
                    &raccountno,
                    amount,
                    &transferer,
                );

                fulldetails.push_str(&format!(
                    "This individual is the subject of an internal investigation into corruption:\n\
                     NAME : {}\n\
                     ACCNO: {}\n\
                     He recently transfered a sizable amount of money to another account.\
                     Trace this account and find the name of the receiver of this money.",
                    taccount_name, taccount_no
                ));

                completion_c = raccount_name;
                ("Trace a recent balance transfer", "TraceTransfer")
            }
            _ => panic!("Unrecognised mission type"),
        };

        //
        // Generate the remaining fields of the mission
        //

        fulldetails.push_str(&format!(
            "\nSend a notice of completion to\n{}\n\nEND",
            personname
        ));

        let whoisthetarget = "Nobody you would know.";

        let postdate = random_post_date(g);

        //
        // Insert the mission
        //

        let mut mission = Mission::new();
        mission.set_type(MISSION_FINDDATA);
        mission.set_completion(
            Some(completion_a.as_str()),
            Some(completion_b),
            Some(completion_c.as_str()),
            None,
            None,
        );
        mission.set_employer(employer);
        mission.set_contact(&personname);
        mission.set_payment(payment, max_payment(payment));
        mission.set_difficulty(difficulty);
        mission.set_min_rating(minrating);
        mission.set_accept_rating(acceptrating);
        mission.set_description(description);
        mission.set_details(&details);
        mission.set_full_details(&fulldetails);
        mission.set_why_so_much_money(Some("The data is highly personal, and well protected"));
        set_create_date_unless_running(g, &mut mission, &postdate);
        mission.set_who_is_the_target(Some(whoisthetarget));
        mission.give_link(&t.ip);

        Some(post_mission(g, mission))
    }

    // Special case missions that can't be randomly generated -- they must be specifically requested

    /// Not posted to the BBS; the caller gives it to the person.
    pub fn generate_pay_fine(
        g: &mut Ctx,
        person: &str,
        company: &str,
        amount: i32,
        duedate: &Date,
        reason: &str,
    ) -> Mission {
        //
        // Locate a target bank account
        //

        let bank_key =
            WorldGenerator::get_random_computer_of_type(g, COMPUTER_TYPE_PUBLICBANKSERVER);
        let (bank_name, bank_ip) = computer_name_ip(g, &bank_key);
        let accno = bank_random_account(g, &bank_key).accountnumber;

        //
        // Look up a contact at the company
        //

        let personname = internal_contact(company); // Person to send completion email to

        //
        // Fill in the fields of the mission
        //

        let person_obj_name = person_name_of(g, person);
        let completion_a = if person_obj_name == "PLAYER" {
            g.player().agent().handle.clone()
        } else {
            person_obj_name
        };
        let completion_b = format!("{} {}", bank_ip, accno);
        let completion_c = amount.to_string();

        //
        // Create the descriptive text
        //

        let description = format!("Pay a {}c fine", amount);

        let before = duedate.get_long_string();
        let fulldetails = format!(
            "{}\n\nAs a result of this action you are legally required to pay a fine to the following account. Failure to pay this fine before the due date could result in imprisonment.\n\nBANK : {}\nIP : {}\nACCNO : {}\nFINE : {} credits\nBEFORE : {}\n\n\nTo pay this fine, access your financial screen and click on the account you wish to transfer from.\nThis will connect you to that account.\nThen select 'transfer' from the menu and fill in the details as above.\n\nOnce you have paid this fine, send a mail to {} to confirm receipt.\nEND.",
            reason, bank_name, bank_ip, accno, amount, before, personname
        );

        let postdate = random_post_date(g);

        //
        // Build the mission
        //

        let mut mission = Mission::new();
        mission.set_type(MISSION_PAYFINE);
        mission.set_completion(
            Some(completion_a.as_str()),
            Some(completion_b.as_str()),
            Some(completion_c.as_str()),
            None,
            None,
        );
        mission.set_due_date(Some(duedate.clone()));
        mission.set_employer(company);
        mission.set_contact(&personname);
        mission.set_payment(0, -1);
        mission.set_difficulty(0);
        mission.set_min_rating(0);
        mission.set_accept_rating(0);
        mission.set_description(&description);
        mission.set_details(reason);
        mission.set_full_details(&fulldetails);
        if !g.game.is_running() {
            mission.set_create_date(&postdate);
        }

        mission
    }

    /// `Generate_FrameUser ( Company *employer, Person *person )`
    pub fn generate_frame_user_person(
        g: &mut Ctx,
        employer: &str,
        person: &str,
    ) -> Option<Mission> {
        let difficulty = roll_difficulty(MINDIFFICULTY_MISSION_FRAMEUSER);

        //
        // Set up the basic variables of the mission
        //

        let mut payment = roll_payment(difficulty, PAYMENT_MISSION_FRAMEUSER);
        let (minrating, acceptrating) = ratings(difficulty, 3);
        payment = (payment / 100) * 100; // Rounds payment to the nearest 100

        let contact = internal_contact(employer); // Person to send completion email to

        let (target_person, localhost) = {
            let p = g.world().get_person(person).expect("UplinkAssert(person)");
            (p.name.clone(), p.localhost.clone())
        };

        let details = standard_details(payment, difficulty, acceptrating);

        let mut fulldetails = String::new();

        /* =================================================================
           type 1	:		Frame a user for hacking
           type 2	:		Frame a user for financial fraud
           type 3	:		Frame a user for destruction of data
        ================================================================= */

        let missiontype = NumberGenerator::random_number(3) + 1;

        let description: &str;
        let completion_b: &str;
        let mut completion_c: Option<String> = None;
        let howsecure: Option<&str>;

        if missiontype == 1 {
            // Frame a user for hacking
            description = "Frame a user for computer crimes";

            fulldetails.push_str(
                "A member of the public has been interfering with our plans recently and we believe his arrest and conviction for computer crimes will put his operation at an end.\nPerform a break in on a high-priority computer system and ensure that this man is charged with the crime.  He must be jailed for several years.\n",
            );

            completion_b = "HACKING";
            howsecure = Some("That depends on what you frame him for.");
        } else if missiontype == 2 {
            // Frame a user for bank fraud
            description = "Frame a man for bank fraud";

            let bank_key =
                WorldGenerator::get_random_computer_of_type(g, COMPUTER_TYPE_PUBLICBANKSERVER);
            let (bank_name, bank_ip) = computer_name_ip(g, &bank_key);

            fulldetails.push_str(&format!(
                "We need this man to be arrested and charged for breaking into a bank and attempting some kind of financial fraud.\nWe would like you to use the bank server specified below.\n\nPerform some form of illegal operation (such as disabling some of their security systems and making sure they notice you) then modify the records so that he is incriminated.\n\n\nTARGET BANK:\n{}\nIP: {}\n",
                bank_name, bank_ip
            ));

            completion_b = "FINANCIAL";
            completion_c = Some(bank_ip);

            payment = (payment as f64 * 1.2) as i32;
            howsecure = Some("Banks always have very high security.");
        } else {
            // Frame a user for destruction of data (missiontype == 3)
            description = "Frame a user for destruction of data.";

            let target_key = WorldGenerator::get_random_computer_of_type(
                g,
                COMPUTER_TYPE_INTERNALSERVICESMACHINE,
            );
            let (target_name, target_ip) = computer_name_ip(g, &target_key);

            fulldetails.push_str(&format!(
                "Our corporation has recently found a way to kill two birds with one proverbial stone.  We wish to destroy some valuable research data, whilst at the same time incriminating one of our long standing enemies for the crime.\n\nBreak into the computer below and destroy all data files.  Then modify the logs so that this man is charged with the crime and sent to jail.\n\n\nTARGET COMPUTER:\n{}\nIP: {}\n",
                target_name, target_ip
            ));

            completion_b = "DESTROYALL";
            completion_c = Some(target_ip);

            payment = (payment as f64 * 1.2) as i32;
            howsecure = None;
        }

        //
        // Fill in the rest of the mission
        //

        fulldetails.push_str(&format!(
            "\nTarget individual :\n{}\nHis local system : \n{}\n\nOnce you have completed your mission and his arrest is in the news, send a mail to {} and we will transfer the payment.\nYou will only receive payment if he is charged for the crime we have outlined.\nGood luck.\n",
            target_person, localhost, contact
        ));

        let postdate = random_post_date(g);

        //
        // Build the mission
        //

        let mut m = Mission::new();
        m.set_type(MISSION_FRAMEUSER);
        m.set_completion(
            Some(target_person.as_str()),
            Some(completion_b),
            completion_c.as_deref(),
            None,
            None,
        );
        m.set_employer(employer);
        m.set_contact(&contact);
        m.set_payment(payment, max_payment(payment));
        m.set_difficulty(difficulty);
        m.set_min_rating(minrating);
        m.set_accept_rating(acceptrating);
        m.set_description(description);
        m.set_details(&details);
        m.set_full_details(&fulldetails);
        m.set_why_so_much_money(Some("His employer and friends may come after you."));
        m.set_who_is_the_target(Some("Someone we don't like very much"));
        m.set_how_secure(howsecure);

        m.give_link(&localhost);
        if let Some(ip) = completion_c.as_deref() {
            m.give_link(ip);
        }

        // PORT: the C++ builds `postdate` (consuming the random numbers) but never applies it to
        // this mission (no SetCreateDate), so CreateMission stamps the world date instead.
        let _ = postdate;

        post_mission_p2(g, m)
    }

    /// `Generate_TraceHacker ( Mission *completedmission, Person *hacker )`
    pub fn generate_trace_hacker_mission(
        g: &mut Ctx,
        completedmission: &Mission,
        hacker: &str,
    ) -> Option<Mission> {
        //
        // Look up the computer that was hacked
        //

        let hackedip = completedmission
            .links
            .first()
            .expect("UplinkAssert(hackedip)")
            .clone();
        let comp_name = {
            let vl = g
                .world()
                .get_vlocation(&hackedip)
                .expect("UplinkAssert(vl)");
            g.world()
                .get_computer(&vl.computer)
                .expect("UplinkAssert(comp)")
                .name
                .clone()
        };

        //
        // Generate a follow up mission to trace the bastard that did the hack
        //

        Self::generate_trace_hacker_computer(g, &comp_name, hacker)
    }

    /// `Generate_TraceHacker ( Computer *hacked, Person *hacker )`; `hacked` = Computer name (may be a LAN computer).
    pub fn generate_trace_hacker_computer(
        g: &mut Ctx,
        hacked: &str,
        hacker: &str,
    ) -> Option<Mission> {
        let difficulty = roll_difficulty(MINDIFFICULTY_MISSION_TRACEUSER);

        let mut payment = roll_payment(difficulty, PAYMENT_MISSION_TRACEUSER);

        // PORT: unlike the other generators, this one does not clamp minrating/acceptrating to 10
        let minrating = difficulty;
        let acceptrating = minrating + NumberGenerator::random_number(3);

        payment = (payment / 100) * 100; // Rounds payment to the nearest 100

        let (hacked_name, hacked_ip, hacked_type, companyname) = {
            let c = g
                .world()
                .get_computer(hacked)
                .expect("UplinkAssert(hacked)");
            (c.name.clone(), c.ip.clone(), c.type_, c.companyname.clone())
        };
        let hacker_name = person_name_of(g, hacker);

        let provideaccount = hacked_type == COMPUTER_TYPE_INTERNALSERVICESMACHINE;

        let mut password = String::new();
        let mut username = String::new();

        if provideaccount {
            //
            // Add in a new account for the player to use
            //

            password = NameGenerator::generate_password(g);

            username.push_str("temp");
            for _ in 0..4 {
                let letter = (b'a' + NumberGenerator::random_number(26) as u8) as char;
                username.push(letter);
            }

            let mut record = Record::new();
            record.add_field(RECORDBANK_NAME, &username);
            record.add_field(RECORDBANK_PASSWORD, &password);
            record.add_field(RECORDBANK_SECURITY, "3");
            g.world_mut()
                .get_computer_mut(&hacked_name)
                .expect("UplinkAssert(hacked)")
                .recordbank
                .add_record(record);
        }

        //
        // Look up the owner
        //

        let contact = internal_contact(&companyname);

        //
        // Fill in the text fields
        //

        let description = "Trace a hacker who recently broke into our systems";
        let details = standard_details(payment, difficulty, acceptrating);

        let date_short = g.world().date.get_short_string();
        let mut fulldetails = format!(
            "Our computer systems were recently compromised by a computer hacker.\nFind this hacker and send his name to us.  We will deal with him in our own way.\n\nThe compromised system is :\n{}\nIP : {}\n\nThe hack occured around {}.\n",
            hacked_name, hacked_ip, date_short
        );

        if provideaccount {
            fulldetails.push_str(&format!(
                "We have provided you with level 3 clearence to our system.\nYour username is {}\nAnd your password is {}\n\n",
                username, password
            ));
        }

        fulldetails.push_str(&format!(
            "Once completed, send the name of the hacker to {} and payment will be delivered.\nEND",
            contact
        ));

        let mut m = Mission::new();
        m.set_type(MISSION_TRACEUSER);
        m.set_completion(
            Some(hacker_name.as_str()),
            Some(hacked_ip.as_str()),
            None,
            None,
            None,
        );
        m.set_employer(&companyname);
        m.set_contact(&contact);
        m.set_payment(payment, max_payment(payment));
        m.set_difficulty(difficulty);
        m.set_min_rating(minrating);
        m.set_accept_rating(acceptrating);
        m.set_description(description);
        m.set_details(&details);
        m.set_full_details(&fulldetails);
        m.give_link(&hacked_ip);

        m.set_why_so_much_money(Some("The hacker will certainly have covered his tracks"));
        m.set_how_secure(Some("You may have to hack several targets."));
        m.set_who_is_the_target(Some("That is for you to determine."));

        if provideaccount {
            let code = Computer::generate_access_code_name(&username, &password);
            m.give_code(&hacked_ip, &code);
        }

        if is_player(g, hacker) {
            m.set_npc_priority(true);
        }

        post_mission_p2(g, m)
    }

    /// `source` / `target` = bank Computer names; `sacc` / `tacc` = snapshots of the accounts.
    #[allow(clippy::too_many_arguments)]
    pub fn generate_trace_user_money_transfer(
        g: &mut Ctx,
        employer: &str,
        source: &str,
        _target: &str,
        sacc: &BankAccount,
        tacc: &BankAccount,
        amount: i32,
        hacker: &str,
    ) -> Option<Mission> {
        //
        // Set up the basic variables of the mission
        //

        let difficulty = roll_difficulty(MINDIFFICULTY_MISSION_TRACEUSER_BANKFRAUD);
        let payment = roll_payment(difficulty, PAYMENT_MISSION_TRACEUSER);
        let (minrating, acceptrating) = ratings(difficulty, 3);
        let payment = (payment / 100) * 100; // Rounds payment to the nearest 100

        let contact = internal_contact(employer); // Person to send completion email to

        let description = "Trace an unauthorised money transfer";
        let details = standard_details(payment, difficulty, acceptrating);

        let (source_name, source_ip) = computer_name_ip(g, source);
        let hacker_name = person_name_of(g, hacker);
        let date_short = g.world().date.get_short_string();

        let fulldetails = format!(
            "One of our bank accounts was recently hacked, and funds were transfered to an unknown account.Use any methods necessary to trace this hacker.\nOur compromised account is at:\n{}\n IP    : {}\n ACCNO : {}\nThe access code to the hacked account is {}\n(The username is the account number)\nThe amount transfered was {} credits.\nThis occured around {}\n\nYou are authorised to access the compromised account.\n\nOnce you have traced the hacker, send his name to us at\n{} and we will take matters further if necessary.\n\nEND",
            source_name,
            source_ip,
            sacc.accountnumber,
            sacc.password,
            amount,
            date_short,
            contact
        );

        let accesscode = Computer::generate_access_code_accno(sacc.accountnumber, &sacc.password);

        let postdate = random_post_date(g);

        let whoisthetarget = "That's for you to find out.";

        //
        // Insert the mission
        //

        let mut mission = Mission::new();
        mission.set_type(MISSION_TRACEUSER);
        mission.set_completion(
            Some(hacker_name.as_str()),
            Some(source_ip.as_str()),
            Some(tacc.name.as_str()),
            None,
            None,
        );
        mission.set_employer(employer);
        mission.set_contact(&contact);
        mission.set_payment(payment, max_payment(payment));
        mission.set_difficulty(difficulty);
        mission.set_min_rating(minrating);
        mission.set_accept_rating(acceptrating);
        mission.set_description(description);
        mission.set_details(&details);
        mission.set_full_details(&fulldetails);
        if !g.game.is_running() {
            mission.set_create_date(&postdate);
        }
        mission.give_link(&source_ip);
        mission.give_code(&source_ip, &accesscode);
        mission.set_why_so_much_money(Some("Bank records are notoriously hard to crack"));
        mission.set_how_secure(Some("Banks are always well defended."));
        mission.set_who_is_the_target(Some(whoisthetarget));

        if is_player(g, hacker) {
            mission.set_npc_priority(true);
        }

        post_mission_p2(g, mission)
    }

    // Tests of whether the player has completed a mission he was given. They also act upon
    // his success or failure. `person` = Person name, `message` = the mail that claims completion.

    pub fn is_mission_complete(
        g: &mut Ctx,
        mission: &mut Mission,
        person: &str,
        message: &Message,
    ) -> bool {
        // PORT: `mission` is &mut because the completion functions reduce mission->payment
        // (StealAllFiles, RemoveComputer) and that reduced payment is paid out in MissionCompleted.
        match mission.type_ {
            MISSION_STEALFILE => Self::is_mission_complete_steal_file(g, mission, person, message),
            MISSION_DESTROYFILE => {
                Self::is_mission_complete_destroy_file(g, mission, person, message)
            }
            MISSION_FINDDATA => {
                Self::is_mission_complete_find_financial(g, mission, person, message)
            }
            MISSION_CHANGEDATA => {
                Self::is_mission_complete_change_data(g, mission, person, message)
            }
            MISSION_FRAMEUSER => Self::is_mission_complete_frame_user(g, mission, person, message),
            MISSION_TRACEUSER => Self::is_mission_complete_trace_user(g, mission, person, message),
            MISSION_CHANGEACCOUNT => {
                Self::is_mission_complete_change_account(g, mission, person, message)
            }
            MISSION_REMOVECOMPUTER => {
                Self::is_mission_complete_remove_computer(g, mission, person, message)
            }
            MISSION_REMOVEUSER => {
                Self::is_mission_complete_remove_user(g, mission, person, message)
            }
            MISSION_PAYFINE => Self::is_mission_complete_pay_fine(g, mission, person, message),
            MISSION_SPECIAL => Self::is_mission_complete_special(g, mission, person, message),
            _ => panic!("Unknown Mission Type"),
        }
    }

    pub fn is_mission_complete_steal_file(
        g: &mut Ctx,
        mission: &mut Mission,
        person: &str,
        message: &Message,
    ) -> bool {
        let datatitle = mission.completionb.clone(); // Required data title

        if datatitle == "ALL" {
            return Self::is_mission_complete_steal_all_files(g, mission, person, message);
        }

        // Data must be uncompressed and not encrypted
        // (C++ has a commented-out "compressed" branch here; it is disabled in the source.)
        match message.get_data() {
            Some(msgdata) if msgdata.title == datatitle && msgdata.encrypted == 0 => {
                Self::mission_completed(g, mission, person, message);
                true
            }
            Some(msgdata) if msgdata.title == datatitle && msgdata.encrypted > 0 => {
                // Message is encrypted
                let reason = format!(
                    "Your mission is not yet complete.\n\
                     The data you sent us is encrypted with level {} encryption, which we are unable to break.\n\
                     \nPlease break the encryption on the file and send it too us again.\n\
                     You will not be paid until this has been accomplished.",
                    msgdata.encrypted
                );
                Self::mission_not_completed(g, mission, person, message, &reason);
                false
            }
            _ => {
                Self::mission_not_completed(
                    g,
                    mission,
                    person,
                    message,
                    "You don't appear to have attached the file.",
                );
                false
            }
        }
    }

    pub fn is_mission_complete_steal_all_files(
        g: &mut Ctx,
        mission: &mut Mission,
        person: &str,
        message: &Message,
    ) -> bool {
        let ip = mission.completiona.clone();
        let ourcompip = mission.completionc.clone();
        let (numfiles, datasize) = scan_two_ints(&mission.completiond);
        let datatype = mission.completione.clone();

        // Look up the target computer
        let target_companyname = computer_at(g.world(), &ip).companyname.clone();

        // Work out the filenames that should be here
        let missiontypestring = match datatype.as_str() {
            "SCIENTIFIC" => "rsrch",
            "RESEARCH" => "rsrch", // Not needed anymore, but older user files may have this
            "CORPORATE" => "corp",
            "CUSTOMER" => "custmr",
            "SOFTWARE" => "softw",
            _ => panic!("Unrecognised data type"),
        };
        // C++ sprintf: first four characters of the company name, then "-<type>"
        let prefix: String = target_companyname.chars().take(4).collect();
        let stolendatatitle = format!("{}-{}", prefix, missiontypestring);

        // Lookup the dump computer, then work out how much of the data has been successfully
        // stolen. Ignore duplicate files or anything that doesn't have the correct filename.
        let (foundsize, foundencrypted) = {
            let ourcomp = computer_at(g.world(), &ourcompip);
            let mut foundsize = 0;
            let mut foundencrypted = false;
            let mut filefound = vec![false; numfiles.max(0) as usize];

            for i in 0..ourcomp.databank.get_data_size() {
                if let Some(thisfile) = ourcomp.databank.get_data_file(i) {
                    if thisfile.encrypted != 0 {
                        foundencrypted = true;
                    }

                    if thisfile.title.contains(stolendatatitle.as_str()) && thisfile.encrypted == 0
                    {
                        let thisfileindex = data_file_index(&thisfile.title);

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
            (foundsize, foundencrypted)
        };

        // If they've got less than half, they've failed
        // Otherwise, base their remaining payment on the
        // fraction that they successfully downloaded.
        if foundsize == 0 {
            if foundencrypted {
                Self::mission_not_completed(
                    g,
                    mission,
                    person,
                    message,
                    "We can see some files on the fileserver but they appear to be encrypted.  You need to decrypt them before we make payment.",
                );
            } else {
                Self::mission_not_completed(
                    g,
                    mission,
                    person,
                    message,
                    "We can't find any of the files on the file server we specified.",
                );
            }
            false
        } else if foundsize < datasize / 2 {
            if foundencrypted {
                Self::mission_not_completed(
                    g,
                    mission,
                    person,
                    message,
                    "You haven't recovered very much data and a lot of it is encrypted - please copy more and decrypt all of it.",
                );
            } else {
                Self::mission_not_completed(
                    g,
                    mission,
                    person,
                    message,
                    "You haven't copied very much of the data - we'll need more before we pay you.",
                );
            }
            false
        } else if foundsize == datasize {
            Self::mission_completed(g, mission, person, message);

            // Shut down the file server
            // Not done in the C++ either: the file server is left running.

            true
        } else {
            let fractionfound = foundsize as f32 / datasize as f32;
            mission.payment = (fractionfound * mission.payment as f32) as i32;
            mission.payment = 100 * (mission.payment / 100);
            if mission.payment < 100 {
                mission.payment = 100;
            }

            let newmessage = format!(
                "We've reviewed your recent mission to copy data files.\n\
                 It looks like you got most of the files but some are missing.\n\
                 We've decided to reduce your remaining payment to {}c.",
                mission.payment
            );

            send_mail(g, "PLAYER", person, "Your recent mission", &newmessage);

            Self::mission_completed(g, mission, person, message);

            // Shut down the file server
            // Not done in the C++ either: the file server is left running.

            true
        }
    }

    pub fn is_mission_complete_destroy_file(
        g: &mut Ctx,
        mission: &Mission,
        person: &str,
        message: &Message,
    ) -> bool {
        let ip = mission.completiona.clone();
        let filename = mission.completionb.clone();

        debug_assert!(g.world().get_vlocation(&ip).is_some(), "UplinkAssert(vl)");

        if filename == "ALL" {
            let uplink = uplink_company(g.world());

            // method 1 - Delete all files
            // method 2 - Destroy the entire computer
            let damaged = uplink.news.iter().any(|news| {
                (news.newstype == NEWS_TYPE_DELETED || news.newstype == NEWS_TYPE_COMPUTERDESTROYED)
                    && news.data1 == ip
            });

            if damaged {
                Self::mission_completed(g, mission, person, message);
                return true;
            }

            Self::mission_not_completed(
                g,
                mission,
                person,
                message,
                "We have not heard word yet that the damage has been done.",
            );
            false
        } else {
            let file_exists = computer_at(g.world(), &ip)
                .databank
                .get_data_by_title(&filename)
                .is_some();

            if !file_exists {
                Self::mission_completed(g, mission, person, message);
                true
            } else {
                Self::mission_not_completed(
                    g,
                    mission,
                    person,
                    message,
                    "According to our sources, the file has not yet been deleted.",
                );
                false
            }
        }
    }

    pub fn is_mission_complete_find_data(
        g: &mut Ctx,
        mission: &Mission,
        person: &str,
        message: &Message,
    ) -> bool {
        let value1 = lower_case_string(&mission.completiona);
        let value2 = lower_case_string(&mission.completionb);
        let value3 = lower_case_string(&mission.completionc);
        let value4 = lower_case_string(&mission.completiond);
        let value5 = lower_case_string(&mission.completione);

        // All 5 values need to be in the message
        // (Unless they are NULL -- an empty completion string matches anything, as in strstr.)
        let msgbody = lower_case_string(message.get_body());
        let mut success = true;

        for value in [&value1, &value2, &value3, &value4, &value5] {
            if !value.is_empty() && !msgbody.contains(value.as_str()) {
                success = false;
            }
        }

        if success {
            Self::mission_completed(g, mission, person, message);
            true
        } else {
            Self::mission_not_completed(
                g,
                mission,
                person,
                message,
                "This data appears to be invalid.  Please verify it.",
            );
            false
        }
    }

    pub fn is_mission_complete_find_financial(
        g: &mut Ctx,
        mission: &Mission,
        person: &str,
        message: &Message,
    ) -> bool {
        // CompletionA = sourceIP accountNum
        // CompletionB = Field (balance, loan etc)
        // CompletionC = name of person responsible (if type == find source of transfer)
        let (source_ip, source_acc) = scan_word_int(&mission.completiona);
        let source_acc_s = source_acc.to_string();
        let field = mission.completionb.as_str();

        let (balance, loan) = {
            let source_account = BankAccount::get_account(g.world(), &source_ip, &source_acc_s)
                .expect("UplinkAssert(source_account)");
            (source_account.balance, source_account.loan)
        };

        match field {
            "Balance" => {
                let sbalance = balance.to_string();
                if message.get_body().contains(sbalance.as_str()) {
                    Self::mission_completed(g, mission, person, message);
                    true
                } else {
                    Self::mission_not_completed(
                        g,
                        mission,
                        person,
                        message,
                        "Could you please verify your results.  We have reason to believe they are not accurate.",
                    );
                    false
                }
            }
            "Loan" => {
                let sloan = loan.to_string();
                if message.get_body().contains(sloan.as_str()) {
                    Self::mission_completed(g, mission, person, message);
                    true
                } else {
                    Self::mission_not_completed(
                        g,
                        mission,
                        person,
                        message,
                        "Could you please verify your results.  We have reason to believe they are not accurate.",
                    );
                    false
                }
            }
            "TraceTransfer" => {
                let personresponsible = lower_case_string(&mission.completionc);
                if lower_case_string(message.get_body()).contains(personresponsible.as_str()) {
                    Self::mission_completed(g, mission, person, message);
                    true
                } else {
                    Self::mission_not_completed(
                        g,
                        mission,
                        person,
                        message,
                        "Could you please verify your results.  We have reason to believe that person is not responsible.",
                    );
                    false
                }
            }
            _ => panic!("Error in IsMissionComplete_FindFinancial"),
        }
    }

    pub fn is_mission_complete_change_data(
        g: &mut Ctx,
        mission: &Mission,
        person: &str,
        message: &Message,
    ) -> bool {
        let ip = mission.completiona.clone();
        let personname = mission.completionb.clone();
        let fieldname = mission.completionc.clone();
        let string1 = lower_case_string(&mission.completiond);
        let string2 = lower_case_string(&mission.completione);

        // The record that should have been jimmied (None = record not found)
        let fieldvalue: Option<Option<String>> = {
            let comp = computer_at(g.world(), &ip);
            comp.recordbank
                .get_record_from_name(&personname)
                .map(|rec| rec.get_field(&fieldname).map(lower_case_string))
        };

        let Some(fieldvalue) = fieldvalue else {
            // Oh dear.  The record hasn't been found, but it should be there
            panic!("MissionGenerator::IsMissionComplete_ChangeData - could not find record");
        };

        let changed = match &fieldvalue {
            Some(fv) => fv.contains(string1.as_str()) && fv.contains(string2.as_str()),
            None => false,
        };

        if changed {
            // Mission Accomplished
            Self::mission_completed(g, mission, person, message);
            true
        } else {
            Self::mission_not_completed(
                g,
                mission,
                person,
                message,
                "You haven't changed the data yet",
            );
            false
        }
    }

    pub fn is_mission_complete_frame_user(
        g: &mut Ctx,
        mission: &Mission,
        person: &str,
        message: &Message,
    ) -> bool {
        let personname = mission.completiona.as_str();
        let framedfor = mission.completionb.as_str();
        let ip = mission.completionc.as_str();

        // Has the person been arrested?
        // Look through recent news articles
        let arrested = {
            let uplink = uplink_company(g.world());
            let mut arrested = false;

            for news in &uplink.news {
                if news.newstype == NEWS_TYPE_ARREST && news.data1 == personname {
                    if framedfor == "FINANCIAL" || framedfor == "DESTROYALL" {
                        // Make sure he was arrested for hacking this computer
                        arrested = news.data2 == ip;
                    } else {
                        // Could have hacked anywhere - ip not important
                        arrested = true;
                    }
                    break;
                }
            }
            arrested
        };

        if !arrested {
            Self::mission_not_completed(
                g,
                mission,
                person,
                message,
                "That person has not yet been arrested.  We will not make payment until then.",
            );
            return false;
        }

        // They have been arrested - but was the crime commited?
        if framedfor == "HACKING" {
            // Bog standard hack
            if message.from == "PLAYER" {
                g.player_mut().score_peoplefucked += 1;
            }

            Self::mission_completed(g, mission, person, message);
            return true;
        } else if framedfor == "FINANCIAL" {
            // Financial fraud
            let hacked = uplink_company(g.world())
                .news
                .iter()
                .any(|news| news.newstype == NEWS_TYPE_HACKED && news.data1 == ip);

            if hacked {
                if message.from == "PLAYER" {
                    g.player_mut().score_peoplefucked += 1;
                }

                Self::mission_completed(g, mission, person, message);
                return true;
            }
        } else if framedfor == "DESTROYALL" {
            // Destroying all files
            let deleted = uplink_company(g.world())
                .news
                .iter()
                .any(|news| news.newstype == NEWS_TYPE_DELETED && news.data1 == ip);

            if deleted {
                if message.from == "PLAYER" {
                    g.player_mut().score_peoplefucked += 1;
                    g.player_mut().score_systemsfucked += 1;
                }

                Self::mission_completed(g, mission, person, message);
                return true;
            }
        }

        // If we get here then the crime wasn't committed
        Self::mission_failed(
            g,
            mission,
            person,
            "The person was arrested but not for the crimes we specifically asked for.  As such we will not be making payment.",
        );
        false
    }

    pub fn is_mission_complete_trace_user(
        g: &mut Ctx,
        mission: &Mission,
        person: &str,
        message: &Message,
    ) -> bool {
        let personname = lower_case_string(&mission.completiona);
        // NULL unless this is a TraceUser_MoneyTransfer (an empty string stands for NULL here)
        let targetaccount = if mission.completionc.is_empty() {
            None
        } else {
            Some(lower_case_string(&mission.completionc))
        };
        let msgbody = lower_case_string(message.get_body());

        // Check the person's name is in the mail
        if msgbody.contains(personname.as_str()) {
            if message.from == "PLAYER" {
                g.player_mut().score_peoplefucked += 1;
            }

            Self::mission_completed(g, mission, person, message);
            true
        } else {
            match targetaccount {
                Some(ta) if msgbody.contains(ta.as_str()) => {
                    Self::mission_not_completed(
                        g,
                        mission,
                        person,
                        message,
                        "We've determined that the money was transfered into \
                         an account owned by that person, but we do not believe \
                         he was personally responsible.\n\n\
                         We want the name of the Hacker who performed the transfer.",
                    );
                }
                _ => {
                    Self::mission_not_completed(
                        g,
                        mission,
                        person,
                        message,
                        "This person was not responsible.",
                    );
                }
            }
            false
        }
    }

    pub fn is_mission_complete_change_account(
        g: &mut Ctx,
        mission: &Mission,
        person: &str,
        message: &Message,
    ) -> bool {
        let (source_ip, source_acc) = scan_word_int(&mission.completiona);
        let (target_ip, target_acc) = scan_word_int(&mission.completionb);
        let amount = scan_one_int(&mission.completionc);

        // Check the money has been transferred
        let source_acc_s = source_acc.to_string();
        let success = {
            let source_account = BankAccount::get_account(g.world(), &source_ip, &source_acc_s)
                .expect("UplinkAssert(source_account)");
            source_account.has_transfer_occured(
                g.world(),
                &source_ip,
                &target_ip,
                target_acc,
                amount,
            )
        };

        if success {
            Self::mission_completed(g, mission, person, message);
        } else {
            Self::mission_not_completed(
                g,
                mission,
                person,
                message,
                "The transfer has not occured.",
            );
        }

        success
    }

    pub fn is_mission_complete_remove_computer(
        g: &mut Ctx,
        mission: &mut Mission,
        person: &str,
        message: &Message,
    ) -> bool {
        let sourceip = mission.completiona.clone();

        // UplinkAssert(vl) and UplinkAssert(comp)
        let _ = computer_at(g.world(), &sourceip);

        // look for word that the system was shut down
        let hit = uplink_company(g.world())
            .news
            .iter()
            .find(|news| {
                (news.newstype == NEWS_TYPE_COMPUTERDESTROYED
                    || news.newstype == NEWS_TYPE_COMPUTERSHUTDOWN)
                    && news.data1 == sourceip
            })
            .map(|news| news.newstype);

        match hit {
            Some(NEWS_TYPE_COMPUTERDESTROYED) => {
                // Data bank formatted
                // System shut down
                if message.from == "PLAYER" {
                    g.player_mut().score_systemsfucked += 1;
                }

                Self::mission_completed(g, mission, person, message);
                true
            }
            Some(NEWS_TYPE_COMPUTERSHUTDOWN) => {
                // System shut down
                // But files not deleted
                send_mail(
                    g,
                    &message.from,
                    person,
                    "Problems with mission completion",
                    "It appears the target system was shut down, but the files were not deleted.\n\
                     This means the company will most likely be able to recover the data.\n\n\
                     As such, we will only be paying you half the remaining money.\n\n\
                     In future, you should delete all files before shutting the system down.",
                );

                mission.payment /= 2;
                Self::mission_completed(g, mission, person, message);
                true
            }
            _ => {
                Self::mission_not_completed(
                    g,
                    mission,
                    person,
                    message,
                    "We have not heard word yet that the system has been destroyed.",
                );
                false
            }
        }
    }

    pub fn is_mission_complete_remove_company(
        _g: &mut Ctx,
        _mission: &Mission,
        _person: &str,
        _message: &Message,
    ) -> bool {
        false
    }

    pub fn is_mission_complete_remove_user(
        g: &mut Ctx,
        mission: &Mission,
        person: &str,
        message: &Message,
    ) -> bool {
        let personname = mission.completiona.as_str();

        // Look through recent news articles
        let arrested = uplink_company(g.world())
            .news
            .iter()
            .any(|news| news.newstype == NEWS_TYPE_ARREST && news.data1 == personname);

        if arrested {
            if message.from == "PLAYER" {
                g.player_mut().score_peoplefucked += 1;
            }

            Self::mission_completed(g, mission, person, message);
            true
        } else {
            Self::mission_not_completed(
                g,
                mission,
                person,
                message,
                "We don't believe you have done sufficient damage yet.",
            );
            false
        }
    }

    pub fn is_mission_complete_pay_fine(
        g: &mut Ctx,
        mission: &Mission,
        person: &str,
        message: &Message,
    ) -> bool {
        let finedperson = mission.completiona.as_str();
        let (target_ip, target_accno) = scan_two_words(&mission.completionb);
        let amount = scan_one_int(&mission.completionc);

        // Get the target account that should have the money in
        // Check the money has been transfered
        let success = {
            let target_account = BankAccount::get_account(g.world(), &target_ip, &target_accno)
                .expect("UplinkAssert(target_account)");
            target_account.has_transfer_occured_from(finedperson, amount)
        };

        let details = mission.details.as_deref().unwrap_or("");

        if success {
            let body = format!(
                "Payment of your fine has been received for the following offence: \n{}\n\n\
                 We hope this has been a sufficient deterent to hacking \
                 into our systems in future.",
                details
            );
            send_mail(g, &message.from, person, "Fine paid", &body);
        } else {
            let body = format!(
                "We have still not received payment for the following fine: \n{}\n\n\
                 Please ensure this payment reaches us soon to avoid further action.",
                details
            );
            send_mail(
                g,
                &message.from,
                person,
                "Outstanding fine still not paid",
                &body,
            );
        }

        success
    }

    pub fn is_mission_complete_special(
        g: &mut Ctx,
        mission: &Mission,
        person: &str,
        message: &Message,
    ) -> bool {
        if mission.description == PlotGenerator::special_mission_description(SPECIALMISSION_MOLE) {
            // First - has the player already done this?
            if g.world()
                .plotgenerator
                .player_completed_special_mission(SPECIALMISSION_MOLE)
            {
                send_mail(
                    g,
                    &message.from,
                    person,
                    "Mission withdrawn",
                    "It appears you have already sold the information.\n\n\
                     Since it is now in the public domain, we have withdrawn the mission offer.",
                );

                return true;
            }

            let ourcompip = mission.completiona.as_str();

            // Work out how much of the data has been successfully stolen
            // Ignore duplicate files or anything that doesn't have the
            // correct filename
            let (foundsize, foundprogram) = {
                let ourcomp = computer_at(g.world(), ourcompip);
                let numfiles: i32 = 10;
                let mut foundsize = 0;
                let mut foundprogram = false;
                let mut filefound = vec![false; numfiles as usize];

                for i in 0..ourcomp.databank.get_data_size() {
                    if let Some(thisfile) = ourcomp.databank.get_data_file(i) {
                        if thisfile.title.contains("Uplink_Agent_Data") && thisfile.encrypted == 0 {
                            let thisfileindex = data_file_index(&thisfile.title);

                            if thisfileindex >= 0
                                && thisfileindex < numfiles
                                && !filefound[thisfileindex as usize]
                            {
                                foundsize += thisfile.size;
                                filefound[thisfileindex as usize] = true;
                            }
                        }

                        if thisfile.title == "Uplink_Agent_List" && thisfile.encrypted == 0 {
                            foundprogram = true;
                        }
                    }
                }

                (foundsize, foundprogram)
            };

            // Respond based on the data size found
            if foundsize == 0 && !foundprogram {
                Self::mission_not_completed(
                    g,
                    mission,
                    person,
                    message,
                    "We don't see any data on our fileserver that contains \
                     the names of any Uplink Agents.  Did you upload and \
                     decrypt the data?",
                );
                return false;
            } else if foundsize == 0 && foundprogram {
                Self::mission_not_completed(
                    g,
                    mission,
                    person,
                    message,
                    "Looks like you've uploaded the Uplink Agent List program, \
                     but we don't see any data on our fileserver that contains \
                     the names of any Uplink Agents.  The program is useless \
                     without the data.\n\n\
                     Did you upload and decrypt the data?",
                );
                return false;
            } else if foundsize > 0 {
                let foundpercent = (100.0 * (foundsize as f32 / 50.0)) as i32;

                let mut payment = (PAYMENT_SPECIALMISSION_MOLE as f64
                    * 0.75
                    * (foundpercent as f32 / 100.0) as f64)
                    as i32;
                if foundprogram {
                    payment = (payment as f64 + PAYMENT_SPECIALMISSION_MOLE as f64 * 0.25) as i32;
                }
                payment = 100 * (payment / 100);

                let mut body = String::new();
                body.push_str(
                    "Weve just checked our fileserver, and we can see lots of data containing the handles of \
                     Agents and their real-world names.  This data will come in very handy.\n",
                );

                if foundpercent == 100 {
                    body.push_str("In the end we were able to achieve almost all of the Uplink Agent names.\n");
                } else {
                    body.push_str(&format!(
                        "In the end we were able to achieve roughly {} percent of the Uplink Agent names.\n",
                        10 * (foundpercent / 10)
                    ));
                }

                if foundprogram {
                    body.push_str("We also noticed you uploaded the List program used by Uplink Corporation.\n");
                } else {
                    body.push_str(
                        "Unfortunately you didnt upload the List program used to access the data. This will make \
                         recovery of the source data more difficult.\n",
                    );
                }

                if payment == PAYMENT_SPECIALMISSION_MOLE {
                    body.push_str(&format!(
                        "We will therefore be paying you the full {} credits.\n\n",
                        payment
                    ));
                } else {
                    body.push_str(&format!(
                        "Based on this, we will only be paying you {} credits for the mission, \
                         out of an available {}.\n\n",
                        payment, PAYMENT_SPECIALMISSION_MOLE
                    ));
                }

                body.push_str("Once again, congratulations on an incredible hack.\n\nEND");

                send_mail(g, &message.from, person, "Mission completed", &body);

                // Money, ratings etc
                Person::change_balance(g, "PLAYER", payment, "Services Rendered");
                Rating::change_neuromancer_score(g, "PLAYER", -200);
                g.world_mut().plotgenerator.specialmissionscompleted |= 1 << SPECIALMISSION_MOLE;

                // Schedule the murders for a week from now
                let mut rundate = Date::default();
                rundate.set_date_from(&g.world().date);
                rundate.advance_day(7);

                let mut ne = NotificationEvent::new();
                ne.set_type(NOTIFICATIONEVENT_TYPE_AGENTSONLISTDIE);
                ne.base.set_run_date(&rundate);
                g.world_mut()
                    .scheduler
                    .schedule_event(UplinkEvent::NotificationEvent(ne));

                return true;
            }
        }

        false
    }

    pub fn mission_completed(g: &mut Ctx, mission: &Mission, person: &str, message: &Message) {
        let body = format!(
            "Congratulations on your success.\n\
             Funds transfered : {}c\n\
             \n\
             Thankyou for working for {}.\n\n\
             THIS IS AN AUTOMATED RESPONSE.  PLEASE DO NOT REPLY.",
            mission.payment, mission.employer
        );
        send_mail(g, &message.from, person, "Mission completed", &body);

        if mission.paymentmethod == MISSIONPAYMENT_HALFATSTART {
            Person::change_balance(g, "PLAYER", mission.payment / 2, "Anonymous benefactor");
        } else if mission.paymentmethod == MISSIONPAYMENT_AFTERCOMPLETION {
            Person::change_balance(g, "PLAYER", mission.payment, "Anonymous benefactor");
        }

        Rating::change_uplink_score(g, "PLAYER", mission.difficulty);
        // PORT: C++ indexes NEUROMANCERCHANGE directly; types outside the table (PAYFINE never
        // reaches here) read out of bounds there, so they count as 0 here.
        let neuromancerchange = usize::try_from(mission.type_)
            .ok()
            .and_then(|t| NEUROMANCERCHANGE.get(t).copied())
            .unwrap_or(0);
        Rating::change_neuromancer_score(g, "PLAYER", neuromancerchange);

        ConsequenceGenerator::mission_completed(g, mission, "PLAYER");

        if globals::DEMOGAME {
            DemoPlotGenerator::player_completed_mission(g, mission);
        }

        // Unselect all of the links on the world map
        for ip in &mission.links {
            let targetable = computer_at(g.world(), ip).istargetable;
            if targetable {
                g.world_mut()
                    .get_vlocation_mut(ip)
                    .expect("UplinkAssert(vl)")
                    .set_displayed(false);
            }
        }

        sound::play_sound("sounds/success.wav", None, false);
    }

    pub fn mission_not_completed(
        g: &mut Ctx,
        _mission: &Mission,
        person: &str,
        message: &Message,
        reason: &str,
    ) {
        let body = format!(
            "{}\n\nTHIS IS AN AUTOMATED RESPONSE.  PLEASE DO NOT REPLY.",
            reason
        );
        send_mail(g, &message.from, person, "Mission NOT completed", &body);
    }

    /// Call this if a person fails a mission -- ie runs out of time or totally messes it up.
    pub fn mission_failed(g: &mut Ctx, mission: &Mission, person: &str, reason: &str) {
        // Knock off some points from the person's score
        Rating::change_uplink_score(g, "PLAYER", mission.difficulty * -2);

        if mission.type_ == MISSION_PAYFINE {
            ConsequenceGenerator::didnt_pay_fine(g, person, mission);

            send_mail(
                g,
                "PLAYER",
                &mission.employer,
                "Failure to pay fine",
                "You have chosen not to pay a fine. As such, \
                 legal proceedings will begin against you shortly.\n\n\
                 You are advised to seek legal assistance.",
            );
        } else {
            // Send a mail to the person
            let body = format!(
                "You have failed the following mission: \n{}\n{}\n\n\
                 In future we will employ someone more qualified.\n\n\
                 THIS IS AN AUTOMATED RESPONSE.  PLEASE DO NOT REPLY.",
                mission.description, reason
            );
            send_mail(g, "PLAYER", &mission.employer, "Mission FAILED", &body);
        }
    }
}

/// C++ `int x = RandomNormalNumber(mean, range)`: the float result is truncated toward zero.
fn normal_int(mean: i32, range: f32) -> i32 {
    NumberGenerator::random_normal_number(mean as f32, range) as i32
}

// ---- Private helpers shared by the generators above (no C++ counterpart) ----

/// `"internal@%s.net"`: the completion-mail contact at an employer.
fn internal_contact(employer: &str) -> String {
    format!("internal@{}.net", employer)
}

/// `(int) NumberGenerator::RandomNormalNumber(mean, range)`: the float result is truncated.
fn random_normal_int(mean: f32, range: f32) -> i32 {
    NumberGenerator::random_normal_number(mean, range) as i32
}

/// `int difficulty = RandomNormalNumber(min, DIFFICULTY_MISSION_VARIANCE); if (< min) = min;`
fn roll_difficulty(min: i32) -> i32 {
    let difficulty = random_normal_int(min as f32, DIFFICULTY_MISSION_VARIANCE as f32);
    if difficulty < min {
        min
    } else {
        difficulty
    }
}

/// `RandomNormalNumber(difficulty * per_unit, difficulty * per_unit * PAYMENT_MISSION_VARIANCE)`
/// (C++ evaluates the variance in float).
fn roll_payment(difficulty: i32, per_unit: i32) -> i32 {
    let base = difficulty * per_unit;
    let variance = base as f32 * PAYMENT_MISSION_VARIANCE;
    random_normal_int(base as f32, variance)
}

/// `minrating = difficulty; acceptrating = minrating + RandomNumber(spread);`, both clamped to 10.
fn ratings(difficulty: i32, spread: i32) -> (i32, i32) {
    let mut minrating = difficulty;
    let mut acceptrating = minrating + NumberGenerator::random_number(spread);

    if minrating > 10 {
        minrating = 10;
    }
    if acceptrating > 10 {
        acceptrating = 10;
    }

    (minrating, acceptrating)
}

/// C++ `payment * 1.1` passed as the `int newmaxpayment` of `SetPayment`: truncated toward zero.
fn max_payment(payment: i32) -> i32 {
    (payment as f64 * 1.1) as i32
}

/// Snapshot of the Computer fields the generators read (C++ `->name`, `->ip`, `->companyname`, `->TYPE`).
struct Target {
    name: String,
    ip: String,
    companyname: String,
    type_: i32,
}

fn target_info(g: &Ctx, key: &str) -> Target {
    let c = g
        .world()
        .get_computer(key)
        .unwrap_or_else(|| panic!("computer {key} not found"));
    Target {
        name: c.name.clone(),
        ip: c.ip.clone(),
        companyname: c.companyname.clone(),
        type_: c.type_,
    }
}

fn computer_mut<'a>(g: &'a mut Ctx, key: &str) -> &'a mut Computer {
    g.world_mut()
        .get_computer_mut(key)
        .unwrap_or_else(|| panic!("computer {key} not found"))
}

/// C++ `BankComputer::GetRandomAccount()` on the bank `key`; returns the account's (name, accountnumber).
fn random_account_of(g: &Ctx, key: &str) -> (String, i32) {
    let bank = g
        .world()
        .get_computer(key)
        .and_then(|c| c.bank())
        .unwrap_or_else(|| panic!("{key} is not a bank computer"));
    let acc = bank
        .get_random_account()
        .expect("bank computer has accounts");
    (acc.name.clone(), acc.accountnumber)
}

/// Name of the player's agent (C++ `GetPlayer()->handle`).
fn player_handle(g: &Ctx) -> String {
    g.player().person.agent().handle.clone()
}

/// C++ `Date postdate; postdate.SetDate(world date); AdvanceHour(-RandomNumber(96)); AdvanceMinute(-RandomNumber(60));`
fn random_post_date(g: &Ctx) -> Date {
    let mut postdate = Date::default();
    postdate.set_date_from(&g.world().date);
    postdate.advance_hour(-NumberGenerator::random_number(96));
    postdate.advance_minute(-NumberGenerator::random_number(60));
    postdate
}

/// C++ `if ( !game->IsRunning () ) mission->SetCreateDate ( &postdate );`
fn set_create_date_unless_running(g: &Ctx, mission: &mut Mission, postdate: &Date) {
    if !g.game.is_running() {
        mission.set_create_date(postdate);
    }
}

/// C++ `CompanyUplink::CreateMission(mission)` followed by `return mission`. CreateMission stamps the
/// world date when the mission has none (year 1000); the returned clone carries the same stamp, as the
/// C++ pointer would.
fn post_mission(g: &mut Ctx, mut mission: Mission) -> Mission {
    if mission.createdate.get_year() == 1000 {
        mission.set_create_date(&g.world().date);
    }
    CompanyUplink::create_mission(g, mission.clone());
    mission
}

/// C++ `if (FindValidPlacement(data) == -1) SetSize(GetSize() + data->size + 2); InsertData(data);`
fn add_data_to_computer(g: &mut Ctx, key: &str, data: Data) {
    let c = computer_mut(g, key);
    if c.databank.find_valid_placement(&data) == -1 {
        let newsize = c.databank.get_size() + data.size + 2;
        c.databank.set_size(newsize);
    }
    c.databank.insert_data(data);
}

/// C++ `while (FindValidPlacement(file) == -1) SetSize(GetSize() + file->size); PutData(file);`
fn put_file(g: &mut Ctx, key: &str, file: Data) {
    let c = computer_mut(g, key);
    while c.databank.find_valid_placement(&file) == -1 {
        let newsize = c.databank.get_size() + file.size;
        c.databank.set_size(newsize);
    }
    c.databank.put_data(file);
}

/// C++ `companyname[0..3]`: the first four characters of the company name.
fn company_prefix(companyname: &str) -> String {
    companyname.chars().take(4).collect()
}

/// C++ `'a' + RandomNumber(26)`.
fn random_letter() -> char {
    (b'a' + NumberGenerator::random_number(26) as u8) as char
}

/// The "Payment for this job..." block that begins every mission's `details`.
fn standard_details(payment: i32, difficulty: i32, acceptrating: i32) -> String {
    format!(
        "Payment for this job is {} credits.\nThis job has been assigned an Uplink difficulty of {}.\nAn UplinkRating of {} or above will be sufficient for automatic acceptance.\n\n",
        payment,
        difficulty,
        Rating::get_uplink_rating_string(acceptrating)
    )
}

/// `cu->CreateMission(mission); return mission;`. CreateMission stamps an unset create date
/// (year 1000) with the world date; that stamp is applied here before the copy is returned,
/// so the returned mission matches the posted one.
fn post_mission_p2(g: &mut Ctx, mut mission: Mission) -> Option<Mission> {
    if mission.createdate.get_year() == 1000 {
        mission.set_create_date(&g.world().date);
    }
    CompanyUplink::create_mission(g, mission.clone());
    Some(mission)
}

/// `(name, ip)` of the computer `key` (UplinkAssert non-NULL in C++).
fn computer_name_ip(g: &Ctx, key: &str) -> (String, String) {
    let comp = g.world().get_computer(key).expect("UplinkAssert(computer)");
    (comp.name.clone(), comp.ip.clone())
}

/// A clone of a random account on bank computer `key` (UplinkAssert non-NULL in C++).
fn bank_random_account(g: &Ctx, key: &str) -> BankAccount {
    g.world()
        .get_computer(key)
        .and_then(|c| c.bank())
        .and_then(|b| b.get_random_account())
        .expect("UplinkAssert(account)")
        .clone()
}

/// `name` of the person `key` (UplinkAssert non-NULL in C++).
fn person_name_of(g: &Ctx, key: &str) -> String {
    g.world()
        .get_person(key)
        .expect("UplinkAssert(person)")
        .name
        .clone()
}

/// `hacker == game->GetWorld()->GetPlayer()`: pointer equality on the player's Person.
fn is_player(g: &Ctx, person: &str) -> bool {
    g.world()
        .get_person(person)
        .is_some_and(|p| std::ptr::eq(p, &g.player().person))
}

/// `GetRandomRecord(query)` on computer `computer`, returning the person it names.
/// `None` (C++ `return NULL`) when there is no matching record or it is the player's own record.
fn pick_target_person(g: &Ctx, computer: &str, query: &str) -> Option<String> {
    let comp = g
        .world()
        .get_computer(computer)
        .expect("UplinkAssert(target)");
    let record = comp.recordbank.get_random_record(query)?;
    let personname = record
        .get_field(RECORDBANK_NAME)
        .expect("UplinkAssert(personname)")
        .to_string();

    if personname == g.player().agent().handle {
        return None;
    }

    assert!(
        g.world().get_person(&personname).is_some(),
        "UplinkAssert(person)"
    );
    Some(personname)
}

// ---------------------------------------------------------------------------
// Helpers for the mission checks (replace the C++ sscanf / UplinkAssert idioms)
// ---------------------------------------------------------------------------

/// C++ `sscanf("%d")` on one token: optional sign, then leading decimal digits.
fn scan_int(token: &str) -> Option<i32> {
    let bytes = token.as_bytes();
    let mut end = 0;
    if end < bytes.len() && (bytes[end] == b'+' || bytes[end] == b'-') {
        end += 1;
    }
    let digits_start = end;
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    if end == digits_start {
        return None;
    }
    token[..end].parse::<i32>().ok()
}

/// C++ `sscanf(s, "%d", &a)`.
fn scan_one_int(s: &str) -> i32 {
    s.split_whitespace().next().and_then(scan_int).unwrap_or(0)
}

/// C++ `sscanf(s, "%s %d", ...)`.
fn scan_word_int(s: &str) -> (String, i32) {
    let mut it = s.split_whitespace();
    let word = it.next().unwrap_or("").to_string();
    let n = it.next().and_then(scan_int).unwrap_or(0);
    (word, n)
}

/// C++ `sscanf(s, "%s %s", ...)`.
fn scan_two_words(s: &str) -> (String, String) {
    let mut it = s.split_whitespace();
    let a = it.next().unwrap_or("").to_string();
    let b = it.next().unwrap_or("").to_string();
    (a, b)
}

/// C++ `sscanf(s, "%d %d", ...)`.
fn scan_two_ints(s: &str) -> (i32, i32) {
    let mut it = s.split_whitespace();
    let a = it.next().and_then(scan_int).unwrap_or(0);
    let b = it.next().and_then(scan_int).unwrap_or(0);
    (a, b)
}

/// The file index in a title such as "corp-ABCD 3" (second word); -1 if absent.
fn data_file_index(title: &str) -> i32 {
    title
        .split_whitespace()
        .nth(1)
        .and_then(scan_int)
        .unwrap_or(-1)
}

/// The "Uplink" company (mission board and news). UplinkAssert: must exist.
fn uplink_company(world: &World) -> &CompanyUplink {
    world
        .get_company("Uplink")
        .expect("UplinkAssert(uplink)")
        .uplink
        .as_deref()
        .expect("UplinkAssert(uplink)")
}

/// The computer at `ip`. UplinkAssert(vl) and UplinkAssert(comp).
fn computer_at<'a>(world: &'a World, ip: &str) -> &'a Computer {
    world
        .get_vlocation(ip)
        .expect("UplinkAssert(vl)")
        .get_computer(world)
        .expect("UplinkAssert(comp)")
}

/// Sends a mail with the given to / from / subject / body.
fn send_mail(g: &mut Ctx, to: &str, from: &str, subject: &str, body: &str) {
    let mut msg = Message::new();
    msg.set_to(to);
    msg.set_from(from);
    msg.set_subject(subject);
    msg.set_body(body);
    msg.send(g);
}
