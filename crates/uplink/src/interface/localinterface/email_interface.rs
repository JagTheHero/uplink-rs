//! Port of source/interface/localinterface/email_interface.h + source/interface/localinterface/email_interface.cpp
//! Views one email (`Message`) from the player's inbox.

use crate::app::opengl_interface::{create_stextbox, remove_stextbox, textbox_draw};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::impl_as_any;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::localinterface::localinterface::{LocalInterface, PANELSIZE, SCREEN_EMAIL, SCREEN_NONE, SCREEN_SENDMAIL};
use crate::tosser::BTree;
use crate::world::message::Message;

#[derive(Debug, Clone)]
pub struct EmailInterface {
    /// Index into `player.messages` of the email being shown (C++ `int index`).
    pub index: i32,
    pub timesync: i32,
    /// C++ `Message *message`, a non-owning pointer into `player.messages`. Kept here as a
    /// snapshot clone; the body compares it against `player.messages[index]` where C++ compared pointers.
    // PORT: pointer identity replaced by a snapshot plus the index.
    pub message: Option<Message>,
}

impl Default for EmailInterface {
    fn default() -> Self {
        EmailInterface {
            index: -1,
            timesync: 0,
            message: None,
        }
    }
}

/// C++ `ostrstream`: inserting a NULL `char *` sets badbit, after which every later insertion is dropped.
pub(crate) struct Stream {
    buf: String,
    bad: bool,
}

impl Stream {
    pub(crate) fn new() -> Self {
        Stream { buf: String::new(), bad: false }
    }

    pub(crate) fn put(&mut self, s: &str) {
        if !self.bad {
            self.buf.push_str(s);
        }
    }

    /// Inserts a `char *` that may be NULL (a C++ `DArray::GetData` past the end).
    pub(crate) fn put_opt(&mut self, s: Option<&str>) {
        match s {
            Some(s) => self.put(s),
            None => self.bad = true,
        }
    }

    pub(crate) fn into_string(self) -> String {
        self.buf
    }
}

/// The "links included" and "codes included" listing shared by the email and mission screens.
pub(crate) fn append_links_and_codes(out: &mut Stream, g: &Ctx, links: &[String], codes: &BTree<String>) {
    if !links.is_empty() {
        out.put("\n\nLinks included : \n");
        for link in links {
            let computer = g
                .world()
                .get_vlocation(link)
                .and_then(|vl| g.world().get_computer(&vl.computer));
            match computer {
                Some(c) => out.put(&format!("- {}\n", c.name)),
                None => out.put(&format!("- {}(Invalid)\n", link)),
            }
        }
    }

    if !codes.is_empty() {
        out.put("\n\nCodes included : \n");
        // ConvertToDArray / ConvertIndexToDArray: BTree pre-order
        for (i, (ip, value)) in codes.iter().enumerate() {
            let computer = g
                .world()
                .get_vlocation(ip)
                .and_then(|vl| g.world().get_computer(&vl.computer));
            match computer {
                Some(c) => out.put(&format!("[{}]\n", c.name)),
                None => {
                    // The C++ prints `links.GetData(i)` here, indexed by the codes position. Past the end
                    // of `links` that is NULL, which drops the rest of the text.
                    out.put("[");
                    out.put_opt(links.get(i).map(String::as_str));
                    out.put("(Invalid)]\n");
                }
            }
            out.put(&format!("- {}\n", value));
        }
    }
}

/// `(screenw, screenh, paneltop, panelwidth)`, recomputed from the options as in the C++.
fn panel_metrics(g: &Ctx) -> (i32, i32, i32, i32) {
    let screenw = g.app.options.get_option_value("graphics_screenwidth");
    let screenh = g.app.options.get_option_value("graphics_screenheight");
    let paneltop = (100.0 * ((screenw as f64 * PANELSIZE) / 188.0) + 30.0) as i32;
    let panelwidth = (screenw as f64 * PANELSIZE) as i32;
    (screenw, screenh, paneltop, panelwidth)
}

