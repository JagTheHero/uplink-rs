//! Port of source/interface/remoteinterface/linksscreen_interface.h + source/interface/remoteinterface/linksscreen_interface.cpp

use uplink_engine::{gl, text};

use crate::app::miscutils::lower_case_string;
use crate::app::opengl_interface::{
    border_draw, button_assignbitmaps, button_click, button_highlight, clear_draw, imagebutton_draw,
    set_colour, sy, textbutton_draw,
};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::game::data::data::IP_INTERNIC;
use crate::interface::interfacescreen::{InterfaceScreen, ScreenSlot};
use crate::interface::localinterface::phonedialler::PhoneDialler;
use crate::interface::remoteinterface::remoteinterface::{RemoteInterface, SCREEN_LINKSSCREEN};
use crate::interface::remoteinterface::remoteinterfacescreen::{draw_main_title, draw_sub_title};
use crate::interface::scrollbox::ScrollBox;
use crate::objref::ScreenRef;
use crate::world::computer::computerscreen::linksscreen::{
    LinksScreen, LINKSSCREENTYPE_ALLLINKS, LINKSSCREENTYPE_LOCALLINKS, LINKSSCREENTYPE_PLAYERLINKS,
};
use crate::world::connection::Connection;
use crate::world::person::Person;

/// The player's key in the world (`World::get_person("PLAYER")`).
const PLAYER: &str = "PLAYER";

/// `interface->GetRemoteInterface()->GetInterfaceScreen()` slot, for `ScreenSlot::call`.
fn ri_screen_slot(g: &mut Ctx) -> &mut ScreenSlot {
    &mut g.interface_mut().ri.screen
}

/// Runs `f` on the links screen with the screen taken out of its slot, so that `f` can use
/// `&mut self` and `g` together (C++ `GetInterfaceScreen()` + calls on `this`). `None` when no
/// links screen is showing.
fn with_links_screen<R>(
    g: &mut Ctx,
    f: impl FnOnce(&mut LinksScreenInterface, &mut Ctx) -> R,
) -> Option<R> {
    ScreenSlot::call(g, ri_screen_slot, |s, g| {
        let lsi = s.as_any_mut().downcast_mut::<LinksScreenInterface>()?;
        Some(f(lsi, g))
    })?
}

/// The number at the end of a row button's name (`"linksscreen_link 3"` -> 3), as the C++
/// `sscanf(button->name, "linksscreen_link %d", ..)` read it.
fn row_number(name: &str, prefix: &str) -> i32 {
    name.strip_prefix(prefix)
        .and_then(|s| s.trim().parse::<i32>().ok())
        .unwrap_or(0)
}

/// For a row button: the list index `row + baseoffset` (C++ `linkindex`) and the filtered link at
/// that index (C++ `filteredlist->ValidIndex(linkindex) ? GetData(linkindex) : NULL`).
fn row_link(g: &Ctx, name: &str, prefix: &str) -> (i32, Option<String>) {
    let row = row_number(name, prefix);
    let empty: &[String] = &[];
    let (base, list) = match g.interface().ri.screen.get::<LinksScreenInterface>() {
        Some(lsi) => (lsi.baseoffset, lsi.filteredlist.as_slice()),
        None => (0, empty),
    };
    let index = row + base;
    let link = usize::try_from(index).ok().and_then(|i| list.get(i)).cloned();
    (index, link)
}

/// Links screen: lists IP addresses (the player's links, all listed locations, or the local
/// company's locations) with an optional name filter, add/delete/show buttons and a scroll box.
#[derive(Debug, Default)]
pub struct LinksScreenInterface {
    pub cs: Option<ScreenRef>,
    // PORT: `baseoffset` was a `static` member in C++ (shared by every instance). It is kept per
    // instance here; `apply_filter` and `scroll_change` keep it in step with the scroll box.
    /// First link index shown in the list (scroll offset).
    pub baseoffset: i32,
    /// Every link for this screen, copied from the source list (C++ `fulllist`).
    pub fulllist: Vec<String>,
    /// The links that pass the current filter (C++ `filteredlist`).
    pub filteredlist: Vec<String>,
}

impl LinksScreenInterface {
    /// `NumLinksOnScreen` (static): rows that fit under the current screen height.
    pub fn num_links_on_screen(g: &Ctx) -> i32 {
        let screenheight = g.app.options.get_option_value("graphics_screenheight");
        let available_pixels = screenheight - 120 - 145;
        available_pixels / 15
    }

