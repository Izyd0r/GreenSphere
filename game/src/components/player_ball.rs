use bevy::prelude::*;

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct PlayerBall {
    pub current_velocity: Vec3,
    pub hp: f32,
    pub invincibility_timer: f32,
}