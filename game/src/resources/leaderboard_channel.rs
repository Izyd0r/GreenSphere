use bevy::prelude::*;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Mutex;

#[derive(Resource)]
pub struct LeaderboardChannel {
    pub tx: Sender<Vec<(String, usize, f32)>>,
    pub rx: Mutex<Receiver<Vec<(String, usize, f32)>>>,
}

impl Default for LeaderboardChannel {
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self { tx, rx: Mutex::new(rx) }
    }
}