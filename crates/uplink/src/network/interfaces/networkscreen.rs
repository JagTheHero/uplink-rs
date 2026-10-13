//! Port of source/network/interfaces/networkscreen.h + source/network/interfaces/networkscreen.cpp
//!
//! The C++ base class has no data. Its virtuals (Create / Remove / Update / IsVisible / ScreenID)
//! are the `InterfaceScreen` trait methods (PORTING.md §3.7), so subclasses implement that trait
//! and this marker trait.

use crate::interface::interfacescreen::InterfaceScreen;

/// Marker for the screens a network client can be shown.
pub trait NetworkScreen: InterfaceScreen {}