    /// `SetFullList(LList*)`: replaces `fulllist` with a copy of `new_full`, then applies no filter.
    pub fn set_full_list(&mut self, g: &mut Ctx, new_full: &[String]) {
        self.fulllist = new_full.to_vec();
        self.apply_filter(g, None);
    }

    /// `SetFullList()`: rebuilds `fulllist` from the current `filteredlist`, then applies no filter.
    pub fn set_full_list_from_filtered(&mut self, g: &mut Ctx) {
        self.fulllist = self.filteredlist.clone();
        self.apply_filter(g, None);
    }

    /// `ApplyFilter`: fills `filteredlist` from `fulllist` by case-insensitive computer name match
    /// (`None` keeps everything), resets the scroll offset, and dirties the rows.
    pub fn apply_filter(&mut self, g: &mut Ctx, filter: Option<&str>) {
        self.filteredlist.clear();

        match filter {
            Some(filter) => {
                let lowercasefilter = lower_case_string(filter);

                for ip in &self.fulllist {
                    // C++ dereferenced the VLocation directly; a missing one matches nothing here.
                    let computername = g
                        .world()
                        .get_vlocation(ip)
                        .map(|vl| vl.computer.as_str())
                        .unwrap_or("");
                    let lowercasename = lower_case_string(computername);

                    if lowercasename.contains(lowercasefilter.as_str()) {
                        self.filteredlist.push(ip.clone());
                    }
                }
            }
            None => {
                self.filteredlist = self.fulllist.clone();
            }
        }

        self.baseoffset = 0;

        //
        // Dirty the buttons
        //

        let num_links = Self::num_links_on_screen(g);
        let is_player_links = self
            .get_computer_screen(g)
            .is_some_and(|s| s.screentype == LINKSSCREENTYPE_PLAYERLINKS);

        for ib in 0..num_links {
            g.ecl.dirty_button(&format!("linksscreen_link {ib}"));

            if is_player_links {
                g.ecl.dirty_button(&format!("linksscreen_deletelink {ib}"));
                g.ecl.dirty_button(&format!("linksscreen_showlink {ib}"));
            } else {
                g.ecl.dirty_button(&format!("linksscreen_addlink {ib}"));
            }
        }

        //
        // Update the scrollbar
        //

        // C++ SetNumItems calls ScrollChange when it clamps the index. That callback cannot reach
        // this screen while it is running (the screen is taken out of its slot), so the new
        // offset is copied back here instead.
        let before = ScrollBox::get_scroll_box(g, "linksscreen_scroll").map(|sb| sb.current_index);
        ScrollBox::set_num_items(g, "linksscreen_scroll", self.filteredlist.len() as i32);
        let after = ScrollBox::get_scroll_box(g, "linksscreen_scroll").map(|sb| sb.current_index);
        if before != after {
            if let Some(index) = after {
                self.baseoffset = index;
            }
        }
    }

