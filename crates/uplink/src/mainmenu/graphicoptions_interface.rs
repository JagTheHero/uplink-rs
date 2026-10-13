//! Port of source/mainmenu/graphicoptions_interface.h + source/mainmenu/graphicoptions_interface.cpp

use uplink_engine::{gl, text};

use crate::app::opengl_interface::{
    border_draw, button_assignbitmaps, button_click, button_draw, button_highlight, create_msgbox,
    imagebutton_draw, set_colour, textbutton_draw,
};
use crate::ctx::Ctx;
use crate::eclipse::{Button, MOVE_STRAIGHTLINE};
use crate::interface::interfacescreen::{InterfaceScreen, ScreenSlot};
use crate::mainmenu::mainmenu::{MainMenu, MAINMENU_GRAPHICOPTIONS, MAINMENU_OPTIONS};
use crate::mainmenu::mainmenuscreen;

/// Graphic options screen: screen resolution settings etc.
#[derive(Debug, Default)]
pub struct GraphicOptionsInterface {
    /// C++ `optionTYPE[16]`: the option type shown.
    pub option_type: String,
    /// C++ statics `newScreenWidth`, `newScreenHeight`, `newColourDepth`, `newRefresh`,
    /// `screenSettingsChanged`. Now per-screen state.
    pub new_screen_width: i32,
    pub new_screen_height: i32,
    pub new_colour_depth: i32,
    pub new_refresh: i32,
    pub screen_settings_changed: bool,
}

/// `sscanf(s, "%d")`: optional leading whitespace, optional sign, then digits. `None` if no number.
fn scan_int(token: &str) -> Option<i32> {
    let s = token.trim_start();
    let mut end = 0;
    for (i, c) in s.char_indices() {
        let sign = i == 0 && (c == '-' || c == '+');
        if c.is_ascii_digit() || sign {
            end = i + c.len_utf8();
        } else {
            break;
        }
    }
    s[..end].parse().ok()
}

/// `sscanf(name, "%s %d %d %d", unused, &a, &b, &c)` on a button name: the three numbers after the
/// first word. Missing or bad numbers read as 0.
fn name_numbers(name: &str) -> (i32, i32, i32) {
    let mut nums = name.split_whitespace().skip(1).map(|t| scan_int(t).unwrap_or(0));
    let a = nums.next().unwrap_or(0);
    let b = nums.next().unwrap_or(0);
    let c = nums.next().unwrap_or(0);
    (a, b, c)
}

/// The main menu's current screen slot (C `app->GetMainMenu()->GetMenuScreen()`).
fn main_menu_slot(g: &mut Ctx) -> &mut ScreenSlot {
    &mut g.app.mainmenu.screen
}

/// Runs `f` on the graphic options screen (taken out of its slot while it runs). `None` if the
/// current screen is not the graphic options screen.
fn with_screen<R>(g: &mut Ctx, f: impl FnOnce(&mut GraphicOptionsInterface, &mut Ctx) -> R) -> Option<R> {
    ScreenSlot::call(g, main_menu_slot, |s, g| {
        let this = s.as_any_mut().downcast_mut::<GraphicOptionsInterface>()?;
        Some(f(this, g))
    })
    .flatten()
}

impl GraphicOptionsInterface {
    pub fn new() -> Self {
        Self::default()
    }

    /// Body of the static `ApplyClick`, run on the screen itself.
    fn apply(&mut self, g: &mut Ctx) {
        let mut index = 0;
        let mut shutdown_required = false;

        // Each option is a "graphic_option N" name button with a "graphic_value N" value button
        loop {
            let name1 = format!("graphic_option {index}");
            let Some(namebutton) = g.ecl.get_button(&name1) else { break };
            let optionlabel = namebutton.caption.clone();

            let name2 = format!("graphic_value {index}");
            let Some(valuebutton) = g.ecl.get_button(&name2) else {
                debug_assert!(false, "ApplyClick: value button missing: {name2}");
                break;
            };
            let valuecaption = valuebutton.caption.clone();

            // Extract the values
            let optionname = format!("{}_{}", self.option_type, optionlabel);
            let newvalue = scan_int(&valuecaption).unwrap_or(0);

            // If they've changed, update them
            let changed = g.app.options.get_option(&optionname).is_some_and(|o| o.value != newvalue);
            if changed && self.change_option_value(g, &optionname, newvalue) {
                shutdown_required = true;
            }

            index += 1;
        }

        if self.screen_settings_changed {
            g.app.options.request_shutdown_change("graphics_screenwidth", self.new_screen_width);
            g.app.options.request_shutdown_change("graphics_screenheight", self.new_screen_height);
            g.app.options.request_shutdown_change("graphics_screendepth", self.new_colour_depth);
            g.app.options.request_shutdown_change("graphics_screenrefresh", self.new_refresh);
            shutdown_required = true;
        }

        if shutdown_required {
            create_msgbox(g, "Shutdown", "Uplink must be restarted\nbefore this change can occur");
            g.ecl.register_button_callback("msgbox_close", Self::exit_game_click);
            g.ecl.register_button_callback("msgbox_title", Self::exit_game_click);
        }
    }

