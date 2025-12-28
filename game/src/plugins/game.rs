use bevy::prelude::*;

use crate::resources::vjoy_output::VjoyOutput;
use crate::resources::planet_settings::PlanetSettings;
use crate::components::planet::{Planet, PlanetData, TileState};
use crate::components::player_ball::PlayerBall;
use crate::components::planet_pivot::PlanetPivot;
use crate::components::camera::BirdEyeCamera;

pub(crate) fn plugin(app: &mut App) {
    app
        .init_resource::<PlanetSettings>()
        .register_type::<PlanetSettings>()
        .add_systems(Startup, setup_game)
        .add_systems(Update, (
            planetary_control_system,
            tile_restoration_system,
            sync_visuals,
        ).chain());
}

fn setup_game(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    settings: Res<PlanetSettings>,
) {
    commands.spawn((
        DirectionalLight { illuminance: 7000.0, shadows_enabled: true, ..default() },
        Transform::from_xyz(100.0, 100.0, 100.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    let mut mesh = Sphere::new(1.0).mesh().ico(settings.subdivisions as u32).unwrap();
    mesh.duplicate_vertices();
    mesh.compute_flat_normals();
    let vertex_count = mesh.count_vertices();
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![[0.15, 0.15, 0.15, 1.0]; vertex_count]);
    mesh.asset_usage = bevy::asset::RenderAssetUsages::default();

    commands.spawn((
        Planet,
        PlanetData { vertex_states: vec![TileState::Wasteland; vertex_count] },
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::WHITE, ..default() })),
        Transform::from_scale(Vec3::splat(settings.radius)),
    ));

    commands.spawn((
        PlanetPivot,
        Transform::IDENTITY,
        Visibility::default(),
    ))
    .with_children(|parent| {
        parent.spawn((
            PlayerBall { current_velocity: Vec3::ZERO }, 
            Mesh3d(meshes.add(Sphere::new(1.0).mesh().ico(5).unwrap())),
            MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.0, 1.0, 0.5), ..default() })),
            Transform::from_xyz(0.0, settings.radius + settings.player_radius, 0.0)
                .with_scale(Vec3::splat(settings.player_radius)),
        ));

        parent.spawn((
            BirdEyeCamera,
            Camera3d::default(),
            Transform::from_xyz(0.0, settings.radius + settings.camera_height, 0.0)
                .looking_at(Vec3::new(0.0, settings.radius, 0.0), -Vec3::Z),
        ));
    });
}

fn planetary_control_system(
    joy: Res<VjoyOutput>,
    time: Res<Time>,
    settings: Res<PlanetSettings>,
    mut q_pivot: Query<&mut Transform, With<PlanetPivot>>,
    mut q_player: Query<(&mut PlayerBall, &mut Transform), (Without<PlanetPivot>, Without<BirdEyeCamera>)>,
) {
    let Ok(mut pivot_trans) = q_pivot.single_mut() else { return; };
    let Ok((mut ball, mut ball_trans)) = q_player.single_mut() else { return; };
    let dt = time.delta_secs();

    let size_factor = (4.0 / settings.player_radius).clamp(0.2, 2.0);
    
    let effective_accel = settings.acceleration * size_factor;
    let effective_max_speed = settings.max_speed * size_factor;

    if joy.dir.length() > 0.01 {
        let accel_force = Vec3::new(joy.dir.x, 0.0, -joy.dir.y) * effective_accel * dt;
        ball.current_velocity += accel_force;
    }
    
    ball.current_velocity *= settings.friction.powf(dt * 60.0);
    
    if ball.current_velocity.length() > effective_max_speed {
        ball.current_velocity = ball.current_velocity.normalize() * effective_max_speed;
    }

    let current_speed = ball.current_velocity.length();

    if current_speed > 0.1 {
        let rotation_x = Quat::from_rotation_x(ball.current_velocity.z * dt / settings.radius);
        let rotation_z = Quat::from_rotation_z(-ball.current_velocity.x * dt / settings.radius);
        pivot_trans.rotation = pivot_trans.rotation * rotation_x * rotation_z;

        let roll_speed = current_speed * dt / settings.player_radius;
        let roll_axis_raw = Vec3::new(ball.current_velocity.z, 0.0, -ball.current_velocity.x);
        if let Ok(axis) = Dir3::new(roll_axis_raw.normalize()) {
            ball_trans.rotate_axis(axis, roll_speed);
        }
    }
}


fn tile_restoration_system(
    settings: Res<PlanetSettings>,
    q_player: Query<&GlobalTransform, With<PlayerBall>>,
    mut q_planet: Query<(&Transform, &Mesh3d, &mut PlanetData), With<Planet>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let Ok(player_gtrans) = q_player.single() else { return; };
    let Ok((planet_trans, mesh_handle, mut planet_data)) = q_planet.single_mut() else { return; };
    let Some(mesh) = meshes.get_mut(mesh_handle) else { return; };

    let player_world_pos = player_gtrans.translation();
    let player_rel_pos = player_world_pos - planet_trans.translation;
    let local_player_pos = player_rel_pos / settings.radius;
    
    let brush_size = (settings.player_radius * 1.5) / settings.radius;
    let brush_sq = brush_size * brush_size;

    let Some(bevy::mesh::VertexAttributeValues::Float32x3(v_pos)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else { return; };
    let v_pos_local = v_pos.clone(); 

    if let Some(bevy::mesh::VertexAttributeValues::Float32x4(v_col)) = mesh.attribute_mut(Mesh::ATTRIBUTE_COLOR) {
        for i in 0..v_pos_local.len() {
            if planet_data.vertex_states[i] == TileState::Healthy { continue; }
            let v = Vec3::from(v_pos_local[i]);
            if (v.x - local_player_pos.x).abs() > brush_size { continue; }
            
            if v.distance_squared(local_player_pos) < brush_sq {
                planet_data.vertex_states[i] = TileState::Healthy;
                v_col[i] = [0.0, 1.0, 0.4, 1.0];
            }
        }
    }
}

fn sync_visuals(
    settings: Res<PlanetSettings>,
    mut q_planet: Query<&mut Transform, (With<Planet>, Without<PlayerBall>, Without<BirdEyeCamera>)>,
    mut q_ball: Query<&mut Transform, (With<PlayerBall>, Without<Planet>, Without<BirdEyeCamera>)>,
    mut q_cam: Query<&mut Transform, (With<BirdEyeCamera>, Without<Planet>, Without<PlayerBall>)>,
) {
    if let Ok(mut t) = q_planet.single_mut() { 
        t.scale = Vec3::splat(settings.radius); 
    }

    if let Ok(mut t) = q_ball.single_mut() { 
        t.scale = Vec3::splat(settings.player_radius); 
        t.translation.y = settings.radius + settings.player_radius;
    }

    if let Ok(mut t) = q_cam.single_mut() {
        let dynamic_zoom = settings.camera_height + (settings.player_radius * 5.0);
        t.translation.y = settings.radius + dynamic_zoom;
        
        let target = Vec3::new(0.0, settings.radius, 0.0);
        t.look_at(target, -Vec3::Z);
    }
}