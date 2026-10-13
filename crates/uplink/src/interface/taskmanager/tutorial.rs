//! Port of source/interface/taskmanager/tutorial.h + source/interface/taskmanager/tutorial.cpp

use uplink_engine::gl;

use crate::app::opengl_interface::{
    border_draw, button_assignbitmaps, button_click, button_draw, button_highlight, text_draw,
};
use crate::ctx::Ctx;
use crate::eclipse::{Button, MOVE_STRAIGHTLINE};
use crate::game::data::data::{IP_INTERNIC, IP_UPLINKINTERNALSERVICES, IP_UPLINKPUBLICACCESSSERVER, IP_UPLINKTESTMACHINE};
use crate::impl_as_any;
use crate::interface::localinterface::localinterface::{
    SCREEN_EMAIL, SCREEN_FINANCE, SCREEN_HW, SCREEN_MEMORY, SCREEN_MISSION, SCREEN_NONE, SCREEN_SENDMAIL, SCREEN_STATUS,
};
use crate::interface::localinterface::sendmail_interface::SendMailInterface;
use crate::interface::localinterface::sw_interface::SWInterface;
use crate::interface::localinterface::worldmap::worldmap_interface::{WorldMapInterface, WORLDMAP_LARGE, WORLDMAP_SMALL};
use crate::interface::remoteinterface::remoteinterface::{
    RemoteInterface, SCREEN_BBSSCREEN, SCREEN_CHANGEGATEWAY, SCREEN_FILESERVERSCREEN, SCREEN_SWSALESSCREEN,
};
use crate::interface::taskmanager::filecopier::{FileCopier, FILECOPIER_INPROGRESS, FILECOPIER_WAITINGFORTARGET};
use crate::interface::taskmanager::taskmanager::TaskManager;
use crate::interface::taskmanager::uplinktask::{UplinkTask, UplinkTaskBase};
use crate::vanbakel::Vanbakel;
use crate::world::computer::computerscreen::computerscreen::ComputerScreen;

/// Tutorial program: sits on screen and tells you what to do every now and again.
#[derive(Debug, Default)]
pub struct Tutorial {
    pub base: UplinkTaskBase,
    pub current_tutorial: i32,
    pub current_part: i32,
    /// True if the NEXT button should be visible.
    pub nextvisible: bool,
    /// True if the NEXT button has been clicked.
    pub nextclicked: bool,
    /// True if we're in the selection menu.
    pub menu: bool,
}

/// Button names of the main window: `sprintf(title, "tutorial_title %d", pid)` and friends.
struct Names {
    title: String,
    skip: String,
    close: String,
    text: String,
    next: String,
}

fn names(pid: i32) -> Names {
    Names {
        title: format!("tutorial_title {pid}"),
        skip: format!("tutorial_skip {pid}"),
        close: format!("tutorial_close {pid}"),
        text: format!("tutorial_text {pid}"),
        next: format!("tutorial_next {pid}"),
    }
}

/// Button names of the selection menu.
struct MenuNames {
    part1: String,
    part2: String,
    part3: String,
    part1button: String,
    part2button: String,
    part3button: String,
}

fn menu_names(pid: i32) -> MenuNames {
    MenuNames {
        part1: format!("tutorial_part1 1 {pid}"),
        part2: format!("tutorial_part2 2 {pid}"),
        part3: format!("tutorial_part3 3 {pid}"),
        part1button: format!("tutorial_part1button 1 {pid}"),
        part2button: format!("tutorial_part2button 2 {pid}"),
        part3button: format!("tutorial_part3button 3 {pid}"),
    }
}

/// `sscanf(button->name, "%s %d", name, &pid)`: the PID is the number after the button's name.
fn button_pid(button: &Button) -> i32 {
    button.name.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(-1)
}

/// `(Tutorial *) SvbGetTask(pid)` followed by a method call on it. `None` if there is no such
/// task, it is not a Tutorial, or it is running right now.
fn with_tutorial<R>(g: &mut Ctx, pid: i32, f: impl FnOnce(&mut Tutorial, &mut Ctx) -> R) -> Option<R> {
    Vanbakel::with_task(g, pid, |t, g| t.as_any_mut().downcast_mut::<Tutorial>().map(|tut| f(tut, g))).flatten()
}

/// `game->GetWorld()->GetPlayer()->GetRemoteHost()->ip` ("" where the C++ would have crashed).
fn remote_ip(g: &Ctx) -> &str {
    g.player().get_remote_host(g.world()).map_or("", |h| h.ip.as_str())
}

/// `game->GetInterface()->GetRemoteInterface()->GetInterfaceScreen()->ScreenID()`
fn remote_screen_id(g: &Ctx) -> i32 {
    g.interface().ri.screen.screen_id()
}

impl Tutorial {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run_menu(&mut self, g: &mut Ctx) {
        //
        // Change the state of the existing system
        //

        self.menu = true;
        self.nextvisible = false;
        self.current_tutorial = 0;
        self.current_part = 0;

        let pid = self.base.pid;
        let text = format!("tutorial_text {pid}");

        g.ecl.register_caption_change_auto(&text, " ", None);

        g.ecl.super_unhighlight_all();

        //
        // Create the menu
        //

        self.create_menu_interface(g);
    }

    pub fn run_tutorial(&mut self, g: &mut Ctx, tutorial: i32, part: i32) {
        self.current_tutorial = tutorial;
        self.current_part = part;

        if tutorial == 1 {
            self.run_tutorial1(g, part);
        } else if tutorial == 2 {
            self.run_tutorial2(g, part);
        } else if tutorial == 3 {
            self.run_tutorial3(g, part);
        } else {
            let pid = self.base.pid;
            let text = format!("tutorial_text {pid}");

            g.ecl.register_caption_change_auto(&text, "[To be completed]", None);
            println!("Tutorial::RunTutorial, unrecognised tutorial number {tutorial}");
        }
    }

