//! Port of source/interface/localinterface/irc_interface.h + source/interface/localinterface/irc_interface.cpp
//! A cut-down IRC client so players can talk to each other while playing.
//! The external irc library the C++ links (`irc.h`, contrib/irclib) is ported at the top of this file.

use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use uplink_engine::gl;
use uplink_engine::image::Image;
use uplink_engine::text;

use crate::app::miscutils::lower_case_string;
use crate::app::opengl_interface::{
    border_draw, button_click, button_draw, button_highlight, clear_draw, textbutton_draw, wordwraptext,
};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::impl_as_any;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::localinterface::localinterface::{PANELSIZE, SCREEN_IRC};
use crate::interface::localinterface::localinterfacescreen::background_draw;
use crate::interface::scrollbox::ScrollBox;

/// C++ `#define IRCBUFFERSIZE 200`: maximum number of lines kept in the chat buffer.
pub const IRC_BUFFER_SIZE: usize = 200;

// C++ `#define COLOUR_*` (r, g, b) triples used by AddText.
pub const COLOUR_DEFAULT: [f32; 3] = [0.6, 0.6, 0.6];
pub const COLOUR_USER: [f32; 3] = [0.7, 0.8, 1.0];
pub const COLOUR_TEXT: [f32; 3] = [1.0, 1.0, 1.0];
pub const COLOUR_ACTION: [f32; 3] = [1.0, 0.7, 0.8];
pub const COLOUR_JOINPART: [f32; 3] = [0.6, 1.0, 0.6];
pub const COLOUR_TOPIC: [f32; 3] = [1.0, 0.8, 0.7];
pub const COLOUR_MOTD: [f32; 3] = [0.7, 0.8, 1.0];
pub const COLOUR_MODE: [f32; 3] = [1.0, 0.8, 0.8];

// ============================================================================
// The irc library
//
// PORT: the C++ links the external irc library from the Developer CD (contrib/irclib: irc.h,
// irc.cpp, socket.cpp; an RFC 1459 client). It is ported here with std only, keeping the parts
// this screen uses: CIrcMessage (parser + AsString), CIrcSessionInfo and CIrcSession. Deviations:
// - CIrcSession::Connect resolved and connected on the calling (game) thread. Here a background
//   thread owns the TcpStream and does all of the network work, so the game never blocks; Connect
//   returns at once and a failed connection simply closes the session again (the C++ caller
//   ignores the result, so nothing is shown either way).
// - The library called the monitors straight from its receive thread. Here received messages go
//   through a channel that IRCInterface::Update drains (the C++ Linux build's
//   `CCrossThreadsMessagingDevice::ProcessMessages` call site).
// - PING is answered on the network thread, so the connection stays up while the IRC screen is
//   closed. The library's reply went through AsString, which drops parameter 0 and so sent a bare
//   "PONG"; the port sends "PONG :<token>". The monitor still sees both messages, as in C++.
// - The ident server (RFC 1413 on port 113) is not started: binding port 113 needs root on Linux,
//   where the C++ CIrcIdentServer::Start fails and the session carries on without it.

/// C++ `WinsockInit`: Windows socket start-up. Nothing to do on Linux.
pub struct WinsockInit;

/// C++ `struct CIrcSessionInfo`: the connection parameters ConnectClick fills in.
#[derive(Debug, Clone, Default)]
pub struct CIrcSessionInfo {
    pub s_server: String,
    pub s_server_name: String,
    pub i_port: i32,
    pub s_nick: String,
    pub s_user_id: String,
    pub s_full_name: String,
    pub s_password: String,
    pub b_ident_server: bool,
    pub s_ident_server_type: String,
    pub i_ident_server_port: i32,
}

impl CIrcSessionInfo {
    /// C++ `CIrcSessionInfo::Reset`
    pub fn reset(&mut self) {
        *self = CIrcSessionInfo::default();
    }
}

/// C++ `CIrcMessage::Prefix`: who sent a message (nick!user@host).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CIrcPrefix {
    pub s_nick: String,
    pub s_user: String,
    pub s_host: String,
}

/// C++ `class CIrcMessage`: one IRC message, received or sent.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CIrcMessage {
    /// C++ `prefix`
    pub prefix: CIrcPrefix,
    /// C++ `sCommand` ("001", "PRIVMSG", ...), always upper case.
    pub s_command: String,
    /// C++ `parameters`
    pub parameters: Vec<String>,
    /// C++ `m_bIncoming`: false for a message this client sent itself.
    pub m_b_incoming: bool,
}

/// C `isspace` on a byte.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

impl CIrcMessage {
    /// C++ `CIrcMessage(const char *lpszCmdLine, bool bIncoming)`: the parser constructor.
    pub fn new(cmd_line: &str, incoming: bool) -> Self {
        let mut msg = CIrcMessage {
            m_b_incoming: incoming,
            ..CIrcMessage::default()
        };
        msg.parse_irc_command(cmd_line);
        msg
    }

    /// C++ `CIrcMessage::ParseIrcCommand`. Every split happens at an ASCII byte, so the slices
    /// stay valid UTF-8.
    fn parse_irc_command(&mut self, line: &str) {
        let bytes = line.as_bytes();
        let at = |i: usize| bytes.get(i).copied().unwrap_or(0);
        let mut p1 = 0;
        let mut p2;

        // prefix exists ?
        if at(p1) == b':' {
            // break prefix into its components (nick!user@host)
            p1 += 1;
            p2 = p1;
            while at(p2) != 0 && at(p2) != b' ' && at(p2) != b'!' {
                p2 += 1;
            }
            self.prefix.s_nick = line[p1..p2].to_string();
            if at(p2) == b'!' {
                p2 += 1;
                p1 = p2;
                while at(p2) != 0 && at(p2) != b' ' && at(p2) != b'@' {
                    p2 += 1;
                }
                self.prefix.s_user = line[p1..p2].to_string();
                if at(p2) == b'@' {
                    p2 += 1;
                    p1 = p2;
                    while at(p2) != 0 && !is_c_space(at(p2)) {
                        p2 += 1;
                    }
                    self.prefix.s_host = line[p1..p2].to_string();
                }
            }
            // end_of_prefix
            while at(p2) != 0 && is_c_space(at(p2)) {
                p2 += 1;
            }
            p1 = p2;
        }

        // get command
        p2 = p1;
        while at(p2) != 0 && !is_c_space(at(p2)) {
            p2 += 1;
        }
        self.s_command = line[p1..p2].to_ascii_uppercase();
        while at(p2) != 0 && is_c_space(at(p2)) {
            p2 += 1;
        }
        p1 = p2;

        // get parameters
        while at(p1) != 0 {
            if at(p1) == b':' {
                p1 += 1;
                // seek end-of-message
                while at(p2) != 0 {
                    p2 += 1;
                }
                self.parameters.push(line[p1..p2].to_string());
                break;
            } else {
                // seek end of parameter
                while at(p2) != 0 && !is_c_space(at(p2)) {
                    p2 += 1;
                }
                self.parameters.push(line[p1..p2].to_string());
                // see next parameter
                while at(p2) != 0 && is_c_space(at(p2)) {
                    p2 += 1;
                }
                p1 = p2;
            }
        }
    }

    /// C++ `CIrcMessage::AsString`: the message as a wire line, ending in "\r\n".
    /// Parameter 0 is never written: the game's lines name the sender first
    /// ("privmsg <nick>: <channel> :<text>") and only the rest goes to the server.
    pub fn as_string(&self) -> String {
        let mut s = String::new();

        if !self.prefix.s_nick.is_empty() {
            s.push(':');
            s.push_str(&self.prefix.s_nick);
            if !self.prefix.s_user.is_empty() && !self.prefix.s_host.is_empty() {
                s.push('!');
                s.push_str(&self.prefix.s_user);
                s.push('@');
                s.push_str(&self.prefix.s_host);
            }
            s.push(' ');
        }

        s.push_str(&self.s_command);

        for (i, param) in self.parameters.iter().enumerate().skip(1) {
            s.push(' ');
            if i == self.parameters.len() - 1 {
                // is last parameter ?
                s.push(':');
            }
            s.push_str(param);
        }

        s.push_str("\r\n");
        s
    }
}

