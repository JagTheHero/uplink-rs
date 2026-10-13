//! Port of `InterfaceScreen` — the base of every screen (local, remote, main menu, network) —
//! plus [`ScreenSlot`], the holder each host keeps its current screen in.
//!
//! While a screen method runs the screen is *taken out* of its slot, so the method can have
//! `&mut self` and `&mut Ctx` at the same time. Re-entrant screen switches requested during that
//! time are deferred with [`Ctx::defer`] and run as soon as the method returns.

use std::any::Any;

use crate::ctx::Ctx;
use crate::objref::ScreenRef;

pub trait InterfaceScreen: Any {
    fn create(&mut self, _g: &mut Ctx) {}
    fn remove(&mut self, _g: &mut Ctx) {}
    fn update(&mut self, _g: &mut Ctx) {}
    fn is_visible(&self, _g: &Ctx) -> bool {
        false
    }
    fn screen_id(&self) -> i32 {
        0
    }
    /// Return true to block further handling of the return key.
    fn return_key_pressed(&mut self, _g: &mut Ctx) -> bool {
        false
    }
    /// Return true to block further handling of the escape key.
    fn escape_key_pressed(&mut self, _g: &mut Ctx) -> bool {
        false
    }
    /// Remote screens: the `ComputerScreen` this interface shows. Called before `create`.
    fn set_computer_screen(&mut self, _cs: ScreenRef) {}
    /// Remote screens: the `ComputerScreen` this interface shows.
    fn computer_screen_ref(&self) -> Option<&ScreenRef> {
        None
    }
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Implements `as_any`/`as_any_mut` inside an `impl InterfaceScreen for X` (or `UplinkTask`) block.
#[macro_export]
macro_rules! impl_as_any {
    () => {
        fn as_any(&self) -> &dyn ::std::any::Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn ::std::any::Any {
            self
        }
    };
}

/// Where a host keeps its current screen.
#[derive(Default)]
pub struct ScreenSlot {
    screen: Option<Box<dyn InterfaceScreen>>,
    busy: bool,
}

impl ScreenSlot {
    pub fn new() -> Self {
        Self::default()
    }

    /// True while one of the screen's own methods is running (the screen is taken out).
    pub fn is_busy(&self) -> bool {
        self.busy
    }

    /// True if there is a screen (including one that is currently running a method).
    pub fn is_some(&self) -> bool {
        self.screen.is_some() || self.busy
    }

    /// Screen id of the current screen (0 if none or busy).
    pub fn screen_id(&self) -> i32 {
        self.screen.as_ref().map(|s| s.screen_id()).unwrap_or(0)
    }

    /// Downcast the current screen. `None` if there is no screen, it is another type, or busy.
    pub fn get<T: InterfaceScreen>(&self) -> Option<&T> {
        self.screen.as_ref().and_then(|s| s.as_any().downcast_ref::<T>())
    }

    /// Mutable downcast of the current screen.
    pub fn get_mut<T: InterfaceScreen>(&mut self) -> Option<&mut T> {
        self.screen.as_mut().and_then(|s| s.as_any_mut().downcast_mut::<T>())
    }

    /// The current screen as a trait object (None while busy).
    pub fn get_dyn(&self) -> Option<&dyn InterfaceScreen> {
        self.screen.as_deref()
    }

    pub fn get_dyn_mut(&mut self) -> Option<&mut (dyn InterfaceScreen + 'static)> {
        self.screen.as_deref_mut()
    }

    /// Put a screen in the slot without calling anything (replaces silently).
    pub fn set(&mut self, screen: Box<dyn InterfaceScreen>) {
        self.screen = Some(screen);
    }

    /// Take the screen out without calling anything.
    pub fn take(&mut self) -> Option<Box<dyn InterfaceScreen>> {
        self.screen.take()
    }

    /// Run `f` with the screen taken out of the slot located by `slot`.
    /// Returns `None` if the slot is empty or already busy (re-entrant call).
    pub fn call<R>(
        g: &mut Ctx,
        slot: fn(&mut Ctx) -> &mut ScreenSlot,
        f: impl FnOnce(&mut dyn InterfaceScreen, &mut Ctx) -> R,
    ) -> Option<R> {
        let s = slot(g);
        if s.busy {
            return None;
        }
        let mut screen = s.screen.take()?;
        s.busy = true;
        let r = f(screen.as_mut(), g);
        let s = slot(g);
        s.busy = false;
        if s.screen.is_none() {
            s.screen = Some(screen);
        }
        g.run_deferred();
        Some(r)
    }

    /// Replace the current screen: `remove()` the old one, store `new`, then `create()` it.
    /// If the slot is busy (called from inside the current screen's method) the switch is
    /// deferred until that method returns.
    pub fn replace(g: &mut Ctx, slot: fn(&mut Ctx) -> &mut ScreenSlot, new: Option<Box<dyn InterfaceScreen>>) {
        if slot(g).busy {
            g.defer(move |g| ScreenSlot::replace(g, slot, new));
            return;
        }
        if let Some(mut old) = slot(g).screen.take() {
            slot(g).busy = true;
            old.remove(g);
            slot(g).busy = false;
        }
        if let Some(new) = new {
            slot(g).screen = Some(new);
            ScreenSlot::call(g, slot, |s, g| s.create(g));
        }
        g.run_deferred();
    }
}
