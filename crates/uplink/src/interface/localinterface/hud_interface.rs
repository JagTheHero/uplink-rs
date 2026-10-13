//! Port of source/interface/localinterface/hud_interface.h + source/interface/localinterface/hud_interface.cpp
//! The HUD: worldmap, main toolbar, speed buttons, email/mission shortcuts and the upgrade buttons.
//! `HUDInterface` owns the sub-screens `wmi` (worldmap), `si` (software menu) and `gw` (gateway).

use std::rc::Rc;

use uplink_engine::gl;
use uplink_engine::image::Image;
use uplink_engine::sound;

use crate::app::app::App;
use crate::app::globals::cheatmodes_enabled;
use crate::app::opengl_interface::{
    button_assignbitmaps, button_click, button_draw, button_highlight, create_msgbox, imagebutton_draw,
    remove_msgbox,
};
use crate::ctx::Ctx;
use crate::eclipse::{Button, MouseFn, MOVE_STRAIGHTLINE};
use crate::game::game::{GAMESPEED_NORMAL, GAMESPEED_PAUSED};
use crate::impl_as_any;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::localinterface::gateway_interface::GatewayInterface;
use crate::interface::localinterface::localinterface::{
    LocalInterface, SCREEN_ANALYSER, SCREEN_CHEATS, SCREEN_EMAIL, SCREEN_EVTQUEUE, SCREEN_FINANCE, SCREEN_HUD,
    SCREEN_HW, SCREEN_IRC, SCREEN_LAN, SCREEN_MEMORY, SCREEN_MISSION, SCREEN_NONE, SCREEN_SENDMAIL,
    SCREEN_STATUS,
};
use crate::interface::localinterface::sw_interface::SWInterface;
use crate::interface::localinterface::worldmap::worldmap_interface::{WorldMapInterface, WORLDMAP_SMALL};
use crate::interface::remoteinterface::remoteinterface::RemoteInterface;
use crate::mainmenu::mainmenu::{MainMenu, MAINMENU_LOGIN};
use crate::vanbakel::Vanbakel;
use crate::world::computer::gateway::{
    HUDUPGRADE_CONNECTIONANALYSIS, HUDUPGRADE_IRCCLIENT, HUDUPGRADE_LANVIEW,
};
use crate::world::connection::Connection;

/// C++ `struct HUDInterfaceUpgrade`. `number` is a `char` id (u8 here); the strings are literal
/// file names and labels, so they are `&'static str` to allow the static table below.
// PORT: `char` ids as u8, literal `char*` as `&'static str`.
#[derive(Clone, Copy)]
pub struct HUDInterfaceUpgrade {
    pub number: u8,
    pub name: &'static str,
    pub tooltip: &'static str,
    pub button_name: &'static str,
    pub button_filename: &'static str,
    pub button_filename_h: &'static str,
    pub button_filename_c: &'static str,
    pub callback: Option<MouseFn>,
}