/// What the game thread asks the network thread to do.
#[derive(Debug)]
enum IrcNetCommand {
    /// Write this line (already ending in "\r\n").
    Send(String),
    /// C++ `CIrcSession::Disconnect`: send QUIT with this message, then close.
    Quit(String),
}

/// C++ `class CIrcSession`: the connection to the server.
// PORT: the socket lives on a background thread (see the note at the top of the library).
#[derive(Debug, Default)]
pub struct CIrcSession {
    /// C++ `m_info`
    pub m_info: CIrcSessionInfo,
    /// Lines for the network thread.
    to_net: Option<mpsc::Sender<IrcNetCommand>>,
    /// Messages from the network thread, in arrival order.
    from_net: Option<mpsc::Receiver<CIrcMessage>>,
    /// C++ `operator bool` (`m_socket` is open): true from Connect until the connection fails or closes.
    socket_open: Arc<AtomicBool>,
}

impl CIrcSession {
    pub fn new() -> Self {
        CIrcSession::default()
    }

    /// C++ `CIrcSession::Connect`. Starts the network thread, which connects, sends NICK/USER
    /// and then relays lines both ways. Returns false only if the thread could not be started.
    pub fn connect(&mut self, info: &CIrcSessionInfo) -> bool {
        if self.to_net.is_some() {
            return false;
        }

        let (to_net, net_rx) = mpsc::channel();
        let (net_tx, from_net) = mpsc::channel();
        let socket_open = Arc::new(AtomicBool::new(true));

        self.m_info = info.clone();
        let thread_info = info.clone();
        let thread_open = Arc::clone(&socket_open);
        let started = std::thread::Builder::new()
            .name("uplink-irc".to_string())
            .spawn(move || irc_session_thread(thread_info, net_rx, net_tx, thread_open))
            .is_ok();
        if !started {
            return false;
        }

        self.to_net = Some(to_net);
        self.from_net = Some(from_net);
        self.socket_open = socket_open;
        true
    }

    /// C++ `CIrcSession::Disconnect`: sends QUIT; the network thread then waits up to 5 s for
    /// the server to close the connection before closing it itself. Never blocks the caller.
    pub fn disconnect(&mut self, message: &str) {
        if let Some(to_net) = self.to_net.take() {
            let _ = to_net.send(IrcNetCommand::Quit(message.to_string()));
        }
        self.socket_open.store(false, Ordering::SeqCst);
    }

    /// C++ `operator bool`: is the connection (still) open?
    pub fn is_open(&self) -> bool {
        self.to_net.is_some() && self.socket_open.load(Ordering::SeqCst)
    }

    /// The socket half of C++ `operator<<(CIrcSession&, const CIrcMessage&)`: writes
    /// `msg.AsString()` when the connection is open. Returns whether it was sent (the caller then
    /// notifies the monitor, as the C++ operator does).
    pub fn send(&mut self, msg: &CIrcMessage) -> bool {
        if !self.is_open() {
            return false;
        }
        match &self.to_net {
            Some(to_net) => to_net.send(IrcNetCommand::Send(msg.as_string())).is_ok(),
            None => false,
        }
    }

    /// Messages received since the last call, in arrival order (what the C++ receive thread
    /// handed to `Notify`).
    pub fn take_incoming(&mut self) -> Vec<CIrcMessage> {
        match &self.from_net {
            Some(from_net) => from_net.try_iter().collect(),
            None => Vec::new(),
        }
    }
}

/// C++ `GetComputerName` (gethostname on Linux), for the USER line.
fn local_host_name() -> String {
    std::fs::read_to_string("/proc/sys/kernel/hostname")
        .map(|s| s.trim().to_string())
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "localhost".to_string())
}

/// C++ `InetAddr addr(server, port); m_socket.Connect(addr)`.
fn irc_connect(server: &str, port: i32) -> Option<TcpStream> {
    let port = u16::try_from(port).ok()?;
    let addrs = (server, port).to_socket_addrs().ok()?;
    addrs
        .into_iter()
        .find_map(|addr| TcpStream::connect_timeout(&addr, Duration::from_secs(20)).ok())
}

/// Splits complete lines off the front of `inbuf` (C++ `CIrcSession::DoReceive`: a line ends at
/// '\r' or '\n'; empty lines are skipped; an unfinished line stays in the buffer).
fn irc_take_lines(inbuf: &mut Vec<u8>) -> Vec<String> {
    let mut lines = Vec::new();
    let mut start = 0;
    while let Some(len) = inbuf[start..].iter().position(|&b| b == b'\r' || b == b'\n') {
        let end = start + len;
        if end > start {
            lines.push(String::from_utf8_lossy(&inbuf[start..end]).into_owned());
        }
        start = end + 1;
    }
    inbuf.drain(..start);
    lines
}

/// The network thread: C++ `CIrcSession::Connect` (from the socket connect on) plus
/// `CIrcSession::DoReceive`, `Disconnect`'s QUIT and `CIrcDefaultMonitor::OnIrc_PING`.
fn irc_session_thread(
    info: CIrcSessionInfo,
    from_game: mpsc::Receiver<IrcNetCommand>,
    to_game: mpsc::Sender<CIrcMessage>,
    socket_open: Arc<AtomicBool>,
) {
    let Some(mut stream) = irc_connect(&info.s_server, info.i_port) else {
        eprintln!("IRC: failed to connect to {}:{}", info.s_server, info.i_port);
        socket_open.store(false, Ordering::SeqCst);
        return;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(50)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(10)));

    // Sent straight on the socket in C++ (not through operator<<, so no monitor sees them)
    let mut login = String::new();
    if !info.s_password.is_empty() {
        login.push_str(&format!("PASS {}\r\n", info.s_password));
    }
    login.push_str(&format!("NICK {}\r\n", info.s_nick));
    login.push_str(&format!(
        "USER {} {} {} :{}\r\n",
        info.s_user_id,
        local_host_name(),
        "server",
        info.s_full_name
    ));
    let mut ok = stream.write_all(login.as_bytes()).is_ok();

    // C++ receives into a 4 KB buffer; a longer unfinished line ends the session there too.
    const IRC_INBUF_SIZE: usize = 1024 * 4;
    let mut inbuf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 1024 * 4];
    let mut quit_deadline: Option<Instant> = None;

    while ok {
        // Outgoing lines
        loop {
            match from_game.try_recv() {
                Ok(IrcNetCommand::Send(line)) => {
                    if stream.write_all(line.as_bytes()).is_err() {
                        ok = false;
                        break;
                    }
                }
                Ok(IrcNetCommand::Quit(message)) => {
                    let _ = stream.write_all(format!("QUIT :{}\r\n", message).as_bytes());
                    quit_deadline = Some(Instant::now() + Duration::from_secs(5));
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    // The session is gone without a QUIT (or the QUIT wait is running)
                    if quit_deadline.is_none() {
                        ok = false;
                    }
                    break;
                }
            }
        }
        if !ok || quit_deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            break;
        }

        // Incoming lines
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                inbuf.extend_from_slice(&chunk[..n]);
                for line in irc_take_lines(&mut inbuf) {
                    let msg = CIrcMessage::new(&line, true);
                    if msg.s_command == "PING" {
                        // C++ CIrcDefaultMonitor::OnIrc_PING: `m_session << "PONG <token>"`, whose
                        // notification reaches the monitor before the PING itself.
                        let token = msg.parameters.first().cloned().unwrap_or_default();
                        if stream.write_all(format!("PONG :{}\r\n", token).as_bytes()).is_err() {
                            ok = false;
                        }
                        let _ = to_game.send(CIrcMessage::new(&format!("PONG {}", token), false));
                    }
                    let _ = to_game.send(msg);
                }
                if inbuf.len() >= IRC_INBUF_SIZE {
                    break;
                }
            }
            Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted) => {}
            Err(_) => break,
        }
    }

    // C++ then calls Notify(NULL); UplinkIRCMonitor does nothing on a disconnection.
    socket_open.store(false, Ordering::SeqCst);
    let _ = stream.shutdown(Shutdown::Both);
}

/// C++ `class UplinkIRCMessage`: one line of the chat buffer.
#[derive(Debug, Clone, Default)]
pub struct UplinkIRCMessage {
    /// C++ `char *user` (None when the line has no speaker).
    pub user: Option<String>,
    /// C++ `char *text`.
    pub text: String,
    pub red: f32,
    pub green: f32,
    pub blue: f32,
}

