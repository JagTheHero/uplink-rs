//! Port of source/network/clientconnection.h + source/network/clientconnection.cpp

use std::io::{ErrorKind, Read, Write};
use std::net::TcpStream;

use crate::ctx::Ctx;
use crate::game::data::data::IP_LOCALHOST;
use crate::game::game::GAMESPEED_PAUSED;
use crate::network::networkclient::{CLIENT_COMMS, CLIENT_NONE, CLIENT_STATUS};
use crate::tosser::DArray;
use crate::world::computer::bankaccount::BankAccount;
use crate::world::computer::gateway::{
    Gateway, HUDUPGRADE_CONNECTIONANALYSIS, HUDUPGRADE_IRCCLIENT, HUDUPGRADE_LANVIEW,
    HUDUPGRADE_MAPSHOWSTRACE,
};
use crate::world::date::Date;
use crate::world::generator::recordgenerator::RecordGenerator;
use crate::world::rating::Rating;

/// C++ `char thebuffer [128]` in `ClientConnection::Update`: the largest message read from a client.
const CLIENT_RECV_BUFFER: usize = 128;

/// Outcome of a read from a socket (the `TCP4U_*` results of `TcpRecvUntilStr`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RecvStatus {
    Success,
    SocketClosed,
    Overflow,
    Error,
}

/// C++ `TcpSend` of `bytes`: a blocking write. The socket is put in non-blocking mode again after
/// the write, because reads on it are non-blocking. Returns false on failure.
pub(crate) fn tcp_send_bytes(socket: &mut TcpStream, bytes: &[u8]) -> bool {
    if socket.set_nonblocking(false).is_err() {
        return false;
    }
    let sent = socket.write_all(bytes).is_ok();
    let _ = socket.set_nonblocking(true);
    sent
}

/// Sends a server message: `body`, then "\n\r#" and the NUL that ends it. That is C++
/// `(char) 10 << (char) 13 << '#' << '\x0'` followed by `TcpSend(strlen+1)`.
pub(crate) fn tcp_send_message(socket: &mut TcpStream, body: &str) -> bool {
    let mut bytes = Vec::with_capacity(body.len() + 4);
    bytes.extend_from_slice(body.as_bytes());
    bytes.extend_from_slice(b"\n\r#\0");
    tcp_send_bytes(socket, &bytes)
}

/// C++ `TcpRecvUntilStr(socket, buffer, &len, "#", 2, ...)` without blocking. Returns the message
/// up to and including its "#" (the NUL terminator is not part of the string) once it has fully
/// arrived. While it is incomplete nothing is consumed and the string is empty.
/// `capacity` is the C buffer size: the message and its NUL must fit in it.
pub(crate) fn tcp_recv_until_hash(socket: &mut TcpStream, capacity: usize) -> (String, RecvStatus) {
    if socket.set_nonblocking(true).is_err() {
        return (String::new(), RecvStatus::Error);
    }
    let mut peek = vec![0u8; capacity - 1];
    let available = match socket.peek(&mut peek) {
        Ok(0) => return (String::new(), RecvStatus::SocketClosed),
        Ok(n) => n,
        Err(e) if e.kind() == ErrorKind::WouldBlock => return (String::new(), RecvStatus::Success),
        Err(_) => return (String::new(), RecvStatus::Error),
    };

    if let Some(pos) = peek[..available].windows(2).position(|w| w == b"#\0") {
        let mut message = vec![0u8; pos + 2];
        if socket.read_exact(&mut message).is_err() {
            return (String::new(), RecvStatus::Error);
        }
        return (cstr_to_string(&message), RecvStatus::Success);
    }

    if available == capacity - 1 {
        // Buffer full with no terminator: C++ TCP4U_OVERFLOW
        let mut message = vec![0u8; available];
        if socket.read_exact(&mut message).is_err() {
            return (String::new(), RecvStatus::Error);
        }
        return (cstr_to_string(&message), RecvStatus::Overflow);
    }

    (String::new(), RecvStatus::Success)
}

