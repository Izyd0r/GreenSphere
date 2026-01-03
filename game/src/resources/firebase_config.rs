use bevy::prelude::*;

#[derive(Resource)]
pub struct FirebaseConfig {
    pub url: String,
}

impl Default for FirebaseConfig {
    fn default() -> Self {
        Self {
            url: option_env!("FIREBASE_URL")
                .unwrap_or("ERROR")
                .to_string(),
        }
    }
}