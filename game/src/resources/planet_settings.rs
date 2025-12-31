use bevy::prelude::*;

#[derive(Resource, Reflect, Debug)]
#[reflect(Resource)]
pub struct PlanetSettings {
    pub radius: f32,
    pub subdivisions: u32,
    pub player_speed: f32,
    pub player_radius: f32,
    pub camera_height: f32,
    pub acceleration: f32,
    pub friction: f32,
    pub max_speed: f32,
    pub camera_smoothing: f32,
    pub god_mode: bool,        // For testing
    pub max_hp_radius: f32,    // Set this to 16.0
}

impl Default for PlanetSettings {
    fn default() -> Self {
        Self {
            radius: 150.0,
            subdivisions: 16,
            player_speed: 60.0,
            player_radius: 4.0,
            camera_height: 100.0,
            acceleration: 150.0,
            friction: 0.985,
            max_speed: 80.0,
            camera_smoothing: 0.1,
            god_mode: false,
            max_hp_radius: 16.0,
        }
    }
}