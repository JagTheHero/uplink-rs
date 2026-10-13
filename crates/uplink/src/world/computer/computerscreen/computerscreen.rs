//! Port of source/world/computer/computerscreen/computerscreen.h + source/world/computer/computerscreen/computerscreen.cpp

use serde::{Deserialize, Serialize};

use crate::world::computer::computer::Computer;
use crate::world::computer::computerscreen::bbsscreen::BBSScreen;
use crate::world::computer::computerscreen::cypherscreen::CypherScreen;
use crate::world::computer::computerscreen::dialogscreen::DialogScreen;
use crate::world::computer::computerscreen::disconnectedscreen::DisconnectedScreen;
use crate::world::computer::computerscreen::genericscreen::GenericScreen;
use crate::world::computer::computerscreen::highsecurityscreen::HighSecurityScreen;
use crate::world::computer::computerscreen::linksscreen::LinksScreen;
use crate::world::computer::computerscreen::logscreen::LogScreen;
use crate::world::computer::computerscreen::menuscreen::MenuScreen;
use crate::world::computer::computerscreen::messagescreen::MessageScreen;
use crate::world::computer::computerscreen::passwordscreen::PasswordScreen;
use crate::world::computer::computerscreen::shareslistscreen::SharesListScreen;
use crate::world::computer::computerscreen::useridscreen::UserIDScreen;
use crate::world::world::World;

/// The fields every computer screen has (C++ `ComputerScreen` base class).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ComputerScreenBase {
    pub maintitle: String,
    pub subtitle: String,
    /// Name of the owning computer (key into `World::computers`).
    pub computer: String,
}

impl Default for ComputerScreenBase {
    fn default() -> Self {
        ComputerScreenBase {
            maintitle: " ".to_string(),
            subtitle: " ".to_string(),
            computer: " ".to_string(),
        }
    }
}

impl ComputerScreenBase {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_main_title(&mut self, newmaintitle: &str) {
        self.maintitle = newmaintitle.to_string();
    }

    pub fn set_sub_title(&mut self, newsubtitle: &str) {
        self.subtitle = newsubtitle.to_string();
    }

    pub fn set_computer(&mut self, newcomputer: &str) {
        self.computer = newcomputer.to_string();
    }

    /// `ComputerScreen::GetComputer` (read-only world lookup of `self.computer`).
    pub fn get_computer<'a>(&self, world: &'a World) -> Option<&'a Computer> {
        world.get_computer(&self.computer)
    }
}

/// Every concrete computer screen; the variant name is the C++ class name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComputerScreen {
    GenericScreen(GenericScreen),
    MessageScreen(MessageScreen),
    PasswordScreen(PasswordScreen),
    MenuScreen(MenuScreen),
    DialogScreen(DialogScreen),
    UserIDScreen(UserIDScreen),
    LogScreen(LogScreen),
    BBSScreen(BBSScreen),
    LinksScreen(LinksScreen),
    SharesListScreen(SharesListScreen),
    CypherScreen(CypherScreen),
    HighSecurityScreen(HighSecurityScreen),
    DisconnectedScreen(DisconnectedScreen),
}

impl Default for ComputerScreen {
    fn default() -> Self {
        ComputerScreen::GenericScreen(GenericScreen::default())
    }
}

macro_rules! screen_accessors {
    ($(($variant:ident, $as_fn:ident, $as_mut_fn:ident)),* $(,)?) => {
        impl ComputerScreen {
            /// The common `ComputerScreen` fields (`maintitle`, `subtitle`, `computer`).
            pub fn base(&self) -> &ComputerScreenBase {
                match self { $(ComputerScreen::$variant(s) => &s.base,)* }
            }

            pub fn base_mut(&mut self) -> &mut ComputerScreenBase {
                match self { $(ComputerScreen::$variant(s) => &mut s.base,)* }
            }

            /// The C++ class name of this screen (variant name).
            pub fn screen_type_name(&self) -> &'static str {
                match self { $(ComputerScreen::$variant(_) => stringify!($variant),)* }
            }

            $(
                pub fn $as_fn(&self) -> Option<&$variant> {
                    match self { ComputerScreen::$variant(s) => Some(s), _ => None }
                }

                pub fn $as_mut_fn(&mut self) -> Option<&mut $variant> {
                    match self { ComputerScreen::$variant(s) => Some(s), _ => None }
                }
            )*
        }
    };
}

screen_accessors! {
    (GenericScreen, as_generic_screen, as_generic_screen_mut),
    (MessageScreen, as_message_screen, as_message_screen_mut),
    (PasswordScreen, as_password_screen, as_password_screen_mut),
    (MenuScreen, as_menu_screen, as_menu_screen_mut),
    (DialogScreen, as_dialog_screen, as_dialog_screen_mut),
    (UserIDScreen, as_user_id_screen, as_user_id_screen_mut),
    (LogScreen, as_log_screen, as_log_screen_mut),
    (BBSScreen, as_bbs_screen, as_bbs_screen_mut),
    (LinksScreen, as_links_screen, as_links_screen_mut),
    (SharesListScreen, as_shares_list_screen, as_shares_list_screen_mut),
    (CypherScreen, as_cypher_screen, as_cypher_screen_mut),
    (HighSecurityScreen, as_high_security_screen, as_high_security_screen_mut),
    (DisconnectedScreen, as_disconnected_screen, as_disconnected_screen_mut),
}

impl ComputerScreen {
    /// `ComputerScreen::Update` (virtual; no subclass overrides it).
    pub fn update(&mut self) {}
}
