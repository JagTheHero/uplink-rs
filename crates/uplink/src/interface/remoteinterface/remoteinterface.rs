//! Port of source/interface/remoteinterface/remoteinterface.h + source/interface/remoteinterface/remoteinterface.cpp
#![allow(unused_variables, unused_imports, unused_mut, dead_code)]

use crate::ctx::Ctx;
use crate::interface::interfacescreen::{InterfaceScreen, ScreenSlot};
use crate::interface::remoteinterface::academicscreen_interface::AcademicScreenInterface;
use crate::interface::remoteinterface::accountscreen_interface::AccountScreenInterface;
use crate::interface::remoteinterface::bbsscreen_interface::BBSScreenInterface;
use crate::interface::remoteinterface::changegatewayscreen_interface::ChangeGatewayScreenInterface;
use crate::interface::remoteinterface::codecardscreen_interface::CodeCardScreenInterface;
use crate::interface::remoteinterface::companyinfoscreen_interface::CompanyInfoScreenInterface;
use crate::interface::remoteinterface::consolescreen_interface::ConsoleScreenInterface;
use crate::interface::remoteinterface::contactscreen_interface::ContactScreenInterface;
use crate::interface::remoteinterface::criminalscreen_interface::CriminalScreenInterface;
use crate::interface::remoteinterface::cypherscreen_interface::CypherScreenInterface;
use crate::interface::remoteinterface::dialogscreen_interface::DialogScreenInterface;
use crate::interface::remoteinterface::disconnectedscreen_interface::DisconnectedScreenInterface;
use crate::interface::remoteinterface::faithscreen_interface::FaithScreenInterface;
use crate::interface::remoteinterface::fileserverscreen_interface::FileServerScreenInterface;
use crate::interface::remoteinterface::highsecurityscreen_interface::HighSecurityScreenInterface;
use crate::interface::remoteinterface::hwsalesscreen_interface::HWSalesScreenInterface;
use crate::interface::remoteinterface::linksscreen_interface::LinksScreenInterface;
use crate::interface::remoteinterface::loansscreen_interface::LoansScreenInterface;
use crate::interface::remoteinterface::logscreen_interface::LogScreenInterface;
use crate::interface::remoteinterface::menuscreen_interface::MenuScreenInterface;
use crate::interface::remoteinterface::messagescreen_interface::MessageScreenInterface;
use crate::interface::remoteinterface::nearestgatewayscreen_interface::NearestGatewayScreenInterface;
use crate::interface::remoteinterface::newsscreen_interface::NewsScreenInterface;
use crate::interface::remoteinterface::nuclearwarscreen_interface::NuclearWarScreenInterface;
use crate::interface::remoteinterface::passwordscreen_interface::PasswordScreenInterface;
use crate::interface::remoteinterface::protovisionscreen_interface::ProtovisionScreenInterface;
use crate::interface::remoteinterface::radiotransmitterscreen_interface::RadioTransmitterScreenInterface;
use crate::interface::remoteinterface::rankingscreen_interface::RankingScreenInterface;
use crate::interface::remoteinterface::recordscreen_interface::RecordScreenInterface;
use crate::interface::remoteinterface::securityscreen_interface::SecurityScreenInterface;
use crate::interface::remoteinterface::shareslistscreen_interface::SharesListScreenInterface;
use crate::interface::remoteinterface::sharesviewscreen_interface::SharesViewScreenInterface;
use crate::interface::remoteinterface::socialsecurityscreen_interface::SocialSecurityScreenInterface;
use crate::interface::remoteinterface::swsalesscreen_interface::SWSalesScreenInterface;
use crate::interface::remoteinterface::useridscreen_interface::UserIDScreenInterface;
use crate::interface::remoteinterface::voiceanalysisscreen_interface::VoiceAnalysisScreenInterface;
use crate::interface::remoteinterface::voicephonescreen_interface::VoicePhoneScreenInterface;
use crate::objref::ScreenRef;
use crate::vanbakel::Vanbakel;
use crate::world::computer::computer::Computer;
use crate::world::computer::computerscreen::computerscreen::ComputerScreen;
use crate::world::connection::Connection;

// === List all screen codes here ===

