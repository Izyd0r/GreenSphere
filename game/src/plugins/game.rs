use bevy::prelude::*;

use crate::resources::vjoy_output::VjoyOutput;
use crate::resources::planet_settings::PlanetSettings;
use crate::components::planet::{Planet, PlanetData, TileState};
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
            tile_restoration_system,
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

    let mut mesh = Sphere::new(1.0)
        .mesh()
        .ico(settings.subdivisions as u32)
        .unwrap();

    mesh.duplicate_vertices();
    mesh.compute_flat_normals();

    let vertex_count = mesh.count_vertices();
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![[0.15, 0.15, 0.15, 1.0]; vertex_count]);
    
    mesh.asset_usage = bevy::asset::RenderAssetUsages::default();

    commands.spawn((
        Planet,
        PlanetData { vertex_states: vec![TileState::Wasteland; vertex_count] },
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            perceptual_roughness: 0.9,
            ..default()
        })),
        Transform::from_scale(Vec3::splat(settings.radius)),
    ));

    let spawn_pos = Vec3::new(0.0, settings.radius + settings.player_radius, 0.0);
    commands.spawn((
        PlayerBall,
        Mesh3d(meshes.add(Sphere::new(1.0).mesh().ico(5).unwrap())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.0, 1.0, 0.5),
            ..default()
        })),
        Transform::from_translation(spawn_pos).with_scale(Vec3::splat(settings.player_radius)),
    ));

    commands.spawn((
        BirdEyeCamera { last_normal: Vec3::Y },
        Camera3d::default(),
        Transform::from_xyz(0.0, settings.radius + settings.camera_height, 0.0)
            .looking_at(spawn_pos, -Vec3::Z),
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

    if new_normal.distance_squared(old_normal) > 0.00001 {
        let delta_rotation = Quat::from_rotation_arc(old_normal, new_normal);
        
        cam_transform.translation = delta_rotation * cam_transform.translation;
        cam_transform.rotation = delta_rotation * cam_transform.rotation;
    }

    cam_transform.translation = new_normal * (settings.radius + settings.camera_height);
    
    camera_state.last_normal = new_normal;
}

fn tile_restoration_system(
    settings: Res<PlanetSettings>,
    q_player: Query<&Transform, With<PlayerBall>>,
    mut q_planet: Query<(&Transform, &Mesh3d, &mut PlanetData), With<Planet>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let Ok(player_transform) = q_player.single() else { return; };
    let Ok((planet_transform, mesh_handle, mut planet_data)) = q_planet.single_mut() else { return; };
    let Some(mesh) = meshes.get_mut(mesh_handle) else { return; };

    let player_rel_pos = player_transform.translation - planet_transform.translation;
    
    let local_player_pos = player_rel_pos / settings.radius;
    
    let brush_size = (settings.player_radius * 1.5) / settings.radius;
    let brush_sq = brush_size * brush_size;

    let Some(bevy::mesh::VertexAttributeValues::Float32x3(v_pos)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else { return; };
    let v_pos_local = v_pos.clone(); 

    if let Some(bevy::mesh::VertexAttributeValues::Float32x4(v_col)) = mesh.attribute_mut(Mesh::ATTRIBUTE_COLOR) {
        for i in 0..v_pos_local.len() {
            if planet_data.vertex_states[i] == TileState::Healthy {
                continue;
            }

            let v = Vec3::from(v_pos_local[i]);
            
            if v.distance_squared(local_player_pos) < brush_sq {
                planet_data.vertex_states[i] = TileState::Healthy;
                v_col[i] = [0.0, 1.0, 0.4, 1.0];
            }
        }
    }
}

#[cfg(feature = "dev")]
fn sync_planet_visuals(
    settings: Res<PlanetSettings>,
    mut q_planet: Query<&mut Transform, With<Planet>>,
) {
    if let Ok(mut transform) = q_planet.single_mut() {
        transform.scale = Vec3::splat(settings.radius);
    }
}

#[cfg(feature = "dev")]
fn sync_player_visuals(
    settings: Res<PlanetSettings>,
    mut q_player: Query<&mut Transform, With<PlayerBall>>,
) {
    if let Ok(mut transform) = q_player.single_mut() {
        transform.scale = Vec3::splat(settings.player_radius);
    }
}