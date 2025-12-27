use bevy::prelude::*;

/// The public state of the virtual joystick. 
/// Read this from your movement systems to control entities.
#[derive(Resource, Default)]
pub struct VjoyOutput {
    /// Normalized direction vector ranging from -1.0 to 1.0.
    /// (0,0) represents the center/idle state.
    pub dir: Vec2
}