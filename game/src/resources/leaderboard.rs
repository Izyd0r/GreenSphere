use bevy::prelude::*;

#[derive(Resource, Reflect, Debug)]
#[reflect(Resource)]
pub struct Leaderboard {
    pub entries: Vec<(String, usize, f32)>,
}

impl Default for Leaderboard {
    fn default() -> Self {
        Self {
            entries: vec![
                ("MossMaster".to_string(), 50000, 360.5),
                ("GreenKing".to_string(), 35000, 240.0), 
                ("PlanetSaver".to_string(), 12000, 115.2),
            ],
        }
    }
}