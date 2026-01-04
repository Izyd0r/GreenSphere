use bevy::prelude::*;

#[derive(Resource, Reflect, Debug)]
#[reflect(Resource)]
pub struct PlayerSettings {
    pub player_speed: f32,
    pub player_radius: f32,
    pub camera_height: f32,
    pub acceleration: f32,
    pub max_speed: f32,
    pub camera_smoothing: f32,
    pub god_mode: bool,
    pub max_hp_radius: f32,
}

impl Default for PlayerSettings {
    fn default() -> Self {
        Self {
            player_speed: 60.0,
            player_radius: 4.0,
            camera_height: 100.0,
            acceleration: 150.0,
            max_speed: 80.0,
            camera_smoothing: 0.1,
            god_mode: false,
            max_hp_radius: 16.0,
        }
    }
}