/// C++ `HUDInterface::hudUpgrades[8]`. Entries 0..3 are the three upgrades; the rest are empty
/// (number 0, no callback), as in the C++ zero-initialised array.
// `number` values: 2 = HUDUPGRADE_CONNECTIONANALYSIS, 4 = HUDUPGRADE_IRCCLIENT, 8 = HUDUPGRADE_LANVIEW.
pub const HUD_UPGRADES: [HUDInterfaceUpgrade; 8] = [
    HUDInterfaceUpgrade {
        number: 2,
        name: "Analyser",
        tooltip: "Show the connection analyser",
        button_name: "hud_analyser",
        button_filename: "hud/analyser.tif",
        button_filename_h: "hud/analyser_h.tif",
        button_filename_c: "hud/analyser_c.tif",
        callback: Some(HUDInterface::analyser_click),
    },
    HUDInterfaceUpgrade {
        number: 4,
        name: "IRCClient",
        tooltip: "Show the IRC Client",
        button_name: "hud_ircclient",
        button_filename: "hud/irc.tif",
        button_filename_h: "hud/irc_h.tif",
        button_filename_c: "hud/irc_c.tif",
        callback: Some(HUDInterface::irc_click),
    },
    HUDInterfaceUpgrade {
        number: 8,
        name: "LANView",
        tooltip: "Show the LAN Viewer",
        button_name: "hud_lanview",
        button_filename: "hud/lan.tif",
        button_filename_h: "hud/lan_h.tif",
        button_filename_c: "hud/lan_c.tif",
        callback: Some(HUDInterface::lan_click),
    },
    HUDInterfaceUpgrade {
        number: 0,
        name: "",
        tooltip: "",
        button_name: "",
        button_filename: "",
        button_filename_h: "",
        button_filename_c: "",
        callback: None,
    },
    HUDInterfaceUpgrade {
        number: 0,
        name: "",
        tooltip: "",
        button_name: "",
        button_filename: "",
        button_filename_h: "",
        button_filename_c: "",
        callback: None,
    },
    HUDInterfaceUpgrade {
        number: 0,
        name: "",
        tooltip: "",
        button_name: "",
        button_filename: "",
        button_filename_h: "",
        button_filename_c: "",
        callback: None,
    },
    HUDInterfaceUpgrade {
        number: 0,
        name: "",
        tooltip: "",
        button_name: "",
        button_filename: "",
        button_filename_h: "",
        button_filename_c: "",
        callback: None,
    },
    HUDInterfaceUpgrade {
        number: 0,
        name: "",
        tooltip: "",
        button_name: "",
        button_filename: "",
        button_filename_h: "",
        button_filename_c: "",
        callback: None,
    },
];

#[derive(Default)]
pub struct HUDInterface {
    pub wmi: WorldMapInterface,
    pub si: SWInterface,
    pub gw: GatewayInterface,

    /// `char *previoushighlight`: name of the toolbar button currently highlighted.
    pub previoushighlight: Option<String>,
    /// `Image *previousimage`: the image swapped out by the current highlight.
    pub previousimage: Option<Rc<Image>>,
    /// `char visibleUpgrades[8]`: upgrade id shown in each toolbar slot (0 = free).
    pub visible_upgrades: [u8; 8],
}

/// `sscanf(name, "<prefix> %d", &index)`: the integer after `prefix` and whitespace, or -1 if none.
fn scan_index(name: &str, prefix: &str) -> i32 {
    name.strip_prefix(prefix)
        .and_then(|rest| {
            let rest = rest.trim_start();
            let end = rest
                .char_indices()
                .find(|&(i, c)| !(c.is_ascii_digit() || (i == 0 && c == '-')))
                .map_or(rest.len(), |(i, _)| i);
            rest[..end].parse::<i32>().ok()
        })
        .unwrap_or(-1)
}

/// `ValidIndex` on a list of `len` entries.
fn valid_index(len: usize, index: i32) -> bool {
    index >= 0 && (index as usize) < len
}

/// `GetHUD()->previoushighlight && strcmp(previoushighlight, name) == 0`.
fn is_previous_highlight(g: &Ctx, name: &str) -> bool {
    HUDInterface::get_hud(g).previoushighlight.as_deref() == Some(name)
}

/// Shared body of the toolbar buttons that open a screen: clicking the highlighted button closes it.
fn toggle_screen_click(g: &mut Ctx, button: &Button, screen: i32, index: i32) {
    if is_previous_highlight(g, &button.name) {
        LocalInterface::run_screen(g, SCREEN_NONE, -1);
    } else {
        LocalInterface::run_screen(g, screen, index);
    }
}

/// `button->SetTooltip(tooltip); button_highlight(button);`. The callback's button is a snapshot,
/// so the real button gets the tooltip and the highlight runs on a copy that carries it.
fn highlight_with_tooltip(g: &mut Ctx, button: &Button, tooltip: &str) {
    if let Some(real) = g.ecl.get_button_mut(&button.name) {
        real.set_tooltip(tooltip);
    }
    let mut snapshot = button.clone();
    snapshot.set_tooltip(tooltip);
    button_highlight(g, &snapshot);
}

