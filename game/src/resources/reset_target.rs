use bevy::prelude::*;

use crate::plugins::game::GameState;

#[derive(Resource)]
pub struct ResetTarget(pub GameState);

impl Default for ResetTarget {
    fn default() -> Self {
        Self(GameState::MainMenu)
    }
}