pub const SCREEN_UNKNOWN: i32 = 0;
pub const SCREEN_MESSAGESCREEN: i32 = 1;
pub const SCREEN_PASSWORDSCREEN: i32 = 2;
pub const SCREEN_MENUSCREEN: i32 = 3;
pub const SCREEN_BBSSCREEN: i32 = 4;
pub const SCREEN_DIALOGSCREEN: i32 = 5;
pub const SCREEN_FILESERVERSCREEN: i32 = 6;
pub const SCREEN_LINKSSCREEN: i32 = 7;
pub const SCREEN_LOGSCREEN: i32 = 8;
pub const SCREEN_SWSALESSCREEN: i32 = 9;
pub const SCREEN_HWSALESSCREEN: i32 = 10;
pub const SCREEN_RECORDSCREEN: i32 = 11;
pub const SCREEN_USERIDSCREEN: i32 = 12;
pub const SCREEN_ACCOUNTSCREEN: i32 = 13;
pub const SCREEN_CONTACTSCREEN: i32 = 14;
pub const SCREEN_NEWSSCREEN: i32 = 15;
pub const SCREEN_CRIMINALSCREEN: i32 = 16;
pub const SCREEN_SECURITYSCREEN: i32 = 17;
pub const SCREEN_ACADEMICSCREEN: i32 = 18;
pub const SCREEN_RANKINGSCREEN: i32 = 19;
pub const SCREEN_CONSOLESCREEN: i32 = 20;
pub const SCREEN_SOCSECSCREEN: i32 = 21;
pub const SCREEN_LOANSSCREEN: i32 = 22;
pub const SCREEN_SHARESLISTSCREEN: i32 = 23;
pub const SCREEN_SHARESVIEWSCREEN: i32 = 24;
pub const SCREEN_FAITHSCREEN: i32 = 25;
pub const SCREEN_CYPHERSCREEN: i32 = 26;
pub const SCREEN_VOICEANALYSIS: i32 = 27;
pub const SCREEN_COMPANYINFO: i32 = 28;
pub const SCREEN_VOICEPHONE: i32 = 29;
pub const SCREEN_HIGHSECURITYSCREEN: i32 = 30;
pub const SCREEN_NEARESTGATEWAY: i32 = 31;
pub const SCREEN_CHANGEGATEWAY: i32 = 32;
pub const SCREEN_CODECARD: i32 = 33;
pub const SCREEN_DISCONNECTEDSCREEN: i32 = 34;
pub const SCREEN_PROTOVISION: i32 = 35;
pub const SCREEN_NUCLEARWAR: i32 = 36;
pub const SCREEN_RADIOTRANSMITTER: i32 = 37;

fn screen_slot(g: &mut Ctx) -> &mut ScreenSlot {
    &mut g.interface_mut().ri.screen
}

/// The `LanMonitor::currentSelected` static.
fn lan_current_selected(g: &Ctx) -> i32 {
    g.lan_monitor.current_selected
}

/// The screen index a fresh location opens on: a special location's screen, or the screen of the
/// LAN system selected in the LAN monitor. 0 when neither applies.
fn location_screen_index(g: &Ctx) -> i32 {
    let world = g.world();
    let Some(host) = g.player().get_remote_host(world) else {
        return 0;
    };
    if let Some(special) = &host.special {
        return special.screen_index;
    }
    let Some(lan) = world
        .get_computer(&host.computer)
        .and_then(|comp| comp.lan())
    else {
        return 0;
    };
    match lan.systems.get_data(lan_current_selected(g)) {
        Some(system) if system.screen_index != -1 => system.screen_index,
        _ => 0,
    }
}

/// The computer behind the current remote screen (`GetComputerScreen()->GetComputer()`).
fn current_computer(g: &Ctx) -> Option<&Computer> {
    let sref = g.interface().ri.screen.get_dyn()?.computer_screen_ref()?;
    g.world().get_computer(&sref.computer)
}

