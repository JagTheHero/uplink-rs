//! Port of source/world/person.h + source/world/person.cpp

use serde::{Deserialize, Serialize};

use crate::app::globals;
use crate::ctx::Ctx;
use crate::game::data::data::TIME_TOPAYLEGALFINE;
use crate::world::agent::Agent;
use crate::world::computer::bankaccount::BankAccount;
use crate::world::connection::Connection;
use crate::world::date::Date;
use crate::world::generator::demoplotgenerator::DemoPlotGenerator;
use crate::world::generator::missiongenerator::MissionGenerator;
use crate::world::generator::plotgenerator::PlotGenerator;
use crate::world::message::Message;
use crate::world::player::Player;
use crate::world::rating::Rating;
use crate::world::vlocation::VLocation;
use crate::world::world::World;

pub const PERSON_STATUS_NONE: i32 = 0;
pub const PERSON_STATUS_INJAIL: i32 = 1;
pub const PERSON_STATUS_DEAD: i32 = 2;

/// `sscanf(account, "%s %s", ip, accno)`: an account entry is "IP ACCNO".
fn split_account(account: &str) -> (&str, &str) {
    let mut parts = account.split_whitespace();
    (parts.next().unwrap_or(""), parts.next().unwrap_or(""))
}

/// True if `name` is the player's Person (C++ `this == game->GetWorld()->GetPlayer()`).
fn is_player_person(world: &World, name: &str) -> bool {
    matches!(world.get_person(name), Some(p) if p.name == "PLAYER")
}

/// A non-player-character in the Uplink world, including his physical behaviour, attributes,
/// skills etc. The player is also a Person (see `world::player::Player`).
// PORT: C++ Person <- Agent <- Player. `agent` holds the Agent-specific data (`Some` for agents
// and for the player); the Agent methods are `impl Person` in `world/agent.rs`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Person {
    pub name: String,
    pub age: i32,

    /// Which picture to use
    pub photoindex: i32,
    /// Which voice samples to use
    pub voiceindex: i32,

    /// Bit field - see PERSON_STATUS_*
    pub status: i32,

    pub localhost: String,
    pub remotehost: String,
    pub phonenumber: String,

    pub messages: Vec<Message>,
    /// List of "IP ACCNO"
    pub accounts: Vec<String>,
    /// Account for all current transfers
    pub currentaccount: i32,

    pub connection: Connection,
    pub rating: Rating,

    /// Can I be targeted by Random missions?
    pub istargetable: bool,

    /// Agent-specific data, `None` for a plain Person.
    pub agent: Option<Box<Agent>>,
}

impl Default for Person {
    fn default() -> Self {
        Person {
            name: " ".to_string(),
            age: -1,
            photoindex: 0,
            voiceindex: 0,
            status: PERSON_STATUS_NONE,
            localhost: " ".to_string(),
            remotehost: " ".to_string(),
            phonenumber: " ".to_string(),
            messages: Vec::new(),
            accounts: Vec::new(),
            currentaccount: 0,
            connection: Connection::default(),
            rating: Rating::default(),
            istargetable: true,
            agent: None,
        }
    }
}

impl Person {
    pub fn new() -> Self {
        Person::default()
    }

    /// Also sets the owner of the connection and the rating.
    pub fn set_name(&mut self, newname: &str) {
        self.name = newname.to_string();
        self.connection.set_owner(newname);
        self.rating.set_owner(newname);
    }

    pub fn set_age(&mut self, newage: i32) {
        self.age = newage;
    }

    pub fn set_photo_index(&mut self, newindex: i32) {
        self.photoindex = newindex;
    }

    pub fn set_voice_index(&mut self, newindex: i32) {
        self.voiceindex = newindex;
    }

    // PORT: the C++ asserted that the VLocation exists (via the world); dropped here because
    // a Person cannot reach the world. `World::create_person` checks it for new people.
    pub fn set_local_host(&mut self, newip: &str) {
        self.localhost = newip.to_string();
    }

    // PORT: see `set_local_host`.
    pub fn set_remote_host(&mut self, newip: &str) {
        self.remotehost = newip.to_string();
    }

    // PORT: see `set_local_host`.
    pub fn set_phone_number(&mut self, newphone: &str) {
        self.phonenumber = newphone.to_string();
    }

