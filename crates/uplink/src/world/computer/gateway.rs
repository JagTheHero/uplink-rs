//! Port of source/world/computer/gateway.h + source/world/computer/gateway.cpp
//!
//! The player's Gateway computer. Lives at `player.gateway`, so every method that needs the world
//! (gateway definitions, messages) takes `g: &mut Ctx` and works on `g.player_mut().gateway`.

use serde::{Deserialize, Serialize};

use crate::app::globals;
use crate::ctx::Ctx;
use crate::game::data::data::{
    get_hardware_upgrade, ComputerUpgrade, NUM_STARTINGSOFTWAREUPGRADES, PLAYER_START_CPUTYPE,
    PLAYER_START_GATEWAYTYPE, PLAYER_START_MEMORYSIZE, PLAYER_START_MODEMSPEED, SOFTWARE_UPGRADES,
};
use crate::world::computer::databank::{
    Data, DataBank, DATATYPE_PROGRAM, SOFTWARETYPE_FILEUTIL, SOFTWARETYPE_OTHER,
};
use crate::world::message::Message;

pub const HUDUPGRADE_MAPSHOWSTRACE: i32 = 1;
pub const HUDUPGRADE_CONNECTIONANALYSIS: i32 = 2;
pub const HUDUPGRADE_IRCCLIENT: i32 = 4;
pub const HUDUPGRADE_LANVIEW: i32 = 8;

pub const GATEWAYUPGRADETYPE_CPU: i32 = 1;
pub const GATEWAYUPGRADETYPE_MODEM: i32 = 2;
pub const GATEWAYUPGRADETYPE_COOLING: i32 = 3;
pub const GATEWAYUPGRADETYPE_MEMORY: i32 = 4;
pub const GATEWAYUPGRADETYPE_SECURITY: i32 = 5;
pub const GATEWAYUPGRADETYPE_POWER: i32 = 6;

/// `GetHardwareUpgrade(name)` followed by `UplinkAssert(cu)`: every installed item must be known.
fn hw_upgrade(name: &str) -> &'static ComputerUpgrade {
    get_hardware_upgrade(name).expect("Gateway: installed hardware item has no upgrade definition")
}

