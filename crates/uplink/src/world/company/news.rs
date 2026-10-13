//! Port of source/world/company/news.h + source/world/company/news.cpp

use serde::{Deserialize, Serialize};

use crate::world::date::Date;

pub const NEWS_TYPE_NONE: i32 = 0;
/// Data1 = IP
pub const NEWS_TYPE_HACKED: i32 = 1;
/// Data1 = name of person
pub const NEWS_TYPE_ARREST: i32 = 2;
/// Data1 = IP
pub const NEWS_TYPE_DELETED: i32 = 3;
/// Data1 = IP
pub const NEWS_TYPE_STOLEN: i32 = 4;
/// Data1 = IP
pub const NEWS_TYPE_COMPUTERSHUTDOWN: i32 = 5;
/// Data1 = IP. All databanks formatted first.
pub const NEWS_TYPE_COMPUTERDESTROYED: i32 = 6;

/// A single news story posted onto one of the news servers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct News {
    pub details: Option<String>,

    pub date: Date,
    pub headline: String,

    pub newstype: i32,
    pub data1: String,
    pub data2: String,
}

impl Default for News {
    fn default() -> Self {
        News {
            details: None,
            date: Date::default(),
            headline: " ".to_string(),
            newstype: NEWS_TYPE_NONE,
            data1: " ".to_string(),
            data2: " ".to_string(),
        }
    }
}

impl News {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_date(&mut self, newdate: &Date) {
        // Date::SetDate(Date*) copies only the six time fields (not updateme/previousupdate).
        self.date.set_date_from(newdate);
    }

    pub fn set_headline(&mut self, newheadline: &str) {
        self.headline = newheadline.to_string();
    }

    pub fn set_details(&mut self, newdetails: &str) {
        self.details = Some(newdetails.to_string());
    }

    pub fn get_details(&self) -> Option<&str> {
        self.details.as_deref()
    }

    /// `None` data strings become " " (as in C++).
    pub fn set_data(&mut self, newstype: i32, newdata1: Option<&str>, newdata2: Option<&str>) {
        self.newstype = newstype;
        self.data1 = newdata1.unwrap_or(" ").to_string();
        self.data2 = newdata2.unwrap_or(" ").to_string();
    }
}
