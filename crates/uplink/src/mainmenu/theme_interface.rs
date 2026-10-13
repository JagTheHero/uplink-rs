//! Port of source/mainmenu/theme_interface.h + source/mainmenu/theme_interface.cpp

use std::cell::Cell;

use uplink_engine::{archive, gl, text};

use crate::app::miscutils::{list_subdirs, lower_case_string};
use crate::app::opengl_interface::{
    border_draw, button_assignbitmap, button_assignbitmaps, button_click, button_highlight,
    set_colour, textbutton_draw,
};
use crate::ctx::Ctx;
use crate::eclipse::{Button, MOVE_STRAIGHTLINE};
use crate::interface::interfacescreen::{InterfaceScreen, ScreenSlot};
use crate::mainmenu::mainmenu::{MainMenu, MAINMENU_OPTIONS, MAINMENU_THEME};
use crate::mainmenu::mainmenuscreen;
use crate::tosser::DArray;

thread_local! {
    /// C++ static `ThemeInterface::currentSelect` (initially -1). It is a static in the original, so
    /// it survives the screen being rebuilt by `RunScreen` (Apply / Restore Default). Kept here for
    /// the same reason, rather than on the screen.
    static CURRENT_SELECT: Cell<i32> = const { Cell::new(-1) };
}

fn current_select() -> i32 {
    CURRENT_SELECT.with(|c| c.get())
}

fn set_current_select(index: i32) {
    CURRENT_SELECT.with(|c| c.set(index));
}

/// `sscanf(s, "%d")`: optional leading whitespace, optional sign, then digits.
fn scan_int(s: &str) -> Option<i32> {
    let t = s.trim_start();
    let end = t
        .char_indices()
        .find(|&(i, c)| !(c.is_ascii_digit() || (i == 0 && (c == '-' || c == '+'))))
        .map_or(t.len(), |(i, _)| i);
    t[..end].parse().ok()
}

/// `sscanf(name, "%s %d", unused, &index)` on a button name such as "theme 3".
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

/// Theme interface: preview and select from the themes available.
#[derive(Debug, Default)]
pub struct ThemeInterface {
    /// C++ static `themes` (NULL until listed). Now per-screen state.
    pub themes: Option<DArray<String>>,
}

impl ThemeInterface {
    pub fn new() -> Self {
        Self::default()
    }

    /// Static `ApplyClick` (MouseFn).
    pub fn apply_click(g: &mut Ctx, _button: &Button) {
        ScreenSlot::call(g, mainmenu_screen_slot, |s, g| {
            let Some(theme) = s.as_any_mut().downcast_mut::<ThemeInterface>() else {
                return;
            };
            let themes = theme.list_available_themes(g);
            if let Some(newthemename) = themes.get_data(current_select()).cloned() {
                g.app.options.set_theme_name(&newthemename);
            }
            let screenw = g.app.options.get_option_value("graphics_screenwidth");
            let screenh = g.app.options.get_option_value("graphics_screenheight");
            g.ecl.reset(screenw, screenh);
        });

        MainMenu::run_screen(g, MAINMENU_THEME);
    }

    /// Static `DefaultClick` (MouseFn).
    pub fn default_click(g: &mut Ctx, _button: &Button) {
        g.app.options.set_theme_name("graphics");

        let screenw = g.app.options.get_option_value("graphics_screenwidth");
        let screenh = g.app.options.get_option_value("graphics_screenheight");
        g.ecl.reset(screenw, screenh);

        MainMenu::run_screen(g, MAINMENU_THEME);
        set_current_select(-1);
    }

    /// Static `ReturnToMainMenuClick` (MouseFn).
    pub fn return_to_main_menu_click(g: &mut Ctx, _button: &Button) {
        MainMenu::run_screen(g, MAINMENU_OPTIONS);
    }

    /// Static `ThemeNameDraw` (DrawFn).
    pub fn theme_name_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        let index = button_index(&button.name);

