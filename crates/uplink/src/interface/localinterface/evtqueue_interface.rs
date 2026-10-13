//! Port of source/interface/localinterface/evtqueue_interface.h + source/interface/localinterface/evtqueue_interface.cpp
//! Debug screen for watching the world's event queue (`scheduler.events`).

use std::cell::Cell;

use uplink_engine::{gl, text};

use crate::app::opengl_interface::{
    button_assignbitmaps, button_click, button_draw, button_highlight, clear_draw, create_stextbox, imagebutton_draw,
    remove_stextbox,
};
use crate::ctx::Ctx;
use crate::eclipse::Button;
use crate::impl_as_any;
use crate::interface::interfacescreen::InterfaceScreen;
use crate::interface::localinterface::localinterface::{LocalInterface, PANELSIZE, SCREEN_EVTQUEUE, SCREEN_NONE};
use crate::world::scheduler::uplinkevent::UplinkEvent;

thread_local! {
    /// C++ `static int EventQueueInterface::baseoffset` (shared by every instance, as in C++).
    static BASE_OFFSET: Cell<i32> = const { Cell::new(0) };
    /// C++ `static int EventQueueInterface::previousupdate` (never written by the C++, so always 0).
    static PREVIOUS_UPDATE: Cell<i32> = const { Cell::new(0) };
}

#[derive(Debug, Clone, Default)]
pub struct EventQueueInterface {}

/// C++ `sscanf(button->name, "<prefix> %d", &index)`: the number that follows `prefix` in a button name.
fn parse_index(name: &str, prefix: &str) -> i32 {
    name.strip_prefix(prefix)
        .and_then(|rest| rest.trim_start().parse::<i32>().ok())
        .expect("sscanf: button name has no index")
}

/// The event at `index` of `scheduler.events` (C++ `index < events.Size()` and `GetData(index)`).
fn event_at(g: &Ctx, index: i32) -> Option<&UplinkEvent> {
    g.world().scheduler.events.get(usize::try_from(index).ok()?)
}

/// C++ `time(NULL)`: wall-clock unix seconds.
fn unix_time_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// `screenw`, `paneltop` and `panelwidth` as computed in the C++ `Create` and `EventClick`.
fn panel_metrics(g: &Ctx) -> (i32, i32, i32) {
    let screenw = g.app.options.get_option_value("graphics_screenwidth");
    let paneltop = (100.0 * ((screenw as f64 * PANELSIZE) / 188.0) + 30.0) as i32;
    let panelwidth = (screenw as f64 * PANELSIZE) as i32;
    (screenw, paneltop, panelwidth)
}

/// C++ `EclDirtyButton` for each of the nine event rows.
fn dirty_event_rows(g: &mut Ctx) {
    for i in 0..9 {
        let name = format!("evtqueue_event {i}");
        g.ecl.dirty_button(&name);
    }
}

impl EventQueueInterface {
    /// C++ `EventQueueInterface::TitleClick` (static button callback).
    pub fn title_click(g: &mut Ctx, _button: &Button) {
        LocalInterface::run_screen(g, SCREEN_NONE, -1);
    }

    /// C++ `EventQueueInterface::ScrollUpClick`
    pub fn scroll_up_click(g: &mut Ctx, _button: &Button) {
        let mut offset = BASE_OFFSET.get() - 1;
        if offset < 0 {
            offset = 0;
        }
        BASE_OFFSET.set(offset);

        // Invalidate all the buttons
        dirty_event_rows(g);
    }

    /// C++ `EventQueueInterface::ScrollDownClick`
    pub fn scroll_down_click(g: &mut Ctx, _button: &Button) {
        BASE_OFFSET.set(BASE_OFFSET.get() + 1);

        // Invalidate all the buttons
        dirty_event_rows(g);
    }

    /// C++ `EventQueueInterface::EventDraw` (static draw callback).
    pub fn event_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        let index = parse_index(&button.name, "evtqueue_event") + BASE_OFFSET.get();

