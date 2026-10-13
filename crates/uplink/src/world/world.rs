//! Port of source/world/world.h + source/world/world.cpp

use std::io::{BufRead, Cursor};

use serde::{Deserialize, Serialize};

use crate::app::globals::{DEMOGAME, WAREZRELEASE};
use crate::ctx::Ctx;
use crate::game::data::data::{TIME_TOWAREZGAMEOVER, WAREZ_MAXPLAYTIME, WORLD_START_DATE};
use crate::tosser::{BTree, DArray};
use crate::world::company::company::Company;
use crate::world::computer::computer::Computer;
use crate::world::computer::gatewaydef::GatewayDef;
use crate::world::date::Date;
use crate::world::generator::consequencegenerator::ConsequenceGenerator;
use crate::world::generator::demoplotgenerator::DemoPlotGenerator;
use crate::world::generator::langenerator::LanGenerator;
use crate::world::generator::missiongenerator::MissionGenerator;
use crate::world::generator::namegenerator::NameGenerator;
use crate::world::generator::newsgenerator::NewsGenerator;
use crate::world::generator::numbergenerator::NumberGenerator;
use crate::world::generator::plotgenerator::PlotGenerator;
use crate::world::generator::recordgenerator::RecordGenerator;
use crate::world::generator::worldgenerator::WorldGenerator;
use crate::world::person::Person;
use crate::world::player::Player;
use crate::world::scheduler::eventscheduler::EventScheduler;
use crate::world::scheduler::notificationevent::{
    NotificationEvent, NOTIFICATIONEVENT_TYPE_WAREZGAMEOVER,
};
use crate::world::scheduler::uplinkevent::UplinkEvent;
use crate::world::vlocation::VLocation;

/// The database of information representing the game world, including companies, people,
/// computers, missions etc.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct World {
    pub nextupdate: Date,

    pub date: Date,
    pub scheduler: EventScheduler,
    pub plotgenerator: PlotGenerator,
    pub demoplotgenerator: DemoPlotGenerator,

    /// Keyed on ip
    pub locations: BTree<VLocation>,
    /// Keyed on name
    pub companies: BTree<Company>,
    /// Keyed on name
    pub computers: BTree<Computer>,
    /// NPC people, keyed on name. The player's data is NOT in here: see `player`.
    pub people: BTree<Person>,

    // PORT: the C++ kept the player in `people` under "PLAYER" (cast to Player); the Rust world
    // stores it here and `get_person("PLAYER")` returns its Person. `people` holds a placeholder
    // key "PLAYER" where the C++ entry was (see `set_player`), so walks of `people` keep its place.
    pub player: Option<Box<Player>>,

    /// No need to serialise
    #[serde(skip)]
    pub passwords: DArray<String>,
    /// No need to serialise
    #[serde(skip)]
    pub gatewaydefs: DArray<GatewayDef>,
}

