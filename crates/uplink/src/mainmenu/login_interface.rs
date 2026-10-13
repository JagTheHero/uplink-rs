//! Port of source/mainmenu/login_interface.h + source/mainmenu/login_interface.cpp

use std::rc::Rc;

use uplink_engine::{gl, sound, text};

use crate::app::app::App;
use crate::app::globals;
use crate::app::opengl::{display, timer_func};
use crate::app::opengl_interface::{
    border_draw, button_assignbitmap, button_assignbitmaps, button_click, button_highlight,
    imagebutton_draw, set_colour, sx, sy, text_draw, textbutton_draw,
};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::game::game::Game;
use crate::game::scriptlibrary::ScriptLibrary;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::mainmenu::mainmenu::{
    MainMenu, MAINMENU_CLOSING, MAINMENU_FIRSTLOAD, MAINMENU_LOADING, MAINMENU_LOGIN, MAINMENU_OPTIONS,
};
use crate::mainmenu::mainmenuscreen;
use crate::network::network::NETWORK_CLIENT;
use crate::network::networkclient::{NetworkClient, CLIENT_COMMS, CLIENT_STATUS};
use crate::tosser::DArray;

/// Login interface: the first main menu (Load, New, Options, Exit etc).
#[derive(Debug, Default)]
pub struct LoginInterface {}

impl LoginInterface {
    pub fn new() -> Self {
        Self::default()
    }

    /// Protected static `ListExistingGames`: the names of the saved `.usr` games in the user path.
    pub fn list_existing_games(g: &Ctx) -> DArray<String> {
        let mut existing = DArray::new();

        if let Ok(entries) = std::fs::read_dir(&g.app.userpath) {
            for entry in entries.flatten() {
                let file_name = entry.file_name();
                let file_name = file_name.to_string_lossy();
                // strstr(entry->d_name, ".usr"): keep the text before the first ".usr"
                if let Some(p) = file_name.find(".usr") {
                    existing.put_data(file_name[..p].to_string());
                }
            }
        }

        existing
    }

    /// Static `NewGameClick` (MouseFn).
    pub fn new_game_click(g: &mut Ctx, _button: &Button) {
        if !globals::TESTGAME {
            // Do the start sequence

            g.app.options.set_option_value("game_firsttime", 1);

            MainMenu::run_screen(g, MAINMENU_FIRSTLOAD);
            timer_func(2000, ScriptLibrary::run_script, 30);
        } else {
            // Skip it

            MainMenu::run_screen(g, MAINMENU_LOADING);
            display(g);

            Game::new_game(g);
            MainMenu::remove(g);
        }

        if g.app.options.is_option_equal_to("sound_musicenabled", 1) {
            //SgPlaylist_Play ("ambient");
            sound::playlist_play("main");
        }
    }

    /// Static `ProceedClick` (MouseFn).
    pub fn proceed_click(g: &mut Ctx, _button: &Button) {
        Self::proceed(g)
    }

    /// The body of `ProceedClick`, shared with `ReturnKeyPressed` (C++ called `ProceedClick(NULL)`).
    // PORT: C++ passes a NULL button which the body never reads, so the button-less body is split out.
    pub fn proceed(g: &mut Ctx) {
        debug_assert!(g.ecl.get_button("userid_name").is_some(), "ProceedClick: userid_name button missing");
        let username = g.ecl.get_button("userid_name").map(|b| b.caption.clone()).unwrap_or_default();

        MainMenu::run_screen(g, MAINMENU_LOADING);
        display(g);

        App::load_game(g, &username);

        if g.app.options.is_option_equal_to("sound_musicenabled", 1) {
            //SgPlaylist_Play ("ambient");
            sound::playlist_play("main");
        }
    }

    /// Static `OptionsClick` (MouseFn).
    pub fn options_click(g: &mut Ctx, _button: &Button) {
        MainMenu::run_screen(g, MAINMENU_OPTIONS);
        //    SgPlaylist_NextSong();
    }

