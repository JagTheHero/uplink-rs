//! Port of source/network/network.h + source/network/network.cpp
//!
//! The C++ base `UplinkObject` is not ported (PORTING.md §6): Load/Save/Print/GetID are replaced by
//! serde/Debug. `Network` lives at `g.app.network`.

use std::net::{IpAddr, Ipv4Addr, ToSocketAddrs};

use crate::app::globals;
use crate::ctx::Ctx;
use crate::network::networkclient::NetworkClient;
use crate::network::networkserver::NetworkServer;

// Status of network
pub const NETWORK_NONE: i32 = 0;
pub const NETWORK_CLIENT: i32 = 1;
pub const NETWORK_SERVER: i32 = 2;

#[derive(Default)]
pub struct Network {
    pub server: NetworkServer,
    pub client: NetworkClient,
    /// C++ `STATUS`: one of NETWORK_NONE / NETWORK_CLIENT / NETWORK_SERVER
    pub status: i32,
}

/// `TcpGetLocalID`: this machine's host name and its IPv4 address. std has no gethostname, so the
/// name is read from the kernel and the address comes from resolving that name (gethostbyname in
/// tcp4u). None when either step fails.
fn local_id() -> Option<(String, Ipv4Addr)> {
    let host = std::fs::read_to_string("/proc/sys/kernel/hostname")
        .ok()
        .or_else(|| std::env::var("HOSTNAME").ok())?
        .trim()
        .to_string();
    if host.is_empty() {
        return None;
    }
    let ip = (host.as_str(), 0u16)
        .to_socket_addrs()
        .ok()?
        .find_map(|addr| match addr.ip() {
            IpAddr::V4(v4) => Some(v4),
            IpAddr::V6(_) => None,
        })?;
    Some((host, ip))
}

impl Network {
    /// C++ constructor. `Tcp4uInit` has no std::net equivalent (std needs no setup), so it is not
    /// ported and the status stays NETWORK_NONE.
    pub fn new() -> Self {
        if globals::DOCLABRELEASE {
            // PORT: the DOCLABRELEASE build mails "User X used Doc release of Uplink" to the
            // developers through smtp4u (SmtpSendMessage), via a long-gone university SMTP server.
            // Deliberately not ported (smtp4u is not ported, and the port never sends mail); the
            // flag is off in the FULLGAME build, so the C++ sent nothing here either.
        }
        Self::default()
    }

    /// `SetStatus`
    pub fn set_status(&mut self, new_status: i32) {
        self.status = new_status;
    }

    /// `GetServer`. The C++ asserted `STATUS == NETWORK_SERVER` before returning.
    pub fn get_server(&self) -> &NetworkServer {
        debug_assert!(self.status == NETWORK_SERVER);
        &self.server
    }

    /// Mutable form of `GetServer` (C++ returned a non-const pointer).
    pub fn get_server_mut(&mut self) -> &mut NetworkServer {
        debug_assert!(self.status == NETWORK_SERVER);
        &mut self.server
    }

    /// `GetClient`. The C++ asserted `STATUS == NETWORK_CLIENT` before returning.
    pub fn get_client(&self) -> &NetworkClient {
        debug_assert!(self.status == NETWORK_CLIENT);
        &self.client
    }

    /// Mutable form of `GetClient`.
    pub fn get_client_mut(&mut self) -> &mut NetworkClient {
        debug_assert!(self.status == NETWORK_CLIENT);
        &mut self.client
    }

    /// `GetLocalHost`: host name of this machine, None on failure.
    pub fn get_local_host(&self) -> Option<String> {
        match local_id() {
            Some((host, _)) => Some(host),
            None => {
                println!("Network::GetLocalHost, failed to get local host");
                None
            }
        }
    }

    /// `GetLocalIP`: dotted IPv4 address of this machine, None on failure.
    // PORT: the C++ printed each octet with %u from a signed char, so octets >= 128 came out as
    // 4294967xxx. The real dotted quad is produced here.
    pub fn get_local_ip(&self) -> Option<String> {
        match local_id() {
            Some((_, ip)) => Some(ip.to_string()),
            None => {
                println!("Network::GetLocalIP, failed to get local ip");
                None
            }
        }
    }

    /// `StartServer`: only from NETWORK_NONE; sets status to NETWORK_SERVER on success.
    pub fn start_server(&mut self) {
        match self.status {
            NETWORK_NONE => {
                let result = self.server.start_server();
                if result {
                    self.status = NETWORK_SERVER;
                } else {
                    println!("Network::StartServer, failed to start server");
                }
            }
            NETWORK_CLIENT => {
                println!("Network::StartServer, Cannot start server when running as a client");
            }
            NETWORK_SERVER => {
                println!("Network::StartServer, Cannot start server when server is already running");
            }
            _ => {}
        }
    }

    /// `StopServer`: only from NETWORK_SERVER; sets status to NETWORK_NONE.
    pub fn stop_server(&mut self) {
        if self.status == NETWORK_SERVER {
            self.server.stop_server();
            self.status = NETWORK_NONE;
        }
    }

    /// `StartClient(ip)`: only from NETWORK_NONE; sets status to NETWORK_CLIENT on success.
    pub fn start_client(&mut self, ip: &str) {
        match self.status {
            NETWORK_NONE => {
                let result = self.client.start_client(ip);
                if result {
                    self.status = NETWORK_CLIENT;
                } else {
                    println!("Network::StartClient, failed to start client");
                }
            }
            NETWORK_CLIENT => {
                println!("Network::StartClient, Cannot start client when running as a client");
            }
            NETWORK_SERVER => {
                println!("Network::StartClient, Cannot start client when server is already running");
            }
            _ => {}
        }
    }

    /// `StopClient`: only from NETWORK_CLIENT; sets status to NETWORK_NONE on success.
    pub fn stop_client(&mut self) {
        if self.status == NETWORK_CLIENT {
            let result = self.get_client_mut().stop_client();
            if !result {
                println!("Network::StopClient, failed to stop client");
            } else {
                self.status = NETWORK_NONE;
            }
        }
    }

    /// `IsActive`
    pub fn is_active(&self) -> bool {
        self.status != NETWORK_NONE
    }

    /// `Update`: runs the server or client half. The halves call into the game and the eclipse
    /// clock, so this is an associated fn taking the context (`Network` is at `g.app.network`).
    pub fn update(g: &mut Ctx) {
        match g.app.network.status {
            NETWORK_SERVER => NetworkServer::update(g),
            NETWORK_CLIENT => NetworkClient::update(g),
            _ => {}
        }
    }
}
