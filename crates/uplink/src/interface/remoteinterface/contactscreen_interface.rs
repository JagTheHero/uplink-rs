//! Port of source/interface/remoteinterface/contactscreen_interface.h + source/interface/remoteinterface/contactscreen_interface.cpp

use uplink_engine::gl;

use crate::app::opengl_interface::{
    border_draw, button_assignbitmaps, button_click, button_highlight, clear_draw, text_draw,
    textbutton_draw,
};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::interface::interfacescreen::{InterfaceScreen, ScreenSlot};
use crate::interface::remoteinterface::remoteinterface::{RemoteInterface, SCREEN_CONTACTSCREEN};
use crate::objref::ScreenRef;
use crate::world::company::mission::{Mission, MISSIONPAYMENT_ALLATSTART, MISSIONPAYMENT_HALFATSTART};
use crate::world::computer::computerscreen::genericscreen::GenericScreen;
use crate::world::generator::numbergenerator::NumberGenerator;
use crate::world::generator::recordgenerator::RecordGenerator;
use crate::world::person::Person;

/// Number of visible text lines in the conference.
pub const NUMLINES: i32 = 14;

pub const CONTACT_TYPE_NONE: i32 = 0;
pub const CONTACT_TYPE_MISSION: i32 = 1;

/// The player's key in the world (`World::get_person("PLAYER")`).
const PLAYER: &str = "PLAYER";

/// `interface->GetRemoteInterface()->GetInterfaceScreen()` slot, for `ScreenSlot::call`.
fn ri_screen_slot(g: &mut Ctx) -> &mut ScreenSlot {
    &mut g.interface_mut().ri.screen
}

/// Runs `f` on the contact screen with the screen taken out of its slot, so that `f` can use
/// `&mut self` and `g` together (C++ `GetInterfaceScreen(SCREEN_CONTACTSCREEN)` + calls on
/// `thisint`). `None` when no contact screen is showing.
fn with_contact_screen<R>(
    g: &mut Ctx,
    f: impl FnOnce(&mut ContactScreenInterface, &mut Ctx) -> R,
) -> Option<R> {
    ScreenSlot::call(g, ri_screen_slot, |s, g| {
        let csi = s.as_any_mut().downcast_mut::<ContactScreenInterface>()?;
        Some(f(csi, g))
    })?
}

/// The Uplink company's bulletin-board missions (C++ `(CompanyUplink*)GetCompany("Uplink")->missions`).
fn uplink_missions(g: &Ctx) -> &Vec<Mission> {
    &g.world()
        .get_company("Uplink")
        .expect("ContactScreenInterface: no Uplink company")
        .uplink
        .as_deref()
        .expect("ContactScreenInterface: Uplink company has no CompanyUplink")
        .missions
}

fn uplink_missions_mut(g: &mut Ctx) -> &mut Vec<Mission> {
    &mut g
        .world_mut()
        .get_company_mut("Uplink")
        .expect("ContactScreenInterface: no Uplink company")
        .uplink
        .as_deref_mut()
        .expect("ContactScreenInterface: Uplink company has no CompanyUplink")
        .missions
}

/// The contact's mission by its index in the Uplink missions list (`UplinkAssert` on the lookup).
fn mission_ref(g: &Ctx, index: i32) -> &Mission {
    uplink_missions(g)
        .get(index as usize)
        .expect("ContactScreenInterface: mission not on the Uplink bulletin board")
}

fn mission_mut(g: &mut Ctx, index: i32) -> &mut Mission {
    uplink_missions_mut(g)
        .get_mut(index as usize)
        .expect("ContactScreenInterface: mission not on the Uplink bulletin board")
}

/// The question index from a question button's name (`"contact_question 3"` -> 3), as the C++
/// `sscanf(button->name, "%s %d", ..)` read it.
fn question_index(name: &str) -> i32 {
    name.rsplit(' ').next().and_then(|s| s.parse().ok()).unwrap_or(0)
}

