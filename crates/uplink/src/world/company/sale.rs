//! Port of source/world/company/sale.h + source/world/company/sale.cpp

use serde::{Deserialize, Serialize};

use crate::tosser::DArray;

pub const SALETYPE_NONE: i32 = 0;
pub const SALETYPE_SOFTWARE: i32 = 1;
pub const SALETYPE_HARDWARE: i32 = 2;

/// A sellable item: software, hardware or information.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Sale {
    pub title: String,
    pub saletype: i32,
    /// Defaults to SOFTWARETYPE_NONE (0).
    pub swhwtype: i32,

    pub versions: DArray<SaleVersion>,
}

impl Sale {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_title(&mut self, newtitle: &str) {
        self.title = newtitle.to_string();
    }

    pub fn set_sale_type(&mut self, new_type: i32) {
        self.saletype = new_type;
    }

    pub fn set_swhw_type(&mut self, new_swhw_type: i32) {
        self.swhwtype = new_swhw_type;
    }

    pub fn add_version(&mut self, details: &str, cost: i32, size: i32, data: i32) {
        let mut sv = SaleVersion::new();
        sv.set(details, cost, size, data);
        self.versions.put_data(sv);
    }

    /// Can return None.
    pub fn get_version(&self, index: i32) -> Option<&SaleVersion> {
        self.versions.get_data(index)
    }

    pub fn get_version_mut(&mut self, index: i32) -> Option<&mut SaleVersion> {
        self.versions.get_data_mut(index)
    }

    pub fn update(&mut self) {}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SaleVersion {
    pub details: Option<String>,

    pub cost: i32,
    pub size: i32,
    /// SW version or HW speed
    pub data: i32,
}

impl Default for SaleVersion {
    fn default() -> Self {
        SaleVersion {
            details: None,
            cost: -1,
            size: -1,
            data: -1,
        }
    }
}

impl SaleVersion {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, newdetails: &str, newcost: i32, newsize: i32, newdata: i32) {
        self.details = Some(newdetails.to_string());
        self.cost = newcost;
        self.size = newsize;
        self.data = newdata;
    }

    pub fn get_details(&self) -> Option<&str> {
        self.details.as_deref()
    }

    pub fn update(&mut self) {}
}