    /// Static `ExitGameClick` (MouseFn).
    pub fn exit_game_click(g: &mut Ctx, _button: &Button) {
        if globals::DEMOGAME {
            MainMenu::run_screen(g, MAINMENU_CLOSING);
        } else {
            App::close(g);
        }
    }

    /// Static `CommsClick` (MouseFn).
    pub fn comms_click(g: &mut Ctx, _button: &Button) {
        debug_assert!(g.app.network.status == NETWORK_CLIENT);

        let width = g.app.options.get_option_value("graphics_screenwidth");
        let height = g.app.options.get_option_value("graphics_screenheight");
        g.ecl.reset(width, height);

        NetworkClient::set_client_type(g, CLIENT_COMMS);
    }

    /// Static `StatusClick` (MouseFn).
    pub fn status_click(g: &mut Ctx, _button: &Button) {
        debug_assert!(g.app.network.status == NETWORK_CLIENT);

        let width = g.app.options.get_option_value("graphics_screenwidth");
        let height = g.app.options.get_option_value("graphics_screenheight");
        g.ecl.reset(width, height);

        NetworkClient::set_client_type(g, CLIENT_STATUS);
    }

    /// Static `LargeTextBoxDraw` (DrawFn).
    pub fn large_text_box_draw(g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        set_colour(g, "TitleText");
        let ypos = (button.y + button.height / 2) + 5;

        text::draw_text_styled(button.x, ypos, &button.caption, text::HELVETICA_18);
    }

    /// Static `UserIDButtonDraw` (DrawFn).
    pub fn user_id_button_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        let screenheight = g.app.options.get_option_value("graphics_screenheight");
        gl::scissor(button.x, screenheight - (button.y + button.height), button.width, button.height);
        gl::enable(gl::SCISSOR_TEST);

        // Draw a background colour

        set_colour(g, "PasswordBoxBackground");

        gl::begin(gl::QUADS);

        gl::vertex2i(button.x, button.y);
        gl::vertex2i(button.x + button.width - 1, button.y);
        gl::vertex2i(button.x + button.width - 1, button.y + button.height);
        gl::vertex2i(button.x, button.y + button.height);

        gl::end();

        // Draw the text

        set_colour(g, "DefaultText");

        text_draw(g, button, highlighted, clicked);

        // Draw a border

        if highlighted || clicked {
            border_draw(button);
        }

        gl::disable(gl::SCISSOR_TEST);
    }

    /// Static `CodeButtonDraw` (DrawFn).
    pub fn code_button_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        let screenheight = g.app.options.get_option_value("graphics_screenheight");
        gl::scissor(button.x, screenheight - (button.y + button.height), button.width, button.height);
        gl::enable(gl::SCISSOR_TEST);

        // Draw a background colour

        set_colour(g, "PasswordBoxBackground");

        gl::begin(gl::QUADS);

        gl::vertex2i(button.x, button.y);
        gl::vertex2i(button.x + button.width - 1, button.y);
        gl::vertex2i(button.x + button.width - 1, button.y + button.height);
        gl::vertex2i(button.x, button.y + button.height);

        gl::end();

        // Print the text

        set_colour(g, "DefaultText");

        // One '*' per byte of the caption (C++ strlen)
        let stars = "*".repeat(button.caption.len());
        text::draw_text_styled(button.x + 10, button.y + 10, &stars, text::BITMAP_15);

        // Draw a box around the text if highlighted

        if highlighted || clicked {
            border_draw(button);
        }

        gl::disable(gl::SCISSOR_TEST);
    }

    /// Static `UserNameClick` (MouseFn).
    pub fn user_name_click(g: &mut Ctx, button: &Button) {
        g.ecl.register_caption_change_auto("userid_name", &button.caption, None);
        g.ecl.register_caption_change_auto("userid_code", "PASSWORD", None);
    }
}

