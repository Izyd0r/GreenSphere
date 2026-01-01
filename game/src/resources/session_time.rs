use bevy::prelude::*;

#[derive(Resource, Default, Reflect, Debug)]
#[reflect(Resource)]
pub struct SessionTime {
    pub elapsed: f32,
}

impl SessionTime {
    pub fn format(&self) -> String {
        let minutes = (self.elapsed / 60.0) as u32;
        let seconds = (self.elapsed % 60.0) as u32;
        format!("{:02}:{:02}", minutes, seconds)
    }
}