/// The interface for a computer screen (the type switch in C++ `RunScreen`).
fn screen_for(cs: &ComputerScreen) -> Box<dyn InterfaceScreen> {
    match cs {
        ComputerScreen::MessageScreen(_) => Box::new(MessageScreenInterface::default()),
        ComputerScreen::PasswordScreen(_) => Box::new(PasswordScreenInterface::default()),
        ComputerScreen::MenuScreen(_) => Box::new(MenuScreenInterface::default()),
        ComputerScreen::DialogScreen(_) => Box::new(DialogScreenInterface::default()),
        ComputerScreen::UserIDScreen(_) => Box::new(UserIDScreenInterface::default()),
        ComputerScreen::LogScreen(_) => Box::new(LogScreenInterface::default()),
        ComputerScreen::BBSScreen(_) => Box::new(BBSScreenInterface::default()),
        ComputerScreen::LinksScreen(_) => Box::new(LinksScreenInterface::default()),
        ComputerScreen::SharesListScreen(_) => Box::new(SharesListScreenInterface::default()),
        ComputerScreen::CypherScreen(_) => Box::new(CypherScreenInterface::default()),
        ComputerScreen::HighSecurityScreen(_) => Box::new(HighSecurityScreenInterface::default()),
        ComputerScreen::DisconnectedScreen(_) => Box::new(DisconnectedScreenInterface::default()),
        ComputerScreen::GenericScreen(generic) => match generic.screen_type {
            SCREEN_FILESERVERSCREEN => Box::new(FileServerScreenInterface::default()),
            SCREEN_SWSALESSCREEN => Box::new(SWSalesScreenInterface::default()),
            SCREEN_HWSALESSCREEN => Box::new(HWSalesScreenInterface::default()),
            SCREEN_RECORDSCREEN => Box::new(RecordScreenInterface::default()),
            SCREEN_ACCOUNTSCREEN => Box::new(AccountScreenInterface::default()),
            SCREEN_CONTACTSCREEN => Box::new(ContactScreenInterface::default()),
            SCREEN_NEWSSCREEN => Box::new(NewsScreenInterface::default()),
            SCREEN_CRIMINALSCREEN => Box::new(CriminalScreenInterface::default()),
            SCREEN_SECURITYSCREEN => Box::new(SecurityScreenInterface::default()),
            SCREEN_ACADEMICSCREEN => Box::new(AcademicScreenInterface::default()),
            SCREEN_RANKINGSCREEN => Box::new(RankingScreenInterface::default()),
            SCREEN_CONSOLESCREEN => Box::new(ConsoleScreenInterface::default()),
            SCREEN_SOCSECSCREEN => Box::new(SocialSecurityScreenInterface::default()),
            SCREEN_LOANSSCREEN => Box::new(LoansScreenInterface::default()),
            SCREEN_SHARESVIEWSCREEN => Box::new(SharesViewScreenInterface::default()),
            SCREEN_FAITHSCREEN => Box::new(FaithScreenInterface::default()),
            SCREEN_VOICEANALYSIS => Box::new(VoiceAnalysisScreenInterface::default()),
            SCREEN_COMPANYINFO => Box::new(CompanyInfoScreenInterface::default()),
            SCREEN_VOICEPHONE => Box::new(VoicePhoneScreenInterface::default()),
            SCREEN_NEARESTGATEWAY => Box::new(NearestGatewayScreenInterface::default()),
            SCREEN_CHANGEGATEWAY => Box::new(ChangeGatewayScreenInterface::default()),
            SCREEN_CODECARD => Box::new(CodeCardScreenInterface::default()),
            SCREEN_PROTOVISION => Box::new(ProtovisionScreenInterface::default()),
            SCREEN_NUCLEARWAR => Box::new(NuclearWarScreenInterface::default()),
            SCREEN_RADIOTRANSMITTER => Box::new(RadioTransmitterScreenInterface::default()),
            _ => panic!("Unrecognised GenericScreen"),
        },
    }
}

/// The interface projected by the current host machine (password screen, log screen, ...).
/// `screen` holds a `RemoteInterfaceScreen` subclass.
// PORT: C++ Load/Save persisted the four plain fields, then called `Create()`.
pub struct RemoteInterface {
    pub screen: ScreenSlot,
    pub previousscreenindex: i32,
    pub currentscreenindex: i32,
    pub security_name: String,
    pub security_level: i32,
}

impl RemoteInterface {
    pub fn new() -> Self {
        RemoteInterface {
            screen: ScreenSlot::new(),
            previousscreenindex: 0,
            currentscreenindex: 0,
            security_name: " ".to_string(),
            security_level: 10,
        }
    }

