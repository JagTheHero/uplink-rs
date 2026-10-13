//! Port of source/interface/remoteinterface/newsscreen_interface.h + source/interface/remoteinterface/newsscreen_interface.cpp

use uplink_engine::{gl, text};

use crate::app::opengl_interface::{
    border_draw, button_click, button_highlight, clear_draw, create_stextbox, remove_stextbox, set_colour, sx, sy,
    wordwraptext,
};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::remoteinterface::remoteinterface::{RemoteInterface, SCREEN_NEWSSCREEN};
use crate::interface::scrollbox::ScrollBox;
use crate::objref::ScreenRef;
use crate::world::company::companyuplink::CompanyUplink;
use crate::world::computer::computerscreen::genericscreen::GenericScreen;

/// Remote news screen: list of news items with a details pane.
#[derive(Debug)]
pub struct NewsScreenInterface {
    pub cs: Option<ScreenRef>,
    pub baseoffset: i32,
    /// Selected news item (-1 when none); reset by `create`.
    pub currentselect: i32,
    pub previousnummessages: i32,
}

impl Default for NewsScreenInterface {
    fn default() -> Self {
        NewsScreenInterface {
            cs: None,
            baseoffset: 0,
            currentselect: -1,
            previousnummessages: 0,
        }
    }
}

/// Name of news row `i` (`"news_story %d"` in C++).
fn story_name(i: i32) -> String {
    format!("news_story {i}")
}

/// `sscanf(name, "news_story %d", &index)`. PORT: C++ leaves `index` unset when the name does not
/// match; 0 is used instead. Every caller passes a row button name.
fn scan_story(name: &str) -> i32 {
    name.strip_prefix("news_story ")
        .and_then(|rest| rest.trim_start().parse::<i32>().ok())
        .unwrap_or(0)
}

/// Number of news rows shown at once (`SY(260) / 40` in C++).
fn num_rows() -> i32 {
    sy(260) / 40
}

/// The shown screen as this type, mutably.
fn screen_mut(g: &mut Ctx) -> Option<&mut NewsScreenInterface> {
    g.interface_mut().ri.screen.get_mut::<NewsScreenInterface>()
}

/// `(baseoffset, currentselect)` of the shown screen. PORT: C++ keeps these as statics; callers
/// fall back to their initial values when no such screen is shown.
fn selection(g: &Ctx) -> Option<(i32, i32)> {
    g.interface()
        .ri
        .screen
        .get::<NewsScreenInterface>()
        .map(|s| (s.baseoffset, s.currentselect))
}

/// The Uplink company's news (`GetCompany("Uplink")`, C++ `UplinkAssert`).
fn uplink_company(g: &Ctx) -> Option<&CompanyUplink> {
    g.world().get_company("Uplink")?.uplink.as_deref()
}

/// One `glBegin(GL_QUADS)` block: four corners, each drawn in its own named theme colour.
fn quad_corners(g: &Ctx, corners: [(&str, i32, i32); 4]) {
    gl::begin(gl::QUADS);
    for (colour, x, y) in corners {
        set_colour(g, colour);
        gl::vertex2i(x, y);
    }
    gl::end();
}

/// The `ExitClick` body: runs the generic screen's next page.
fn exit_to_next_page(g: &mut Ctx) {
    let Some(next) = RemoteInterface::get_computer_screen(g)
        .and_then(|cs| cs.as_generic_screen())
        .map(|gs| gs.nextpage)
    else {
        return;
    };
    RemoteInterface::run_screen(g, next);
}