    /// Static `ApplyClick` (MouseFn).
    pub fn apply_click(g: &mut Ctx, _button: &Button) {
        let done = with_screen(g, |this, g| this.apply(g));
        debug_assert!(done.is_some(), "ApplyClick: graphic options screen is not the current screen");
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
        let (buttonindex, _, _) = name_numbers(&button.name);

        // Grab the appropriate button
        let name2 = format!("graphic_value {buttonindex}");
        let Some(button2) = g.ecl.get_button_mut(&name2) else {
            debug_assert!(false, "ToggleBoxClick: value button missing: {name2}");
            return;
        };

        // Read the value from the button
        if button2.caption.starts_with('0') {
            button2.set_caption("1");
        } else {
            button2.set_caption("0");
        }
    }

    /// Static `ScreenOptionDraw` (DrawFn).
    pub fn screen_option_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        let (ty, value_a, value_b) = name_numbers(&button.name);

        let (new_width, new_height, new_depth, new_refresh) = match g.app.mainmenu.screen.get::<GraphicOptionsInterface>() {
            Some(s) => (s.new_screen_width, s.new_screen_height, s.new_colour_depth, s.new_refresh),
            None => (0, 0, 0, 0),
        };

        let current_value = (ty == 1 && value_a == new_width && value_b == new_height)
            || (ty == 2 && value_a == new_depth)
            || (ty == 3 && value_a == new_refresh);

        if highlighted || clicked || current_value {
            gl::begin(gl::QUADS);
            set_colour(g, "PanelHighlightA");
            gl::vertex2i(button.x, button.y + button.height);
            set_colour(g, "PanelHighlightB");
            gl::vertex2i(button.x, button.y);
            set_colour(g, "PanelHighlightA");
            gl::vertex2i(button.x + button.width, button.y);
            set_colour(g, "PanelHighlightB");
            gl::vertex2i(button.x + button.width, button.y + button.height);
            gl::end();
        } else {
            gl::begin(gl::QUADS);
            set_colour(g, "PanelBackgroundA");
            gl::vertex2i(button.x, button.y + button.height);
            set_colour(g, "PanelBackgroundB");
            gl::vertex2i(button.x, button.y);
            set_colour(g, "PanelBackgroundA");
            gl::vertex2i(button.x + button.width, button.y);
            set_colour(g, "PanelBackgroundB");
            gl::vertex2i(button.x + button.width, button.y + button.height);
            gl::end();
        }

        if clicked || current_value {
            set_colour(g, "PanelHighlightBorder");
            border_draw(button);
        } else {
            set_colour(g, "PanelBorder");
            border_draw(button);
        }

        let xpos = (button.x + button.width / 2) - (text::text_width_default(&button.caption) / 2);
        let ypos = (button.y + button.height / 2) + 2;

