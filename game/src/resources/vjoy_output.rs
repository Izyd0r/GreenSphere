use bevy::prelude::*;

/// The public state of the virtual joystick. 
/// Read this from your movement systems to control entities.
#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
pub struct VjoyOutput {
    pub dir: Vec2
}