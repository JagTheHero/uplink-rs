//! Port of source/mainmenu/genericoptions_interface.h + source/mainmenu/genericoptions_interface.cpp

use uplink_engine::sound;

use crate::app::app::App;
use crate::app::opengl_interface::{
    button_assignbitmaps, button_click, button_draw, button_highlight, create_msgbox,
    imagebutton_draw, textbutton_draw,
};
use crate::ctx::Ctx;
use crate::eclipse::{Button, MOVE_STRAIGHTLINE};
use crate::interface::interfacescreen::{InterfaceScreen, ScreenSlot};
use crate::mainmenu::mainmenu::{MainMenu, MAINMENU_GENERICOPTIONS, MAINMENU_OPTIONS};
use crate::mainmenu::mainmenuscreen;

/// `sscanf(s, "%d")`: optional leading whitespace, optional sign, then digits.
fn scan_int(s: &str) -> Option<i32> {
    let t = s.trim_start();
    let end = t
        .char_indices()
        .find(|&(i, c)| !(c.is_ascii_digit() || (i == 0 && (c == '-' || c == '+'))))
        .map_or(t.len(), |(i, _)| i);
    t[..end].parse().ok()
}

/// `sscanf(name, "%s %d", unused, &index)` on a button name such as "generic_value 3".
fn button_index(name: &str) -> i32 {
    let after_word = name
        .trim_start()
        .trim_start_matches(|c: char| !c.is_whitespace());
    scan_int(after_word).unwrap_or(0)
}

/// The screen slot the main menu keeps its current screen in (`app->GetMainMenu()->GetMenuScreen()`).
fn mainmenu_screen_slot(g: &mut Ctx) -> &mut ScreenSlot {
    &mut g.app.mainmenu.screen
}

/// Generic options screen: picks out all options of one type (GAME, SOUND etc) and lets you edit them.
#[derive(Debug, Default)]
pub struct GenericOptionsInterface {
    /// C++ `optionTYPE[16]`: the option type shown (eg GAME, SOUND).
    pub option_type: String,
}

impl GenericOptionsInterface {
    pub fn new() -> Self {
        Self::default()
    }

    /// Static `ApplyClick` (MouseFn).
    pub fn apply_click(g: &mut Ctx, _button: &Button) {
        ScreenSlot::call(g, mainmenu_screen_slot, |s, g| {
            let thisint = s
                .as_any_mut()
                .downcast_mut::<GenericOptionsInterface>()
                .expect("GenericOptionsInterface::apply_click: current screen is not GenericOptionsInterface");

            let mut index = 0;
            while let Some(namecaption) = g
                .ecl
                .get_button(&format!("generic_option {index}"))
                .map(|b| b.caption.clone())
            {
                // Look up the next option button
                let valuecaption = g
                    .ecl
                    .get_button(&format!("generic_value {index}"))
                    .map(|b| b.caption.clone())
                    .expect("generic_value button missing for generic_option");

                // Extract the values
                let optionname = format!("{}_{}", thisint.option_type, namecaption);
                let newvalue = scan_int(&valuecaption).unwrap_or(0);

                // If they've changed, update them
                let changed = g
                    .app
                    .options
                    .get_option(&optionname)
                    .is_some_and(|o| o.value != newvalue);
                if changed {
                    thisint.change_option_value(g, &optionname, newvalue);
                }

                // Next button
                index += 1;
            }
        });
    }

    /// Static `ReturnToMainMenuClick` (MouseFn).
    pub fn return_to_main_menu_click(g: &mut Ctx, _button: &Button) {
        MainMenu::run_screen(g, MAINMENU_OPTIONS);
    }

    /// Static `ToggleBoxDraw` (DrawFn).
    pub fn toggle_box_draw(g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        if button.caption.starts_with('0') {
            imagebutton_draw(g, button, false, false);
        } else {
            imagebutton_draw(g, button, true, true);
        }
    }

    /// Static `ToggleBoxClick` (MouseFn).
    pub fn toggle_box_click(g: &mut Ctx, button: &Button) {
        // Get the button index
        let buttonindex = button_index(&button.name);

        // Grab the appropriate button
        let button2 = g
            .ecl
            .get_button_mut(&format!("generic_value {buttonindex}"))
            .expect("generic_value button missing");

        // Read the value from the button
        if button2.caption.starts_with('0') {
            button2.set_caption("1");
        } else {
            button2.set_caption("0");
        }
    }

    /// Static `ExitGameClick` (MouseFn).
    pub fn exit_game_click(g: &mut Ctx, _button: &Button) {
        App::close(g);
    }