impl HUDInterface {
    /// C++ `HUDInterface::MainMenuClick` (static button callback).
    pub fn main_menu_click(g: &mut Ctx, _button: &Button) {
        sound::play_sound("sounds/close.wav", None, false);

        // Close any connections
        Connection::disconnect(g, "PLAYER");
        Connection::reset(g, "PLAYER");

        RemoteInterface::run_new_location(g);
        RemoteInterface::run_screen(g, 0);

        // Save game and log off
        let handle = g.player().agent().handle.clone();
        App::save_game(g, &handle);
        g.game.set_game_speed(GAMESPEED_PAUSED);

        let screenwidth = g.app.options.get_option_value("graphics_screenwidth");
        let screenheight = g.app.options.get_option_value("graphics_screenheight");
        g.ecl.reset(screenwidth, screenheight);
        MainMenu::run_screen(g, MAINMENU_LOGIN);
    }

    /// C++ `HUDInterface::SoftwareClick` (static button callback).
    pub fn software_click(g: &mut Ctx, _button: &Button) {
        crate::interface::localinterface::sw_interface::SWInterface::toggle_software_menu(g);
    }

    /// C++ `HUDInterface::HardwareClick` (static button callback).
    pub fn hardware_click(g: &mut Ctx, button: &Button) {
        toggle_screen_click(g, button, SCREEN_HW, -1);
    }

    /// C++ `HUDInterface::MemoryClick` (static button callback).
    pub fn memory_click(g: &mut Ctx, button: &Button) {
        toggle_screen_click(g, button, SCREEN_MEMORY, -1);
    }

    /// C++ `HUDInterface::StatusClick` (static button callback).
    pub fn status_click(g: &mut Ctx, button: &Button) {
        toggle_screen_click(g, button, SCREEN_STATUS, -1);
    }

    /// C++ `HUDInterface::FinanceClick` (static button callback).
    pub fn finance_click(g: &mut Ctx, button: &Button) {
        toggle_screen_click(g, button, SCREEN_FINANCE, -1);
    }

    /// C++ `HUDInterface::SendMailClick` (static button callback).
    pub fn send_mail_click(g: &mut Ctx, button: &Button) {
        toggle_screen_click(g, button, SCREEN_SENDMAIL, -1);
    }

    /// C++ `HUDInterface::AnalyserClick` (static button callback).
    pub fn analyser_click(g: &mut Ctx, button: &Button) {
        toggle_screen_click(g, button, SCREEN_ANALYSER, -1);
    }

    /// C++ `HUDInterface::IRCClick` (static button callback).
    pub fn irc_click(g: &mut Ctx, button: &Button) {
        toggle_screen_click(g, button, SCREEN_IRC, -1);
    }

    /// C++ `HUDInterface::LANClick` (static button callback).
    pub fn lan_click(g: &mut Ctx, button: &Button) {
        toggle_screen_click(g, button, SCREEN_LAN, -1);
    }