/// IRC-style conference with an employer (mission negotiation) or other contact.
#[derive(Debug, Default)]
pub struct ContactScreenInterface {
    pub cs: Option<ScreenRef>,
    /// C++ `TYPE`: one of the `CONTACT_TYPE_*` constants.
    pub type_: i32,
    /// 0 is system, 1 is player, 2+ are NPC's.
    pub users: Vec<String>,
    /// Messages NOT YET POSTED.
    pub messagequeue: Vec<String>,
    /// True while a message is "scrolling" on.
    pub waiting: bool,
    /// Number of clickable questions.
    pub numquestions: i32,
    /// `Mission *mission` (TYPE mission): non-owning; stored as the index into the Uplink
    /// company's `missions` list.
    pub mission: Option<i32>,
    pub origionalpayment: i32,
    pub origionalpaymentmethod: i32,
    /// Stops you from asking more than once.
    pub askedpayment: bool,
    /// Stops you from asking more than once.
    pub askedpaymentmethod: bool,
}

impl ContactScreenInterface {
    /// `SetTYPE`
    pub fn set_type(&mut self, new_type: i32) {
        self.type_ = new_type;
    }

    /// `SetMission`: starts a mission negotiation (`mission` = index in the Uplink company's missions).
    pub fn set_mission(&mut self, g: &mut Ctx, mission_index: i32) {
        // UplinkAssert ( TYPE == CONTACT_TYPE_MISSION )
        assert!(self.type_ == CONTACT_TYPE_MISSION);

        self.mission = Some(mission_index);

        {
            let mission = mission_ref(g, mission_index);
            self.origionalpayment = mission.payment;
            self.origionalpaymentmethod = mission.paymentmethod;
        }
        self.askedpayment = false;
        self.askedpaymentmethod = false;

        //
        // Add the contact to the screen
        //

        self.add_user("Anonymous");

        //
        // Will the contact speak to us?
        //

        let numconvictions = {
            let rec = RecordGenerator::get_criminal(g, PLAYER)
                .expect("ContactScreenInterface::SetMission: no criminal record for PLAYER");
            let crimrec = rec
                .get_field("Convictions")
                .expect("ContactScreenInterface::SetMission: no Convictions field");
            crimrec.matches('\n').count()
        };

        self.put_message(0, " ");

        let minuplinkrating = mission_ref(g, mission_index).minuplinkrating;

        if g.player().rating.uplinkrating == minuplinkrating && numconvictions > 0 {
            // If the player is only just good enough,
            // and has a criminal record, the employer
            // will choose someone else

            self.put_message(2, "I'm sorry, but we were looking for someone");
            self.put_message(0, "with a cleaner criminal history.");

            self.add_question(g, "OK, Fine", 7);
        } else {
            self.put_message(2, "Thank you for your time, agent.");
            self.put_message(0, "How can we help?");

            // Add the questions

            self.add_question(g, "Why so much money?", -1);
            self.add_question(g, "Who is the target?", -1);
            self.add_question(g, "How secure is the target?", -1);
            self.add_question(g, "I want more money", -1);
            self.add_question(g, "I want half the money now", -1);
            self.add_question(g, "I want all the money now", -1);
            self.add_question(g, "OK, agreed.", -1);
            self.add_question(g, "No thanks.", -1);
        }
    }

    /// `AddUser`: adds a conference member and posts their arrival (or the system banner).
    pub fn add_user(&mut self, name: &str) {
        self.users.push(name.to_string());

        if name == "System" {
            self.put_message(0, "Secure online communication channel established, Port 80");
            self.put_message(0, "Running SCi V2.6, copyright Frontier Communications Ltd");
            self.put_message(0, "Conference online");
            self.put_message(0, " ");
        } else {
            self.put_message(0, &format!("User '{name}' has entered the conference"));
        }
    }

    /// `PutMessage(userid, message)`: queues a line; userid 0 = plain, otherwise "name : message".
    pub fn put_message(&mut self, userid: i32, message: &str) {
        let fullmessage = if userid == 0 {
            message.to_string()
        } else {
            format!("{} : {}", self.users[userid as usize], message)
        };

        self.messagequeue.push(fullmessage);
    }

    /// `AddQuestion(question, index)`: `index` -1 appends a new question.
    pub fn add_question(&mut self, g: &mut Ctx, question: &str, index: i32) {
        let index = if index == -1 {
            let new_index = self.numquestions;
            self.numquestions += 1;
            new_index
        } else {
            if index > self.numquestions {
                self.numquestions = index + 1;
            }
            index
        };

        let name1 = format!("contact_questionb {index}");
        let name2 = format!("contact_question {index}");

        let x = if index < 5 { 20 } else { 230 };
        let y = if index < 5 {
            300 + index * 20
        } else {
            300 + (index - 5) * 20
        };

        g.ecl
            .register_button(x, y, 16, 16, question, "Click to ask this question", &name1);
        button_assignbitmaps(
            g,
            &name1,
            "menuscreenoption.tif",
            "menuscreenoption_h.tif",
            "menuscreenoption_c.tif",
        );
        g.ecl.register_button_callback(&name1, Self::question_click);

        g.ecl.register_button(
            x + 15,
            y,
            185,
            15,
            question,
            "Click to ask this question",
            &name2,
        );
        g.ecl.register_button_callbacks(
            &name2,
            Some(Self::question_draw),
            Some(Self::question_click),
            Some(button_click),
            Some(button_highlight),
        );
    }

