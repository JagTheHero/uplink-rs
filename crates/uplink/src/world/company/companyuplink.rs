//! Port of source/world/company/companyuplink.h + source/world/company/companyuplink.cpp

use serde::{Deserialize, Serialize};

use crate::ctx::Ctx;
use crate::world::company::mission::Mission;
use crate::world::company::news::News;
use crate::world::company::sale::Sale;

/// The extra fields of the company "Uplink" (PORTING.md §3.5): lives in `Company::uplink`.
/// The methods that need the game date act on the world company named "Uplink".
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CompanyUplink {
    pub missions: Vec<Mission>,
    /// Hardware
    pub hw_sales: Vec<Sale>,
    /// Software
    pub sw_sales: Vec<Sale>,
    pub news: Vec<News>,
}

/// The extra data of the world company named "Uplink" (C++ `this` for the methods above).
fn uplink_mut(g: &mut Ctx) -> &mut CompanyUplink {
    g.world_mut()
        .get_company_mut("Uplink")
        .and_then(|c| c.uplink.as_deref_mut())
        .expect("CompanyUplink: the world has no Uplink company")
}

impl CompanyUplink {
    /// Extra data only; the Company named "Uplink" is built by `Company::new_uplink()`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Stamps the mission with the world date if it has none, then inserts it in date order.
    /// C++ inserts before the first mission that `mission.createdate.After(..)` (keep that test).
    pub fn create_mission(g: &mut Ctx, mut mission: Mission) {
        if mission.createdate.get_year() == 1000 {
            mission.set_create_date(&g.world().date);
        }

        // Insert the mission in date order
        let uplink = uplink_mut(g);
        match uplink
            .missions
            .iter()
            .position(|m| mission.createdate.after(&m.createdate))
        {
            Some(i) => uplink.missions.insert(i, mission),
            // If we get here, put the mission at the end of the list
            None => uplink.missions.push(mission),
        }
    }

    /// C++ overload `CreateMission(employer, payment, description, details, fulldetails)`.
    /// Builds the Mission, stamps the date, inserts in date order.
    /// NOTE: this overload tests `mission.createdate.Before(..)` (the other uses After); keep each as is.
    pub fn create_mission_with(
        g: &mut Ctx,
        employer: &str,
        payment: i32,
        description: &str,
        details: &str,
        fulldetails: &str,
    ) {
        let mut mission = Mission::new();

        // C++ Mission::SetEmployer asserts that the employer company exists
        debug_assert!(
            g.world().get_company(employer).is_some(),
            "CreateMission: employer company does not exist"
        );
        mission.set_employer(employer);
        mission.set_payment(payment, -1);
        mission.set_description(description);
        mission.set_details(details);
        mission.set_full_details(fulldetails);

        if mission.createdate.get_year() == 1000 {
            mission.set_create_date(&g.world().date);
        }

        // Insert the mission in date order
        let uplink = uplink_mut(g);
        match uplink
            .missions
            .iter()
            .position(|m| mission.createdate.before(&m.createdate))
        {
            Some(i) => uplink.missions.insert(i, mission),
            // If we get here, put the mission at the end of the list
            None => uplink.missions.push(mission),
        }
    }

    pub fn create_hw_sale(&mut self, newsale: Sale) {
        self.hw_sales.push(newsale);
    }

    pub fn create_sw_sale(&mut self, newsale: Sale) {
        self.sw_sales.push(newsale);
    }

    /// Stamps the news item with the world date if it has none, then puts it at the start of the list.
    pub fn create_news(g: &mut Ctx, mut newnews: News) {
        if newnews.date.get_year() == 1000 {
            newnews.set_date(&g.world().date);
        }

        uplink_mut(g).news.insert(0, newnews);
    }

    pub fn get_mission(&self, index: i32) -> Option<&Mission> {
        if index < 0 {
            return None;
        }
        self.missions.get(index as usize)
    }

    pub fn get_mission_mut(&mut self, index: i32) -> Option<&mut Mission> {
        if index < 0 {
            return None;
        }
        self.missions.get_mut(index as usize)
    }

    pub fn get_sw_sale(&self, index: i32) -> Option<&Sale> {
        if index < 0 {
            return None;
        }
        self.sw_sales.get(index as usize)
    }

    pub fn get_hw_sale(&self, index: i32) -> Option<&Sale> {
        if index < 0 {
            return None;
        }
        self.hw_sales.get(index as usize)
    }

    pub fn get_news(&self, index: i32) -> Option<&News> {
        if index < 0 {
            return None;
        }
        self.news.get(index as usize)
    }

    pub fn get_news_mut(&mut self, index: i32) -> Option<&mut News> {
        if index < 0 {
            return None;
        }
        self.news.get_mut(index as usize)
    }

    pub fn update(&mut self) {}
}