        let texts = event_at(g, index).map(|e| (e.rundate().get_long_string(), e.get_short_string()));
        match texts {
            Some((date, shortdesc)) => {
                // Draw the button background
                button_draw(g, button, highlighted, clicked);

                let screenheight = g.app.options.get_option_value("graphics_screenheight");
                gl::scissor(button.x, screenheight - (button.y + button.height), button.width, button.height);
                gl::enable(gl::SCISSOR_TEST);

                // Print the date
                text::draw_text_styled(button.x + 5, button.y + 12, &date, text::HELVETICA_10);

                // Print a short description
                text::draw_text_styled(button.x + 5, button.y + 24, &shortdesc, text::HELVETICA_10);

                gl::disable(gl::SCISSOR_TEST);
            }
            None => {
                clear_draw(g, button.x, button.y, button.width, button.height);
            }
        }
    }

    /// C++ `EventQueueInterface::EventClick`
    pub fn event_click(g: &mut Ctx, button: &Button) {
        let index = parse_index(&button.name, "evtqueue_event") + BASE_OFFSET.get();

        // Create a more detailed version
        let texts = event_at(g, index).map(|e| (e.rundate().get_long_string(), e.get_long_string(g.world())));
        let Some((date, longdesc)) = texts else { return };

        let (screenw, paneltop, panelwidth) = panel_metrics(g);

        g.ecl.register_button(screenw - panelwidth - 3, paneltop + 20, panelwidth, 15, &date, "Click to close", "evtqueue_fulldate");
        g.ecl.register_button_callback("evtqueue_fulldate", Self::full_details_close_click);
        create_stextbox(g, screenw - panelwidth - 3, paneltop + 40, panelwidth, 9 * 30, &longdesc, "evtqueue_fulldetails");
    }

    /// C++ `EventQueueInterface::DeleteEventDraw` (static draw callback).
    pub fn delete_event_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
        let index = parse_index(&button.name, "evtqueue_deleteevent") + BASE_OFFSET.get();

        if event_at(g, index).is_some() {
            imagebutton_draw(g, button, highlighted, clicked);
        }
    }

    /// C++ `EventQueueInterface::DeleteEventClick`
    pub fn delete_event_click(g: &mut Ctx, button: &Button) {
        let index = parse_index(&button.name, "evtqueue_deleteevent") + BASE_OFFSET.get();

        if event_at(g, index).is_some() {
            g.world_mut().scheduler.events.remove(index as usize);
        }
    }

    /// C++ `EventQueueInterface::FullDetailsCloseClick`
    pub fn full_details_close_click(g: &mut Ctx, _button: &Button) {
        g.ecl.remove_button("evtqueue_fulldate");
        remove_stextbox(g, "evtqueue_fulldetails");
    }
}

impl InterfaceScreen for EventQueueInterface {
    fn create(&mut self, g: &mut Ctx) {
        if self.is_visible(g) {
            return;
        }

        let (screenw, paneltop, panelwidth) = panel_metrics(g);

        // Title

        g.ecl.register_button(screenw - panelwidth - 3, paneltop, panelwidth, 15, "EVENT QUEUE", "Remove the screen", "evtqueue_title");
        g.ecl.register_button_callback("evtqueue_title", Self::title_click);

        for i in 0..9 {
            let name = format!("evtqueue_event {i}");
            g.ecl.register_button(
                screenw - panelwidth - 3,
                paneltop + 20 + i * 30,
                panelwidth - 20,
                28,
                "",
                "Click to enlarge this event",
                &name,
            );
            g.ecl.register_button_callbacks(
                &name,
                Some(Self::event_draw),
                Some(Self::event_click),
                Some(button_click),
                Some(button_highlight),
            );

            let deletename = format!("evtqueue_deleteevent {i}");
            g.ecl.register_button(
                (screenw - panelwidth - 3) + panelwidth - 35,
                paneltop + 20 + i * 30,
                13,
                13,
                " ",
                "Click to delete this event",
                &deletename,
            );
            button_assignbitmaps(g, &deletename, "close.tif", "close_h.tif", "close_c.tif");
            g.ecl.register_button_callbacks(
                &deletename,
                Some(Self::delete_event_draw),
                Some(Self::delete_event_click),
                Some(button_click),
                Some(button_highlight),
            );
        }

        g.ecl.register_button(screenw - 20, paneltop + 20, 15, 15, "^", "Scroll upwards", "evtqueue_scrollup");
        g.ecl.register_button_callback("evtqueue_scrollup", Self::scroll_up_click);

        g.ecl.register_button(screenw - 20, paneltop + 38, 15, 9 * 30 - 10, "", " ", "evtqueue_scrollbar");

        g.ecl.register_button(screenw - 20, paneltop + 50 + 9 * 30 - 17, 15, 15, "v", "Scroll downwards", "evtqueue_scrolldown");
        g.ecl.register_button_callback("evtqueue_scrolldown", Self::scroll_down_click);
    }

    fn remove(&mut self, g: &mut Ctx) {
        if !self.is_visible(g) {
            return;
        }

        g.ecl.remove_button("evtqueue_title");

        for i in 0..9 {
            g.ecl.remove_button(&format!("evtqueue_event {i}"));
            g.ecl.remove_button(&format!("evtqueue_deleteevent {i}"));
        }

        g.ecl.remove_button("evtqueue_scrollup");
        g.ecl.remove_button("evtqueue_scrollbar");
        g.ecl.remove_button("evtqueue_scrolldown");
    }

    fn update(&mut self, g: &mut Ctx) {
        // Update every 5 seconds
        if unix_time_secs() > (PREVIOUS_UPDATE.get() + 5) as i64 {
            // Invalidate all the buttons
            dirty_event_rows(g);
        }
    }

    fn is_visible(&self, g: &Ctx) -> bool {
        g.ecl.get_button("evtqueue_title").is_some()
    }

    fn screen_id(&self) -> i32 {
        SCREEN_EVTQUEUE
    }

    impl_as_any!();
}
