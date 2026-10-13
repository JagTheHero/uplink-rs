//! Port of source/interface/localinterface/sendmail_interface.h + source/interface/localinterface/sendmail_interface.cpp
//! Send-mail screen: composes an email to a contact, optionally attaching a file from the gateway databank.

use crate::app::opengl_interface::{button_click, button_draw, button_highlight, create_stextbox, draw_stextbox, remove_stextbox, textbox_draw};
use crate::ctx::Ctx;
use crate::eclipse::{Button, MOVE_STRAIGHTLINE};
use crate::impl_as_any;
use crate::interface::interfacescreen::{InterfaceScreen, ScreenSlot};
use crate::interface::localinterface::localinterface::{LocalInterface, PANELSIZE, SCREEN_NONE, SCREEN_SENDMAIL};
use crate::world::message::Message;

#[derive(Debug, Clone, Default)]
pub struct SendMailInterface {
    /// C++ `static Message *m`: the message being built up. Static in C++, so the draft survives
    /// closing the screen; the host keeps one persistent instance (`li.sendmail`) for that.
    // PORT: static -> field; the static callbacks reach it through the local interface.
    pub m: Message,
    /// C++ `static bool changed`: has `m` changed since the screen last read it?
    pub changed: bool,
}

/// `(screenw, screenh, paneltop, panelwidth)` as computed at the top of `SendMailInterface::Create`.
fn panel_geometry(g: &Ctx) -> (i32, i32, i32, i32) {
    let screenw = g.app.options.get_option_value("graphics_screenwidth");
    let screenh = g.app.options.get_option_value("graphics_screenheight");
    let paneltop = (100.0 * ((screenw as f64 * PANELSIZE) / 188.0) + 30.0) as i32;
    let panelwidth = (screenw as f64 * PANELSIZE) as i32;
    (screenw, screenh, paneltop, panelwidth)
}

/// The local interface's screen slot, used with `ScreenSlot::call`.
fn screen_slot(g: &mut Ctx) -> &mut ScreenSlot {
    &mut g.interface_mut().li.screen
}

/// The draft message (C++ static `SendMailInterface::m`), reached through the open sendmail screen.
fn with_draft<R>(g: &mut Ctx, f: impl FnOnce(&mut SendMailInterface) -> R) -> R {
    let draft = g
        .interface_mut()
        .li
        .screen
        .get_mut::<SendMailInterface>()
        .expect("sendmail: screen is not open");
    f(draft)
}

/// Caption of a button on this screen (C++ `EclGetButton(name)->caption`).
fn caption_of(g: &Ctx, name: &str) -> String {
    g.ecl.get_button(name).expect("sendmail: button missing").caption.clone()
}

/// Sets a button's caption (C++ `EclGetButton(name)->SetCaption(c)`).
fn set_caption(g: &mut Ctx, name: &str, caption: &str) {
    g.ecl
        .get_button_mut(name)
        .expect("sendmail: button missing")
        .set_caption(caption);
}

/// The to, subject and body captions the user has typed (C++ "Update m with the captions").
fn draft_captions(g: &Ctx) -> (String, String, String) {
    (
        caption_of(g, "sendmail_to"),
        caption_of(g, "sendmail_subject"),
        caption_of(g, "sendmail_body box"),
    )
}

/// C++ `while (EclGetButton(bname)) EclRemoveButton(bname)` over `"<prefix> 0"`, `"<prefix> 1"`, ...
fn remove_numbered_buttons(g: &mut Ctx, prefix: &str) {
    let mut index = 0;
    while g.ecl.get_button(&format!("{prefix} {index}")).is_some() {
        g.ecl.remove_button(&format!("{prefix} {index}"));
        index += 1;
    }
}

impl SendMailInterface {
    /// C++ `SendMailInterface::sGetMessage`
    pub fn s_get_message(&self) -> &Message {
        &self.m
    }

    /// C++ `SendMailInterface::TitleClick` (static button callback; also the Close button).
    pub fn title_click(g: &mut Ctx, _button: &Button) {
        LocalInterface::run_screen(g, SCREEN_NONE, -1);
    }

    /// C++ `SendMailInterface::FileClick` (static button callback: toggles the file list).
    pub fn file_click(g: &mut Ctx, button: &Button) {
        Self::toggle_file_list(g, button.x - 130, button.y);
    }

    /// C++ `SendMailInterface::AttachFileClick` (static button callback).
    pub fn attach_file_click(g: &mut Ctx, button: &Button) {
        let title = button.caption.clone();

        // Update m with the captions which may have been changed
        let (to, subject, body) = draft_captions(g);

        // Check this is still in memory
        let data = {
            let db = &g.player().gateway.databank;
            (0..db.get_data_size())
                .filter_map(|i| db.get_data_file(i))
                .find(|d| d.title == title)
                .cloned()
        };

        with_draft(g, |sm| {
            sm.m.set_to(&to);
            sm.m.set_from("PLAYER");
            sm.m.set_subject(&subject);
            sm.m.set_body(&body);

            if let Some(data) = &data {
                sm.m.attach_data(data);
                sm.changed = true;
            }
        });

        Self::toggle_file_list(g, 0, 0);
    }

