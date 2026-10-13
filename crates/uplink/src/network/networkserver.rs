//! Port of source/network/networkserver.h + source/network/networkserver.cpp

use std::net::{IpAddr, Ipv4Addr, Shutdown, TcpListener};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::ctx::Ctx;
use crate::network::clientconnection::ClientConnection;
use crate::network::networkclient::CLIENT_NONE;
use crate::tosser::DArray;

/// Host name of `ip` by reverse DNS (getnameinfo), or its dotted form when it has none.
#[cfg(unix)]
fn reverse_lookup(ip: Ipv4Addr) -> String {
    // SAFETY: `addr` is a valid sockaddr_in of the given length, `host` is a writable buffer of
    // the given length, and getnameinfo NUL-terminates what it writes there on success.
    unsafe {
        let mut addr: libc::sockaddr_in = std::mem::zeroed();
        addr.sin_family = libc::AF_INET as libc::sa_family_t;
        addr.sin_addr.s_addr = u32::from(ip).to_be();
        let mut host = [0 as libc::c_char; 1025];
        let rc = libc::getnameinfo(
            &addr as *const libc::sockaddr_in as *const libc::sockaddr,
            std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t,
            host.as_mut_ptr(),
            host.len() as libc::socklen_t,
            std::ptr::null_mut(),
            0,
            0,
        );
        if rc != 0 {
            return ip.to_string();
        }
        std::ffi::CStr::from_ptr(host.as_ptr()).to_string_lossy().into_owned()
    }
}

#[cfg(not(unix))]
fn reverse_lookup(ip: Ipv4Addr) -> String {
    ip.to_string()
}

/// Port the server listens on
const SERVER_PORT: u16 = 31337;

/// C `time(NULL)`: unix seconds
fn unix_time() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Server half of `Network` (`g.app.network.server`): listens on port 31337 and owns the clients.
#[derive(Debug, Default)]
pub struct NetworkServer {
    /// C++ `SOCKET listensocket` (None when not listening)
    listensocket: Option<TcpListener>,
    /// C++ `time_t lastlisten` (unix seconds)
    lastlisten: i64,
    listening: bool,
    /// C++ `DArray<ClientConnection *> clients`: owned; slot indices are stable and are the
    /// `index` each connection stores
    pub clients: DArray<ClientConnection>,
}

impl NetworkServer {
    /// C++ constructor
    pub fn new() -> Self {
        Self::default()
    }

    /// `StartServer`: open the listen socket on port 31337. Returns success.
    // PORT: tcp4u listened with a backlog of 5; std uses the OS default backlog.
    pub fn start_server(&mut self) -> bool {
        // Establish a listen socket
        // PORT: the listener is non-blocking because `update` polls it. tcp4u's TcpAccept waited
        // up to 1 s inside the game loop; here a connection waits in the kernel backlog until the
        // next 5 second check.
        let listener = TcpListener::bind(("0.0.0.0", SERVER_PORT)).and_then(|listener| {
            listener.set_nonblocking(true)?;
            Ok(listener)
        });
        self.lastlisten = unix_time();

        match listener {
            Ok(listener) => {
                self.listensocket = Some(listener);
                true
            }
            Err(_) => false,
        }
    }

    /// `StopServer`: close every client connection and empty `clients`.
    // PORT: the C++ left the listen socket open here, so port 31337 stayed bound: a second
    // StartServer failed ("failed to start server") and clients could still connect to a stopped
    // server. The listen socket is closed here instead.
    pub fn stop_server(&mut self) {
        // Close all open sockets
        for i in 0..self.clients.size() {
            if self.clients.valid_index(i) {
                let closed = match self.clients.get_data_mut(i).and_then(|cc| cc.socket.take()) {
                    Some(socket) => {
                        // TcpClose succeeds on any open socket; the peer may already be gone
                        let _ = socket.shutdown(Shutdown::Both);
                        true
                    }
                    None => false,
                };
                if !closed {
                    println!("NetworkServer::StopServer, failed to close connection {}", i);
                }
            }
        }

        self.clients.empty();

        self.listensocket = None;
        self.listening = false;
    }

