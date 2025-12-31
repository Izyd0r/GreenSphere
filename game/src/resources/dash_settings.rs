use bevy::prelude::*;

#[derive(Resource, Reflect, Debug)]
#[reflect(Resource)]
pub struct DashSettings {
    pub dash_force: f32,
    pub dash_duration: f32,
    pub max_energy: f32,
    pub regen_rate: f32,
    pub dash_cost: f32,
    pub cooldown_secs: f32,
}

impl Default for DashSettings {
    fn default() -> Self {
        Self {
            dash_force: 200.0,
            max_energy: 100.0,
            regen_rate: 33.0,
            dash_cost: 100.0,
            cooldown_secs: 1.0,
            dash_duration: 0.5,
        }
    }
}