    /// C++ `SendMailInterface::IsVisibleFileList` (static).
    pub fn is_visible_file_list(g: &Ctx) -> bool {
        g.ecl.get_button("sendmail_file 0").is_some()
    }

    /// C++ `SendMailInterface::ToggleFileList` (static): opens or closes the attachment list at (x, y).
    pub fn toggle_file_list(g: &mut Ctx, x: i32, y: i32) {
        if !Self::is_visible_file_list(g) {
            // Create a list of all files in memory
            let titles: Vec<String> = {
                let db = &g.player().gateway.databank;
                (0..db.get_data_size())
                    .filter_map(|i| db.get_data_file(i))
                    .map(|d| d.title.clone())
                    .collect()
            };

            let numfiles = titles.len() as i32;

            if numfiles > 0 {
                let timesplit = 500 / numfiles; // Ensures total time = 500ms

                for (si, title) in titles.iter().enumerate() {
                    let si = si as i32;
                    let name = format!("sendmail_file {si}");

                    g.ecl.register_button(x, y, 120, 15, title, "Attach this file to the mail message", &name);
                    g.ecl.register_button_callbacks(
                        &name,
                        Some(button_draw),
                        Some(Self::attach_file_click),
                        Some(button_click),
                        Some(button_highlight),
                    );
                    g.ecl.register_movement(&name, x, y - si * 17, si * timesplit, MOVE_STRAIGHTLINE, None);
                }
            } else {
                g.ecl.register_button(x, y, 120, 15, "No files", "", "sendmail_file 0");
            }
        } else {
            remove_numbered_buttons(g, "sendmail_file");
        }
    }

    /// C++ `SendMailInterface::ToClick` (static button callback: toggles the address book).
    pub fn to_click(g: &mut Ctx, button: &Button) {
        Self::toggle_address_book(g, button.x - 150, button.y);
    }

    /// C++ `SendMailInterface::IsVisibleAddressBook` (static).
    pub fn is_visible_address_book(g: &Ctx) -> bool {
        g.ecl.get_button("sendmail_addressbook 0").is_some()
    }

    /// C++ `SendMailInterface::ToggleAddressBook` (static): opens or closes the contacts list at (x, y).
    pub fn toggle_address_book(g: &mut Ctx, x: i32, y: i32) {
        if !Self::is_visible_address_book(g) {
            // Create address book
            let contacts: Vec<String> = g
                .player()
                .agent()
                .missions
                .iter()
                .map(|mission| mission.contact.clone())
                .collect();

            let numcontacts = contacts.len() as i32;

            if numcontacts > 0 {
                let timesplit = 500 / numcontacts; // Ensures total time = 500ms

                for (i, contact) in contacts.iter().enumerate() {
                    let i = i as i32;
                    let name = format!("sendmail_addressbook {i}");

                    g.ecl.register_button(x, y, 140, 15, contact, "Send to this recipient", &name);
                    g.ecl.register_button_callbacks(
                        &name,
                        Some(button_draw),
                        Some(Self::address_click),
                        Some(button_click),
                        Some(button_highlight),
                    );
                    g.ecl.register_movement(&name, x, y + i * 20, i * timesplit, MOVE_STRAIGHTLINE, None);
                }
            } else {
                g.ecl.register_button(x, y, 140, 15, "No contacts", "", "sendmail_addressbook 0");
            }
        } else {
            // Remove address book
            remove_numbered_buttons(g, "sendmail_addressbook");
        }
    }

    /// C++ `SendMailInterface::AddressClick` (static button callback).
    pub fn address_click(g: &mut Ctx, button: &Button) {
        // Copy the address to the TO box
        g.ecl.register_caption_change_auto("sendmail_to", &button.caption, None);

        // Remove the address book
        Self::toggle_address_book(g, 0, 0);
    }

    /// C++ `SendMailInterface::ClearClick` (static button callback).
    pub fn clear_click(g: &mut Ctx, _button: &Button) {
        // C++ `if (m) { delete m; m = new Message(); changed = true; }`: the draft always exists here.
        with_draft(g, |sm| {
            sm.m = Message::new();
            sm.changed = true;
        });
    }

    /// C++ `SendMailInterface::SendClick` (static button callback).
    pub fn send_click(g: &mut Ctx, _button: &Button) {
        // Update m with the captions which may have been changed
        let (to, subject, body) = draft_captions(g);

        // C++ `m->Send()`; the message is then in use elsewhere, so the draft is replaced with a new one.
        let msg = with_draft(g, |sm| {
            sm.m.set_to(&to);
            sm.m.set_from("PLAYER");
            sm.m.set_subject(&subject);
            sm.m.set_body(&body);
            std::mem::replace(&mut sm.m, Message::new())
        });

        msg.send(g);

        with_draft(g, |sm| sm.changed = true);
        ScreenSlot::call(g, screen_slot, |s, g| s.update(g));

        LocalInterface::run_screen(g, SCREEN_NONE, -1);
    }
}

