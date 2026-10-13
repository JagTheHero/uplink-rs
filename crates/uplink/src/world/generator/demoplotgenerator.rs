//! Port of source/world/generator/demoplotgenerator.h + source/world/generator/demoplotgenerator.cpp

use serde::{Deserialize, Serialize};

use crate::ctx::Ctx;
use crate::game::data::data::{
    DEMO_MAXUPLINKRATING, IP_ACADEMICDATABASE, IP_SOCIALSECURITYDATABASE,
    PAYMENT_MISSION_CHANGEDATA_SOCIAL, PAYMENT_MISSION_VARIANCE, TIME_TODEMOGAMEOVER, UPLINKRATING,
};
use crate::world::company::companyuplink::CompanyUplink;
use crate::world::company::mission::Mission;
use crate::world::company::news::News;
use crate::world::computer::computer::COMPUTER_TYPE_INTERNALSERVICESMACHINE;
use crate::world::computer::recordbank::RECORDBANK_NAME;
use crate::world::generator::missiongenerator::{
    MissionGenerator, MISSION_CHANGEDATA, MISSION_DESTROYFILE, MISSION_STEALFILE,
};
use crate::world::generator::namegenerator::NameGenerator;
use crate::world::generator::numbergenerator::NumberGenerator;
use crate::world::generator::worldgenerator::WorldGenerator;
use crate::world::message::Message;
use crate::world::person::Person;
use crate::world::rating::Rating;
use crate::world::scheduler::notificationevent::{
    NotificationEvent, NOTIFICATIONEVENT_TYPE_DEMOGAMEOVER,
};
use crate::world::scheduler::runplotsceneevent::RunPlotSceneEvent;
use crate::world::scheduler::uplinkevent::UplinkEvent;

/// Only one instance occurs (`World::demoplotgenerator`), and only in the demo. Responsible for
/// ensuring there is always something interesting to do in the demo.
///
/// Every method that touches the world takes `g: &mut Ctx` instead of `self` (the generator
/// lives inside the world); it reaches its own state through `g.world_mut().demoplotgenerator`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DemoPlotGenerator {
    pub scene: i32,
    pub arcmissiontarget: String,
}

impl DemoPlotGenerator {
    pub fn new() -> Self {
        DemoPlotGenerator {
            scene: 0,
            arcmissiontarget: String::new(),
        }
    }

    pub fn initialise(&mut self) {}

    /// `UplinkObject::Update`: schedules a DemoGameOver notification once a player at the maximum
    /// demo rating is hacking.
    pub fn update(g: &mut Ctx) {
        let player = g.player();
        if player.connection.trace_in_progress()
            && player.rating.uplinkscore >= UPLINKRATING[DEMO_MAXUPLINKRATING as usize].score
        {
            // The player is performing a hack
            // but has reached the maximum rating.
            // Schedule a DemoGameOver event

            let mut rundate = g.world().date.clone();
            rundate.advance_minute(TIME_TODEMOGAMEOVER);

            let mut ne = NotificationEvent::new();
            ne.set_type(NOTIFICATIONEVENT_TYPE_DEMOGAMEOVER);
            ne.base.set_run_date(&rundate);

            g.world_mut()
                .scheduler
                .schedule_event(UplinkEvent::NotificationEvent(ne));
        }
    }

    // protected helpers

    /// Moves a mission that was just posted to Uplink's BBS to a random spot in the list.
    /// PORT: C++ found the mission in `CompanyUplink::missions` by pointer; `mission` is a snapshot
    /// of the mission just returned by the generator, so locate it by value (employer, description,
    /// fulldetails) in `Uplink`'s mission list.
    pub fn move_new_mission_to_random_location(g: &mut Ctx, mission: &Mission) {
        let cu = uplink_board_mut(g);

        // Remove the mission from the list first
        let mut moved = match board_index_of(cu, mission) {
            Some(i) => cu.missions.remove(i),
            // PORT: the C++ re-inserted the same pointer even when the removal loop found nothing
            None => mission.clone(),
        };

        // Re-insert at a random spot
        let position = 3 + NumberGenerator::random_number(9);
        let prevdate = cu
            .missions
            .get((position - 1) as usize)
            .expect("UplinkAssert(prevmission)")
            .createdate
            .clone();
        moved.set_create_date(&prevdate);
        cu.missions.insert(position as usize, moved);
    }

