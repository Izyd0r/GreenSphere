use bevy::prelude::*;

#[derive(Component)]
pub struct BirdEyeCamera {
    pub last_normal: Vec3,
}