    pub fn set_status(&mut self, new_status: i32) {
        self.status = new_status;
    }

    /// `Person::GiveMessage` virtual: delivers `message` to the person called `name` with the
    /// behaviour of the real class (Person: `give_message_base`; Agent: `give_message_agent`;
    /// the player: `Player::give_message_player`).
    pub fn give_message(g: &mut Ctx, name: &str, message: Message) {
        // Virtual dispatch on the dynamic class: the player, then agents, then plain people.
        let (is_player, is_agent) = match g.world().get_person(name) {
            Some(p) => (p.name == "PLAYER", p.is_agent()),
            None => return,
        };
        if is_player {
            Player::give_message_player(g, message);
        } else if is_agent {
            Person::give_message_agent(g, name, message);
        } else if let Some(p) = g.world_mut().get_person_mut(name) {
            p.give_message_base(message);
        }
    }

    /// The base `Person::GiveMessage`: just stores it.
    pub fn give_message_base(&mut self, message: Message) {
        self.messages.push(message);
    }

    /// true if localhost != remotehost
    pub fn is_connected(&self) -> bool {
        self.localhost != self.remotehost
    }

    pub fn get_connection(&self) -> &Connection {
        &self.connection
    }

    pub fn get_connection_mut(&mut self) -> &mut Connection {
        &mut self.connection
    }