    /// Counts the missions of `missiontype` available to the player (the first 10 on the BBS,
    /// with `minuplinkrating` <= the player's rating).
    /// PORT: C++ declared the return type `bool` but returns the count; kept as `i32`.
    pub fn num_available_missions(g: &Ctx, missiontype: i32) -> i32 {
        let cu = uplink_board(g);
        let playerrating = g.player().rating.uplinkrating;

        // "available" now means "in the first 10" as well
        let maxmission = cu.missions.len().min(10);

        let mut found = 0;
        for m in &cu.missions[..maxmission] {
            if m.type_ == missiontype && m.minuplinkrating <= playerrating {
                found += 1;
            }
        }
        found
    }

    pub fn generate_file_copy_mission(g: &mut Ctx) -> Mission {
        let mut mission = generate_file_mission(g, MissionGenerator::generate_steal_single_file);
        set_posted_min_rating(g, &mut mission, 2);
        mission
    }

    pub fn generate_file_delete_mission(g: &mut Ctx) -> Mission {
        let mut mission = generate_file_mission(g, MissionGenerator::generate_destroy_single_file);
        set_posted_min_rating(g, &mut mission, 2);
        mission
    }

    pub fn generate_change_academic_mission(g: &mut Ctx) -> Mission {
        let mut mission = loop {
            let employer = WorldGenerator::get_random_company(g);
            if let Some(m) = MissionGenerator::generate_change_data_academic_record(g, &employer) {
                break m;
            }
        };
        set_posted_min_rating(g, &mut mission, 3);
        mission
    }

    pub fn generate_change_social_record_arc(g: &mut Ctx) -> Mission {
        let target = g
            .world()
            .get_computer("International Social Security Database")
            .expect("UplinkAssert(target)");
        let employer = g
            .world()
            .get_company("ARC")
            .expect("UplinkAssert(employer)");
        let employer_name = employer.name.clone();

        //
        // Set up the basic variables of the mission
        //

        let difficulty: i32 = 10;
        let base_payment = difficulty * PAYMENT_MISSION_CHANGEDATA_SOCIAL;
        let mut payment = NumberGenerator::random_normal_number(
            base_payment as f32,
            // PORT: PAYMENT_MISSION_VARIANCE is the C++ double literal 0.3 (data.rs stores it as f32), so
            // the product is formed from the f32 constant's decimal value to match the double math.
            (base_payment as f64 * PAYMENT_MISSION_VARIANCE.to_string().parse::<f64>().unwrap_or(0.3)) as f32,
        ) as i32;

        let minrating: i32 = 3;
        let acceptrating: i32 = 4;

        payment = (payment / 100) * 100; // Rounds payment to the nearest 100

        let personname = "internal@ARC.net"; // Person to send completion email to

        let description = "Modify a protected Social Security document";

        let details = format!(
            "Payment for this job is {} credits.\n\
             This job has been assigned an Uplink difficulty of {}.\n\
             An UplinkRating of {} or above will be sufficient for automatic acceptance.\n\n",
            payment,
            difficulty,
            Rating::get_uplink_rating_string(acceptrating)
        );

        // Find someone who isn't dead
        let person_name = loop {
            // PORT: as in the C++, this keeps looping while no record matches
            if let Some(record) = target
                .recordbank
                .get_random_record("Personal Status ! Deceased")
            {
                let name = record
                    .get_field(RECORDBANK_NAME)
                    .expect("UplinkAssert(personname)");
                if name != g.player().agent().handle {
                    break name.to_string();
                }
            }
        };
        let person = g
            .world()
            .get_person(&person_name)
            .expect("UplinkAssert(person)");
        let person_realname = person.name.clone();

        let newnumber = 10000000 + NumberGenerator::random_number(99999999);
        let newnumber_s = newnumber.to_string();

        let mut fulldetails = String::new();
        fulldetails.push_str(
            "Thank you for replying Agent.\n\
             We have some work for you which you can get started on immediately.\n\
             Your skills with the Academic Database will come in useful here.\n\n\
             Break into the International Social Security Database:\n\
             TARGET COMPUTER DATA :\n",
        );
        fulldetails.push_str(&format!(
            "   LOCATION: {}\n   IP      : {}\n\n\n",
            target.name, target.ip
        ));
        fulldetails.push_str(
            "Below you will find the personal details of our target. We have selected this man \
             from a large candidate list due to his suitability to our aims. We wish to make use of his \
             identity for one of our own undercover agents, and in order for this to go ahead we first need \
             to make some changes to his Social Security Record.\n\n\
             You can help us by making the necessary changes.  All we need you to do is to change his \
             Social Security Number to ",
        );
        fulldetails.push_str(&newnumber_s);
        fulldetails.push_str(".  We will take care of the rest.\n");

        let completion_a = target.ip.clone();
        let completion_b = person_realname.clone();

        fulldetails.push_str(&format!(
            "\nTARGET INDIVIDUAL :\n   NAME    : {}\n\n\n\
             Once you have finished, be sure to let us know as soon as possible.\n\
             Payment for this job is {} credits.\n\
             Send a notice of completion to\n\
             {}\n\
             \n\
             END",
            person.name, payment, personname
        ));

        // PORT: the C++ strcpy'd the target into a static buffer that the completion event reads back
        g.world_mut().demoplotgenerator.arcmissiontarget = person_realname;

        //
        // Generate the mission
        //

        let mut mission = Mission::new();
        mission.set_type(MISSION_CHANGEDATA);
        mission.set_completion(
            Some(completion_a.as_str()),
            Some(completion_b.as_str()),
            Some("Social Security"),
            Some(newnumber_s.as_str()),
            Some(newnumber_s.as_str()),
        );
        mission.set_employer(&employer_name);
        mission.set_contact(personname);
        mission.set_payment(payment, (payment as f64 * 1.1) as i32);
        mission.set_difficulty(difficulty);
        mission.set_min_rating(minrating);
        mission.set_accept_rating(acceptrating);
        mission.set_description(description);
        mission.set_details(&details);
        mission.set_full_details(&fulldetails);
        mission.give_link(&completion_a);

        mission
    }