    /// `AskQuestion(index)`: runs the answer for a clicked question.
    pub fn ask_question(&mut self, g: &mut Ctx, index: i32) {
        // UplinkAssert ( thisint->mission )
        let mission_index = self
            .mission
            .expect("ContactScreenInterface::AskQuestion: no mission");

        match self.type_ {
            CONTACT_TYPE_NONE => {}

            CONTACT_TYPE_MISSION => match index {
                0 => {
                    // Why so much money?

                    let answer = mission_ref(g, mission_index)
                        .get_why_so_much_money()
                        .map(str::to_string);
                    match answer {
                        Some(answer) => self.put_message(2, &answer),
                        None => self.put_message(2, "No particular reason - it's a standard mission."),
                    }

                    self.put_message(0, " ");
                }

                1 => {
                    // Who is the target?

                    let answer = mission_ref(g, mission_index)
                        .get_who_is_the_target()
                        .map(str::to_string);
                    match answer {
                        Some(answer) => self.put_message(2, &answer),
                        None => self.put_message(2, "We'd rather not reveal that at this time."),
                    }

                    self.put_message(0, " ");
                }

                2 => {
                    // How secure is the target

                    let answer = mission_ref(g, mission_index)
                        .get_how_secure()
                        .map(str::to_string);
                    match answer {
                        Some(answer) => self.put_message(2, &answer),
                        None => {
                            let difficulty = mission_ref(g, mission_index).difficulty;

                            match difficulty {
                                1 => self.put_message(2, "The security will be non-existent."),
                                2 | 3 => self.put_message(2, "The security will be low."),
                                4 | 5 => self.put_message(2, "There will be some defense in action."),
                                6 | 7 => self.put_message(2, "We expect security to be high."),
                                8 | 9 => self.put_message(2, "Security will be very tight."),
                                10 => self.put_message(2, "Security will be extremely high."),
                                _ => panic!("Unrecognised mission difficulty"), // UplinkAbort
                            }

                            if difficulty > 6 {
                                self.put_message(0, "You will certainly encounter Proxy's or Firewalls.");
                            } else if difficulty > 4 {
                                self.put_message(0, "You may encounter Proxy servers or Firewalls.");
                            }
                        }
                    }

                    self.put_message(0, " ");
                }

                3 => {
                    // I want more money

                    if self.askedpayment {
                        self.put_message(2, "We've already agreed on the amount.");
                    } else {
                        // Chance of more money - 90% if he has rating of (missiondifficulty+3) or more
                        // Decreasing by 30% for each rating below that
                        // no chance if rating <= missiondifficulty

                        let acceptrating = mission_ref(g, mission_index).acceptrating;
                        let ratingdifference = g.player().rating.uplinkrating - (acceptrating + 3);
                        let chance = 90 + ratingdifference * 30;

                        if chance >= 100
                            || (chance > 0 && NumberGenerator::random_number(100) <= chance)
                        {
                            let (maxpayment, payment) = {
                                let mission = mission_ref(g, mission_index);
                                (mission.maxpayment, mission.payment)
                            };

                            if maxpayment == 0 || payment == maxpayment {
                                self.put_message(2, "I'm afraid we can't offer you any more money.");
                            } else {
                                self.put_message(
                                    2,
                                    &format!(
                                        "OK, we'll give you {maxpayment}c because of your reputation."
                                    ),
                                );

                                mission_mut(g, mission_index).payment = maxpayment;
                            }
                        } else {
                            self.put_message(2, "I'm afraid we can't offer you any more money.");
                        }

                        self.askedpayment = true;
                    }

                    self.put_message(0, " ");
                }

                4 => {
                    // I want half the money now

                    if self.askedpaymentmethod {
                        self.put_message(2, "We've already agreed on payment.");
                    } else {
                        // Chance of half payment : 100% at rating 7,
                        // decreasing by 10% for each rating

                        let ratingdifference = g.player().rating.uplinkrating - 7;
                        let chance = 100 + ratingdifference * 10;

                        if chance >= 100
                            || (chance > 0 && NumberGenerator::random_number(100) <= chance)
                        {
                            self.put_message(2, "OK, agreed.");
                            mission_mut(g, mission_index).paymentmethod = MISSIONPAYMENT_HALFATSTART;
                        } else {
                            self.put_message(2, "I'm afraid I can't agree to that.");
                        }

                        self.askedpaymentmethod = true;
                    }

                    self.put_message(0, " ");
                }

                5 => {
                    // I want all the money now

                    if self.askedpaymentmethod {
                        self.put_message(2, "We've already agreed on payment.");
                    } else {
                        // Chance of full payment : 100% at rating 10,
                        // decreasing by 10% for each rating

                        let ratingdifference = g.player().rating.uplinkrating - 10;
                        let chance = 100 + ratingdifference * 10;

                        if chance >= 100
                            || (chance > 0 && NumberGenerator::random_number(100) <= chance)
                        {
                            self.put_message(2, "OK, since you have such an excellent record.");
                            mission_mut(g, mission_index).paymentmethod = MISSIONPAYMENT_ALLATSTART;
                        } else {
                            self.put_message(2, "No way - do you think i'm stupid?");
                        }

                        self.askedpaymentmethod = true;
                    }

                    self.put_message(0, " ");
                }

                6 => {
                    // OK, agreed

                    // Give the mission to the player
                    // (C++ GiveMission(thisint->mission), then FindData + RemoveData on the
                    // bulletin board; the list index stands in for the mission pointer here)

                    let mission = uplink_missions(g)
                        .get(mission_index as usize)
                        .expect("ContactScreenInterface::AskQuestion: mission not on the Uplink bulletin board")
                        .clone();
                    Person::give_mission(g, PLAYER, mission);

                    // Remove the mission from the bulletin board

                    uplink_missions_mut(g).remove(mission_index as usize);

                    // Run the bulletin board

                    let nextpage = self
                        .get_computer_screen(g)
                        .expect("ContactScreenInterface::AskQuestion: no computer screen")
                        .nextpage;
                    RemoteInterface::run_screen(g, nextpage);
                }

                7 => {
                    // No thanks

                    // Reset the mission
                    let (origionalpayment, origionalpaymentmethod) =
                        (self.origionalpayment, self.origionalpaymentmethod);
                    let mission = mission_mut(g, mission_index);
                    mission.payment = origionalpayment;
                    mission.paymentmethod = origionalpaymentmethod;

                    let nextpage = self
                        .get_computer_screen(g)
                        .expect("ContactScreenInterface::AskQuestion: no computer screen")
                        .nextpage;
                    RemoteInterface::run_screen(g, nextpage);
                }

                _ => panic!("Unrecognised question index"), // UplinkAbort
            },

            _ => panic!("Unrecognised contact TYPE"), // UplinkAbort
        }
    }