    /// `GetComputerScreen` (typed accessor; the C++ casts `cs`).
    pub fn get_computer_screen<'a>(&self, g: &'a Ctx) -> Option<&'a LinksScreen> {
        let cs = self.cs.as_ref()?;
        g.world()
            .get_computer(&cs.computer)?
            .get_computer_screen(cs.index)?
            .as_links_screen()
    }

    /// `LinkDraw` (static DrawFn): one row with the IP and computer name.
    pub fn link_draw(g: &mut Ctx, button: &Button, highlighted: bool, _clicked: bool) {
        clear_draw(g, button.x, button.y, button.width, button.height);

        let (linkindex, link) = row_link(g, &button.name, "linksscreen_link ");
        let Some(link) = link else { return };

        // Alternate the row shading
        if linkindex % 2 == 0 {
            gl::begin(gl::QUADS);
            set_colour(g, "DarkPanelB");
            gl::vertex2i(button.x, button.y);
            set_colour(g, "DarkPanelA");
            gl::vertex2i(button.x + button.width, button.y);
            set_colour(g, "DarkPanelB");
            gl::vertex2i(button.x + button.width, button.y + button.height);
            set_colour(g, "DarkPanelA");
            gl::vertex2i(button.x, button.y + button.height);
            gl::end();
        } else {
            gl::begin(gl::QUADS);
            set_colour(g, "DarkPanelA");
            gl::vertex2i(button.x, button.y);
            set_colour(g, "DarkPanelB");
            gl::vertex2i(button.x + button.width, button.y);
            set_colour(g, "DarkPanelA");
            gl::vertex2i(button.x + button.width, button.y + button.height);
            set_colour(g, "DarkPanelB");
            gl::vertex2i(button.x, button.y + button.height);
            gl::end();
        }

        set_colour(g, "DefaultText");

        // Draw the text

        text::draw_text_default(button.x + 10, button.y + 10, &link);

        let caption = match g.world().get_vlocation(&link) {
            Some(vl) => vl.computer.clone(),
            None => "This link has expired".to_string(),
        };

        text::draw_text_default(button.x + 120, button.y + 10, &caption);

        // Draw a bounding box
        if highlighted {
            set_colour(g, "PanelHighlightBorder");
            border_draw(button);
        }
    }

    /// `LinkClick` (static MouseFn): disconnects, dials the chosen IP, connects and runs the new location.
    pub fn link_click(g: &mut Ctx, button: &Button) {
        let (_, link) = row_link(g, &button.name, "linksscreen_link ");
        let Some(ip) = link else { return };

        Connection::disconnect(g, PLAYER);
        Connection::reset(g, PLAYER);

        let mut pd = PhoneDialler::default();
        pd.create_at(g, 100, 100);
        pd.dial_number(g, &ip);
        pd.remove(g);

        g.player_mut().connection.add_vlocation(&ip);
        Connection::connect(g, PLAYER);

        RemoteInterface::run_new_location(g);
    }

    /// `LinkMouseDown` (static MouseFn): only rows that hold a link respond.
    pub fn link_mouse_down(g: &mut Ctx, button: &Button) {
        let (_, link) = row_link(g, &button.name, "linksscreen_link ");
        if link.is_some() {
            button_click(g, button);
        }
    }

    /// `LinkMouseMove` (static MouseFn): only rows that hold a link highlight.
    pub fn link_mouse_move(g: &mut Ctx, button: &Button) {
        let (_, link) = row_link(g, &button.name, "linksscreen_link ");
        if link.is_some() {
            button_highlight(g, button);
        }
    }

    /// `FilterDraw` (static DrawFn): text box with its border.
    pub fn filter_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        textbutton_draw(g, button, highlighted, clicked);

        gl::color3f(1.0, 1.0, 1.0);
        border_draw(button);
    }

    /// `FilterClick` (static MouseFn): applies the text in the filter box.
    pub fn filter_click(g: &mut Ctx, _button: &Button) {
        let Some(filter) = g
            .ecl
            .get_button("linksscreen_filtertext")
            .map(|b| b.caption.clone())
        else {
            return;
        };

        with_links_screen(g, |lsi, g| {
            if filter.is_empty() {
                lsi.apply_filter(g, None);
            } else {
                lsi.apply_filter(g, Some(filter.as_str()));
            }
        });
    }

    /// `DeleteLinkDraw` (static DrawFn): shows the delete icon only on rows that hold a link.
    pub fn delete_link_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        let (_, link) = row_link(g, &button.name, "linksscreen_deletelink ");

        if link.is_some() {
            imagebutton_draw(g, button, highlighted, clicked);
        } else {
            clear_draw(g, button.x, button.y, button.width, button.height);
        }
    }

    /// `DeleteLinkClick` (static MouseFn): removes a player link (never the internic link), then re-filters.
    pub fn delete_link_click(g: &mut Ctx, button: &Button) {
        // Safe to delete this link
        // (This button would not exist if these were not player links)

        let (_, link) = row_link(g, &button.name, "linksscreen_deletelink ");
        let Some(link) = link else { return };

        if !g.player().has_link(&link) || link == IP_INTERNIC {
            return;
        }

        g.player_mut().remove_link(&link);

        let links = g.player().agent().links.clone();

        with_links_screen(g, |lsi, g| {
            let currentbaseoffset = lsi.baseoffset;
            lsi.set_full_list(g, &links);

            // Re-apply the filter
            match g
                .ecl
                .get_button("linksscreen_filtertext")
                .map(|b| b.caption.clone())
            {
                Some(filter) => {
                    if filter.is_empty() {
                        lsi.apply_filter(g, None);
                    } else {
                        lsi.apply_filter(g, Some(filter.as_str()));
                    }
                    lsi.baseoffset = currentbaseoffset;
                }
                None => lsi.apply_filter(g, None),
            }
        });
    }

    /// `AddLinkDraw` (static DrawFn): shows the add icon only for links the player does not hold.
    pub fn add_link_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        let (_, link) = row_link(g, &button.name, "linksscreen_addlink ");
        let show = link.is_some_and(|l| !g.player().has_link(&l));

        if show {
            imagebutton_draw(g, button, highlighted, clicked);
        } else {
            clear_draw(g, button.x, button.y, button.width, button.height);
        }
    }

    /// `AddLinkClick` (static MouseFn): gives the player the link.
    pub fn add_link_click(g: &mut Ctx, button: &Button) {
        let (_, link) = row_link(g, &button.name, "linksscreen_addlink ");

        if let Some(link) = link {
            if !g.player().has_link(&link) {
                Person::give_link(g, PLAYER, &link);
            }
        }
    }

    /// `ShowLinkDraw` (static DrawFn): shows the map toggle, lit when the location is displayed.
    pub fn show_link_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        let (_, link) = row_link(g, &button.name, "linksscreen_showlink ");

        // C++ dereferenced the VLocation; a missing one counts as not displayed here.
        let displayed = link
            .as_ref()
            .is_some_and(|l| g.world().get_vlocation(l).is_some_and(|vl| vl.displayed));

        if displayed {
            imagebutton_draw(g, button, highlighted, clicked);
        } else {
            clear_draw(g, button.x, button.y, button.width, button.height);
        }
    }

    /// `ShowLinkClick` (static MouseFn): toggles whether the location is displayed on the map.
    pub fn show_link_click(g: &mut Ctx, button: &Button) {
        let (_, link) = row_link(g, &button.name, "linksscreen_showlink ");

        if let Some(link) = link {
            if let Some(vl) = g.world_mut().get_vlocation_mut(&link) {
                let newdisplayed = !vl.displayed;
                vl.set_displayed(newdisplayed);
            }
        }
    }

    /// `CloseClick` (static MouseFn): runs the next page of the links screen.
    pub fn close_click(g: &mut Ctx, _button: &Button) {
        let nextpage = {
            let gr: &Ctx = &*g;
            gr.interface()
                .ri
                .screen
                .get::<LinksScreenInterface>()
                .and_then(|lsi| lsi.get_computer_screen(gr))
                .map(|cs| cs.nextpage)
        };

        if let Some(nextpage) = nextpage {
            if nextpage != -1 {
                RemoteInterface::run_screen(g, nextpage);
            }
        }
    }

    /// `ScrollChange` (scroll box callback): sets `baseoffset` and dirties the visible rows.
    pub fn scroll_change(g: &mut Ctx, _scrollname: &str, new_value: i32) {
        if let Some(lsi) = g.interface_mut().ri.screen.get_mut::<LinksScreenInterface>() {
            lsi.baseoffset = new_value;
        }

        let num_links = Self::num_links_on_screen(g);
        for i in 0..num_links {
            g.ecl.dirty_button(&format!("linksscreen_link {i}"));
            g.ecl.dirty_button(&format!("linksscreen_deletelink {i}"));
            g.ecl.dirty_button(&format!("linksscreen_addlink {i}"));
            g.ecl.dirty_button(&format!("linksscreen_showlink {i}"));
        }
    }
}