        set_colour(g, "DefaultText");
        text::draw_text_default(xpos, ypos, &button.caption);
    }

    /// Static `ScreenOptionClick` (MouseFn).
    pub fn screen_option_click(g: &mut Ctx, button: &Button) {
        // Change new button
        let (ty, value_a, value_b) = name_numbers(&button.name);

        if let Some(this) = g.app.mainmenu.screen.get_mut::<GraphicOptionsInterface>() {
            if ty == 1 {
                this.new_screen_width = value_a;
                this.new_screen_height = value_b;
            } else if ty == 2 {
                this.new_colour_depth = value_a;
            } else if ty == 3 {
                this.new_refresh = value_a;
            }

            this.screen_settings_changed = true;
        }

        let screen_w = g.app.options.get_option_value("graphics_screenwidth");
        let screen_h = g.app.options.get_option_value("graphics_screenheight");
        let start_y = screen_h - 170;
        g.ecl.dirty_rectangle(0, start_y, screen_w, screen_h - start_y);
    }

    /// Static `ExitGameClick` (MouseFn).
    pub fn exit_game_click(g: &mut Ctx, _button: &Button) {
        crate::app::app::App::close(g);
    }

    /// `SetOptionTYPE`: sets the option type and builds the list of options of that type.
    pub fn set_option_type(&mut self, g: &mut Ctx, newtype: &str) {
        self.option_type = newtype.to_string();

        //
        // Build a list of all options of this type
        //

        let options: Vec<crate::options::options::UplinkOption> =
            g.app.options.get_all_options(Some(newtype), false).into_iter().cloned().collect();

        //
        // Create Title
        //

        let screenw = g.app.options.get_option_value("graphics_screenwidth");
        let screenh = g.app.options.get_option_value("graphics_screenheight");

        let title = format!("{} options", self.option_type);
        g.ecl.register_button(screenw - 210, screenh - 40, 200, 15, &title, "Close this options screen", "graphic_title");
        g.ecl.register_button_callback("graphic_title", Self::return_to_main_menu_click);
        g.ecl.register_movement(
            "graphic_title",
            screenw - 210,
            screenh - 100 - options.len() as i32 * 20,
            500,
            MOVE_STRAIGHTLINE,
            None,
        );

        //
        // Create Each option name and text edit box
        //

        let count = options.len() as i32;
        for (i, option) in options.iter().enumerate() {
            let i = i as i32;
            let name1 = format!("graphic_option {i}");
            let name2 = format!("graphic_value {i}");

            // strchr(option->name, '_') + 1
            let optionname = option.name.split_once('_').map_or(option.name.as_str(), |(_, rest)| rest);

            let value = format!("{}", option.value);

            g.ecl.register_button(screenw - 210, screenh - 40, 150, 15, optionname, &option.tooltip, &name1);
            g.ecl.register_button_callbacks(
                &name1,
                Some(button_draw),
                Some(Self::toggle_box_click),
                Some(button_click),
                Some(button_highlight),
            );

            if option.yesorno {
                g.ecl.register_button(screenw - 60, screenh - 40, 15, 15, &value, "click to Enable or Disable this option", &name2);
                button_assignbitmaps(g, &name2, "mainmenu/optionno.tif", "mainmenu/optionyes.tif", "mainmenu/optionyes.tif");
                g.ecl.register_button_callbacks(
                    &name2,
                    Some(Self::toggle_box_draw),
                    Some(Self::toggle_box_click),
                    Some(button_click),
                    Some(button_highlight),
                );
            } else {
                g.ecl.register_button(screenw - 60, screenh - 40, 50, 15, &value, "enter the new value here", &name2);
                g.ecl.register_button_callbacks(&name2, Some(textbutton_draw), None, Some(button_click), Some(button_highlight));
                g.ecl.make_button_editable(&name2);
            }

            let timems = (500.0_f32 * ((i + 1) as f32 / count as f32)) as i32;
            g.ecl.register_movement(&name1, screenw - 210, screenh - 90 - i * 20, timems, MOVE_STRAIGHTLINE, None);
            g.ecl.register_movement(&name2, screenw - 60, screenh - 90 - i * 20, timems, MOVE_STRAIGHTLINE, None);
        }

        //
        // Create "Apply" button
        //

        g.ecl.register_button(screenw - 210, screenh - 40, 145, 15, "Apply Changes", "Apply all changes", "graphic_apply");
        g.ecl.register_button_callback("graphic_apply", Self::apply_click);
    }

    /// `ChangeOptionValue`. Returns true if a shutdown (restart) is required.
    pub fn change_option_value(&mut self, g: &mut Ctx, option: &str, newvalue: i32) -> bool {
        //
        // Deal with specific options
        //

        if option == "graphics_buttonanimations" {
            g.app.options.set_option_value(option, newvalue);

            if newvalue == 0 {
                g.ecl.disable_animations();
            } else {
                g.ecl.enable_animations();
            }

            false
        } else if matches!(
            option,
            "graphics_fullscreen"
                | "graphics_screendepth"
                | "graphics_screenrefresh"
                | "graphics_safemode"
                | "graphics_screenwidth"
                | "graphics_screenheight"
        ) {
            g.app.options.request_shutdown_change(option, newvalue);

            true
        } else {
            g.app.options.set_option_value(option, newvalue);
            false
        }
    }
}

