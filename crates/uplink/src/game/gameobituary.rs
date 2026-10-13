//! Port of source/game/gameobituary.h + source/game/gameobituary.cpp
//!
//! Represents a game that has ended, so the player can keep tabs on his old characters.

#![allow(unused_variables, unused_imports, unused_mut, dead_code)]

use serde::{Deserialize, Serialize};

use crate::world::world::World;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GameObituary {
    /// `None` = C++ NULL.
    pub gameoverreason: Option<String>,
    pub name: String,
    pub money: i32,
    pub uplinkrating: i32,
    pub neuromancerrating: i32,
    pub specialmissionscompleted: i32,
    pub score_peoplefucked: i32,
    pub score_systemsfucked: i32,
    pub score_highsecurityhacks: i32,
    pub score: i32,
    /// True if this occured due to demo game over.
    pub demogameover: bool,
    /// True if this occured due to warez game over.
    pub warezgameover: bool,
}

impl Default for GameObituary {
    fn default() -> Self {
        GameObituary {
            gameoverreason: None,
            name: " ".to_string(),
            money: 0,
            uplinkrating: 0,
            neuromancerrating: 0,
            specialmissionscompleted: 0,
            score_peoplefucked: 0,
            score_systemsfucked: 0,
            score_highsecurityhacks: 0,
            score: 0,
            demogameover: false,
            warezgameover: false,
        }
    }
}

impl GameObituary {
    pub fn new() -> Self {
        GameObituary::default()
    }

    pub fn set_game_over_reason(&mut self, newreason: &str) {
        self.gameoverreason = Some(newreason.to_string());
    }

    pub fn game_over_reason(&self) -> Option<&str> {
        self.gameoverreason.as_deref()
    }

    pub fn set_demo_game_over(&mut self, newvalue: bool) {
        self.demogameover = newvalue;
    }

    pub fn set_warez_game_over(&mut self, newvalue: bool) {
        self.warezgameover = newvalue;
    }

    /// Looks at the player and stores his stats.
    // PORT: only reads the world, so it takes `&World` rather than the whole `Ctx`.
    pub fn evaluate(&mut self, world: &World) {
        let player = world.get_player();

        self.name = player.agent().handle.clone();
        self.money = player.get_balance(world);
        self.uplinkrating = player.rating.uplinkrating;
        self.neuromancerrating = player.rating.neuromancerrating;
        self.specialmissionscompleted = world.plotgenerator.get_special_missions_completed();

        self.score_peoplefucked = player.score_peoplefucked;
        self.score_systemsfucked = player.score_systemsfucked;
        self.score_highsecurityhacks = player.score_highsecurityhacks;

        // UplinkAssert (gatewaydef) in the C++: a missing definition aborts the game.
        let gatewaydef = world
            .get_gateway_def(player.gateway.type_, Some(&player.gateway.name))
            .expect("GameObituary::Evaluate: gateway definition not found");
        let gatewayvalue = gatewaydef.cost;

        if self.money > 10_000_000 {
            self.money = 10_000_000;
        }

        let mut numspecialmissions = 0;
        for i in 0..16 {
            if world.plotgenerator.player_completed_special_mission(i) {
                numspecialmissions += 1;
            }
        }

        self.score = player.rating.uplinkscore
            + self.money / 100
            + gatewayvalue / 1000
            + self.score_peoplefucked
            + self.score_systemsfucked
            + self.score_highsecurityhacks
            + numspecialmissions * 200;

        // C++ int arithmetic wraps; Rust debug builds would panic on overflow.
        self.score = self.score.wrapping_mul(100);

        if self.warezgameover {
            self.score = 0;
        }
    }

    pub fn update(&mut self) {}
}
