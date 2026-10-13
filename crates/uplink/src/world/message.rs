//! Port of source/world/message.h + source/world/message.cpp

use serde::{Deserialize, Serialize};

use crate::ctx::Ctx;
use crate::tosser::BTree;
use crate::world::computer::databank::Data;
use crate::world::date::Date;
use crate::world::person::Person;

/// The data representing a message (system message, email).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Message {
    // PORT: `subject`/`body` were protected `char *` read through GetSubject/GetBody.
    pub subject: String,
    pub body: String,

    /// An attached file/program
    pub data: Option<Box<Data>>,

    pub to: String,
    pub from: String,

    /// All useful ip addresses (to be added to links)
    pub links: Vec<String>,
    /// Access codes, indexed on ip
    pub codes: BTree<String>,

    /// Date of send
    pub date: Date,
}

impl Default for Message {
    fn default() -> Self {
        Message {
            subject: " ".to_string(),
            body: " ".to_string(),
            data: None,
            to: " ".to_string(),
            from: " ".to_string(),
            links: Vec::new(),
            codes: BTree::new(),
            date: Date::default(),
        }
    }
}

impl Message {
    pub fn new() -> Self {
        Message::default()
    }

    pub fn set_to(&mut self, newto: &str) {
        self.to = newto.to_string();
    }

    pub fn set_from(&mut self, newfrom: &str) {
        self.from = newfrom.to_string();
    }

    pub fn set_subject(&mut self, newsubject: &str) {
        self.subject = newsubject.to_string();
    }

    pub fn set_body(&mut self, newbody: &str) {
        self.body = newbody.to_string();
    }

    pub fn set_date(&mut self, newdate: &Date) {
        self.date.set_date_from(newdate);
    }

    pub fn give_link(&mut self, ip: &str) {
        self.links.push(ip.to_string());
    }

    pub fn give_code(&mut self, ip: &str, code: &str) {
        self.codes.put_data(ip, code.to_string());
    }

    /// Attaches a copy of `newdata`.
    pub fn attach_data(&mut self, newdata: &Data) {
        self.data = Some(Box::new(newdata.clone()));
    }

    pub fn get_data(&self) -> Option<&Data> {
        self.data.as_deref()
    }

    pub fn get_subject(&self) -> &str {
        &self.subject
    }

    pub fn get_body(&self) -> &str {
        &self.body
    }

    /// Stamps the message with the world date and delivers it to `self.to` via
    /// `Person::give_message(g, to, msg)` (which dispatches Person/Agent/Player behaviour);
    /// bounces a "MAIL SERVER" failure message back to `from` if the recipient is unknown.
    // PORT: consumes the message, since delivery transfers ownership to the recipient.
    pub fn send(mut self, g: &mut Ctx) {
        let now = g.world().date.clone();
        self.set_date(&now);

        if g.world().get_person(&self.to).is_some() {
            let to = self.to.clone();
            Person::give_message(g, &to, self);
        } else {
            // Send an error message to the sender
            let mut msg = Message::new();
            msg.set_to(&self.from);
            msg.set_from("MAIL SERVER");
            msg.set_subject("Failed to deliver this email - unknown recipient");
            msg.set_body(&self.body);
            msg.send(g);
        }
    }
}
