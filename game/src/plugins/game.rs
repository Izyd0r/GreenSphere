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
        PlayerBall::default(),
        Mesh3d(meshes.add(Sphere::new(1.0).mesh().ico(5).unwrap())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.0, 1.0, 0.5),
            ..default()
        })),
        Transform::from_translation(spawn_pos).with_scale(Vec3::splat(settings.player_radius)),
    ));

    commands.spawn((
        BirdEyeCamera { 
            // Initial forward points toward World North (Y) 
            // unless we are at the pole, then Z.
            manual_forward: Vec3::Z, 
        },
        Camera3d::default(),
        Transform::from_xyz(0.0, settings.radius + settings.camera_height, 0.0)
            .looking_at(Vec3::ZERO, -Vec3::Z),
    ));
}

fn player_movement_system(
    joy: Res<VjoyOutput>,
    time: Res<Time>,
    settings: Res<PlanetSettings>,
    q_camera: Query<&Transform, (With<BirdEyeCamera>, Without<PlayerBall>)>,
    mut q_player: Query<(&mut Transform, &mut PlayerBall)>,
) {
    let Ok((mut transform, mut player)) = q_player.single_mut() else { return; };
    let Ok(cam_transform) = q_camera.single() else { return; };
    let dt = time.delta_secs();

    let normal = transform.translation.normalize();

    // 1. ACCELERATION
    if joy.dir.length() > 0.01 {
        let move_dir = (cam_transform.right() * joy.dir.x + cam_transform.up() * joy.dir.y).normalize();
        player.velocity += move_dir * settings.acceleration * dt;
    }

    // 2. FRICTION
    player.velocity *= settings.friction.powf(dt * 60.0);

    // 3. PROJECT VELOCITY (Keep it flat on the ground)
    player.velocity = player.velocity - normal * player.velocity.dot(normal);

    // 4. APPLY POSITION
    transform.translation += player.velocity * dt;
    transform.translation = transform.translation.normalize() * (settings.radius + settings.player_radius);

    // --- THE FIX FOR THE CRASH ---
    // We calculate the axis, then safely try to create a Dir3.
    // Dir3::new returns 'Err' if the vector is too small or zero.
    let raw_axis = player.velocity.cross(normal);
    if let Ok(rotation_axis) = Dir3::new(raw_axis) {
        let rotation_speed = player.velocity.length() * dt / settings.player_radius;
        transform.rotate_axis(rotation_axis, rotation_speed);
    }
}

fn camera_follow_system(
    settings: Res<PlanetSettings>,
    q_player: Query<&Transform, (With<PlayerBall>, Without<BirdEyeCamera>)>,
    mut q_camera: Query<(&mut Transform, &mut BirdEyeCamera)>,
) {
    let Ok(player_transform) = q_player.single() else { return; };
    let Ok((mut cam_transform, mut camera_state)) = q_camera.single_mut() else { return; };

    let ball_pos = player_transform.translation;
    let ball_normal = ball_pos.normalize();

    // 1. POSITIONING:
    // Place the camera exactly (Radius + Height) away from the center, 
    // aligned with the ball's position.
    let target_translation = ball_normal * (settings.radius + settings.camera_height);
    
    // Smoothly lerp the position to follow the ball's inertia
    cam_transform.translation = cam_transform.translation.lerp(target_translation, settings.camera_smoothing);

    // 2. ORIENTATION (The Look-At Fix):
    // We want the camera to look at the ball, but we need a stable "Up" vector
    // so it doesn't spin. We use our stored "manual_forward".
    
    // We project the stored forward onto the surface tangent so it's always "flat"
    let forward_on_tangent = (camera_state.manual_forward - ball_normal * camera_state.manual_forward.dot(ball_normal)).normalize();
    
    // Update the camera to look at the ball.
    // 'ball_pos' is where it looks.
    // 'forward_on_tangent' is what the camera considers "Up" (top of your monitor).
    cam_transform.look_at(ball_pos, forward_on_tangent);

    // 3. STABILIZE FORWARD:
    // Save the orientation for the next frame to prevent "Gimbal Lock" or spinning
    camera_state.manual_forward = forward_on_tangent;
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