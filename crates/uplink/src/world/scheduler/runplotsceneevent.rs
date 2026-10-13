//! Port of source/world/scheduler/runplotsceneevent.h + source/world/scheduler/runplotsceneevent.cpp

use serde::{Deserialize, Serialize};

use crate::ctx::Ctx;
use crate::world::generator::demoplotgenerator::DemoPlotGenerator;
use crate::world::generator::plotgenerator::PlotGenerator;
use crate::world::scheduler::uplinkevent::UplinkEventBase;

/// Runs the next scene in the main plot line.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RunPlotSceneEvent {
    pub base: UplinkEventBase,
    /// If act = -1, this is a DemoPlotSceneEvent
    pub act: i32,
    pub scene: i32,
}

impl RunPlotSceneEvent {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_act_and_scene(&mut self, newact: i32, newscene: i32) {
        self.act = newact;
        self.scene = newscene;
    }

    pub fn run(&self, g: &mut Ctx) {
        if self.act != -1 {
            PlotGenerator::run_scene(g, self.act, self.scene);
        } else {
            DemoPlotGenerator::run_scene(g, self.scene);
        }
    }

    pub fn get_short_string(&self) -> String {
        self.get_long_string()
    }

    pub fn get_long_string(&self) -> String {
        if self.act != -1 {
            format!("Run Plot Act {}, Scene {}", self.act, self.scene)
        } else {
            format!("Run Demo Plot Scene {}", self.scene)
        }
    }
}
