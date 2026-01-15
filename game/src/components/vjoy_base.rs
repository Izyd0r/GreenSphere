use bevy::prelude::*;
use bevy::picking::pointer::PointerId;

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub struct VjoyBase {
    pub radius: f32,
    pub active_pointer: Option<PointerId>, 
}