    /// `SetOptionTYPE`: sets the option type and rebuilds the list of options of that type.
    pub fn set_option_type(&mut self, g: &mut Ctx, newtype: &str) {
        self.option_type = newtype.to_string();

        // Build a list of all options of this type
        let options: Vec<(String, String, bool, i32)> = g
            .app
            .options
            .get_all_options(Some(newtype), false)
            .into_iter()
            .map(|o| (o.name.clone(), o.tooltip.clone(), o.yesorno, o.value))
            .collect();

        // Create Title
        let screenw = g.app.options.get_option_value("graphics_screenwidth");
        let screenh = g.app.options.get_option_value("graphics_screenheight");

        let title = format!("{} options", self.option_type);
        g.ecl.register_button(screenw - 210, screenh - 40, 200, 15, &title, "Close this options screen", "generic_title");
        g.ecl.register_button_callback("generic_title", Self::return_to_main_menu_click);
        let n = options.len() as i32;
        g.ecl.register_movement("generic_title", screenw - 210, screenh - 100 - n * 20, 500, MOVE_STRAIGHTLINE, None);

        // Create Each option name and text edit box
        for (i, (name, tooltip, yesorno, value)) in options.iter().enumerate() {
            let i = i as i32;
            let name1 = format!("generic_option {i}");
            let name2 = format!("generic_value {i}");

            let optionname = name
                .split_once('_')
                .map(|(_, rest)| rest)
                .expect("option name has no '_' separator");

            let valuetext = value.to_string();

            g.ecl.register_button(screenw - 210, screenh - 40, 150, 15, optionname, tooltip, &name1);
            g.ecl.register_button_callbacks(
                &name1,
                Some(button_draw),
                Some(Self::toggle_box_click),
                Some(button_click),
                Some(button_highlight),
            );

            if *yesorno {
                g.ecl.register_button(screenw - 60, screenh - 40, 15, 15, &valuetext, "click to Enable or Disable this option", &name2);
                button_assignbitmaps(g, &name2, "mainmenu/optionno.tif", "mainmenu/optionyes.tif", "mainmenu/optionyes.tif");
                g.ecl.register_button_callbacks(
                    &name2,
                    Some(Self::toggle_box_draw),
                    Some(Self::toggle_box_click),
                    Some(button_click),
                    Some(button_highlight),
                );
            } else {
                g.ecl.register_button(screenw - 60, screenh - 40, 50, 15, &valuetext, "enter the new value here", &name2);
                g.ecl.register_button_callbacks(&name2, Some(textbutton_draw), None, Some(button_click), Some(button_highlight));
                g.ecl.make_button_editable(&name2);
            }

            let timems = (500.0_f32 * ((i + 1) as f32 / n as f32)) as i32;
            g.ecl.register_movement(&name1, screenw - 210, screenh - 90 - i * 20, timems, MOVE_STRAIGHTLINE, None);
            g.ecl.register_movement(&name2, screenw - 60, screenh - 90 - i * 20, timems, MOVE_STRAIGHTLINE, None);
        }

        // Create "Apply" button
        g.ecl.register_button(screenw - 210, screenh - 40, 145, 15, "Apply Changes", "Apply all changes", "generic_apply");
        g.ecl.register_button_callback("generic_apply", Self::apply_click);
    }

    /// `ChangeOptionValue`: stores the option and applies the side effects of the specific options.
    pub fn change_option_value(&mut self, g: &mut Ctx, option: &str, newvalue: i32) {
        g.app.options.set_option_value(option, newvalue);

        // Deal with specific options
        if option == "graphics_buttonanimations" {
            if newvalue == 0 {
                g.ecl.disable_animations();
            } else {
                g.ecl.enable_animations();
            }
        }

        if option == "graphics_fullscreen"
            || option == "graphics_screendepth"
            || option == "graphics_screenrefresh"
            || option == "graphics_safemode"
        {
            create_msgbox(g, "Shutdown", "Uplink must be restarted\nbefore this change can occur");
            g.ecl.register_button_callback("msgbox_close", Self::exit_game_click);
            g.ecl.register_button_callback("msgbox_title", Self::exit_game_click);
        }

        if option == "sound_musicenabled" {
            if newvalue == 0 {
                sound::playlist_stop();
            } else {
                sound::playlist_play("main");
            }
        }
    }
}

impl InterfaceScreen for GenericOptionsInterface {
    fn create(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            mainmenuscreen::create(g);

            let screenw = g.app.options.get_option_value("graphics_screenwidth");
            let screenh = g.app.options.get_option_value("graphics_screenheight");

            // Close button
            g.ecl.register_button(screenw - 60, screenh - 40, 50, 15, "Close", "Return to Options menu", "generic_return");
            g.ecl.register_button_callback("generic_return", Self::return_to_main_menu_click);
        }
    }

    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            g.ecl.remove_button("generic_return");
            g.ecl.remove_button("generic_title");

            let mut index = 0;
            while g.ecl.get_button(&format!("generic_option {index}")).is_some() {
                g.ecl.remove_button(&format!("generic_option {index}"));
                g.ecl.remove_button(&format!("generic_value {index}"));
                index += 1;
            }

            g.ecl.remove_button("generic_apply");
        }
    }

    fn update(&mut self, _g: &mut Ctx) {}
    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("generic_return").is_some()
    }
    fn screen_id(&self) -> i32 {
        MAINMENU_GENERICOPTIONS
    }
    crate::impl_as_any!();
}