    /// C++ `HUDInterface::ToolbarButtonDraw` (static draw callback).
    pub fn toolbar_button_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        gl::color4f(1.0, 1.0, 1.0, 1.0);
        imagebutton_draw(g, button, highlighted, clicked);
    }

    /// C++ `HUDInterface::EmailHighlight` (static mouse-move callback).
    pub fn email_highlight(g: &mut Ctx, button: &Button) {
        let index = scan_index(&button.name, "hud_message");
        let tooltip = match usize::try_from(index).ok().and_then(|i| g.player().messages.get(i)) {
            Some(message) => format!("'{}', From '{}'", message.get_subject(), message.from),
            None => return,
        };
        highlight_with_tooltip(g, button, &tooltip);
    }

    /// C++ `HUDInterface::EmailClick` (static button callback).
    pub fn email_click(g: &mut Ctx, button: &Button) {
        let index = scan_index(&button.name, "hud_message");
        if !valid_index(g.player().messages.len(), index) {
            println!("HUDInterface WARNING: Tried to view an email that didn't exist");
            g.ecl.remove_button(&button.name);
            return;
        }
        toggle_screen_click(g, button, SCREEN_EMAIL, index);
    }

    /// C++ `HUDInterface::MissionHighlight` (static mouse-move callback).
    pub fn mission_highlight(g: &mut Ctx, button: &Button) {
        let index = scan_index(&button.name, "hud_mission");
        let tooltip = match usize::try_from(index).ok().and_then(|i| g.player().agent().missions.get(i)) {
            Some(mission) => mission.description.clone(),
            None => return,
        };
        highlight_with_tooltip(g, button, &tooltip);
    }

    /// C++ `HUDInterface::MissionClick` (static button callback).
    pub fn mission_click(g: &mut Ctx, button: &Button) {
        let index = scan_index(&button.name, "hud_mission");
        if !valid_index(g.player().agent().missions.len(), index) {
            println!("HUDInterface WARNING: Tried to view a mission that didn't exist");
            g.ecl.remove_button(&button.name);
            return;
        }
        toggle_screen_click(g, button, SCREEN_MISSION, index);
    }

    /// C++ `HUDInterface::SpeedButtonDraw` (static draw callback).
    pub fn speed_button_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        let index = scan_index(&button.name, "hud_speed");
        if index == g.game.game_speed() {
            imagebutton_draw(g, button, highlighted, true);
        } else {
            imagebutton_draw(g, button, highlighted, clicked);
        }
    }

    /// C++ `HUDInterface::SpeedButtonClick` (static button callback).
    pub fn speed_button_click(g: &mut Ctx, button: &Button) {
        let index = scan_index(&button.name, "hud_speed");
        g.game.set_game_speed(index);
        dirty_speed_buttons(g);
    }

    /// C++ `HUDInterface::PauseButtonClick` (static button callback).
    pub fn pause_button_click(g: &mut Ctx, _button: &Button) {
        g.game.set_game_speed(GAMESPEED_PAUSED);

        // Create a message and tie the close button to Play
        create_msgbox(g, "Message", "Uplink is paused\nClick Close to continue");
        g.ecl.register_button_callback("msgbox_close", Self::un_pause_click);
        g.ecl.register_button_callback("msgbox_title", Self::un_pause_click);

        dirty_speed_buttons(g);
    }

    /// C++ `HUDInterface::UnPauseClick` (static button callback).
    pub fn un_pause_click(g: &mut Ctx, _button: &Button) {
        remove_msgbox(g);
        g.game.set_game_speed(GAMESPEED_NORMAL);

        dirty_speed_buttons(g);
    }

    /// C++ `HUDInterface::HighlightToolbarButton` (static).
    pub fn highlight_toolbar_button(g: &mut Ctx, bname: &str) {
        let Some(button) = g.ecl.get_button_mut(bname) else {
            println!("HUDInterface::HighlightToolbarButton, invalid button : {bname}");
            return;
        };

        // Swap the highlighted/unhighlighted/clicked graphics around
        let previous_image = button.image_highlighted.clone();
        let new_standard = button.image_clicked.clone();
        let new_highlighted = button.image_clicked.clone();
        let new_clicked = button.image_standard.clone();
        button.image_standard = new_standard;
        button.image_highlighted = new_highlighted;
        button.image_clicked = new_clicked;

        g.ecl.dirty_button(bname);

        // Remember this button was highlighted
        let hud = Self::get_hud_mut(g);
        hud.previousimage = previous_image;
        hud.previoushighlight = Some(bname.to_string());
    }

    /// C++ `HUDInterface::UnHighlightToolbarButton` (static).
    pub fn un_highlight_toolbar_button(g: &mut Ctx) {
        let Some(name) = Self::get_hud(g).previoushighlight.clone() else {
            return;
        };

        let previous_image = Self::get_hud(g).previousimage.clone();
        if let Some(button) = g.ecl.get_button_mut(&name) {
            // Swap the highlighted/unhighlighted/clicked graphics around
            let new_standard = button.image_clicked.clone();
            let new_highlighted = previous_image;
            let new_clicked = button.image_standard.clone();
            button.image_standard = new_standard;
            button.image_highlighted = new_highlighted;
            button.image_clicked = new_clicked;

            Self::get_hud_mut(g).previousimage = None;
            g.ecl.dirty_button(&name);
        }

        // No button is now highlighted
        Self::get_hud_mut(g).previoushighlight = None;
    }

    /// C++ `HUDInterface::MoveSelecter`: highlights the toolbar button for a screen.
    pub fn move_selecter(g: &mut Ctx, screen_id: i32, screen_index: i32) {
        // screenindex is only used in some cases when a particular screenID might not be enough to
        // locate the button - for example the emails at the bottom of the screen (all the same
        // screenID, but different buttons based on index)
        Self::un_highlight_toolbar_button(g);

        match screen_id {
            SCREEN_HW => Self::highlight_toolbar_button(g, "hud_hardware"),
            SCREEN_MEMORY => Self::highlight_toolbar_button(g, "hud_memory"),
            SCREEN_STATUS => Self::highlight_toolbar_button(g, "hud_status"),
            SCREEN_FINANCE => Self::highlight_toolbar_button(g, "hud_finance"),
            SCREEN_SENDMAIL => Self::highlight_toolbar_button(g, "hud_sendmail"),
            SCREEN_ANALYSER => Self::highlight_toolbar_button(g, "hud_analyser"),
            SCREEN_IRC => Self::highlight_toolbar_button(g, "hud_ircclient"),
            SCREEN_LAN => Self::highlight_toolbar_button(g, "hud_lanview"),
            SCREEN_CHEATS | SCREEN_EVTQUEUE | SCREEN_NONE => {}
            SCREEN_EMAIL => Self::highlight_toolbar_button(g, &format!("hud_message {screen_index}")),
            SCREEN_MISSION => Self::highlight_toolbar_button(g, &format!("hud_mission {screen_index}")),
            _ => panic!("Unknown screenID"),
        }
    }

    /// C++ `HUDInterface::IsUpgradeVisible`
    pub fn is_upgrade_visible(&self, g: &Ctx, upgrade: u8) -> bool {
        let mut visible = 0;

        if self.visible_upgrades.contains(&upgrade) {
            visible += 1;
        }

        if let Some(the_upgrade) = self.get_upgrade(upgrade) {
            if g.ecl.get_button(the_upgrade.button_name).is_some() {
                visible += 1;
            }
        }

        visible == 2
    }

    /// C++ `HUDInterface::AddUpgrade`
    pub fn add_upgrade(&mut self, g: &mut Ctx, upgrade: u8) {
        if self.is_upgrade_visible(g, upgrade) {
            return;
        }

        let Some(inserted_at) = self.visible_upgrades.iter().position(|&u| u == 0) else {
            return;
        };
        self.visible_upgrades[inserted_at] = upgrade;

        let Some(the_upgrade) = self.get_upgrade(upgrade) else {
            return;
        };

        let x_pos = 195 + 27 * inserted_at as i32;
        let screenh = g.app.options.get_option_value("graphics_screenheight");

        g.ecl.register_button(60, screenh - 42, 24, 24, the_upgrade.name, the_upgrade.tooltip, the_upgrade.button_name);
        button_assignbitmaps(
            g,
            the_upgrade.button_name,
            the_upgrade.button_filename,
            the_upgrade.button_filename_h,
            the_upgrade.button_filename_c,
        );
        g.ecl.register_button_callbacks(
            the_upgrade.button_name,
            Some(Self::toolbar_button_draw),
            the_upgrade.callback,
            Some(button_click),
            Some(button_highlight),
        );
        g.ecl.register_movement(the_upgrade.button_name, x_pos, screenh - 42, 700, MOVE_STRAIGHTLINE, None);
    }

    /// C++ `HUDInterface::RemoveUpgrade` (aborts in the original: "never needed").
    pub fn remove_upgrade(&mut self, _g: &mut Ctx, _upgrade: u8) {
        panic!("You said this was never needed!")
    }

    /// C++ `HUDInterface::GetUpgrade`: finds the table entry for `upgrade`.
    pub fn get_upgrade(&self, upgrade: u8) -> Option<&'static HUDInterfaceUpgrade> {
        HUD_UPGRADES.iter().find(|u| u.number == upgrade)
    }

    /// C++ `HUDInterface::GetHUD()`: the HUD owned by the local interface.
    /// Panics while the HUD is taken out of its slot (during its own methods).
    pub fn get_hud(g: &Ctx) -> &HUDInterface {
        g.interface().li.hud.get::<HUDInterface>().expect("HUD screen is busy or missing")
    }

    /// Mutable form of [`HUDInterface::get_hud`].
    pub fn get_hud_mut(g: &mut Ctx) -> &mut HUDInterface {
        g.interface_mut().li.hud.get_mut::<HUDInterface>().expect("HUD screen is busy or missing")
    }
}

