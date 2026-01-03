use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Resource, Reflect, Debug, Default)]
#[reflect(Resource)]
pub struct Leaderboard {
    pub entries: Vec<(String, usize, f32)>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FirebaseEntry {
    pub name: String,
    pub score: usize,
    pub time: f32,
}