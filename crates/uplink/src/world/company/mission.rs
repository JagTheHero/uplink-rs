//! Port of source/world/company/mission.h + source/world/company/mission.cpp

use serde::{Deserialize, Serialize};

use crate::tosser::BTree;
use crate::world::date::Date;
use crate::world::generator::missiongenerator::MISSION_NONE;

pub const MISSIONPAYMENT_AFTERCOMPLETION: i32 = 0;
pub const MISSIONPAYMENT_HALFATSTART: i32 = 1;
pub const MISSIONPAYMENT_ALLATSTART: i32 = 2;

/// A mission or task to be completed. Lives in `CompanyUplink::missions` or an agent's mission list
/// (key = index in that list).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Mission {
    /// Listed on bbs boards
    pub details: Option<String>,
    /// Listed after acceptance (contains actual target details)
    pub fulldetails: Option<String>,
    /// Can be None
    pub duedate: Option<Date>,

    /// Answers to these questions; can be None
    pub whysomuchmoney: Option<String>,
    pub howsecure: Option<String>,
    pub whoisthetarget: Option<String>,

    /// Defined in MissionGenerator
    pub type_: i32,
    pub employer: String,
    pub contact: String,
    pub description: String,

    pub createdate: Date,

    /// Strings used to determine if this mission is done
    pub completiona: String,
    pub completionb: String,
    pub completionc: String,
    pub completiond: String,
    pub completione: String,

    pub payment: i32,
    /// You can negotiate this if your rating is high enough
    pub maxpayment: i32,
    /// Eg all now, half now, none now
    pub paymentmethod: i32,

    pub difficulty: i32,
    /// If you have this rating you can view this mission
    pub minuplinkrating: i32,
    /// If you have this rating you can auto-accept this mission
    pub acceptrating: i32,

    /// NPC's will take this one first if this is true
    pub npcpriority: bool,

    /// All useful ip addresses (to be added to links)
    pub links: Vec<String>,
    /// Access codes, indexed on ip
    pub codes: BTree<String>,
}

impl Default for Mission {
    fn default() -> Self {
        Mission {
            details: None,
            fulldetails: None,
            duedate: None,
            whysomuchmoney: None,
            howsecure: None,
            whoisthetarget: None,
            type_: MISSION_NONE,
            employer: String::new(),
            contact: String::new(),
            description: String::new(),
            // Date::default() is the C++ Date() constructor (year 1000 = "not set")
            createdate: Date::default(),
            completiona: String::new(),
            completionb: String::new(),
            completionc: String::new(),
            completiond: String::new(),
            completione: String::new(),
            payment: 0,
            maxpayment: 0,
            paymentmethod: MISSIONPAYMENT_AFTERCOMPLETION,
            difficulty: 0,
            minuplinkrating: -1,
            acceptrating: 0,
            npcpriority: false,
            links: Vec::new(),
            codes: BTree::new(),
        }
    }
}

impl Mission {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_type(&mut self, new_type: i32) {
        self.type_ = new_type;
    }

    /// `None` leaves the corresponding completion string unchanged.
    pub fn set_completion(
        &mut self,
        a: Option<&str>,
        b: Option<&str>,
        c: Option<&str>,
        d: Option<&str>,
        e: Option<&str>,
    ) {
        if let Some(a) = a {
            self.completiona = a.to_string();
        }
        if let Some(b) = b {
            self.completionb = b.to_string();
        }
        if let Some(c) = c {
            self.completionc = c.to_string();
        }
        if let Some(d) = d {
            self.completiond = d.to_string();
        }
        if let Some(e) = e {
            self.completione = e.to_string();
        }
    }

    pub fn set_create_date(&mut self, date: &Date) {
        // Date::SetDate(Date*) copies only the six time fields (not updateme/previousupdate).
        self.createdate.set_date_from(date);
    }

    // PORT: C++ asserted the employer company exists in the world (game->GetWorld()->GetCompany);
    // dropped so this needs no world access. Callers resolve the company themselves.
    pub fn set_employer(&mut self, newemployer: &str) {
        self.employer = newemployer.to_string();
    }

    // PORT: C++ asserted the contact person exists in the world (GetPerson); dropped, see set_employer.
    pub fn set_contact(&mut self, newcontact: &str) {
        self.contact = newcontact.to_string();
    }

    /// Pass `newmaxpayment = -1` to make the max payment equal to the payment.
    pub fn set_payment(&mut self, newpayment: i32, newmaxpayment: i32) {
        self.payment = newpayment;

        if newmaxpayment != -1 {
            self.maxpayment = newmaxpayment;
        } else {
            self.maxpayment = self.payment;
        }
    }

    pub fn set_difficulty(&mut self, newdifficulty: i32) {
        self.difficulty = newdifficulty;
    }

    pub fn set_min_rating(&mut self, newrating: i32) {
        self.minuplinkrating = newrating;
    }

    pub fn set_accept_rating(&mut self, newrating: i32) {
        self.acceptrating = newrating;
    }

    pub fn set_npc_priority(&mut self, newpriority: bool) {
        self.npcpriority = newpriority;
    }

    pub fn set_description(&mut self, newdescription: &str) {
        self.description = newdescription.to_string();
    }

    pub fn set_details(&mut self, newdetails: &str) {
        self.details = Some(newdetails.to_string());
    }

    pub fn set_full_details(&mut self, newdetails: &str) {
        self.fulldetails = Some(newdetails.to_string());
    }

    pub fn set_why_so_much_money(&mut self, answer: Option<&str>) {
        self.whysomuchmoney = answer.map(|s| s.to_string());
    }

    pub fn set_how_secure(&mut self, answer: Option<&str>) {
        self.howsecure = answer.map(|s| s.to_string());
    }

    pub fn set_who_is_the_target(&mut self, answer: Option<&str>) {
        self.whoisthetarget = answer.map(|s| s.to_string());
    }

    pub fn give_link(&mut self, ip: &str) {
        self.links.push(ip.to_string());
    }

    pub fn give_code(&mut self, ip: &str, code: &str) {
        self.codes.put_data(ip, code.to_string());
    }

    /// Takes ownership of the date (C++ stored the pointer); `None` clears it.
    pub fn set_due_date(&mut self, newdate: Option<Date>) {
        self.duedate = newdate;
    }

    /// Can return None.
    pub fn get_due_date(&self) -> Option<&Date> {
        self.duedate.as_ref()
    }

    pub fn get_due_date_mut(&mut self) -> Option<&mut Date> {
        self.duedate.as_mut()
    }

    /// C++ asserted non-NULL.
    pub fn get_details(&self) -> &str {
        self.details
            .as_deref()
            .expect("Mission::get_details: no details")
    }

    /// C++ asserted non-NULL.
    pub fn get_full_details(&self) -> &str {
        self.fulldetails
            .as_deref()
            .expect("Mission::get_full_details: no full details")
    }

    pub fn get_why_so_much_money(&self) -> Option<&str> {
        self.whysomuchmoney.as_deref()
    }

    pub fn get_how_secure(&self) -> Option<&str> {
        self.howsecure.as_deref()
    }

    pub fn get_who_is_the_target(&self) -> Option<&str> {
        self.whoisthetarget.as_deref()
    }
}