impl InterfaceScreen for GraphicOptionsInterface {
    fn create(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            mainmenuscreen::create(g);

            let screenw = g.app.options.get_option_value("graphics_screenwidth");
            let screenh = g.app.options.get_option_value("graphics_screenheight");

            //
            // Close button
            //

            g.ecl.register_button(screenw - 60, screenh - 40, 50, 15, "Close", "Return to Options menu", "graphic_return");
            g.ecl.register_button_callback("graphic_return", GraphicOptionsInterface::return_to_main_menu_click);

            //
            // Set me up baby
            //

            self.set_option_type(g, "graphics");

            //
            // Additional resolution change buttons
            //

            let start_y = screenh - 170;

            g.ecl.register_button(50, start_y, 100, 15, "RESOLUTION", "Select your screen resolution", "graphic_screenrestitle");
            g.ecl.register_button(50, start_y + 30, 100, 15, "640x480", "Choose this resolution", "graphic 1 640 480");
            g.ecl.register_button(50, start_y + 50, 100, 15, "800x600", "Choose this resolution", "graphic 1 800 600");
            g.ecl.register_button(50, start_y + 70, 100, 15, "1024x768", "Choose this resolution", "graphic 1 1024 768");
            g.ecl.register_button(50, start_y + 90, 100, 15, "1280x1024", "Choose this resolution", "graphic 1 1280 1024");
            g.ecl.register_button(50, start_y + 110, 100, 15, "1600x1200", "Choose this resolution", "graphic 1 1600 1200");
            for name in ["graphic 1 640 480", "graphic 1 800 600", "graphic 1 1024 768", "graphic 1 1280 1024", "graphic 1 1600 1200"] {
                g.ecl.register_button_callbacks(
                    name,
                    Some(Self::screen_option_draw),
                    Some(Self::screen_option_click),
                    Some(button_click),
                    Some(button_highlight),
                );
            }

            g.ecl.register_button(170, start_y, 100, 15, "COLOUR DEPTH", "Select your colour depth", "graphic_colourdepthtitle");
            g.ecl.register_button(170, start_y + 30, 100, 15, "8 Bit", "Choose this colour depth", "graphic 2 8 0");
            g.ecl.register_button(170, start_y + 50, 100, 15, "16 Bit", "Choose this colour depth", "graphic 2 16 0");
            g.ecl.register_button(170, start_y + 70, 100, 15, "32 Bit", "Choose this colour depth", "graphic 2 32 0");
            for name in ["graphic 2 8 0", "graphic 2 16 0", "graphic 2 32 0"] {
                g.ecl.register_button_callbacks(
                    name,
                    Some(Self::screen_option_draw),
                    Some(Self::screen_option_click),
                    Some(button_click),
                    Some(button_highlight),
                );
            }

            g.ecl.register_button(290, start_y, 100, 15, "REFRESH", "Select your refresh rate", "graphic_refreshtitle");
            g.ecl.register_button(290, start_y + 30, 100, 15, "60 Hz", "Choose this refresh rate", "graphic 3 60 0");
            g.ecl.register_button(290, start_y + 50, 100, 15, "70 Hz", "Choose this refresh rate", "graphic 3 70 0");
            g.ecl.register_button(290, start_y + 70, 100, 15, "75 Hz", "Choose this refresh rate", "graphic 3 75 0");
            g.ecl.register_button(290, start_y + 90, 100, 15, "85 Hz", "Choose this refresh rate", "graphic 3 85 0");
            g.ecl.register_button(290, start_y + 110, 100, 15, "100 Hz", "Choose this refresh rate", "graphic 3 100 0");
            for name in ["graphic 3 60 0", "graphic 3 70 0", "graphic 3 75 0", "graphic 3 85 0", "graphic 3 100 0"] {
                g.ecl.register_button_callbacks(
                    name,
                    Some(Self::screen_option_draw),
                    Some(Self::screen_option_click),
                    Some(button_click),
                    Some(button_highlight),
                );
            }

            self.new_screen_width = g.app.options.get_option_value("graphics_screenwidth");
            self.new_screen_height = g.app.options.get_option_value("graphics_screenheight");
            self.new_colour_depth = g.app.options.get_option_value("graphics_screendepth");
            self.new_refresh = g.app.options.get_option_value("graphics_screenrefresh");
            self.screen_settings_changed = false;
        }
    }

    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            g.ecl.remove_button("graphic_return");
            g.ecl.remove_button("graphic_title");

            let mut index = 0;
            while g.ecl.get_button(&format!("graphic_option {index}")).is_some() {
                g.ecl.remove_button(&format!("graphic_option {index}"));
                g.ecl.remove_button(&format!("graphic_value {index}"));
                index += 1;
            }

            g.ecl.remove_button("graphic_apply");

            g.ecl.remove_button("graphic_screenrestitle");
            g.ecl.remove_button("graphic 1 640 480");
            g.ecl.remove_button("graphic 1 800 600");
            g.ecl.remove_button("graphic 1 1024 768");
            g.ecl.remove_button("graphic 1 1280 1024");
            g.ecl.remove_button("graphic 1 1600 1200");

            g.ecl.remove_button("graphic_colourdepthtitle");
            g.ecl.remove_button("graphic 2 8 0");
            g.ecl.remove_button("graphic 2 16 0");
            g.ecl.remove_button("graphic 2 32 0");

            g.ecl.remove_button("graphic_refreshtitle");
            g.ecl.remove_button("graphic 3 60 0");
            g.ecl.remove_button("graphic 3 70 0");
            g.ecl.remove_button("graphic 3 75 0");
            g.ecl.remove_button("graphic 3 85 0");
            g.ecl.remove_button("graphic 3 100 0");
        }
    }

    fn update(&mut self, _g: &mut Ctx) {}

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("graphic_return").is_some()
    }
    fn screen_id(&self) -> i32 {
        MAINMENU_GRAPHICOPTIONS
    }
    crate::impl_as_any!();
}
