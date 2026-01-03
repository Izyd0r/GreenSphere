use bevy::prelude::*;

#[derive(Resource, Reflect, Debug)]
#[reflect(Resource)]
pub struct PlayerProfile {
    pub username: String,
}

impl Default for PlayerProfile {
    fn default() -> Self {
        Self { username: "".to_string() }
    }
}