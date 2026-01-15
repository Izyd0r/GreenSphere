use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct DashTouch {
    pub id: Option<u64>,
}