    // public

    pub fn player_rating_change(g: &mut Ctx) {
        Self::generate_new_mission(g, true);

        // PORT: the C++ "Maximum uplink rating reached" email was commented out in the source
        // (demo only, rating cap), so it is not ported.
    }

    pub fn player_completed_mission(g: &mut Ctx, mission: &Mission) {
        let scene = g.world().demoplotgenerator.scene;

        if mission.type_ == MISSION_CHANGEDATA
            && mission.completiona == IP_ACADEMICDATABASE
            && scene == 0
        {
            //
            // Just finished the academic database hack
            // Send a "we're impressed" msg
            // to the player from ARC
            //

            schedule_demo_scene(g, 1);
        } else if mission.type_ == MISSION_CHANGEDATA
            && mission.completiona == IP_SOCIALSECURITYDATABASE
            && mission.contact == "internal@ARC.net"
        {
            //
            // Just finished the special social security mission from ARC
            // Send congrats
            // Kill of the target of that mission
            //

            Self::run_scene(g, 3);
        }
    }

    /// Occurs every few hours.
    pub fn generate_new_mission(g: &mut Ctx, randomdate: bool) {
        let playerrating = g.player().rating.uplinkrating;

        if playerrating == 2 {
            // Ensure he always has a steal file mission

            let numm1 = Self::num_available_missions(g, MISSION_STEALFILE);
            if numm1 == 0 {
                let m = Self::generate_file_copy_mission(g);
                if randomdate {
                    Self::move_new_mission_to_random_location(g, &m);
                }
            }

            // Ensure he always has a destroy file mission

            let numm2 = Self::num_available_missions(g, MISSION_DESTROYFILE);
            if numm2 == 0 {
                let m = Self::generate_file_delete_mission(g);
                if randomdate {
                    Self::move_new_mission_to_random_location(g, &m);
                }
            }
        } else if playerrating == 3 {
            // Ensure he always has an academic hack

            let numm1 = Self::num_available_missions(g, MISSION_CHANGEDATA);
            if numm1 == 0 {
                let m = Self::generate_change_academic_mission(g);
                if randomdate {
                    Self::move_new_mission_to_random_location(g, &m);
                }
            }
        }
    }

