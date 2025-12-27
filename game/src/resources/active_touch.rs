use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct ActiveTouch {
    pub id: Option<u64>
}