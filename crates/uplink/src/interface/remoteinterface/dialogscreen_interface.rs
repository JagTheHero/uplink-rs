//! Port of source/interface/remoteinterface/dialogscreen_interface.h + source/interface/remoteinterface/dialogscreen_interface.cpp

use uplink_engine::{gl, text};

use crate::app::globals::ALPHA;
use crate::app::opengl::screen_h;
use crate::app::opengl_interface::{
    border_draw, button_assignbitmaps, button_click, button_draw, button_highlight, clear_draw,
    textbutton_draw,
};
use crate::ctx::Ctx;
use crate::eclipse::{Button, Eclipse};
use crate::game::scriptlibrary::ScriptLibrary;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::remoteinterface::remoteinterface::{RemoteInterface, SCREEN_DIALOGSCREEN};
use crate::interface::remoteinterface::remoteinterfacescreen::{draw_main_title, draw_sub_title};
use crate::objref::ScreenRef;
use crate::world::computer::computerscreen::dialogscreen::{
    DialogScreen, DialogScreenWidget, WIDGET_BASIC, WIDGET_CAPTION, WIDGET_FIELDVALUE,
    WIDGET_IMAGE, WIDGET_IMAGEBUTTON, WIDGET_NEXTPAGE, WIDGET_NONE, WIDGET_PASSWORDBOX,
    WIDGET_SCRIPTBUTTON, WIDGET_TEXTBOX,
};

/// Dialog screen: a page built from data-driven widgets (text boxes, password boxes, page and
/// script buttons, record fields) described by the computer's `DialogScreen`.
#[derive(Debug, Default)]
pub struct DialogScreenInterface {
    pub cs: Option<ScreenRef>,
}

/// The eclipse button name of a widget: `"%s %d %d"` with the widget name and its two data values.
fn widget_button_name(dsw: &DialogScreenWidget) -> String {
    format!("{} {} {}", dsw.name, dsw.data1, dsw.data2)
}

/// `sscanf(button->name, "%s %d %d", bname, &data1, &data2)`: the two numbers after the name.
/// A missing or bad number leaves the C++ variable uninitialised; -1 ("no page / no script") is used here.
fn parse_name_data(name: &str) -> (i32, i32) {
    let mut parts = name.split_whitespace().skip(1);
    let data1 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(-1);
    let data2 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(-1);
    (data1, data2)
}

/// `Button *button` set to the caption of an eclipse button, if it exists.
fn set_caption(g: &mut Ctx, name: &str, caption: &str) {
    if let Some(b) = g.ecl.get_button_mut(name) {
        b.set_caption(caption);
    }
}

impl DialogScreenInterface {
    /// `PasswordBoxDraw` (static DrawFn): draws the text as asterisks.
    pub fn password_box_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        let screenheight = screen_h();
        gl::scissor(button.x, screenheight - (button.y + button.height), button.width, button.height);
        gl::enable(gl::SCISSOR_TEST);

        clear_draw(g, button.x, button.y, button.width, button.height);

        gl::color4f(1.0, 1.0, 1.0, ALPHA);

        // Print the text: one asterisk per character of the caption

        let caption = "*".repeat(button.caption.len());
        text::draw_text_styled(button.x + 10, button.y + 10, &caption, text::BITMAP_15);

        // Draw a box around the text if highlighted

        if highlighted || clicked {
            border_draw(button);
        }

        gl::disable(gl::SCISSOR_TEST);
    }

    /// `NextPageClick` (static MouseFn): parses "name page _" and runs that remote screen.
    pub fn next_page_click(g: &mut Ctx, button: &Button) {
        let (nextpage, _) = parse_name_data(&button.name);
        if nextpage != -1 {
            RemoteInterface::run_screen(g, nextpage);
        }
    }

    /// `ScriptButtonClick` (static MouseFn): parses "name script page" and runs the script, then the page.
    pub fn script_button_click(g: &mut Ctx, button: &Button) {
        let (scriptindex, nextpage) = parse_name_data(&button.name);

        if scriptindex != -1 {
            ScriptLibrary::run_script(g, scriptindex);
        }

        if nextpage != -1 {
            RemoteInterface::run_screen(g, nextpage);
        }
    }

    /// `GetComputerScreen` (typed accessor; the C++ casts `cs`).
    pub fn get_computer_screen<'a>(&self, g: &'a Ctx) -> Option<&'a DialogScreen> {
        let r = self.cs.as_ref()?;
        g.world().get_computer(&r.computer)?.get_computer_screen(r.index)?.as_dialog_screen()
    }

    /// `ReturnKeyPressed` / `EscapeKeyPressed` body: presses the widget named by `key`.
    /// Returns false if there is no such key or no widget with that name (the C++ `return false`).
    fn press_key_widget(&self, g: &mut Ctx, key: Option<String>) -> bool {
        let Some(key) = key else { return false };
        let target = self
            .get_computer_screen(g)
            .and_then(|ds| ds.widgets.iter().find(|w| w.name == key).map(widget_button_name));
        let Some(name) = target else { return false };
        Eclipse::button_mouse_up(g, &name);
        true
    }
}

impl InterfaceScreen for DialogScreenInterface {
    crate::impl_as_any!();

    /// `Create(ComputerScreen *newcs)` body (the `cs` is already set by `set_computer_screen`).
    fn create(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            return;
        }

