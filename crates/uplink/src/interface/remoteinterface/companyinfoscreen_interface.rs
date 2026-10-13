//! Port of source/interface/remoteinterface/companyinfoscreen_interface.h + source/interface/remoteinterface/companyinfoscreen_interface.cpp

use crate::app::opengl_interface::{button_assignbitmaps, textbutton_draw};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::remoteinterface::remoteinterface::{RemoteInterface, SCREEN_COMPANYINFO};
use crate::interface::remoteinterface::remoteinterfacescreen::{draw_main_title, draw_sub_title};
use crate::objref::ScreenRef;
use crate::world::computer::computerscreen::genericscreen::GenericScreen;
use crate::world::person::Person;

/// Company information page: managing director and administrator contact details.
#[derive(Debug, Default)]
pub struct CompanyInfoScreenInterface {
    pub cs: Option<ScreenRef>,
}

/// `RunScreen(gs->nextpage)` when the generic screen has a next page (not -1).
fn run_next_page(g: &mut Ctx, nextpage: Option<i32>) {
    if let Some(nextpage) = nextpage {
        if nextpage != -1 {
            RemoteInterface::run_screen(g, nextpage);
        }
    }
}

/// `GetCompany(companyname)` for the computer behind the current remote screen, as the
/// (boss, administrator) names of that company.
fn screen_company_people(g: &Ctx) -> Option<(String, String)> {
    let computer = RemoteInterface::get_computer_screen(g)?.base().computer.clone();
    let companyname = g.world().get_computer(&computer)?.companyname.clone();
    g.world()
        .get_company(&companyname)
        .map(|c| (c.boss.clone(), c.administrator.clone()))
}

/// `if ( person && person->phonenumber != " " ) GiveLink ( person->phonenumber )`.
fn give_phone_link(g: &mut Ctx, person_name: &str) {
    let phone = g.world().get_person(person_name).map(|p| p.phonenumber.clone());
    if let Some(phone) = phone {
        if phone != " " {
            Person::give_link(g, "PLAYER", &phone);
        }
    }
}

impl CompanyInfoScreenInterface {
    /// `MdAddClick` (static MouseFn): adds the managing director's phone to the player's contacts.
    pub fn md_add_click(g: &mut Ctx, button: &Button) {
        if let Some((boss, _)) = screen_company_people(g) {
            give_phone_link(g, &boss);
        }

        g.ecl.remove_button(&button.name);
    }

    /// `AdminAddClick` (static MouseFn): adds the administrator's phone to the player's contacts.
    pub fn admin_add_click(g: &mut Ctx, button: &Button) {
        if let Some((_, admin)) = screen_company_people(g) {
            give_phone_link(g, &admin);
        }

        g.ecl.remove_button(&button.name);
    }

    /// `BackClick` (static MouseFn)
    pub fn back_click(g: &mut Ctx, _button: &Button) {
        let nextpage = RemoteInterface::get_computer_screen(g)
            .and_then(|s| s.as_generic_screen())
            .map(|gs| gs.nextpage);
        run_next_page(g, nextpage);
    }

    /// `GetComputerScreen` (typed accessor; the C++ casts `cs`).
    pub fn get_computer_screen<'a>(&self, g: &'a Ctx) -> Option<&'a GenericScreen> {
        let r = self.cs.as_ref()?;
        g.world().get_computer(&r.computer)?.get_computer_screen(r.index)?.as_generic_screen()
    }
}

impl InterfaceScreen for CompanyInfoScreenInterface {
    crate::impl_as_any!();

    /// `Create(ComputerScreen *newcs)` body (the `cs` is already set by `set_computer_screen`).
    fn create(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            return;
        }

        let Some(gs) = self.get_computer_screen(g).cloned() else { return };

        g.ecl.register_button(80, 60, 350, 25, &gs.base.maintitle, "", "companyscreen_maintitle");
        g.ecl.register_button_callbacks("companyscreen_maintitle", Some(draw_main_title), None, None, None);
        g.ecl.register_button(80, 80, 350, 20, &gs.base.subtitle, "", "companyscreen_subtitle");
        g.ecl.register_button_callbacks("companyscreen_subtitle", Some(draw_sub_title), None, None, None);

        let companyname = g
            .world()
            .get_computer(&gs.base.computer)
            .map(|c| c.companyname.clone())
            .unwrap_or_default();
        let Some((boss_name, admin_name)) = g
            .world()
            .get_company(&companyname)
            .map(|c| (c.boss.clone(), c.administrator.clone()))
        else {
            return;
        };

        // The people behind the company names (None when there is no such person)
        let boss = g.world().get_person(&boss_name).map(|p| p.phonenumber.clone());
        let admin = g.world().get_person(&admin_name).map(|p| p.phonenumber.clone());

