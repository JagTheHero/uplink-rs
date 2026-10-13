//! Port of source/view/view.h + source/view/view.cpp
//!
//! The View used to draw a 3D representation of the game world; in the shipped game
//! it only runs the frame-rate counter. PORT: methods take `&mut Eclipse` (not `Ctx`)
//! so the view can be driven while `g.game.view` is borrowed.

#![allow(unused_variables, unused_imports, unused_mut, dead_code)]

use serde::{Deserialize, Serialize};

use crate::eclipse::Eclipse;
use crate::view::fps::FrameRate;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct View {
    #[serde(skip)]
    pub fps: FrameRate,
}

impl View {
    pub fn new() -> Self {
        View::default()
    }

    pub fn initialise(&mut self, ecl: &mut Eclipse) {
        self.fps.start(ecl);
    }

    /// This is the big one.
    pub fn draw(&mut self, ecl: &mut Eclipse) {
        self.fps.tick(ecl);
    }

    pub fn update(&mut self) {}
}