    /// `WaitingCallback` (static, caption-change callback)
    pub fn waiting_callback(g: &mut Ctx) {
        with_contact_screen(g, |csi, _g| {
            csi.waiting = false;
        });
    }

    /// `BorderDraw` (static DrawFn)
    pub fn border_draw(_g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        gl::color3f(1.0, 1.0, 1.0);
        border_draw(button);
    }

    /// `MessageDraw` (static DrawFn)
    pub fn message_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        gl::color3f(0.7, 0.7, 0.7);
        clear_draw(g, button.x, button.y, button.width, button.height);
        text_draw(g, button, highlighted, clicked);
    }

    /// `PostClick` (static MouseFn)
    pub fn post_click(g: &mut Ctx, _button: &Button) {
        with_contact_screen(g, |csi, g| csi.post_click_inner(g));
    }

    /// `PostClick` body, for callers that have no button (C++ `PostClick(NULL)` from
    /// `ReturnKeyPressed`).
    fn post_click_inner(&mut self, g: &mut Ctx) {
        // UplinkAssert ( b ). The "contact_typehere" button is only registered in the block that
        // is commented out in C++ Create(), so this assert fails there too (C++ exits).
        let caption = g
            .ecl
            .get_button("contact_typehere")
            .expect("ContactScreenInterface::PostClick: contact_typehere button missing")
            .caption
            .clone();

        self.put_message(1, &caption);

        g.ecl
            .get_button_mut("contact_typehere")
            .expect("ContactScreenInterface::PostClick: contact_typehere button missing")
            .set_caption(" ");
        g.ecl.dirty_button("contact_typehere");
    }

    /// `TypeHereDraw` (static DrawFn)
    pub fn type_here_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        textbutton_draw(g, button, highlighted, clicked);
    }

    /// `QuestionDraw` (static DrawFn)
    pub fn question_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        textbutton_draw(g, button, highlighted, clicked);
    }

    /// `QuestionClick` (static MouseFn)
    pub fn question_click(g: &mut Ctx, button: &Button) {
        // Find the question index
        let index = question_index(&button.name);

        // Post this message on the board, then trigger a response
        let caption = button.caption.clone();
        with_contact_screen(g, |csi, g| {
            csi.put_message(1, &caption);
            csi.ask_question(g, index);
        });
    }

    /// `GetComputerScreen` (typed accessor; the C++ casts `cs`).
    pub fn get_computer_screen<'a>(&self, g: &'a Ctx) -> Option<&'a GenericScreen> {
        let cs = self.cs.as_ref()?;
        g.world()
            .get_computer(&cs.computer)?
            .get_computer_screen(cs.index)?
            .as_generic_screen()
    }
}