    /// Handles all emails from the player to the main companies. True if the email was handled.
    pub fn player_contacts_arc(g: &mut Ctx, _msg: &Message) -> bool {
        if g.world().demoplotgenerator.scene == 1 {
            // This is a response to the offer from ARC
            Self::run_scene(g, 2);
            return true;
        }

        false
    }

    pub fn run_scene(g: &mut Ctx, newscene: i32) {
        g.world_mut().demoplotgenerator.scene = newscene;

        match newscene {
            1 => {
                // ARC offer a special mission to the Player:
                // Jig this guys social-security record
                //

                let handle = g.player().agent().handle.clone();

                let mut m = Message::new();
                m.set_from("internal@ARC.net");
                m.set_to("PLAYER");
                m.set_subject("Offer from Andromeda Research Corporation");
                m.set_body(&format!(
                    "Agent {},\n\
                     We have been monitoring your progress as a new Uplink Agent, \
                     and we've been extremely impressed with the rate at which you are \
                     advancing through the ranks.\n\n\
                     We have some work which needs completing, and your recent experience \
                     with the International Academic Database makes you particularly suited to the job.\n\
                     The work that we are doing is of a similar nature.\n\n\
                     You will find working for Andromeda Research Corporation is much more profitable \
                     than working for Uplink Corporation. In any case, we need an answer soon.\n\
                     If you are interested, simply reply to this email within the next 24 hours.\n\n\
                     [END]",
                    handle
                ));
                m.send(g);
            }
            2 => {
                // He accepts the offer
                // Send him the mission

                let mission = Self::generate_change_social_record_arc(g);
                Person::give_mission(g, "PLAYER", mission);
            }
            3 => {
                // Player completed the ARC mission

                let mut m = Message::new();
                m.set_from("internal@ARC.net");
                m.set_to("PLAYER");
                m.set_subject("Message from ARC");
                m.set_body(
                    "Congratulations on your recent success with the Social Security Database. \
                     We knew you would be successfull - we would not have selected you otherwise.\n\n\
                     If you check your bank accounts, you will see we have deposited the amount promised.\n\n\
                     We hope you will be willing to work for us in the future. We will contact you soon \
                     with more work, as it becomes available.\n\n\
                     Andromeda Research Corporation",
                );
                m.send(g);

                // The target of that mission dies in one minute

                schedule_demo_scene(g, 4);
            }
            4 => {
                // The target of the ARC mission dies
                // News story

                let agenthandle = NameGenerator::generate_agent_alias();
                let arcmissiontarget = g.world().demoplotgenerator.arcmissiontarget.clone();

                let newsdetails = format!(
                    "An Uplink Agent going by the handle of {} \
                     has been found dead in his house after an apparent suicide.\n\
                     Preliminary forensic analysis suggests he overdosed on sleeping pills.\n\n\
                     The agent's real name is {} and he had been working for the controversial \
                     Andromeda Research Corporation, otherwise known as ARC.  ARC have been in the news in previous months, \
                     responding to allegations that they have been developing some kind of weapon using the expertise of new and naive Uplink Agents.\n\
                     In particular, they have been accused of targeting users of the Demo Uplink Client, \
                     who often have little understanding or knowledge of the companies involved.\n\n\
                     ARC released a statement earlier today, stating 'He was the best programmer we had ever seen and \
                     was a lead player in the project.  For something like this happen is a tradgedy of the greatest order.  \
                     Our thoughts go out to this mans family and friends.'\n\n\
                     Due to the seriousness of these claims, Uplink Corporation has asked all agents not to accept work from ARC while this issue is resolved.",
                    agenthandle, arcmissiontarget
                );

                let mut news = News::new();
                news.set_headline("Uplink agent dies in mysterious circumstances");
                news.set_details(&newsdetails);
                CompanyUplink::create_news(g, news);

                // Warning mail to all

                let mut m = Message::new();
                m.set_from("Uplink Corporation");
                m.set_to("PLAYER");
                m.set_subject("Warning to all Agents");
                m.set_body(
                    "[Sent to ALL-AGENTS]\n\n\
                     We have recently uncovered some alarming evidence regarding the \
                     Andromeda Research Corporation, otherwise known as ARC.  Several of our agents \
                     have made accusations and we are currently investigating.\n\n\
                     In the mean time, we wish to request that Uplink Agents refuse any work offered to them \
                     by ARC until this matter is resolved.\n\n\
                     More details can be found on our News Server, on the Uplink Internal Services System.\n\n\
                     END",
                );
                m.send(g);

                g.world_mut().demoplotgenerator.scene = 5;
            }
            _ => {
                // No more plot for the demo
            }
        }
    }
}

