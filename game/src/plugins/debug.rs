use bevy::app::App;
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};

pub(crate) fn plugin(app: &mut App) {
    app.add_plugins((
        LogDiagnosticsPlugin::default(),
        FrameTimeDiagnosticsPlugin::default(),
    ));

    #[cfg(feature = "dev")]
    {
        use bevy_egui::EguiPlugin;
        use bevy_inspector_egui::quick::ResourceInspectorPlugin;
        use crate::prelude::dash_settings::DashSettings;
        use crate::prelude::dash_state::DashState;
        use crate::resources::planet_settings::PlanetSettings;        
        use crate::resources::vjoy_config::VjoyConfig;
        use crate::resources::vjoy_output::VjoyOutput;
        use crate::resources::enemy_settings::EnemySettings;
        
        app.add_plugins(EguiPlugin::default());
        
        app.add_plugins(ResourceInspectorPlugin::<PlanetSettings>::default());
        app.add_plugins(ResourceInspectorPlugin::<EnemySettings>::default());
        
        app.add_plugins(ResourceInspectorPlugin::<VjoyConfig>::default());
        app.add_plugins(ResourceInspectorPlugin::<VjoyOutput>::default());

        app.add_plugins(ResourceInspectorPlugin::<DashSettings>::default());
        app.add_plugins(ResourceInspectorPlugin::<DashState>::default());
    }
}