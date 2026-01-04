#![allow(unused_imports)]

use bevy::prelude::*;

mod components;
mod plugins;
mod resources;
mod states;

mod prelude {
    pub use super::*;
    pub use {components::*, plugins::*, resources::*};
    pub use crate::states::GameState;
}

pub struct AppPlugin;

impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        // All custome plugins goes here
        app.add_plugins((
            plugins::defaults::plugin,
            // plugins::camera::plugin,
            plugins::game::plugin,
            plugins::menu::plugin,
            plugins::hud::plugin,
            plugins::vjoy::plugin,
        ));
        #[cfg(feature="dev")]
        app.add_plugins(plugins::debug::plugin);
    }
}