// ============================================================================
// Private helpers (C++ file-level code and repeated blocks)
// ============================================================================

/// The Uplink company's mission board (`CompanyUplink::missions`).
fn uplink_board(g: &Ctx) -> &CompanyUplink {
    let company = g
        .world()
        .get_company("Uplink")
        .expect("UplinkAssert(cu): no Uplink company");
    company
        .uplink
        .as_deref()
        .expect("UplinkAssert(cu): Uplink company has no CompanyUplink data")
}

fn uplink_board_mut(g: &mut Ctx) -> &mut CompanyUplink {
    let company = g
        .world_mut()
        .get_company_mut("Uplink")
        .expect("UplinkAssert(cu): no Uplink company");
    company
        .uplink
        .as_deref_mut()
        .expect("UplinkAssert(cu): Uplink company has no CompanyUplink data")
}

/// The mission generators post the mission they make to the board and return a copy of it; the
/// C++ returned the posted pointer. Finds the board entry the copy stands for (same employer,
/// description, details and completion targets).
fn board_index_of(cu: &CompanyUplink, mission: &Mission) -> Option<usize> {
    cu.missions.iter().position(|m| {
        m.employer == mission.employer
            && m.description == mission.description
            && m.details == mission.details
            && m.fulldetails == mission.fulldetails
            && m.completiona == mission.completiona
            && m.completionb == mission.completionb
    })
}

/// `mission->SetMinRating(rating)` on a mission the generator just posted: the posted board copy
/// changes too, as it did in the C++ (both were the same object there).
fn set_posted_min_rating(g: &mut Ctx, mission: &mut Mission, rating: i32) {
    let cu = uplink_board_mut(g);
    if let Some(i) = board_index_of(cu, mission) {
        cu.missions[i].set_min_rating(rating);
    }
    mission.set_min_rating(rating);
}

/// `strcmp(employer->name, target->companyname) == 0`, with the computer given by its key.
fn same_company_as_computer(g: &Ctx, employer: &str, computer: &str) -> bool {
    let target = g
        .world()
        .get_computer(computer)
        .expect("UplinkAssert(target)");
    target.companyname == employer
}

/// Shared loop of `Generate_FileCopyMission` / `Generate_FileDeleteMission`: picks an employer and
/// a low security internal services computer of a different company, then asks `generate` for the
/// mission until one is made. The caller sets the minimum rating.
fn generate_file_mission(
    g: &mut Ctx,
    generate: fn(&mut Ctx, &str, &str) -> Option<Mission>,
) -> Mission {
    let mut employer: Option<String> = None;
    let mut target: Option<String> = None;

    loop {
        // while (!employer || !target || strcmp(employer->name, target->companyname) == 0) pick again
        loop {
            if let (Some(e), Some(t)) = (&employer, &target) {
                if !same_company_as_computer(g, e, t) {
                    break;
                }
            }
            employer = Some(WorldGenerator::get_random_company(g));
            target = Some(WorldGenerator::get_random_low_security_computer(
                g,
                COMPUTER_TYPE_INTERNALSERVICESMACHINE,
            ));
        }

        if let (Some(e), Some(t)) = (employer.as_deref(), target.as_deref()) {
            if let Some(mission) = generate(g, e, t) {
                return mission;
            }
        }
    }
}

/// C++ `RunPlotSceneEvent` for demo `scene` (act -1), scheduled one game minute from now.
fn schedule_demo_scene(g: &mut Ctx, scene: i32) {
    let mut rpse = RunPlotSceneEvent::new();
    rpse.set_act_and_scene(-1, scene);

    let mut rundate = g.world().date.clone();
    rundate.advance_minute(1);
    rpse.base.set_run_date(&rundate);

    g.world_mut()
        .scheduler
        .schedule_event(UplinkEvent::RunPlotSceneEvent(rpse));
}
