//! Port of source/world/generator/recordgenerator.h + source/world/generator/recordgenerator.cpp

use crate::ctx::Ctx;
use crate::game::data::data::{
    GAME_START_DATE, PERCENTAGE_AGENTSWITHCONVICTIONS, PERCENTAGE_PEOPLEWITHCONVICTIONS,
};
use crate::world::computer::bankaccount::BankAccount;
use crate::world::computer::bankcomputer::BankComputer;
use crate::world::computer::computer::COMPUTER_TYPE_PUBLICBANKSERVER;
use crate::world::computer::logbank::{
    AccessLog, LOG_NOTSUSPICIOUS, LOG_TYPE_TRANSFERFROM, LOG_TYPE_TRANSFERTO,
};
use crate::world::computer::recordbank::{Record, RECORDBANK_NAME};
use crate::world::date::Date;
use crate::world::generator::namegenerator::NameGenerator;
use crate::world::generator::numbergenerator::NumberGenerator;
use crate::world::generator::worldgenerator::WorldGenerator;
use crate::world::person::Person;
use crate::world::world::World;

/*
    Record Generator Class

    Responsible for generating records for a person -
    such as criminal records, social security etc.
*/

// PORT: every generator adds records to the world databases (International Social Security
// Database, Global Criminal Database, ...), so they take `g: &mut Ctx` and the person's name.

// Database names (the computers that hold each kind of record)
const SOCIAL_SECURITY_DB: &str = "International Social Security Database";
const CRIMINAL_DB: &str = "Global Criminal Database";
const MEDICAL_DB: &str = "Central Medical Database";
const ACADEMIC_DB: &str = "International Academic Database";

/// The bank computer called `bank_name` (C: `(BankComputer *) comp`, asserted to be one).
fn bank_of<'a>(world: &'a World, bank_name: &str) -> &'a BankComputer {
    world
        .get_computer(bank_name)
        .and_then(|c| c.bank())
        .expect("RecordGenerator: not a bank computer")
}

fn bank_of_mut<'a>(world: &'a mut World, bank_name: &str) -> &'a mut BankComputer {
    world
        .get_computer_mut(bank_name)
        .and_then(|c| c.bank_mut())
        .expect("RecordGenerator: not a bank computer")
}

/// Adds `record` to the record bank of the computer called `computer_name`.
fn add_record(g: &mut Ctx, computer_name: &str, record: Record, sortfield: &str) {
    let comp = g
        .world_mut()
        .get_computer_mut(computer_name)
        .expect("RecordGenerator: database computer missing");
    comp.recordbank.add_record_sorted(record, sortfield);
}

/// The name a record lookup uses: "PLAYER" stands for the player's handle.
fn record_lookup_name(g: &Ctx, personname: &str) -> String {
    if personname == "PLAYER" {
        g.world().get_player().agent().handle.clone()
    } else {
        personname.to_string()
    }
}

/// `comp->recordbank.GetRecordFromName(name)` on the database computer `computer_name`.
fn get_record_in<'a>(g: &'a Ctx, computer_name: &str, personname: &str) -> Option<&'a Record> {
    let comp = g
        .world()
        .get_computer(computer_name)
        .expect("RecordGenerator: database computer missing");
    let name = record_lookup_name(g, personname);
    comp.recordbank.get_record_from_name(&name)
}

pub struct RecordGenerator;

impl RecordGenerator {
    pub fn initialise() {}

    // Top level functions for generating an entire set of records for a person

    pub fn generate_records(g: &mut Ctx, personname: &str) {
        let person = g
            .world()
            .get_person(personname)
            .expect("RecordGenerator::GenerateRecords, no person");
        let age = person.age;

        Self::generate_record_social_security(g, personname, age);
        Self::generate_record_criminal(g, personname, age);
        // C: GenerateRecord_Medical is commented out in the original, so it is not generated.
        Self::generate_record_academic(g, personname, age);
        Self::generate_record_financial(g, personname, age);
    }