        let (mdemail, adminemail, mdtel, admintel) = match (&boss, &admin) {
            (Some(boss_phone), Some(admin_phone)) => (
                format!("{}@{}.net", boss_name, companyname),
                format!("{}@{}.net", admin_name, companyname),
                format!("tel. {}", boss_phone),
                format!("tel. {}", admin_phone),
            ),
            _ => (
                "Unlisted".to_string(),
                "Unlisted".to_string(),
                "Unlisted".to_string(),
                "Unlisted".to_string(),
            ),
        };

        g.ecl.register_button(40, 120, 300, 25, "Contact details", " ", "companyscreen_contactdetails");
        g.ecl.register_button_callbacks("companyscreen_contactdetails", Some(draw_main_title), None, None, None);

        g.ecl.register_button(50, 150, 150, 15, "MANAGING DIRECTOR", " ", "companyscreen_mdtitle");
        g.ecl.register_button(50, 170, 150, 15, &boss_name, " ", "companyscreen_md");
        g.ecl.register_button(50, 190, 250, 15, &mdemail, " ", "companyscreen_mdemail");
        g.ecl.register_button(50, 210, 150, 15, &mdtel, " ", "companyscreen_mdtel");

        if let Some(boss_phone) = &boss {
            if !g.player().has_link(boss_phone) {
                g.ecl.register_button(35, 211, 13, 13, " ", "Add this phone number to your contact list", "companyscreen_mdadd");
                button_assignbitmaps(g, "companyscreen_mdadd", "add.tif", "add_h.tif", "add_c.tif");
                g.ecl.register_button_callback("companyscreen_mdadd", Self::md_add_click);
            }
        }

        g.ecl.register_button(300, 150, 150, 15, "ADMINISTRATOR", " ", "companyscreen_admintitle");
        g.ecl.register_button(300, 170, 150, 15, &admin_name, " ", "companyscreen_admin");
        g.ecl.register_button(300, 190, 250, 15, &adminemail, " ", "companyscreen_adminemail");
        g.ecl.register_button(300, 210, 150, 15, &admintel, " ", "companyscreen_admintel");

        if let Some(admin_phone) = &admin {
            if !g.player().has_link(admin_phone) {
                g.ecl.register_button(285, 211, 13, 13, " ", "Add this phone number to your contact list", "companyscreen_adminadd");
                button_assignbitmaps(g, "companyscreen_adminadd", "add.tif", "add_h.tif", "add_c.tif");
                g.ecl.register_button_callback("companyscreen_adminadd", Self::admin_add_click);
            }
        }

        g.ecl.register_button_callbacks("companyscreen_mdtitle", Some(textbutton_draw), None, None, None);
        g.ecl.register_button_callbacks("companyscreen_md", Some(textbutton_draw), None, None, None);
        g.ecl.register_button_callbacks("companyscreen_mdemail", Some(textbutton_draw), None, None, None);
        g.ecl.register_button_callbacks("companyscreen_mdtel", Some(textbutton_draw), None, None, None);

        g.ecl.register_button_callbacks("companyscreen_admintitle", Some(textbutton_draw), None, None, None);
        g.ecl.register_button_callbacks("companyscreen_admin", Some(textbutton_draw), None, None, None);
        g.ecl.register_button_callbacks("companyscreen_adminemail", Some(textbutton_draw), None, None, None);
        g.ecl.register_button_callbacks("companyscreen_admintel", Some(textbutton_draw), None, None, None);

        if gs.nextpage != -1 {
            g.ecl.register_button(300, 250, 60, 15, "Back", "Return to the main menu", "companyscreen_back");
            g.ecl.register_button_callback("companyscreen_back", Self::back_click);
        }
    }

    fn remove(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            return;
        }

        for name in [
            "companyscreen_maintitle",
            "companyscreen_subtitle",
            "companyscreen_contactdetails",
            "companyscreen_mdtitle",
            "companyscreen_md",
            "companyscreen_mdemail",
            "companyscreen_mdtel",
            "companyscreen_mdadd",
            "companyscreen_admintitle",
            "companyscreen_admin",
            "companyscreen_adminemail",
            "companyscreen_admintel",
            "companyscreen_adminadd",
            "companyscreen_back",
        ] {
            g.ecl.remove_button(name);
        }
    }

    fn update(&mut self, _g: &mut Ctx) {}

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("companyscreen_maintitle").is_some()
    }

    fn escape_key_pressed(&mut self, g: &mut Ctx) -> bool {
        let nextpage = self.get_computer_screen(g).map(|gs| gs.nextpage);
        run_next_page(g, nextpage);
        true
    }

    fn screen_id(&self) -> i32 {
        SCREEN_COMPANYINFO
    }

    fn set_computer_screen(&mut self, cs: ScreenRef) {
        self.cs = Some(cs);
    }

    fn computer_screen_ref(&self) -> Option<&ScreenRef> {
        self.cs.as_ref()
    }
}