/// The text up to the first NUL, as in a C string.
fn cstr_to_string(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

/// C++ `sscanf(s, "%d")`: optional sign then digits from the start of `s`; 0 when there are none.
fn c_int(s: &str) -> i32 {
    let end = s
        .char_indices()
        .find(|&(i, c)| !(c.is_ascii_digit() || (i == 0 && (c == '-' || c == '+'))))
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    s[..end].parse().unwrap_or(0)
}

/// The client at `index`. C++ dereferences the pointer, so a missing entry is a bug.
fn client(clients: &DArray<ClientConnection>, index: i32) -> &ClientConnection {
    clients.get_data(index).expect("UplinkAssert: client connection")
}

fn client_mut(clients: &mut DArray<ClientConnection>, index: i32) -> &mut ClientConnection {
    clients.get_data_mut(index).expect("UplinkAssert: client connection")
}

/// Sends `body` to the client at `index`. On failure the client is removed from `clients`
/// (C++ `clients.RemoveData(index)`). Takes the client table alone, so the caller can keep
/// the world borrowed while sending.
fn send_or_drop(clients: &mut DArray<ClientConnection>, index: i32, body: &str) -> bool {
    let sent = match clients.get_data_mut(index).and_then(|c| c.socket.as_mut()) {
        Some(socket) => tcp_send_message(socket, body),
        None => false,
    };
    if !sent {
        clients.remove_data(index);
    }
    sent
}

/// One connected client as the server sees it (lives in `NetworkServer::clients`).
#[derive(Debug, Default)]
pub struct ClientConnection {
    // Specific to CLIENT_COMMS
    connectionsize: i32,
    traceprogress: i32,
    numlinks: i32,
    sentgateway: bool,

    // Specific to CLIENT_STATUS
    lastnewsdate: Date,
    rating: Rating,
    balance: i32,
    numaccounts: i32,
    convictions: String,
    /// C++ `char hudupgrades`
    hudupgrades: i8,
    cpumodemmemory: i32,
    numhwitems: i32,
    ip: String,

    pub socket: Option<TcpStream>,
    pub clienttype: i32,
    /// Eclipse clock (ms) when the connection was accepted
    pub connectiontime: i32,
    /// Eclipse clock (ms) of the previous data burst to the client
    pub previousupdate: i32,
    /// Into `g.app.network.server.clients`
    pub index: i32,
}

impl ClientConnection {
    /// C++ constructor
    pub fn new() -> Self {
        ClientConnection {
            clienttype: CLIENT_NONE,
            convictions: " ".to_string(),
            ip: " ".to_string(),
            ..Default::default()
        }
    }

    /// `SetSocket`
    pub fn set_socket(&mut self, newsocket: TcpStream) {
        self.socket = Some(newsocket);
    }

    /// `SetClientType`
    pub fn set_client_type(&mut self, newclienttype: i32) {
        self.clienttype = newclienttype;
    }

    /// `SetConnectionTime`
    pub fn set_connection_time(&mut self, newtime: i32) {
        self.connectiontime = newtime;
    }

    /// `SetIndex`
    pub fn set_index(&mut self, newindex: i32) {
        self.index = newindex;
    }

    /// `TimeActive`: seconds since the connection was accepted. Reads the eclipse clock, which
    /// needs `&mut` (the clock is advanced on read), so it takes the context and the connection's
    /// key (`index` in `g.app.network.server.clients`).
    pub fn time_active(g: &mut Ctx, index: i32) -> i32 {
        let now = g.ecl.get_accurate_time();
        let active = now - client(&g.app.network.server.clients, index).connectiontime;
        active / 1000
    }

    /// `Handle_ClientCommsInterface`: send the comms data (trace progress, gateway, links,
    /// connection) to this client. Key: `index` in `g.app.network.server.clients`.
    pub fn handle_client_comms_interface(g: &mut Ctx, index: i32) {
        let world = g.game.world.as_deref().expect("UplinkAssert: world");
        let player = world.get_player();
        let conn = &player.connection;
        let clients = &mut g.app.network.server.clients;

        //
        // Update trace progress
        //

        if conn.traceprogress != client(clients, index).traceprogress {
            let body = format!("CLIENTCOMMS-TRACEPROGRESS {}", conn.traceprogress);
            if !send_or_drop(clients, index, &body) {
                return;
            }
            client_mut(clients, index).traceprogress = conn.traceprogress;
        }

        //
        // Update gateway
        //

        if !client(clients, index).sentgateway {
            let vl = world.get_vlocation(IP_LOCALHOST).expect("UplinkAssert: localhost");
            let body = format!("CLIENTCOMMS-IPNAME {} {} {} {}", vl.ip, vl.x, vl.y, vl.computer);
            if !send_or_drop(clients, index, &body) {
                return;
            }
            client_mut(clients, index).sentgateway = true;
        }

        //
        // Update known links
        //

        let links = &player.agent().links;
        let link_count = links.len() as i32;
        let numlinks = client(clients, index).numlinks;

        if link_count > numlinks {
            // The player has added some new links; update the client
            let numnewlinks = link_count - numlinks;

            for i in 0..numnewlinks {
                // As in the C++, this sends the first `numnewlinks` links (GetData(i)), not the links
                // added since the last update.
                let vl = world
                    .get_vlocation(&links[i as usize])
                    .expect("UplinkAssert: link location");
                let body = format!("CLIENTCOMMS-IPNAME {} {} {} {}", vl.ip, vl.x, vl.y, vl.computer);
                if !send_or_drop(clients, index, &body) {
                    return;
                }
            }

            client_mut(clients, index).numlinks = link_count;
        } else if link_count < numlinks {
            // The player has deleted some links; no need to update the client
            client_mut(clients, index).numlinks = link_count;
        }

        //
        // Update connection
        //

        let size = conn.get_size();
        if size != client(clients, index).connectionsize {
            let mut body = format!("CLIENTCOMMS {} ", size);
            for ip in &conn.vlocations {
                let vl = world.get_vlocation(ip).expect("UplinkAssert: connection location");
                body.push_str(&format!("{} {} {} ", vl.x, vl.y, vl.ip));
            }
            if !send_or_drop(clients, index, &body) {
                return;
            }
            client_mut(clients, index).connectionsize = size;
        }
    }

    /// `Handle_ClientStatusInterface`: send the status data (rating, finance, criminal record,
    /// HUD, hardware, location, news) to this client. Key: `index`.
    pub fn handle_client_status_interface(g: &mut Ctx, index: i32) {
        // Read through the whole context first: these take `&Ctx`, which cannot be held while the
        // client table is borrowed mutably below.
        let bandwidth = Gateway::get_bandwidth(g);
        let crimrec: Option<String> = RecordGenerator::get_criminal(g, "PLAYER").map(|rec| {
            rec.get_field("Convictions")
                .expect("UplinkAssert: convictions")
                .to_string()
        });

        let world = g.game.world.as_deref().expect("UplinkAssert: world");
        let player = world.get_player();
        let clients = &mut g.app.network.server.clients;

        //
        // Send latest rating
        //

        let rating_changed = {
            let cc = client(clients, index);
            player.rating.uplinkrating != cc.rating.uplinkrating
                || player.rating.neuromancerrating != cc.rating.neuromancerrating
        };
        if rating_changed {
            let body = format!(
                "CLIENTSTATUS-RATING Uplink rating : {}\nNeuromancer   : {}\nNotoriety     : None",
                player.rating.get_uplink_rating(),
                player.rating.get_neuromancer_rating()
            );
            if !send_or_drop(clients, index, &body) {
                return;
            }
            let cc = client_mut(clients, index);
            cc.rating.uplinkrating = player.rating.uplinkrating;
            cc.rating.neuromancerrating = player.rating.neuromancerrating;
        }

        //
        // Send latest balance
        //

        let balance = player.get_balance(world);
        let numaccounts = player.accounts.len() as i32;
        let finance_changed = {
            let cc = client(clients, index);
            balance != cc.balance || numaccounts != cc.numaccounts
        };
        if finance_changed {
            let mut body = format!("CLIENTSTATUS-FINANCE Balance : {}c\n\n", balance);
            for account in &player.accounts {
                // C++ sscanf("%s %s", ip, accno)
                let mut parts = account.split_whitespace();
                let ip = parts.next().unwrap_or("");
                let accno = parts.next().unwrap_or("");
                let ba = BankAccount::get_account(world, ip, accno).expect("UplinkAssert: bank account");
                body.push_str(&format!("Account: {} Balance: {}c\n", accno, ba.balance));
            }
            if !send_or_drop(clients, index, &body) {
                return;
            }
            let cc = client_mut(clients, index);
            cc.balance = balance;
            cc.numaccounts = numaccounts;
        }

        //
        // Send latest criminal record
        //

        if let Some(crimrec) = &crimrec {
            if client(clients, index).convictions != *crimrec {
                let body = format!("CLIENTSTATUS-CRIMINAL {}", crimrec);
                if !send_or_drop(clients, index, &body) {
                    return;
                }
                client_mut(clients, index).convictions = crimrec.clone();
            }
        }

        //
        // Send latest HUD upgrades
        //

        let hud = player.gateway.get_hud_upgrades() as i8;
        if hud != client(clients, index).hudupgrades {
            let mut body = String::from("CLIENTSTATUS-HUD ");
            if !player.gateway.has_any_hud_upgrade() {
                body.push_str("None\n");
            }
            if player.gateway.has_hud_upgrade(HUDUPGRADE_MAPSHOWSTRACE) {
                body.push_str("Show trace on map\n");
            }
            if player.gateway.has_hud_upgrade(HUDUPGRADE_CONNECTIONANALYSIS) {
                body.push_str("Connection analysis\n");
            }
            if player.gateway.has_hud_upgrade(HUDUPGRADE_IRCCLIENT) {
                body.push_str("IRC Client\n");
            }
            if player.gateway.has_hud_upgrade(HUDUPGRADE_LANVIEW) {
                body.push_str("LAN View\n");
            }
            if !send_or_drop(clients, index, &body) {
                return;
            }
            client_mut(clients, index).hudupgrades = hud;
        }

        //
        // Send latest CPU modem memory data
        //
        // No need to store cpu, modem and mem separately: multiply them together and compare.
        // This will work pretty much all of the time.

        let gateway = &player.gateway;
        let cpumodemmemory = gateway
            .get_cpu_speed()
            .wrapping_mul(bandwidth)
            .wrapping_mul(gateway.memorysize);
        let numhwitems = gateway.hardware.len() as i32;
        let hw_changed = {
            let cc = client(clients, index);
            cpumodemmemory != cc.cpumodemmemory || numhwitems != cc.numhwitems
        };
        if hw_changed {
            let mut body = format!(
                "CLIENTSTATUS-HW {}\n{}\nMemory : {}\n",
                gateway.get_cpu_speed(),
                gateway.modemtype,
                gateway.memorysize
            );
            for hw in &gateway.hardware {
                body.push_str(hw);
                body.push('\n');
            }
            if !send_or_drop(clients, index, &body) {
                return;
            }
            let cc = client_mut(clients, index);
            cc.cpumodemmemory = cpumodemmemory;
            cc.numhwitems = numhwitems;
        }

        //
        // Send latest location
        //

        let remote = player
            .get_remote_host(world)
            .expect("UplinkAssert: remote host");
        let remote_ip = remote.ip.clone();
        if remote_ip != client(clients, index).ip {
            let mut body = String::from("CLIENTSTATUS-IP ");
            if player.is_connected() {
                let computer = remote
                    .get_computer(world)
                    .expect("UplinkAssert: remote computer");
                body.push_str(&format!("Connected to {}\n({})\n", remote_ip, computer.name));
            } else {
                body.push_str("Not connected");
            }
            if !send_or_drop(clients, index, &body) {
                return;
            }
            client_mut(clients, index).ip = remote_ip;
        }

        //
        // Send latest news items
        //

        let uplink = world
            .get_company("Uplink")
            .and_then(|company| company.uplink.as_deref())
            .expect("UplinkAssert: uplink company");
        let latest = uplink.get_news(0).expect("UplinkAssert: latest news");
        let lastnewsdate = client(clients, index).lastnewsdate.clone();

        if latest.date.after(&lastnewsdate) {
            // Find the first "new" news story
            let news = &uplink.news;
            let mut indexofnew = news.len() as i32;
            for i in 1..news.len() {
                if !news[i].date.after(&lastnewsdate) {
                    indexofnew = i as i32 - 1;
                    break;
                }
            }

            // No need to send more than 7
            if indexofnew > 5 {
                indexofnew = 5;
            }

            // Send all new stories, oldest first. Note: C++ asserts here when indexofnew equals the
            // list size (every story is newer and there are at most 5), which is kept.
            let mut bodies = Vec::new();
            for in_ in (0..=indexofnew).rev() {
                let thisnews = uplink.get_news(in_).expect("UplinkAssert: news item");
                bodies.push(format!(
                    "CLIENTSTATUS-NEWS - {}: {}",
                    thisnews.date.get_short_string(),
                    thisnews.headline
                ));
            }

            let latest_date = latest.date.clone();
            for body in &bodies {
                if !send_or_drop(clients, index, body) {
                    return;
                }
            }

            client_mut(clients, index).lastnewsdate.set_date_from(&latest_date);
        }
    }


    /// `Update`: read the client's instruction (SETCLIENTTYPE) and send the periodic data.
    /// Key: `index`. May remove this connection from `g.app.network.server.clients`.
    pub fn update(g: &mut Ctx, index: i32) {
        // Read any input from the client (C++ does this only while the socket is open)
        let received = match g
            .app
            .network
            .server
            .clients
            .get_data_mut(index)
            .and_then(|c| c.socket.as_mut())
        {
            Some(socket) => Some(tcp_recv_until_hash(socket, CLIENT_RECV_BUFFER)),
            None => None,
        };
        let Some((buffer, status)) = received else {
            return;
        };

        match status {
            RecvStatus::SocketClosed => {
                g.app.network.server.clients.remove_data(index);
                return;
            }
            RecvStatus::Overflow => eprintln!("Overflow on incoming data"),
            RecvStatus::Error => eprintln!("Error in incoming data"),
            RecvStatus::Success => {}
        }

        // Examine and act on the information

        if !buffer.is_empty() {
            let mut parts = buffer.split_whitespace();
            let instruction = parts.next().unwrap_or("");
            let data1 = parts.next().map(c_int).unwrap_or(0);

            if instruction == "SETCLIENTTYPE" {
                client_mut(&mut g.app.network.server.clients, index).set_client_type(data1);
                println!("ClientConnection::Update, Successfully set CLIENTTYPE to {}", data1);
            } else {
                println!("ClientConnection::Update, incoming instruction not recognised");
                println!("Instruction: {}", instruction);
                println!("Data : {}", data1);
            }
        }

        // Send out the relevant data
        // (If a certain amount of time has passed and if a game is running)

        if g.game.game_speed() > GAMESPEED_PAUSED {
            let now = g.ecl.get_accurate_time();
            if now - client(&g.app.network.server.clients, index).previousupdate > 1000 {
                match client(&g.app.network.server.clients, index).clienttype {
                    CLIENT_COMMS => Self::handle_client_comms_interface(g, index),
                    CLIENT_STATUS => Self::handle_client_status_interface(g, index),
                    _ => eprintln!("Unrecognised Client Type"),
                }

                // A handler may have removed this client; its slot is then empty and skipped.
                let now = g.ecl.get_accurate_time();
                if let Some(cc) = g.app.network.server.clients.get_data_mut(index) {
                    cc.previousupdate = now;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread::sleep;
    use std::time::Duration;

    /// A connected loopback pair: (writer end, reader end).
    fn pair() -> (TcpStream, TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let a = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (b, _) = listener.accept().unwrap();
        (a, b)
    }

    /// Reads until a full message arrives (the data is on the loopback within milliseconds).
    fn recv(socket: &mut TcpStream, capacity: usize) -> (String, RecvStatus) {
        for _ in 0..200 {
            let (message, status) = tcp_recv_until_hash(socket, capacity);
            if !message.is_empty() || status != RecvStatus::Success {
                return (message, status);
            }
            sleep(Duration::from_millis(5));
        }
        (String::new(), RecvStatus::Success)
    }

    #[test]
    fn server_message_is_body_newline_cr_hash_nul() {
        // C++: msgstream << body << (char) 10 << (char) 13 << '#' << '\x0', sent with strlen+1 bytes
        let (mut a, mut b) = pair();
        assert!(tcp_send_message(&mut a, "CLIENTCOMMS-TRACEPROGRESS 2"));
        let mut got = [0u8; 31];
        b.read_exact(&mut got).unwrap();
        assert_eq!(&got[..], &b"CLIENTCOMMS-TRACEPROGRESS 2\n\r#\0"[..]);
    }

    #[test]
    fn recv_returns_message_with_hash_and_one_message_at_a_time() {
        let (mut a, mut b) = pair();
        tcp_send_message(&mut a, "ONE");
        tcp_send_message(&mut a, "TWO");
        assert_eq!(recv(&mut b, 512), ("ONE\n\r#".to_string(), RecvStatus::Success));
        assert_eq!(recv(&mut b, 512), ("TWO\n\r#".to_string(), RecvStatus::Success));
    }

    #[test]
    fn recv_waits_for_the_terminator_and_consumes_nothing_before_it() {
        let (mut a, mut b) = pair();
        a.write_all(b"SETCLIENTTYPE 1").unwrap();
        sleep(Duration::from_millis(50));
        assert_eq!(tcp_recv_until_hash(&mut b, 128), (String::new(), RecvStatus::Success));
        a.write_all(b"#\0").unwrap();
        assert_eq!(recv(&mut b, 128), ("SETCLIENTTYPE 1#".to_string(), RecvStatus::Success));
    }

    #[test]
    fn recv_reports_overflow_when_the_buffer_fills_without_a_terminator() {
        let (mut a, mut b) = pair();
        a.write_all(&[b'x'; 200]).unwrap();
        assert_eq!(recv(&mut b, 128).1, RecvStatus::Overflow);
    }

    #[test]
    fn recv_reports_a_closed_socket() {
        let (a, mut b) = pair();
        drop(a);
        assert_eq!(recv(&mut b, 128).1, RecvStatus::SocketClosed);
    }

    #[test]
    fn instruction_parse_like_sscanf() {
        assert_eq!(c_int("2"), 2);
        assert_eq!(c_int("-1x"), -1);
        assert_eq!(c_int("x"), 0);
    }
}