    pub fn generate_records_player(g: &mut Ctx, playername: &str) {
        // Social Security Record

        let mut socsec = Record::new();
        socsec.add_field(RECORDBANK_NAME, playername);
        socsec.add_field("D.O.B", "24 - 3 - 1989");
        socsec.add_field("Social Security", "14011401");
        socsec.add_field("Marital Status", "Single");
        socsec.add_field("Personal Status", "Self-Employed");

        add_record(g, SOCIAL_SECURITY_DB, socsec, RECORDBANK_NAME);

        // Criminal Record

        let mut crim = Record::new();
        crim.add_field(RECORDBANK_NAME, playername);
        crim.add_field("Convictions", "None");

        add_record(g, CRIMINAL_DB, crim, RECORDBANK_NAME);

        // Academic Record

        let mut ac = Record::new();
        ac.add_field(RECORDBANK_NAME, playername);
        ac.add_field("IQ", "172");
        ac.add_field(
            "College",
            "Maths    -   A\n\
             Physics    -    A\n\
             Electronics    -   A\n\
             Computing    -    A",
        );
        ac.add_field("University", "Computer Science, Class 1");
        ac.add_field("Other", "Registered Uplink Agent");

        add_record(g, ACADEMIC_DB, ac, RECORDBANK_NAME);
    }

    // Low level functions for generating specific records

    pub fn generate_record_social_security(g: &mut Ctx, personname: &str, age: i32) {
        // Generate a Social Security Record
        // Add it into the International Social Security Database

        // C: sprintf ( dob, "%d - %d - %d", RandomNumber(28)+1, RandomNumber(12)+1, 2010 - age );
        // PORT: C leaves the order of the two random draws unspecified; drawn left to right here.
        let dob_day = NumberGenerator::random_number(28) + 1;
        let dob_month = NumberGenerator::random_number(12) + 1;
        let dob = format!("{} - {} - {}", dob_day, dob_month, 2010 - age);

        let socialsecurity = format!("{}", 10000000 + NumberGenerator::random_number(99999999));

        let maritalstatus = match NumberGenerator::random_number(5) {
            0 => "Single",
            1 => "Married",
            2 => "Seperated",
            3 => "Divorced",
            4 => "Widowed",
            _ => unreachable!("RandomNumber(5) is 0..=4"),
        };

        let personalstatus = match NumberGenerator::random_number(5) {
            0 => "Employed",
            1 => "Un-Employed",
            2 => "Student",
            3 => "Self-Employed",
            4 => "Deceased",
            _ => unreachable!("RandomNumber(5) is 0..=4"),
        };

        let mut soc = Record::new();
        soc.add_field(RECORDBANK_NAME, personname);
        soc.add_field("Social Security", &socialsecurity);
        soc.add_field("D.O.B", &dob);
        soc.add_field("Marital Status", maritalstatus);
        soc.add_field("Personal Status", personalstatus);

        add_record(g, SOCIAL_SECURITY_DB, soc, "Social Security");
    }

