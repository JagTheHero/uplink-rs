//! Port of source/world/generator/langenerator.h + source/world/generator/langenerator.cpp

use uplink_engine::archive;

use crate::app::globals::TESTGAME;
use crate::ctx::Ctx;
use crate::game::data::data::{
    COMPANYALIGNMENT_AVERAGE, COMPANYALIGNMENT_RANGE, COMPANYGROWTH_AVERAGE, COMPANYGROWTH_RANGE,
    COMPANYSIZE_AVERAGE, COMPANYSIZE_RANGE, HACKDIFFICULTY_GLOBALCRIMINALDATABASE,
    HACKDIFFICULTY_LANAUTHENTICATIONSERVER, HACKDIFFICULTY_LANLOGSERVER,
    HACKDIFFICULTY_LANMAINSERVER, HACKDIFFICULTY_LANMODEM, HACKDIFFICULTY_LANTERMINAL,
    LAN_LINKPORTRANGE, LAN_SUBNETRANGE, RADIOTRANSMITTER_MAXRANGE, RADIOTRANSMITTER_MINRANGE,
    TRACESPEED_LAN,
};
use crate::interface::remoteinterface::remoteinterface::{
    SCREEN_CONSOLESCREEN, SCREEN_FILESERVERSCREEN, SCREEN_RADIOTRANSMITTER, SCREEN_VOICEANALYSIS,
};
use crate::tosser::DArray;
use crate::world::company::company::COMPANYTYPE_COMMERCIAL;
use crate::world::computer::computer::{
    Computer, COMPUTER_TRACEACTION_DISCONNECT, COMPUTER_TRACEACTION_LEGAL, COMPUTER_TYPE_LAN,
};
use crate::world::computer::computerscreen::cypherscreen::CypherScreen;
use crate::world::computer::computerscreen::computerscreen::ComputerScreen;
use crate::world::computer::computerscreen::dialogscreen::{
    DialogScreen, WIDGET_CAPTION, WIDGET_SCRIPTBUTTON,
};
use crate::world::computer::computerscreen::genericscreen::GenericScreen;
use crate::world::computer::computerscreen::highsecurityscreen::HighSecurityScreen;
use crate::world::computer::computerscreen::logscreen::{LogScreen, LOGSCREEN_TARGET_ACCESSLOGS};
use crate::world::computer::computerscreen::menuscreen::MenuScreen;
use crate::world::computer::computerscreen::messagescreen::MessageScreen;
use crate::world::computer::computerscreen::passwordscreen::PasswordScreen;
use crate::world::computer::databank::{Data, SOFTWARETYPE_NONE};
use crate::world::computer::lancomputer::{
    LanComputerLink, LanComputerSystem, LANSYSTEMVISIBLE_TYPE, LANSYSTEM_AUTHENTICATION,
    LANSYSTEM_FAXPRINTER, LANSYSTEM_HUB, LANSYSTEM_ISOLATIONBRIDGE, LANSYSTEM_LOCK,
    LANSYSTEM_LOGSERVER, LANSYSTEM_MAINSERVER, LANSYSTEM_MODEM, LANSYSTEM_RADIORECEIVER,
    LANSYSTEM_RADIOTRANSMITTER, LANSYSTEM_ROUTER, LANSYSTEM_SESSIONKEYSERVER,
    LANSYSTEM_TERMINAL,
};
use crate::world::computer::logbank::{AccessLog, LOG_NOTSUSPICIOUS, LOG_TYPE_TEXT};
use crate::world::computer::recordbank::{
    Record, RECORDBANK_ADMIN, RECORDBANK_NAME, RECORDBANK_PASSWORD, RECORDBANK_SECURITY,
};
use crate::world::computer::securitysystem::{SECURITY_TYPE_MONITOR, SECURITY_TYPE_PROXY};
use crate::world::generator::namegenerator::NameGenerator;
use crate::world::generator::numbergenerator::NumberGenerator;
use crate::world::generator::worldgenerator::WorldGenerator;
use crate::world::vlocation::VLocation;

// PORT: the C++ takes `LanComputer *comp` everywhere. In Rust the LAN-specific data lives in
// `Computer::kind == ComputerKind::LanComputer(..)` (PORTING.md §3.3) and the screens/security/
// databank live on `Computer`, so the functions that need the world (they all do, directly or via
// NameGenerator::generate_password / WorldGenerator) take `g: &mut Ctx` plus the computer's name
// (`comp_name`) and look the computer up with `g.world_mut().get_computer_mut(comp_name)`.
// Clusters are temporary objects that are not part of the world, so they are passed as `&mut`.
//
// PORT: LanGenerator::GenerateIsolationTest / GenerateRadioTest are declared in langenerator.h but
// never defined in the C++ (calling them would not link), so they are not ported.

/*
    Alignment values:       -1 = Left aligned
                            0  = Centre aligned
                            +1 = Right aligned
*/

pub struct LanGenerator;

impl LanGenerator {
    pub fn initialise() {}

    //
    // Top level functions for generating entire networks

    /// `LoadLAN`: returns the name of the new LAN computer, or `None` if it was already loaded
    /// (or the file is empty).
    pub fn load_lan(g: &mut Ctx, filename: &str) -> Option<String> {
        let text = archive::read_text(filename).unwrap_or_default().replace("\r\n", "\n");
        if text.is_empty() {
            return None;
        }
        let mut file = Cursor::new(&text);

        //
        // Load the header information

        file.token();
        file.ws();
        let computer_name = file.getline().unwrap_or_default().to_string();

        file.token();
        file.ws();
        let company_name = file.getline().unwrap_or_default().to_string();

        file.token();
        file.ws();
        let ip = file.getline().unwrap_or_default().to_string();

        let x = scan_keyword_int(file.getline().unwrap_or_default(), "XPOS");
        let y = scan_keyword_int(file.getline().unwrap_or_default(), "YPOS");

        print!("Loading LAN from {}...", filename);

        if g.world().get_vlocation(&ip).is_some() {
            println!("[Already Loaded]");
            return None;
        }

        //
        // Create blank LAN

        g.world_mut().create_vlocation(&ip, x, y);
        //	game->GetWorld()->CreateCompany( companyName );
        let size = NumberGenerator::random_normal_number(
            COMPANYSIZE_AVERAGE as f32,
            COMPANYSIZE_RANGE as f32,
        ) as i32;
        let growth = NumberGenerator::random_normal_number(
            COMPANYGROWTH_AVERAGE as f32,
            COMPANYGROWTH_RANGE as f32,
        ) as i32;
        let alignment = NumberGenerator::random_normal_number(
            COMPANYALIGNMENT_AVERAGE as f32,
            COMPANYALIGNMENT_RANGE as f32,
        ) as i32;
        let company = WorldGenerator::generate_company_named(
            g,
            &company_name,
            size,
            COMPANYTYPE_COMMERCIAL,
            growth,
            alignment,
        );

        let ceo = WorldGenerator::get_random_person(g);
        let admin = WorldGenerator::get_random_person(g);
        let company_obj = g.world_mut().get_company_mut(&company).expect("generated company");
        company_obj.set_boss(&ceo);
        company_obj.set_admin(&admin);

        let mut comp = Computer::new_lan_computer();
        comp.set_type(COMPUTER_TYPE_LAN);
        comp.set_trace_speed(TRACESPEED_LAN);
        comp.set_trace_action(COMPUTER_TRACEACTION_DISCONNECT | COMPUTER_TRACEACTION_LEGAL);
        comp.security.add_system_new(SECURITY_TYPE_PROXY, 5, -1);
        comp.security.add_system_new(SECURITY_TYPE_MONITOR, 5, -1);
        comp.set_name(&computer_name);
        comp.set_company_name(&company_name);
        comp.set_ip(&ip);
        comp.set_is_targetable(false);
        g.world_mut().create_computer_from(comp);

        let mut cluster = LanCluster::new();

        //
        // Load each LAN system

        while let Some(full_line) = file.getline() {
            if full_line.starts_with('.') {
                break;
            }
            if full_line.len() < 2 {
                continue;
            }
            if full_line.starts_with(';') {
                continue;
            }

            let mut line = Cursor::new(full_line);
            line.token(); // the C++ reads `index` here but never uses it
            let system_name = line.token();
            let x = line.int();
            let y = line.int();
            let security = line.int();
            let data1 = line.int();
            let data2 = line.int();
            let data3 = line.int();
            line.ws();

            let mut data_x = String::new();
            if system_name.contains("MODEM") {
                data_x = line.rest().to_string();
                if data_x.len() < 3 {
                    panic!(
                        "Failed to load {}. Modem found without phone number.",
                        filename
                    );
                }
            }

            let index_used = if system_name.contains("ROUTER") {
                Self::generate_router(g, &computer_name, &mut cluster, x, y, security)
            } else if system_name.contains("HUB") {
                Self::generate_hub(g, &computer_name, &mut cluster, x, y, security)
            } else if system_name.contains("TERMINAL") {
                Self::generate_terminal(g, &computer_name, &mut cluster, x, y, security)
            } else if system_name.contains("MAINSERVER") {
                Self::generate_main_server(g, &computer_name, &mut cluster, x, y, security)
            } else if system_name.contains("AUTHENTICATION") {
                Self::generate_authentication(g, &computer_name, &mut cluster, x, y, security)
            } else if system_name.contains("LOCK") {
                Self::generate_lock(g, &computer_name, &mut cluster, x, y, security)
            } else if system_name.contains("ISOLATIONBRIDGE") {
                Self::generate_isolation_bridge(g, &computer_name, &mut cluster, x, y, security)
            } else if system_name.contains("SESSIONKEYSERVER") {
                Self::generate_session_key_server(g, &computer_name, &mut cluster, x, y, security)
            } else if system_name.contains("RADIOTRANSMITTER") {
                Self::generate_radio_transmitter(g, &computer_name, &mut cluster, x, y, security)
            } else if system_name.contains("RADIORECEIVER") {
                Self::generate_radio_receiver(g, &computer_name, &mut cluster, x, y, security)
            } else if system_name.contains("FAXPRINTER") {
                Self::generate_fax_printer(g, &computer_name, &mut cluster, x, y, security)
            } else if system_name.contains("MODEM") {
                Self::generate_modem(
                    g,
                    &computer_name,
                    &mut cluster,
                    x,
                    y,
                    security,
                    Some(&mut data_x),
                )
            } else if system_name.contains("LOGSERVER") {
                Self::generate_log_server(g, &computer_name, &mut cluster, x, y, security)
            } else {
                print!(
                    "LanGenerator::LoadLAN WARNING: Unrecognised System TYPE {}\n({})\n",
                    system_name, filename
                );
                -1
            };

            if index_used != -1 {
                if let Some(system) = cluster.systems.get_data_mut(index_used) {
                    system.data1 = data1;
                    system.data2 = data2;
                    system.data3 = data3;
                }
            }
        }

        //
        // Load all links

        while let Some(full_line) = file.getline() {
            if full_line.starts_with('.') {
                break;
            }
            if full_line.len() < 2 {
                continue;
            }
            if full_line.starts_with(';') {
                continue;
            }

            let mut line = Cursor::new(full_line);
            line.token(); // linkName
            let from = line.int();
            let from_x = line.float();
            let from_y = line.float();
            let to = line.int();
            let to_x = line.float();
            let to_y = line.float();
            let security = line.int();

            cluster.add_lan_link(from, from_x, from_y, to, to_x, to_y, security);
        }

        //
        // Load Valid Subnets

        while let Some(full_line) = file.getline() {
            if full_line.starts_with('.') {
                break;
            }
            if full_line.len() < 2 {
                continue;
            }
            if full_line.starts_with(';') {
                continue;
            }

            let mut line = Cursor::new(full_line);
            line.token(); // name
            let system_index = line.int();
            let valid_index = line.int();

            if let Some(system) = cluster.systems.get_data_mut(system_index) {
                system.valid_subnets.push(valid_index);
            }
        }

        //
        // Merge all of those systems into place

        cluster.merge(g, &computer_name);

        //
        // Load hidden data values

        while let Some(full_line) = file.getline() {
            if full_line.starts_with('.') {
                break;
            }
            if full_line.len() < 2 {
                continue;
            }
            if full_line.starts_with(';') {
                continue;
            }

            let mut line = Cursor::new(full_line);
            line.token(); // dataName
            let index = line.int();
            line.ws();
            let all_data = line.rest();

            let comp = lan_computer_mut(g, &computer_name);
            Self::hide_data(comp, index, all_data);
        }

        //
        // Create the files, logs and login

        Self::generate_lan_contents(g, &computer_name, &company_name);

        println!("done");

        Some(computer_name)
    }

