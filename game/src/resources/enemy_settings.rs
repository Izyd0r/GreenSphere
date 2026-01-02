use bevy::prelude::*;

#[derive(Resource, Reflect, Debug)]
#[reflect(Resource)]
pub struct EnemySettings {
    pub factory_count: usize,
    pub pollution_radius: f32,
    pub pollution_color: [f32; 4],
    pub spread_tick_rate: f32,
    pub boosted_spread_chance: f32,
    pub natural_spread_chance: f32,
    pub machine_spawn_interval: f32,
    pub machine_speed: f32,
    pub machine_detection_range: f32,
    pub machine_acceleration: f32,
    pub factory_spawn_timer: Timer,
    pub difficulty_scale: f32,
    pub difficulty_growth_rate: f32,
}

impl Default for EnemySettings {
    fn default() -> Self {
        Self {
            factory_count: 3,
            pollution_radius: 12.0,
            pollution_color: [0.8, 0.1, 0.1, 1.0],
            spread_tick_rate: 10.0,
            boosted_spread_chance: 0.4,
            natural_spread_chance: 0.02,
            machine_spawn_interval: 10.0,
            machine_speed: 40.0,
            machine_detection_range: 200.0,
            machine_acceleration: 100.0,
            factory_spawn_timer: Timer::from_seconds(30.0, TimerMode::Repeating),
            difficulty_scale: 1.0,
            difficulty_growth_rate: 0.01,
        }
    }
}