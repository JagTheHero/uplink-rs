//! Port of source/world/computer/gatewaydef.h + source/world/computer/gatewaydef.cpp
//!
//! Complete definition of a gateway: size, position of CPUs, max memory etc.

use std::io::BufRead;

use serde::{Deserialize, Serialize};

use crate::tosser::DArray;

/// C++ `isspace` in the "C" locale.
fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C)
}

/// C++ `thefile >> ws`: skips whitespace (possibly across several buffers).
fn skip_ws(thefile: &mut dyn BufRead) {
    loop {
        let (taken, at_end) = {
            let buf = match thefile.fill_buf() {
                Ok(b) => b,
                Err(_) => return,
            };
            if buf.is_empty() {
                return;
            }
            let n = buf.iter().take_while(|&&c| is_space(c)).count();
            (n, n == buf.len())
        };
        thefile.consume(taken);
        if !at_end {
            return;
        }
    }
}

/// C++ `thefile >> word` for a string: skips whitespace, then reads up to the next whitespace.
fn read_token(thefile: &mut dyn BufRead) -> String {
    skip_ws(thefile);
    let mut token: Vec<u8> = Vec::new();
    loop {
        let (taken, at_end) = {
            let buf = match thefile.fill_buf() {
                Ok(b) => b,
                Err(_) => break,
            };
            if buf.is_empty() {
                break;
            }
            let n = buf.iter().take_while(|&&c| !is_space(c)).count();
            token.extend_from_slice(&buf[..n]);
            (n, n < buf.len())
        };
        thefile.consume(taken);
        if at_end {
            break;
        }
    }
    String::from_utf8_lossy(&token).into_owned()
}

/// C++ `thefile >> int`. PORT: a token that is not a number reads as 0 (C++ would fail the stream).
fn read_int(thefile: &mut dyn BufRead) -> i32 {
    read_token(thefile).parse().unwrap_or(0)
}

/// C++ `thefile.getline(buf, n)`: the rest of the current line without its newline. A trailing CR is
/// dropped, because the definitions were written in Windows text mode.
fn getline(thefile: &mut dyn BufRead) -> String {
    let mut line: Vec<u8> = Vec::new();
    let _ = thefile.read_until(b'\n', &mut line);
    if line.last() == Some(&b'\n') {
        line.pop();
    }
    if line.last() == Some(&b'\r') {
        line.pop();
    }
    String::from_utf8_lossy(&line).into_owned()
}

pub const SIZE_GATEWAY_CPU_W: i32 = 32;
pub const SIZE_GATEWAY_CPU_H: i32 = 32;

pub const SIZE_GATEWAY_MEM_W: i32 = 10;
pub const SIZE_GATEWAY_MEM_H: i32 = 25;

pub const SIZE_GATEWAY_SEC_W: i32 = 32;
pub const SIZE_GATEWAY_SEC_H: i32 = 32;

pub const SIZE_GATEWAY_MODEM_W: i32 = 50;
pub const SIZE_GATEWAY_MODEM_H: i32 = 20;

pub const SIZE_GATEWAY_POWER_W: i32 = 40;
pub const SIZE_GATEWAY_POWER_H: i32 = 30;

/// Physical layout information.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct GatewayDefLocation {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GatewayDef {
    // id information
    pub name: String,
    pub description: String,
    pub filename: String,
    pub thumbnail: String,
    pub index: i32,

    // Game data definitions
    pub cost: i32,
    pub maxcpus: i32,
    pub maxmemory: i32,
    pub maxupgrades: i32,
    pub maxsecurity: i32,
    pub bandwidth: i32,

    // Physical dimensions and layout
    pub width: i32,
    pub height: i32,

    pub cpus: DArray<GatewayDefLocation>,
    pub memory: DArray<GatewayDefLocation>,
    pub security: DArray<GatewayDefLocation>,

    pub modem_x: i32,
    pub modem_y: i32,
    pub power_x: i32,
    pub power_y: i32,
}

impl Default for GatewayDef {
    fn default() -> Self {
        GatewayDef {
            name: "UnNamed".to_string(),
            description: "UnDescribed ;)".to_string(),
            filename: "None".to_string(),
            thumbnail: "None".to_string(),
            index: 0,
            cost: 0,
            maxcpus: 0,
            maxmemory: 0,
            maxupgrades: 0,
            maxsecurity: 0,
            bandwidth: 0,
            width: 0,
            height: 0,
            cpus: DArray::new(),
            memory: DArray::new(),
            security: DArray::new(),
            modem_x: 0,
            modem_y: 0,
            power_x: 0,
            power_y: 0,
        }
    }
}

