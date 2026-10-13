//! Port of source/options/options.h + source/options/options.cpp
//!
//! Stores all of the options specified by the user. Unlike other world objects the
//! options are persisted to their own file (`<userpath>options`); PORT: as JSON via serde
//! instead of the C++ binary format (`load` / `save` take the user path from `App`).
//!
//! PORT: the C++ class `Option` is renamed `UplinkOption` so it does not shadow `std::option::Option`.

#![allow(unused_variables, unused_imports, unused_mut, dead_code)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use uplink_engine::archive;

use crate::app::globals;
use crate::app::miscutils;
use crate::tosser::BTree;

pub const SIZE_OPTION_NAME: usize = 64;
pub const SIZE_OPTION_TOOLTIP: usize = 128;

// ============================================================================

/// One named integer option (C++ `Option`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UplinkOption {
    pub name: String,
    pub tooltip: String,
    /// Is this a boolean option
    pub yesorno: bool,
    /// Is this a hidden option
    pub visible: bool,
    pub value: i32,
}

impl Default for UplinkOption {
    fn default() -> Self {
        UplinkOption {
            name: String::new(),
            tooltip: String::new(),
            yesorno: false,
            visible: true,
            value: 0,
        }
    }
}

impl UplinkOption {
    pub fn new() -> Self {
        UplinkOption::default()
    }

    pub fn set_name(&mut self, newname: &str) {
        debug_assert!(newname.len() < SIZE_OPTION_NAME);
        self.name = newname.to_string();
    }

    pub fn set_tooltip(&mut self, newtooltip: &str) {
        debug_assert!(newtooltip.len() < SIZE_OPTION_TOOLTIP);
        self.tooltip = newtooltip.to_string();
    }

    pub fn set_yes_or_no(&mut self, newyesorno: bool) {
        self.yesorno = newyesorno;
    }

    pub fn set_visible(&mut self, newvisible: bool) {
        self.visible = newvisible;
    }

    pub fn set_value(&mut self, newvalue: i32) {
        self.value = newvalue;
    }

    pub fn update(&mut self) {}
}

/// A queued option change, applied by `Options::apply_shutdown_changes`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OptionChange {
    pub name: String,
    pub value: i32,
}

/// A theme colour (components 0.0..1.0).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ColourOption {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl ColourOption {
    pub fn new(r: f32, g: f32, b: f32) -> Self {
        ColourOption { r, g, b }
    }
}

/// Contents of `<userpath>options`. PORT: JSON instead of the C++ binary layout; the C++ header
/// (`SAVEFILE_VERSION`) is kept as `version`, and the theme name trailer as `theme_name`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct OptionsFile {
    version: String,
    options: BTreeMap<String, UplinkOption>,
    /// Option names in C++ `BTree` pre-order, so reloading rebuilds the same tree shape.
    order: Vec<String>,
    theme_name: String,
}

/// C `isspace` (the separators `>>` skips in the theme file).
fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

/// Latin-1 bytes to a string, one char per byte (like the C `char` buffers).
fn latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| b as char).collect()
}

/// `istream >> token` on a byte cursor: skips leading whitespace, reads up to the next whitespace.
fn read_token(text: &[u8], pos: &mut usize) -> String {
    skip_ws(text, pos);
    let start = *pos;
    while *pos < text.len() && !is_space(text[*pos]) {
        *pos += 1;
    }
    latin1(&text[start..*pos])
}

/// `istream >> ws`
fn skip_ws(text: &[u8], pos: &mut usize) {
    while *pos < text.len() && is_space(text[*pos]) {
        *pos += 1;
    }
}

/// `istream::getline(buf, max)`: stores at most `max - 1` bytes, stops after a newline (not stored).
fn getline(text: &[u8], pos: &mut usize, max: usize) -> String {
    let mut out = Vec::new();
    while *pos < text.len() && out.len() < max - 1 {
        let c = text[*pos];
        *pos += 1;
        if c == b'\n' {
            break;
        }
        out.push(c);
    }
    latin1(&out)
}

// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Options {
    /// tosser BTree: `GetAllOptions` walks it in pre-order, so the option screens list the
    /// options in an order that depends on insertion order.
    pub options: BTree<UplinkOption>,
    pub shutdown_changes: Vec<OptionChange>,
    pub theme_name: String,
    pub theme_author: String,
    pub theme_title: String,
    pub theme_description: String,
    pub colours: BTree<ColourOption>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            options: BTree::new(),
            shutdown_changes: Vec::new(),
            theme_name: "graphics".to_string(),
            theme_author: String::new(),
            theme_title: String::new(),
            theme_description: String::new(),
            colours: BTree::new(),
        }
    }
}

impl Options {
    pub fn new() -> Self {
        Options::default()
    }

    /// Fills in all options not yet created, and hides the old resolution options.
    pub fn create_default_options(&mut self) {
        // Game

        if self.get_option("game_debugstart").is_none() {
            self.set_option_value_full("game_debugstart", 0, "z", true, false);
        }

        if self.get_option("game_firsttime").is_none() {
            let firsttime = if globals::TESTGAME { 0 } else { 1 };
            self.set_option_value_full("game_firsttime", firsttime, "z", true, false);
        }

        if self.get_option("game_version").is_none() {
            self.set_option_value_full("game_version", 100, "z", false, false);
        }

        // Graphics

        if self.get_option("graphics_screenwidth").is_none() {
            self.set_option_value_full("graphics_screenwidth", 640, "Sets the width of the screen", false, false);
        }
        if self.get_option("graphics_screenheight").is_none() {
            self.set_option_value_full("graphics_screenheight", 480, "Sets the height of the screen", false, false);
        }
        if self.get_option("graphics_screendepth").is_none() {
            self.set_option_value_full(
                "graphics_screendepth",
                -1,
                "Sets the colour depth. -1 Means use desktop colour depth.",
                false,
                false,
            );
        }
        if self.get_option("graphics_screenrefresh").is_none() {
            self.set_option_value_full(
                "graphics_screenrefresh",
                -1,
                "Sets the refresh rate. -1 Means use desktop refresh.",
                false,
                false,
            );
        }
        if self.get_option("graphics_fullscreen").is_none() {
            self.set_option_value_full(
                "graphics_fullscreen",
                1,
                "Sets the game to run fullscreen or in a window",
                true,
                true,
            );
        }
        if self.get_option("graphics_buttonanimations").is_none() {
            self.set_option_value_full(
                "graphics_buttonanimations",
                1,
                "Enables or disables button animations",
                true,
                true,
            );
        }
        if self.get_option("graphics_safemode").is_none() {
            self.set_option_value_full(
                "graphics_safemode",
                0,
                "Enables graphical safemode for troubleshooting",
                true,
                true,
            );
        }
        if self.get_option("graphics_softwaremouse").is_none() {
            self.set_option_value_full(
                "graphics_softwaremouse",
                0,
                "Render a software mouse.  Use to correct mouse problems.",
                true,
                true,
            );
        }

        // Sound

        if self.get_option("sound_musicenabled").is_none() {
            self.set_option_value_full("sound_musicenabled", 1, "Enables or disables music", true, true);
        }

        // Network

        // Theme

        // SetThemeName ( "graphics" );

        // Repair old options files

        for name in [
            "graphics_screenwidth",
            "graphics_screenheight",
            "graphics_screendepth",
            "graphics_screenrefresh",
        ] {
            // PORT: the C++ dereferenced the option unconditionally; every name was created above.
            if let Some(option) = self.get_option_mut(name) {
                option.set_visible(false);
            }
        }
    }

    /// Returns None if not found.
    pub fn get_option(&self, name: &str) -> Option<&UplinkOption> {
        self.options.get(name)
    }

    pub fn get_option_mut(&mut self, name: &str) -> Option<&mut UplinkOption> {
        self.options.get_mut(name)
    }

    /// Asserts existence of `name` (panics, like `UplinkAbort`).
    pub fn get_option_value(&self, name: &str) -> i32 {
        match self.options.get(name) {
            Some(o) => o.value,
            None => panic!("Option {} not found", name),
        }
    }

    pub fn is_option_equal_to(&self, name: &str, value: i32) -> bool {
        self.options.get(name).is_some_and(|o| o.value == value)
    }

    /// Does not create a new option; prints a warning for unknown names.
    pub fn set_option_value(&mut self, name: &str, newvalue: i32) {
        match self.options.get_mut(name) {
            Some(o) => o.set_value(newvalue),
            None => println!("Tried to set unrecognised option: {}", name),
        }
    }

