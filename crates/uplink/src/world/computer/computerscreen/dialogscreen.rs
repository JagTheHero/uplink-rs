//! Port of source/world/computer/computerscreen/dialogscreen.h + source/world/computer/computerscreen/dialogscreen.cpp

use serde::{Deserialize, Serialize};

use crate::world::computer::computerscreen::computerscreen::ComputerScreenBase;

/// Error occurs (unassigned)
pub const WIDGET_NONE: i32 = 0;
/// Standard blue button
pub const WIDGET_BASIC: i32 = 1;
/// Text only, locked
pub const WIDGET_CAPTION: i32 = 2;
/// Text only, editable
pub const WIDGET_TEXTBOX: i32 = 3;
/// Text only, editable, hidden with '*' characters
pub const WIDGET_PASSWORDBOX: i32 = 4;
/// data1 = nextpageindex
pub const WIDGET_NEXTPAGE: i32 = 5;
/// stringdata1 = filename (normal), sdata2 = highlighted
pub const WIDGET_IMAGE: i32 = 6;
/// data1 = nextpageindex, stringdata1/2 = filename
pub const WIDGET_IMAGEBUTTON: i32 = 7;
/// data1 = script index, data2 = nextpageindex
pub const WIDGET_SCRIPTBUTTON: i32 = 8;
/// stringdata1 = fieldname
pub const WIDGET_FIELDVALUE: i32 = 9;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DialogScreen {
    pub base: ComputerScreenBase,
    pub widgets: Vec<DialogScreenWidget>,
    /// Button name pressed when the return key is pressed
    pub returnkeybutton: Option<String>,
    /// Button name pressed when the escape key is pressed
    pub escapekeybutton: Option<String>,
}

impl DialogScreen {
    pub fn new() -> Self {
        Self::default()
    }

    /// `AddWidget` (8-argument overload)
    #[allow(clippy::too_many_arguments)]
    pub fn add_widget(
        &mut self,
        name: &str,
        widgettype: i32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        caption: &str,
        tooltip: &str,
    ) {
        let mut dsw = DialogScreenWidget::new();
        dsw.set_name(name);
        dsw.set_type(widgettype);
        dsw.set_position(x, y);
        dsw.set_size(width, height);
        dsw.set_caption(caption);
        dsw.set_tooltip(tooltip);
        self.widgets.push(dsw);
    }

    /// `AddWidget` (12-argument overload with data/stringdata)
    #[allow(clippy::too_many_arguments)]
    pub fn add_widget_with_data(
        &mut self,
        name: &str,
        widgettype: i32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        caption: &str,
        tooltip: &str,
        data1: i32,
        data2: i32,
        stringdata1: Option<&str>,
        stringdata2: Option<&str>,
    ) {
        let mut dsw = DialogScreenWidget::new();
        dsw.set_name(name);
        dsw.set_type(widgettype);
        dsw.set_position(x, y);
        dsw.set_size(width, height);
        dsw.set_caption(caption);
        dsw.set_tooltip(tooltip);
        dsw.set_data(data1, data2);
        dsw.set_string_data(stringdata1, stringdata2);
        self.widgets.push(dsw);
    }

    /// Removes the first widget called `name`.
    pub fn remove_widget(&mut self, name: &str) {
        if let Some(i) = self.widgets.iter().position(|w| w.name == name) {
            self.widgets.remove(i);
        }
    }

    pub fn set_return_key_button(&mut self, name: &str) {
        self.returnkeybutton = Some(name.to_string());
    }

    pub fn set_escape_key_button(&mut self, name: &str) {
        self.escapekeybutton = Some(name.to_string());
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DialogScreenWidget {
    // PORT: protected in C++; public here, with the C++ getters kept below.
    pub name: String,
    pub caption: String,
    pub tooltip: String,
    pub stringdata1: Option<String>,
    pub stringdata2: Option<String>,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub widgettype: i32,
    pub data1: i32,
    pub data2: i32,
}

impl DialogScreenWidget {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_name(&mut self, newname: &str) {
        self.name = newname.to_string();
    }

    /// `SetTYPE`
    pub fn set_type(&mut self, new_widgettype: i32) {
        self.widgettype = new_widgettype;
    }

    pub fn set_position(&mut self, newx: i32, newy: i32) {
        self.x = newx;
        self.y = newy;
    }

    pub fn set_size(&mut self, newwidth: i32, newheight: i32) {
        self.width = newwidth;
        self.height = newheight;
    }

    pub fn set_tooltip(&mut self, newtooltip: &str) {
        self.tooltip = newtooltip.to_string();
    }

    pub fn set_caption(&mut self, newcaption: &str) {
        self.caption = newcaption.to_string();
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_tooltip(&self) -> &str {
        &self.tooltip
    }

    pub fn get_caption(&self) -> &str {
        &self.caption
    }

    pub fn set_data(&mut self, newdata1: i32, newdata2: i32) {
        self.data1 = newdata1;
        self.data2 = newdata2;
    }

    pub fn set_string_data(&mut self, newstringdata1: Option<&str>, newstringdata2: Option<&str>) {
        self.stringdata1 = newstringdata1.map(str::to_string);
        self.stringdata2 = newstringdata2.map(str::to_string);
    }

    pub fn get_string_data1(&self) -> Option<&str> {
        self.stringdata1.as_deref()
    }

    pub fn get_string_data2(&self) -> Option<&str> {
        self.stringdata2.as_deref()
    }
}