        if highlighted || clicked || index == current_select() {
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

        if clicked || index == current_select() {
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

    /// Static `ThemeNameClick` (MouseFn).
    pub fn theme_name_click(g: &mut Ctx, button: &Button) {
        set_current_select(button_index(&button.name));

        ScreenSlot::call(g, mainmenu_screen_slot, |s, g| {
            let Some(theme) = s.as_any_mut().downcast_mut::<ThemeInterface>() else {
                return;
            };
            let themes = theme.list_available_themes(g);
            for i in 0..themes.size() {
                if themes.valid_index(i) {
                    let bname = format!("theme {i}");
                    g.ecl.dirty_button(&bname);
                }
            }
        });
    }

    /// Static `ListAvailableThemes`: lists the theme subdirectories once and caches them in `themes`.
    pub fn list_available_themes(&mut self, g: &mut Ctx) -> &DArray<String> {
        if self.themes.is_none() {
            let mut themes: DArray<String> = DArray::new();
            for name in list_subdirs(&g.app.path) {
                themes.put_data(name);
            }

            for i in (0..themes.size()).rev() {
                if let Some(themename) = themes.get_data(i).cloned() {
                    // Is there a theme in here?
                    let themefilename = format!("{themename}/theme.txt");
                    if !archive::exists(&themefilename) {
                        themes.remove_data(i);
                        continue;
                    }

                    if lower_case_string(&themename) == "graphics" {
                        themes.remove_data(i);
                    }
                }
            }

            self.themes = Some(themes);
        }
        self.themes.as_ref().expect("themes listed above")
    }
}

impl InterfaceScreen for ThemeInterface {
    fn create(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            return;
        }

        mainmenuscreen::create(g);

        let screenw = g.app.options.get_option_value("graphics_screenwidth");
        let screenh = g.app.options.get_option_value("graphics_screenheight");

        let num_themes = self.list_available_themes(g).num_used();

        // Title

        g.ecl.register_button(screenw - 210, screenh - 40, 200, 15, "THEME options", "Close this options screen", "theme_title");
        g.ecl.register_button_callback("theme_title", Self::return_to_main_menu_click);
        g.ecl.register_movement("theme_title", screenw - 210, screenh - 170 - num_themes * 20, 500, MOVE_STRAIGHTLINE, None);

        // Available themes

        g.ecl.register_button(screenw - 210, screenh - 40, 200, 15, "Available Themes", " ", "theme_text");
        g.ecl.register_button_callbacks("theme_text", Some(textbutton_draw), None, None, None);
        g.ecl.register_movement("theme_text", screenw - 210, screenh - 150 - num_themes * 20, 500, MOVE_STRAIGHTLINE, None);

        // One button for each theme

        let mut ypos = screenh - 130;

        let themes = self.list_available_themes(g);
        for i in 0..themes.size() {
            if let Some(theme_name) = themes.get_data(i) {
                let bname = format!("theme {i}");
                g.ecl.register_button(screenw - 210, screenh - 40, 200, 15, theme_name, "Select this theme", &bname);
                g.ecl.register_button_callbacks(
                    &bname,
                    Some(Self::theme_name_draw),
                    Some(Self::theme_name_click),
                    Some(button_click),
                    Some(button_highlight),
                );
                g.ecl.register_movement(&bname, screenw - 210, ypos, 500, MOVE_STRAIGHTLINE, None);
                ypos -= 20;
            }
        }

        // Apply button

        g.ecl.register_button(screenw - 210, screenh - 40, 200, 15, "Apply", "Set the current theme", "theme_apply");
        g.ecl.register_button_callback("theme_apply", Self::apply_click);
        g.ecl.register_movement("theme_apply", screenw - 210, screenh - 90, 500, MOVE_STRAIGHTLINE, None);

        // Default button

        g.ecl.register_button(screenw - 210, screenh - 40, 200, 15, "Restore Default", "Set the theme back to standard", "theme_default");
        g.ecl.register_button_callback("theme_default", Self::default_click);
        g.ecl.register_movement("theme_default", screenw - 210, screenh - 70, 500, MOVE_STRAIGHTLINE, None);

        // Close button

        g.ecl.register_button(screenw - 60, screenh - 40, 50, 15, "Close", "Return to Options menu", "theme_return");
        g.ecl.register_button_callback("theme_return", Self::return_to_main_menu_click);

        // Put some icons left of the screen for a sample

        g.ecl.register_button(-150, 300, 220, 120, "userid", " ", "theme_userid");
        button_assignbitmap(g, "theme_userid", "userid.tif");
        g.ecl.register_movement("theme_userid", 50, 300, 500, MOVE_STRAIGHTLINE, None);

        g.ecl.register_button(290, screenh + 10, 24, 24, "email", " ", "theme_email");
        button_assignbitmaps(g, "theme_email", "hud/email.tif", "hud/email_h.tif", "hud/email_c.tif");
        g.ecl.register_movement("theme_email", 290, 300, 500, MOVE_STRAIGHTLINE, None);

        g.ecl.register_button(290, screenh + 50, 24, 24, "finance", " ", "theme_finance");
        button_assignbitmaps(g, "theme_finance", "hud/finance.tif", "hud/finance_h.tif", "hud/finance_c.tif");
        g.ecl.register_movement("theme_finance", 290, 332, 500, MOVE_STRAIGHTLINE, None);

        g.ecl.register_button(290, screenh + 90, 24, 24, "hardware", " ", "theme_hardware");
        button_assignbitmaps(g, "theme_hardware", "hud/hardware.tif", "hud/hardware_h.tif", "hud/hardware_c.tif");
        g.ecl.register_movement("theme_hardware", 290, 364, 500, MOVE_STRAIGHTLINE, None);

        g.ecl.register_button(290, screenh + 130, 24, 24, "status", " ", "theme_status");
        button_assignbitmaps(g, "theme_status", "hud/status.tif", "hud/status_h.tif", "hud/status_c.tif");
        g.ecl.register_movement("theme_status", 290, 396, 500, MOVE_STRAIGHTLINE, None);
    }

    fn remove(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            return;
        }

        g.ecl.remove_button("theme_title");
        g.ecl.remove_button("theme_text");
        g.ecl.remove_button("theme_name");
        g.ecl.remove_button("theme_default");
        g.ecl.remove_button("theme_apply");
        g.ecl.remove_button("theme_return");

        if let Some(themes) = self.themes.take() {
            for i in 0..themes.size() {
                if themes.valid_index(i) {
                    let bname = format!("theme {i}");
                    g.ecl.remove_button(&bname);
                }
            }
        }

        g.ecl.remove_button("theme_userid");
        g.ecl.remove_button("theme_email");
        g.ecl.remove_button("theme_finance");
        g.ecl.remove_button("theme_hardware");
        g.ecl.remove_button("theme_status");
    }

    fn update(&mut self, _g: &mut Ctx) {}
    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("theme_return").is_some()
    }
    fn screen_id(&self) -> i32 {
        MAINMENU_THEME
    }
    crate::impl_as_any!();
}
