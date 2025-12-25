#![allow(unused)]

use bevy::{app::App, prelude::*};
use crate::prelude::input::MousePosition;

// TODO: for testing purposes, delete later
fn display_coords(mouse_point: Res<MousePosition>) {
    println!("{}", mouse_point.coords.x);
}

pub(crate) fn plugin(app: &mut App) {
    // Game logic here
    app.add_systems(Update, display_coords);
}