impl NewsScreenInterface {
    /// `GetComputerScreen` (typed accessor; the C++ casts `cs`).
    pub fn get_computer_screen<'a>(&self, g: &'a Ctx) -> Option<&'a GenericScreen> {
        let cs = self.cs.as_ref()?;
        g.world()
            .get_computer(&cs.computer)?
            .get_computer_screen(cs.index)?
            .as_generic_screen()
    }

    /// `DrawNewsButton` (static DrawFn)
    pub fn draw_news_button(g: &mut Ctx, button: &Button, highlighted: bool, _clicked: bool) {
        let (baseoffset, currentselect) = selection(g).unwrap_or((0, -1));
        let index = scan_story(&button.name) + baseoffset;

        clear_draw(g, button.x, button.y, button.width, button.height);

        // Get the text from news item (index + baseoffset)
        let Some(news) = uplink_company(g).and_then(|cu| cu.get_news(index)) else { return };

        let screenheight = g.app.options.get_option_value("graphics_screenheight");
        let (x, y, w, h) = (button.x, button.y, button.width, button.height);
        gl::scissor(x, screenheight - (y + h), w, h);
        gl::enable(gl::SCISSOR_TEST);

        if index == currentselect {
            quad_corners(
                g,
                [
                    ("PanelHighlightA", x, y),
                    ("PanelHighlightB", x + w, y),
                    ("PanelHighlightA", x + w, y + h),
                    ("PanelHighlightB", x, y + h),
                ],
            );
        } else if index % 2 == 0 {
            quad_corners(
                g,
                [
                    ("DarkPanelA", x, y + h),
                    ("DarkPanelB", x, y),
                    ("DarkPanelA", x + w, y),
                    ("DarkPanelB", x + w, y + h),
                ],
            );
        } else {
            quad_corners(
                g,
                [
                    ("DarkPanelB", x, y + h),
                    ("DarkPanelA", x, y),
                    ("DarkPanelB", x + w, y),
                    ("DarkPanelA", x + w, y + h),
                ],
            );
        }

        if highlighted {
            set_colour(g, "PanelHighlightBorder");
            border_draw(button);
        }

        // Get the date and subject
        let subject = news.headline.clone();
        let date = news.date.get_short_string();

        // Get the first line of the news story: up to the first newline, at most 255 bytes.
        // Without a newline the C++ shows a single space.
        let full_details = news.details.as_deref().unwrap_or("");
        let details_line = match full_details.find('\n') {
            Some(pos) => {
                let end = pos.min(255);
                String::from_utf8_lossy(&full_details.as_bytes()[..end]).into_owned()
            }
            None => " ".to_string(),
        };

        // Draw the text items
        set_colour(g, "DefaultText");
        text::draw_text_default(x + 110, y + 10, &subject);
        text::draw_text_default(x + 5, y + 10, &date);

        set_colour(g, "DimmedText");
        text::draw_text_default(x + 110, y + 25, &details_line);

        gl::disable(gl::SCISSOR_TEST);
    }

    /// `ClickNewsButton` (static MouseFn)
    pub fn click_news_button(g: &mut Ctx, button: &Button) {
        let (baseoffset, currentselect) = selection(g).unwrap_or((0, -1));
        let index = scan_story(&button.name) + baseoffset;

        // Dirty the old button
        g.ecl.dirty_button(&story_name(currentselect - baseoffset));

        let details = uplink_company(g)
            .and_then(|cu| cu.get_news(index))
            .map(|news| news.details.clone().unwrap_or_default());
        let Some(details) = details else { return };

        if let Some(state) = screen_mut(g) {
            state.currentselect = index;
        }

        // Reset the offset so the player can read it from line 1
        if ScrollBox::get_scroll_box(g, "news_details").is_some() {
            ScrollBox::set_current_index(g, "news_details", 0);

            let Some(width) = g.ecl.get_button("news_details box").map(|b| b.width) else { return };
            let wrapped_len = wordwraptext(&details, width).len() as i32;
            ScrollBox::set_num_items(g, "news_details", wrapped_len);
        }

        g.ecl.register_caption_change("news_details box", &details, 2000, None);
    }

    /// `MouseDownNewsButton` (static MouseFn)
    pub fn mousedown_news_button(g: &mut Ctx, button: &Button) {
        let baseoffset = selection(g).map_or(0, |s| s.0);
        let index = scan_story(&button.name);
        let exists = uplink_company(g)
            .and_then(|cu| cu.get_news(index + baseoffset))
            .is_some();
        if exists {
            button_click(g, button);
        }
    }

    /// `HighlightNewsButton` (static MouseFn)
    pub fn highlight_news_button(g: &mut Ctx, button: &Button) {
        let baseoffset = selection(g).map_or(0, |s| s.0);
        let index = scan_story(&button.name);
        let exists = uplink_company(g)
            .and_then(|cu| cu.get_news(index + baseoffset))
            .is_some();
        if exists {
            button_highlight(g, button);
        }
    }

    /// `DrawDetails` (static DrawFn): the scrollable details text of the selected story.
    pub fn draw_details(g: &mut Ctx, button: &Button, _highlighted: bool, _clicked: bool) {
        let screenheight = g.app.options.get_option_value("graphics_screenheight");
        let (x, y, w, h) = (button.x, button.y, button.width, button.height);
        gl::scissor(x, screenheight - (y + h), w, h);
        gl::enable(gl::SCISSOR_TEST);

        // Get the offset (name_base is the first word of the button name)
        let name_base = button.name.split_whitespace().next().unwrap_or("");
        let Some(scroll_box) = ScrollBox::get_scroll_box(g, name_base) else { return };
        let offset = scroll_box.current_index;

        // Draw the button
        quad_corners(
            g,
            [
                ("PanelBackgroundA", x, y + h),
                ("PanelBackgroundB", x, y),
                ("PanelBackgroundA", x + w, y),
                ("PanelBackgroundB", x + w, y + h),
            ],
        );

        set_colour(g, "PanelBorder");
        border_draw(button);

        // Draw the text
        let maxnumlines = (h - 10) / 15;

        set_colour(g, "DefaultText");

        let wrapped = wordwraptext(&button.caption, w);
        for i in offset.max(0)..wrapped.len() as i32 {
            if i > maxnumlines + offset {
                break;
            }
            let xpos = x + 10;
            let ypos = y + 10 + (i - offset) * 15;
            text::draw_text_styled(xpos, ypos, &wrapped[i as usize], text::HELVETICA_10);
        }

        gl::disable(gl::SCISSOR_TEST);
    }

    /// `ExitClick` (static MouseFn)
    pub fn exit_click(g: &mut Ctx, _button: &Button) {
        exit_to_next_page(g);
    }

    /// `ScrollChange` (ScrollBox callback: scroll bar name and new value)
    pub fn scroll_change(g: &mut Ctx, _scrollname: &str, new_value: i32) {
        if let Some(state) = screen_mut(g) {
            state.baseoffset = new_value;
        }
        for i in 0..num_rows() {
            g.ecl.dirty_button(&story_name(i));
        }
    }
}

