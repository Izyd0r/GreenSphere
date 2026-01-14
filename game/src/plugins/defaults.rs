use bevy::{asset::AssetMetaCheck, prelude::*};
// TODO: delete this variable
const BACKGROUND_COLOR: Color = Color::srgb(0.4, 0.4, 0.4);

// Sets up the default plugins like windows, assets, etc

pub(crate) fn plugin(app: &mut App) {
    app.insert_resource(ClearColor(BACKGROUND_COLOR))
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        // TODO: create a config file that contains all these config properties
                        title: "Bevy game".into(),
                        resizable: true,
                        resolution: (800, 600).into(),
                        canvas: Some("#bevy".to_owned()),
                        desired_maximum_frame_latency: core::num::NonZero::new(1u32),
                        fit_canvas_to_parent: true,
                        ..default()
                    }),
                    ..default()
                }),
        );
}