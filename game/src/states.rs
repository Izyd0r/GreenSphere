use bevy::prelude::*;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default, Reflect)]
pub enum GameState {
    #[default]
    MainMenu,
    Resetting,
    GameOver,
    Playing,
}