impl UplinkIRCMessage {
    /// C++ `UplinkIRCMessage::Set`
    pub fn set(&mut self, newuser: Option<&str>, newtext: &str, colour: [f32; 3]) {
        self.user = newuser.map(str::to_string);
        self.text = newtext.to_string();
        self.red = colour[0];
        self.green = colour[1];
        self.blue = colour[2];
    }
}

/// C++ `class UplinkIRCUser`: one entry in the channel user list.
#[derive(Debug, Clone, Default)]
pub struct UplinkIRCUser {
    /// C++ `char *name`.
    pub name: String,
    /// C++ `int status` (operator/voice flags from MODE).
    pub status: i32,
}

impl UplinkIRCUser {
    /// C++ `UplinkIRCUser::Set`
    pub fn set(&mut self, newname: &str) {
        self.name = newname.to_string();
    }
}

/// C++ `class UplinkIRCMonitor : CIrcDefaultMonitor`: receives IRC server messages and routes them
/// to the handlers below. The DEFINE_IRC_MAP table becomes [`UplinkIRCMonitor::dispatch`].
// PORT: the C++ base keeps a reference to the session. Here the session is owned by IRCInterface
// (`c_irc_session`) and messages reach the monitor through `irc_notify`, so the monitor holds no
// session of its own. It has no state either: `li.irc.uplink_irc_monitor` only records that the
// monitor is added to the session (C++ AddMonitor / RemoveMonitor), and `irc_notify` dispatches
// through a fresh value, so a handler that sends a line (whose notification nests, as in C++)
// can borrow `g`.
#[derive(Debug, Default)]
pub struct UplinkIRCMonitor;

impl UplinkIRCMonitor {
    /// C++ `UplinkIRCMonitor::UplinkIRCMonitor(CIrcSession &session)`. The IRC_MAP_ENTRY table is
    /// the match in [`UplinkIRCMonitor::dispatch`].
    pub fn new() -> Self {
        UplinkIRCMonitor
    }

    /// C++ DEFINE_IRC_MAP / IRC_MAP_ENTRY dispatch: routes `command` ("001", "PRIVMSG", ...) to its handler.
    /// Returns false when no handler is mapped to the command, or the handler wants the default
    /// handling (C++ `CIrcMonitor::OnCrossThreadsMessage` then calls OnIrcDefault).
    // The base map (CIrcDefaultMonitor) adds NICK, 002 (both shadowed here), PING and 005, whose
    // handlers all return false. PING's PONG is sent by the network thread, so those two simply
    // fall through to the default handling.
    pub fn dispatch(&mut self, g: &mut Ctx, command: &str, msg: &CIrcMessage) -> bool {
        match command {
            "001" => self.received_rpl_welcome(g, msg),
            "002" | "003" | "250" | "251" | "252" | "253" | "254" | "255" | "265" | "266" | "NOTICE" => {
                self.received_rpl_luser(g, msg)
            }
            "331" | "332" => self.received_rpl_topic(g, msg),
            "353" => self.received_rpl_namreply(g, msg),
            "366" => self.received_rpl_endofnames(g, msg),
            "372" | "375" | "376" => self.received_motd(g, msg),
            "PRIVMSG" => self.received_privmsg(g, msg),
            "JOIN" => self.received_join(g, msg),
            "PART" => self.received_part(g, msg),
            "TOPIC" => self.received_topic(g, msg),
            "MODE" => self.received_mode(g, msg),
            "NICK" => self.received_nick(g, msg),
            "QUIT" => self.received_quit(g, msg),
            _ => false,
        }
    }

    /// C++ `OnIrcDefault`: prints unhandled commands to the chat buffer.
    pub fn on_irc_default(&mut self, g: &mut Ctx, msg: &CIrcMessage) {
        let command = format!("{} : {}", msg.s_command, msg.as_string());
        IRCInterface::add_text(g, None, &command, [1.0, 1.0, 1.0]);

        for (i, param) in msg.parameters.iter().enumerate() {
            let this_param = format!("Param{}: {}", i, param);
            IRCInterface::add_text(g, None, &this_param, COLOUR_DEFAULT);
        }
    }

    /// C++ `Received_PRIVMSG`
    pub fn received_privmsg(&mut self, g: &mut Ctx, msg: &CIrcMessage) -> bool {
        if !msg.m_b_incoming {
            // This is a message from us
            IRCInterface::add_text(g, Some(getirc_param(msg, 0)), getirc_param(msg, 2), COLOUR_TEXT);
        } else {
            // This is a message from somebody else
            let param1 = getirc_param(msg, 1);
            let action = find_from(param1.as_bytes(), b"ACTION", 0);
            let version = find_from(param1.as_bytes(), b"VERSION", 0).is_some();
            let ping = find_from(param1.as_bytes(), b"PING", 0).is_some();
            let finger = find_from(param1.as_bytes(), b"FINGER", 0).is_some();

            if let Some(pos) = action {
                let parsed = format!("{} {}", msg.prefix.s_nick, text_from(param1, pos + 7));
                IRCInterface::add_text(g, None, &parsed, COLOUR_ACTION);
            } else if version {
                let reply = format!(
                    "NOTICE {}: {} :\u{1}VERSION UplinkIRC v{}\u{1}",
                    own_nick(g),
                    msg.prefix.s_nick,
                    g.app.version
                );
                send_line(g, &reply);
            } else if ping {
                let reply = format!("NOTICE {}: {} :{}", own_nick(g), msg.prefix.s_nick, getirc_param(msg, 1));
                send_line(g, &reply);
            } else if finger {
                let nick = own_nick(g);
                let reply = format!(
                    "NOTICE {}: {} :\u{1}FINGER {} running UplinkIRC v{}, www.introversion.co.uk\u{1}",
                    nick, msg.prefix.s_nick, nick, g.app.version
                );
                send_line(g, &reply);
            } else {
                IRCInterface::add_text(g, Some(&msg.prefix.s_nick), getirc_param(msg, 1), COLOUR_TEXT);
            }
        }

        true
    }

    /// C++ `Received_JOIN`
    pub fn received_join(&mut self, g: &mut Ctx, msg: &CIrcMessage) -> bool {
        let parsed = format!("{} has joined this channel from {}", msg.prefix.s_nick, msg.prefix.s_host);
        IRCInterface::add_text(g, None, &parsed, COLOUR_JOINPART);
        IRCInterface::add_user(g, &msg.prefix.s_nick);

        true
    }

    /// C++ `Received_PART`
    pub fn received_part(&mut self, g: &mut Ctx, msg: &CIrcMessage) -> bool {
        let parsed = format!("{} has left this channel", msg.prefix.s_nick);
        IRCInterface::add_text(g, None, &parsed, COLOUR_JOINPART);
        IRCInterface::remove_user(g, &msg.prefix.s_nick);

        true
    }

    /// C++ `Received_TOPIC`
    pub fn received_topic(&mut self, g: &mut Ctx, msg: &CIrcMessage) -> bool {
        let parsed = format!("{} has changed the topic to '{}'", msg.prefix.s_nick, getirc_param(msg, 1));
        IRCInterface::add_text(g, None, &parsed, COLOUR_TOPIC);

        true
    }

    /// C++ `Received_RPL_TOPIC` (331/332)
    pub fn received_rpl_topic(&mut self, g: &mut Ctx, msg: &CIrcMessage) -> bool {
        let parsed = format!("The topic is '{}'", getirc_param(msg, 2));
        IRCInterface::add_text(g, None, &parsed, COLOUR_TOPIC);

        true
    }

    /// C++ `Received_MOTD`
    pub fn received_motd(&mut self, g: &mut Ctx, msg: &CIrcMessage) -> bool {
        IRCInterface::add_text(g, Some("MOTD"), getirc_param(msg, 1), COLOUR_MOTD);

        true
    }

