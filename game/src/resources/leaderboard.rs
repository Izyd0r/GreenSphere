use bevy::prelude::*;

#[derive(Resource, Reflect, Debug)]
#[reflect(Resource)]
pub struct Leaderboard {
    pub entries: Vec<(String, usize)>,
}

// TODO: implement firebase
impl Default for Leaderboard {
    fn default() -> Self {
        Self {
            entries: vec![
                ("MossMaster".to_string(), 50000),
                ("GreenKing".to_string(), 35000),
                ("PlanetSaver".to_string(), 12000),
            ],
        }
    }
}