use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Default, Reflect)]
pub enum TileState {
    #[default]
    Wasteland,
    Healthy,
    Polluted,
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct PlanetData {
    pub vertex_states: Vec<TileState>,
    pub adjacency: Vec<Vec<usize>>, 
}

#[derive(Component)]
pub struct Planet;