impl World {
    /// `World::World`: sets the start date and activates it, initialises the generators and
    /// loads `data/wordlist.txt` (passwords) and `data/gatewaydefs.txt` (gateway definitions).
    // PORT: `Default` gives an empty world without any of that. The generators' `initialise`
    // takes the context, hence `g` (the world is not yet in it while this runs).
    pub fn new(g: &Ctx) -> World {
        let mut world = World::default();

        let (second, minute, hour, day, month, year) = WORLD_START_DATE;
        world.date.set_date(second, minute, hour, day, month, year);
        world.date.activate(); // Make it update with the game

        //
        // Initialise the static generators
        //

        WorldGenerator::initialise(g);
        MissionGenerator::initialise();
        RecordGenerator::initialise();
        NumberGenerator::initialise();
        NameGenerator::initialise();
        ConsequenceGenerator::initialise();
        NewsGenerator::initialise();
        LanGenerator::initialise();

        //
        // Load the password list from a disk file
        //

        let wordlist = uplink_engine::archive::read_text("data/wordlist.txt")
            .expect("data/wordlist.txt is missing from the archive");
        for word in wordlist.split_whitespace() {
            world.create_password(word);
        }

        //
        // Load the gateway list from a disk file
        //

        // dos2unix: the C++ stream drops every '\r'
        let text = uplink_engine::archive::read_text("data/gatewaydefs.txt")
            .expect("data/gatewaydefs.txt is missing from the archive")
            .replace('\r', "");
        let mut thefile = Cursor::new(text.as_bytes());

        let mut unused = String::new();
        thefile
            .read_line(&mut unused)
            .expect("reading gatewaydefs.txt");
        let _ = read_word(&mut thefile);
        // PORT: C++ leaves numgateways uninitialised (garbage) when the token is not a number; 0 is used.
        let numgateways: i32 = read_word(&mut thefile).parse().unwrap_or(0);
        skip_whitespace(&mut thefile);

        for i in 0..numgateways {
            let mut def = GatewayDef::new();
            def.load_gateway_definition(&mut thefile);
            def.set_index(i);
            def.set_filename(&format!("gateway/gateway{}.tif", i));
            def.set_thumbnail(&format!("gateway/gateway_t{}.tif", i));
            world.create_gateway_def(def);
        }

        world
    }

    // ---- Database management functions ----

    /// `CreateVLocation(ip, x, y)`: warns on a duplicate ip.
    pub fn create_vlocation(&mut self, ip: &str, phys_x: i32, phys_y: i32) -> &mut VLocation {
        if self.locations.contains_key(ip) {
            eprintln!("Duplicate IP created : {}", ip);
        }
        let mut vl = VLocation::default();
        vl.set_ip(ip);
        vl.set_plocation(phys_x, phys_y);
        self.locations.put_data(ip, vl)
    }

    pub fn create_company(&mut self, name: &str) -> &mut Company {
        let mut company = Company::new();
        company.set_name(name);
        self.companies.put_data(name, company)
    }

    pub fn create_computer(&mut self, name: &str, companyname: &str, ip: &str) -> &mut Computer {
        let mut computer = Computer::new();
        computer.set_name(name);
        computer.set_company_name(companyname);
        computer.set_ip(ip);

        // UplinkAssert ( GetVLocation ( ip ) ); GetVLocation ( ip )->SetComputer ( name );
        // PORT: done before the PutData so that the new computer can be returned.
        self.locations
            .get_mut(ip)
            .expect("create computer: no VLocation at ip")
            .set_computer(name);

        self.computers.put_data(name, computer)
    }

    /// Creates a plain Person living at `localhost` (also its remote host).
    pub fn create_person(&mut self, name: &str, localhost: &str) -> &mut Person {
        let mut person = Person::default();
        person.set_name(name);
        person.set_local_host(localhost);
        person.set_remote_host(localhost);
        self.people.put_data(name, person)
    }

    /// `CreatePerson ( (Person *) player )` for the player: stores it in `self.player` and puts
    /// the "PLAYER" placeholder key into `people` where the C++ entry went.
    pub fn set_player(&mut self, player: Player) {
        self.people.put_placeholder(&player.person.name);
        self.player = Some(Box::new(player));
    }

    /// `people.ConvertToDArray ()`: every person in pre-order, with the player's Person at its
    /// "PLAYER" key (the C++ tree held the Player itself there).
    pub fn all_people(&self) -> Vec<(&str, &Person)> {
        self.people
            .iter_all()
            .filter_map(|(name, p)| match p {
                Some(p) => Some((name, p)),
                None if name == "PLAYER" => self.player.as_deref().map(|pl| (name, &pl.person)),
                None => None,
            })
            .collect()
    }

    pub fn create_password(&mut self, password: &str) {
        self.passwords.put_data(password.to_string());
    }

    /// `CreateVLocation(VLocation *)` (the overload; Rust has no overloading): stores it under its ip.
    pub fn create_vlocation_from(&mut self, vlocation: VLocation) {
        let ip = vlocation.ip.clone();
        self.locations.put_data(&ip, vlocation);
    }