/// Redraws the four (five with cheats) speed buttons.
fn dirty_speed_buttons(g: &mut Ctx) {
    for i in 0..4 {
        g.ecl.dirty_button(&format!("hud_speed {i}"));
    }
    if cheatmodes_enabled() {
        g.ecl.dirty_button("hud_speed 4");
    }
}

impl InterfaceScreen for HUDInterface {
    /// C++ `HUDInterface::Create`
    fn create(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            return;
        }

        let screenh = g.app.options.get_option_value("graphics_screenheight");

        // Main world map
        self.wmi.create_size(g, WORLDMAP_SMALL);

        // Software button
        g.ecl.register_button(3, screenh - 70, 53, 53, "", "Run a software application", "hud_software");
        button_assignbitmaps(g, "hud_software", "hud/software.tif", "hud/software_h.tif", "hud/software_c.tif");
        g.ecl.register_button_callback("hud_software", Self::software_click);

        // Button bar
        g.ecl.register_button(60, screenh - 42, 24, 24, "Hardware", "Show the hardware installed in your gateway", "hud_hardware");
        button_assignbitmaps(g, "hud_hardware", "hud/hardware.tif", "hud/hardware_h.tif", "hud/hardware_c.tif");
        g.ecl.register_button_callbacks(
            "hud_hardware",
            Some(Self::toolbar_button_draw),
            Some(Self::hardware_click),
            Some(button_click),
            Some(button_highlight),
        );