    pub fn generate_record_criminal(g: &mut Ctx, personname: &str, age: i32) {
        // Generate a possible criminal past
        // Only a small percentage of people have any convictions

        let uplinkrating = g
            .world()
            .get_person(personname)
            .expect("RecordGenerator::GenerateRecord_Criminal, no person")
            .rating
            .uplinkrating;

        // C: ostrstream convictions; each conviction is written as a line ending in "\n".
        let mut convictions = String::new();

        if uplinkrating == 0 {
            // Ordinary person

            if NumberGenerator::random_number(100) < PERCENTAGE_PEOPLEWITHCONVICTIONS {
                // Greater age = more possible convictions

                let numconvictions = (age / 10) + NumberGenerator::random_number(4);

                for _ in 0..numconvictions {
                    let line = match NumberGenerator::random_normal_number(6.0, 6.0) as i32 {
                        0 => "Murder\n",
                        1 => "Armed Robbery\n",
                        2 => "Robbery\n",
                        3 => "Petty theft\n",
                        4..=7 => "",
                        8 => "Disturbing the peace\n",
                        9 => "Reckless driving\n",
                        10 => "Man-slaughter\n",
                        11 => "G-B-H\n",
                        12 => "Rape\n",
                        _ => panic!(
                            "RecordGenerator::GenerateRecord_Criminal, unrecognised conviction id"
                        ),
                    };
                    convictions.push_str(line);
                }
            } else {
                convictions.push_str("None");
            }
        } else {
            // This guy is an Uplink Agent

            if NumberGenerator::random_number(100) < PERCENTAGE_AGENTSWITHCONVICTIONS {
                // Greater rating = more possible convictions

                let numconvictions = uplinkrating + NumberGenerator::random_number(2);

                for _ in 0..numconvictions {
                    let line = match NumberGenerator::random_normal_number(6.0, 6.0) as i32 {
                        0 => "High tech fraud\n",
                        1 => "Willful destruction of data\n",
                        2 => "Theft of sensitive data\n",
                        3 => "Unlawful access of classified data\n",
                        4..=7 => "",
                        8 => "Falsifying academic results\n",
                        9 => "Illegal modification of credit rating\n",
                        10 => "Illegally transfering funds\n",
                        11 => "Credit card fraud\n",
                        12 => "Destruction of financial data\n",
                        _ => panic!(
                            "RecordGenerator::GenerateRecord_Criminal, unrecognised conviction id"
                        ),
                    };
                    convictions.push_str(line);
                }
            } else {
                convictions.push_str("None");
            }
        }

        // C: convictions << '\x0'; the string ends at that NUL, so nothing after it is kept.

        let mut crim = Record::new();
        crim.add_field(RECORDBANK_NAME, personname);
        crim.add_field("Convictions", &convictions);

        add_record(g, CRIMINAL_DB, crim, RECORDBANK_NAME);
    }

    pub fn generate_record_medical(g: &mut Ctx, personname: &str, age: i32) {
        // C never reads age here.
        let _ = age;

        let mut rec = Record::new();
        rec.add_field(RECORDBANK_NAME, personname);
        rec.add_field("Known alergies", "None");
        rec.add_field("History", "None");

        add_record(g, MEDICAL_DB, rec, RECORDBANK_NAME);
    }

    pub fn generate_record_academic(g: &mut Ctx, personname: &str, age: i32) {
        // Generate an academic history based on age

        let brains = NumberGenerator::random_normal_number(50.0, 50.0) as i32; // 0 = thick, 50 = average, 100 = rocket scientist

        let mut collegequals = String::new();
        let mut universityquals = String::new();
        let mut otherquals = String::new();

        if age >= 16 {
            // Has some college quals

            let mut possiblequals: Vec<&str> = vec![
                "Science",
                "Maths",
                "Physics",
                "Biology",
                "Art",
                "History ",
                "Psychology",
                "Electronics",
                "Computing",
                "Media studies",
                "Sociology",
            ];
            let grades = ['A', 'B', 'C', 'D', 'E', 'F', 'G'];

            let numquals = NumberGenerator::random_number(brains / 10);

            if numquals > 0 {
                for _ in 0..numquals {
                    let thisqualindex = NumberGenerator::random_number(possiblequals.len() as i32);
                    assert!(
                        thisqualindex >= 0 && (thisqualindex as usize) < possiblequals.len(),
                        "RecordGenerator::GenerateRecord_Academic, qualification index invalid"
                    );
                    let idx = thisqualindex as usize;

                    // C: int grade = 1 + RandomNormalNumber ( 100 - brains, 40 ) / 15;   (float, truncated)
                    let mut grade = (1.0f32
                        + NumberGenerator::random_normal_number((100 - brains) as f32, 40.0)
                            / 15.0f32) as i32;
                    if grade < 0 {
                        grade = 0;
                    }
                    if grade > 6 {
                        grade = 6;
                    }

                    collegequals.push_str(&format!(
                        "{}      {}\n",
                        possiblequals[idx], grades[grade as usize]
                    ));

                    possiblequals.remove(idx);
                }
            } else {
                collegequals.push_str("None");
            }
        } else {
            collegequals.push_str("None");
        }

        if age >= 21 && brains > 50 {
            // Has a degree

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
                _ => panic!("RecordGenerator::GenerateRecord_Academic, error in Degree Type"),
            };

            let degreelevel = 1 + (100 - brains) / 25;

            universityquals.push_str(&format!("{}, Class {}", degreesubject, degreelevel));
        } else {
            // Does not have a degree
            universityquals.push_str("None");
        }