    pub fn get_local_host<'a>(&self, world: &'a World) -> Option<&'a VLocation> {
        world.get_vlocation(&self.localhost)
    }

    pub fn get_remote_host<'a>(&self, world: &'a World) -> Option<&'a VLocation> {
        world.get_vlocation(&self.remotehost)
    }

    /// `CreateNewAccount` virtual: creates a bank account at `bankip` for the person called
    /// `name` (Agents also remember the access code: `create_new_account_agent`). Returns the
    /// account number.
    pub fn create_new_account(
        g: &mut Ctx,
        name: &str,
        bankip: &str,
        accname: &str,
        password: &str,
        balance: i32,
        loan: i32,
    ) -> i32 {
        // Virtual dispatch: Agent (and so the Player) also remembers the access code.
        if matches!(g.world().get_person(name), Some(p) if p.is_agent()) {
            Person::create_new_account_agent(g, name, bankip, accname, password, balance, loan)
        } else {
            Person::create_new_account_base(g, name, bankip, accname, password, balance, loan)
        }
    }

    /// The base `Person::CreateNewAccount`.
    pub fn create_new_account_base(
        g: &mut Ctx,
        name: &str,
        bankip: &str,
        accname: &str,
        password: &str,
        balance: i32,
        loan: i32,
    ) -> i32 {
        // Get the bank in question
        let bankcomputer = g
            .world()
            .get_vlocation(bankip)
            .expect("Person::CreateNewAccount: bank VLocation not found")
            .computer
            .clone();

        // Add the account in
        let accountnumber = g
            .world_mut()
            .get_computer_mut(&bankcomputer)
            .expect("Person::CreateNewAccount: bank computer not found")
            .create_bank_account_new(accname, password, 3, balance + loan, loan);

        // Add it into the person
        let newaccount = format!("{} {}", bankip, accountnumber);
        g.world_mut()
            .get_person_mut(name)
            .expect("Person::CreateNewAccount: person not found")
            .accounts
            .push(newaccount);

        // Return the account number
        accountnumber
    }

    /// Overall bank balance
    // PORT: read-only bank account lookups through the world.
    pub fn get_balance(&self, world: &World) -> i32 {
        let mut result = 0;
        for account in &self.accounts {
            let (ip, accno) = split_account(account);
            let ba = BankAccount::get_account(world, ip, accno)
                .expect("Person::GetBalance: account not found");
            result += ba.balance;
        }
        result
    }

    /// Changes the balance of the current account of the person called `name`; an overdrawn
    /// player gets a PayFine mission.
    pub fn change_balance(g: &mut Ctx, name: &str, amount: i32, description: &str) {
        let (ip, accno) = {
            let Some(person) = g.world().get_person(name) else {
                return;
            };
            if person.accounts.is_empty() {
                return;
            }

            // Change the current account
            let fullcurrentaccount = usize::try_from(person.currentaccount)
                .ok()
                .and_then(|i| person.accounts.get(i))
                .expect("Person::ChangeBalance: no current account");
            let (ip, accno) = split_account(fullcurrentaccount);
            (ip.to_string(), accno.to_string())
        };

        BankAccount::change_balance(g, &ip, &accno, amount, Some(description));

        // If the person has gone overdrawn, fine them
        let balance = BankAccount::get_account(g.world(), &ip, &accno)
            .expect("Person::ChangeBalance: account not found")
            .balance;

        if amount < 0 && balance < 0 && is_player_person(g.world(), name) {
            let (compname, companyname) = {
                let world = g.world();
                let vl = world
                    .get_vlocation(&ip)
                    .expect("Person::ChangeBalance: VLocation not found");
                let comp = vl
                    .get_computer(world)
                    .expect("Person::ChangeBalance: no computer at VLocation");
                (comp.name.clone(), comp.companyname.clone())
            };
            g.world()
                .get_company(&companyname)
                .expect("Person::ChangeBalance: company not found");

            let reason = format!(
                "It has come to our attention that your bank account below has gone \
                 overdrawn, without authorisation.\n\n{}\nAccNo: {}\n",
                compname, accno
            );

            let mut date = Date::default();
            date.set_date_from(&g.world().date);
            date.advance_minute(TIME_TOPAYLEGALFINE);
            let (day, month, year) = (date.get_day(), date.get_month(), date.get_year());
            date.set_date(0, 0, 0, day, month, year); // Round to midnight

            let payfine = MissionGenerator::generate_pay_fine(
                g,
                name,
                &companyname,
                500,
                &date,
                &reason,
            );
            Person::give_mission(g, "PLAYER", payfine);
        }
    }

    pub fn set_current_account(&mut self, index: i32) {
        self.currentaccount = index;
    }

    pub fn set_is_targetable(&mut self, newvalue: bool) {
        self.istargetable = newvalue;
    }

    /// `Person::Update` (also the Agent version, which only calls it) for the person called
    /// `name`: handles the first queued message sent by the player. Not called for "PLAYER".
    pub fn update(g: &mut Ctx, name: &str) {
        // Not for the player (his own Person never reads mail here)
        let msg = {
            let Some(person) = g.world().get_person(name) else {
                return;
            };
            if person.name == "PLAYER" {
                return;
            }
            // This person reads his mail
            match person.messages.first() {
                Some(m) => m.clone(),
                None => return,
            }
        };

        if msg.from == "PLAYER" {
            // This is a message from the player - most likely a "completed mission" email
            // But it may be a mail to a "special" company
            // But then again it might just be for the company

            let mut handled = false;

            if msg.to == "internal@ARC.net" {
                handled = if globals::DEMOGAME {
                    DemoPlotGenerator::player_contacts_arc(g, &msg)
                } else {
                    PlotGenerator::player_contacts_arc(g, &msg)
                };
            } else if msg.to == "internal@Arunmor.net" {
                handled = PlotGenerator::player_contacts_arunmor(g, &msg);
            }

            if !handled {
                // Go through each of the players missions and see if this email satisfies any of them
                // (C++ removes entry i and then ++i, so the entry that slides into i is skipped)
                let mut i = 0;
                while i < g.player().agent().missions.len() {
                    let mut mis = g.player().agent().missions[i].clone();

                    if mis.contact == name
                        && msg.get_body().contains(mis.description.as_str())  // ie the name of the mission appears in your email
                    {
                        // PORT: the C++ checks the mission in place (it may lower the payment), so
                        // the checked copy is written back unless the mission is complete.
                        let complete = MissionGenerator::is_mission_complete(g, &mut mis, name, &msg);
                        let missions = &mut g.player_mut().agent_mut().missions;
                        if complete {
                            missions.remove(i);
                        } else if i < missions.len() {
                            missions[i] = mis;
                        }
                    }
                    i += 1;
                }
            }
        }
        // else: Message is not from the player

        // Finished dealing with this email
        if let Some(p) = g.world_mut().get_person_mut(name) {
            if !p.messages.is_empty() {
                p.messages.remove(0);
            }
        }
    }
}