/// `Gateway::GetCPUs`-style scan with the "slowest" rule shared by ExchangeGatewayComplete and
/// GiveCPU: the first CPU wins ties, a strictly slower CPU replaces it.
fn slowest_cpu_index(hardware: &[String]) -> Option<usize> {
    let mut slowest: Option<(usize, &'static ComputerUpgrade)> = None;
    for (i, name) in hardware.iter().enumerate() {
        let cu = hw_upgrade(name);
        if cu.type_ == GATEWAYUPGRADETYPE_CPU {
            let replace = match slowest {
                None => true,
                Some((_, best)) => cu.data < best.data,
            };
            if replace {
                slowest = Some((i, cu));
            }
        }
    }
    slowest.map(|(i, _)| i)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Gateway {
    /// Bitfield of HUDUPGRADE_* (C++ `char hudupgrades`, protected)
    hudupgrades: u8,

    pub databank: DataBank,

    /// Unique ID number
    pub id: i32,
    /// Indexes into the list of Gateways (set to -1 by `set_type`: `name` identifies the gateway def now)
    pub type_: i32,
    /// New type recently purchased, awaiting log out. Or -1.
    pub newtype: i32,
    /// Has the player blown it up?
    pub nuked: bool,
    /// Number of people nearby
    pub proximity: i32,

    /// Replaces type field (indexes are no longer fixed)
    pub name: String,

    pub modemtype: String,

    pub modemspeed: i32,
    pub memorysize: i32,

    /// Names of all hardware items owned
    pub hardware: Vec<String>,
}

impl Default for Gateway {
    fn default() -> Self {
        Gateway {
            hudupgrades: 0,
            databank: DataBank::default(),
            id: 0,
            type_: 0,
            newtype: -1,
            nuked: false,
            proximity: 0,
            name: " ".to_string(),
            modemtype: " ".to_string(),
            modemspeed: 0,
            memorysize: 0,
            hardware: Vec::new(),
        }
    }
}

impl Gateway {
    pub fn new() -> Self {
        Self::default()
    }

    /// Call when gateway is changed
    pub fn generate_new_id(&mut self) {
        self.id += 1;
    }

    /// Call when gateway is nuked: resets software, hardware and the gateway type (player's gateway).
    pub fn nuke(g: &mut Ctx) {
        //
        // Reset software
        //

        {
            let gw = &mut g.player_mut().gateway;
            gw.databank.format();
            gw.databank.set_size(PLAYER_START_MEMORYSIZE);
            gw.give_starting_software();
            gw.hudupgrades = 0;
        }

        //
        // Reset all of the hardware items
        //

        Gateway::set_type(g, PLAYER_START_GATEWAYTYPE);

        g.player_mut().gateway.hardware.clear();

        Gateway::give_starting_hardware(g);

        //
        // Change the Gateway ID  (so any SeizeGatewayEvents will now fail)
        //

        g.player_mut().gateway.generate_new_id();

        //
        // Set it to be nuked, to remind us next time we log in
        //

        let gw = &mut g.player_mut().gateway;
        gw.proximity = 0;
        gw.nuked = true;
    }

    /// Someone approaches
    pub fn increment_proximity(&mut self, n: i32) {
        self.proximity += n;
    }

    /// Someone leaves
    pub fn decrement_proximity(&mut self, n: i32) {
        self.proximity -= n;
        if self.proximity < 0 {
            self.proximity = 0;
        }
    }

    /// Looks the gateway definition up in the world, copies its name and sets `type_ = -1`
    /// (player's gateway).
    pub fn set_type(g: &mut Ctx, newtype: i32) {
        let newname = g
            .world()
            .get_gateway_def(newtype, None)
            .expect("Gateway::SetType: no gateway definition for this type")
            .name
            .clone();

        let gw = &mut g.player_mut().gateway;
        gw.name = newname;
        gw.type_ = -1;
    }

    pub fn set_modem_type(&mut self, newmodem: &str, newmodemspeed: i32) {
        self.modemtype = newmodem.to_string();
        self.modemspeed = newmodemspeed;
    }

    pub fn set_memory_size(&mut self, newsize: i32) {
        self.memorysize = newsize;
        self.databank.set_size(self.memorysize);
    }

    pub fn change_memory_size(&mut self, amount: i32) {
        self.set_memory_size(self.memorysize + amount);
    }

    /// Call this when the gateway upgrade is ready
    pub fn exchange_gateway(&mut self, newgatewaytype: i32) {
        self.newtype = newgatewaytype;
    }

    /// Does the actual upgrade (player's gateway); may send the player a Message listing removed items.
    pub fn exchange_gateway_complete(g: &mut Ctx) {
        let oldtype = g.player().gateway.type_;
        let old_name = g.player().gateway.name.clone();

        let newtype = g.player().gateway.newtype;
        Gateway::set_type(g, newtype);
        {
            let gw = &mut g.player_mut().gateway;
            gw.newtype = -1;
            gw.nuked = false;
            gw.generate_new_id();
        }

        //
        // Handle loss of hardware or software
        //

        // UplinkAssert (oldgateway) in the C++
        g.world()
            .get_gateway_def(oldtype, Some(&old_name))
            .expect("Gateway::ExchangeGatewayComplete: old gateway definition missing");

        let (new_type, new_name) = {
            let gw = &g.player().gateway;
            (gw.type_, gw.name.clone())
        };
        let (maxcpus, maxmemory, maxsecurity) = {
            let newgateway = g
                .world()
                .get_gateway_def(new_type, Some(&new_name))
                .expect("Gateway::ExchangeGatewayComplete: new gateway definition missing");
            (newgateway.maxcpus, newgateway.maxmemory, newgateway.maxsecurity)
        };

        let mut removed_items: Vec<String> = Vec::new();

        //
        // NumCPUS - remove the slowest until we have the right number
        //

        while maxcpus < g.player().gateway.get_num_cpus() {
            let gw = &mut g.player_mut().gateway;
            match slowest_cpu_index(&gw.hardware) {
                Some(index_slowest) => {
                    let upgrade_name = hw_upgrade(&gw.hardware[index_slowest]).name;
                    removed_items.push(upgrade_name.to_string());
                    gw.hardware.remove(index_slowest);
                }
                None => panic!("Gateway::ExchangeGatewayComplete, there should be a slowest CPU"),
            }
        }

        //
        // Memory - remove the last N GQs until the right amount is reached
        //

        if maxmemory * 8 < g.player().gateway.databank.get_size() {
            let gw = &mut g.player_mut().gateway;

            for i in (maxmemory * 8)..gw.memorysize {
                gw.databank.remove_data(i);
            }

            removed_items.push(format!(
                "{}Gq of memory",
                gw.memorysize - (maxmemory * 8)
            ));

            gw.set_memory_size(maxmemory * 8);
        }

        //
        // Security - Remove N security items, first come first serve ;)
        //

        let mut num_security = g.player().gateway.get_num_security();

        while maxsecurity < num_security {
            let gw = &mut g.player_mut().gateway;

            // Same index walk as the C++ for loop: after a removal the next item is skipped
            let mut i = 0;
            while i < gw.hardware.len() {
                let cu = hw_upgrade(&gw.hardware[i]);
                if cu.type_ == GATEWAYUPGRADETYPE_SECURITY {
                    removed_items.push(cu.name.to_string());
                    gw.hardware.remove(i);
                }
                i += 1;
            }

            num_security = g.player().gateway.get_num_security();
        }

        //
        // Tell the player what we removed
        //

        if !removed_items.is_empty() {
            let mut removed_text = String::from(
                "Unfortunately, your new gateway does not have enough space to contain \
                 all of your old hardware.  As such, we have been forced to remove the \
                 following items from your new system :\n\n",
            );

            for item in &removed_items {
                removed_text.push_str(item);
                removed_text.push('\n');
            }

            let mut msg = Message::new();
            msg.set_to("PLAYER");
            msg.set_from("Uplink Corporation");
            msg.set_subject("Removed hardware items");
            msg.set_body(&removed_text);
            msg.send(g);
        }
    }

    /// `upgrade` is one of HUDUPGRADE_*
    pub fn give_hud_upgrade(&mut self, upgrade: i32) {
        self.hudupgrades |= upgrade as u8;
    }

    pub fn has_hud_upgrade(&self, upgrade: i32) -> bool {
        (self.hudupgrades & upgrade as u8) != 0
    }

    /// Returns the bitfield - be careful!
    pub fn get_hud_upgrades(&self) -> u8 {
        self.hudupgrades
    }

    pub fn has_any_hud_upgrade(&self) -> bool {
        self.hudupgrades != 0
    }

    /// Adds the CPU, or replaces the slowest one when the gateway has its maximum number of CPUs.
    pub fn give_cpu(g: &mut Ctx, cpu_name: &str) {
        let (gw_type, gw_name) = {
            let gw = &g.player().gateway;
            (gw.type_, gw.name.clone())
        };
        let maxcpus = g
            .world()
            .get_gateway_def(gw_type, Some(&gw_name))
            .expect("Gateway::GiveCPU: gateway definition missing")
            .maxcpus;

        let gw = &mut g.player_mut().gateway;

        if gw.get_num_cpus() < maxcpus {
            // Room for another CPU
            gw.hardware.push(cpu_name.to_string());
        } else {
            // The player already has a full set of CPUs
            // So replace his slowest
            match slowest_cpu_index(&gw.hardware) {
                Some(index_slowest) => {
                    gw.hardware[index_slowest] = cpu_name.to_string();
                }
                None => panic!("Gateway::GiveCPU, there should be a slowest CPU"),
            }
        }
    }

    pub fn get_num_cpus(&self) -> i32 {
        self.hardware
            .iter()
            .filter(|name| hw_upgrade(name).type_ == GATEWAYUPGRADETYPE_CPU)
            .count() as i32
    }

    pub fn get_cpu_speed(&self) -> i32 {
        let mut speed = 0;
        for name in &self.hardware {
            let cu = hw_upgrade(name);
            if cu.type_ == GATEWAYUPGRADETYPE_CPU {
                speed += cu.data;
            }
        }
        speed
    }

    /// Names of the installed CPUs.
    pub fn get_cpus(&self) -> Vec<String> {
        self.hardware
            .iter()
            .filter(|name| hw_upgrade(name).type_ == GATEWAYUPGRADETYPE_CPU)
            .cloned()
            .collect()
    }

    /// Returns min ( modemspeed, gatewaydef.bandwidth )  (player's gateway)
    pub fn get_bandwidth(g: &Ctx) -> i32 {
        let gw = &g.player().gateway;
        let gatewaydef = g
            .world()
            .get_gateway_def(gw.type_, Some(&gw.name))
            .expect("Gateway::GetBandwidth: gateway definition missing");

        if gw.modemspeed < gatewaydef.bandwidth {
            gw.modemspeed
        } else {
            gatewaydef.bandwidth
        }
    }

    pub fn get_num_security(&self) -> i32 {
        self.hardware
            .iter()
            .filter(|name| hw_upgrade(name).type_ == GATEWAYUPGRADETYPE_SECURITY)
            .count() as i32
    }

    /// Names of the installed security devices.
    pub fn get_security(&self) -> Vec<String> {
        self.hardware
            .iter()
            .filter(|name| hw_upgrade(name).type_ == GATEWAYUPGRADETYPE_SECURITY)
            .cloned()
            .collect()
    }

    pub fn give_starting_hardware(g: &mut Ctx) {
        Gateway::set_type(g, PLAYER_START_GATEWAYTYPE);

        // CPU

        Gateway::give_cpu(g, PLAYER_START_CPUTYPE);

        // Modem

        let modemname = format!("Modem ({} Ghz)", PLAYER_START_MODEMSPEED);
        let gw = &mut g.player_mut().gateway;
        gw.set_modem_type(&modemname, PLAYER_START_MODEMSPEED);

        // Memory

        gw.set_memory_size(PLAYER_START_MEMORYSIZE);
    }

    pub fn give_hardware(&mut self, newhardware: &str) {
        self.hardware.push(newhardware.to_string());
    }

    pub fn is_hw_installed(&self, name: &str) -> bool {
        self.hardware.iter().any(|h| h == name)
    }

    pub fn give_starting_software(&mut self) {
        let mut copier = Data::new();
        copier.set_title("File_Copier");
        copier.set_details(DATATYPE_PROGRAM, 1, 0, 0, 1.0, SOFTWARETYPE_FILEUTIL);
        self.databank.put_data_at(copier, 14);

        let mut deleter = Data::new();
        deleter.set_title("File_Deleter");
        deleter.set_details(DATATYPE_PROGRAM, 1, 0, 0, 1.0, SOFTWARETYPE_FILEUTIL);
        self.databank.put_data_at(deleter, 15);

        let mut tutorial = Data::new();
        tutorial.set_title("Tutorial");
        tutorial.set_details(DATATYPE_PROGRAM, 4, 0, 0, 1.0, SOFTWARETYPE_OTHER);
        self.databank.put_data_at(tutorial, 17);

        if globals::TESTGAME {
            let mut defrag = Data::new();
            defrag.set_title("Defrag");
            defrag.set_details(DATATYPE_PROGRAM, 2, 0, 0, 1.0, SOFTWARETYPE_FILEUTIL);
            self.databank.put_data_at(defrag, 4);
        }
    }

    pub fn give_all_software(&mut self) {
        for upgrade in SOFTWARE_UPGRADES.iter().take(NUM_STARTINGSOFTWAREUPGRADES as usize) {
            let mut prog = Data::new();
            prog.set_title(upgrade.name);
            prog.set_details(
                DATATYPE_PROGRAM,
                upgrade.size,
                0,
                0,
                upgrade.data as f32,
                upgrade.type_,
            );
            self.databank.put_data(prog);
        }

        // Give all upgrades

        self.hudupgrades = 255;
    }

    pub fn give_all_hardware(g: &mut Ctx) {
        // Best gateway

        let best_gateway_index = g.world().gatewaydefs.size() - 1;
        let (maxmemory, bandwidth) = {
            let gateway_def = g
                .world()
                .get_gateway_def(best_gateway_index, None)
                .expect("Gateway::GiveAllHardware: no best gateway definition");
            (gateway_def.maxmemory, gateway_def.bandwidth)
        };

        Gateway::set_type(g, best_gateway_index);

        // Some big CPUs

        for _ in 0..4 {
            Gateway::give_cpu(g, "CPU ( 150 Ghz )");
        }

        // Max all stats

        let gw = &mut g.player_mut().gateway;
        gw.set_memory_size(maxmemory * 8);
        gw.set_modem_type("Fast fucker", bandwidth);
    }

    pub fn update(&mut self) {
        self.databank.update();
    }
}