        g.ecl.register_button(60, screenh - 42, 24, 24, "Memory", "Show your memory banks", "hud_memory");
        button_assignbitmaps(g, "hud_memory", "hud/memory.tif", "hud/memory_h.tif", "hud/memory_c.tif");
        g.ecl.register_button_callbacks(
            "hud_memory",
            Some(Self::toolbar_button_draw),
            Some(Self::memory_click),
            Some(button_click),
            Some(button_highlight),
        );

        g.ecl.register_button(60, screenh - 42, 24, 24, "Status", "Show your status in the World", "hud_status");
        button_assignbitmaps(g, "hud_status", "hud/status.tif", "hud/status_h.tif", "hud/status_c.tif");
        g.ecl.register_button_callbacks(
            "hud_status",
            Some(Self::toolbar_button_draw),
            Some(Self::status_click),
            Some(button_click),
            Some(button_highlight),
        );

        g.ecl.register_button(60, screenh - 42, 24, 24, "Finances", "Show your financial situation", "hud_finance");
        button_assignbitmaps(g, "hud_finance", "hud/finance.tif", "hud/finance_h.tif", "hud/finance_c.tif");
        g.ecl.register_button_callbacks(
            "hud_finance",
            Some(Self::toolbar_button_draw),
            Some(Self::finance_click),
            Some(button_click),
            Some(button_highlight),
        );

        g.ecl.register_button(60, screenh - 42, 24, 24, "New Mail", "Send an email", "hud_sendmail");
        button_assignbitmaps(g, "hud_sendmail", "hud/newmail.tif", "hud/newmail_h.tif", "hud/newmail_c.tif");
        g.ecl.register_button_callbacks(
            "hud_sendmail",
            Some(Self::toolbar_button_draw),
            Some(Self::send_mail_click),
            Some(button_click),
            Some(button_highlight),
        );

