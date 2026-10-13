//! Port of source/world/generator/namegenerator.h + source/world/generator/namegenerator.cpp

use std::sync::{Mutex, MutexGuard};

use crate::ctx::Ctx;
use crate::world::computer::databank::{DATATYPE_DATA, DATATYPE_NONE, DATATYPE_PROGRAM};
use crate::world::generator::numbergenerator::NumberGenerator;

// PORT: the C++ keeps the name lists in class statics and hands results back through a shared
// static buffer (`tempname`). Here the lists are one private process-wide instance of
// `NameGenerator` behind a `Mutex` (so no `Ctx` is needed just to draw a name) and every
// generator returns an owned `String`.
static NAMES: Mutex<NameGenerator> = Mutex::new(NameGenerator::new());

#[derive(Debug, Default)]
pub struct NameGenerator {
    fornames: Vec<String>,
    surnames: Vec<String>,
    agentaliases: Vec<String>,
    companynames_a: Vec<String>,
    companynames_b: Vec<String>,
}

/// C: `fscanf ( file, "%s\n", name )` in a `while ( !feof ( file ) )` loop, then "Remove last entry".
// PORT: the C loop only exits once the last word has been read, because the trailing whitespace
// of the `\n` directive hits end-of-file. So the entries are every whitespace-separated word, and
// the "remove last entry" step drops the final word. Kept as in the C.
fn read_name_list(filename: &str) -> Vec<String> {
    let text = uplink_engine::archive::read_text(filename)
        .unwrap_or_else(|| panic!("NameGenerator::LoadNames, cannot open {}", filename));
    let mut names: Vec<String> = text.split_whitespace().map(|w| w.to_string()).collect();
    names.pop();
    names
}

impl NameGenerator {
    const fn new() -> Self {
        NameGenerator {
            fornames: Vec::new(),
            surnames: Vec::new(),
            agentaliases: Vec::new(),
            companynames_a: Vec::new(),
            companynames_b: Vec::new(),
        }
    }

    fn state() -> MutexGuard<'static, NameGenerator> {
        NAMES.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Loads data/fornames.txt, surnames.txt, agentaliases.txt, companya.txt, companyb.txt
    /// for every list that is currently empty.
    fn load_names(&mut self) {
        // Load fornames (only required once)
        if self.fornames.is_empty() {
            self.fornames = read_name_list("data/fornames.txt");
        }

        // Load surnames (required when all surnames have been used)
        if self.surnames.is_empty() {
            self.surnames = read_name_list("data/surnames.txt");
        }

        // Load Agent aliases
        if self.agentaliases.is_empty() {
            self.agentaliases = read_name_list("data/agentaliases.txt");
        }

        // Load company names (partA)
        // These may run out and will then need to be reloaded
        if self.companynames_a.is_empty() {
            self.companynames_a = read_name_list("data/companya.txt");
        }

        // Load company names (partB)
        if self.companynames_b.is_empty() {
            self.companynames_b = read_name_list("data/companyb.txt");
        }
    }

    pub fn initialise() {}

    pub fn shutdown() {
        let mut s = Self::state();
        s.fornames.clear();
        s.surnames.clear();
        s.agentaliases.clear();
        s.companynames_a.clear();
        s.companynames_b.clear();
    }

    pub fn generate_company_name() -> String {
        let mut s = Self::state();
        s.load_names();

        let size1 = s.companynames_a.len() as i32;
        let size2 = s.companynames_b.len() as i32;

        // Choose part 1 (primary name part1)
        let index1 = NumberGenerator::random_number(size1);

        // Choose part 2 (primary name part2)
        let index2 = NumberGenerator::random_number(size2);

        // Build the name
        let companynamea = s
            .companynames_a
            .get(index1 as usize)
            .cloned()
            .expect("NameGenerator::GenerateCompanyName, no company name part A");
        let companynameb = s
            .companynames_b
            .get(index2 as usize)
            .cloned()
            .expect("NameGenerator::GenerateCompanyName, no company name part B");

        // Part A is used up (part B is only read)
        s.companynames_a.remove(index1 as usize);

        format!("{} {}", companynamea, companynameb)
    }

    pub fn generate_person_name() -> String {
        let mut s = Self::state();
        s.load_names();

        let size_f = s.fornames.len() as i32;
        let size_s = s.surnames.len() as i32;

        let index_f = NumberGenerator::random_number(size_f);
        let index_s = NumberGenerator::random_number(size_s);

        // Build the name
        let forname = s
            .fornames
            .get(index_f as usize)
            .cloned()
            .expect("NameGenerator::GeneratePersonName, no forname");
        let surname = s
            .surnames
            .get(index_s as usize)
            .cloned()
            .expect("NameGenerator::GeneratePersonName, no surname");

        // Surnames are used up; fornames are only read
        s.surnames.remove(index_s as usize);

        format!("{} {}", forname, surname)
    }

