//! Port of source/network/networkclient.h + source/network/networkclient.cpp
//!
//! C++ `NetworkScreen *screen` is a `ScreenSlot` (PORTING.md §3.7): the current network screen is
//! taken out of the slot while its methods run.

use std::net::{Shutdown, TcpStream};

use crate::ctx::Ctx;
use crate::interface::interfacescreen::{InterfaceScreen, ScreenSlot};
use crate::mainmenu::mainmenu::{MainMenu, MAINMENU_NETWORKOPTIONS};
use crate::network::clientconnection::{tcp_recv_until_hash, tcp_send_bytes, RecvStatus};
use crate::network::interfaces::clientcommsinterface::{ClientCommsInterface, ClientCommsStatics};
use crate::network::interfaces::clientstatusinterface::ClientStatusInterface;
use crate::network::network::NETWORK_NONE;
use crate::world::vlocation::VLocation;

// Types of client
pub const CLIENT_NONE: i32 = 0;
pub const CLIENT_COMMS: i32 = 1;
pub const CLIENT_STATUS: i32 = 2;

/// C++ `unsigned short portnum = 31337` in `StartClient`
const CLIENT_PORT: u16 = 31337;

/// C++ `char compname [SIZE_COMPUTER_NAME]` (SIZE_COMPUTER_NAME = 64, see computer.h). No Rust
/// constant exists yet, so it is defined here.
const SIZE_COMPUTER_NAME: usize = 64;

/// Reads a message the way the C++ `istrstream` does: whitespace-separated words, and `getline`.
struct MsgStream<'a> {
    rest: &'a str,
}

impl<'a> MsgStream<'a> {
    fn new(buffer: &'a str) -> Self {
        MsgStream { rest: buffer }
    }

    /// C++ `msgstream >> word`: the next whitespace-delimited word ("" at the end).
    fn word(&mut self) -> &'a str {
        let s = self.rest.trim_start();
        let end = s.find(char::is_whitespace).unwrap_or(s.len());
        self.rest = &s[end..];
        &s[..end]
    }

    /// C++ `msgstream >> int`: a value that does not parse gives 0.
    fn int(&mut self) -> i32 {
        self.word().parse().unwrap_or(0)
    }

    /// C++ `msgstream >> ws` followed by `getline`: the rest of the line, without the newline.
    fn line(&mut self) -> &'a str {
        let s = self.rest.trim_start();
        match s.find('\n') {
            Some(end) => {
                self.rest = &s[end + 1..];
                &s[..end]
            }
            None => {
                self.rest = "";
                s
            }
        }
    }
}

/// Runs on a client computer: handles incoming data from the server and owns the current screen.
#[derive(Default)]
pub struct NetworkClient {
    /// C++ `SOCKET socket` (None when not connected)
    socket: Option<TcpStream>,
    clienttype: i32,
    currentscreencode: i32,
    /// C++ `NetworkScreen *screen` (the screen is `None` for CLIENT_NONE)
    pub screen: ScreenSlot,
    /// The C++ statics of `ClientCommsInterface`, parked here between two comms screens
    pub(crate) comms_statics: Option<ClientCommsStatics>,
}

impl NetworkClient {
    /// C++ constructor
    pub fn new() -> Self {
        Self::default()
    }

    /// C++ `NetworkClient::Print`. PORT: the socket is printed with `Debug` (no SOCKET number).
    pub fn print(&self) {
        println!("NetworkClient : SOCKET:{:?}", self.socket);
        println!("\tcurrentscreen:{}", self.currentscreencode);
    }

    /// `StartClient(ip)`: connect to the server on port 31337. Returns success.
    pub fn start_client(&mut self, ip: &str) -> bool {
        match TcpStream::connect((ip, CLIENT_PORT)) {
            Ok(socket) => {
                self.socket = Some(socket);
                true
            }
            Err(_) => false,
        }
    }

    /// `StopClient`: close the connection. Returns success.
    pub fn stop_client(&mut self) -> bool {
        match self.socket.take() {
            Some(socket) => socket.shutdown(Shutdown::Both).is_ok(),
            None => false,
        }
    }