    /// C++ overload `SetOptionValue(name, value, tooltip, yesorno, visible)`: creates the option if necessary.
    pub fn set_option_value_full(
        &mut self,
        name: &str,
        newvalue: i32,
        tooltip: &str,
        yesorno: bool,
        visible: bool,
    ) {
        if let Some(o) = self.options.get_mut(name) {
            o.set_value(newvalue);
            o.set_tooltip(tooltip);
            o.set_yes_or_no(yesorno);
            o.set_visible(visible);
        } else {
            // Create a new entry
            let mut o = UplinkOption::new();
            o.set_name(name);
            o.set_value(newvalue);
            o.set_tooltip(tooltip);
            o.set_yes_or_no(yesorno);
            o.set_visible(visible);
            self.options.put_data(name, o);
        }
    }

    /// `searchstring` None returns all options (substring match on the name otherwise).
    pub fn get_all_options(
        &self,
        searchstring: Option<&str>,
        returnhidden: bool,
    ) -> Vec<&UplinkOption> {
        // ConvertToDArray: BTree pre-order
        self.options
            .values()
            .filter(|o| o.visible || returnhidden)
            .filter(|o| searchstring.is_none_or(|s| o.name.contains(s)))
            .collect()
    }

    /// Parses `<theme>/theme.txt` (title, author, description, then colour lines).
    pub fn set_theme_name(&mut self, new_theme_name: &str) {
        self.theme_name = new_theme_name.to_string();

        // Parse the theme.txt file

        let theme_text_filename = self.theme_filename("theme.txt");
        if let Some(raw) = archive::read(&theme_text_filename) {
            // dos2unixstream: every carriage return is dropped
            let text: Vec<u8> = raw.into_iter().filter(|&c| c != b'\r').collect();
            let mut pos = 0usize;

            // Header: each of the three first lines is "<label> <value...>"
            // PORT: the C++ buffers are 128 / 128 / 1024 bytes; getline truncates the same way.

            let _unused = read_token(&text, &mut pos);
            skip_ws(&text, &mut pos);
            self.theme_title = getline(&text, &mut pos, 128);

            let _unused = read_token(&text, &mut pos);
            skip_ws(&text, &mut pos);
            self.theme_author = getline(&text, &mut pos, 128);

            let _unused = read_token(&text, &mut pos);
            skip_ws(&text, &mut pos);
            self.theme_description = getline(&text, &mut pos, 1024);

            // Colours: "<name> <r> <g> <b>" per line; lines shorter than 2 chars and ';' comments are skipped

            while pos < text.len() {
                let line = getline(&text, &mut pos, 256);

                if line.chars().count() < 2 {
                    continue;
                }
                if line.starts_with(';') {
                    continue;
                }

                let mut fields = line.split_ascii_whitespace();
                let Some(colour_name) = fields.next() else {
                    // PORT: the C++ read an uninitialised name from a whitespace-only line
                    continue;
                };
                let mut component = || fields.next().and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.0);
                let r = component();
                let g = component();
                let b = component();

                match self.colours.get_mut(colour_name) {
                    Some(exists) => *exists = ColourOption::new(r, g, b),
                    None => {
                        self.colours.put_data(colour_name, ColourOption::new(r, g, b));
                    }
                }
            }
        }
    }

    pub fn get_theme_name(&self) -> &str {
        &self.theme_name
    }

    pub fn get_theme_title(&self) -> &str {
        &self.theme_title
    }

    pub fn get_theme_description(&self) -> &str {
        &self.theme_description
    }

    /// Always safe - returns BLACK (and prints a warning) if not found.
    pub fn get_colour(&self, colour_name: &str) -> ColourOption {
        match self.colours.get(colour_name) {
            Some(colour) => *colour,
            None => {
                println!("Options::GetColour WARNING : Couldn't find colour {}", colour_name);
                ColourOption::new(0.0, 0.0, 0.0)
            }
        }
    }

    /// Archive path of a theme file: `graphics/<f>`, or `<theme>/<f>` if the theme provides it.
    // PORT: existence is checked through the archive/data dir rather than `app->path`.
    pub fn theme_filename(&self, filename: &str) -> String {
        if self.theme_name == "graphics" {
            return format!("graphics/{}", filename);
        }

        let themed = format!("{}/{}", self.theme_name, filename);
        if archive::exists(&themed) {
            themed
        } else {
            format!("graphics/{}", filename)
        }
    }

    pub fn request_shutdown_change(&mut self, option_name: &str, new_value: i32) {
        self.shutdown_changes.push(OptionChange {
            name: option_name.to_string(),
            value: new_value,
        });
    }

    pub fn apply_shutdown_changes(&mut self) {
        for oc in std::mem::take(&mut self.shutdown_changes) {
            self.set_option_value(&oc.name, oc.value);
        }
    }

    /// Reads `<userpath>options` (JSON). Missing / unreadable file leaves the defaults.
    pub fn load(&mut self, userpath: &str) {
        // Read from our own options file

        let filename = format!("{}options", userpath);
        print!("Loading uplink options from {}...", filename);

        let Ok(bytes) = std::fs::read(&filename) else {
            println!("failed");
            return;
        };

        let parsed: Result<OptionsFile, serde_json::Error> = serde_json::from_slice(&bytes);
        let file = match parsed {
            Ok(file) if file.version == globals::SAVEFILE_VERSION => file,
            _ => {
                println!("\nERROR : Could not load options due to old version format");
                return;
            }
        };

        println!("success");

        // Insert in the saved pre-order so the tree shape is rebuilt like LoadBTree did
        let mut options = file.options;
        for name in file.order {
            if let Some(option) = options.remove(&name) {
                self.put_option(name, option);
            }
        }
        for (name, option) in options {
            self.put_option(name, option);
        }

        // The theme name is only taken if it fits the 128 byte buffer, like the C++ check
        if !file.theme_name.is_empty() && file.theme_name.len() + 1 < 128 {
            self.theme_name = file.theme_name;
        }
    }

    /// Writes `<userpath>options` (JSON), creating the directory if needed.
    pub fn save(&self, userpath: &str) {
        // Write to our own options file

        miscutils::make_directory(userpath);

        let filename = format!("{}options", userpath);
        print!("Saving uplink options to {}...", filename);

        let file = OptionsFile {
            version: globals::SAVEFILE_VERSION.to_string(),
            options: self.options.iter().map(|(k, o)| (k.to_string(), o.clone())).collect(),
            order: self.options.keys_all(),
            theme_name: self.theme_name.clone(),
        };

        let written = serde_json::to_vec_pretty(&file)
            .ok()
            .and_then(|bytes| std::fs::write(&filename, bytes).ok());
        match written {
            Some(()) => println!("success"),
            None => println!("failed"),
        }
    }

    pub fn update(&mut self) {}

    /// LoadBTree's `PutData`
    fn put_option(&mut self, name: String, option: UplinkOption) {
        self.options.put_data(&name, option);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::scratch_dir;

    fn defaults() -> Options {
        let mut o = Options::new();
        o.create_default_options();
        o
    }

    #[test]
    fn default_options() {
        let o = defaults();
        assert_eq!(o.get_option_value("graphics_screenwidth"), 640);
        assert_eq!(o.get_option_value("graphics_screenheight"), 480);
        assert_eq!(o.get_option_value("graphics_fullscreen"), 1);
        assert!(o.is_option_equal_to("game_firsttime", 1));
        assert!(o.is_option_equal_to("game_version", 100));
        assert!(!o.is_option_equal_to("no_such_option", 0));
        assert_eq!(o.get_theme_name(), "graphics");
    }

    #[test]
    fn creating_the_defaults_twice_keeps_changed_values() {
        let mut o = defaults();
        o.set_option_value("graphics_screenwidth", 1024);
        let count = o.options.size();
        o.create_default_options();
        assert_eq!(o.get_option_value("graphics_screenwidth"), 1024);
        assert_eq!(o.options.size(), count);
    }

    #[test]
    fn resolution_options_are_hidden_from_the_option_screens() {
        let o = defaults();
        let visible: Vec<&str> = o.get_all_options(None, false).iter().map(|o| o.name.as_str()).collect();
        assert!(visible.contains(&"graphics_fullscreen"));
        assert!(!visible.contains(&"graphics_screenwidth"));
        assert!(!visible.contains(&"game_version"));
        let all = o.get_all_options(None, true);
        assert_eq!(all.len(), o.options.size() as usize);
        let graphics = o.get_all_options(Some("graphics_"), true);
        assert!(graphics.iter().all(|o| o.name.starts_with("graphics_")));
        assert!(graphics.len() >= 8);
    }

    #[test]
    fn unknown_options_are_not_created_by_set_option_value() {
        let mut o = defaults();
        let before = o.options.size();
        o.set_option_value("made_up", 5);
        assert_eq!(o.options.size(), before);
        o.set_option_value_full("made_up", 5, "tip", true, true);
        assert_eq!(o.options.size(), before + 1);
        assert_eq!(o.get_option_value("made_up"), 5);
        assert_eq!(o.get_option("made_up").unwrap().tooltip, "tip");
    }

    #[test]
    #[should_panic(expected = "not found")]
    fn reading_an_unknown_option_value_panics_like_the_assert() {
        defaults().get_option_value("made_up");
    }

    #[test]
    fn shutdown_changes_are_applied_once() {
        let mut o = defaults();
        o.request_shutdown_change("graphics_fullscreen", 0);
        assert_eq!(o.get_option_value("graphics_fullscreen"), 1);
        o.apply_shutdown_changes();
        assert_eq!(o.get_option_value("graphics_fullscreen"), 0);
        o.set_option_value("graphics_fullscreen", 1);
        o.apply_shutdown_changes();
        assert_eq!(o.get_option_value("graphics_fullscreen"), 1);
    }

    #[test]
    fn options_survive_a_save_and_load() {
        let dir = scratch_dir("options");
        let mut o = defaults();
        o.set_option_value("graphics_screenwidth", 1280);
        o.set_option_value("sound_musicenabled", 0);
        o.set_option_value_full("game_extra", 7, "Extra", true, true);
        o.theme_name = "mytheme".to_string();
        o.save(&dir);
        assert!(std::path::Path::new(&format!("{dir}options")).is_file());

        let mut back = Options::new();
        back.load(&dir);
        assert_eq!(back.get_option_value("graphics_screenwidth"), 1280);
        assert_eq!(back.get_option_value("sound_musicenabled"), 0);
        assert_eq!(back.get_option_value("game_extra"), 7);
        assert_eq!(back.get_theme_name(), "mytheme");
        // the option screens list the options in the same order after a reload
        let names = |o: &Options| o.get_all_options(None, true).iter().map(|o| o.name.clone()).collect::<Vec<_>>();
        assert_eq!(names(&back), names(&o));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn loading_a_missing_or_wrong_version_file_leaves_the_defaults() {
        let dir = scratch_dir("options-bad");
        let mut o = Options::new();
        o.load(&dir);
        assert!(o.options.is_empty());

        std::fs::write(format!("{dir}options"), br#"{"version":"SAV01","options":{"x":{"name":"x","value":3}}}"#).unwrap();
        o.load(&dir);
        assert!(o.options.is_empty());

        std::fs::write(format!("{dir}options"), b"not json at all").unwrap();
        o.load(&dir);
        assert!(o.options.is_empty());

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn colours_not_in_the_theme_are_black() {
        let o = defaults();
        let c = o.get_colour("NoSuchColour");
        assert_eq!((c.r, c.g, c.b), (0.0, 0.0, 0.0));
    }

    #[test]
    fn theme_files_come_from_graphics_unless_the_theme_has_them() {
        let o = defaults();
        assert_eq!(o.theme_filename("theme.txt"), "graphics/theme.txt");
        let mut o = defaults();
        o.theme_name = "no_such_theme".to_string();
        assert_eq!(o.theme_filename("theme.txt"), "graphics/theme.txt");
    }

    #[test]
    fn the_theme_file_of_the_game_parses() {
        if !crate::testutil::mount_game_data() {
            return;
        }
        let mut o = defaults();
        o.set_theme_name("graphics");
        assert!(!o.get_theme_title().is_empty());
        let panel = o.get_colour("PanelBackgroundA");
        for v in [panel.r, panel.g, panel.b] {
            assert!((0.0..=1.0).contains(&v));
        }
        assert!(o.colours.size() > 20);
    }
}