    /// C++ `Received_MODE`
    pub fn received_mode(&mut self, g: &mut Ctx, msg: &CIrcMessage) -> bool {
        let parsed = format!(
            "{} sets mode to {} {}",
            msg.prefix.s_nick,
            getirc_param(msg, 1),
            getirc_param(msg, 2)
        );
        IRCInterface::add_text(g, None, &parsed, COLOUR_MODE);

        if let Some(index) = IRCInterface::get_user(g, getirc_param(msg, 2)) {
            let new_status = if getirc_param(msg, 1).contains('+') { 1 } else { 0 };
            state_mut(g).users[index].status = new_status;
        }

        true
    }

    /// C++ `Received_NICK`
    pub fn received_nick(&mut self, g: &mut Ctx, msg: &CIrcMessage) -> bool {
        let parsed = format!("{} is now known as {}", msg.prefix.s_nick, getirc_param(msg, 0));
        IRCInterface::add_text(g, None, &parsed, COLOUR_JOINPART);

        if let Some(index) = IRCInterface::get_user(g, &msg.prefix.s_nick) {
            state_mut(g).users[index].set(getirc_param(msg, 0));
        }

        true
    }

    /// C++ `Received_QUIT`
    pub fn received_quit(&mut self, g: &mut Ctx, msg: &CIrcMessage) -> bool {
        let parsed = format!("{} has quit ({})", msg.prefix.s_nick, getirc_param(msg, 0));
        IRCInterface::add_text(g, None, &parsed, COLOUR_JOINPART);

        true
    }

    /// C++ `Received_RPL_WELCOME` (001): joins the channel once connected.
    pub fn received_rpl_welcome(&mut self, g: &mut Ctx, _msg: &CIrcMessage) -> bool {
        // We can now join the channel
        let join_command = format!("join {}: {}", own_nick(g), state(g).channel_name);
        send_line(g, &join_command);

        true
    }

    /// C++ `Received_RPL_NAMREPLY` (353)
    pub fn received_rpl_namreply(&mut self, g: &mut Ctx, msg: &CIrcMessage) -> bool {
        IRCInterface::add_text(g, None, getirc_param(msg, 3), COLOUR_JOINPART);

        // The C++ splits the list in place: every space becomes a terminator and each name that
        // starts after a space (with more than one byte left in the list) is queued. The first
        // name is queued last. Names are then handed to AddUser, which ignores names under 2 bytes.
        // PORT: the C++ first `strchr` is not guarded; with no space in the list it calls strchr on
        // NULL (a crash). Here that case simply queues only the first name.
        let param = getirc_param(msg, 3).as_bytes();
        let mut name_starts: Vec<usize> = Vec::new();
        if let Some(first_space) = find_from(param, b" ", 0) {
            let mut current = first_space;
            while let Some(space) = find_from(param, b" ", current) {
                let next = space + 1;
                if param.len() - next > 1 {
                    name_starts.push(next);
                }
                current = next;
            }
        }

        let mut names: Vec<String> = name_starts.iter().map(|&start| token_at(param, start)).collect();
        names.push(token_at(param, 0));
        for name in &names {
            IRCInterface::add_user(g, name);
        }

        true
    }

    /// C++ `Received_RPL_ENDOFNAMES` (366)
    pub fn received_rpl_endofnames(&mut self, g: &mut Ctx, msg: &CIrcMessage) -> bool {
        IRCInterface::add_text(g, None, getirc_param(msg, 2), COLOUR_JOINPART);

        true
    }

    /// C++ `Received_RPL_LUSER` (002-255, 265, 266, NOTICE)
    pub fn received_rpl_luser(&mut self, g: &mut Ctx, msg: &CIrcMessage) -> bool {
        IRCInterface::add_text(g, None, getirc_param(msg, 1), COLOUR_MOTD);

        true
    }
}

/// C++ `class IRCInterface : LocalInterfaceScreen`. The C++ statics are fields here and are reached
/// through the local interface (`g.interface().li.irc`).
// PORT: static -> field for every `static` member of IRCInterface.
#[derive(Default)]
pub struct IRCInterface {
    /// C++ `static bool connected`
    pub connected: bool,
    /// C++ `static LList<UplinkIRCMessage *> buffer` (capped at IRC_BUFFER_SIZE).
    pub buffer: Vec<UplinkIRCMessage>,
    /// C++ `static LList<UplinkIRCUser *> users`
    pub users: Vec<UplinkIRCUser>,
    /// C++ `static char channelName[256]`
    pub channel_name: String,
    /// C++ `static WinsockInit *winSockInit`
    pub win_sock_init: Option<WinsockInit>,
    /// C++ `static CIrcSession *cIrcSession`
    pub c_irc_session: Option<CIrcSession>,
    /// C++ `static UplinkIRCMonitor *uplinkIRCMonitor`
    pub uplink_irc_monitor: Option<UplinkIRCMonitor>,
    /// C++ `static CIrcSessionInfo *cIrcSessionInfo`
    pub c_irc_session_info: Option<CIrcSessionInfo>,
    /// C++ `static Image *imgSmileyHappy`, `imgSmileySad`, `imgSmileyWink` (protected statics).
    pub img_smiley_happy: Option<Image>,
    pub img_smiley_sad: Option<Image>,
    pub img_smiley_wink: Option<Image>,
}

/// The IRC statics. They live on the local interface, not on the screen object, because the screen
/// is replaced whenever another local screen opens while the C++ statics survive it.
fn state(g: &Ctx) -> &IRCInterface {
    &g.interface().li.irc
}

fn state_mut(g: &mut Ctx) -> &mut IRCInterface {
    &mut g.interface_mut().li.irc
}

/// `EclGetButton(name)->caption`. The C++ dereferences a button that always exists here; a missing
/// button reads as an empty caption.
fn caption(g: &Ctx, name: &str) -> String {
    g.ecl.get_button(name).map(|b| b.caption.clone()).unwrap_or_default()
}

/// `app->GetOptions()->GetOptionValue("graphics_screenwidth")`
fn screen_width(g: &Ctx) -> i32 {
    g.app.options.get_option_value("graphics_screenwidth")
}

/// `app->GetOptions()->GetOptionValue("graphics_screenheight")`
fn screen_height(g: &Ctx) -> i32 {
    g.app.options.get_option_value("graphics_screenheight")
}

/// `LList::ValidIndex`
fn valid_index(index: i32, len: usize) -> Option<usize> {
    usize::try_from(index).ok().filter(|&i| i < len)
}

/// `s + n` on a C string: the text from byte `n` on, or "" when `n` is past the end.
fn text_from(s: &str, n: usize) -> String {
    String::from_utf8_lossy(s.as_bytes().get(n..).unwrap_or(&[])).into_owned()
}

/// `strstr`: byte offset of `needle` in `hay` at or after `from`.
fn find_from(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if from > hay.len() {
        return None;
    }
    hay[from..]
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|p| p + from)
}

/// The name that starts at byte `start` and runs up to the next space (or the end).
fn token_at(bytes: &[u8], start: usize) -> String {
    let end = bytes[start..]
        .iter()
        .position(|&b| b == b' ')
        .map_or(bytes.len(), |p| start + p);
    String::from_utf8_lossy(&bytes[start..end]).into_owned()
}

