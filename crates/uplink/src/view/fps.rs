//! Port of source/view/fps.h + source/view/fps.cpp
//!
//! Frame rate checker utility class. PORT: takes `&mut Eclipse` (for `EclGetAccurateTime`)
//! instead of the whole `Ctx`, so `View` can call it while borrowing only `g.ecl`.

#![allow(unused_variables, unused_imports, unused_mut, dead_code)]

use crate::eclipse::Eclipse;

/// Higher = more accurate, less frequent updates.
pub const NUMFRAMES: i32 = 200;

#[derive(Debug, Clone)]
pub struct FrameRate {
    pub starttime: i32,
    /// The number still to be timed.
    pub numframesremaining: i32,
    pub fps: f32,
}

impl Default for FrameRate {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameRate {
    pub fn new() -> Self {
        FrameRate {
            starttime: 0,
            numframesremaining: NUMFRAMES,
            fps: 0.0,
        }
    }

    pub fn start(&mut self, ecl: &mut Eclipse) {
        self.starttime = ecl.get_accurate_time();
        self.numframesremaining = NUMFRAMES;
        self.fps = 0.0;
    }

    pub fn tick(&mut self, ecl: &mut Eclipse) {
        if self.numframesremaining == 0 {
            let endtime = ecl.get_accurate_time();
            let time = endtime - self.starttime;
            self.fps = NUMFRAMES as f32 / time as f32;
            self.numframesremaining = NUMFRAMES;
            self.starttime = ecl.get_accurate_time();
        } else {
            self.numframesremaining -= 1;
        }
    }

    pub fn fps(&self) -> f32 {
        self.fps
    }
}