impl InterfaceScreen for NewsScreenInterface {
    crate::impl_as_any!();

    fn create(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            return;
        }

        let rows = num_rows();
        let item_width = sx(390);

        g.ecl.register_button(20, 34, 104, 15, "Date", "This column shows the date the story was posted", "news_date");
        g.ecl.register_button(125, 34, item_width - 105 + 15, 15, "Subject", "This column shows the subject of the story", "news_subject");

        for i in 0..rows {
            let name = story_name(i);
            g.ecl.register_button(20, i * 40 + 50, item_width, 39, "", "Read this News story", &name);
            g.ecl.register_button_callbacks(
                &name,
                Some(Self::draw_news_button),
                Some(Self::click_news_button),
                Some(Self::mousedown_news_button),
                Some(Self::highlight_news_button),
            );
        }

        create_stextbox(g, 20, 50 + rows * 40, item_width + 16, sy(105), "", "news_details");
        g.ecl.register_button_callbacks("news_details box", Some(Self::draw_details), None, None, None);

        g.ecl.register_button(
            20 + item_width + 16 - sx(70),
            51 + rows * 40 + sy(105),
            sx(70),
            15,
            "Exit News",
            "Close the News screen and return to the main menu",
            "news_exit",
        );
        g.ecl.register_button_callback("news_exit", Self::exit_click);

        let num_news = uplink_company(g).map_or(0, |cu| cu.news.len() as i32);
        ScrollBox::create_scroll_box(
            g,
            "news_scroll",
            item_width + 21,
            50,
            15,
            rows * 40,
            num_news,
            rows,
            0,
            Some(Self::scroll_change),
        );

        self.baseoffset = 0;
        self.currentselect = -1;
    }

    fn remove(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            return;
        }

        for i in 0..num_rows() {
            g.ecl.remove_button(&story_name(i));
        }

        remove_stextbox(g, "news_details");

        g.ecl.remove_button("news_date");
        g.ecl.remove_button("news_subject");
        g.ecl.remove_button("news_exit");

        ScrollBox::remove_scroll_box(g, "news_scroll");
    }

    fn update(&mut self, g: &mut Ctx) {
        let Some(new_num) = uplink_company(g).map(|cu| cu.news.len() as i32) else { return };

        if new_num != self.previousnummessages {
            for i in 0..num_rows() {
                g.ecl.dirty_button(&story_name(i));
            }

            ScrollBox::set_num_items(g, "news_scroll", new_num);
            self.previousnummessages = new_num;
        }
    }

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("news_exit").is_some()
    }

    fn screen_id(&self) -> i32 {
        SCREEN_NEWSSCREEN
    }

    /// `EscapeKeyPressed`: same as the exit button, and always reports the key as handled.
    fn escape_key_pressed(&mut self, g: &mut Ctx) -> bool {
        if let Some(next) = self.get_computer_screen(g).map(|gs| gs.nextpage) {
            RemoteInterface::run_screen(g, next);
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
