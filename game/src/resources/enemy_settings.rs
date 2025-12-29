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
}

impl Default for EnemySettings {
    fn default() -> Self {
        Self {
            factory_count: 5,
            pollution_radius: 12.0,
            pollution_color: [0.8, 0.1, 0.1, 1.0],
            spread_tick_rate: 10.0,
            boosted_spread_chance: 0.4,
            natural_spread_chance: 0.02,
        }
    }
}