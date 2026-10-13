//! Port of source/interface/remoteinterface/messagescreen_interface.h + source/interface/remoteinterface/messagescreen_interface.cpp

use crate::app::opengl_interface;
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::remoteinterface::remoteinterface::{RemoteInterface, SCREEN_MESSAGESCREEN};
use crate::interface::remoteinterface::remoteinterfacescreen::{draw_main_title, draw_sub_title};
use crate::objref::ScreenRef;
use crate::world::computer::computerscreen::messagescreen::MessageScreen;
use crate::world::message::Message;

/// Remote message screen: shows a message and optionally lets the player mail it.
#[derive(Debug, Default)]
pub struct MessageScreenInterface {
    pub cs: Option<ScreenRef>,
}

/// `sscanf ( name, "<word> %d" )`: the integer after the first word of a button name.
fn name_int(name: &str) -> Option<i32> {
    name.split_whitespace().nth(1)?.parse().ok()
}

impl MessageScreenInterface {
    /// `GetComputerScreen` (typed accessor; the C++ casts `cs`).
    pub fn get_computer_screen<'a>(&self, g: &'a Ctx) -> Option<&'a MessageScreen> {
        let cs = self.cs.as_ref()?;
        g.world().get_computer(&cs.computer)?.get_computer_screen(cs.index)?.as_message_screen()
    }

    /// `Click` (static MouseFn)
    pub fn click(g: &mut Ctx, button: &Button) {
        let Some(nextpage) = name_int(&button.name) else {
            return;
        };

        if nextpage != -1 {
            RemoteInterface::run_screen(g, nextpage);
        }
    }

    /// `MailMeClick` (static MouseFn)
    pub fn mail_me_click(g: &mut Ctx, _button: &Button) {
        let Some(me) = g.interface().ri.screen.get::<Self>() else {
            return;
        };
        let Some(msi) = me.get_computer_screen(g) else {
            debug_assert!(false, "MessageScreenInterface::mail_me_click, no MessageScreen");
            return;
        };

        let mut m = Message::new();
        m.set_to("PLAYER");
        m.set_from(&me.cs.as_ref().map(|cs| cs.computer.as_str()).unwrap_or_default());
        m.set_subject(&msi.base.subtitle);
        m.set_body(msi.textmessage.as_deref().unwrap_or_default());
        m.send(g);
    }
}

impl InterfaceScreen for MessageScreenInterface {
    crate::impl_as_any!();

    fn create(&mut self, g: &mut Ctx) {
        if self.cs.is_none() {
            println!("MessageScreenInterface::Create, tried to create when MessageScreen==NULL");
            return;
        }

        if !self.is_visible(g) {
            let Some(ms) = self.get_computer_screen(g) else {
                debug_assert!(false, "MessageScreenInterface::create, no MessageScreen");
                return;
            };
            let maintitle = ms.base.maintitle.clone();
            let subtitle = ms.base.subtitle.clone();
            let textmessage = ms.textmessage.clone().unwrap_or_default();
            let buttonmessage = ms.buttonmessage.clone();
            let nextpage = ms.nextpage;
            let mailthistome = ms.mailthistome;

            g.ecl.register_button(80, 60, 350, 25, &maintitle, "", "messagescreen_maintitle");
            g.ecl.register_button_callbacks("messagescreen_maintitle", Some(draw_main_title), None, None, None);
            g.ecl.register_button(80, 80, 350, 20, &subtitle, "", "messagescreen_subtitle");
            g.ecl.register_button_callbacks("messagescreen_subtitle", Some(draw_sub_title), None, None, None);

            g.ecl.register_button(50, 120, 400, 245, "", "", "messagescreen_message");
            g.ecl.register_button_callbacks("messagescreen_message", Some(opengl_interface::textbutton_draw), None, None, None);
            g.ecl.register_caption_change("messagescreen_message", &textmessage, 2000, None);

            if let Some(buttonmessage) = &buttonmessage {
                let width = 20 + buttonmessage.len() as i32 * 10;
                let name = format!("messagescreen_click {}", nextpage);
                g.ecl.register_button(320 - width / 2, 370, width, 20, buttonmessage, buttonmessage, &name);
                g.ecl.register_button_callback(&name, Self::click);
            }

            if mailthistome {
                let name = format!("messagescreen_click {}", nextpage);
                let Some((bx, by)) = g.ecl.get_button(&name).map(|b| (b.x, b.y)) else {
                    debug_assert!(false, "MessageScreenInterface::create, button {name} not found");
                    return;
                };

                g.ecl.register_button(
                    bx - 120,
                    by,
                    100,
                    20,
                    "Mail this to me",
                    "Click to send this information to yourself in an email",
                    "messagescreen_mailme",
                );
                g.ecl.register_button_callback("messagescreen_mailme", Self::mail_me_click);
            }
        }
    }

    fn remove(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            let Some(nextpage) = self.get_computer_screen(g).map(|ms| ms.nextpage) else {
                debug_assert!(false, "MessageScreenInterface::remove, no MessageScreen");
                return;
            };

            g.ecl.remove_button("messagescreen_message");

            g.ecl.remove_button("messagescreen_maintitle");
            g.ecl.remove_button("messagescreen_subtitle");

            let name = format!("messagescreen_click {}", nextpage);
            g.ecl.remove_button(&name);

            g.ecl.remove_button("messagescreen_mailme");
        }
    }

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("messagescreen_message").is_some()
    }

    fn screen_id(&self) -> i32 {
        SCREEN_MESSAGESCREEN
    }

    fn return_key_pressed(&mut self, g: &mut Ctx) -> bool {
        let nextpage = self.get_computer_screen(g).map(|ms| ms.nextpage);
        if let Some(nextpage) = nextpage {
            if nextpage != -1 {
                RemoteInterface::run_screen(g, nextpage);
            }
        }

        true
    }

    fn set_computer_screen(&mut self, cs: ScreenRef) {
        self.cs = Some(cs);
    }

    fn computer_screen_ref(&self) -> Option<&ScreenRef> {
        self.cs.as_ref()
    }
}