    /// `SetClientType`: asks the server for this client type and, on success, runs its screen.
    /// Uses the context to switch screens, so it is an associated fn.
    pub fn set_client_type(g: &mut Ctx, newtype: i32) {
        // Request this mode from the server. The C++ sends its 32-byte buffer; this sends the
        // message with its terminating NUL, which is what the server reads up to.
        let message = format!("SETCLIENTTYPE {}#\0", newtype);
        let sent = match g.app.network.client.socket.as_mut() {
            Some(socket) => tcp_send_bytes(socket, message.as_bytes()),
            None => false,
        };

        if sent {
            g.app.network.client.clienttype = newtype;
            Self::run_screen(g, newtype);
        } else {
            println!("NetworkClient::SetClientType, failed to convince server to co-operate");
        }
    }

    /// `InScreen`: id code of the current screen
    pub fn in_screen(&self) -> i32 {
        self.currentscreencode
    }

    /// `RunScreen(SCREENCODE)`: removes the current screen and creates the one for `screencode`
    /// (CLIENT_NONE leaves no screen). Associated fn because it creates and removes screens.
    pub fn run_screen(g: &mut Ctx, screencode: i32) {
        // The code is set before the switch, as in C++. A switch requested from inside the current
        // screen is deferred by ScreenSlot::replace.
        g.app.network.client.currentscreencode = screencode;

        let new: Option<Box<dyn InterfaceScreen>> = match screencode {
            CLIENT_COMMS => Some(Box::new(ClientCommsInterface::new())),
            CLIENT_STATUS => Some(Box::new(ClientStatusInterface::new())),
            CLIENT_NONE => None,
            _ => panic!("Tried to create a screen with unknown SCREENCODE"),
        };
        ScreenSlot::replace(g, Self::screen_slot, new);
    }

    /// `GetNetworkScreen`. The C++ asserted a screen existed and returned it as `NetworkScreen *`;
    /// callers downcast with `.get::<T>()` on the returned slot.
    pub fn get_network_screen(&self) -> &ScreenSlot {
        debug_assert!(self.screen.is_some());
        &self.screen
    }

    /// The client's screen slot as seen from the context (for `ScreenSlot::call` / `replace`).
    pub fn screen_slot(g: &mut Ctx) -> &mut ScreenSlot {
        &mut g.app.network.client.screen
    }

    /// `Handle_ClientCommsData`: a CLIENTCOMMS / CLIENTCOMMS-TRACEPROGRESS / CLIENTCOMMS-IPNAME
    /// message updates the comms screen in the slot.
    pub fn handle_client_comms_data(g: &mut Ctx, buffer: &str) {
        let mut msg = MsgStream::new(buffer);
        let msgtype = msg.word();

        if msgtype == "CLIENTCOMMS" {
            // This is a list of IP's making up the players connection
            let cci = g
                .app
                .network
                .client
                .screen
                .get_mut::<ClientCommsInterface>()
                .expect("UplinkAssert: comms screen");

            cci.connection.clear();

            let numrelays = msg.int();
            for _ in 0..numrelays {
                // The C++ reads x and y here too, but the code that used them is commented out
                // there, so they are only consumed.
                let _ = (msg.int(), msg.int());
                let ip = msg.word();

                // We must already know this IP (C++ UplinkAssert on the location lookup)
                debug_assert!(cci.locations.contains_key(ip), "UplinkAssert: location lookup");

                cci.connection.push(ip.to_string());
            }
        } else if msgtype == "CLIENTCOMMS-TRACEPROGRESS" {
            // This is an update on the trace progress
            let traceprogress = msg.int();
            let cci = g
                .app
                .network
                .client
                .screen
                .get_mut::<ClientCommsInterface>()
                .expect("UplinkAssert: comms screen");

            cci.traceprogress = traceprogress;
            println!("Traceprogress is now {}", traceprogress);
        } else if msgtype == "CLIENTCOMMS-IPNAME" {
            // This is the name of a computer at one of the IP's in your connection
            let ip = msg.word();
            let x = msg.int();
            let y = msg.int();
            let compname: String = msg.line().chars().take(SIZE_COMPUTER_NAME - 1).collect();
            println!("'{}'", compname);

            let cci = g
                .app
                .network
                .client
                .screen
                .get_mut::<ClientCommsInterface>()
                .expect("UplinkAssert: comms screen");

            if let Some(vl) = cci.locations.get_mut(ip) {
                vl.set_computer(&compname);
                vl.set_plocation(x, y);
            } else {
                let mut vl = VLocation::new();
                vl.set_plocation(x, y);
                vl.set_ip(ip);
                vl.set_computer(&compname);
                cci.put_location(ip, vl);
            }
            cci.layout_labels();
        } else {
            eprintln!("NetworkClient::Handle_ClientCommsData, received data but did not recognise it");
        }
    }

