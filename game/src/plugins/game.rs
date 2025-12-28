use bevy::prelude::*;
use crate::resources::vjoy_output::VjoyOutput;
use crate::resources::planet_settings::PlanetSettings;
use crate::components::planet::Planet;
use crate::components::player_ball::PlayerBall;
use crate::components::camera::BirdEyeCamera;

pub(crate) fn plugin(app: &mut App) {
    app
        .init_resource::<PlanetSettings>()
        .register_type::<PlanetSettings>()
        .add_systems(Startup, setup_game)
        .add_systems(Update, (
            player_movement_system,
            camera_follow_system,
        ).chain());

    #[cfg(feature = "dev")]
    app.add_systems(Update, (sync_planet_visuals, sync_player_visuals));
}

fn setup_game(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    settings: Res<PlanetSettings>,
) {
    commands.spawn((
        DirectionalLight {
            illuminance: 7000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(50.0, 100.0, 50.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Planet,
        Mesh3d(meshes.add(Sphere::new(settings.radius).mesh().ico(settings.subdivisions).unwrap())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.15, 0.15, 0.15),
            ..default()
        })),
    ));

    let player_radius = 2.0;
    let start_pos = Vec3::new(0.0, settings.radius + player_radius, 0.0);
    
    commands.spawn((
        PlayerBall,
        Mesh3d(meshes.add(Sphere::new(player_radius).mesh().ico(5).unwrap())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.0, 1.0, 0.5),
            ..default()
        })),
        Transform::from_translation(start_pos),
    ));

    commands.spawn((
        BirdEyeCamera { last_normal: Vec3::Y },
        Camera3d::default(),
        Transform::from_xyz(0.0, settings.radius + settings.camera_height, 0.0)
            .looking_at(start_pos, -Vec3::Z),
    ));
}

fn player_movement_system(
    joy: Res<VjoyOutput>,
    time: Res<Time>,
    settings: Res<PlanetSettings>,
    q_camera: Query<&Transform, (With<BirdEyeCamera>, Without<PlayerBall>)>,
    mut q_player: Query<&mut Transform, With<PlayerBall>>,
) {
    let Ok(mut transform) = q_player.single_mut() else { return; };
    let Ok(cam_transform) = q_camera.single() else { return; };
    
    if joy.dir.length() < 0.01 { return; }

    let normal = transform.translation.normalize();

    let move_direction = (cam_transform.right() * joy.dir.x + cam_transform.up() * joy.dir.y).normalize();
    let movement = move_direction * settings.player_speed * time.delta_secs();
    
    transform.translation += movement;

    transform.translation = transform.translation.normalize() * (settings.radius + settings.player_radius);

    if let Ok(axis) = Dir3::new(movement.cross(normal)) {
        let angle = movement.length() / settings.player_radius;
        transform.rotate_axis(axis, angle);
    }
}

fn camera_follow_system(
    settings: Res<PlanetSettings>,
    q_player: Query<&Transform, (With<PlayerBall>, Without<BirdEyeCamera>)>,
    mut q_camera: Query<(&mut Transform, &mut BirdEyeCamera)>,
) {
    let Ok(player_transform) = q_player.single() else { return; };
    let Ok((mut cam_transform, mut camera_state)) = q_camera.single_mut() else { return; };

    let new_normal = player_transform.translation.normalize();
    let old_normal = camera_state.last_normal;

    let delta_rotation = Quat::from_rotation_arc(old_normal, new_normal);

    cam_transform.translation = delta_rotation * cam_transform.translation;
    cam_transform.rotation = delta_rotation * cam_transform.rotation;

    cam_transform.translation = new_normal * (settings.radius + settings.camera_height);
    
    camera_state.last_normal = new_normal;
}

#[cfg(feature = "dev")]
fn sync_planet_visuals(
    settings: Res<PlanetSettings>,
    mut q_planet: Query<&mut Transform, With<Planet>>,
) {
    if let Ok(mut transform) = q_planet.single_mut() {
        let scale = settings.radius / 100.0;
        transform.scale = Vec3::splat(scale);
    }
}

#[cfg(feature = "dev")]
fn sync_player_visuals(
    settings: Res<PlanetSettings>,
    mut q_player: Query<&mut Transform, With<PlayerBall>>,
) {
    if let Ok(mut transform) = q_player.single_mut() {
        let scale = settings.player_radius / 2.0;
        transform.scale = Vec3::splat(scale);
    }
}