/// `sscanf(s, "%d", &x)`: leading whitespace, an optional sign, then digits. 0 when there are none
/// (the C++ leaves the variable unset in that case).
fn c_scan_int(s: &str) -> i32 {
    let t = s.trim_start();
    let (negative, digits) = match t.as_bytes().first() {
        Some(b'-') => (true, &t[1..]),
        Some(b'+') => (false, &t[1..]),
        _ => (false, t),
    };
    let mut value: i64 = 0;
    for b in digits.bytes() {
        if !b.is_ascii_digit() {
            break;
        }
        value = value.saturating_mul(10).saturating_add((b - b'0') as i64);
    }
    if negative {
        value = -value;
    }
    value.clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

/// C++ `GETIRCPARAM(n)`: parameter `n`, or " " when the message has fewer parameters.
fn getirc_param(msg: &CIrcMessage, n: usize) -> &str {
    msg.parameters.get(n).map(String::as_str).unwrap_or(" ")
}

/// `(*IRCInterface::cIrcSessionInfo)->sNick`, or "" before a connection exists.
fn own_nick(g: &Ctx) -> String {
    state(g)
        .c_irc_session_info
        .as_ref()
        .map(|info| info.s_nick.clone())
        .unwrap_or_default()
}

/// `(*IRCInterface::cIrcSession) << line`: the line is parsed into a CIrcMessage, sent if the
/// connection is open, and then handed to the monitor as an outgoing message.
fn send_line(g: &mut Ctx, line: &str) {
    let msg = CIrcMessage::new(line, false);
    let Some(session) = state_mut(g).c_irc_session.as_mut() else {
        return;
    };
    if session.send(&msg) {
        irc_notify(g, &msg);
    }
}

/// C++ `CIrcSession::Notify` + `CIrcMonitor::OnCrossThreadsMessage`: hands one message to the
/// monitor, if it is added to the session. Messages no handler takes go to `on_irc_default`.
fn irc_notify(g: &mut Ctx, msg: &CIrcMessage) {
    if state(g).uplink_irc_monitor.is_none() {
        return;
    }
    let mut monitor = UplinkIRCMonitor::new();
    if !monitor.dispatch(g, &msg.s_command, msg) {
        monitor.on_irc_default(g, msg);
    }
}

/// `ScrollBox::SetNumItems` on the named box, if it exists.
fn sync_scroll_items(g: &mut Ctx, scroll_name: &str, count: usize) {
    if ScrollBox::get_scroll_box(g, scroll_name).is_some() {
        ScrollBox::set_num_items(g, scroll_name, count as i32);
    }
}

/// `glColor3f(r, g, b)` from a colour triple.
fn colour3f(colour: [f32; 3]) {
    gl::color3f(colour[0], colour[1], colour[2]);
}

/// C++ `CCrossThreadsMessagingDevice::ProcessMessages`: hands the messages the network thread
/// received to the monitor.
fn process_messages(g: &mut Ctx) {
    let incoming = state_mut(g)
        .c_irc_session
        .as_mut()
        .map(|session| session.take_incoming())
        .unwrap_or_default();
    for msg in &incoming {
        irc_notify(g, msg);
    }
}

impl IRCInterface {
    /// C++ `IRCInterface::ReturnKeyPressed`: sends the input box text to the channel.
    pub fn return_key_pressed(g: &mut Ctx) -> bool {
        Self::post_input(g);
        true
    }

    /// C++ `IRCInterface::CreateTalkWindow`
    pub fn create_talk_window(g: &mut Ctx) {
        //
        // Main text box

        let screenw = screen_width(g);
        let screenh = screen_height(g);
        let main_width = (screenw as f64 * 0.65) as i32;
        let main_height = (screenh as f64 * 0.8) as i32;

        g.ecl.register_button(30, 30, main_width - 15, main_height, " ", " ", "irc_maintext");
        g.ecl.register_button_callbacks("irc_maintext", Some(Self::main_text_draw), None, None, None);

        //
        // Scroll bars

        let num_rows = main_height / 15;
        let buffer_len = state(g).buffer.len() as i32;
        ScrollBox::create_scroll_box(
            g,
            "ircscroller",
            (30 + main_width) - 15,
            30,
            15,
            main_height,
            buffer_len,
            num_rows,
            buffer_len - num_rows,
            Some(Self::text_scroll_change),
        );

        //
        // Input box

        g.ecl.register_button(
            30,
            main_height + 30,
            main_width - 50,
            15,
            " ",
            "Enter your text here",
            "irc_inputbox",
        );
        g.ecl.register_button_callbacks(
            "irc_inputbox",
            Some(textbutton_draw),
            None,
            Some(button_click),
            Some(button_highlight),
        );
        g.ecl.make_button_editable("irc_inputbox");

        //
        // Post button

        g.ecl.register_button(
            (30 + main_width) - 50,
            main_height + 30,
            50,
            15,
            "Post",
            "Click to post your message",
            "irc_postbutton",
        );
        g.ecl.register_button_callback("irc_postbutton", Self::post_click);
    }

    /// C++ `IRCInterface::RemoveTalkWindow`
    pub fn remove_talk_window(g: &mut Ctx) {
        g.ecl.remove_button("irc_maintext");
        g.ecl.remove_button("irc_inputbox");
        g.ecl.remove_button("irc_postbutton");

        ScrollBox::remove_scroll_box(g, "ircscroller");
    }

    /// C++ `IRCInterface::BackBlackoutDraw` (static draw callback).
    pub fn back_blackout_draw(g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        clear_draw(g, button.x, button.y, button.width, button.height);
    }

    /// C++ `IRCInterface::MainTextDraw` (static draw callback).
    pub fn main_text_draw(g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        // Clipping

        let screenh = screen_height(g);
        gl::scissor(button.x, screenh - (button.y + button.height), button.width, button.height);
        gl::enable(gl::SCISSOR_TEST);

        clear_draw(g, button.x, button.y, button.width, button.height);
        gl::color3f(1.0, 1.0, 1.0);
        border_draw(button);

        let main_height = (screenh as f64 * 0.8) as i32;
        let num_rows = main_height / 15 - 1;
        // PORT: the C++ returns here without glDisable, so the scissor test stays on. Kept as is.
        let base_offset = match ScrollBox::get_scroll_box(g, "ircscroller") {
            Some(scroll_box) => scroll_box.current_index,
            None => return,
        };

        let (happy, sad, wink) = {
            let st = state(g);
            (
                st.img_smiley_happy.as_ref(),
                st.img_smiley_sad.as_ref(),
                st.img_smiley_wink.as_ref(),
            )
        };

        for i in 0..num_rows {
            let this_row = base_offset + i + 1;
            let buffer_len = state(g).buffer.len();
            let Some(index) = valid_index(this_row, buffer_len) else {
                continue;
            };
            let msg = &state(g).buffer[index];

            let mut xpos = button.x + 10;
            let ypos = button.y + 10 + i * 15;

            if let Some(user) = &msg.user {
                colour3f(COLOUR_USER);
                text::draw_text_default(xpos, ypos, user);
                xpos += 80;
                // TODO : Handle big nick names
            }

            gl::color3f(msg.red, msg.green, msg.blue);
            text::draw_text_default(xpos, ypos, &msg.text);

            Self::add_emoticons(g, i, ":)", happy);
            Self::add_emoticons(g, i, "=)", happy);
            Self::add_emoticons(g, i, ":-)", happy);
            Self::add_emoticons(g, i, ":(", sad);
            Self::add_emoticons(g, i, ":-(", sad);
            Self::add_emoticons(g, i, ";)", wink);
        }

        gl::disable(gl::SCISSOR_TEST);
    }

    /// C++ `IRCInterface::ConnectDraw` (static draw callback).
    pub fn connect_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        let label = if state(g).connected { "Disconnect" } else { "Connect" };

        // C++ changed the live button's caption; the draw uses a copy, so update both.
        if let Some(live) = g.ecl.get_button_mut(&button.name) {
            live.set_caption(label);
        }
        let mut shown = button.clone();
        shown.set_caption(label);

        button_draw(g, &shown, highlighted, clicked);
    }

    /// C++ `IRCInterface::UserListDraw` (static draw callback).
    pub fn user_list_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        let screenheight = screen_height(g);
        gl::scissor(button.x, screenheight - (button.y + button.height), button.width, button.height);
        gl::enable(gl::SCISSOR_TEST);

        background_draw(g, button, highlighted, clicked);

        // PORT: as in MainTextDraw, the C++ returns here without glDisable. Kept as is.
        let base_offset = match ScrollBox::get_scroll_box(g, "irc_userscroll") {
            Some(scroll_box) => scroll_box.current_index,
            None => return,
        };
        let num_rows = (button.height - 20) / 17;

        if state(g).connected {
            let xpos = button.x + 20;

            for i in 0..num_rows {
                let users_len = state(g).users.len();
                let Some(index) = valid_index(i + base_offset, users_len) else {
                    continue;
                };
                let user = &state(g).users[index];

                let ypos = button.y + 20 + i * 17;

                if user.status == 0 {
                    gl::color3f(1.0, 1.0, 1.0);
                } else {
                    gl::color3f(1.0, 0.5, 0.5);
                }

                text::draw_text_default(xpos, ypos, &user.name);
            }
        }

        gl::disable(gl::SCISSOR_TEST);
    }

    /// C++ `IRCInterface::ConnectClick` (static button callback): connects to the server in the form fields.
    pub fn connect_click(g: &mut Ctx, _button: &Button) {
        if state(g).connected {
            {
                let st = state_mut(g);
                if let Some(session) = st.c_irc_session.as_mut() {
                    session.disconnect("Bye everyone!");
                }
                // C++: RemoveMonitor, then delete the monitor, the session and the winsock object.
                st.uplink_irc_monitor = None;
                st.c_irc_session = None;
                st.win_sock_init = None;
            }

            Self::reset_users(g);
            Self::remove_talk_window(g);
        } else {
            Self::create_talk_window(g);
            Self::reset_users(g);

            {
                let st = state_mut(g);
                st.win_sock_init = Some(WinsockInit);
                st.c_irc_session = Some(CIrcSession::new());
                st.uplink_irc_monitor = Some(UplinkIRCMonitor::new());
            }

            let port = c_scan_int(&caption(g, "irc_port"));
            let server = caption(g, "irc_server");
            let full_name = caption(g, "irc_fullname");

            let mut info = CIrcSessionInfo::default();
            info.reset();
            info.s_server = server.clone();
            info.s_server_name = server;
            info.i_port = port;
            info.s_nick = caption(g, "irc_nickname");
            info.s_full_name = full_name.clone();
            info.s_user_id = full_name;
            info.b_ident_server = true;
            info.i_ident_server_port = 113;
            info.s_ident_server_type = "UNIX".to_string();

            let channel = caption(g, "irc_channel");

            let st = state_mut(g);
            st.c_irc_session_info = Some(info);
            st.channel_name = channel;

            // C++: `bool success = cIrcSession->Connect(*cIrcSessionInfo);` The result is not used,
            // so the toggle below happens either way.
            if let (Some(session), Some(info)) = (st.c_irc_session.as_mut(), st.c_irc_session_info.as_ref()) {
                let _success = session.connect(info);
            }
        }

        let st = state_mut(g);
        st.connected = !st.connected;
    }

    /// C++ `IRCInterface::PostClick` (static button callback): sends the input box text.
    pub fn post_click(g: &mut Ctx, _button: &Button) {
        Self::post_input(g);
    }

    /// The body of C++ `PostClick`. ReturnKeyPressed runs it too (C++ passes NULL, which PostClick
    /// never reads).
    fn post_input(g: &mut Ctx) {
        {
            let st = state(g);
            if !st.connected {
                return;
            }
            if st.c_irc_session.is_none() {
                return;
            }
        }

        let input = caption(g, "irc_inputbox");
        let nick = own_nick(g);
        let channel = state(g).channel_name.clone();

        // C++ `caption[1] == '\xA3'`: the second character is a pound sign. The text after it is sent.
        // PORT: '£' is two bytes in UTF-8, so the text starts after the character, not at byte 3.
        let second = input.char_indices().nth(1);
        let new_message = if let Some((index, pound)) = second.filter(|&(_, c)| c == '£') {
            text_from(&input, index + pound.len_utf8())
        } else if input.contains("/nick") {
            // PORT: byte 7 is kept from the C++ (`caption+7`), so the nick text starts 2 bytes into the argument.
            format!("nick {}: {}", nick, text_from(&input, 7))
        } else if input.contains("/action") {
            // PORT: byte 9 is kept from the C++ (`caption+9`).
            format!("privmsg {}: {} :\u{1}ACTION {}\u{1}", nick, channel, text_from(&input, 9))
        } else {
            format!("privmsg {}: {} :{}", nick, channel, text_from(&input, 1))
        };

        send_line(g, &new_message);

        if let Some(button) = g.ecl.get_button_mut("irc_inputbox") {
            button.set_caption(" ");
        }
        g.ecl.dirty_button("irc_inputbox");
    }

    /// C++ `IRCInterface::TextScrollChange` (static scroll-box callback).
    pub fn text_scroll_change(g: &mut Ctx, _name: &str, _new_value: i32) {
        g.ecl.dirty_button("irc_maintext");
    }

    /// C++ `IRCInterface::UserScrollChange` (static scroll-box callback).
    pub fn user_scroll_change(g: &mut Ctx, _name: &str, _new_value: i32) {
        g.ecl.dirty_button("irc_userlist");
    }

    /// C++ `IRCInterface::AddEmoticons` (static): draws a smiley image into the chat row `row`.
    /// `img_smiley` is None where the C++ passed NULL (the image was never loaded).
    // PORT: signature changed from `(&mut Ctx, row, smiley, &Image)`. Only reads `g`, and takes the
    // optional image so MainTextDraw can pass the statics without a borrow conflict.
    pub fn add_emoticons(g: &Ctx, row: i32, smiley: &str, img_smiley: Option<&Image>) {
        let base_offset = match ScrollBox::get_scroll_box(g, "ircscroller") {
            Some(scroll_box) => scroll_box.current_index,
            None => return,
        };
        let this_row = base_offset + row + 1;

        let buffer = &state(g).buffer;
        let Some(index) = valid_index(this_row, buffer.len()) else {
            return;
        };
        let msg = &buffer[index];

        let Some(button) = g.ecl.get_button("irc_maintext") else {
            return;
        };

        let mut xpos = button.x + 10;
        let ypos = button.y + 10 + row * 15;
        if msg.user.is_some() {
            xpos += 80;
        }

        let Some(img) = img_smiley else {
            return;
        };

        let text = msg.text.as_bytes();
        let mut next_smiley = find_from(text, smiley.as_bytes(), 0);
        while let Some(pos) = next_smiley {
            let text_so_far = String::from_utf8_lossy(&text[..pos]);
            let smiley_xpos = xpos + text::text_width_default(&text_so_far);
            next_smiley = find_from(text, smiley.as_bytes(), pos + 2);

            img.draw(smiley_xpos, ypos - 7);
        }
    }

    /// C++ `IRCInterface::AddText` (static): appends a line to the chat buffer.
    pub fn add_text(g: &mut Ctx, user: Option<&str>, text: &str, colour: [f32; 3]) {
        // Break up the text into word wrapped lines

        let screenw = screen_width(g);
        let main_width = (screenw as f64 * 0.65) as i32;

        let wrapped = wordwraptext(text, main_width);

        // Add those lines into the buffer

        for (i, the_line) in wrapped.iter().enumerate() {
            if the_line.len() > 1 {
                let this_user = if i == 0 { user } else { None };
                let mut msg = UplinkIRCMessage::default();
                msg.set(this_user, the_line, colour);
                state_mut(g).buffer.push(msg);
            }
        }

        // Trim the buffer to size

        while state(g).buffer.len() > IRC_BUFFER_SIZE {
            state_mut(g).buffer.remove(0);
        }

        // Update the scrollbox

        if let Some((current_index, num_items, window_size)) =
            ScrollBox::get_scroll_box(g, "ircscroller").map(|sb| (sb.current_index, sb.num_items, sb.window_size))
        {
            let viewing_newest = current_index == num_items - window_size;

            let buffer_len = state(g).buffer.len() as i32;
            ScrollBox::set_num_items(g, "ircscroller", buffer_len);
            if viewing_newest {
                ScrollBox::set_current_index(g, "ircscroller", buffer_len - window_size);
            }
        }

        // Tell the screen to redraw

        g.ecl.dirty_button("irc_maintext");
    }

    /// C++ `IRCInterface::ResetUsers` (static): clears the user list.
    pub fn reset_users(g: &mut Ctx) {
        state_mut(g).users.clear();

        sync_scroll_items(g, "irc_userscroll", 0);
        g.ecl.dirty_button("irc_userlist");
    }

    /// C++ `IRCInterface::AddUser` (static)
    pub fn add_user(g: &mut Ctx, name: &str) {
        // Does the user exist already?

        if Self::get_user(g, name).is_some() {
            return;
        }
        if name.len() < 2 {
            return;
        }

        // Create the new user; a leading '@' marks a channel operator.

        let (status, user_name) = match name.strip_prefix('@') {
            Some(rest) => (1, rest),
            None => (0, name),
        };
        let new_user = UplinkIRCUser {
            name: user_name.to_string(),
            status,
        };

        // Insert in order (the sort key is the lower-cased name as received, including any '@').

        let lower_case_name = lower_case_string(name);
        let mut insert_at = None;
        for (i, this_user) in state(g).users.iter().enumerate() {
            let lower_case_this_user = lower_case_string(&this_user.name);
            if (status == 1 && this_user.status == 0)
                || (status == 1 && this_user.status == 1 && lower_case_name < lower_case_this_user)
                || (status == 0 && this_user.status == 0 && lower_case_name < lower_case_this_user)
            {
                insert_at = Some(i);
                break;
            }
        }

        let st = state_mut(g);
        match insert_at {
            Some(i) => st.users.insert(i, new_user),
            None => st.users.push(new_user),
        }

        // Update the scrollbar

        let users_len = state(g).users.len();
        sync_scroll_items(g, "irc_userscroll", users_len);

        // Update the screen

        g.ecl.dirty_button("irc_userlist");
    }

    /// C++ `IRCInterface::RemoveUser` (static)
    pub fn remove_user(g: &mut Ctx, name: &str) {
        let Some(index) = state(g).users.iter().position(|user| user.name == name) else {
            return;
        };

        state_mut(g).users.remove(index);
        let users_len = state(g).users.len();
        sync_scroll_items(g, "irc_userscroll", users_len);
    }

    /// C++ `IRCInterface::GetUser` (static): index into `users` of the user called `name`.
    // PORT: returns an index instead of a pointer into the user list.
    pub fn get_user(g: &Ctx, name: &str) -> Option<usize> {
        state(g).users.iter().position(|user| user.name == name)
    }
}

