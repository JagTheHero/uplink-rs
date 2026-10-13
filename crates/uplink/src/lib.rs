//! Rust port of Uplink (Introversion Software, 2001), from the Uplink Developer CD source.
//! See `docs/PORTING.md` for how the C++ maps onto this crate.
#![allow(clippy::too_many_arguments, clippy::module_inception, clippy::new_without_default)]

pub mod ctx;
pub mod eclipse;
pub mod objref;
pub mod tosser;
pub mod uplink_main;
pub mod vanbakel;

pub mod app;
pub mod game;
pub mod interface;
pub mod mainmenu;
pub mod network;
pub mod options;
pub mod view;
pub mod world;

#[cfg(test)]
pub(crate) mod testutil;