    /// `Handle_ClientStatusData`: a CLIENTSTATUS-* message updates the status screen in the slot.
    pub fn handle_client_status_data(g: &mut Ctx, buffer: &str) {
        let msgtype = MsgStream::new(buffer).word();

        // C++: data = buffer + strlen(msgtype) + 1, then the last two bytes ("\r#") are chopped.
        let after_type = buffer.get(msgtype.len() + 1..).unwrap_or("");
        let data = after_type
            .get(..after_type.len().saturating_sub(2))
            .unwrap_or("");

        match msgtype {
            "CLIENTSTATUS-NEWS" => ClientStatusInterface::add_news_story(g, data),
            "CLIENTSTATUS-RATING" => ClientStatusInterface::set_rating(g, data),
            "CLIENTSTATUS-FINANCE" => ClientStatusInterface::set_financial(g, data),
            "CLIENTSTATUS-CRIMINAL" => ClientStatusInterface::set_criminal(g, data),
            "CLIENTSTATUS-HUD" => ClientStatusInterface::set_hud_upgrades(g, data),
            "CLIENTSTATUS-HW" => ClientStatusInterface::set_hardware(g, data),
            "CLIENTSTATUS-IP" => ClientStatusInterface::set_connection(g, data),
            _ => eprintln!("NetworkClient::Handle_ClientStatusData, received data but did not recognise it"),
        }
    }

    /// `Update`: read one message from the server, dispatch it, then update the screen.
    pub fn update(g: &mut Ctx) {
        // Check for input from server (C++ buffer [512])
        let received = g
            .app
            .network
            .client
            .socket
            .as_mut()
            .map(|socket| tcp_recv_until_hash(socket, 512));

        if let Some((buffer, status)) = received {
            match status {
                RecvStatus::SocketClosed => {
                    let width = g.app.options.get_option_value("graphics_screenwidth");
                    let height = g.app.options.get_option_value("graphics_screenheight");
                    g.ecl.reset(width, height);
                    g.app.network.client.socket = None;
                    g.app.network.set_status(NETWORK_NONE);
                    MainMenu::run_screen(g, MAINMENU_NETWORKOPTIONS);
                    return;
                }
                RecvStatus::Overflow => panic!("buffer overflow"),
                RecvStatus::Error => panic!("Tcp4u Error occured"),
                RecvStatus::Success => {}
            }

            if !buffer.is_empty() {
                // Deal with the input
                match g.app.network.client.clienttype {
                    CLIENT_COMMS => Self::handle_client_comms_data(g, &buffer),
                    CLIENT_STATUS => Self::handle_client_status_data(g, &buffer),
                    CLIENT_NONE => println!("Client type not set"),
                    _ => eprintln!("Unrecognised client type"),
                }
            }
        }

        // Update interface
        ScreenSlot::call(g, Self::screen_slot, |s, g| s.update(g));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn msgstream_reads_like_istrstream() {
        // "CLIENTCOMMS-IPNAME ip x y compname\n\r#": words, ints, then getline for the computer name
        let mut msg = MsgStream::new("CLIENTCOMMS-IPNAME 128.185.0.8 182 69 Uplink Internal Services System\n\r#");
        assert_eq!(msg.word(), "CLIENTCOMMS-IPNAME");
        assert_eq!(msg.word(), "128.185.0.8");
        assert_eq!(msg.int(), 182);
        assert_eq!(msg.int(), 69);
        assert_eq!(msg.line(), "Uplink Internal Services System");

        // "CLIENTCOMMS n x y ip x y ip ...": the connection list
        let mut msg = MsgStream::new("CLIENTCOMMS 2 282 69 127.0.0.1 182 69 128.185.0.8 \n\r#");
        assert_eq!(msg.word(), "CLIENTCOMMS");
        assert_eq!(msg.int(), 2);
        assert_eq!((msg.int(), msg.int(), msg.word()), (282, 69, "127.0.0.1"));
        assert_eq!((msg.int(), msg.int(), msg.word()), (182, 69, "128.185.0.8"));
    }
}