        g.ecl.register_movement("hud_hardware", 60, screenh - 42, 200, MOVE_STRAIGHTLINE, None);
        g.ecl.register_movement("hud_memory", 87, screenh - 42, 300, MOVE_STRAIGHTLINE, None);
        g.ecl.register_movement("hud_status", 114, screenh - 42, 400, MOVE_STRAIGHTLINE, None);
        g.ecl.register_movement("hud_finance", 141, screenh - 42, 500, MOVE_STRAIGHTLINE, None);
        g.ecl.register_movement("hud_sendmail", 168, screenh - 42, 600, MOVE_STRAIGHTLINE, None);

        // Status bar
        g.ecl.register_button(3, 4, 13, 13, "", "Close any connections and log off", "hud_mainmenu");
        button_assignbitmaps(g, "hud_mainmenu", "close.tif", "close_h.tif", "close_c.tif");
        g.ecl.register_button_callback("hud_mainmenu", Self::main_menu_click);

        g.ecl.register_button(0, 0, 3, 3, "", "Close any connections and log off", "hud_topleftclick");
        g.ecl.register_button_callbacks(
            "hud_topleftclick",
            None,
            Some(Self::main_menu_click),
            Some(button_click),
            Some(button_highlight),
        );

        g.ecl.register_button(20, 3, 147, 15, "", "Shows the current Date and Time", "hud_date");
        g.ecl.register_button(170, 3, 90, 15, "", "Shows your current Uplink location", "hud_location");

        g.ecl.register_button_callbacks("hud_date", Some(button_draw), None, None, None);
        g.ecl.register_button_callbacks("hud_location", Some(button_draw), None, None, None);

        // Speed buttons
        g.ecl.register_button(263, 3, 15, 15, "", "Run at PAUSED speed", "hud_speed 0");
        button_assignbitmaps(g, "hud_speed 0", "hud/speed0.tif", "hud/speed0_h.tif", "hud/speed0_c.tif");
        g.ecl.register_button_callbacks(
            "hud_speed 0",
            Some(Self::speed_button_draw),
            Some(Self::pause_button_click),
            Some(button_click),
            Some(button_highlight),
        );

        g.ecl.register_button(280, 3, 15, 15, "", "Run at NORMAL speed", "hud_speed 1");
        button_assignbitmaps(g, "hud_speed 1", "hud/speed1.tif", "hud/speed1_h.tif", "hud/speed1_c.tif");
        g.ecl.register_button_callbacks(
            "hud_speed 1",
            Some(Self::speed_button_draw),
            Some(Self::speed_button_click),
            Some(button_click),
            Some(button_highlight),
        );

        g.ecl.register_button(297, 3, 15, 15, "", "Run at FAST speed", "hud_speed 2");
        button_assignbitmaps(g, "hud_speed 2", "hud/speed2.tif", "hud/speed2_h.tif", "hud/speed2_c.tif");
        g.ecl.register_button_callbacks(
            "hud_speed 2",
            Some(Self::speed_button_draw),
            Some(Self::speed_button_click),
            Some(button_click),
            Some(button_highlight),
        );

        g.ecl.register_button(314, 3, 15, 15, "", "Run at VERYFAST speed", "hud_speed 3");
        button_assignbitmaps(g, "hud_speed 3", "hud/speed3.tif", "hud/speed3_h.tif", "hud/speed3_c.tif");
        g.ecl.register_button_callbacks(
            "hud_speed 3",
            Some(Self::speed_button_draw),
            Some(Self::speed_button_click),
            Some(button_click),
            Some(button_highlight),
        );

        if cheatmodes_enabled() {
            g.ecl.register_button(331, 3, 15, 15, "", "Run at OH-MY-GOD speed (DEBUG ONLY)", "hud_speed 4");
            button_assignbitmaps(g, "hud_speed 4", "hud/speed3.tif", "hud/speed3_h.tif", "hud/speed3_c.tif");
            g.ecl.register_button_callbacks(
                "hud_speed 4",
                Some(Self::speed_button_draw),
                Some(Self::speed_button_click),
                Some(button_click),
                Some(button_highlight),
            );
        }