impl InterfaceScreen for LinksScreenInterface {
    crate::impl_as_any!();

    /// `Create(ComputerScreen *newcs)` body. The C++ no-argument `Create()` overload maps to this
    /// too, since `cs` is already set by `set_computer_screen`.
    fn create(&mut self, g: &mut Ctx) {
        let Some((maintitle, subtitle, screentype, nextpage, computer_name)) =
            self.get_computer_screen(g).map(|screen| {
                (
                    screen.base.maintitle.clone(),
                    screen.base.subtitle.clone(),
                    screen.screentype,
                    screen.nextpage,
                    screen.base.computer.clone(),
                )
            })
        else {
            println!("LinksScreenInterface::Create, tried to create when GenericScreen==NULL");
            return;
        };

        if self.is_visible(g) {
            return;
        }

        let is_player_links = screentype == LINKSSCREENTYPE_PLAYERLINKS;

        // Draw the screen titles

        g.ecl.register_button(80, 60, 350, 25, &maintitle, "", "linksscreen_maintitle");
        g.ecl.register_button_callbacks(
            "linksscreen_maintitle",
            Some(draw_main_title),
            None,
            None,
            None,
        );
        g.ecl.register_button(80, 80, 350, 20, &subtitle, "", "linksscreen_subtitle");
        g.ecl.register_button_callbacks(
            "linksscreen_subtitle",
            Some(draw_sub_title),
            None,
            None,
            None,
        );

        // Create the links

        let num_links = Self::num_links_on_screen(g);

        for li in 0..num_links {
            let name = format!("linksscreen_link {li}");
            g.ecl.register_button(
                30,
                145 + li * 15,
                sy(375),
                14,
                "",
                "Connect to this computer",
                &name,
            );
            g.ecl.register_button_callbacks(
                &name,
                Some(Self::link_draw),
                Some(Self::link_click),
                Some(Self::link_mouse_down),
                Some(Self::link_mouse_move),
            );

            if is_player_links {
                // Give the player the option to delete any link

                let name = format!("linksscreen_deletelink {li}");
                g.ecl.register_button(
                    15,
                    145 + li * 15,
                    13,
                    13,
                    "",
                    "Delete this link",
                    &name,
                );
                button_assignbitmaps(g, &name, "close.tif", "close_h.tif", "close_c.tif");
                g.ecl.register_button_callbacks(
                    &name,
                    Some(Self::delete_link_draw),
                    Some(Self::delete_link_click),
                    Some(button_click),
                    Some(button_highlight),
                );

                // Give the player the option to display the link on the map

                let displaylink = format!("linksscreen_showlink {li}");
                g.ecl.register_button(
                    34 + sy(375),
                    145 + li * 15,
                    13,
                    13,
                    "",
                    "Toggle the visibility of this link on the map",
                    &displaylink,
                );
                button_assignbitmaps(g, &displaylink, "link.tif", "link_h.tif", "link_c.tif");
                g.ecl.register_button_callbacks(
                    &displaylink,
                    Some(Self::show_link_draw),
                    Some(Self::show_link_click),
                    Some(button_click),
                    Some(button_highlight),
                );
            } else {
                // Give the player the option to add this link to his collection

                let name = format!("linksscreen_addlink {li}");
                g.ecl.register_button(15, 145 + li * 15, 13, 13, "", "Store this link", &name);
                button_assignbitmaps(g, &name, "add.tif", "add_h.tif", "add_c.tif");
                g.ecl.register_button_callbacks(
                    &name,
                    Some(Self::add_link_draw),
                    Some(Self::add_link_click),
                    Some(button_click),
                    Some(button_highlight),
                );
            }
        }

        //
        // Set the full list depending on the screen type
        //

        if is_player_links {
            let links = g.player().agent().links.clone();
            self.set_full_list(g, &links);
        } else if screentype == LINKSSCREENTYPE_ALLLINKS {
            // locations.ConvertToDArray: BTree pre-order (insertion-dependent), not sorted by ip
            let alllinks: Vec<String> = g
                .world()
                .locations
                .values()
                .filter(|loc| loc.listed)
                .map(|loc| loc.ip.clone())
                .collect();
            self.set_full_list(g, &alllinks);
        } else if screentype == LINKSSCREENTYPE_LOCALLINKS {
            let companyname = g
                .world()
                .get_computer(&computer_name)
                .expect("LinksScreenInterface::Create: screen computer missing")
                .companyname
                .clone();

            let alllinks: Vec<String> = g
                .world()
                .locations
                .values()
                .filter(|loc| !loc.is_special())
                .map(|loc| loc.ip.clone())
                .collect();
            self.set_full_list(g, &alllinks);
            self.apply_filter(g, Some(companyname.as_str()));
            self.set_full_list_from_filtered(g);
        }

        if self.fulllist.len() as i32 >= num_links {
            // Create the scrollbar

            ScrollBox::create_scroll_box(
                g,
                "linksscreen_scroll",
                50 + sy(375),
                145,
                15,
                num_links * 15,
                self.fulllist.len() as i32,
                num_links,
                0,
                Some(Self::scroll_change),
            );

            g.ecl
                .register_button(50 + sy(375), 125, 15, 15, "", "", "linksscreen_topright");

            // Create the filter buttons

            g.ecl.register_button(
                15,
                150 + num_links * 15,
                135,
                15,
                "Filter",
                "Click here to apply the filter",
                "linksscreen_filter",
            );
            g.ecl
                .register_button_callback("linksscreen_filter", Self::filter_click);

            let mut width = (65 + sy(375)) - 155;
            if nextpage != -1 {
                width -= 50;
            }

            g.ecl.register_button(
                155,
                150 + num_links * 15,
                width,
                15,
                "",
                "Enter filter pattern here",
                "linksscreen_filtertext",
            );
            g.ecl.register_button_callbacks(
                "linksscreen_filtertext",
                Some(Self::filter_draw),
                None,
                Some(button_click),
                Some(button_highlight),
            );
            g.ecl.make_button_editable("linksscreen_filtertext");
        }

        // Create the exit button

        if nextpage != -1 {
            let width = (65 + sy(375)) - 205;
            g.ecl.register_button(
                160 + width,
                150 + num_links * 15,
                45,
                15,
                "Close",
                "Close this screen",
                "linksscreen_close",
            );
            g.ecl
                .register_button_callback("linksscreen_close", Self::close_click);
        }

        // Create the title bars if this isn't the player links screen,
        // or if more than 13 links are on display

        if screentype != LINKSSCREENTYPE_PLAYERLINKS || self.fulllist.len() as i32 >= num_links {
            let xpos = 15;
            let width = 150 - xpos;
            g.ecl.register_button(
                xpos,
                125,
                width,
                15,
                "IP Address",
                "Shows the IP address of the location",
                "linksscreen_iptitle",
            );
            g.ecl.register_button(
                155,
                125,
                (48 + sy(375)) - 155,
                15,
                "Location name",
                "Shows the computer name at that location",
                "linksscreen_comptitle",
            );
        }

        self.apply_filter(g, None);
    }

