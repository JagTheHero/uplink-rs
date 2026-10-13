//! Port of source/world/company/company.h + source/world/company/company.cpp

use serde::{Deserialize, Serialize};

use crate::ctx::Ctx;
use crate::world::company::companyuplink::CompanyUplink;
use crate::world::generator::numbergenerator::NumberGenerator;

pub const COMPANYTYPE_UNKNOWN: i32 = 0;

pub const COMPANYTYPE_INDUSTRIAL: i32 = 1;
pub const COMPANYTYPE_COMMERCIAL: i32 = 2;
pub const COMPANYTYPE_FINANCIAL: i32 = 3;
pub const COMPANYTYPE_ACADEMIC: i32 = 4;
pub const COMPANYTYPE_GOVERNMENT: i32 = 6;

/// A company in the game world (corporation, military, government, academic...).
/// World key: `name`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Company {
    /// Indexed on month (0..12).
    pub sharehistory: [i32; 12],
    /// Most recently set value index in `sharehistory`.
    pub lastmonthset: i32,

    pub name: String,
    pub boss: String,
    pub administrator: String,

    pub size: i32,
    pub type_: i32,
    /// Percentage.
    pub growth: i32,
    /// eg -100 = Satanic ... 0 = Neutral ... +100 = Angelic.
    pub alignment: i32,

    /// Company -> CompanyUplink (PORTING.md §3.5): the extra fields, only set for the company "Uplink".
    pub uplink: Option<Box<CompanyUplink>>,
}

impl Default for Company {
    fn default() -> Self {
        Company {
            sharehistory: [0; 12],
            lastmonthset: 0,
            name: " ".to_string(),
            boss: "Unlisted".to_string(),
            administrator: "Unlisted".to_string(),
            size: 0,
            type_: 0,
            growth: 0,
            alignment: 0,
            uplink: None,
        }
    }
}

impl Company {
    pub fn new() -> Self {
        Self::default()
    }

    /// `CompanyUplink::CompanyUplink()`: a Company named "Uplink" carrying the CompanyUplink data.
    pub fn new_uplink() -> Company {
        let mut c = Company::new();
        c.set_name("Uplink");
        c.uplink = Some(Box::new(CompanyUplink::new()));
        c
    }

    pub fn set_name(&mut self, newname: &str) {
        self.name = newname.to_string();
    }

    pub fn set_size(&mut self, newsize: i32) {
        self.size = newsize;
    }

    pub fn set_type(&mut self, new_type: i32) {
        self.type_ = new_type;
    }

    pub fn set_growth(&mut self, newgrowth: i32) {
        self.growth = newgrowth;
    }

    pub fn set_alignment(&mut self, newalignment: i32) {
        self.alignment = newalignment;
    }

    pub fn set_boss(&mut self, bossname: &str) {
        self.boss = bossname.to_string();
    }

    pub fn set_admin(&mut self, adminname: &str) {
        self.administrator = adminname.to_string();
    }

    /// Applies growth; records the new share price for the current game month.
    /// Reads the game date and uses NumberGenerator, so it takes the context and the company name.
    pub fn grow(g: &mut Ctx, company_name: &str, dayspast: i32) {
        let month = g.world().date.get_month() - 1;
        let company = g
            .world_mut()
            .get_company_mut(company_name)
            .expect("Company::grow: company is not in the world");

        let fraction_of_growth = dayspast as f32 / 90.0; // 90 days is ~ 1/4 year

        // C++: size *= (100.0 + (fraction_of_growth * growth)) / 100.0;  (int size, double result)
        let factor = (100.0 + (fraction_of_growth * company.growth as f32) as f64) / 100.0;
        company.size = (company.size as f64 * factor) as i32;

        // Put our new price into our share history
        debug_assert!((0..12).contains(&month));
        let month_index = month as usize;
        company.sharehistory[month_index] = NumberGenerator::apply_variance(company.size, 30);
        if company.sharehistory[month_index] < 1 {
            company.sharehistory[month_index] = 1;
        }

        company.lastmonthset = month;

        // If the previous month share price is 0, then we should
        // generate previous share prices to fill in the gap
        let mut index = month - 1;
        if index < 0 {
            index = 11;
        }

        while company.sharehistory[index as usize] == 0 {
            company.sharehistory[index as usize] = NumberGenerator::random_number(company.size);
            index -= 1;
            if index < 0 {
                index = 11;
            }
        }
    }

    pub fn vary_growth(&mut self) {
        self.growth = NumberGenerator::apply_variance(self.growth, 20);
    }

    /// If `month` is -1, returns the price of the share now.
    pub fn get_share_price(&self, month: i32) -> i32 {
        let month = if month == -1 {
            self.lastmonthset
        } else {
            month
        };
        debug_assert!((0..12).contains(&month));
        self.sharehistory[month as usize]
    }

    /// From previous month, in percent.
    pub fn get_share_change(&self) -> i32 {
        let mut month = self.lastmonthset;
        debug_assert!((0..12).contains(&month));
        let currentprice = self.sharehistory[month as usize];

        month -= 1;
        if month < 0 {
            month = 11;
        }
        let oldprice = self.sharehistory[month as usize];

        if oldprice == 0 {
            return 0;
        }

        (100.0_f32 * (currentprice - oldprice) as f32 / oldprice as f32) as i32
    }

    /// Index of recently set share price.
    pub fn get_share_last_month_set(&self) -> i32 {
        self.lastmonthset
    }

    pub fn update(&mut self) {}
}