impl InterfaceScreen for SendMailInterface {
    fn create(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            let (screenw, screenh, paneltop, panelwidth) = panel_geometry(g);

            g.ecl.register_button(screenw - panelwidth - 3, paneltop, panelwidth, 15, "SEND MAIL", "Remove the send mail screen", "sendmail_title");
            g.ecl.register_button_callback("sendmail_title", Self::title_click);

            g.ecl.register_button(screenw - panelwidth - 3, paneltop + 20, 50, 15, "To", "Show a list of known recipients", "sendmail_tto");
            g.ecl.register_button(screenw - panelwidth - 3, paneltop + 40, 50, 40, "Subject", "", "sendmail_tsubject");

            g.ecl.register_button_callbacks(
                "sendmail_tto",
                Some(button_draw),
                Some(Self::to_click),
                Some(button_click),
                Some(button_highlight),
            );
            g.ecl.register_button_callbacks("sendmail_tsubject", Some(button_draw), None, None, None);

            g.ecl.register_button(screenw - panelwidth + 49, paneltop + 20, panelwidth - 52, 15, "", "Enter recipient name here", "sendmail_to");
            g.ecl.register_button(screenw - panelwidth + 49, paneltop + 40, panelwidth - 52, 40, "", "Enter subject here", "sendmail_subject");

            g.ecl.register_button_callbacks("sendmail_to", Some(textbox_draw), None, None, Some(button_highlight));
            g.ecl.register_button_callbacks("sendmail_subject", Some(textbox_draw), None, None, Some(button_highlight));

            let boxheight = screenh - (paneltop + 85) - 90;
            create_stextbox(g, screenw - panelwidth - 3, paneltop + 85, panelwidth, boxheight, "", "sendmail_body");
            g.ecl.register_button_callbacks("sendmail_body box", Some(draw_stextbox), None, None, Some(button_highlight));

            g.ecl.register_button(
                screenw - panelwidth - 3,
                paneltop + 85 + boxheight + 5,
                panelwidth,
                15,
                "File: None attached",
                "Attach a file to this email",
                "sendmail_file",
            );
            g.ecl.register_button_callback("sendmail_file", Self::file_click);

            let x1 = screenw - panelwidth - 2;
            let x2 = x1 + panelwidth / 3 + 2;
            let x3 = x2 + panelwidth / 3 + 2;
            let y = paneltop + 85 + boxheight + 25;
            let width = panelwidth / 3 - 2;

            g.ecl.register_button(x1, y, width, 15, "Close", "Close the sendmail screen", "sendmail_close");
            g.ecl.register_button(x2, y, width, 15, "Clear", "Clear this screen", "sendmail_clear");
            g.ecl.register_button(x3, y, width, 15, "Send", "Send this email", "sendmail_send");

            g.ecl.register_button_callback("sendmail_close", Self::title_click);
            g.ecl.register_button_callback("sendmail_send", Self::send_click);
            g.ecl.register_button_callback("sendmail_clear", Self::clear_click);

            g.ecl.make_button_editable("sendmail_to");
            g.ecl.make_button_editable("sendmail_subject");
            g.ecl.make_button_editable("sendmail_body box");

            self.changed = true;
        }
    }

    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            // Update m with the captions which may have been changed
            self.m.set_to(&caption_of(g, "sendmail_to"));
            self.m.set_subject(&caption_of(g, "sendmail_subject"));
            self.m.set_body(&caption_of(g, "sendmail_body box"));

            g.ecl.remove_button("sendmail_title");

            g.ecl.remove_button("sendmail_tto");
            g.ecl.remove_button("sendmail_tsubject");

            g.ecl.remove_button("sendmail_to");
            g.ecl.remove_button("sendmail_subject");
            remove_stextbox(g, "sendmail_body");

            g.ecl.remove_button("sendmail_file");
            g.ecl.remove_button("sendmail_close");
            g.ecl.remove_button("sendmail_clear");
            g.ecl.remove_button("sendmail_send");

            if Self::is_visible_address_book(g) {
                Self::toggle_address_book(g, 0, 0);
            }
            if Self::is_visible_file_list(g) {
                Self::toggle_file_list(g, 0, 0);
            }
        }
    }

    fn update(&mut self, g: &mut Ctx) {
        if self.changed && self.is_visible(g) {
            // Update the interface buttons
            set_caption(g, "sendmail_to", &self.m.to);
            set_caption(g, "sendmail_subject", self.m.get_subject());
            set_caption(g, "sendmail_body box", self.m.get_body());

            let attached = match self.m.get_data() {
                Some(data) => format!("File: {}", data.title),
                None => "File: None attached".to_string(),
            };
            set_caption(g, "sendmail_file", &attached);

            self.changed = false;
        }
    }

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("sendmail_title").is_some()
    }

    fn screen_id(&self) -> i32 {
        SCREEN_SENDMAIL
    }

    impl_as_any!();
}