impl InterfaceScreen for IRCInterface {
    /// C++ `IRCInterface::Create`
    fn create(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            return;
        }

        //
        // Title button

        let screenw = screen_width(g);
        let screenh = screen_height(g);
        let paneltop = (100.0 * ((screenw as f64 * PANELSIZE) / 188.0) + 30.0) as i32;
        let panelwidth = (screenw as f64 * PANELSIZE) as i32;

        g.ecl.register_button(
            screenw - panelwidth - 3,
            paneltop,
            panelwidth,
            15,
            "INTERNET RELAY CHAT",
            "Remove the IRC screen",
            "irc_title",
        );

        //
        // Background blackout

        let main_width = (screenw as f64 * 0.65) as i32;
        let main_height = (screenh as f64 * 0.8) as i32;

        g.ecl.register_button(0, 30, main_width, main_height, " ", " ", "irc_backblackout");
        g.ecl.register_button_callbacks("irc_backblackout", Some(Self::back_blackout_draw), None, None, None);

        //
        // User list

        g.ecl.register_button(
            screenw - panelwidth - 3,
            paneltop + 20,
            panelwidth - 15,
            370,
            "USERS",
            " ",
            "irc_userlist",
        );
        g.ecl.register_button_callbacks("irc_userlist", Some(Self::user_list_draw), None, None, None);