    /// `GenerateLAN`: difficulty 0 = low, 5 = high. Returns the name of the new LAN computer.
    pub fn generate_lan(g: &mut Ctx, companyname: &str, difficulty: i32) -> String {
        //
        // Generate the location and computer

        let computername = NameGenerator::generate_lan_name(companyname);
        let ip = WorldGenerator::generate_location(g);
        g.world_mut()
            .get_vlocation_mut(&ip)
            .expect("generated location")
            .set_listed(false);

        let mut comp = Computer::new_lan_computer();
        comp.set_type(COMPUTER_TYPE_LAN);
        comp.set_name(&computername);
        comp.set_company_name(companyname);
        comp.set_trace_speed(TRACESPEED_LAN);
        comp.set_trace_action(COMPUTER_TRACEACTION_DISCONNECT | COMPUTER_TRACEACTION_LEGAL);
        comp.security.add_system_new(SECURITY_TYPE_PROXY, 5, -1);
        comp.security.add_system_new(SECURITY_TYPE_MONITOR, 5, -1);
        comp.set_ip(&ip);
        g.world_mut().create_computer_from(comp);

        //
        // Generate the LAN cluster

        Self::generate_lan_cluster(g, &computername, difficulty);

        //
        // Create the files, logs and login

        Self::generate_lan_contents(g, &computername, companyname);

        computername
    }

    pub fn generate_lan_cluster(g: &mut Ctx, comp_name: &str, difficulty: i32) {
        let mut cluster = LanCluster::new();

        match difficulty {
            0 => Self::generate_lan_level0_cluster(g, comp_name, &mut cluster, 10, 100, 200, 200),
            1 => Self::generate_lan_level1_cluster(g, comp_name, &mut cluster, 10, 100, 275, 275),
            2 => Self::generate_lan_level2_cluster(g, comp_name, &mut cluster),
            3 => Self::generate_lan_level3_cluster(g, comp_name, &mut cluster),
            4 => Self::generate_lan_level4_cluster(g, comp_name, &mut cluster),
            _ => {}
        }

        let input = sys(&cluster, cluster.input).clone();
        let output = sys(&cluster, cluster.output).clone();

        let router_index = Self::generate_router(
            g,
            comp_name,
            &mut cluster,
            input.x - 70,
            input.y - 100,
            1,
        );
        cluster.add_lan_link(router_index, 0.5, 1.0, cluster.input, 0.5, 0.0, 1);

        if difficulty == 0 || difficulty == 1 {
            let main_server_index = Self::generate_main_server(
                g,
                comp_name,
                &mut cluster,
                output.x + 70,
                output.y + 120,
                1,
            );
            cluster.add_lan_link(cluster.output, 0.5, 1.0, main_server_index, 0.5, 0.0, 1);
        }

        let rotate_by = NumberGenerator::random_number(4);
        cluster.rotate(rotate_by);
        cluster.merge(g, comp_name);
    }

