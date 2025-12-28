use bevy::prelude::*;

#[derive(Component, Default)]
pub struct PlayerBall {
    pub velocity: Vec3,
}