    /// Looks at the player's location: picks the starting screen (special VLocation screen
    /// index / LAN system screen), creates it, and disconnects if the computer is down or
    /// not externally open.
    pub fn run_new_location(g: &mut Ctx) {
        // A screen method is running (the slot is taken out): start the new location once it returns.
        if screen_slot(g).is_busy() {
            g.defer(Self::run_new_location);
            return;
        }

        // Get rid of the current remote interface
        ScreenSlot::replace(g, screen_slot, None);

        let start = location_screen_index(g);
        {
            let ri = &mut g.interface_mut().ri;
            ri.previousscreenindex = 0;
            ri.currentscreenindex = start;
            ri.set_security("Guest", 10);
        }

        Self::create(g);

        let owner = g.player().connection.owner.clone();

        if !current_computer(g)
            .expect("RemoteInterface: no computer behind the screen")
            .isrunning
        {
            // This computer is fucked, so disconnect now
            Connection::disconnect(g, &owner);
            Connection::reset(g, &owner);

            Self::run_new_location(g);
            Self::run_screen(g, 8);
        }

        let (externally_open, company) = {
            let comp = current_computer(g).expect("RemoteInterface: no computer behind the screen");
            (comp.isexternallyopen, comp.companyname.clone())
        };
        if !externally_open {
            // This computer cannot be connected to from an external location
            let ipfrom = g
                .player()
                .get_connection()
                .get_ghost()
                .expect("RemoteInterface: connection has no source")
                .to_string();
            let vlfrom = g
                .world()
                .get_vlocation(&ipfrom)
                .expect("RemoteInterface: source location not found");
            let compfrom = g
                .world()
                .get_computer(&vlfrom.computer)
                .expect("RemoteInterface: source location has no computer");
            let same_company = compfrom.companyname == company;

            if !same_company {
                Connection::disconnect(g, &owner);
                Connection::reset(g, &owner);

                Self::run_new_location(g);
                Self::run_screen(g, 9);
            }
        }
    }

    /// Runs a screen at the existing location: builds the matching `*ScreenInterface` for the
    /// computer screen `currentscreenindex`, `set_computer_screen`s it and installs it with
    /// `ScreenSlot::replace(g, screen_slot, ..)`, then brings all running tasks to the front.
    pub fn run_screen(g: &mut Ctx, screenindex: i32) {
        // A screen method is running (the slot is taken out): switch once it returns.
        if screen_slot(g).is_busy() {
            g.defer(move |g| Self::run_screen(g, screenindex));
            return;
        }

        // Get rid of the current remote interface
        ScreenSlot::replace(g, screen_slot, None);

        {
            let ri = &mut g.interface_mut().ri;
            ri.previousscreenindex = ri.currentscreenindex;
            ri.currentscreenindex = screenindex;
        }

        // Interface with this computer
        let host = g
            .player()
            .get_remote_host(g.world())
            .expect("RemoteInterface::RunScreen: no remote host");
        let computer_name = host.computer.clone();
        let comp = g
            .world()
            .get_computer(&computer_name)
            .expect("RemoteInterface::RunScreen: remote host has no computer");
        let cs = comp
            .get_computer_screen(screenindex)
            .unwrap_or_else(|| panic!("Tried to run an invalid screenid"));

        let mut screen = screen_for(cs);
        screen.set_computer_screen(ScreenRef::new(&computer_name, screenindex));
        ScreenSlot::replace(g, screen_slot, Some(screen));

        // Bring all running tasks to the front
        Vanbakel::show_all_tasks(g);
    }

    pub fn set_security(&mut self, newname: &str, newvalue: i32) {
        self.security_name = newname.to_string();
        self.security_level = newvalue;
    }

    pub fn create(g: &mut Ctx) {
        if !Self::is_visible(g) {
            // Run the interface
            let index = g.interface().ri.currentscreenindex;
            Self::run_screen(g, index);
        }
    }

    pub fn remove(g: &mut Ctx) {
        if Self::is_visible(g) {
            ScreenSlot::call(g, screen_slot, |s, g| s.remove(g));
        }
    }

    pub fn is_visible(g: &Ctx) -> bool {
        g.interface()
            .ri
            .screen
            .get_dyn()
            .is_some_and(|s| s.is_visible(g))
    }

    /// `GetComputerScreen`: the `ComputerScreen` the current remote screen shows
    /// (resolved from its `ScreenRef` against the world).
    pub fn get_computer_screen(g: &Ctx) -> Option<&ComputerScreen> {
        let sref = g.interface().ri.screen.get_dyn()?.computer_screen_ref()?;
        g.world()
            .get_computer(&sref.computer)?
            .get_computer_screen(sref.index)
    }

    pub fn update(g: &mut Ctx) {
        ScreenSlot::call(g, screen_slot, |s, g| s.update(g));
    }
}

impl Default for RemoteInterface {
    fn default() -> Self {
        RemoteInterface::new()
    }
}