        // A snapshot of the screen: the widget loop registers buttons on `g` while it runs.
        let Some(ds) = self.get_computer_screen(g).cloned() else { return };

        g.ecl.register_button(0, 0, 0, 0, " ", " ", "dialogscreen");
        g.ecl.register_button_callbacks("dialogscreen", None, None, None, None);

        g.ecl.register_button(80, 60, 350, 25, &ds.base.maintitle, "", "dialogscreen_maintitle");
        g.ecl.register_button_callbacks("dialogscreen_maintitle", Some(draw_main_title), None, None, None);
        g.ecl.register_button(80, 80, 350, 20, &ds.base.subtitle, "", "dialogscreen_subtitle");
        g.ecl.register_button_callbacks("dialogscreen_subtitle", Some(draw_sub_title), None, None, None);

        // Draw each widget in this dialogscreen

        for dsw in &ds.widgets {
            let name = widget_button_name(dsw);

            g.ecl.register_button(dsw.x, dsw.y, dsw.width, dsw.height, dsw.get_caption(), dsw.get_tooltip(), &name);

            // Customize the widget depending on its type

            match dsw.widgettype {
                WIDGET_BASIC => {
                    g.ecl.register_button_callbacks(&name, Some(button_draw), None, None, None);
                }
                WIDGET_CAPTION => {
                    g.ecl.register_button_callbacks(&name, Some(textbutton_draw), None, None, None);
                }
                WIDGET_TEXTBOX => {
                    g.ecl.register_button_callbacks(
                        &name,
                        Some(textbutton_draw),
                        None,
                        Some(button_click),
                        Some(button_highlight),
                    );
                    g.ecl.make_button_editable(&name);
                }
                WIDGET_PASSWORDBOX => {
                    g.ecl.register_button_callbacks(
                        &name,
                        Some(Self::password_box_draw),
                        None,
                        Some(button_click),
                        Some(button_highlight),
                    );
                    g.ecl.make_button_editable(&name);
                }
                WIDGET_NEXTPAGE => {
                    g.ecl.register_button_callbacks(
                        &name,
                        Some(button_draw),
                        Some(Self::next_page_click),
                        Some(button_click),
                        Some(button_highlight),
                    );
                }
                // PORT: a missing string (C++ NULL) is passed on as an empty name.
                WIDGET_IMAGE => {
                    let s1 = dsw.get_string_data1().unwrap_or("");
                    let s2 = dsw.get_string_data2().unwrap_or("");
                    button_assignbitmaps(g, &name, s1, s2, s2);
                }
                WIDGET_IMAGEBUTTON => {
                    let s1 = dsw.get_string_data1().unwrap_or("");
                    let s2 = dsw.get_string_data2().unwrap_or("");
                    button_assignbitmaps(g, &name, s1, s2, s2);
                    g.ecl.register_button_callback(&name, Self::next_page_click);
                }
                WIDGET_SCRIPTBUTTON => {
                    g.ecl.register_button_callbacks(
                        &name,
                        Some(button_draw),
                        Some(Self::script_button_click),
                        Some(button_click),
                        Some(button_highlight),
                    );
                }
                WIDGET_FIELDVALUE => {
                    g.ecl.register_button_callbacks(&name, Some(textbutton_draw), None, None, None);
                    g.ecl.make_button_editable(&name);

                    // The value of the field from the record of the logged-in user (if any)
                    let security_name = g.interface().ri.security_name.clone();
                    let value = {
                        let world = g.world();
                        dsw.get_string_data1().and_then(|field| {
                            world
                                .get_computer(&ds.base.computer)?
                                .recordbank
                                .get_record_from_name(&security_name)?
                                .get_field(field)
                                .map(str::to_string)
                        })
                    };
                    if let Some(value) = value {
                        set_caption(g, &name, &value);
                    }
                }
                WIDGET_NONE => {
                    println!("Error : DialogScreenInterface::Create, WIDGETTYPE==NONE");
                    g.ecl.remove_button(&name);
                }
                _ => {}
            }
        }
    }

    fn remove(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            return;
        }

        g.ecl.remove_button("dialogscreen");

        g.ecl.remove_button("dialogscreen_maintitle");
        g.ecl.remove_button("dialogscreen_subtitle");

        let names: Vec<String> = self
            .get_computer_screen(g)
            .map(|ds| ds.widgets.iter().map(widget_button_name).collect())
            .unwrap_or_default();
        for name in names {
            g.ecl.remove_button(&name);
        }
    }

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("dialogscreen").is_some()
    }

    /// `ReturnKeyPressed`: presses the widget named by the screen's return key button.
    fn return_key_pressed(&mut self, g: &mut Ctx) -> bool {
        let key = self.get_computer_screen(g).and_then(|ds| ds.returnkeybutton.clone());
        self.press_key_widget(g, key)
    }

    /// `EscapeKeyPressed`: presses the widget named by the screen's escape key button.
    fn escape_key_pressed(&mut self, g: &mut Ctx) -> bool {
        let key = self.get_computer_screen(g).and_then(|ds| ds.escapekeybutton.clone());
        self.press_key_widget(g, key)
    }

    fn screen_id(&self) -> i32 {
        SCREEN_DIALOGSCREEN
    }

    fn set_computer_screen(&mut self, cs: ScreenRef) {
        self.cs = Some(cs);
    }

    fn computer_screen_ref(&self) -> Option<&ScreenRef> {
        self.cs.as_ref()
    }
}
