#![allow(unused)]
use bevy::prelude::*;

#[derive(Resource)]
pub struct MousePosition {
    pub coords: Vec2
}

pub(crate) fn plugin(app: &mut App) {
    app.insert_resource(MousePosition{ coords: Vec2::default() });
}