    /// Peer IPv4 address of client `socketindex`. None when the slot is empty, has no socket, or
    /// the peer is not IPv4.
    fn remote_ipv4(&self, socketindex: i32) -> Option<Ipv4Addr> {
        let socket = self.clients.get_data(socketindex)?.socket.as_ref()?;
        match socket.peer_addr().ok()?.ip() {
            IpAddr::V4(v4) => Some(v4),
            IpAddr::V6(_) => None,
        }
    }

    /// `GetRemoteHost(socketindex)`: host name of a client, None on failure.
    // PORT: tcp4u's TcpGetRemoteID reverse-resolves the peer (gethostbyaddr); getnameinfo does the
    // same and gives the dotted address when the peer has no name.
    pub fn get_remote_host(&self, socketindex: i32) -> Option<String> {
        debug_assert!(self.clients.valid_index(socketindex));

        match self.remote_ipv4(socketindex) {
            Some(ip) => Some(reverse_lookup(ip)),
            None => {
                println!("NetworkServer::GetRemoteHost, failed to get remote host");
                None
            }
        }
    }

    /// `GetRemoteIP(socketindex)`: dotted IPv4 address of a client, None on failure.
    // PORT: the C++ printed each octet with %u from a signed char, so octets >= 128 came out as
    // 4294967xxx. The real dotted quad is produced here.
    pub fn get_remote_ip(&self, socketindex: i32) -> Option<String> {
        debug_assert!(self.clients.valid_index(socketindex));

        match self.remote_ipv4(socketindex) {
            Some(ip) => Some(ip.to_string()),
            None => {
                println!("NetworkServer::GetRemoteIP, failed to get remote ip");
                None
            }
        }
    }

    /// `Listen`: start accepting new connections in `update`
    pub fn listen(&mut self) {
        self.listening = true;
    }

    /// `StopListening`
    pub fn stop_listening(&mut self) {
        self.listening = false;
    }

    /// `Update`: accept a new connection (checked every 5 s) and update every client.
    /// Associated fn because the clients need the context. Iterate `clients` by index and re-check
    /// each slot: a client's update may remove itself from `clients`.
    pub fn update(g: &mut Ctx) {
        // Check the listen socket for new connections every 5 seconds
        let now = unix_time();
        let accept_due = {
            let server = &g.app.network.server;
            server.listening && server.listensocket.is_some() && now > server.lastlisten + 5
        };

        if accept_due {
            let accepted = match g.app.network.server.listensocket.as_ref() {
                Some(listener) => listener.accept().ok(),
                None => None,
            };

            if let Some((thesocket, _peer)) = accepted {
                // New connection requested - deal with it
                let connectiontime = g.ecl.get_accurate_time();

                let mut cc = ClientConnection::new();
                cc.set_socket(thesocket);
                cc.set_client_type(CLIENT_NONE);
                cc.set_connection_time(connectiontime);

                let server = &mut g.app.network.server;
                let index = server.clients.put_data(cc);
                if let Some(cc) = server.clients.get_data_mut(index) {
                    cc.set_index(index);
                }
            }

            g.app.network.server.lastlisten = unix_time();
        }

        // Read any input from the open sockets
        // And send out the neccisary data
        // (C++ UpdateDArray: a client's update may remove itself, so every slot is re-checked)
        let mut i = 0;
        while i < g.app.network.server.clients.size() {
            if g.app.network.server.clients.valid_index(i) {
                ClientConnection::update(g, i);
            }
            i += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reverse_lookup_gives_a_name_or_the_address() {
        let host = reverse_lookup(Ipv4Addr::LOCALHOST);
        assert!(!host.is_empty());
    }

    #[test]
    fn stop_server_releases_the_port_so_the_server_can_be_started_again() {
        let mut server = NetworkServer::new();
        // Another process (e.g. a running game) may hold the port; then there is nothing to test.
        if server.start_server() {
            server.stop_server();
            assert!(server.listensocket.is_none());
            assert!(server.start_server());
            server.stop_server();
        }
    }
}