/// Unix seconds, C++ `time(NULL)`.
fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// `player.messages.GetData(index)`: the message at `index`, if there is one.
fn message_at(g: &Ctx, index: i32) -> Option<&Message> {
    usize::try_from(index).ok().and_then(|i| g.player().messages.get(i))
}

/// Identity of a message, compared by content (C++ compared pointers).
fn same_message(a: &Message, b: &Message) -> bool {
    match (serde_json::to_string(a), serde_json::to_string(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

/// Where `snapshot` currently sits in `player.messages`: `hint` if it still matches there, else its
/// new position. C++ DArray removals leave holes so a message never changes index; `Vec::remove`
/// shifts later messages down, so the pointer-identity check is done by searching for the message.
fn find_message(g: &Ctx, snapshot: &Message, hint: i32) -> Option<usize> {
    if message_at(g, hint).is_some_and(|real| same_message(snapshot, real)) {
        return usize::try_from(hint).ok();
    }
    g.player().messages.iter().position(|real| same_message(snapshot, real))
}

/// The message this screen is showing, with the index it was set to.
fn current_email(g: &Ctx) -> Option<(i32, Message)> {
    let em = g.interface().li.screen.get::<EmailInterface>()?;
    Some((em.index, em.message.clone()?))
}

impl EmailInterface {
    /// C++ `EmailInterface::TitleClick` (static button callback).
    pub fn title_click(g: &mut Ctx, _button: &Button) {
        LocalInterface::run_screen(g, SCREEN_NONE, -1);
    }

    /// C++ `EmailInterface::EmailClose` (static button callback).
    pub fn email_close(g: &mut Ctx, _button: &Button) {
        LocalInterface::run_screen(g, SCREEN_NONE, -1);
    }

    /// C++ `EmailInterface::EmailReply` (static button callback).
    pub fn email_reply(g: &mut Ctx, _button: &Button) {
        let Some((index, message)) = current_email(g) else {
            debug_assert!(false, "EmailInterface::email_reply: no email is shown");
            return;
        };

        // Check it matches (ie the player hasn't lost the email
        if find_message(g, &message, index).is_none() {
            LocalInterface::run_screen(g, SCREEN_NONE, -1);
            // PORT: C++ falls through here and reads the message it has just lost; stop instead.
            return;
        }

        let contact = message.from.clone();
        let body = format!("RE your email\n'{}'\n\n", message.get_subject());

        LocalInterface::run_screen(g, SCREEN_SENDMAIL, -1);
        LocalInterface::update(g);

        g.ecl.register_caption_change("sendmail_to", &contact, 1, None); // Should occur instantly
        g.ecl.register_caption_change_auto("sendmail_subject", "RE your email", None);
        g.ecl.register_caption_change("sendmail_body box", &body, 1000, None);
    }

    /// C++ `EmailInterface::EmailDelete` (static button callback).
    pub fn email_delete(g: &mut Ctx, button: &Button) {
        let Some((index, message)) = current_email(g) else {
            debug_assert!(false, "EmailInterface::email_delete: no email is shown");
            return;
        };

        // Check it matches (ie the player hasn't lost the email
        if let Some(real_index) = find_message(g, &message, index) {
            // C++ `Person::messages` is an LList: RemoveData unlinks the message and the later ones
            // shift down, exactly like Vec::remove.
            g.player_mut().messages.remove(real_index);
        }

        Self::email_close(g, button);
    }

    /// C++ `EmailInterface::SetMessage`: looks the message up in `player.messages`.
    pub fn set_message(&mut self, g: &mut Ctx, newindex: i32) {
        self.index = newindex;
        self.message = message_at(g, newindex).cloned();
        debug_assert!(self.message.is_some(), "EmailInterface::set_message: no message at index {newindex}");
    }
}

impl InterfaceScreen for EmailInterface {
    fn create(&mut self, g: &mut Ctx) {
        let Some(message) = self.message.as_ref() else {
            debug_assert!(false, "EmailInterface::create without a message");
            return;
        };

        if self.is_visible(g) {
            return;
        }

        let (screenw, screenh, paneltop, panelwidth) = panel_metrics(g);

        g.ecl.register_button(screenw - panelwidth - 3, paneltop, panelwidth, 15, "EMAIL", "Remove the email screen", "email_title");
        g.ecl.register_button_callback("email_title", Self::title_click);

        let from = format!("From : {}", message.from);
        let date = format!("Date : {}", message.date.get_short_string());
        let subject = format!("Subject : {}", message.get_subject());

        let mut body = Stream::new();
        body.put(message.get_body());

        // Concatenate any links and codes onto the end of the message
        append_links_and_codes(&mut body, g, &message.links, &message.codes);

        // Concatenate the data file onto the message if it exists
        if let Some(data) = message.get_data() {
            body.put("\n\nData included : \n");
            body.put(&data.title);
        }
        let body = body.into_string();

        g.ecl.register_button(screenw - panelwidth - 3, paneltop + 20, panelwidth, 30, "", "", "email_from");
        g.ecl.register_button(screenw - panelwidth - 3, paneltop + 55, panelwidth, 15, "", "", "email_date");
        g.ecl.register_button(screenw - panelwidth - 3, paneltop + 75, panelwidth, 30, "", "", "email_subject");
        g.ecl.register_button_callbacks("email_from", Some(textbox_draw), None, None, None);
        g.ecl.register_button_callbacks("email_date", Some(textbox_draw), None, None, None);
        g.ecl.register_button_callbacks("email_subject", Some(textbox_draw), None, None, None);
        g.ecl.register_caption_change("email_from", &from, 100, None);
        g.ecl.register_caption_change("email_date", &date, 100, None);
        g.ecl.register_caption_change("email_subject", &subject, 100, None);

        let boxheight = screenh - (paneltop + 110) - 70;
        create_stextbox(g, screenw - panelwidth - 3, paneltop + 110, panelwidth, boxheight, &body, "email_body");

        let x1 = screenw - panelwidth - 2;
        let x2 = x1 + panelwidth / 3 + 2;
        let x3 = x2 + panelwidth / 3 + 2;
        let y = paneltop + 110 + boxheight + 5;
        let w = panelwidth / 3 - 2;
        g.ecl.register_button(x1, y, w, 15, "Close", "Close and store this email", "email_close");
        g.ecl.register_button(x2, y, w, 15, "Reply", "Reply to this email", "email_reply");
        g.ecl.register_button(x3, y, w, 15, "Delete", "Close and delete this email", "email_delete");
        g.ecl.register_button_callback("email_close", Self::email_close);
        g.ecl.register_button_callback("email_reply", Self::email_reply);
        g.ecl.register_button_callback("email_delete", Self::email_delete);
    }

    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            g.ecl.remove_button("email_title");
            g.ecl.remove_button("email_from");
            g.ecl.remove_button("email_date");
            g.ecl.remove_button("email_subject");
            remove_stextbox(g, "email_body");
            g.ecl.remove_button("email_close");
            g.ecl.remove_button("email_reply");
            g.ecl.remove_button("email_delete");
        }
    }

    fn update(&mut self, g: &mut Ctx) {
        if now_secs() > self.timesync as i64 + 1 {
            // Check it matches (ie the player hasn't lost the email
            let found = self.message.as_ref().and_then(|snapshot| find_message(g, snapshot, self.index));
            match found {
                Some(pos) => self.index = pos as i32,
                None => LocalInterface::run_screen(g, SCREEN_NONE, -1),
            }
            self.timesync = now_secs() as i32;
        }
    }

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("email_title").is_some()
    }

    fn screen_id(&self) -> i32 {
        SCREEN_EMAIL
    }

    impl_as_any!();
}