        if age > 18 && brains > 50 {
            // Generate qualification

            let qualification = match NumberGenerator::random_number(5) {
                0 => "Diploma in Computing",
                1 => "IEEE Accreditation",
                2 => "Certified systems engineer",
                3 => "Btec in IT",
                4 => "Systems Engineering diploma",
                _ => panic!("RecordGenerator::GenerateRecord_Academic, error in Qualification Type"),
            };

            otherquals.push_str(qualification);
        } else {
            otherquals.push_str("None");
        }

        let mut rec = Record::new();
        rec.add_field(RECORDBANK_NAME, personname);
        rec.add_field_int("IQ", brains + 50);
        rec.add_field("College", &collegequals);
        rec.add_field("University", &universityquals);
        rec.add_field("Other", &otherquals);

        add_record(g, ACADEMIC_DB, rec, RECORDBANK_NAME);
    }

    pub fn generate_record_financial(g: &mut Ctx, personname: &str, age: i32) {
        // C never reads age here.
        let _ = age;

        // Find a financial computer - owned by one of the banks of course
        // And create an account for this person

        let bank_name = WorldGenerator::get_random_computer_of_type(g, COMPUTER_TYPE_PUBLICBANKSERVER);
        let bank_ip = {
            let comp = g
                .world()
                .get_computer(&bank_name)
                .expect("RecordGenerator::GenerateRecord_Financial, no bank computer");
            assert!(
                comp.bank().is_some(),
                "RecordGenerator::GenerateRecord_Financial, not a bank computer"
            );
            comp.ip.clone()
        };

        // Base level value - used to determine this man's situation
        //

        let financial_situation = NumberGenerator::random_normal_number(10.0, 10.0) as i32; // 0 = fucked, 10 = average, 20 = Gates

        let balance = ((financial_situation - 6) as f32
            * NumberGenerator::random_normal_number(1000.0, 500.0)) as i32;
        let loan = if financial_situation < 10 {
            (NumberGenerator::random_normal_number((10 - financial_situation) as f32, 5.0) * 1000.0)
                as i32
        } else {
            0
        };
        let loan = (loan / 100) * 100;
        // C also computes `creditrating = financial_situation - 10` here, but never uses it.

        // Add this account in

        let person_name = {
            let person: &Person = g
                .world()
                .get_person(personname)
                .expect("RecordGenerator::GenerateRecord_Financial, no person");
            person.name.clone()
        };
        let password = NameGenerator::generate_password(g);
        let accno = Person::create_new_account(
            g,
            personname,
            &bank_ip,
            personname,
            &password,
            balance,
            loan,
        );

        let accno_s = accno.to_string();

        // BankAccount *myaccount = bank->accounts.GetData (accno_s);  UplinkAssert (myaccount);
        assert!(
            bank_of(g.world(), &bank_name).accounts.contains_key(&accno_s),
            "RecordGenerator::GenerateRecord_Financial, new account missing"
        );

        // Generate some records
        // By performing a transfer every few days
        // (This won't do anything if we have passed the game start date)

        let mut transferdate = Date::default();
        let mut nomoredate = Date::default();
        transferdate.set_date_from(&g.world().date);
        let (s, mi, h, d, mo, y) = GAME_START_DATE;
        nomoredate.set_date(s, mi, h, d, mo, y);

        while transferdate.before(&nomoredate) {
            let target_bank_name =
                WorldGenerator::get_random_computer_of_type(g, COMPUTER_TYPE_PUBLICBANKSERVER);

            // target_bank->GetRandomAccount ()  (its ip, and the owner name and number of the account)
            let (target_bank_ip, target_acc) = {
                let target_comp = g
                    .world()
                    .get_computer(&target_bank_name)
                    .expect("RecordGenerator::GenerateRecord_Financial, no target bank");
                let target_bank = target_comp
                    .bank()
                    .expect("RecordGenerator::GenerateRecord_Financial, target is not a bank");
                let acc = target_bank
                    .get_random_account()
                    .map(|a| (a.name.clone(), a.accountnumber));
                (target_comp.ip.clone(), acc)
            };

            if let Some((target_owner, target_accno)) = target_acc {
                // Prevent transfers to player
                let player_handle = g.world().get_player().agent().handle.clone();
                if target_owner != player_handle {
                    // Prevent transfers to my own account
                    if !(target_bank_name == bank_name && target_accno == accno) {
                        let myaccount_balance = bank_of(g.world(), &bank_name)
                            .accounts
                            .get(&accno_s)
                            .expect("RecordGenerator::GenerateRecord_Financial, my account missing")
                            .balance;

                        let mean = (myaccount_balance as f64 * 0.3) as f32;
                        let range = (myaccount_balance as f64 * 0.2) as f32;
                        let mut amount = NumberGenerator::random_normal_number(mean, range) as i32;
                        amount = (amount / 50) * 50;

                        let source_data = format!("{} {}", target_bank_ip, target_accno);
                        let target_data = format!("{} {}", bank_ip, accno);
                        let s_amount = amount.to_string();

                        let mut source_log = AccessLog::new();
                        source_log.set_properties(
                            &transferdate,
                            &bank_ip,
                            &person_name,
                            LOG_NOTSUSPICIOUS,
                            LOG_TYPE_TRANSFERTO,
                        );
                        source_log.set_data1(Some(&source_data));
                        source_log.set_data2(Some(&s_amount));
                        source_log.set_data3(Some(&person_name));
                        bank_of_mut(g.world_mut(), &bank_name)
                            .accounts
                            .get_mut(&accno_s)
                            .expect("RecordGenerator::GenerateRecord_Financial, my account missing")
                            .log
                            .add_log(source_log, -1);

                        let mut target_log = AccessLog::new();
                        target_log.set_properties(
                            &transferdate,
                            &bank_ip,
                            &person_name,
                            LOG_NOTSUSPICIOUS,
                            LOG_TYPE_TRANSFERFROM,
                        );
                        target_log.set_data1(Some(&target_data));
                        target_log.set_data2(Some(&s_amount));
                        target_log.set_data3(Some(&person_name));
                        bank_of_mut(g.world_mut(), &target_bank_name)
                            .accounts
                            .get_mut(&target_accno.to_string())
                            .expect("RecordGenerator::GenerateRecord_Financial, target account missing")
                            .log
                            .add_log(target_log, -1);
                    }
                }
            }

            transferdate.advance_day(NumberGenerator::random_number(14));
            transferdate.advance_hour(NumberGenerator::random_number(24));
            transferdate.advance_minute(NumberGenerator::random_number(60));
        }
    }

    // Access methods for looking up specific records
    // All return the first record found for that name ("PLAYER" is looked up by the player's handle)
    // All return None on failure

    pub fn get_social_security<'a>(g: &'a Ctx, personname: &str) -> Option<&'a Record> {
        get_record_in(g, SOCIAL_SECURITY_DB, personname)
    }

    pub fn get_criminal<'a>(g: &'a Ctx, personname: &str) -> Option<&'a Record> {
        get_record_in(g, CRIMINAL_DB, personname)
    }

    pub fn get_medical<'a>(g: &'a Ctx, personname: &str) -> Option<&'a Record> {
        get_record_in(g, MEDICAL_DB, personname)
    }

    pub fn get_academic<'a>(g: &'a Ctx, personname: &str) -> Option<&'a Record> {
        get_record_in(g, ACADEMIC_DB, personname)
    }

    pub fn get_financial<'a>(g: &'a Ctx, personname: &str) -> Option<&'a BankAccount> {
        let person = g
            .world()
            .get_person(personname)
            .expect("RecordGenerator::GetFinancial, no person");

        // Current account is the first one: "IP ACCNO"
        let fullcurrentaccount = person
            .accounts
            .first()
            .expect("RecordGenerator::GetFinancial, no account");

        // C: sscanf ( fullcurrentaccount, "%s %s", ip, accno );
        let mut parts = fullcurrentaccount.split_whitespace();
        let ip = parts.next().unwrap_or("");
        let accno = parts.next().unwrap_or("");

        let ba = BankAccount::get_account(g.world(), ip, accno)
            .expect("RecordGenerator::GetFinancial, account not found");

        Some(ba)
    }
}
