use bevy::prelude::*;

#[derive(Component)]
pub struct AlienFactory;

#[derive(Component)]
pub struct FactorySpawner {
    pub timer: Timer,
}