        let num_users = 350 / 17;
        let users_len = state(g).users.len() as i32;
        ScrollBox::create_scroll_box(
            g,
            "irc_userscroll",
            screenw - 18,
            paneltop + 20,
            15,
            370,
            users_len,
            num_users,
            0,
            Some(Self::user_scroll_change),
        );

        //
        // Connect / Disconnect button

        g.ecl.register_button(
            screenw - panelwidth - 3,
            paneltop + 400,
            panelwidth,
            15,
            "CONNECT",
            "Click to connect/disconnect",
            "irc_connect",
        );
        g.ecl.register_button_callbacks(
            "irc_connect",
            Some(Self::connect_draw),
            Some(Self::connect_click),
            Some(button_click),
            Some(button_highlight),
        );

        //
        // Connect buttons

        g.ecl.register_button(30, 150, 100, 15, "Server", "The location of the server to connect to", "irc_servertitle");
        g.ecl.register_button(30, 170, 100, 15, "Port", "The port to connect on", "irc_porttitle");
        g.ecl.register_button(30, 190, 100, 15, "Channel", "The name of the channel to join", "irc_channeltitle");
        g.ecl.register_button(30, 210, 100, 15, "Nickname", "Your nickname", "irc_nicknametitle");
        g.ecl.register_button(30, 230, 100, 15, "RealName", "Your real name", "irc_fullnametitle");

        g.ecl.register_button(130, 150, 300, 15, "irc.openprojects.net", "Enter the Server name here", "irc_server");
        g.ecl.register_button(130, 170, 300, 15, "6667", "Enter the port number here", "irc_port");
        g.ecl.register_button(130, 190, 300, 15, "#uplink", "Enter the channel name here", "irc_channel");
        g.ecl.register_button(130, 210, 300, 15, "My Nickname", "Enter your nickname here", "irc_nickname");
        g.ecl.register_button(130, 230, 300, 15, "My Real Name", "Enter your real name here", "irc_fullname");

        for name in ["irc_server", "irc_port", "irc_channel", "irc_nickname", "irc_fullname"] {
            g.ecl.register_button_callbacks(
                name,
                Some(textbutton_draw),
                None,
                Some(button_click),
                Some(button_highlight),
            );
            g.ecl.make_button_editable(name);
        }

        if state(g).connected {
            IRCInterface::create_talk_window(g);
        }

        //
        // Smiley graphics