impl GatewayDef {
    pub fn new() -> Self {
        Self::default()
    }

    // Loaders

    /// Parses one gateway definition: name line, whitespace separated numbers
    /// (cost maxcpus maxmemory maxupgrades maxsecurity bandwidth width height), description line,
    /// then the CPU / memory / security / modem / power position sections.
    pub fn load_gateway_definition(&mut self, thefile: &mut dyn std::io::BufRead) {
        // Basics

        self.name = getline(thefile);

        self.cost = read_int(thefile);
        self.maxcpus = read_int(thefile);
        self.maxmemory = read_int(thefile);
        self.maxupgrades = read_int(thefile);
        self.maxsecurity = read_int(thefile);
        self.bandwidth = read_int(thefile);
        self.width = read_int(thefile);
        self.height = read_int(thefile);
        skip_ws(thefile);

        self.description = getline(thefile);

        // CPU positions

        self.cpus.set_size(self.maxcpus);

        if self.maxcpus > 0 {
            read_token(thefile); // the section header (unused)
            skip_ws(thefile);

            for ic in 0..self.maxcpus {
                let x = read_int(thefile);
                let y = read_int(thefile);
                skip_ws(thefile);
                self.cpus.put_data_at(GatewayDefLocation { x, y }, ic);
            }
        }

        // Memory positions

        if self.maxmemory > 0 {
            self.memory.set_size(self.maxmemory);

            read_token(thefile); // the section header (unused)
            skip_ws(thefile);

            for i in 0..self.maxmemory {
                let x = read_int(thefile);
                let y = read_int(thefile);
                skip_ws(thefile);
                self.memory.put_data_at(GatewayDefLocation { x, y }, i);
            }
        }

        // Security positions

        if self.maxsecurity > 0 {
            self.security.set_size(self.maxsecurity);

            read_token(thefile); // the section header (unused)
            skip_ws(thefile);

            for i in 0..self.maxsecurity {
                let x = read_int(thefile);
                let y = read_int(thefile);
                skip_ws(thefile);
                self.security.put_data_at(GatewayDefLocation { x, y }, i);
            }
        }

        // Modem position

        read_token(thefile); // the section header (unused)
        self.modem_x = read_int(thefile);
        self.modem_y = read_int(thefile);
        skip_ws(thefile);

        // Power position

        read_token(thefile); // the section header (unused)
        self.power_x = read_int(thefile);
        self.power_y = read_int(thefile);
        skip_ws(thefile);
    }

    // Basic data set functions

    pub fn set_name(&mut self, newname: &str) {
        self.name = newname.to_string();
    }

    pub fn set_description(&mut self, newdescription: &str) {
        self.description = newdescription.to_string();
    }

    pub fn set_index(&mut self, newindex: i32) {
        self.index = newindex;
    }

    pub fn set_filename(&mut self, newfilename: &str) {
        self.filename = newfilename.to_string();
    }

    pub fn set_thumbnail(&mut self, newthumbnail: &str) {
        self.thumbnail = newthumbnail.to_string();
    }

    pub fn set_cost(&mut self, newcost: i32) {
        self.cost = newcost;
    }

    pub fn set_max_memory(&mut self, newmaxmemory: i32) {
        self.maxmemory = newmaxmemory;
    }

    pub fn set_max_upgrades(&mut self, newmaxupgrades: i32) {
        self.maxupgrades = newmaxupgrades;
    }

    pub fn set_max_security(&mut self, newmaxsecurity: i32) {
        self.maxsecurity = newmaxsecurity;
    }

    pub fn set_bandwidth(&mut self, newbandwidth: i32) {
        self.bandwidth = newbandwidth;
    }

    // Physical location functions

    pub fn set_size(&mut self, new_w: i32, new_h: i32) {
        self.width = new_w;
        self.height = new_h;
    }

    pub fn set_modem_position(&mut self, new_x: i32, new_y: i32) {
        self.modem_x = new_x;
        self.modem_y = new_y;
    }

    pub fn set_power_position(&mut self, new_x: i32, new_y: i32) {
        self.power_x = new_x;
        self.power_y = new_y;
    }
}
