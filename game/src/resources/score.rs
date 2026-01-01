use bevy::prelude::*;

#[derive(Resource, Default, Reflect, Debug)]
#[reflect(Resource)]
pub struct Score {
    pub current: usize,
}

#[derive(Message)]
pub struct ScoreMessage(pub usize);