    /// `Remove`: also removes the scroll box and filter controls when they were created.
    fn remove(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            return;
        }

        g.ecl.remove_button("linksscreen_maintitle");
        g.ecl.remove_button("linksscreen_subtitle");

        g.ecl.remove_button("linksscreen_iptitle");
        g.ecl.remove_button("linksscreen_comptitle");

        let num_links = Self::num_links_on_screen(g);
        let is_player_links = self
            .get_computer_screen(g)
            .is_some_and(|s| s.screentype == LINKSSCREENTYPE_PLAYERLINKS);

        for i in 0..num_links {
            g.ecl.remove_button(&format!("linksscreen_link {i}"));

            if is_player_links {
                g.ecl.remove_button(&format!("linksscreen_deletelink {i}"));
                g.ecl.remove_button(&format!("linksscreen_showlink {i}"));
            } else {
                g.ecl.remove_button(&format!("linksscreen_addlink {i}"));
            }
        }

        if ScrollBox::get_scroll_box(g, "linksscreen_scroll").is_some() {
            ScrollBox::remove_scroll_box(g, "linksscreen_scroll");

            g.ecl.remove_button("linksscreen_topright");
            g.ecl.remove_button("linksscreen_filter");
            g.ecl.remove_button("linksscreen_filtertext");
        }

        g.ecl.remove_button("linksscreen_close");
    }

    fn update(&mut self, _g: &mut Ctx) {}

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("linksscreen_maintitle").is_some()
    }

    /// `ReturnKeyPressed`: applies the text in the filter box, if there is one.
    fn return_key_pressed(&mut self, g: &mut Ctx) -> bool {
        let Some(filter) = g
            .ecl
            .get_button("linksscreen_filtertext")
            .map(|b| b.caption.clone())
        else {
            return false;
        };

        if filter.is_empty() {
            self.apply_filter(g, None);
        } else {
            self.apply_filter(g, Some(filter.as_str()));
        }
        true
    }

    fn screen_id(&self) -> i32 {
        SCREEN_LINKSSCREEN
    }

    fn set_computer_screen(&mut self, cs: ScreenRef) {
        self.cs = Some(cs);
    }

    fn computer_screen_ref(&self) -> Option<&ScreenRef> {
        self.cs.as_ref()
    }
}
