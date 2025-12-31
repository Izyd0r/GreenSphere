use bevy::prelude::*;

#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
pub struct DashState {
    pub current_energy: f32,
    pub cooldown_timer: f32,
    pub duration_timer: f32,
    pub is_active: bool,
}