    /// Prints an error for every high security screen that points at a menu screen.
    pub fn sanity_check_lan(comp: &Computer) {
        //
        // Sanity check those screens

        for i in 0..comp.screens.size() {
            if comp.screens.valid_index(i) {
                let cs = comp.screens.get_data(i).expect("valid screen index");

                if let ComputerScreen::HighSecurityScreen(hss) = cs {
                    for mso in &hss.systems {
                        if comp.screens.valid_index(mso.nextpage) {
                            let next_page = comp
                                .screens
                                .get_data(mso.nextpage)
                                .expect("valid screen index");
                            if let ComputerScreen::MenuScreen(_) = next_page {
                                print!(
                                    "ERROR Screen {} Points to {} at {}",
                                    i, mso.nextpage, comp.companyname
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    //
    // Mid level functions for generating clusters of computers

    pub fn generate_lan_level0_cluster(
        g: &mut Ctx,
        comp_name: &str,
        cluster: &mut LanCluster,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) {
        cluster.centre_x = x + width / 2;
        cluster.centre_y = y + height / 2;

        let type_ = NumberGenerator::random_number(7) + 1;

        if type_ == 1 {
            let hub_index = Self::generate_hub(g, comp_name, cluster, x, y, 1);
            let t1_index = Self::generate_terminal(g, comp_name, cluster, x + width / 4 * 0, y + 100, 1);
            let t2_index = Self::generate_terminal(g, comp_name, cluster, x + width / 4 * 1, y + 100, 1);
            let t3_index = Self::generate_terminal(g, comp_name, cluster, x + width / 4 * 2, y + 100, 1);
            let t4_index = Self::generate_terminal(g, comp_name, cluster, x + width / 4 * 3, y + 100, 1);

            cluster.add_lan_link(hub_index, 0.2, 1.0, t1_index, 0.5, 0.0, 1);
            cluster.add_lan_link(hub_index, 0.4, 1.0, t2_index, 0.5, 0.0, 1);
            cluster.add_lan_link(hub_index, 0.6, 1.0, t3_index, 0.5, 0.0, 1);
            cluster.add_lan_link(hub_index, 0.8, 1.0, t4_index, 0.5, 0.0, 1);

            cluster.set_input(hub_index);

            match NumberGenerator::random_number(4) {
                0 => cluster.set_output(t1_index),
                1 => cluster.set_output(t2_index),
                2 => cluster.set_output(t3_index),
                3 => cluster.set_output(t4_index),
                _ => {}
            }
        } else if type_ == 2 {
            let hub_index = Self::generate_hub(g, comp_name, cluster, x + width / 2, y, 1);
            let lock_index = Self::generate_lock(g, comp_name, cluster, x + width / 2 - 60, y + height / 2, 2);
            let auth_index = Self::generate_authentication(g, comp_name, cluster, x + width - 60, y + height / 2, 2);

            sys_mut(cluster, auth_index).data1 = lock_index;

            cluster.add_lan_link(hub_index, 0.5, 1.0, lock_index, 0.5, 0.0, 1);
            cluster.add_lan_link(hub_index, 1.0, 0.5, auth_index, 0.5, 0.0, 1);

            cluster.set_input(hub_index);
            cluster.set_output(lock_index);
        } else if type_ == 3 {
            let hub_index = Self::generate_hub(g, comp_name, cluster, x, y, 1);
            let lock_index = Self::generate_lock(g, comp_name, cluster, x + width - 20, y + height - 40, 2);
            let auth_index = Self::generate_authentication(g, comp_name, cluster, x, y + height - 60, 2);

            sys_mut(cluster, auth_index).data1 = lock_index;

            cluster.add_lan_link(hub_index, 1.0, 0.5, lock_index, 0.5, 0.0, 1);
            cluster.add_lan_link(lock_index, 0.0, 0.5, auth_index, 1.0, 0.5, 1);
            cluster.add_lan_link(hub_index, 0.5, 1.0, auth_index, 0.5, 0.0, 1);

            cluster.set_input(hub_index);
            cluster.set_output(lock_index);
        } else if type_ == 4 {
            let hub_index = Self::generate_hub(g, comp_name, cluster, x + width / 2, y, 1);
            let lock1_index = Self::generate_lock(g, comp_name, cluster, x + width - 20, y + height / 2, 1);
            let lock2_index = Self::generate_lock(g, comp_name, cluster, x + width / 2, y + height - 20, 2);
            let isol_index = Self::generate_isolation_bridge(g, comp_name, cluster, x + width - 13, y + height - 27, 2);
            let term_index = Self::generate_terminal(g, comp_name, cluster, x, y + height / 2, 1);

            let isol = sys_mut(cluster, isol_index);
            isol.data1 = lock1_index;
            isol.data2 = lock2_index;

            sys_mut(cluster, lock1_index).data1 = 0;

            cluster.add_lan_link(hub_index, 1.0, 0.5, lock1_index, 0.5, 0.0, 1);
            cluster.add_lan_link(lock1_index, 0.5, 1.0, isol_index, 0.5, 0.0, 1);
            cluster.add_lan_link(hub_index, 0.0, 0.5, term_index, 0.5, 0.0, 1);
            cluster.add_lan_link(term_index, 0.5, 1.0, lock2_index, 0.0, 0.5, 1);
            cluster.add_lan_link(lock2_index, 1.0, 0.5, isol_index, 0.0, 0.5, 1);

            cluster.set_input(hub_index);
            cluster.set_output(lock2_index);
        } else if type_ == 5 {
            let hub_index = Self::generate_hub(g, comp_name, cluster, x, y, 1);
            let auth_index = Self::generate_authentication(g, comp_name, cluster, x + width / 2, y, 2);
            let lock_index = Self::generate_lock(g, comp_name, cluster, x + width - 20, y, 2);
            let term_index = Self::generate_terminal(g, comp_name, cluster, x, y + height - 40, 1);

            sys_mut(cluster, auth_index).data1 = lock_index;

            cluster.add_lan_link(hub_index, 1.0, 0.5, auth_index, 0.0, 0.5, 1);
            cluster.add_lan_link(auth_index, 1.0, 0.5, lock_index, 0.0, 0.5, 1);
            cluster.add_lan_link(hub_index, 0.5, 1.0, term_index, 0.5, 0.0, 1);

            cluster.set_input(hub_index);
            cluster.set_output(lock_index);
        } else if type_ == 6 {
            let hub_index = Self::generate_hub(g, comp_name, cluster, x + width / 2, y, 1);
            let t1_index = Self::generate_terminal(g, comp_name, cluster, x, y + height / 3 * 1, 1);
            let t2_index = Self::generate_terminal(g, comp_name, cluster, x, y + height / 3 * 2, 1);
            let t3_index = Self::generate_terminal(g, comp_name, cluster, x, y + height / 3 * 3, 1);
            let term_index = Self::generate_terminal(g, comp_name, cluster, x + width - 40, y + height / 2, 1);

            let terminal = sys_mut(cluster, term_index);
            terminal.valid_subnets.push(t1_index);
            terminal.valid_subnets.push(t2_index);
            terminal.valid_subnets.push(t3_index);

            cluster.add_lan_link(hub_index, 0.25, 1.0, t1_index, 1.0, 0.5, 1);
            cluster.add_lan_link(hub_index, 0.5, 1.0, t2_index, 1.0, 0.5, 1);
            cluster.add_lan_link(hub_index, 0.75, 1.0, t3_index, 1.0, 0.5, 1);
            cluster.add_lan_link(hub_index, 1.0, 0.5, term_index, 0.5, 0.0, 1);

            cluster.set_input(hub_index);
            cluster.set_output(term_index);
        } else if type_ == 7 {
            let lock1_index = Self::generate_lock(g, comp_name, cluster, x, y, 1);
            let isol_index = Self::generate_isolation_bridge(g, comp_name, cluster, x, y + height / 2, 2);
            let lock2_index = Self::generate_lock(g, comp_name, cluster, x + width - 20, y + height / 2, 2);
            let modem_index = Self::generate_modem(g, comp_name, cluster, x + width / 2, y + height - 40, 2, None);

            let isol = sys_mut(cluster, isol_index);
            isol.data1 = lock1_index;
            isol.data2 = lock2_index;

            sys_mut(cluster, lock1_index).data1 = 0;

            cluster.add_lan_link(lock1_index, 0.5, 1.0, isol_index, 0.5, 0.0, 1);
            cluster.add_lan_link(isol_index, 1.0, 0.5, lock2_index, 0.0, 0.5, 1);
            cluster.add_lan_link(isol_index, 0.5, 1.0, modem_index, 0.0, 0.5, 1);

            cluster.set_input(lock1_index);
            cluster.set_output(lock2_index);
        }
    }

    pub fn generate_lan_level1_cluster(
        g: &mut Ctx,
        comp_name: &str,
        cluster: &mut LanCluster,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) {
        cluster.centre_x = x + width / 2;
        cluster.centre_y = y + height / 2;

        let type_ = NumberGenerator::random_number(6) + 1;

        let x1 = x + width / 4 * 1;
        let x2 = x + width / 4 * 2;
        let x3 = x + width / 4 * 3;

        let y1 = y + height / 4 * 1;
        let y2 = y + height / 4 * 2;
        let y3 = y + height / 4 * 3;

        if type_ == 1 {
            let hub1_index = Self::generate_hub(g, comp_name, cluster, x2, y, 1);
            let hub2_index = Self::generate_hub(g, comp_name, cluster, x, y2, 1);
            let auth_index = Self::generate_authentication(g, comp_name, cluster, x2, y + height - 60, 2);
            let lock_index = Self::generate_lock(g, comp_name, cluster, x + width - 20, y + height - 60, 2);

            let t1_index = Self::generate_terminal(g, comp_name, cluster, x2, y2 - 70, 1);
            let t2_index = Self::generate_terminal(g, comp_name, cluster, x2, y2 - 20, 1);
            let t3_index = Self::generate_terminal(g, comp_name, cluster, x2, y2 + 30, 1);

            let auth = sys_mut(cluster, auth_index);
            auth.valid_subnets.push(t1_index);
            auth.valid_subnets.push(t2_index);
            auth.valid_subnets.push(t3_index);
            auth.data1 = lock_index;

            cluster.add_lan_link(hub1_index, 0.0, 0.5, hub2_index, 0.5, 0.0, 1);
            cluster.add_lan_link(hub2_index, 0.5, 1.0, auth_index, 0.0, 0.5, 1);
            cluster.add_lan_link(hub1_index, 1.0, 0.5, lock_index, 0.5, 0.0, 1);
            cluster.add_lan_link(lock_index, 0.0, 0.5, auth_index, 1.0, 0.5, 1);

            cluster.add_lan_link(hub2_index, 1.0, 0.25, t1_index, 0.0, 0.5, 1);
            cluster.add_lan_link(hub2_index, 1.0, 0.5, t2_index, 0.0, 0.5, 1);
            cluster.add_lan_link(hub2_index, 1.0, 0.75, t3_index, 0.0, 0.5, 1);

            cluster.set_input(hub1_index);
            cluster.set_output(lock_index);
        } else if type_ == 2 {
            let hub_index = Self::generate_hub(g, comp_name, cluster, x2, y, 1);
            let lock1_index = Self::generate_lock(g, comp_name, cluster, x, y2, 1);
            let auth_index = Self::generate_authentication(g, comp_name, cluster, x2, y2, 2);
            let lock2_index = Self::generate_lock(g, comp_name, cluster, x + width - 20, y + height - 40, 1);
            let iso_index = Self::generate_isolation_bridge(g, comp_name, cluster, x2, y + height - 40, 2);

            sys_mut(cluster, auth_index).data1 = lock1_index;

            let iso = sys_mut(cluster, iso_index);
            iso.data1 = lock1_index;
            iso.data2 = lock2_index;

            cluster.add_lan_link(hub_index, 0.2, 1.0, lock1_index, 0.5, 0.0, 1);
            cluster.add_lan_link(hub_index, 0.5, 1.0, auth_index, 0.5, 0.0, 1);
            cluster.add_lan_link(lock1_index, 0.5, 1.0, iso_index, 0.0, 0.5, 1);
            cluster.add_lan_link(hub_index, 1.0, 0.5, lock2_index, 0.5, 0.0, 1);
            cluster.add_lan_link(lock2_index, 0.0, 0.5, iso_index, 1.0, 0.5, 1);

            cluster.set_input(hub_index);
            cluster.set_output(lock2_index);
        } else if type_ == 3 {
            let hub_index = Self::generate_hub(g, comp_name, cluster, x, y1, 1);
            let auth1_index = Self::generate_authentication(g, comp_name, cluster, x1, y1, 1);
            let lock1_index = Self::generate_lock(g, comp_name, cluster, x2, y1, 1);
            let term1_index = Self::generate_terminal(g, comp_name, cluster, x3, y1, 1);
            let term2_index = Self::generate_terminal(g, comp_name, cluster, x2, y, 1);
            let auth2_index = Self::generate_authentication(g, comp_name, cluster, x2, y2, 2);
            let lock2_index = Self::generate_lock(g, comp_name, cluster, x2, y3, 2);
            let term3_index = Self::generate_terminal(g, comp_name, cluster, x1, y3, 2);

            sys_mut(cluster, auth1_index).data1 = lock1_index;

            let auth2 = sys_mut(cluster, auth2_index);
            auth2.data1 = lock2_index;
            auth2.valid_subnets.push(term1_index);

            cluster.add_lan_link(hub_index, 1.0, 0.5, auth1_index, 0.0, 0.5, 1);
            cluster.add_lan_link(auth1_index, 1.0, 0.5, lock1_index, 0.0, 0.5, 1);
            cluster.add_lan_link(lock1_index, 1.0, 0.5, term1_index, 0.0, 0.5, 1);
            cluster.add_lan_link(lock1_index, 0.5, 0.0, term2_index, 0.5, 1.0, 1);
            cluster.add_lan_link(lock1_index, 0.5, 1.0, auth2_index, 0.5, 0.0, 1);
            cluster.add_lan_link(auth2_index, 0.5, 1.0, lock2_index, 0.5, 0.0, 1);
            cluster.add_lan_link(lock2_index, 0.0, 0.5, term3_index, 1.0, 0.5, 1);

            cluster.set_input(hub_index);
            cluster.set_output(lock2_index);
        } else if type_ == 4 {
            let hub_index = Self::generate_hub(g, comp_name, cluster, x1, y, 1);
            let term_index = Self::generate_terminal(g, comp_name, cluster, x, y, 1);
            let auth_index = Self::generate_authentication(g, comp_name, cluster, x, y2, 2);
            let lock_index = Self::generate_lock(g, comp_name, cluster, x1 + 20, y2, 2);
            let trans_index = Self::generate_radio_transmitter(g, comp_name, cluster, x1, y3, 2);
            let recei_index = Self::generate_radio_receiver(g, comp_name, cluster, x3, y1, 3);

            sys_mut(cluster, auth_index).data1 = lock_index;

            let recei = sys_mut(cluster, recei_index);
            set_radio_frequency(recei);
            let freq_data = format!(
                "Radio Receiver Frequency : {}.{} Ghz",
                recei.data1, recei.data2
            );

            let term = snapshot(cluster, term_index);
            hide_in_world(g, comp_name, &term, &freq_data);

            cluster.add_lan_link(hub_index, 0.0, 0.5, term_index, 1.0, 0.5, 1);
            cluster.add_lan_link(hub_index, 0.3, 1.0, auth_index, 0.5, 0.0, 1);
            cluster.add_lan_link(hub_index, 0.7, 1.0, lock_index, 0.5, 0.0, 1);
            cluster.add_lan_link(lock_index, 0.5, 1.0, trans_index, 0.5, 0.0, 1);

            cluster.set_input(hub_index);
            cluster.set_output(recei_index);
        } else if type_ == 5 {
            let hub1_index = Self::generate_hub(g, comp_name, cluster, x, y, 1);
            let trans_index = Self::generate_radio_transmitter(g, comp_name, cluster, x3, y, 1);
            let auth_index = Self::generate_authentication(g, comp_name, cluster, x, y2 + 40, 2);
            let lock_index = Self::generate_lock(g, comp_name, cluster, x2, y2 + 40, 2);
            let hub2_index = Self::generate_hub(g, comp_name, cluster, x2, y1, 3);
            let t1_index = Self::generate_terminal(g, comp_name, cluster, x1, y2 - 80, 3);
            let t2_index = Self::generate_terminal(g, comp_name, cluster, x1, y2 - 40, 3);
            let t3_index = Self::generate_terminal(g, comp_name, cluster, x1, y2 + 0, 3);
            let recei_index = Self::generate_radio_receiver(g, comp_name, cluster, x2, y + height - 40, 3);

            sys_mut(cluster, auth_index).data1 = lock_index;

            let recei = sys_mut(cluster, recei_index);
            set_radio_frequency(recei);
            let freq_data = format!(
                "Radio Receiver Frequency : {}.{} Ghz",
                recei.data1, recei.data2
            );

            // The C++ switch falls through every case into `default`, so the terminal is always t3
            // (the RandomNumber call is still made).
            let _ = NumberGenerator::random_number(3);
            let term = snapshot(cluster, t3_index);
            hide_in_world(g, comp_name, &term, &freq_data);

            cluster.add_lan_link(hub1_index, 0.5, 1.0, auth_index, 0.5, 0.0, 1);
            cluster.add_lan_link(auth_index, 1.0, 0.5, lock_index, 0.0, 0.5, 1);
            cluster.add_lan_link(hub1_index, 1.0, 0.5, trans_index, 0.0, 0.5, 1);
            cluster.add_lan_link(trans_index, 0.5, 1.0, lock_index, 1.0, 0.5, 1);
            cluster.add_lan_link(lock_index, 0.5, 0.0, hub2_index, 0.5, 1.0, 1);
            cluster.add_lan_link(hub2_index, 0.0, 0.25, t1_index, 1.0, 0.5, 1);
            cluster.add_lan_link(hub2_index, 0.0, 0.5, t2_index, 1.0, 0.5, 1);
            cluster.add_lan_link(hub2_index, 0.0, 0.75, t3_index, 1.0, 0.5, 1);

            cluster.set_input(hub1_index);
            cluster.set_output(recei_index);
        } else if type_ == 6 {
            let mut phone_num = "UnInitialised".to_string();

            let hub_index = Self::generate_hub(g, comp_name, cluster, x, y, 1);
            let term1_index = Self::generate_terminal(g, comp_name, cluster, x, y3, 1);
            let auth_index = Self::generate_authentication(g, comp_name, cluster, x2, y, 2);
            let lock_index = Self::generate_lock(g, comp_name, cluster, x2, y2, 2);
            let term2_index = Self::generate_terminal(g, comp_name, cluster, x2, y3, 2);
            let modem_index = Self::generate_modem(g, comp_name, cluster, x3, y, 3, Some(&mut phone_num));

            let auth = sys_mut(cluster, auth_index);
            auth.valid_subnets.push(term1_index);
            auth.data1 = lock_index;

            let data = format!("Local Dial-up number : {}", phone_num);
            let term2 = snapshot(cluster, term2_index);
            hide_in_world(g, comp_name, &term2, &data);

            cluster.add_lan_link(hub_index, 0.25, 1.0, term1_index, 0.5, 0.0, 1);
            cluster.add_lan_link(hub_index, 1.0, 0.5, auth_index, 0.0, 0.5, 1);
            cluster.add_lan_link(hub_index, 0.5, 1.0, lock_index, 0.5, 0.0, 1);
            cluster.add_lan_link(lock_index, 0.5, 1.0, term2_index, 0.5, 0.0, 1);

            cluster.set_input(hub_index);
            cluster.set_output(modem_index);
        }
    }

    pub fn generate_lan_level2_cluster(g: &mut Ctx, comp_name: &str, cluster: &mut LanCluster) {
        let type_ = NumberGenerator::random_number(4) + 1;

        if type_ == 1 {
            // Left

            Self::generate_lan_level0_cluster(g, comp_name, cluster, 0, 50, 200, 200);
            let input1 = cluster.input;
            let output1 = cluster.output;
            let lock_index = Self::generate_lock(g, comp_name, cluster, 100, 350, 3);
            let ms_index = Self::generate_main_server(g, comp_name, cluster, 200, 350, 3);
            cluster.add_lan_link(output1, 0.5, 1.0, lock_index, 0.5, 0.0, 1);
            cluster.add_lan_link(lock_index, 1.0, 0.5, ms_index, 0.0, 0.5, 1);

            // Right

            Self::generate_lan_level1_cluster(g, comp_name, cluster, 250, 50, 275, 275);
            let input2 = cluster.input;
            let output2 = cluster.output;
            let auth_index = Self::generate_authentication(g, comp_name, cluster, 600, 200, 3);
            cluster.add_lan_link(output2, 1.0, 0.5, auth_index, 0.0, 0.5, 1);

            // Remaining

            let hub_index = Self::generate_hub(g, comp_name, cluster, 200, 0, 1);
            cluster.add_lan_link(hub_index, 0.0, 0.5, input1, 0.5, 0.0, 1);
            cluster.add_lan_link(hub_index, 1.0, 0.5, input2, 0.5, 0.0, 1);

            sys_mut(cluster, auth_index).data1 = lock_index;

            cluster.centre_x = 600 / 2;
            cluster.centre_y = 300 / 2;

            cluster.set_input(hub_index);
            cluster.set_output(ms_index);
        } else if type_ == 2 {
            Self::generate_lan_level1_cluster(g, comp_name, cluster, 0, 0, 275, 275);
            let input1 = cluster.input;
            let output1 = cluster.output;

            let mut cluster1 = LanCluster::new();
            Self::generate_lan_level0_cluster(g, comp_name, &mut cluster1, 325, 0, 200, 200);
            cluster1.rotate(3);
            cluster1.merge_cluster(g, cluster, comp_name);
            let input2 = cluster1.input;
            let output2 = cluster1.output;

            let ms_index = Self::generate_main_server(g, comp_name, cluster, 600, 0, 3);

            cluster.add_lan_link(output1, 1.0, 0.5, input2, 0.0, 0.5, 1);
            cluster.add_lan_link(output2, 1.0, 0.5, ms_index, 0.0, 0.5, 1);

            cluster.centre_x = 700 / 2;
            cluster.centre_y = 300 / 2;

            cluster.set_input(input1);
            cluster.set_output(output2);
        } else if type_ == 3 || type_ == 4 {
            Self::generate_lan_level0_cluster(g, comp_name, cluster, 0, 150, 200, 200);
            let input1 = cluster.input;
            let output1 = cluster.output;
            let l1_index = Self::generate_lock(g, comp_name, cluster, 150, 400, 3);
            cluster.add_lan_link(output1, 0.5, 1.0, l1_index, 0.5, 0.0, 1);

            Self::generate_lan_level0_cluster(g, comp_name, cluster, 250, 150, 200, 200);
            let input2 = cluster.input;
            let output2 = cluster.output;
            let l2_index = Self::generate_lock(g, comp_name, cluster, 400, 400, 3);
            cluster.add_lan_link(output2, 0.5, 1.0, l2_index, 0.5, 0.0, 1);

            Self::generate_lan_level0_cluster(g, comp_name, cluster, 500, 150, 200, 200);
            let input3 = cluster.input;
            let output3 = cluster.output;
            let l3_index = Self::generate_lock(g, comp_name, cluster, 650, 400, 3);
            cluster.add_lan_link(output3, 0.5, 1.0, l3_index, 0.5, 0.0, 1);

            let hub_index = Self::generate_hub(g, comp_name, cluster, 350, 0, 1);
            let ms_index = Self::generate_main_server(g, comp_name, cluster, 450, 0, 1);

            let ms = sys_mut(cluster, ms_index);
            ms.data1 = l1_index;
            ms.data2 = l2_index;
            ms.data3 = l3_index;

            cluster.add_lan_link(hub_index, 0.1, 1.0, input1, 0.5, 0.0, 1);
            cluster.add_lan_link(hub_index, 0.3, 1.0, input2, 0.5, 0.0, 1);
            cluster.add_lan_link(hub_index, 0.5, 1.0, input3, 0.5, 0.0, 1);
            cluster.add_lan_link(hub_index, 1.0, 0.5, ms_index, 0.0, 0.5, 1);

            cluster.centre_x = 350;
            cluster.centre_y = 400 / 2;

            cluster.set_input(hub_index);
            cluster.set_output(ms_index);
        }
    }

    pub fn generate_lan_level3_cluster(g: &mut Ctx, comp_name: &str, cluster: &mut LanCluster) {
        // TODO : Fill this in
        Self::generate_lan_level4_cluster(g, comp_name, cluster);
    }

    pub fn generate_lan_level4_cluster(g: &mut Ctx, comp_name: &str, cluster: &mut LanCluster) {
        let type_ = NumberGenerator::random_number(4) + 1;

        if type_ == 1 {
            // Part 1

            Self::generate_lan_level1_cluster(g, comp_name, cluster, 0, 0, 275, 275);
            let input1 = cluster.input;
            let output1 = cluster.output;

            // Part 2

            let mut part2 = LanCluster::new();
            Self::generate_lan_level0_cluster(g, comp_name, &mut part2, 325, 0, 200, 200);
            part2.rotate(3);
            part2.merge_cluster(g, cluster, comp_name);
            let input2 = part2.input;
            let output2 = part2.output;

            // Part 3

            Self::generate_lan_level0_cluster(g, comp_name, cluster, 550, 200, 200, 200);
            let input3 = cluster.input;
            let output3 = cluster.output;

            // Part 4

            let mut part4 = LanCluster::new();
            Self::generate_lan_level1_cluster(g, comp_name, &mut part4, 800, 350, 275, 275);
            part4.rotate(3);
            part4.merge_cluster(g, cluster, comp_name);
            let input4 = part4.input;
            let output4 = part4.output;

            // Main Server

            let ms_index = Self::generate_main_server(g, comp_name, cluster, 1100, 150, 3);

            cluster.add_lan_link(output1, 1.0, 0.5, input2, 0.0, 0.5, 1);
            cluster.add_lan_link(output2, 1.0, 0.5, input3, 0.5, 0.0, 1);
            cluster.add_lan_link(output3, 0.5, 1.0, input4, 0.0, 0.5, 1);
            cluster.add_lan_link(output4, 1.0, 0.5, ms_index, 0.5, 1.0, 1);

            cluster.centre_x = 1200 / 2;
            cluster.centre_y = 600 / 2;

            cluster.set_input(input1);
            cluster.set_output(output4);
        } else if type_ == 2 {
            // Left system

            Self::generate_lan_level1_cluster(g, comp_name, cluster, 0, 50, 275, 275);
            let input1 = cluster.input;
            let output1 = cluster.output;
            let term_index = Self::generate_terminal(g, comp_name, cluster, 275, 400, 3);
            cluster.add_lan_link(output1, 0.5, 1.0, term_index, 0.5, 0.0, 1);

            // Central system

            Self::generate_lan_level1_cluster(g, comp_name, cluster, 330, 300, 300, 300);
            let input2 = cluster.input;
            let output2 = cluster.output;
            let mut phone_num = "UnInitialised".to_string();
            let modem_index = Self::generate_modem(g, comp_name, cluster, 480, 100, 3, Some(&mut phone_num));
            let lock_index = Self::generate_lock(g, comp_name, cluster, 480, 200, 3);
            cluster.add_lan_link(modem_index, 0.5, 1.0, lock_index, 0.5, 0.0, 1);
            cluster.add_lan_link(lock_index, 0.5, 1.0, input2, 0.5, 0.0, 1);

            let ms_index = Self::generate_main_server(g, comp_name, cluster, 630, 650, 3);
            cluster.add_lan_link(output2, 0.5, 1.0, ms_index, 0.5, 0.0, 1);

            // Right system

            Self::generate_lan_level1_cluster(g, comp_name, cluster, 660, 50, 275, 275);
            let input3 = cluster.input;
            let output3 = cluster.output;
            let auth_index = Self::generate_authentication(g, comp_name, cluster, 960, 400, 3);
            cluster.add_lan_link(output3, 0.5, 1.0, auth_index, 0.5, 0.0, 1);

            // Hub system

            let hub_index = Self::generate_hub(g, comp_name, cluster, 480, 0, 1);
            cluster.add_lan_link(hub_index, 0.0, 0.5, input1, 0.5, 0.0, 1);
            cluster.add_lan_link(hub_index, 1.0, 0.5, input3, 0.5, 0.0, 1);

            // Remaining

            sys_mut(cluster, auth_index).data1 = lock_index;

            let data = format!("Local Dial up number: {}", phone_num);
            let term = snapshot(cluster, term_index);
            hide_in_world(g, comp_name, &term, &data);

            cluster.centre_x = 1000 / 2;
            cluster.centre_y = 700 / 2;

            cluster.set_input(hub_index);
            cluster.set_output(ms_index);
        }
        if type_ == 3 {
            // Left authentication

            let mut left_cluster = LanCluster::new();
            let rec1_index = Self::generate_radio_receiver(g, comp_name, &mut left_cluster, 100, 0, 1);
            Self::generate_lan_level1_cluster(g, comp_name, &mut left_cluster, 0, 100, 300, 300);
            let auth1_index = Self::generate_authentication(g, comp_name, &mut left_cluster, 350, 400, 1);
            left_cluster.add_lan_link(rec1_index, 0.5, 1.0, left_cluster.input, 0.5, 0.0, 1);
            left_cluster.add_lan_link(left_cluster.output, 1.0, 0.5, auth1_index, 0.0, 0.5, 1);
            left_cluster.set_output(auth1_index);
            let recei1 = sys_mut(&mut left_cluster, rec1_index);
            set_radio_frequency(recei1);
            let (recei1_d1, recei1_d2) = (recei1.data1, recei1.data2);
            left_cluster.rotate(1);
            left_cluster.merge_cluster(g, cluster, comp_name);
            let output1 = left_cluster.output;

            // Right authentication

            let mut right_cluster = LanCluster::new();
            let rec2_index = Self::generate_radio_receiver(g, comp_name, &mut right_cluster, 1000, 0, 1);
            Self::generate_lan_level1_cluster(g, comp_name, &mut right_cluster, 900, 60, 300, 300);
            let auth2_index = Self::generate_authentication(g, comp_name, &mut right_cluster, 1250, 400, 1);
            right_cluster.add_lan_link(rec2_index, 0.5, 1.0, right_cluster.input, 0.5, 0.0, 1);
            right_cluster.add_lan_link(right_cluster.output, 1.0, 0.5, auth2_index, 0.0, 0.5, 1);
            right_cluster.set_output(auth2_index);
            let recei2 = sys_mut(&mut right_cluster, rec2_index);
            set_radio_frequency(recei2);
            let (recei2_d1, recei2_d2) = (recei2.data1, recei2.data2);
            right_cluster.rotate(3);
            right_cluster.merge_cluster(g, cluster, comp_name);
            let output2 = right_cluster.output;

            // Lower authentication

            let mut lower_cluster = LanCluster::new();
            let rec3_index = Self::generate_radio_receiver(g, comp_name, &mut lower_cluster, 575, 400, 1);
            Self::generate_lan_level1_cluster(g, comp_name, &mut lower_cluster, 460, 500, 300, 300);
            let auth3_index = Self::generate_authentication(g, comp_name, &mut lower_cluster, 800, 900, 1);
            lower_cluster.add_lan_link(rec3_index, 0.5, 1.0, lower_cluster.input, 0.5, 0.0, 1);
            lower_cluster.add_lan_link(lower_cluster.output, 1.0, 0.5, auth3_index, 0.0, 0.5, 1);
            lower_cluster.set_output(auth3_index);
            let recei3 = sys_mut(&mut lower_cluster, rec3_index);
            set_radio_frequency(recei3);
            let (recei3_d1, recei3_d2) = (recei3.data1, recei3.data2);
            lower_cluster.merge_cluster(g, cluster, comp_name);
            let output3 = lower_cluster.output;

            // Central bit

            let x1 = 460;
            let y1 = 0;
            let w = 300;
            let h = 300;

            let hub_index = Self::generate_hub(g, comp_name, cluster, x1 + w / 2, y1, 1);
            let t1_index = Self::generate_terminal(g, comp_name, cluster, x1, y1 + h / 2, 1);
            let t2_index = Self::generate_terminal(g, comp_name, cluster, x1 + w, y1 + h / 2, 1);
            let trans_index = Self::generate_radio_transmitter(g, comp_name, cluster, x1 + w / 2, y1 + h, 1);
            let l1_index = Self::generate_lock(g, comp_name, cluster, x1 + w / 4 - 20, y1 + h / 3, 1);
            let l2_index = Self::generate_lock(g, comp_name, cluster, x1 + w - 40, y1 + h / 3, 1);
            let l3_index = Self::generate_lock(g, comp_name, cluster, x1 + w / 2, y1 + h - 60, 1);
            let ms_index = Self::generate_main_server(g, comp_name, cluster, x1 + w / 2, y1 + h / 2, 1);

            let ms = sys_mut(cluster, ms_index);
            ms.data1 = l1_index;
            ms.data2 = l2_index;
            ms.data3 = l3_index;

            sys_mut(cluster, output1).data1 = l1_index;
            sys_mut(cluster, output2).data1 = l2_index;
            sys_mut(cluster, output3).data1 = l3_index;

            let data = format!(
                "Known Radio Receiver Frequencies :\n\n\
                 Receiver A : {}.{} Ghz\n\
                 Receiver B : {}.{} Ghz\n\
                 Receiver C : {}.{} Ghz",
                recei1_d1, recei1_d2, recei2_d1, recei2_d2, recei3_d1, recei3_d2
            );
            let t1 = snapshot(cluster, t1_index);
            hide_in_world(g, comp_name, &t1, &data);

            cluster.add_lan_link(hub_index, 0.0, 0.3, t1_index, 0.5, 0.0, 1);
            cluster.add_lan_link(t1_index, 0.5, 1.0, trans_index, 0.0, 0.5, 1);
            cluster.add_lan_link(hub_index, 1.0, 0.3, t2_index, 0.5, 0.0, 1);
            cluster.add_lan_link(t2_index, 0.5, 1.0, trans_index, 1.0, 0.5, 1);

            cluster.add_lan_link(hub_index, 0.0, 0.7, l1_index, 0.5, 0.0, 1);
            cluster.add_lan_link(hub_index, 1.0, 0.7, l2_index, 0.5, 0.0, 1);
            cluster.add_lan_link(hub_index, 0.5, 1.0, ms_index, 0.5, 0.0, 1);
            cluster.add_lan_link(l1_index, 1.0, 0.5, ms_index, 0.0, 0.5, 1);
            cluster.add_lan_link(l2_index, 0.0, 0.5, ms_index, 1.0, 0.5, 1);
            cluster.add_lan_link(trans_index, 0.5, 0.0, l3_index, 0.5, 1.0, 1);
            cluster.add_lan_link(l3_index, 0.5, 0.0, ms_index, 0.5, 1.0, 1);

            cluster.set_input(hub_index);
            cluster.set_output(trans_index);

            cluster.centre_x = x1 + w / 2;
            cluster.centre_y = y1 + h / 2;
        } else if type_ == 4 {
            Self::generate_lan_level0_cluster(g, comp_name, cluster, 0, 0, 200, 200);
            let input1 = cluster.input;
            let output1 = cluster.output;

            Self::generate_lan_level1_cluster(g, comp_name, cluster, 0, 275, 300, 300);
            let input2 = cluster.input;
            let output2 = cluster.output;

            let mut part3 = LanCluster::new();
            Self::generate_lan_level1_cluster(g, comp_name, &mut part3, 340, 275, 300, 300);
            part3.rotate(3);
            part3.merge_cluster(g, cluster, comp_name);
            let input3 = part3.input;
            let output3 = part3.output;

            let ms_index = Self::generate_main_server(g, comp_name, cluster, 500, 150, 3);

            cluster.add_lan_link(output1, 0.5, 1.0, input2, 0.5, 0.0, 1);
            cluster.add_lan_link(output2, 1.0, 0.5, input3, 0.0, 0.5, 1);
            cluster.add_lan_link(output3, 0.5, 0.0, ms_index, 0.5, 1.0, 1);

            cluster.set_input(input1);
            cluster.set_output(ms_index);

            cluster.centre_x = 640 / 2;
            cluster.centre_y = 540 / 2;
        }
    }

    pub fn generate_lan_bounding_box(
        g: &mut Ctx,
        comp_name: &str,
        cluster: &mut LanCluster,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) {
        if TESTGAME {
            let l1_index = Self::generate_lock(g, comp_name, cluster, x - 16, y - 16, 1);
            let l2_index = Self::generate_lock(g, comp_name, cluster, x + width - 16, y - 16, 1);
            let l3_index = Self::generate_lock(g, comp_name, cluster, x + width - 16, y + height - 16, 1);
            let l4_index = Self::generate_lock(g, comp_name, cluster, x - 16, y + height, 1);

            cluster.add_lan_link(l1_index, 1.0, 0.5, l2_index, 0.0, 0.5, 1);
            cluster.add_lan_link(l2_index, 0.5, 1.0, l3_index, 0.5, 0.0, 1);
            cluster.add_lan_link(l3_index, 0.0, 0.5, l4_index, 1.0, 0.5, 1);
            cluster.add_lan_link(l4_index, 0.5, 0.0, l1_index, 0.5, 1.0, 1);
        }
    }

    //
    // Low level functions for generating a single atomic item
    // All return index of the item created
    // All create new systems on cluster when required
    // All create new computer screens on comp when required

    pub fn generate_router(
        g: &mut Ctx,
        comp_name: &str,
        cluster: &mut LanCluster,
        x: i32,
        y: i32,
        security: i32,
    ) -> i32 {
        //
        // Create the screen

        let mut ms = MessageScreen::new();
        ms.base.set_main_title(&company_name(g, comp_name));
        ms.base.set_sub_title("Local Area Network");
        ms.set_text_message(
            "EXTERNAL ROUTER ACCESSED\n\n\
             You have accessed the Local Area Network of our company.\n\
             This system is not open to external connections from the Net.\n\n\
             If you reached this page in error, please disconnect now.\n\n\n\
             Your IP has been logged.\n\
             If you continue to connect to this private network\n\
             our system administrators will begin tracing you,\n\
             which could result in legal action.",
        );

        let screen_index = add_screen(g, comp_name, ComputerScreen::MessageScreen(ms));

        //
        // Add the LAN system

        cluster.add_lan_system_new(LANSYSTEM_ROUTER, x, y, security, screen_index, screen_index)
    }

    pub fn generate_hub(
        g: &mut Ctx,
        comp_name: &str,
        cluster: &mut LanCluster,
        x: i32,
        y: i32,
        security: i32,
    ) -> i32 {
        //
        // Create the screen

        let mut ms = MessageScreen::new();
        ms.base.set_main_title(&company_name(g, comp_name));
        ms.base.set_sub_title("Local Area Network");
        ms.set_text_message(
            "INTERNAL HUB ACCESSED\n\n\
             HUB Internal Operating system Active\n\
             Incoming connections Active\n\
             Outgoing connections Active\n\n\
             Maximum HUB bandwidth : 1000 Gq/s\n\
             Peak latency : 10ms\n\
             Packet collisions detected : None\n\n\
             All systems operational",
        );

        let screen_index = add_screen(g, comp_name, ComputerScreen::MessageScreen(ms));

        //
        // Add the LAN system

        cluster.add_lan_system_new(LANSYSTEM_HUB, x, y, security, screen_index, screen_index)
    }

    pub fn generate_terminal(
        g: &mut Ctx,
        comp_name: &str,
        cluster: &mut LanCluster,
        x: i32,
        y: i32,
        security: i32,
    ) -> i32 {
        // Opening message screen

        let mut ms = MessageScreen::new();
        ms.base.set_main_title(&company_name(g, comp_name));
        ms.base.set_sub_title("Local Area Network");
        ms.set_text_message(
            "TERMINAL ACCESSED\n\n\
             Password verification will be required\n\
             for further systems access.  All activity through these\n\
             terminals is logged and analysed on a daily basis.\n\n\
             If you are not a registered employee of this company\n\
             then you must disconnect now, or legal charges may be\n\
             brought against you.",
        );
        ms.set_button_message("OK");

        // Password screen

        let mut ps = PasswordScreen::new();
        ps.base.set_main_title("LAN Terminal");
        ps.base.set_sub_title("Password verification required");
        ps.set_difficulty(HACKDIFFICULTY_LANTERMINAL);
        ps.set_password(&password_for_security(g, security));

        // You're in screen

        let mut ms2 = MessageScreen::new();
        ms2.base.set_main_title(&company_name(g, comp_name));
        ms2.base.set_sub_title("ACCESS GRANTED");
        ms2.set_text_message(
            "All security verification checks out OK.\n\
             You are now authorised to use this Terminal system.\n\n\
             Please respect the wishes and needs of other users,\n\
             and report any suspected illegal activity to the\n\
             system administrator.",
        );

        let ms_index = add_screen(g, comp_name, ComputerScreen::MessageScreen(ms));
        let ps_index = add_screen(g, comp_name, ComputerScreen::PasswordScreen(ps));
        let ms2_index = add_screen(g, comp_name, ComputerScreen::MessageScreen(ms2));

        set_next_page(lan_computer_mut(g, comp_name), ms_index, ps_index);
        set_next_page(lan_computer_mut(g, comp_name), ps_index, ms2_index);

        //
        // Add the LAN system

        cluster.add_lan_system_new(LANSYSTEM_TERMINAL, x, y, security, ms_index, ms2_index)
    }

    pub fn generate_lock(
        g: &mut Ctx,
        comp_name: &str,
        cluster: &mut LanCluster,
        x: i32,
        y: i32,
        security: i32,
    ) -> i32 {
        //
        // Create the screen

        let mut ms = MessageScreen::new();
        ms.base.set_main_title(&company_name(g, comp_name));
        ms.base.set_sub_title("Local Area Network");
        ms.set_text_message(
            "LOCK ACCESSED\n\n\
             Full systems check: OK\n\
             Maximum simultanious clients: 16\n\
             Maximum available bandwidth: 200 Gq/s\
             Security checks valid\n\n\
             This lock is DISABLED",
        );

        let screen_index = add_screen(g, comp_name, ComputerScreen::MessageScreen(ms));

        //
        // Add the LAN system

        let system_index =
            cluster.add_lan_system_new(LANSYSTEM_LOCK, x, y, security, screen_index, screen_index);

        let lock = sys_mut(cluster, system_index);
        lock.data1 = 1;

        system_index
    }

    pub fn generate_authentication(
        g: &mut Ctx,
        comp_name: &str,
        cluster: &mut LanCluster,
        x: i32,
        y: i32,
        security: i32,
    ) -> i32 {
        //
        // Opening message screen

        let mut ms = MessageScreen::new();
        ms.base.set_main_title(&company_name(g, comp_name));
        ms.base.set_sub_title("Local Area Network");
        ms.set_text_message(
            "AUTHENTICATION SERVER ACCESSED\n\n\
             Authentication Server dedicated server software running\n\
             One user currently detected online\n\
             Incoming ports check OK\n\
             Outgoing ports check OK\n\
             Target Lock systems check OK\n\n\
             If you are not the system administrator you must disconnect now\n\
             or you will be punished to the full extent of the law.\n\
             All access to this system is logged and checked on a daily basis.",
        );
        ms.set_button_message("OK");
        let ms_index = add_screen(g, comp_name, ComputerScreen::MessageScreen(ms));

        // Password Screen

        let mut ps = PasswordScreen::new();
        ps.base.set_main_title("Authentication Server");
        ps.base.set_sub_title("Password verification required");
        ps.set_difficulty(HACKDIFFICULTY_LANAUTHENTICATIONSERVER);
        ps.set_password(&password_for_security(g, security));
        let ps_index = add_screen(g, comp_name, ComputerScreen::PasswordScreen(ps));

        // Cypher screen

        let mut cs = CypherScreen::new();
        cs.base.set_main_title("Authentication Server");
        cs.base.set_sub_title("Enter elliptic-curve encryption cypher");
        cs.set_difficulty(HACKDIFFICULTY_GLOBALCRIMINALDATABASE);
        let cs_index = add_screen(g, comp_name, ComputerScreen::CypherScreen(cs));

        // High Security Screen

        let mut hs = HighSecurityScreen::new();
        hs.base.set_main_title("Authentication Server");
        hs.base.set_sub_title("Authorisation required");
        hs.add_system("Password verification", ps_index);
        hs.add_system("Elliptic-Curve Encryption Cypher", cs_index);
        let hs_index = add_screen(g, comp_name, ComputerScreen::HighSecurityScreen(hs));

        set_next_page(lan_computer_mut(g, comp_name), ms_index, hs_index);
        set_next_page(lan_computer_mut(g, comp_name), ps_index, hs_index);
        set_next_page(lan_computer_mut(g, comp_name), cs_index, hs_index);

        // All done screen

        let company = company_name(g, comp_name);
        let mut ds = DialogScreen::new();
        ds.base.set_main_title(&company);
        ds.base.set_sub_title("Authentication Server");
        ds.add_widget(
            "text",
            WIDGET_CAPTION,
            50,
            100,
            200,
            100,
            "Access to authentication Server systems granted.\n\
             Please proceed with caution.",
            " ",
        );
        let ds_index = add_screen(g, comp_name, ComputerScreen::DialogScreen(ds));
        dialog_mut(lan_computer_mut(g, comp_name), ds_index).add_widget_with_data(
            "enable",
            WIDGET_SCRIPTBUTTON,
            50,
            200,
            150,
            20,
            "Enable Security Locks",
            "Enable the locks on this Local Area Network",
            80,
            ds_index,
            None,
            None,
        );
        dialog_mut(lan_computer_mut(g, comp_name), ds_index).add_widget_with_data(
            "disable",
            WIDGET_SCRIPTBUTTON,
            50,
            240,
            150,
            20,
            "Disable security locks",
            "Disable the locks on this Local Area Network",
            81,
            ds_index,
            None,
            None,
        );

        set_next_page(lan_computer_mut(g, comp_name), hs_index, ds_index);

        // Generate the system

        cluster.add_lan_system_new(LANSYSTEM_AUTHENTICATION, x, y, security, ms_index, -1)
    }

    pub fn generate_main_server(
        g: &mut Ctx,
        comp_name: &str,
        cluster: &mut LanCluster,
        x: i32,
        y: i32,
        security: i32,
    ) -> i32 {
        //
        // Opening message screen

        let mut ms = MessageScreen::new();
        ms.base.set_main_title(&company_name(g, comp_name));
        ms.base.set_sub_title("Local Area Network");
        ms.set_text_message(
            "MAIN SERVER ACCESSED\n\n\
             External connection to Main Server granted.\n\
             This connection is being logged.\n\n\
             If you are not authorised to have access to this system\n\
             then you will be caught and prosecuted under the full force\n\
             of the International Data Security Treaty, 2006.\n\n",
        );
        ms.set_button_message("OK");
        let ms_index = add_screen(g, comp_name, ComputerScreen::MessageScreen(ms));

        // Password Screen

        let mut ps = PasswordScreen::new();
        ps.base.set_main_title("Main Server");
        ps.base.set_sub_title("Password verification required");
        ps.set_difficulty(HACKDIFFICULTY_LANMAINSERVER);
        ps.set_password(&password_for_security(g, security));
        let ps_index = add_screen(g, comp_name, ComputerScreen::PasswordScreen(ps));

        // Voice print screen

        let mut gs = GenericScreen::new();
        gs.base.set_main_title("Main Server");
        gs.base.set_sub_title("Voice print analysis required");
        gs.set_screen_type(SCREEN_VOICEANALYSIS);
        let gs_index = add_screen(g, comp_name, ComputerScreen::GenericScreen(gs));

        // High Security Screen

        let mut hs = HighSecurityScreen::new();
        hs.base.set_main_title("Authentication Server");
        hs.base.set_sub_title("Authorisation required");
        hs.add_system("UserID / password verification", ps_index);
        hs.add_system("Voice Print Analysis", gs_index);
        let hs_index = add_screen(g, comp_name, ComputerScreen::HighSecurityScreen(hs));

        set_next_page(lan_computer_mut(g, comp_name), ms_index, hs_index);
        set_next_page(lan_computer_mut(g, comp_name), ps_index, hs_index);
        set_next_page(lan_computer_mut(g, comp_name), gs_index, hs_index);

        // File server

        let mut fss = GenericScreen::new();
        fss.set_screen_type(SCREEN_FILESERVERSCREEN);
        fss.base.set_main_title("Main Server");
        fss.base.set_sub_title("File server");
        let fss_index = add_screen(g, comp_name, ComputerScreen::GenericScreen(fss));

        // Log screen

        let mut ls = LogScreen::new();
        ls.set_target(LOGSCREEN_TARGET_ACCESSLOGS);
        ls.base.set_main_title("Log Server");
        ls.base.set_sub_title("Access Logs");
        let log_index = add_screen(g, comp_name, ComputerScreen::LogScreen(ls));

        // Console

        let mut gs6 = GenericScreen::new();
        gs6.set_screen_type(SCREEN_CONSOLESCREEN);
        gs6.base.set_main_title("Main Server");
        gs6.base.set_sub_title("Console");
        let gs6_index = add_screen(g, comp_name, ComputerScreen::GenericScreen(gs6));

        // Menu

        let mut menu = MenuScreen::new();
        menu.base.set_main_title("Main Server");
        menu.base.set_sub_title("Main Menu");
        menu.add_option("File Server", "Access the file server", fss_index, 3, -1);
        menu.add_option("Log Server", "Access logs on this system", log_index, 1, -1);
        menu.add_option("Console", "Use a console on this system", gs6_index, 1, -1);
        let menu_index = add_screen(g, comp_name, ComputerScreen::MenuScreen(menu));

        set_next_page(lan_computer_mut(g, comp_name), hs_index, menu_index);
        set_next_page(lan_computer_mut(g, comp_name), gs6_index, menu_index);
        set_next_page(lan_computer_mut(g, comp_name), fss_index, menu_index);
        set_next_page(lan_computer_mut(g, comp_name), log_index, menu_index);

        // Generate the system

        cluster.add_lan_system_new(LANSYSTEM_MAINSERVER, x, y, security, ms_index, -1)
    }

    pub fn generate_log_server(
        g: &mut Ctx,
        comp_name: &str,
        cluster: &mut LanCluster,
        x: i32,
        y: i32,
        security: i32,
    ) -> i32 {
        // Message screen

        let mut ms = MessageScreen::new();
        ms.base.set_main_title(&company_name(g, comp_name));
        ms.base.set_sub_title("Log Server");
        ms.set_text_message(
            "External access to LOG server granted.\n\n\
             Checking Operating System : OK\n\
             Checking File system : OK\n\
             Checking LOG integrity : OK\n\n\
             Warning: Only the Sys admin has access to this system.\n\
             All attempted hacks will be logged ;)",
        );
        ms.set_button_message("OK");
        let ms_index = add_screen(g, comp_name, ComputerScreen::MessageScreen(ms));

        // Password Screen

        let mut ps = PasswordScreen::new();
        ps.base.set_main_title("Log Server");
        ps.base.set_sub_title("Password verification required");
        ps.set_difficulty(HACKDIFFICULTY_LANLOGSERVER);
        ps.set_password(&password_for_security(g, security));
        let ps_index = add_screen(g, comp_name, ComputerScreen::PasswordScreen(ps));

        // Log screen

        let mut ls = LogScreen::new();
        ls.set_target(LOGSCREEN_TARGET_ACCESSLOGS);
        ls.base.set_main_title("Log Server");
        ls.base.set_sub_title("Access Logs");
        let ls_index = add_screen(g, comp_name, ComputerScreen::LogScreen(ls));

        set_next_page(lan_computer_mut(g, comp_name), ms_index, ps_index);
        set_next_page(lan_computer_mut(g, comp_name), ps_index, ls_index);

        cluster.add_lan_system_new(LANSYSTEM_LOGSERVER, x, y, security, ms_index, -1)
    }

    pub fn generate_isolation_bridge(
        g: &mut Ctx,
        comp_name: &str,
        cluster: &mut LanCluster,
        x: i32,
        y: i32,
        security: i32,
    ) -> i32 {
        //
        // Main control panel

        let company = company_name(g, comp_name);
        let mut ds = DialogScreen::new();
        ds.base.set_main_title(&company);
        ds.base.set_sub_title("ISOLATION BRIDGE");
        ds.add_widget(
            "text",
            WIDGET_CAPTION,
            50,
            100,
            200,
            100,
            "Isolation bridge accessed",
            " ",
        );
        ds.add_widget_with_data(
            "enable",
            WIDGET_SCRIPTBUTTON,
            50,
            200,
            150,
            20,
            "Enable Isolation Bridge",
            "Enable the bridge (disconnecting Router)",
            82,
            -1,
            None,
            None,
        );
        ds.add_widget_with_data(
            "disable",
            WIDGET_SCRIPTBUTTON,
            50,
            240,
            150,
            20,
            "Disable Isolation Bridge",
            "Disable the bridge (enable Router)",
            83,
            -1,
            None,
            None,
        );
        let ds_index = add_screen(g, comp_name, ComputerScreen::DialogScreen(ds));

        cluster.add_lan_system_new(LANSYSTEM_ISOLATIONBRIDGE, x, y, security, ds_index, -1)
    }

    pub fn generate_session_key_server(
        g: &mut Ctx,
        comp_name: &str,
        cluster: &mut LanCluster,
        x: i32,
        y: i32,
        security: i32,
    ) -> i32 {
        let mut ms = MessageScreen::new();
        ms.base.set_main_title(&company_name(g, comp_name));
        ms.base.set_sub_title("Session key server");
        ms.set_text_message("This is a session key server");
        let ms_index = add_screen(g, comp_name, ComputerScreen::MessageScreen(ms));

        cluster.add_lan_system_new(LANSYSTEM_SESSIONKEYSERVER, x, y, security, ms_index, -1)
    }

    /// `phone_num` is the C++ in/out `char *phoneNum = NULL`: if it holds a number starting with
    /// '0' that number is used, and on return it holds the dial-up number actually created.
    pub fn generate_modem(
        g: &mut Ctx,
        comp_name: &str,
        cluster: &mut LanCluster,
        x: i32,
        y: i32,
        security: i32,
        phone_num: Option<&mut String>,
    ) -> i32 {
        //
        // New modem virtual location

        let (map_x, map_y) = WorldGenerator::generate_valid_map_pos();

        let given = phone_num
            .as_deref()
            .filter(|number| number.starts_with('0'))
            .map(|number| number.to_string());
        let ip = match given {
            Some(number) => number,
            None => {
                let mut digits = [0; 10];
                for digit in digits.iter_mut() {
                    *digit = NumberGenerator::random_number(10);
                }
                format!(
                    "0{}{}{}{}-{}{}{}{}{}{}",
                    digits[0],
                    digits[1],
                    digits[2],
                    digits[3],
                    digits[4],
                    digits[5],
                    digits[6],
                    digits[7],
                    digits[8],
                    digits[9]
                )
            }
        };

        let mut vl = VLocation::new_special();
        vl.set_plocation(map_x, map_y);
        vl.set_ip(&ip);
        g.world_mut().create_vlocation_from(vl);
        {
            let location = g.world_mut().get_vlocation_mut(&ip).expect("modem location");
            location.set_listed(false);
            location.set_computer(comp_name);
        }

        if let Some(number) = phone_num {
            *number = ip.clone();
        }

        //
        // Password Screen

        let mut ps = PasswordScreen::new();
        ps.base.set_main_title("Modem");
        ps.base.set_sub_title("Password verification required");
        ps.set_difficulty(HACKDIFFICULTY_LANMODEM);
        ps.set_password(&password_for_security(g, security));
        let ps_index = add_screen(g, comp_name, ComputerScreen::PasswordScreen(ps));

        //
        // Opening message screen

        let text_msg = format!(
            "LAN DIAL-UP ACCESSED\n\n\
             Phone charges incurred will be your own.\n\
             Our company is not responsible for your misuse of this system.\n\
             Welcome to our LAN.\n\
             If you are not an authorised user you must disconnect now or risk\n\
             legal action and jail time.\n\n\
             The local dial-up number is {}.",
            ip
        );

        let mut ms = MessageScreen::new();
        ms.base.set_main_title(&company_name(g, comp_name));
        ms.base.set_sub_title("Local Area Network");
        ms.set_text_message(&text_msg);
        let ms_index = add_screen(g, comp_name, ComputerScreen::MessageScreen(ms));

        set_next_page(lan_computer_mut(g, comp_name), ps_index, ms_index);
        let system_index =
            cluster.add_lan_system_new(LANSYSTEM_MODEM, x, y, security, ps_index, ms_index);

        if let Some(special) = g
            .world_mut()
            .get_vlocation_mut(&ip)
            .and_then(|location| location.special.as_mut())
        {
            special.set_screen_index(ms_index);
            special.set_security_system_index(system_index);
        }

        system_index
    }

    pub fn generate_radio_transmitter(
        g: &mut Ctx,
        comp_name: &str,
        cluster: &mut LanCluster,
        x: i32,
        y: i32,
        security: i32,
    ) -> i32 {
        let mut gs = GenericScreen::new();
        gs.base.set_main_title(&company_name(g, comp_name));
        gs.base.set_sub_title("Wireless LAN Transmitter");
        gs.set_screen_type(SCREEN_RADIOTRANSMITTER);
        let gs_index = add_screen(g, comp_name, ComputerScreen::GenericScreen(gs));

        cluster.add_lan_system_new(LANSYSTEM_RADIOTRANSMITTER, x, y, security, gs_index, -1)
    }

    pub fn generate_radio_receiver(
        g: &mut Ctx,
        comp_name: &str,
        cluster: &mut LanCluster,
        x: i32,
        y: i32,
        security: i32,
    ) -> i32 {
        let mut ms = MessageScreen::new();
        ms.base.set_main_title(&company_name(g, comp_name));
        ms.base.set_sub_title("Wireless LAN Receiver");
        ms.set_text_message(
            "Wireless connection successfully established.\n\
             Checking receiver systems : OK\n\
             Checking frequency range : OK\n\
             Incoming signal strength : 90%\n\n\
             Maximum system bandwidth : 50Gq/s\n\
             Maximum system latency : 400ms",
        );
        let ms_index = add_screen(g, comp_name, ComputerScreen::MessageScreen(ms));

        cluster.add_lan_system_new(LANSYSTEM_RADIORECEIVER, x, y, security, ms_index, ms_index)
    }

    pub fn generate_fax_printer(
        g: &mut Ctx,
        comp_name: &str,
        cluster: &mut LanCluster,
        x: i32,
        y: i32,
        security: i32,
    ) -> i32 {
        let mut ms = MessageScreen::new();
        ms.base.set_main_title(&company_name(g, comp_name));
        ms.base.set_sub_title("Fax printer");
        ms.set_text_message("This is a fax printer");
        let ms_index = add_screen(g, comp_name, ComputerScreen::MessageScreen(ms));

        cluster.add_lan_system_new(LANSYSTEM_FAXPRINTER, x, y, security, ms_index, ms_index)
    }

    // Attempts to plant the data on the system.
    // Return success / failure

    /// `HideData(comp, systemIndex, data)`: `system_index` indexes the LAN's `systems`.
    pub fn hide_data(comp: &mut Computer, system_index: i32, data: &str) -> bool {
        let system = match comp.lan().and_then(|lan| lan.systems.get_data(system_index)) {
            Some(system) => system.clone(),
            None => return false,
        };
        Self::hide_data_system(comp, &system, data)
    }

    /// `HideData(comp, system, data)`: `system` is a snapshot (clone) of one of the LAN's systems.
    pub fn hide_data_system(comp: &mut Computer, system: &LanComputerSystem, data: &str) -> bool {
        //
        // Try to insert the data into a message screen

        if system.data_screen_index != -1 {
            if let Some(ComputerScreen::MessageScreen(ms)) =
                comp.get_computer_screen_mut(system.data_screen_index)
            {
                let old_text = ms.textmessage.clone().unwrap_or_default();
                ms.set_text_message(&format!("{}\n\n{}", old_text, data));
                return true;
            }
        }

        false
    }

    //
    // Shared by LoadLAN and GenerateLAN: the files, logs and login created on a new LAN computer.

    fn generate_lan_contents(g: &mut Ctx, comp_name: &str, company_name: &str) {
        //
        // Create the files

        let databank_size = NumberGenerator::random_normal_number(100.0, 40.0) as i32;
        lan_computer_mut(g, comp_name).databank.set_size(databank_size);

        let numfiles = NumberGenerator::random_normal_number(10.0, 5.0) as i32;

        for _ in 0..numfiles {
            let type_ = NumberGenerator::random_number(2) + 1;
            let size = NumberGenerator::random_normal_number(6.0, 4.0) as i32;
            let encrypted = NumberGenerator::random_number(2);
            let compressed = NumberGenerator::random_number(2);
            let datatitle = NameGenerator::generate_data_name(company_name, type_);

            let mut data = Data::new();
            data.set_title(&datatitle);
            let encrypted_value = if encrypted != 0 {
                NumberGenerator::random_number(5)
            } else {
                0
            };
            let compressed_value = if compressed != 0 {
                NumberGenerator::random_number(5)
            } else {
                0
            };
            data.set_details(
                type_,
                size,
                encrypted_value,
                compressed_value,
                1.0,
                SOFTWARETYPE_NONE,
            );
            lan_computer_mut(g, comp_name).databank.put_data(data);
        }

        //
        // Create some logs

        let numlogs = NumberGenerator::random_number(10);

        for _ in 0..numlogs {
            let from_ip = WorldGenerator::get_random_location(g);
            let date = g.world().date.clone();
            let mut al = AccessLog::new();
            al.set_properties(&date, &from_ip, " ", LOG_NOTSUSPICIOUS, LOG_TYPE_TEXT);
            al.set_data1(Some("Accessed File"));
            lan_computer_mut(g, comp_name).logbank.add_log(al, -1);
        }

        //
        // Create some log-in's

        let password = NameGenerator::generate_complex_password(g);
        let mut record = Record::new();
        record.add_field(RECORDBANK_NAME, RECORDBANK_ADMIN);
        record.add_field(RECORDBANK_PASSWORD, &password);
        record.add_field(RECORDBANK_SECURITY, "1");
        lan_computer_mut(g, comp_name).recordbank.add_record(record);
    }
}

/*
    LAN cluster class

        Allows any number of systems and links to be added
        The whole lot can then be rotated around a point
        Clusters can be merged to form larger clusters
        Clusters can be merged into a LanComputer, preserving links
*/

/// Transient (not part of the saved world), so no serde derives.
#[derive(Debug, Clone)]
pub struct LanCluster {
    pub systems: DArray<LanComputerSystem>,
    pub links: DArray<LanComputerLink>,

    pub input: i32,
    pub output: i32,

    pub centre_x: i32,
    pub centre_y: i32,
}

impl Default for LanCluster {
    fn default() -> Self {
        Self::new()
    }
}

impl LanCluster {
    pub fn new() -> Self {
        LanCluster {
            systems: DArray::new(),
            links: DArray::new(),
            input: -1,
            output: -1,
            centre_x: 0,
            centre_y: 0,
        }
    }

    /// `AddLanSystem ( LanComputerSystem *system )`
    pub fn add_lan_system(&mut self, system: LanComputerSystem) -> i32 {
        self.systems.put_data(system)
    }

    /// `AddLanSystem ( int TYPE, int x, int y, int security = 2, int screenIndex = -1, int dataScreenIndex = -1 )`
    pub fn add_lan_system_new(
        &mut self,
        type_: i32,
        x: i32,
        y: i32,
        security: i32,
        screen_index: i32,
        data_screen_index: i32,
    ) -> i32 {
        let mut system = LanComputerSystem::new();
        system.type_ = type_;
        system.x = x;
        system.y = y;
        system.subnet = NumberGenerator::random_number(LAN_SUBNETRANGE);
        system.security = security;
        system.screen_index = screen_index;
        system.data_screen_index = data_screen_index;
        if type_ == LANSYSTEM_ROUTER {
            system.visible = LANSYSTEMVISIBLE_TYPE;
        }
        self.add_lan_system(system)
    }

    pub fn add_lan_link(
        &mut self,
        from: i32,
        from_x: f32,
        from_y: f32,
        to: i32,
        to_x: f32,
        to_y: f32,
        security: i32,
    ) -> i32 {
        let mut link = LanComputerLink::new();
        link.from = from;
        link.to = to;
        link.from_x = from_x;
        link.from_y = from_y;
        link.to_x = to_x;
        link.to_y = to_y;
        link.port = NumberGenerator::random_number(LAN_LINKPORTRANGE);
        link.security = security;
        self.links.put_data(link)
    }

    /// 1 = 90 deg, 2 = 180 deg, 3 = 270 deg
    pub fn rotate(&mut self, angle: i32) {
        let centre_x = self.centre_x;
        let centre_y = self.centre_y;

        //
        // Rotate all systems 90 degrees * angle

        for _ in 0..angle {
            for (_, system) in self.systems.iter_mut() {
                let temp_x = system.x - centre_x;
                let temp_y = system.y - centre_y;

                system.x = -temp_y + centre_x;
                system.y = temp_x + centre_y;
            }
        }

        //
        // Rotate all links 90 degrees * angle

        for _ in 0..angle {
            for (_, link) in self.links.iter_mut() {
                let mut temp_x = link.from_x;
                let mut temp_y = link.from_y;
                link.from_x = 1.0 - temp_y;
                link.from_y = temp_x;

                temp_x = link.to_x;
                temp_y = link.to_y;
                link.to_x = 1.0 - temp_y;
                link.to_y = temp_x;
            }
        }
    }

    /// `Merge ( LanCluster *target, LanComputer *comp )`: `comp_name` is the LAN computer in the world.
    ///
    /// The C++ shares the system pointers between this cluster and `target`, so the data remap
    /// applies to both copies; here it is applied to the source and to the target's copies.
    pub fn merge_cluster(&mut self, g: &mut Ctx, target: &mut LanCluster, comp_name: &str) {
        let source_systems: Vec<(i32, LanComputerSystem)> =
            self.systems.iter().map(|(i, system)| (i, system.clone())).collect();
        let system_count = self.systems.size();

        //
        // Make a store for the newly assigned indexes

        let mut new_indexes = vec![-1; system_count as usize];

        //
        // Transfer all systems

        for (i, system) in &source_systems {
            new_indexes[*i as usize] = target.add_lan_system(system.clone());

            if system.type_ == LANSYSTEM_MODEM {
                let target_screen_index = g
                    .world()
                    .get_computer(comp_name)
                    .map_or(-1, |comp| screen_next_page(comp, system.screen_index));

                for vl in g.world_mut().locations.values_mut() {
                    if let Some(special) = vl.special.as_mut() {
                        if vl.computer == comp_name
                            && special.screen_index == target_screen_index
                            && special.security_system_index < system_count
                        {
                            special.set_security_system_index(remap_index(
                                &new_indexes,
                                special.security_system_index,
                            ));
                            break;
                        }
                    }
                }
            }
        }

        //
        // Transfer all links

        for link in self.links.values() {
            target.add_lan_link(
                remap_index(&new_indexes, link.from),
                link.from_x,
                link.from_y,
                remap_index(&new_indexes, link.to),
                link.to_x,
                link.to_y,
                link.security,
            );
        }

        //
        // Update all data values

        for (i, _) in &source_systems {
            let target_index = new_indexes[*i as usize];
            if let Some(system) = self.systems.get_data_mut(*i) {
                remap_system(system, &new_indexes);
            }
            if let Some(system) = target.systems.get_data_mut(target_index) {
                remap_system(system, &new_indexes);
            }
        }

        self.input = remap_input_output(&new_indexes, self.input);
        self.output = remap_input_output(&new_indexes, self.output);
    }

    /// `Merge ( LanComputer *target )`: `target_name` is the LAN computer in the world.
    ///
    /// The computer's systems and links are moved out for the merge, which only reads the
    /// computer's screens and the world's locations, and put back afterwards.
    pub fn merge(&mut self, g: &mut Ctx, target_name: &str) {
        let (systems, links) = {
            let lan = g
                .world_mut()
                .get_computer_mut(target_name)
                .and_then(|comp| comp.lan_mut())
                .expect("LAN computer is not in the world");
            (
                std::mem::replace(&mut lan.systems, DArray::new()),
                std::mem::replace(&mut lan.links, DArray::new()),
            )
        };
        let mut target = LanCluster {
            systems,
            links,
            input: -1,
            output: -1,
            centre_x: 0,
            centre_y: 0,
        };

        self.merge_cluster(g, &mut target, target_name);

        let lan = g
            .world_mut()
            .get_computer_mut(target_name)
            .and_then(|comp| comp.lan_mut())
            .expect("LAN computer is not in the world");
        lan.systems = target.systems;
        lan.links = target.links;
    }

    pub fn set_input(&mut self, new_input: i32) {
        self.input = new_input;
    }

    pub fn set_output(&mut self, new_output: i32) {
        self.output = new_output;
    }
}

//
// Helpers for the port (no C++ equivalent)

/// The C++ istream calls the LAN loader makes: `>> token`, `>> ws`, `getline` and `>> int/float`.
/// A value that fails to parse reads as 0.
struct Cursor<'a> {
    text: &'a str,
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(text: &'a str) -> Self {
        Cursor { text, pos: 0 }
    }

    /// `>> ws`
    fn ws(&mut self) {
        let rest = &self.text[self.pos..];
        self.pos += rest.len() - rest.trim_start().len();
    }

    /// `>> token`: skips whitespace, then reads up to the next whitespace.
    fn token(&mut self) -> &'a str {
        self.ws();
        let rest = &self.text[self.pos..];
        let len = rest.find(char::is_whitespace).unwrap_or(rest.len());
        self.pos += len;
        &rest[..len]
    }

    fn int(&mut self) -> i32 {
        self.token().parse().unwrap_or(0)
    }

    fn float(&mut self) -> f32 {
        self.token().parse().unwrap_or(0.0)
    }

    /// `getline`: the rest of the line, consuming the newline. `None` at the end of the text.
    fn getline(&mut self) -> Option<&'a str> {
        if self.pos >= self.text.len() {
            return None;
        }
        let rest = &self.text[self.pos..];
        let (line, consumed) = match rest.find('\n') {
            Some(i) => (&rest[..i], i + 1),
            None => (rest, rest.len()),
        };
        self.pos += consumed;
        Some(line)
    }

    /// `getline` on what is left of the current line (no newline to consume).
    fn rest(&mut self) -> &'a str {
        let rest = &self.text[self.pos..];
        self.pos = self.text.len();
        rest
    }
}

/// `sscanf(buffer, "KEYWORD %d", &x)`: 0 when the line does not start with the keyword.
fn scan_keyword_int(line: &str, keyword: &str) -> i32 {
    let Some(rest) = line.strip_prefix(keyword) else {
        return 0;
    };
    let rest = rest.trim_start();
    let mut digits = String::new();
    for (i, c) in rest.chars().enumerate() {
        if (i == 0 && (c == '-' || c == '+')) || c.is_ascii_digit() {
            digits.push(c);
        } else {
            break;
        }
    }
    digits.parse().unwrap_or(0)
}

fn lan_computer_mut<'a>(g: &'a mut Ctx, comp_name: &str) -> &'a mut Computer {
    g.world_mut()
        .get_computer_mut(comp_name)
        .expect("LAN computer is not in the world")
}

fn company_name(g: &Ctx, comp_name: &str) -> String {
    g.world()
        .get_computer(comp_name)
        .expect("LAN computer is not in the world")
        .companyname
        .clone()
}

/// `comp->AddComputerScreen ( screen )`: returns the screen's index.
fn add_screen(g: &mut Ctx, comp_name: &str, screen: ComputerScreen) -> i32 {
    lan_computer_mut(g, comp_name).add_computer_screen(screen, -1)
}

/// `screen->SetNextPage ( next )` for a screen already on the computer.
fn set_next_page(comp: &mut Computer, index: i32, next_page: i32) {
    match comp.get_computer_screen_mut(index) {
        Some(ComputerScreen::MessageScreen(s)) => s.set_next_page(next_page),
        Some(ComputerScreen::PasswordScreen(s)) => s.set_next_page(next_page),
        Some(ComputerScreen::CypherScreen(s)) => s.set_next_page(next_page),
        Some(ComputerScreen::GenericScreen(s)) => s.set_next_page(next_page),
        Some(ComputerScreen::HighSecurityScreen(s)) => s.set_next_page(next_page),
        Some(ComputerScreen::LogScreen(s)) => s.set_next_page(next_page),
        _ => panic!("screen {} has no next page", index),
    }
}

fn dialog_mut(comp: &mut Computer, index: i32) -> &mut DialogScreen {
    comp.get_computer_screen_mut(index)
        .and_then(|cs| cs.as_dialog_screen_mut())
        .expect("dialog screen")
}

/// The next page of a modem's screen. The C++ casts the screen to a UserIDScreen; a modem's
/// screen is a PasswordScreen, whose `nextpage` is in the same place.
fn screen_next_page(comp: &Computer, index: i32) -> i32 {
    match comp.get_computer_screen(index) {
        Some(ComputerScreen::UserIDScreen(s)) => s.nextpage,
        Some(ComputerScreen::PasswordScreen(s)) => s.nextpage,
        _ => -1,
    }
}

/// The password for a terminal-style screen: easy (security 1), normal (2), otherwise complex.
fn password_for_security(g: &Ctx, security: i32) -> String {
    if security == 1 {
        NameGenerator::generate_easy_password()
    } else if security == 2 {
        NameGenerator::generate_password(g)
    } else {
        NameGenerator::generate_complex_password(g)
    }
}

/// `radio->data1 = MINRANGE + RandomNumber(...); radio->data2 = RandomNumber(10);`
fn set_radio_frequency(receiver: &mut LanComputerSystem) {
    receiver.data1 = RADIOTRANSMITTER_MINRANGE
        + NumberGenerator::random_number(RADIOTRANSMITTER_MAXRANGE - RADIOTRANSMITTER_MINRANGE);
    receiver.data2 = NumberGenerator::random_number(10);
}

/// `cluster->systems.GetData ( index )`
fn sys(cluster: &LanCluster, index: i32) -> &LanComputerSystem {
    cluster.systems.get_data(index).expect("LAN system")
}

fn sys_mut(cluster: &mut LanCluster, index: i32) -> &mut LanComputerSystem {
    cluster.systems.get_data_mut(index).expect("LAN system")
}

/// A copy of a cluster system, for `HideData ( comp, system, data )` (only `data_screen_index` is read).
fn snapshot(cluster: &LanCluster, index: i32) -> LanComputerSystem {
    sys(cluster, index).clone()
}

/// `HideData ( comp, system, data )` on the world's LAN computer.
fn hide_in_world(g: &mut Ctx, comp_name: &str, system: &LanComputerSystem, data: &str) -> bool {
    LanGenerator::hide_data_system(lan_computer_mut(g, comp_name), system, data)
}

/// Maps an old system index to its index after a merge. `-1` (and out-of-range values, which the
/// C++ reads out of bounds) map to `-1`.
fn remap_index(new_indexes: &[i32], old: i32) -> i32 {
    usize::try_from(old)
        .ok()
        .and_then(|i| new_indexes.get(i).copied())
        .unwrap_or(-1)
}

fn remap_input_output(new_indexes: &[i32], index: i32) -> i32 {
    if index != -1 {
        remap_index(new_indexes, index)
    } else {
        index
    }
}

fn remap_system(system: &mut LanComputerSystem, new_indexes: &[i32]) {
    if matches!(
        system.type_,
        LANSYSTEM_AUTHENTICATION | LANSYSTEM_ISOLATIONBRIDGE | LANSYSTEM_MAINSERVER
    ) {
        if system.data1 != -1 {
            system.data1 = remap_index(new_indexes, system.data1);
        }
        if system.data2 != -1 {
            system.data2 = remap_index(new_indexes, system.data2);
        }
        if system.data3 != -1 {
            system.data3 = remap_index(new_indexes, system.data3);
        }
    }

    for subnet in system.valid_subnets.iter_mut() {
        *subnet = remap_index(new_indexes, *subnet);
    }
}