        if state(g).img_smiley_happy.is_none() {
            let filename = g.app.options.theme_filename("irc/smileyhappy.tif");
            let mut img = Image::new();
            img.load_tif(&filename);
            state_mut(g).img_smiley_happy = Some(img);
        }
        if state(g).img_smiley_sad.is_none() {
            let filename = g.app.options.theme_filename("irc/smileysad.tif");
            let mut img = Image::new();
            img.load_tif(&filename);
            state_mut(g).img_smiley_sad = Some(img);
        }
        if state(g).img_smiley_wink.is_none() {
            let filename = g.app.options.theme_filename("irc/smileywink.tif");
            let mut img = Image::new();
            img.load_tif(&filename);
            state_mut(g).img_smiley_wink = Some(img);
        }
    }

    /// C++ `IRCInterface::Remove`
    fn remove(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            return;
        }

        g.ecl.remove_button("irc_title");
        g.ecl.remove_button("irc_backblackout");
        g.ecl.remove_button("irc_connect");

        g.ecl.remove_button("irc_servertitle");
        g.ecl.remove_button("irc_porttitle");
        g.ecl.remove_button("irc_channeltitle");
        g.ecl.remove_button("irc_nicknametitle");
        g.ecl.remove_button("irc_fullnametitle");

        g.ecl.remove_button("irc_server");
        g.ecl.remove_button("irc_port");
        g.ecl.remove_button("irc_channel");
        g.ecl.remove_button("irc_nickname");
        g.ecl.remove_button("irc_fullname");

        g.ecl.remove_button("irc_userlist");

        ScrollBox::remove_scroll_box(g, "irc_userscroll");

        IRCInterface::remove_talk_window(g);

        //
        // Smileys

        let st = state_mut(g);
        st.img_smiley_happy = None;
        st.img_smiley_sad = None;
        st.img_smiley_wink = None;
    }

    /// C++ `IRCInterface::Update`
    fn update(&mut self, g: &mut Ctx) {
        process_messages(g);
    }

    /// C++ `IRCInterface::IsVisible`
    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("irc_title").is_some()
    }

    /// C++ `IRCInterface::ReturnKeyPressed` (called on the current screen by opengl.cpp).
    fn return_key_pressed(&mut self, g: &mut Ctx) -> bool {
        IRCInterface::return_key_pressed(g)
    }

    fn screen_id(&self) -> i32 {
        SCREEN_IRC
    }

    impl_as_any!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::app::App;
    use crate::game::game::Game;
    use crate::interface::interface::Interface;

    #[test]
    fn parses_full_prefix_and_trailing_parameter() {
        let msg = CIrcMessage::new(":alice!al@host.example PRIVMSG #uplink :hello there :)", true);
        assert_eq!(msg.prefix.s_nick, "alice");
        assert_eq!(msg.prefix.s_user, "al");
        assert_eq!(msg.prefix.s_host, "host.example");
        assert_eq!(msg.s_command, "PRIVMSG");
        assert_eq!(msg.parameters, vec!["#uplink", "hello there :)"]);
        assert!(msg.m_b_incoming);
    }

    #[test]
    fn parses_server_prefix_and_numerics() {
        let msg = CIrcMessage::new(":irc.local 353 Jag = #uplink :@Op alice bob ", true);
        assert_eq!(msg.prefix.s_nick, "irc.local");
        assert_eq!(msg.prefix.s_user, "");
        assert_eq!(msg.prefix.s_host, "");
        assert_eq!(msg.s_command, "353");
        assert_eq!(msg.parameters, vec!["Jag", "=", "#uplink", "@Op alice bob "]);

        let ping = CIrcMessage::new("PING   :irc.local", true);
        assert_eq!(ping.prefix, CIrcPrefix::default());
        assert_eq!(ping.s_command, "PING");
        assert_eq!(ping.parameters, vec!["irc.local"]);

        let nick_only = CIrcMessage::new(":bob!b@h NICK bobby", true);
        assert_eq!(nick_only.parameters, vec!["bobby"]);

        let empty = CIrcMessage::new("", true);
        assert_eq!(empty.s_command, "");
        assert!(empty.parameters.is_empty());
    }

    #[test]
    fn game_lines_name_the_sender_in_parameter_zero() {
        // IRCInterface::PostClick: "privmsg <nick>: <channel> :<text>"
        let msg = CIrcMessage::new("privmsg Jag: #uplink :hi all", false);
        assert_eq!(msg.s_command, "PRIVMSG");
        assert_eq!(msg.parameters, vec!["Jag:", "#uplink", "hi all"]);
        assert!(!msg.m_b_incoming);
        assert_eq!(msg.as_string(), "PRIVMSG #uplink :hi all\r\n");

        // Received_RPL_WELCOME: "join <nick>: <channel>"
        assert_eq!(CIrcMessage::new("join Jag: #uplink", false).as_string(), "JOIN :#uplink\r\n");
        // CTCP ACTION
        assert_eq!(
            CIrcMessage::new("privmsg Jag: #uplink :\u{1}ACTION waves\u{1}", false).as_string(),
            "PRIVMSG #uplink :\u{1}ACTION waves\u{1}\r\n"
        );
    }

    #[test]
    fn as_string_writes_the_prefix() {
        let msg = CIrcMessage::new(":alice!al@host PRIVMSG Jag #uplink :yo", true);
        assert_eq!(msg.as_string(), ":alice!al@host PRIVMSG #uplink :yo\r\n");
        let msg = CIrcMessage::new(":irc.local 001 Jag :Welcome", true);
        assert_eq!(msg.as_string(), ":irc.local 001 :Welcome\r\n");
    }

    #[test]
    fn take_lines_keeps_unfinished_line() {
        let mut buf = b"one\r\ntwo\n\nthr".to_vec();
        assert_eq!(irc_take_lines(&mut buf), vec!["one", "two"]);
        assert_eq!(buf, b"thr");
        buf.extend_from_slice(b"ee\r\n");
        assert_eq!(irc_take_lines(&mut buf), vec!["three"]);
        assert!(buf.is_empty());
    }

    #[test]
    fn connect_failure_closes_the_session() {
        // Port 1 on localhost: nothing listens there, so the connection is refused.
        let mut session = CIrcSession::new();
        let info = CIrcSessionInfo {
            s_server: "127.0.0.1".to_string(),
            i_port: 1,
            s_nick: "Jag".to_string(),
            ..CIrcSessionInfo::default()
        };
        assert!(session.connect(&info));
        let deadline = Instant::now() + Duration::from_secs(10);
        while session.is_open() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(!session.is_open());
        assert!(!session.send(&CIrcMessage::new("privmsg Jag: #uplink :hi", false)));
        assert!(session.take_incoming().is_empty());
    }

    #[test]
    fn session_talks_to_a_local_server() {
        use std::io::{BufRead, BufReader};
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut writer = stream;
            let mut got = Vec::new();
            let mut line = String::new();
            for _ in 0..2 {
                line.clear();
                reader.read_line(&mut line).unwrap();
                got.push(line.trim_end().to_string());
            }
            writer.write_all(b":irc.local 001 Jag :Welcome\r\nPING :tok123\r\n").unwrap();
            for _ in 0..2 {
                line.clear();
                reader.read_line(&mut line).unwrap();
                got.push(line.trim_end().to_string());
            }
            got
        });

        let mut session = CIrcSession::new();
        let info = CIrcSessionInfo {
            s_server: "127.0.0.1".to_string(),
            i_port: port as i32,
            s_nick: "Jag".to_string(),
            s_user_id: "jag".to_string(),
            s_full_name: "Jag the Hero".to_string(),
            ..CIrcSessionInfo::default()
        };
        assert!(session.connect(&info));
        assert!(session.send(&CIrcMessage::new("privmsg Jag: #uplink :hello", false)));

        let got = server.join().unwrap();
        assert_eq!(got[0], "NICK Jag");
        assert!(got[1].starts_with("USER jag ") && got[1].ends_with(" server :Jag the Hero"));
        // The game's line was queued before the server spoke; the PONG follows the PING.
        assert!(got[2..].contains(&"PRIVMSG #uplink :hello".to_string()));
        assert!(got[2..].contains(&"PONG :tok123".to_string()));

        let mut received = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(10);
        while received.len() < 3 && Instant::now() < deadline {
            received.extend(session.take_incoming());
            std::thread::sleep(Duration::from_millis(20));
        }
        let commands: Vec<(&str, bool)> =
            received.iter().map(|m| (m.s_command.as_str(), m.m_b_incoming)).collect();
        assert_eq!(commands, vec![("001", true), ("PONG", false), ("PING", true)]);
    }

    fn test_ctx() -> Ctx {
        let mut g = Ctx::new(App::new(), Game::new());
        g.app.options.create_default_options();
        g.game.ui = Some(Box::new(Interface::new()));
        g
    }

    #[test]
    fn user_list_puts_ops_first_then_sorts_by_name() {
        let mut g = test_ctx();
        let reply = CIrcMessage::new(":irc.local 353 Jag = #uplink :Jag @Op carol alice x", true);
        UplinkIRCMonitor::new().received_rpl_namreply(&mut g, &reply);
        let names: Vec<(String, i32)> =
            state(&g).users.iter().map(|u| (u.name.clone(), u.status)).collect();
        // "x" is shorter than 2 bytes and ignored, as in C++.
        assert_eq!(
            names,
            vec![
                ("Op".to_string(), 1),
                ("alice".to_string(), 0),
                ("carol".to_string(), 0),
                ("Jag".to_string(), 0)
            ]
        );

        UplinkIRCMonitor::new().received_part(&mut g, &CIrcMessage::new(":carol!c@h PART #uplink", true));
        UplinkIRCMonitor::new().received_mode(&mut g, &CIrcMessage::new(":Op!o@h MODE #uplink +o alice", true));
        let names: Vec<(String, i32)> =
            state(&g).users.iter().map(|u| (u.name.clone(), u.status)).collect();
        assert_eq!(
            names,
            vec![("Op".to_string(), 1), ("alice".to_string(), 1), ("Jag".to_string(), 0)]
        );
    }

    #[test]
    fn monitor_messages_match_the_cpp_texts() {
        let mut g = test_ctx();
        state_mut(&mut g).uplink_irc_monitor = Some(UplinkIRCMonitor::new());
        for line in [
            ":alice!al@host.example JOIN :#uplink",
            ":alice!al@host.example PRIVMSG #uplink :hi :)",
            ":bob!b@h PRIVMSG #uplink :\u{1}ACTION waves\u{1}",
            ":irc.local 332 Jag #uplink :Welcome to Uplink",
            ":bob!b@h QUIT :gone",
        ] {
            irc_notify(&mut g, &CIrcMessage::new(line, true));
        }
        let lines: Vec<(Option<String>, String)> =
            state(&g).buffer.iter().map(|m| (m.user.clone(), m.text.clone())).collect();
        assert_eq!(
            lines,
            vec![
                (None, "alice has joined this channel from host.example".to_string()),
                (Some("alice".to_string()), "hi :)".to_string()),
                (None, "bob waves\u{1}".to_string()),
                (None, "The topic is 'Welcome to Uplink'".to_string()),
                (None, "bob has quit (gone)".to_string()),
            ]
        );
        assert_eq!(state(&g).buffer[2].red, COLOUR_ACTION[0]);
    }
}
