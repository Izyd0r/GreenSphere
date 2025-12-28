use bevy::prelude::*;

#[derive(Component, Default)]
pub struct PlayerBall {
    pub current_velocity: Vec3,
}