    pub fn generate_agent_alias() -> String {
        let mut s = Self::state();
        s.load_names();

        let index = NumberGenerator::random_number(s.agentaliases.len() as i32);

        let result = s
            .agentaliases
            .get(index as usize)
            .cloned()
            .expect("NameGenerator::GenerateAgentAlias, no alias");
        s.agentaliases.remove(index as usize);

        result
    }

    pub fn generate_bank_name() -> String {
        Self::generate_company_name()
    }

    /// A random entry of `world.passwords`.
    pub fn generate_password(g: &Ctx) -> String {
        let passwords = &g.world().passwords;

        let numpasswords = passwords.size();

        let mut index = NumberGenerator::random_number(numpasswords);
        while !passwords.valid_index(index) {
            index = NumberGenerator::random_number(numpasswords);
        }

        passwords
            .get_data(index)
            .cloned()
            .expect("NameGenerator::GeneratePassword, no password")
    }

    /// Guessable
    pub fn generate_easy_password() -> String {
        match NumberGenerator::random_number(5) {
            0 => "god".to_string(),
            1 => "admin".to_string(),
            2 => "password".to_string(),
            3 => "love".to_string(),
            4 => "secure".to_string(),
            _ => panic!("You've fucked up"),
        }
    }

    /// Non dictionary hackable
    pub fn generate_complex_password(g: &Ctx) -> String {
        // Get a normal password
        // change up to half the letters

        // C: char password [9]; strcpy ( password, GeneratePassword () );
        // PORT: the 9-byte C buffer is modelled as a byte array. Passwords are at most 8 characters,
        // so the copy never overflows. Bytes past the terminator are zero here; the C left them
        // uninitialised (a letter written past the end is therefore dropped, not appended).
        let normal = Self::generate_password(g);
        let mut password = [0u8; 9];
        let bytes = normal.as_bytes();
        let n = bytes.len().min(8);
        password[..n].copy_from_slice(&bytes[..n]);

        let numchanges = NumberGenerator::random_number(5);

        for _ in 0..numchanges {
            let letternumber = NumberGenerator::random_number(8);
            let newcharacter = (NumberGenerator::random_number(26) + b'a' as i32) as u8;

            password[letternumber as usize] = newcharacter;
        }

        // strcpy ( tempname, password ): up to the first terminator
        let end = password.iter().position(|&b| b == 0).unwrap_or(password.len());
        String::from_utf8_lossy(&password[..end]).into_owned()
    }

    /// `datatype` is one of the `DATATYPE_*` constants.
    pub fn generate_data_name(companyname: &str, datatype: i32) -> String {
        // C: companyname [0], [1], [2]. A C string is NUL-terminated, so reading past its end gives 0.
        let cb = companyname.as_bytes();
        let mut dataname: Vec<u8> = (0..3).map(|i| cb.get(i).copied().unwrap_or(0)).collect();

        let suffix = match datatype {
            DATATYPE_NONE => format!("-file-{}", NumberGenerator::random_number(99999)),
            DATATYPE_DATA => format!("-data-{}", NumberGenerator::random_number(99999)),
            DATATYPE_PROGRAM => format!("-prog-{}", NumberGenerator::random_number(99999)),
            // PORT: the C switch has no default and leaves dataname uninitialised here; no name.
            _ => return String::new(),
        };
        dataname.extend_from_slice(suffix.as_bytes());

        // strcpy ( tempname, dataname ): up to the first terminator (a short company name gives a
        // NUL in the prefix, which ends the C string there)
        let end = dataname.iter().position(|&b| b == 0).unwrap_or(dataname.len());
        String::from_utf8_lossy(&dataname[..end]).into_owned()
    }

    pub fn generate_public_access_server_name(companyname: &str) -> String {
        format!("{} Public Access Server", companyname)
    }

    pub fn generate_access_terminal_name(companyname: &str) -> String {
        format!("{} Access Terminal", companyname)
    }

    pub fn generate_internal_services_server_name(companyname: &str) -> String {
        format!("{} Internal Services Machine", companyname)
    }

    pub fn generate_central_mainframe_name(companyname: &str) -> String {
        format!("{} Central Mainframe", companyname)
    }

    pub fn generate_international_bank_name(companyname: &str) -> String {
        format!("{} International Bank", companyname)
    }