impl InterfaceScreen for ContactScreenInterface {
    crate::impl_as_any!();

    /// `Create(ComputerScreen *newcs)` body (the `cs` is already set by `set_computer_screen`).
    fn create(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            return;
        }

        g.ecl
            .register_button(20, 30, 405, 15, "Secure Communication Channel open", "", "contact_title");

        g.ecl
            .register_button(20, 45, 405, NUMLINES * 15 + 10, "", "", "contact_border");
        g.ecl.register_button_callbacks(
            "contact_border",
            Some(Self::border_draw),
            None,
            None,
            None,
        );

        for i in 0..NUMLINES {
            let y = 50 + (i * 15);
            let name = format!("contact_text {i}");

            g.ecl.register_button(22, y, 400, 15, "", "", &name);
            g.ecl.register_button_callbacks(
                &name,
                Some(Self::message_draw),
                None,
                None,
                None,
            );
        }

        // C++ computes `ybottom` here for the "contact_typehere" and "contact_post" buttons,
        // which are inside a comment block in C++ and are not created.

        // Add in the users

        self.add_user("System");
        let handle = g.player().agent().handle.clone();
        self.add_user(&handle);
    }

    fn remove(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            return;
        }

        g.ecl.remove_button("contact_title");
        g.ecl.remove_button("contact_border");

        for ti in 0..NUMLINES {
            g.ecl.remove_button(&format!("contact_text {ti}"));
        }

        // C++ "contact_typehere" / "contact_post" removal is in a comment block, as in Create().

        // Remove question buttons

        for qi in 0..self.numquestions {
            g.ecl.remove_button(&format!("contact_question {qi}"));
            g.ecl.remove_button(&format!("contact_questionb {qi}"));
        }
    }

    fn update(&mut self, g: &mut Ctx) {
        if self.waiting || self.messagequeue.is_empty() {
            return;
        }

        // Scroll the messages up

        for i in 0..NUMLINES - 1 {
            let name1 = format!("contact_text {i}");
            let name2 = format!("contact_text {}", i + 1);

            let b2caption = g
                .ecl
                .get_button(&name2)
                .expect("ContactScreenInterface::Update: missing contact_text button")
                .caption
                .clone();

            g.ecl
                .get_button_mut(&name1)
                .expect("ContactScreenInterface::Update: missing contact_text button")
                .set_caption(&b2caption);

            g.ecl.dirty_button(&name1);
        }

        // Set the message on the last button and remove the message from the queue

        let message = self.messagequeue.remove(0);
        let time = message.len() as i32 * 5;

        let name1 = format!("contact_text {}", NUMLINES - 1);

        g.ecl
            .register_caption_change(&name1, &message, time, Some(Self::waiting_callback));
        self.waiting = true;
    }

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("contact_title").is_some()
    }

    fn return_key_pressed(&mut self, g: &mut Ctx) -> bool {
        self.post_click_inner(g);
        true
    }

    fn screen_id(&self) -> i32 {
        SCREEN_CONTACTSCREEN
    }

    fn set_computer_screen(&mut self, cs: ScreenRef) {
        self.cs = Some(cs);
    }

    fn computer_screen_ref(&self) -> Option<&ScreenRef> {
        self.cs.as_ref()
    }
}