impl InterfaceScreen for LoginInterface {
    fn create(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            // Play some music

            if g.app.options.is_option_equal_to("sound_musicenabled", 1) {
                //SgPlaylist_Play ("action");
                sound::playlist_play("main");
            }

            mainmenuscreen::create(g);

            if g.app.network.status == NETWORK_CLIENT {
                g.ecl.register_button(20, 20, 110, 20, "COMMS", "Run as a dumb client, showing the communications status", "client_comms");
                g.ecl.register_button_callback("client_comms", Self::comms_click);
                g.ecl.register_button(20, 50, 110, 20, "STATUS", "Run as a dumb client, showing your characters status", "client_status");
                g.ecl.register_button_callback("client_status", Self::status_click);
            }

            // New game

            let buttons_x = sx(600);
            let buttons_y = sy(270);

            g.ecl.register_button(buttons_x, buttons_y, 32, 32, "", "Create a new user's account", "newgame");
            button_assignbitmaps(g, "newgame", "mainmenu/newgame.tif", "mainmenu/newgame_h.tif", "mainmenu/newgame_c.tif");
            g.ecl.register_button(buttons_x - 100, buttons_y + 5, 100, 20, "New User", "Create a new user's account", "newgame_text");
            g.ecl.register_button_callbacks(
                "newgame_text",
                Some(Self::large_text_box_draw),
                Some(Self::new_game_click),
                Some(button_click),
                Some(button_highlight),
            );

            // TODO : Look at this alpha stuff - it don't work no more
            if g.app.network.status == NETWORK_CLIENT {
                g.ecl.register_button_callbacks("newgame", Some(imagebutton_draw), None, None, None);
                // The C++ button owns its image; make_mut gives this button its own copy if it is shared.
                if let Some(img) = g.ecl.get_button_mut("newgame").and_then(|b| b.image_standard.as_mut()) {
                    Rc::make_mut(img).set_alpha(globals::ALPHA_DISABLED);
                }
            } else {
                g.ecl.register_button_callbacks(
                    "newgame",
                    Some(imagebutton_draw),
                    Some(Self::new_game_click),
                    Some(button_click),
                    Some(button_highlight),
                );
            }

            // Options

            g.ecl.register_button(buttons_x, buttons_y + 40, 32, 32, "", "Advanced options", "options");
            button_assignbitmaps(g, "options", "mainmenu/options.tif", "mainmenu/options_h.tif", "mainmenu/options_c.tif");
            g.ecl.register_button_callbacks(
                "options",
                Some(imagebutton_draw),
                Some(Self::options_click),
                Some(button_click),
                Some(button_highlight),
            );

            g.ecl.register_button(buttons_x - 80, buttons_y + 45, 80, 20, "Options", "Advanced options", "options_text");
            g.ecl.register_button_callbacks(
                "options_text",
                Some(Self::large_text_box_draw),
                Some(Self::options_click),
                Some(button_click),
                Some(button_highlight),
            );

            // Exit

            g.ecl.register_button(buttons_x, buttons_y + 80, 32, 32, "", "Exit Uplink", "exitgame");
            button_assignbitmaps(g, "exitgame", "mainmenu/exitgame.tif", "mainmenu/exitgame_h.tif", "mainmenu/exitgame_c.tif");
            g.ecl.register_button_callbacks(
                "exitgame",
                Some(imagebutton_draw),
                Some(Self::exit_game_click),
                Some(button_click),
                Some(button_highlight),
            );

            g.ecl.register_button(buttons_x - 45, buttons_y + 85, 50, 20, "Exit", "Exit Uplink", "exitgame_text");
            g.ecl.register_button_callbacks(
                "exitgame_text",
                Some(Self::large_text_box_draw),
                Some(Self::exit_game_click),
                Some(button_click),
                Some(button_highlight),
            );

            // UserID box

            let userid_x = sx(320) - 110;
            let userid_y = sy(265);

            g.ecl.register_button(userid_x, userid_y, 220, 120, "", "", "userid_image");
            button_assignbitmap(g, "userid_image", "userid.tif");

            g.ecl.register_button(userid_x + 59, userid_y + 55, 147, 15, "", "Enter your userID here", "userid_name");
            g.ecl.register_button_callbacks(
                "userid_name",
                Some(Self::user_id_button_draw),
                None,
                Some(button_click),
                Some(button_highlight),
            );

            g.ecl.register_button(userid_x + 59, userid_y + 81, 147, 15, "", "Enter your access code here", "userid_code");
            g.ecl.register_button_callbacks(
                "userid_code",
                Some(Self::code_button_draw),
                None,
                Some(button_click),
                Some(button_highlight),
            );

            g.ecl.register_button(userid_x, userid_y + 125, 120, 15, "", "", "userid_message");
            g.ecl.register_button_callbacks("userid_message", Some(textbutton_draw), None, None, None);

            g.ecl.register_button(userid_x + 140, userid_y + 125, 80, 15, "Proceed", "Click here when done", "userid_proceed");
            button_assignbitmaps(g, "userid_proceed", "proceed.tif", "proceed_h.tif", "proceed_c.tif");
            g.ecl.register_button_callback("userid_proceed", Self::proceed_click);

            // Text help box

            g.ecl.register_button(userid_x, sy(200), 240, sy(50), "", "", "texthelp");
            g.ecl.register_button_callbacks("texthelp", Some(textbutton_draw), None, None, None);
            g.ecl.register_caption_change_auto(
                "texthelp",
                "If you are registered as an Uplink Agent,\n\
                 enter your username and password below.\n\
                 Otherwise, click on the New User button.",
                None,
            );

            // List of all existing games

            g.ecl.register_button(sx(35), sy(270), sx(170), sy(20), "Valid User Names", "", "usernames_title");
            g.ecl.register_button_callbacks("usernames_title", Some(Self::large_text_box_draw), None, None, None);

            let existing = Self::list_existing_games(g);

            for (i, name) in existing.iter() {
                let bname = format!("username {i}");
                g.ecl.register_button(sx(35), sy(300) + i * 18, sx(170), 15, name, "Log in as this Agent", &bname);
                g.ecl.register_button_callbacks(
                    &bname,
                    Some(textbutton_draw),
                    Some(Self::user_name_click),
                    Some(button_click),
                    Some(button_highlight),
                );
            }

            g.ecl.make_button_editable("userid_name");
            g.ecl.make_button_editable("userid_code");
        }
    }

    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            mainmenuscreen::remove(g);

            if g.app.network.status == NETWORK_CLIENT {
                g.ecl.remove_button("client_comms");
                g.ecl.remove_button("client_status");
            }

            g.ecl.remove_button("newgame");
            g.ecl.remove_button("newgame_text");

            g.ecl.remove_button("options");
            g.ecl.remove_button("options_text");

            g.ecl.remove_button("exitgame");
            g.ecl.remove_button("exitgame_text");

            g.ecl.remove_button("userid_image");
            g.ecl.remove_button("userid_name");
            g.ecl.remove_button("userid_code");
            g.ecl.remove_button("userid_message");
            g.ecl.remove_button("userid_proceed");

            g.ecl.remove_button("texthelp");

            g.ecl.remove_button("usernames_title");

            // Remove all username buttons
            let mut usernameindex = 0;
            while g.ecl.get_button(&format!("username {usernameindex}")).is_some() {
                g.ecl.remove_button(&format!("username {usernameindex}"));
                usernameindex += 1;
            }
        }
    }

    fn update(&mut self, _g: &mut Ctx) {}

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("newgame").is_some()
    }
    fn screen_id(&self) -> i32 {
        MAINMENU_LOGIN
    }
    fn return_key_pressed(&mut self, g: &mut Ctx) -> bool {
        // PORT: this screen is taken out of its slot while this runs, so `proceed`'s
        // RunScreen(LOADING) would be deferred past `App::load_game`'s MainMenu::remove and the
        // loading overlay would stay on top of the game; run it once the slot is free again.
        g.defer(Self::proceed);
        true
    }
    crate::impl_as_any!();
}