    pub fn generate_file_server_name(companyname: &str) -> String {
        format!("{} File Server", companyname)
    }

    pub fn generate_lan_name(companyname: &str) -> String {
        format!("{} Local Area Network", companyname)
    }

    pub fn generate_personal_computer_name(personname: &str) -> String {
        format!("{}'s Personal Computer", personname)
    }

    pub fn generate_voice_phone_system_name(personname: &str) -> String {
        format!("{}'s Voice Phone System", personname)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{bare_ctx, mount_game_data};
    use crate::world::world::World;

    fn ctx_with_passwords(words: &[&str]) -> Ctx {
        let mut g = bare_ctx();
        let mut world = World::default();
        for w in words {
            world.create_password(w);
        }
        g.game.world = Some(Box::new(world));
        g
    }

    #[test]
    fn computer_names_are_the_company_plus_a_fixed_phrase() {
        assert_eq!(NameGenerator::generate_public_access_server_name("Acme"), "Acme Public Access Server");
        assert_eq!(NameGenerator::generate_access_terminal_name("Acme"), "Acme Access Terminal");
        assert_eq!(NameGenerator::generate_internal_services_server_name("Acme"), "Acme Internal Services Machine");
        assert_eq!(NameGenerator::generate_central_mainframe_name("Acme"), "Acme Central Mainframe");
        assert_eq!(NameGenerator::generate_international_bank_name("Acme"), "Acme International Bank");
        assert_eq!(NameGenerator::generate_file_server_name("Acme"), "Acme File Server");
        assert_eq!(NameGenerator::generate_lan_name("Acme"), "Acme Local Area Network");
        assert_eq!(NameGenerator::generate_personal_computer_name("Jo Bloggs"), "Jo Bloggs's Personal Computer");
        assert_eq!(NameGenerator::generate_voice_phone_system_name("Jo Bloggs"), "Jo Bloggs's Voice Phone System");
    }

    #[test]
    fn data_names_have_the_company_prefix_kind_and_a_number() {
        for _ in 0..50 {
            let name = NameGenerator::generate_data_name("Acme Corp", DATATYPE_DATA);
            let number = name.strip_prefix("Acm-data-").unwrap_or_else(|| panic!("{name}"));
            assert!(number.parse::<i32>().unwrap() < 99999);
        }
        assert!(NameGenerator::generate_data_name("Acme", DATATYPE_PROGRAM).starts_with("Acm-prog-"));
        assert!(NameGenerator::generate_data_name("Acme", DATATYPE_NONE).starts_with("Acm-file-"));
        assert_eq!(NameGenerator::generate_data_name("Acme", 99), "");
        // a company name shorter than three letters ends the C string early
        assert_eq!(NameGenerator::generate_data_name("Ab", DATATYPE_DATA), "Ab");
    }

    #[test]
    fn easy_passwords_come_from_the_fixed_list() {
        for _ in 0..200 {
            let p = NameGenerator::generate_easy_password();
            assert!(["god", "admin", "password", "love", "secure"].contains(&p.as_str()), "{p}");
        }
    }

    #[test]
    fn passwords_come_from_the_world_list() {
        let g = ctx_with_passwords(&["alpha", "bravo", "charlie"]);
        for _ in 0..100 {
            let p = NameGenerator::generate_password(&g);
            assert!(["alpha", "bravo", "charlie"].contains(&p.as_str()), "{p}");
        }
    }

    #[test]
    fn complex_passwords_are_short_and_lowercase() {
        let g = ctx_with_passwords(&["abcdefgh", "zyxwvuts"]);
        for _ in 0..300 {
            let p = NameGenerator::generate_complex_password(&g);
            assert!(p.len() <= 8, "{p}");
            assert!(p.bytes().all(|b| b.is_ascii_lowercase()), "{p}");
        }
    }

    #[test]
    fn person_company_and_alias_names_from_the_game_data() {
        if !mount_game_data() {
            return;
        }
        for _ in 0..20 {
            let person = NameGenerator::generate_person_name();
            let words: Vec<&str> = person.split(' ').collect();
            assert_eq!(words.len(), 2, "{person}");
            assert!(words.iter().all(|w| !w.is_empty()));

            let company = NameGenerator::generate_company_name();
            assert_eq!(company.split(' ').count(), 2, "{company}");

            let alias = NameGenerator::generate_agent_alias();
            assert!(!alias.is_empty() && !alias.contains(' '), "{alias}");
        }
        assert_eq!(NameGenerator::generate_bank_name().split(' ').count(), 2);
    }
}