        // Task manager
        Vanbakel::create_interface(g, 335, 15);
    }

    /// C++ `HUDInterface::Update`
    fn update(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            return;
        }

        let screenw = g.app.options.get_option_value("graphics_screenwidth");
        let screenh = g.app.options.get_option_value("graphics_screenheight");

        // Update the location, date/time
        let ip = g.player().get_remote_host(g.world()).map(|h| h.ip.clone()).unwrap_or_default();
        let date = g.world().date.get_long_string();
        if let Some(button) = g.ecl.get_button_mut("hud_location") {
            button.set_caption(&ip);
        }
        if let Some(button) = g.ecl.get_button_mut("hud_date") {
            button.set_caption(&date);
        }

        // Update the messages on display
        let message_count = g.player().messages.len() as i32;
        for mi in 0..message_count {
            let bname = format!("hud_message {mi}");
            if g.ecl.get_button(&bname).is_none() {
                g.ecl.register_button(222, screenh - 41, 24, 24, "", "Read this message", &bname);
                button_assignbitmaps(g, &bname, "hud/email.tif", "hud/email_h.tif", "hud/email_c.tif");
                g.ecl.register_button_callbacks(
                    &bname,
                    Some(imagebutton_draw),
                    Some(Self::email_click),
                    Some(button_click),
                    Some(Self::email_highlight),
                );
                g.ecl.register_movement(&bname, screenw - 30 * (mi + 1), screenh - 41, 1000, MOVE_STRAIGHTLINE, None);
                sound::play_sound("sounds/newmail.wav", None, false);
            }
        }

        // Removes any message buttons that shouldn't be there (ie if they have been deleted)
        let mut remove_message_index = message_count;
        let mut bname = format!("hud_message {remove_message_index}");
        while g.ecl.get_button(&bname).is_some() {
            g.ecl.remove_button(&bname);
            remove_message_index += 1;
            bname = format!("hud_message {remove_message_index}");
        }

        // Update the missions on display
        let base_x = screenw - 30 * (message_count + 1);
        let mission_count = g.player().agent().missions.len() as i32;
        for msi in 0..mission_count {
            let bname = format!("hud_mission {msi}");
            let target_x = base_x - 30 * msi;

            if g.ecl.get_button(&bname).is_none() {
                g.ecl.register_button(222, screenh - 41, 24, 24, "", "View this mission", &bname);
                button_assignbitmaps(g, &bname, "hud/mission.tif", "hud/mission_h.tif", "hud/mission_c.tif");
                g.ecl.register_button_callbacks(
                    &bname,
                    Some(imagebutton_draw),
                    Some(Self::mission_click),
                    Some(button_click),
                    Some(Self::mission_highlight),
                );
                g.ecl.register_movement(&bname, target_x, screenh - 41, 1000, MOVE_STRAIGHTLINE, None);
                sound::play_sound("sounds/newmail.wav", None, false);
            } else if g.ecl.get_button(&bname).is_some_and(|b| b.x != target_x)
                && g.ecl.is_animation_active(&bname) == -1
            {
                g.ecl.register_movement(&bname, target_x, screenh - 41, 300, MOVE_STRAIGHTLINE, None);
            }
        }

        // Remove any mission buttons that shouldn't be here
        let mut remove_mission_index = mission_count;
        let mut bname = format!("hud_mission {remove_mission_index}");
        while g.ecl.get_button(&bname).is_some() {
            g.ecl.remove_button(&bname);
            remove_mission_index += 1;
            bname = format!("hud_mission {remove_mission_index}");
        }

        // Add / remove any upgrade buttons
        for upgrade in [HUDUPGRADE_CONNECTIONANALYSIS, HUDUPGRADE_IRCCLIENT, HUDUPGRADE_LANVIEW] {
            if g.player().gateway.has_hud_upgrade(upgrade) && !self.is_upgrade_visible(g, upgrade as u8) {
                self.add_upgrade(g, upgrade as u8);
            }
        }

        // Update the world map and software menu
        self.wmi.update(g);
        self.si.update(g);

        // Update the task manager
        Vanbakel::update_interface(g);
    }

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("hud_software").is_some()
    }

    fn screen_id(&self) -> i32 {
        SCREEN_HUD
    }

    impl_as_any!();
}