    /// `CreateCompany(Company *)`: stores it under its name.
    pub fn create_company_from(&mut self, company: Company) {
        let name = company.name.clone();
        self.companies.put_data(&name, company);
    }

    /// `CreateComputer(Computer *)`: stores it under its name and points its VLocation at it.
    pub fn create_computer_from(&mut self, computer: Computer) {
        let name = computer.name.clone();
        let ip = computer.ip.clone();
        self.computers.put_data(&name, computer);
        debug_assert!(self.locations.contains_key(&ip));
        if let Some(vl) = self.locations.get_mut(&ip) {
            vl.set_computer(&name);
        }
    }

    /// `CreatePerson(Person *)`: stores it under its name (NPCs only; the player is `self.player`).
    pub fn create_person_from(&mut self, person: Person) {
        let name = person.name.clone();
        self.people.put_data(&name, person);
    }

    pub fn create_gateway_def(&mut self, newdef: GatewayDef) {
        self.gatewaydefs.put_data(newdef);
    }

    // ---- Lookups: None if the object is not in the database ----

    pub fn get_vlocation(&self, ip: &str) -> Option<&VLocation> {
        self.locations.get(ip)
    }

    pub fn get_vlocation_mut(&mut self, ip: &str) -> Option<&mut VLocation> {
        self.locations.get_mut(ip)
    }

    pub fn get_company(&self, name: &str) -> Option<&Company> {
        self.companies.get(name)
    }

    pub fn get_company_mut(&mut self, name: &str) -> Option<&mut Company> {
        self.companies.get_mut(name)
    }

    pub fn get_computer(&self, name: &str) -> Option<&Computer> {
        self.computers.get(name)
    }

    pub fn get_computer_mut(&mut self, name: &str) -> Option<&mut Computer> {
        self.computers.get_mut(name)
    }

    /// "PLAYER" and the player's agent handle both give the player's Person; otherwise an NPC by name.
    pub fn get_person(&self, name: &str) -> Option<&Person> {
        if self.is_player_name(name) {
            return self.player.as_deref().map(|p| &p.person);
        }
        self.people.get(name)
    }

    pub fn get_person_mut(&mut self, name: &str) -> Option<&mut Person> {
        if self.is_player_name(name) {
            return self.player.as_deref_mut().map(|p| &mut p.person);
        }
        self.people.get_mut(name)
    }

    pub fn get_password(&self, index: i32) -> Option<&str> {
        self.passwords.get_data(index).map(|s| s.as_str())
    }

    /// `index` != -1: that definition; otherwise the definition called `name`.
    pub fn get_gateway_def(&self, index: i32, name: Option<&str>) -> Option<&GatewayDef> {
        if index != -1 {
            return self.gatewaydefs.get_data(index);
        }

        if let Some(name) = name {
            for i in 0..self.gatewaydefs.size() {
                if let Some(def) = self.gatewaydefs.get_data(i) {
                    if def.name == name {
                        return Some(def);
                    }
                }
            }
        }

        None
    }

    /// Asserts that the player exists.
    pub fn get_player(&self) -> &Player {
        self.player
            .as_deref()
            .expect("World::get_player: no player")
    }

    pub fn get_player_mut(&mut self) -> &mut Player {
        self.player
            .as_deref_mut()
            .expect("World::get_player_mut: no player")
    }

    // PORT: C++ compared against the player's handle only when the "PLAYER" entry existed; an
    // empty handle (unset) never matches.
    fn is_player_name(&self, name: &str) -> bool {
        match self.player.as_deref() {
            Some(p) => {
                name == "PLAYER"
                    || (p.person.is_agent()
                        && !p.person.agent().handle.is_empty()
                        && name == p.person.agent().handle)
            }
            None => false,
        }
    }

