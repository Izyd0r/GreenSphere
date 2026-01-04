use bevy::prelude::*;

#[derive(Resource, Reflect, Debug)]
#[reflect(Resource)]
pub struct PlanetSettings {
    pub radius: f32,
    pub subdivisions: u32,
    pub friction: f32,
    pub orb_hp_gain: f32,
    pub max_orbs: usize,
    pub orb_spawn_chance: f32,
}

impl Default for PlanetSettings {
    fn default() -> Self {
        Self {
            radius: 150.0,
            subdivisions: 16,
            friction: 0.985,
            orb_hp_gain: 25.0,
            max_orbs: 10,
            orb_spawn_chance: 0.002,
        }
    }
}