    pub fn run_tutorial1(&mut self, g: &mut Ctx, part: i32) {
        let pid = self.base.pid;
        let text = format!("tutorial_text {pid}");
        let time_ms = 3000;

        if part == 1 {
            // Tut 1 Part 1
            g.ecl.register_caption_change(
                &text,
                "Welcome to the Uplink Tutorial.\n\
                 This program should help you to get started.\n\
                 Click next to continue.",
                time_ms,
                None,
            );

            g.ecl.super_highlight(&text);
            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 2 {
            // Tut 1 Part 2
            g.ecl.register_caption_change(
                &text,
                "First things first - you can move this tutorial\n\
                 on the screen by left-clicking on the title bar.\n\
                 Right click to put it down again.",
                time_ms,
                None,
            );

            g.ecl.super_unhighlight(&text);
            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 3 {
            // Tut 1 Part 3
            g.ecl.register_caption_change(
                &text,
                "We're going to start by looking at some of the\n\
                 facilities available on your screen.\n\
                 Click on the flashing HARDWARE button.\n",
                time_ms,
                None,
            );

            g.ecl.super_highlight("hud_hardware");
            self.nextvisible = false;
        } else if part == 4 {
            // Tut 1 Part 4
            g.ecl.register_caption_change(
                &text,
                "This screen shows the status of your GATEWAY.\n\
                 Your gateway is the link between your home PC\n\
                 and the rest of the Net.",
                time_ms,
                None,
            );

            g.ecl.super_unhighlight("hud_hardware");
            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 5 {
            // Tut 1 Part 5
            g.ecl.register_caption_change(
                &text,
                "In reality, the gateway is a computer sitting in\n\
                 one of our offices.  You'll dial into this Gateway\n\
                 automatically every time you start Uplink.",
                time_ms,
                None,
            );

            self.nextclicked = false;
        } else if part == 6 {
            // Tut 1 Part 6
            g.ecl.register_caption_change(
                &text,
                "At the moment your Gateway is very basic.\n\
                 It has a slow processor and very little memory.\n\
                 You'll be able to upgrade later.",
                time_ms,
                None,
            );

            self.nextclicked = false;
        } else if part == 7 {
            // Tut 1 Part 7
            g.ecl.register_caption_change(&text, "Click on the flashing MEMORY BANKS button.", time_ms / 3, None);

            g.ecl.super_unhighlight(&text);
            g.ecl.super_highlight("hud_memory");
            self.nextvisible = false;
        } else if part == 8 {
            // Tut 1 Part 8
            g.ecl.register_caption_change(
                &text,
                "This screen shows all of the files you have stored\n\
                 in your memory banks.  At the moment you only have\n\
                 this tutorial and a couple of file utilites.",
                time_ms,
                None,
            );

            g.ecl.super_unhighlight("hud_memory");
            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 9 {
            // Tut 1 Part 9
            g.ecl.register_caption_change(&text, "Click on the flashing PERSONAL STATUS button.", time_ms / 3, None);

            g.ecl.super_highlight("hud_status");
            self.nextvisible = false;
        } else if part == 10 {
            // Tut 1 Part 10
            g.ecl.register_caption_change(
                &text,
                "This screen shows your official ratings.\n\
                 These reflect your skill and success, and will\n\
                 no doubt increase with time.",
                time_ms,
                None,
            );

            g.ecl.super_unhighlight("hud_status");
            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 11 {
            // Tut 1 Part 11
            g.ecl.register_caption_change(
                &text,
                "As your rating increases, you'll find more\n\
                 interesting and profitable avenues of work\n\
                 become available to you.",
                time_ms,
                None,
            );

            self.nextclicked = false;
        } else if part == 12 {
            // Tut 1 Part 12
            g.ecl.register_caption_change(&text, "Click on the flashing FINANCE button.", time_ms / 3, None);

            g.ecl.super_highlight("hud_finance");
            self.nextvisible = false;
        } else if part == 13 {
            // Tut 1 Part 13
            g.ecl.register_caption_change(
                &text,
                "Here you can see how much money you have\n\
                 in your bank accounts.\n",
                time_ms * 2 / 3,
                None,
            );

            g.ecl.super_unhighlight("hud_finance");

            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 14 {
            // Tut 1 Part 14
            g.ecl.register_caption_change(
                &text,
                "The final button allows you to compose a\n\
                 new email and send it.  We don't need to\n\
                 do this at the moment.",
                time_ms,
                None,
            );

            g.ecl.super_highlight("hud_sendmail");

            self.nextclicked = false;
        } else if part == 15 {
            // Tut 1 Part 15
            g.ecl.register_caption_change(
                &text,
                "The big button to the left allows you to\n\
                 run the software tools on your Gateway.\n\
                 You can start any program from here.",
                time_ms,
                None,
            );

            g.ecl.super_unhighlight("hud_sendmail");
            g.ecl.super_highlight("hud_software");

            self.nextclicked = false;
        } else if part == 16 {
            // Tut 1 Part 16
            g.ecl.register_caption_change(
                &text,
                "These two boxes show the date and time,\n\
                 and your current location on the Net.",
                time_ms * 2 / 3,
                None,
            );

            g.ecl.super_unhighlight("hud_software");
            g.ecl.super_highlight("hud_date");
            g.ecl.super_highlight("hud_location");

            self.nextclicked = false;
        } else if part == 17 {
            // Tut 1 Part 17
            g.ecl.register_caption_change(
                &text,
                "These buttons can be used to accelerate time.\n\
                 This can be useful when you're waiting\n\
                 for something to happen.",
                time_ms,
                None,
            );

            g.ecl.super_unhighlight("hud_date");
            g.ecl.super_unhighlight("hud_location");

            g.ecl.super_highlight("hud_speed 0");
            g.ecl.super_highlight("hud_speed 1");
            g.ecl.super_highlight("hud_speed 2");
            g.ecl.super_highlight("hud_speed 3");

            self.nextclicked = false;
        } else if part == 18 {
            // Tut 1 Part 18
            g.ecl.register_caption_change(
                &text,
                "This column shows all currently running programs,\n\
                 and how much CPU time is used by each.\n\
                 You can see this tutorial program in the list.",
                time_ms,
                None,
            );

            g.ecl.super_unhighlight("hud_speed 0");
            g.ecl.super_unhighlight("hud_speed 1");
            g.ecl.super_unhighlight("hud_speed 2");
            g.ecl.super_unhighlight("hud_speed 3");

            g.ecl.super_highlight("svb_priority_mask 0");

            self.nextclicked = false;
        } else if part == 19 {
            // Tut 1 Part 19
            g.ecl.register_caption_change(
                &text,
                "Bring up the Global Communications screen\n\
                 by clicking on the world map.",
                time_ms * 2 / 3,
                None,
            );

            g.ecl.super_unhighlight("svb_priority_mask 0");
            g.ecl.super_highlight("worldmap_smallmap");

            self.nextvisible = false;
        } else if part == 20 {
            // Tut 1 Part 20
            g.ecl.register_caption_change(
                &text,
                "This screen allows you to set up long distance\n\
                 connections that bounce around the world.\n\
                 You'll make extensive use of this screen.",
                time_ms,
                None,
            );

            g.ecl.super_unhighlight("worldmap_smallmap");

            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 21 {
            // Tut 1 Part 21
            g.ecl.register_caption_change(
                &text,
                "We'll come back here later.\n\
                 Click the close button to return\n\
                 to the main screen.",
                time_ms,
                None,
            );

            g.ecl.super_highlight("worldmap_close");

            self.nextvisible = false;
        } else if part == 22 {
            // Tut 1 Part 22

            // TODO : Check the player has an email

            g.ecl.register_caption_change(
                &text,
                "New emails will queue up in the bottom right\n\
                 of the screen.  Click on the email to read it.\n",
                time_ms * 2 / 3,
                None,
            );

            g.ecl.super_unhighlight("worldmap_close");
            g.ecl.super_highlight("hud_message 0");
        } else if part == 23 {
            // Tut 1 Part 23

            // TODO : Check the player has an email from Uplink Corporation

            g.ecl.register_caption_change(
                &text,
                "This is an email from Uplink Corporation.\n\
                 Its basically a welcome message.  You should take\n\
                 the time to read it at some point.",
                time_ms,
                None,
            );

            g.ecl.super_unhighlight("hud_message 0");

            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 24 {
            // Tut 1 Part 24
            g.ecl.register_caption_change(
                &text,
                "As well as mail, you'll also get missions\n\
                 queuing up at the bottom right of the screen.\n\
                 They'll remain here until you complete them.",
                time_ms,
                None,
            );

            self.nextclicked = false;
        } else if part == 25 {
            // Tut 1 Part 25

            // TODO : Check the player has this mission from uplink

            g.ecl.register_caption_change(
                &text,
                "The only mission you have at the moment is a\n\
                 test mission from Uplink.  Click on it to read\n\
                 the details.",
                time_ms,
                None,
            );

            self.nextvisible = false;
            g.ecl.super_highlight("hud_mission 0");
        } else if part == 26 {
            // Tut 1 Part 26
            g.ecl.register_caption_change(
                &text,
                "This screen shows you all the information you need\n\
                 on how to complete the mission.  You should always\n\
                 study this in depth before beginning work.",
                time_ms,
                None,
            );

            g.ecl.super_unhighlight("hud_mission 0");
            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 27 {
            // Tut 1 Part 27
            g.ecl.register_caption_change(
                &text,
                "You'll learn how to complete missions like this in\n\
                 part 2 of the tutorial.  For now, click on the\n\
                 close button.",
                time_ms,
                None,
            );

            self.nextvisible = false;
            g.ecl.super_highlight("mission_close");
        } else if part == 28 {
            // Tut 1 Part 28
            g.ecl.register_caption_change(
                &text,
                "That is the end of Tutorial One.  You should have\n\
                 an overview by now of the basic functions\n\
                 on screen. Click next to continue.",
                time_ms,
                None,
            );

            g.ecl.super_unhighlight("mission_close");
            g.ecl.super_highlight(&text);
            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 29 {
            g.ecl.super_unhighlight(&text);
            self.run_menu(g);
        }
    }

    pub fn run_tutorial2(&mut self, g: &mut Ctx, part: i32) {
        let pid = self.base.pid;
        let text = format!("tutorial_text {pid}");
        let time_ms = 3000;

        if part == 1 {
            g.ecl.register_caption_change(
                &text,
                "Welcome to Part 2 of the tutorial.\n\
                 We will now show you how to complete your\n\
                 first hack for Uplink.",
                time_ms,
                None,
            );

            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 2 {
            // TODO : Check the player has this mission from uplink

            g.ecl.register_caption_change(
                &text,
                "Your first hack will be the Uplink test mission.\n\
                 Click on the mission description to read the\n\
                 details.",
                time_ms,
                None,
            );

            self.nextvisible = false;
            self.nextclicked = false;
            g.ecl.super_highlight("hud_mission 0");
        } else if part == 3 {
            g.ecl.register_caption_change(
                &text,
                "Your should read the description carefully, but\n\
                 the basic aim is to break into the Uplink Test\n\
                 Machine and steal a file.",
                time_ms,
                None,
            );

            g.ecl.super_unhighlight("hud_mission 0");
            self.nextvisible = true;
            self.nextclicked = false;

            // - Caution -
            // Can jump back to this part from part 6
        } else if part == 4 {
            // Make sure the player isn't connected anywhere

            if g.player().is_connected() {
                g.ecl.register_caption_change(&text, "Disconnect from your current site.", time_ms / 3, None);

                g.ecl.super_highlight("worldmap_connect");
                self.nextvisible = false;
                self.nextclicked = false;
            } else {
                // Completion condition is "disconnected"
                // So next part will play immediately
            }
        } else if part == 5 {
            g.ecl.register_caption_change(
                &text,
                "We're going to start by connecting to the target.\n\
                 Select 'Uplink Test Machine' from the list\n\
                 of bookmarked links on screen.",
                time_ms,
                None,
            );

            g.ecl.super_unhighlight("worldmap_connect");
            self.nextvisible = false;
            self.nextclicked = false;
        } else if part == 6 {
            // We are connected somewhere, but are we at the right spot?

            if remote_ip(g) == IP_UPLINKTESTMACHINE {
                g.ecl.register_caption_change(
                    &text,
                    "We're now dialled in to the Test Machine.\n\
                     You should read the opening text message\n\
                     before clicking next.",
                    time_ms,
                    None,
                );

                self.nextvisible = true;
                self.nextclicked = false;
            } else {
                g.ecl.register_caption_change(
                    &text,
                    "You've connected to the wrong site.\n\
                     The one we want is the 'Uplink Test Machine'.\n\
                     Click next to try again.",
                    time_ms,
                    None,
                );

                self.nextvisible = true;
                self.nextclicked = false;
                self.current_part = 3; // Jump back to just before part 4
            }
        } else if part == 7 {
            g.ecl.register_caption_change(
                &text,
                "Click on the 'Start the test' button\n\
                 to bypass this screen.\n",
                time_ms * 2 / 3,
                None,
            );

            self.nextvisible = false;
            self.nextclicked = false;

            g.ecl.super_highlight("messagescreen_click 1");
        } else if part == 8 {
            // UserID screen

            g.ecl.register_caption_change(
                &text,
                "This is a User ID screen.  You will need\n\
                 a username and password to get past,\n\
                 which we don't have.",
                time_ms,
                None,
            );

            g.ecl.super_unhighlight("messagescreen_click 1");

            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 9 {
            g.ecl.register_caption_change(
                &text,
                "In reality, you will almost never have a\n\
                 genuine username and password.\n\
                 You'll have to hack your way in.",
                time_ms,
                None,
            );

            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 10 {
            g.ecl.register_caption_change(
                &text,
                "In order to hack our way in, we'll need to\n\
                 buy some software.\n\
                 Disconnect from this machine.",
                time_ms,
                None,
            );

            self.nextvisible = false;
            self.nextclicked = false;

            g.ecl.super_highlight("worldmap_connect");
        } else if part == 11 {
            g.ecl.register_caption_change(
                &text,
                "The best place to buy software for hacking\n\
                 Is the 'Uplink Internal Services System'.\n\
                 Connect to that site.",
                time_ms,
                None,
            );

            g.ecl.super_unhighlight("worldmap_connect");
        } else if part == 12 {
            // Uplink Internal Services Machine

            g.ecl.register_caption_change(
                &text,
                "Unlike most sites, you do have a valid\n\
                 Username and password because you are a\n\
                 registered Uplink Agent.",
                time_ms,
                None,
            );

            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 13 {
            g.ecl.register_caption_change(
                &text,
                "You can either type your username/password\n\
                 or just click on them to enter automatically.\n\
                 Do this now then click on proceed.",
                time_ms,
                None,
            );

            self.nextvisible = false;
            self.nextclicked = false;
        } else if part == 14 {
            g.ecl.register_caption_change(
                &text,
                "We're in.  Your authorisation has been accepted\n\
                 and your actions are being monitored by Uplink's\n\
                 central computer system.",
                time_ms,
                None,
            );

            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 15 {
            g.ecl.register_caption_change(
                &text,
                "This computer system provides a number of\n\
                 valuable services to the hacker community,\n\
                 including software tools which we need.",
                time_ms,
                None,
            );

            self.nextclicked = false;
        } else if part == 16 {
            g.ecl.register_caption_change(
                &text,
                "You can access Uplink-related news stories,\n\
                 accept work from contractors, and upgrade your\n\
                 computer system.",
                time_ms,
                None,
            );

            self.nextclicked = false;
        } else if part == 17 {
            g.ecl.register_caption_change(
                &text,
                "We want to upgrade our software so we can\n\
                 hack UserID screens.  Click on 'Software\n\
                 Upgrades' from the menu.",
                time_ms,
                None,
            );

            self.nextvisible = false;
            self.nextclicked = false;
        } else if part == 18 {
            g.ecl.register_caption_change(
                &text,
                "This screen lists all of the software tools\n\
                 that Uplink currently has for sale.  Items\n\
                 in grey are out of your price range.",
                time_ms,
                None,
            );

            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 19 {
            g.ecl.register_caption_change(
                &text,
                "The first tool you will need is one of the\n\
                 basic Uplink Agent tools - \n\
                 the Password Breaker.",
                time_ms,
                None,
            );

            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 20 {
            g.ecl.register_caption_change(
                &text,
                "Scroll down the list and select the Password\n\
                 Breaker.  The current price for this tool is\n\
                 1500 credits.  Click 'purchase'.",
                time_ms,
                None,
            );

            self.nextvisible = false;
            self.nextclicked = false;
        } else if part == 21 {
            g.ecl.register_caption_change(
                &text,
                "That software tool has now been uploaded into\n\
                 your memory banks, and 1500c has been deducted.\n\
                 We can now try breaking that password.",
                time_ms,
                None,
            );

            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 22 {
            g.ecl.register_caption_change(
                &text,
                "Disconnect from here and connect to the Uplink\n\
                 Test machine.  Bypass the opening text screen.",
                time_ms,
                None,
            );

            self.nextvisible = false;
            self.nextclicked = false;
        } else if part == 23 {
            g.ecl.register_caption_change(
                &text,
                "We want to run our Password Breaker software\n\
                 on this screen.  Our new software will appear\n\
                 in our Software menu.",
                time_ms,
                None,
            );

            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 24 {
            g.ecl.register_caption_change(&text, "Click on the software Run button.", time_ms / 3, None);

            g.ecl.super_highlight("hud_software");

            self.nextvisible = false;
            self.nextclicked = false;
        } else if part == 25 {
            g.ecl.register_caption_change(
                &text,
                "Start your new Password Breaker by selecting it\n\
                 from the menu.  It will be under the 'Crackers'\n\
                 sub-menu.",
                time_ms,
                None,
            );

            g.ecl.super_unhighlight("hud_software");
        } else if part == 26 {
            g.ecl.register_caption_change(
                &text,
                "The Password Breaker is now running.\n\
                 We now need to give it a target.",
                time_ms,
                None,
            );

            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 27 {
            g.ecl.register_caption_change(
                &text,
                "Click on the one of the text boxes\n\
                 labelled NAME or CODE.  This will start\n\
                 the hack attempt.",
                time_ms,
                None,
            );

            self.nextvisible = false;
        } else if part == 28 {
            g.ecl.register_caption_change(
                &text,
                "Oh dear. You've been disconnected by the\n\
                 computer you were trying to hack in to.\n",
                time_ms,
                None,
            );

            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 29 {
            g.ecl.register_caption_change(
                &text,
                "They noticed the fact that you were trying\n\
                 to hack their password screen, and kicked you\n\
                 off their system.",
                time_ms,
                None,
            );

            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 30 {
            g.ecl.register_caption_change(
                &text,
                "Because this is a TEST machine, you won't be\n\
                 punished in any way.  In the real world, you\n\
                 could now face a fine or even jail time.",
                time_ms,
                None,
            );

            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 31 {
            g.ecl.register_caption_change(
                &text,
                "This tutorial will now show you how to hack\n\
                 into a computer without being caught in\n\
                 any way.",
                time_ms,
                None,
            );

            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 32 {
            g.ecl.register_caption_change(
                &text,
                "The first thing we need is a warning system,\n\
                 so we can tell if they are about to catch us.\n\
                 We can buy software that will do this.",
                time_ms,
                None,
            );

            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 33 {
            g.ecl.register_caption_change(
                &text,
                "Connect to the Uplink Internal Services System\n\
                 and go to the Software Sales menu, as before.",
                time_ms,
                None,
            );

            self.nextvisible = false;
        } else if part == 34 {
            g.ecl.register_caption_change(
                &text,
                "Scroll down the list and click on\n\
                 'Trace Tracker'.  This item of software can tell\n\
                 you how close you are to being traced.",
                time_ms,
                None,
            );

            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 35 {
            g.ecl.register_caption_change(
                &text,
                "There are several Versions of this item.\n\
                 Higher versions have more features but\n\
                 are more expensive.",
                time_ms,
                None,
            );

            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 36 {
            g.ecl.register_caption_change(
                &text,
                "You can afford a Trace Tracker V2.0.  Use the\n\
                 'Next Version' button to select Version 2.0, then\n\
                 click on 'purchase'.",
                time_ms,
                None,
            );

            self.nextvisible = false;
        } else if part == 37 {
            g.ecl.register_caption_change(
                &text,
                "You've now bought that item of software.\n\
                 You should run it straight away from the\n\
                 software menu, under 'Security'.",
                time_ms,
                None,
            );

            g.ecl.super_highlight("hud_software");

            self.nextvisible = false;
            self.nextclicked = false;
        } else if part == 38 {
            g.ecl.register_caption_change(
                &text,
                "The Trace Tracker is now running.  It is\n\
                 currently telling you that nobody is trying\n\
                 to trace your location at this time.",
                time_ms,
                None,
            );

            self.nextvisible = true;
            self.nextclicked = false;

            g.ecl.super_unhighlight("hud_software");
        } else if part == 39 {
            g.ecl.register_caption_change(
                &text,
                "Having a warning system is a step in the right\n\
                 direction. We also need to slow down any traces\n\
                 that are being performed on us.",
                time_ms,
                None,
            );

            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 40 {
            g.ecl.register_caption_change(
                &text,
                "We do this by 'Call Bouncing'. This involves\n\
                 extending our connection aroung the world, to\n\
                 confuse the opponent.\n",
                time_ms,
                None,
            );

            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 41 {
            g.ecl.register_caption_change(&text, "First, disconnect from this system.", time_ms / 3, None);

            self.nextvisible = false;

            g.ecl.super_highlight("worldmap_connect");
        } else if part == 42 {
            g.ecl.register_caption_change(&text, "Bring up the Global Communications map.", time_ms / 3, None);

            self.nextvisible = false;

            g.ecl.super_unhighlight("worldmap_connect");
            g.ecl.super_highlight("worldmap_smallmap");
        } else if part == 43 {
            g.ecl.register_caption_change(
                &text,
                "We need to establish a long distance connection\n\
                 to our target (the Uplink Test Machine), bounced\n\
                 through several intermediate systems.",
                time_ms,
                None,
            );

            g.ecl.super_unhighlight("worldmap_smallmap");

            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 44 {
            g.ecl.register_caption_change(
                &text,
                "We build this connection by clicking on each\n\
                 of the intermediate nodes in sequence.\n\
                 Click on the location 'InterNIC'.",
                time_ms,
                None,
            );

            self.nextvisible = false;
        } else if part == 45 {
            g.ecl.register_caption_change(
                &text,
                "Click on the location\n\
                 'Uplink Public Access Server'.\n\
                 Click 'Cancel' if you make a mistake.",
                time_ms,
                None,
            );

            self.nextvisible = false;
        } else if part == 46 {
            if g.player().get_connection().get_size() == 1 {
                self.current_part = 43;
                self.run_tutorial2(g, 43);
                return;
            }

            g.ecl.register_caption_change(
                &text,
                "Click on the location\n\
                 'Uplink Internal Services System'.\n\
                 Click 'Cancel' if you make a mistake.",
                time_ms,
                None,
            );

            self.nextvisible = false;
        } else if part == 47 {
            if g.player().get_connection().get_size() == 1 {
                self.current_part = 43;
                self.run_tutorial2(g, 43);
                return;
            }

            g.ecl.register_caption_change(
                &text,
                "Finally, click on the target\n\
                 'Uplink Test Machine'.\n\
                 Click 'Cancel' if you make a mistake.",
                time_ms,
                None,
            );

            self.nextvisible = false;
        } else if part == 48 {
            if g.player().get_connection().get_size() == 1 {
                self.current_part = 43;
                self.run_tutorial2(g, 43);
                return;
            }

            g.ecl.register_caption_change(
                &text,
                "We've now set up a long distance connection\n\
                 to the target, bounced through three\n\
                 intermediate nodes.",
                time_ms,
                None,
            );

            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 49 {
            g.ecl.register_caption_change(
                &text,
                "It will take the opponent a lot longer to\n\
                 trace and disconnect us this time.  Click on\n\
                 'Connect' to establish the link.",
                time_ms,
                None,
            );

            self.nextvisible = false;
        } else if part == 50 {
            g.ecl.register_caption_change(&text, "Bypass the opening screen.", time_ms / 3, None);
        } else if part == 51 {
            g.ecl.register_caption_change(&text, "Start the password breaker now.", time_ms / 3, None);

            self.nextvisible = false;
        } else if part == 52 {
            g.ecl.register_caption_change(
                &text,
                "Target the password box. As soon as you do this,\n\
                 they will begin tracing you. Click on Proceed\n\
                 when it has finished.",
                time_ms,
                None,
            );
        } else if part == 53 {
            g.ecl.register_caption_change(
                &text,
                "The password has been broken and we're in.\n\
                 Note the trace is now in progress. This is only\n\
                 a test machine so it will be a slow trace.",
                time_ms,
                None,
            );

            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 54 {
            g.ecl.register_caption_change(
                &text,
                "If you recall the mission description, we have to\n\
                 steal a file from this system. Click on\n\
                 'Access File Server'.",
                time_ms,
                None,
            );

            self.nextvisible = false;
        } else if part == 55 {
            g.ecl.register_caption_change(
                &text,
                "The file we have to steal is named\n\
                 'Uplink Test Data'. We'll need to use a\n\
                 File Copier to download the file.",
                time_ms,
                None,
            );

            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 56 {
            g.ecl.register_caption_change(
                &text,
                "Start the File Copier utility from your\n\
                 software menu, under 'File Utilities'",
                time_ms / 2,
                None,
            );

            self.nextvisible = false;
        } else if part == 57 {
            g.ecl.register_caption_change(
                &text,
                "Target the file in question by clicking on it.\n\
                 (Uplink Test Data)",
                time_ms / 2,
                None,
            );
        } else if part == 58 {
            g.ecl.register_caption_change(
                &text,
                "The file is now being copied.\n\
                 You can speed this up by clicking on this button\n\
                 at the top to divert more CPU time to this task.",
                time_ms,
                None,
            );

            g.ecl.super_highlight("svb_increase 2");
        } else if part == 59 {
            g.ecl.register_caption_change(
                &text,
                "We've finished copying the file. We now want\n\
                 to dump it into our memory banks. Bring up the\n\
                 Memory banks screen.",
                time_ms,
                None,
            );

            g.ecl.super_unhighlight("svb_increase 2");

            g.ecl.super_highlight("hud_memory");
        } else if part == 60 {
            g.ecl.register_caption_change(&text, "Pick up the File Copier by clicking on it", time_ms / 3, None);

            g.ecl.super_unhighlight("hud_memory");
        } else if part == 61 {
            g.ecl.register_caption_change(
                &text,
                "Now click on a clear area of memory\n\
                 to dump the file in to.",
                time_ms / 2,
                None,
            );

            // The original also highlighted memory blocks 6-11 here, and unhighlighted them in
            // part 62. That was disabled in the C++ ("TODO : Above crashes"), so it is not ported.
        } else if part == 62 {
            // The original's unhighlight of memory blocks 6-11 is disabled in the C++ (it crashed).

            g.ecl.register_caption_change(
                &text,
                "We've now got the file. Disconnect from\n\
                 this computer system.",
                time_ms / 2,
                None,
            );
        } else if part == 63 {
            g.ecl.register_caption_change(
                &text,
                "In a real-world hack, we'd now have to\n\
                 make some attempt to delete the access logs\n\
                 that we've just left behind.",
                time_ms,
                None,
            );

            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 64 {
            g.ecl.register_caption_change(
                &text,
                "Every time you log on to a system, copy\n\
                 a file or make any changes, you leave\n\
                 behind access logs.",
                time_ms,
                None,
            );

            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 65 {
            g.ecl.register_caption_change(
                &text,
                "These logs can be used by other Agents\n\
                 to trace you and put you in jail. You may\n\
                 even end up tracing another Agent one day.",
                time_ms,
                None,
            );

            self.nextclicked = false;
        } else if part == 66 {
            g.ecl.register_caption_change(
                &text,
                "Back to the mission. Our final task is to\n\
                 send the copied file back to the customer.\n\
                 Bring up the mission description.",
                time_ms,
                None,
            );

            self.nextvisible = false;

            g.ecl.super_highlight("hud_mission 0");
        } else if part == 67 {
            g.ecl.register_caption_change(&text, "Click on 'reply'.", time_ms / 3, None);

            g.ecl.super_unhighlight("hud_mission 0");
            g.ecl.super_highlight("mission_reply");
        } else if part == 68 {
            g.ecl.register_caption_change(
                &text,
                "We need to attach the stolen file to this\n\
                 email before we send it. Click on the File\n\
                 button and select 'Uplink Test Data'.",
                time_ms,
                None,
            );

            g.ecl.super_unhighlight("mission_reply");
            g.ecl.super_highlight("sendmail_file");
        } else if part == 69 {
            g.ecl.register_caption_change(&text, "Now click on send.", time_ms / 3, None);

            g.ecl.super_unhighlight("sendmail_file");
            g.ecl.super_highlight("sendmail_send");
        } else if part == 70 {
            g.ecl.register_caption_change(
                &text,
                "The mission has now been completed.\n\
                 You'll receive two emails, both from Uplink.",
                time_ms / 2,
                None,
            );

            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 71 {
            g.ecl.register_caption_change(
                &text,
                "The first is congratulating you on completing\n\
                 the mission, and informing you that you've been\n\
                 paid. (In this case, nothing).",
                time_ms,
                None,
            );

            self.nextclicked = false;
        } else if part == 72 {
            g.ecl.register_caption_change(
                &text,
                "The second is informing you that your rating\n\
                 has gone up, and you'll now be able to do some\n\
                 more dangerous/profitable work.",
                time_ms,
                None,
            );

            self.nextclicked = false;
        } else if part == 73 {
            g.ecl.register_caption_change(
                &text,
                "You should now go to the Uplink Internal\n\
                 Services System and look through the work\n\
                 available to you.",
                time_ms,
                None,
            );

            self.nextclicked = false;
        } else if part == 74 {
            g.ecl.register_caption_change(
                &text,
                "This is the end of Part2 of the tutorial.\n\
                 In part3 you'll learn how to start making\n\
                 money as an Uplink agent.",
                time_ms,
                None,
            );

            self.nextclicked = false;
        } else if part == 75 {
            self.run_menu(g);
        }
    }

    pub fn run_tutorial3(&mut self, g: &mut Ctx, part: i32) {
        let pid = self.base.pid;
        let text = format!("tutorial_text {pid}");
        let time_ms = 3000;

        if part == 1 {
            g.ecl.register_caption_change(
                &text,
                "Welcome to Part 3 of the Tutorial.\n\
                 Hopefully you've finished the Test mission.\n\
                 This part will show you what to do next.",
                time_ms,
                None,
            );

            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 2 {
            g.ecl.register_caption_change(
                &text,
                "The Uplink Internal Services system is the place\n\
                 where you will find all of your future work.\n\
                 Connect there now.",
                time_ms,
                None,
            );

            self.nextvisible = false;
        } else if part == 3 {
            g.ecl.register_caption_change(&text, "Log in with your username and password.\n", time_ms / 3, None);
        } else if part == 4 {
            g.ecl.register_caption_change(
                &text,
                "Uplink maintains a list of available missions.\n\
                 You can access this list by selecting\n\
                 'Mission List' from the menu.",
                time_ms,
                None,
            );
        } else if part == 5 {
            g.ecl.register_caption_change(
                &text,
                "You should see a few missions that are available\n\
                 Now. If you don't see any available missions,\n\
                 you can accelerate time until some appear.",
                time_ms,
                None,
            );

            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 6 {
            g.ecl.register_caption_change(
                &text,
                "You can use these buttons at the top of the screen\n\
                 to accelerate time until a mission becomes\n\
                 available.",
                time_ms,
                None,
            );

            g.ecl.super_highlight("hud_speed 0");
            g.ecl.super_highlight("hud_speed 1");
            g.ecl.super_highlight("hud_speed 2");
            g.ecl.super_highlight("hud_speed 3");

            self.nextclicked = false;
        } else if part == 7 {
            g.ecl.register_caption_change(
                &text,
                "Each time you successfully complete a mission,\n\
                 you'll be paid and your Uplink rating will go up.",
                time_ms,
                None,
            );

            g.ecl.super_unhighlight("hud_speed 0");
            g.ecl.super_unhighlight("hud_speed 1");
            g.ecl.super_unhighlight("hud_speed 2");
            g.ecl.super_unhighlight("hud_speed 3");

            self.nextclicked = false;
        } else if part == 8 {
            g.ecl.register_caption_change(
                &text,
                "As your rating goes up, more missions will appear.\n\
                 Employers are more likely to offer more dangerous\n\
                 missions and more pay as your rating gets higher.",
                time_ms,
                None,
            );

            self.nextclicked = false;
        } else if part == 9 {
            g.ecl.register_caption_change(
                &text,
                "In addition, your Neuromancer rating is affected\n\
                 by the types of missions you do. This rating\n\
                 is effectivly a record of your moral values.",
                time_ms,
                None,
            );

            self.nextclicked = false;
        } else if part == 10 {
            g.ecl.register_caption_change(
                &text,
                "You may find employees are unwilling to give you\n\
                 certain missions if your Neuromancer rating\n\
                 is too low.",
                time_ms,
                None,
            );

            self.nextclicked = false;
        } else if part == 11 {
            g.ecl.register_caption_change(&text, "Exit out from the Mission screen.", time_ms / 3, None);

            self.nextvisible = false;
        } else if part == 12 {
            g.ecl.register_caption_change(
                &text,
                "Uplink Corporation maintains a large database\n\
                 of useful information on the subject of hacking\n\
                 and computer security.",
                time_ms,
                None,
            );

            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 13 {
            g.ecl.register_caption_change(&text, "Select 'Help' from the menu.", time_ms / 3, None);

            self.nextvisible = false;
        } else if part == 14 {
            g.ecl.register_caption_change(
                &text,
                "As you attempt more complex missions,\n\
                 you will undoubtably run into more complex\n\
                 security systems.",
                time_ms,
                None,
            );

            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 15 {
            g.ecl.register_caption_change(
                &text,
                "All of the information we have on these systems\n\
                 is kept in these help pages. If you get stuck,\n\
                 this is the place to look for data.",
                time_ms,
                None,
            );

            self.nextclicked = false;
        } else if part == 16 {
            g.ecl.register_caption_change(
                &text,
                "You may learn that you need some new software\n\
                 or a hardware upgrade.",
                time_ms,
                None,
            );

            self.nextclicked = false;
        } else if part == 17 {
            g.ecl.register_caption_change(&text, "Exit out from the Help screen.", time_ms / 3, None);

            self.nextvisible = false;
        } else if part == 18 {
            g.ecl.register_caption_change(
                &text,
                "Once you are working on the harder missions,\n\
                 you will find your current Gateway is too\n\
                 limiting.",
                time_ms,
                None,
            );

            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 19 {
            g.ecl.register_caption_change(
                &text,
                "You may need faster CPU capabilities,\n\
                 more memory capacity or bandwidth,\n\
                 or more personal security.",
                time_ms,
                None,
            );

            self.nextclicked = false;
        } else if part == 20 {
            g.ecl.register_caption_change(&text, "Select 'Gateway Upgrades' from the menu.", time_ms / 3, None);

            self.nextvisible = false;
        } else if part == 21 {
            g.ecl.register_caption_change(
                &text,
                "This screen shows all of the Gateways\n\
                 that you can buy from Uplink Corporation.\n\
                 Currently you have the first one off the list.",
                time_ms,
                None,
            );

            self.nextvisible = true;
            self.nextclicked = false;
        } else if part == 22 {
            g.ecl.register_caption_change(
                &text,
                "After a few missions you'll be able to afford\n\
                 to upgrade. Your old Gateway is part-exchanged,\n\
                 so you can work up the list slowly.",
                time_ms,
                None,
            );

            self.nextclicked = false;
        } else if part == 23 {
            g.ecl.register_caption_change(
                &text,
                "Uplink Corporation does not store any records\n\
                 on who is using which Gateway. This means that\n\
                 your Gateway can be traced, but you are safe.",
                time_ms,
                None,
            );

            self.nextclicked = false;
        } else if part == 24 {
            g.ecl.register_caption_change(
                &text,
                "If you are traced during a hack, we will be\n\
                 forced to sieze your accounts and Gateway.",
                time_ms,
                None,
            );

            self.nextclicked = false;
        } else if part == 25 {
            g.ecl.register_caption_change(
                &text,
                "This means you'll have to start from the bottom\n\
                 of the Gateway list again when you rejoin.\n",
                time_ms,
                None,
            );

            self.nextclicked = false;
        } else if part == 26 {
            g.ecl.register_caption_change(
                &text,
                "Later gateways have room for your own Security.\n\
                 You can install cameras and motion sensors which\n\
                 will give you early warning of trouble.",
                time_ms,
                None,
            );

            self.nextclicked = false;
        } else if part == 27 {
            g.ecl.register_caption_change(
                &text,
                "If you destroy your gateway before it is siezed,\n\
                 you can normally save your accounts and name.\n\
                 However you will of course lose your Gateway.",
                time_ms,
                None,
            );

            self.nextclicked = false;
        } else if part == 28 {
            g.ecl.register_caption_change(
                &text,
                "This is the end of Tutorial 3.\n\
                 You should have a good idea of what to do next.\n\
                 Uplink Corp. wishes you luck for the future.",
                time_ms,
                None,
            );

            self.nextclicked = false;
        } else if part == 29 {
            self.run_menu(g);
        }
    }

    /// Reads the current game state, so it needs the context.
    pub fn has_completed_current_section(&self, g: &Ctx) -> bool {
        // Default is nextclicked

        if self.current_tutorial == 1 {
            let in_screen = g.interface().li.in_screen();
            match self.current_part {
                3 => in_screen == SCREEN_HW,
                7 => in_screen == SCREEN_MEMORY,
                9 => in_screen == SCREEN_STATUS,
                12 => in_screen == SCREEN_FINANCE,
                19 => WorldMapInterface::is_visible_world_map_interface(g) == WORLDMAP_LARGE,
                21 => WorldMapInterface::is_visible_world_map_interface(g) == WORLDMAP_SMALL,
                22 => in_screen == SCREEN_EMAIL,
                25 => in_screen == SCREEN_MISSION,
                27 => in_screen == SCREEN_NONE,
                _ => self.nextclicked,
            }
        } else if self.current_tutorial == 2 {
            let player = g.player();
            let ip = remote_ip(g);
            let in_screen = g.interface().li.in_screen();
            let remote_screen = remote_screen_id(g);
            let connection = player.get_connection();
            match self.current_part {
                2 => in_screen == SCREEN_MISSION,
                4 => !player.is_connected(),
                5 => player.is_connected(),
                7 => {
                    ip == IP_UPLINKTESTMACHINE
                        && matches!(RemoteInterface::get_computer_screen(g), Some(ComputerScreen::UserIDScreen(_)))
                }
                10 => !player.is_connected(),
                11 => ip == IP_UPLINKINTERNALSERVICES,
                13 => {
                    ip == IP_UPLINKINTERNALSERVICES
                        && matches!(RemoteInterface::get_computer_screen(g), Some(ComputerScreen::MenuScreen(_)))
                }
                17 => ip == IP_UPLINKINTERNALSERVICES && remote_screen == SCREEN_SWSALESSCREEN,
                20 => player.gateway.databank.get_data_by_title("Password_Breaker").is_some(),
                22 => {
                    ip == IP_UPLINKTESTMACHINE
                        && matches!(RemoteInterface::get_computer_screen(g), Some(ComputerScreen::UserIDScreen(_)))
                }
                24 => SWInterface::is_visible_software_menu(g),
                25 => g.svb.get_task_by_name("Password_Breaker") != -1,
                27 => !player.is_connected(),
                33 => ip == IP_UPLINKINTERNALSERVICES && remote_screen == SCREEN_SWSALESSCREEN,
                36 => player.gateway.databank.get_data_by_title("Trace_Tracker").is_some_and(|d| d.version >= 2.0),
                37 => g.svb.get_task_by_name("Trace_Tracker") != -1,
                41 => !player.is_connected(),
                42 => WorldMapInterface::is_visible_world_map_interface(g) == WORLDMAP_LARGE,
                44 => connection.location_included(IP_INTERNIC),
                45 => {
                    connection.location_included(IP_UPLINKPUBLICACCESSSERVER) || connection.get_size() == 1
                }
                46 => connection.location_included(IP_UPLINKINTERNALSERVICES) || connection.get_size() == 1,
                47 => connection.location_included(IP_UPLINKTESTMACHINE) || connection.get_size() == 1,
                49 => ip == IP_UPLINKTESTMACHINE,
                50 => {
                    ip == IP_UPLINKTESTMACHINE
                        && matches!(RemoteInterface::get_computer_screen(g), Some(ComputerScreen::UserIDScreen(_)))
                }
                51 => g.svb.get_task_by_name("Password_Breaker") != -1,
                52 => {
                    ip == IP_UPLINKTESTMACHINE
                        && matches!(RemoteInterface::get_computer_screen(g), Some(ComputerScreen::MenuScreen(_)))
                }
                54 => ip == IP_UPLINKTESTMACHINE && remote_screen == SCREEN_FILESERVERSCREEN,
                56 => g.svb.get_task_by_name("File_Copier") != -1,
                57 => {
                    let pid = g.svb.get_task_by_name("File_Copier");
                    g.svb.get_task::<FileCopier>(pid).is_some_and(|c| c.get_state() == FILECOPIER_INPROGRESS)
                }
                58 => {
                    let pid = g.svb.get_task_by_name("File_Copier");
                    g.svb.get_task::<FileCopier>(pid).is_some_and(|c| c.get_state() == FILECOPIER_WAITINGFORTARGET)
                }
                59 => in_screen == SCREEN_MEMORY,
                60 => TaskManager::get_target_program_name(g).as_deref() == Some("File_Copier"),
                61 => player.gateway.databank.get_data_by_title("Uplink test data").is_some(),
                62 => !player.is_connected(),
                66 => in_screen == SCREEN_MISSION,
                67 => in_screen == SCREEN_SENDMAIL,
                68 => {
                    in_screen == SCREEN_SENDMAIL
                        && g.interface()
                            .li
                            .screen
                            .get::<SendMailInterface>()
                            .and_then(|s| s.s_get_message().get_data())
                            .is_some_and(|d| d.title == "Uplink test data")
                }
                69 => in_screen == SCREEN_NONE,
                _ => self.nextclicked,
            }
        } else if self.current_tutorial == 3 {
            let ip = remote_ip(g);
            let remote_screen = remote_screen_id(g);
            let subtitle = RemoteInterface::get_computer_screen(g).map(|cs| cs.base().subtitle.as_str());
            match self.current_part {
                2 => ip == IP_UPLINKINTERNALSERVICES,
                3 => {
                    ip == IP_UPLINKINTERNALSERVICES
                        && matches!(RemoteInterface::get_computer_screen(g), Some(ComputerScreen::MenuScreen(_)))
                }
                4 => ip == IP_UPLINKINTERNALSERVICES && remote_screen == SCREEN_BBSSCREEN,
                11 => {
                    ip == IP_UPLINKINTERNALSERVICES
                        && matches!(RemoteInterface::get_computer_screen(g), Some(ComputerScreen::MenuScreen(_)))
                }
                13 => ip == IP_UPLINKINTERNALSERVICES && subtitle == Some("Help/"),
                17 => ip == IP_UPLINKINTERNALSERVICES && subtitle == Some("Internal Services Main Menu"),
                20 => ip == IP_UPLINKINTERNALSERVICES && remote_screen == SCREEN_CHANGEGATEWAY,
                _ => self.nextclicked,
            }
        } else {
            false
        }
    }

    pub fn move_menu_to(&mut self, g: &mut Ctx, x: i32, y: i32, time_ms: i32) {
        if self.is_menu_interface_visible(g) {
            let n = menu_names(self.base.pid);

            g.ecl.register_movement(&n.part1button, x + 10, y + 15, time_ms, MOVE_STRAIGHTLINE, None);
            g.ecl.register_movement(&n.part2button, x + 10, y + 15 + 16, time_ms, MOVE_STRAIGHTLINE, None);
            g.ecl.register_movement(&n.part3button, x + 10, y + 15 + 32, time_ms, MOVE_STRAIGHTLINE, None);

            g.ecl.register_movement(&n.part1, x + 90, y + 15, time_ms, MOVE_STRAIGHTLINE, None);
            g.ecl.register_movement(&n.part2, x + 90, y + 15 + 16, time_ms, MOVE_STRAIGHTLINE, None);
            g.ecl.register_movement(&n.part3, x + 90, y + 15 + 32, time_ms, MOVE_STRAIGHTLINE, None);

            g.ecl.button_bring_to_front(&n.part1);
            g.ecl.button_bring_to_front(&n.part2);
            g.ecl.button_bring_to_front(&n.part3);
            g.ecl.button_bring_to_front(&n.part1button);
            g.ecl.button_bring_to_front(&n.part2button);
            g.ecl.button_bring_to_front(&n.part3button);
        }
    }

    pub fn create_menu_interface(&mut self, g: &mut Ctx) {
        if !self.is_menu_interface_visible(g) {
            let pid = self.base.pid;
            let title = format!("tutorial_title {pid}");
            let Some((tx, ty)) = g.ecl.get_button(&title).map(|b| (b.x, b.y)) else {
                debug_assert!(false, "Tutorial::CreateMenuInterface: no title button");
                return;
            };

            //
            // Add the new buttons in
            //

            let n = menu_names(pid);

            g.ecl.register_button(tx + 10, ty + 15, 80, 14, "Tutorial One", "Run the first tutorial", &n.part1button);
            g.ecl.register_button(tx + 10, ty + 15 + 16, 80, 14, "Tutorial Two", "Run the second tutorial", &n.part2button);
            g.ecl.register_button(tx + 10, ty + 15 + 32, 80, 14, "Tutorial Three", "Run the third tutorial", &n.part3button);

            g.ecl.register_button_callback(&n.part1button, Self::menu_button_click);
            g.ecl.register_button_callback(&n.part2button, Self::menu_button_click);
            g.ecl.register_button_callback(&n.part3button, Self::menu_button_click);

            g.ecl.register_button(tx + 90, ty + 15, 190, 14, "The basics of your screen", "Run the first tutorial", &n.part1);
            g.ecl.register_button(
                tx + 90,
                ty + 15 + 16,
                190,
                14,
                "Completing the first mission",
                "Run the second tutorial",
                &n.part2,
            );
            g.ecl.register_button(tx + 90, ty + 15 + 32, 150, 14, "Working as an Agent", "Run the third tutorial", &n.part3);

            g.ecl.register_button_callbacks(
                &n.part1,
                Some(text_draw),
                Some(Self::menu_button_click),
                Some(button_click),
                Some(button_highlight),
            );
            g.ecl.register_button_callbacks(
                &n.part2,
                Some(text_draw),
                Some(Self::menu_button_click),
                Some(button_click),
                Some(button_highlight),
            );
            g.ecl.register_button_callbacks(
                &n.part3,
                Some(text_draw),
                Some(Self::menu_button_click),
                Some(button_click),
                Some(button_highlight),
            );
        }
    }

    pub fn remove_menu_interface(&mut self, g: &mut Ctx) {
        if self.is_menu_interface_visible(g) {
            let n = menu_names(self.base.pid);

            g.ecl.remove_button(&n.part1);
            g.ecl.remove_button(&n.part2);
            g.ecl.remove_button(&n.part3);
            g.ecl.remove_button(&n.part1button);
            g.ecl.remove_button(&n.part2button);
            g.ecl.remove_button(&n.part3button);
        }
    }

    pub fn show_menu_interface(&mut self, g: &mut Ctx) {
        if self.is_menu_interface_visible(g) {
            let n = menu_names(self.base.pid);

            g.ecl.button_bring_to_front(&n.part1);
            g.ecl.button_bring_to_front(&n.part2);
            g.ecl.button_bring_to_front(&n.part3);
            g.ecl.button_bring_to_front(&n.part1button);
            g.ecl.button_bring_to_front(&n.part2button);
            g.ecl.button_bring_to_front(&n.part3button);
        }
    }

    pub fn is_menu_interface_visible(&self, g: &Ctx) -> bool {
        let n = menu_names(self.base.pid);
        g.ecl.get_button(&n.part1).is_some()
    }

    pub fn close_click(g: &mut Ctx, button: &Button) {
        let pid = button_pid(button);

        Vanbakel::remove_task(g, pid);
    }

    pub fn title_click(g: &mut Ctx, button: &Button) {
        let pid = button_pid(button);

        if let Some(thistask) = g.svb.get_task_mut::<Tutorial>(pid) {
            thistask.base.followmouse = true;
        }
        g.interface_mut().tm.set_target_program(pid);
    }

    pub fn skip_click(g: &mut Ctx, button: &Button) {
        let pid = button_pid(button);

        with_tutorial(g, pid, |thistask, g| thistask.run_menu(g));
    }

    pub fn next_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        let pid = button_pid(button);

        let nextvisible = g.svb.get_task::<Tutorial>(pid).is_some_and(|t| t.nextvisible);
        if nextvisible {
            button_draw(g, button, highlighted, clicked);
        }
    }

    pub fn next_mouse_move(g: &mut Ctx, button: &Button) {
        let pid = button_pid(button);

        let nextvisible = g.svb.get_task::<Tutorial>(pid).is_some_and(|t| t.nextvisible);
        if nextvisible {
            button_highlight(g, button);
        }
    }

    pub fn next_mouse_down(g: &mut Ctx, button: &Button) {
        let pid = button_pid(button);

        let nextvisible = g.svb.get_task::<Tutorial>(pid).is_some_and(|t| t.nextvisible);
        if nextvisible {
            button_click(g, button);
        }
    }

    pub fn next_click(g: &mut Ctx, button: &Button) {
        let pid = button_pid(button);

        if let Some(thistask) = g.svb.get_task_mut::<Tutorial>(pid).filter(|t| t.nextvisible) {
            thistask.nextclicked = true;
        }
    }

    pub fn menu_button_click(g: &mut Ctx, button: &Button) {
        // "%s %d %d": the menu part index, then the task's PID.
        let mut parts = button.name.split_whitespace().skip(1);
        let buttonindex: i32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(-1);
        let pid: i32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(-1);

        // Run that part of the tutorial

        with_tutorial(g, pid, |thistask, g| {
            thistask.remove_menu_interface(g);
            thistask.menu = false;
            thistask.run_tutorial(g, buttonindex, 1);
        });
    }

    pub fn main_text_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        gl::begin(gl::QUADS);
        gl::color3ub(8, 20, 0);
        gl::vertex2i(button.x, button.y + button.height);
        gl::color3ub(8, 20, 124);
        gl::vertex2i(button.x, button.y);
        gl::color3ub(8, 20, 0);
        gl::vertex2i(button.x + button.width, button.y);
        gl::color3ub(8, 20, 124);
        gl::vertex2i(button.x + button.width, button.y + button.height);
        gl::end();

        gl::color3ub(81, 138, 215);
        border_draw(button);

        text_draw(g, button, highlighted, clicked);
    }
}

impl UplinkTask for Tutorial {
    fn base(&self) -> &UplinkTaskBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut UplinkTaskBase {
        &mut self.base
    }
    fn initialise(&mut self, _g: &mut Ctx) {}
    fn tick(&mut self, g: &mut Ctx, _n: i32) {
        if !self.menu && self.has_completed_current_section(g) {
            let (tutorial, part) = (self.current_tutorial, self.current_part + 1);
            self.run_tutorial(g, tutorial, part);
        }
    }
    fn move_to(&mut self, g: &mut Ctx, x: i32, y: i32, time_ms: i32) {
        if self.is_interface_visible(g) {
            let n = names(self.base.pid);

            g.ecl.register_movement(&n.title, x + 1, y + 1, time_ms, MOVE_STRAIGHTLINE, None);
            g.ecl.register_movement(&n.skip, x + 236, y + 1, time_ms, MOVE_STRAIGHTLINE, None);
            g.ecl.register_movement(&n.close, x + 276, y + 1, time_ms, MOVE_STRAIGHTLINE, None);
            g.ecl.register_movement(&n.text, x + 1, y + 13, time_ms, MOVE_STRAIGHTLINE, None);
            g.ecl.register_movement(&n.next, x + 247, y + 47, time_ms, MOVE_STRAIGHTLINE, None);

            g.ecl.button_bring_to_front(&n.title);
            g.ecl.button_bring_to_front(&n.close);
            g.ecl.button_bring_to_front(&n.text);
            g.ecl.button_bring_to_front(&n.next);

            if self.is_menu_interface_visible(g) {
                self.move_menu_to(g, x, y, time_ms);
            }
        }
    }
    fn create_interface(&mut self, g: &mut Ctx) {
        if !self.is_interface_visible(g) {
            let n = names(self.base.pid);

            g.ecl.register_button(200, 400, 235, 13, "Tutorial Daemon", "The tutorial software", &n.title);
            g.ecl.register_button_callback(&n.title, Self::title_click);

            g.ecl.register_button(435, 400, 40, 13, "Skip", "Skip this section of the tutorial", &n.skip);
            g.ecl.register_button_callback(&n.skip, Self::skip_click);

            g.ecl.register_button(475, 400, 13, 13, "Close", "Close the tutorial software", &n.close);
            button_assignbitmaps(g, &n.close, "close.tif", "close_h.tif", "close_c.tif");
            g.ecl.register_button_callback(&n.close, Self::close_click);

            g.ecl.register_button(200, 413, 288, 50, "Starting...", "", &n.text);
            g.ecl.register_button_callbacks(&n.text, Some(Self::main_text_draw), None, None, None);

            g.ecl.register_button(447, 447, 40, 15, "Next", "Click to go on to the next stage", &n.next);
            g.ecl.register_button_callbacks(
                &n.next,
                Some(Self::next_draw),
                Some(Self::next_click),
                Some(Self::next_mouse_down),
                Some(Self::next_mouse_move),
            );

            // Run tutorial 1, part 1
            self.run_tutorial(g, 1, 1);

            //self.run_menu(g);
        }
    }
    fn remove_interface(&mut self, g: &mut Ctx) {
        if self.is_interface_visible(g) {
            let n = names(self.base.pid);

            g.ecl.remove_button(&n.title);
            g.ecl.remove_button(&n.skip);
            g.ecl.remove_button(&n.close);
            g.ecl.remove_button(&n.text);
            g.ecl.remove_button(&n.next);

            if self.is_menu_interface_visible(g) {
                self.remove_menu_interface(g);
            }
        }
    }
    fn show_interface(&mut self, g: &mut Ctx) {
        if self.is_interface_visible(g) {
            let n = names(self.base.pid);

            g.ecl.button_bring_to_front(&n.title);
            g.ecl.button_bring_to_front(&n.skip);
            g.ecl.button_bring_to_front(&n.close);
            g.ecl.button_bring_to_front(&n.text);
            g.ecl.button_bring_to_front(&n.next);

            if self.is_menu_interface_visible(g) {
                self.show_menu_interface(g);
            }
        }
    }
    fn is_interface_visible(&self, g: &Ctx) -> bool {
        let n = names(self.base.pid);
        g.ecl.get_button(&n.title).is_some()
    }
    impl_as_any!();
}
