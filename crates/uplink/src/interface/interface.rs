//! Port of source/interface/interface.h + source/interface/interface.cpp
#![allow(unused_variables, unused_imports, unused_mut, dead_code)]

use crate::ctx::Ctx;
use crate::interface::localinterface::localinterface::LocalInterface;
use crate::interface::remoteinterface::remoteinterface::RemoteInterface;
use crate::interface::scrollbox::ScrollBox;
use crate::interface::taskmanager::taskmanager::TaskManager;

/// Handles all user input: the local interface (HUD + gateway screens), the remote interface
/// (the connected computer's screens) and the task manager.
///
/// `GetLocalInterface` / `GetRemoteInterface` / `GetTaskManager` are the public fields below.
#[derive(Default)]
pub struct Interface {
    pub li: LocalInterface,
    pub ri: RemoteInterface,
    pub tm: TaskManager,
    // PORT: the C++ statics `ScrollBox::scrollBoxes` / `ScrollBox::currentGrab` live here
    // (see scrollbox.rs) because there is no static state in the port.
    pub scrollboxes: Vec<ScrollBox>,
    /// Name of the scroll box whose bar is currently grabbed (`currentGrab`; `None` = "None").
    pub scroll_grab: Option<String>,
}

impl Interface {
    /// C++ constructor. PORT: the C++ `TaskManager` constructor called `SvbReset`; after the
    /// new `Interface` has been stored in `g.game.ui` the owner must call `TaskManager::reset(g)`.
    pub fn new() -> Self {
        Interface::default()
    }

    /// `Interface::Create`: creates the remote interface, then (unless this is the first-time
    /// game) resets and creates the local interface.
    pub fn create(g: &mut Ctx) {
        RemoteInterface::create(g);

        if g.app.options.is_option_equal_to("game_firsttime", 1) {
            // Stop the local interface from being created
        } else {
            g.interface_mut().li.reset();
            LocalInterface::create(g);
        }
    }

    /// `Interface::Update`
    pub fn update(g: &mut Ctx) {
        LocalInterface::update(g);
        RemoteInterface::update(g);
        TaskManager::update(g);
    }
}