    /// `World::Update`: advances `world.date`; every two game seconds updates the locations,
    /// companies, computers, people (and the player), the scheduler and the plot generator.
    // PORT: touches game/app, so it takes the context (`g.world_mut()` is the world).
    pub fn update(g: &mut Ctx) {
        Date::update(g);

        let nextupdate = g.world().nextupdate.clone();
        if g.world().date.after(&nextupdate) {
            // UpdateBTree(locations): VLocation has no Update of its own in C++, so nothing runs.

            // UpdateBTree(companies): pre-order, like every walk of a C++ BTree.
            // PORT: entries are looked up again by name, so of several entries with the same name
            // only the first is updated (once each time the name comes up).
            let company_names: Vec<String> = g.world().companies.keys_all();
            for name in company_names {
                if let Some(company) = g.world_mut().get_company_mut(&name) {
                    company.update();
                }
            }

            // UpdateBTree(computers)
            let computer_names: Vec<String> = g.world().computers.keys_all();
            for name in computer_names {
                if g.world().computers.contains_key(&name) {
                    Computer::update(g, &name);
                }
            }

            // UpdateBTree(people): the player is one of the entries (its placeholder key)
            let people_names: Vec<String> = g.world().people.keys_all();
            for name in people_names {
                if name == "PLAYER" && g.world().player.is_some() {
                    Player::update(g);
                } else if g.world().people.contains_key(&name) {
                    Person::update(g, &name);
                }
            }

            EventScheduler::update(g);
            g.world_mut().plotgenerator.update();

            if DEMOGAME {
                DemoPlotGenerator::update(g);
            }

            if WAREZRELEASE {
                let time_playing = g.ecl.get_accurate_time() - g.app.starttime;
                if time_playing > WAREZ_MAXPLAYTIME {
                    let mut rundate = Date::default();
                    rundate.set_date_from(&g.world().date);
                    rundate.advance_minute(TIME_TOWAREZGAMEOVER);

                    let mut ne = NotificationEvent::new();
                    ne.set_type(NOTIFICATIONEVENT_TYPE_WAREZGAMEOVER);
                    ne.base.set_run_date(&rundate);

                    g.world_mut()
                        .scheduler
                        .schedule_event(UplinkEvent::NotificationEvent(ne));
                }
            }

            // Calculate time of next update
            let date = g.world().date.clone();
            let world = g.world_mut();
            world.nextupdate.set_date_from(&date);
            world.nextupdate.advance_second(2);
        }
    }
}

// ---- helpers (private) ----

/// `>> word` on a stream: skips whitespace, then reads up to the next whitespace (left unread).
fn read_word(r: &mut dyn BufRead) -> String {
    skip_whitespace(r);
    let mut word = Vec::new();
    while let Some(b) = peek_byte(r) {
        if b.is_ascii_whitespace() {
            break;
        }
        word.push(b);
        r.consume(1);
    }
    String::from_utf8_lossy(&word).into_owned()
}

/// `>> ws`: skips whitespace.
fn skip_whitespace(r: &mut dyn BufRead) {
    while let Some(b) = peek_byte(r) {
        if !b.is_ascii_whitespace() {
            break;
        }
        r.consume(1);
    }
}

fn peek_byte(r: &mut dyn BufRead) -> Option<u8> {
    r.fill_buf().ok().and_then(|buf| buf.first().copied())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn people_walk_keeps_the_players_place() {
        let mut world = World::default();
        world.create_person("m", "1.1.1.1");
        world.create_person("Z", "1.1.1.1");
        let mut player = Player::new();
        player.set_name("PLAYER");
        world.set_player(player);
        world.create_person("a", "1.1.1.1");
        // "PLAYER" < "Z" < "m" (strcmp): m, Z, PLAYER, a
        let names: Vec<&str> = world.all_people().into_iter().map(|(k, _)| k).collect();
        assert_eq!(names, vec!["m", "Z", "PLAYER", "a"]);
        assert_eq!(world.people.keys_all(), vec!["m", "Z", "PLAYER", "a"]);
        assert_eq!(world.get_person("PLAYER").map(|p| p.name.as_str()), Some("PLAYER"));
    }
}
