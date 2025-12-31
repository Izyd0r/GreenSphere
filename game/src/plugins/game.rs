use bevy::prelude::*;
use bevy::platform::collections::HashSet;

use rand::Rng;

use crate::resources::vjoy_output::VjoyOutput;
use crate::resources::planet_settings::PlanetSettings;
use crate::components::planet::{Planet, PlanetData, TileState};
use crate::components::player_ball::PlayerBall;
use crate::components::planet_pivot::PlanetPivot;
use crate::components::camera::BirdEyeCamera;
use crate::resources::enemy_settings::EnemySettings;
use crate::components::machine::AlienMachine;
use crate::components::factory::{AlienFactory, FactorySpawner};
use crate::resources::dash_settings::DashSettings;
use crate::resources::dash_state::DashState;

pub(crate) fn plugin(app: &mut App) {
    app
        .init_resource::<PlanetSettings>()
        .init_resource::<EnemySettings>()
        .register_type::<PlanetSettings>()
        .register_type::<EnemySettings>()
        .add_systems(Startup, setup_game)
        .add_systems(PostStartup, (build_adjacency, spawn_factories).chain())        
        .add_systems(Update, (
            planetary_control_system,
            tile_restoration_system,
            sync_visuals,
            factory_spawner_system,
            alien_ai_system,
            billboard_system, 
            pollution_lifecycle_system,
            enemy_collision_system,
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
    mesh.asset_usage = bevy::asset::RenderAssetUsages::RENDER_WORLD | bevy::asset::RenderAssetUsages::MAIN_WORLD;

    commands.spawn((
        Planet,
        PlanetData { 
            vertex_states: vec![TileState::Wasteland; vertex_count],
            adjacency: Vec::new(), 
        },
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

fn build_adjacency(
    mut q_planet: Query<(&Mesh3d, &mut PlanetData), With<Planet>>,
    mut meshes: ResMut<Assets<Mesh>>, 
) {
    let Ok((mesh_handle, mut planet_data)) = q_planet.single_mut() else { return; };
    let Some(mesh) = meshes.get_mut(mesh_handle) else { return; };

    let Some(bevy::mesh::VertexAttributeValues::Float32x3(v_pos)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else { return; };
    let vertex_count = v_pos.len();

    let mut pos_map: std::collections::HashMap<[i32; 3], Vec<usize>> = std::collections::HashMap::new();
    for (idx, pos) in v_pos.iter().enumerate() {
        let key = [(pos[0] * 1000.0) as i32, (pos[1] * 1000.0) as i32, (pos[2] * 1000.0) as i32];
        pos_map.entry(key).or_default().push(idx);
    }

    let mut adj = vec![std::collections::HashSet::new(); vertex_count];

    for i in (0..vertex_count).step_by(3) {
        if i + 2 >= vertex_count { break; }
        let corners = [i, i + 1, i + 2];

        for j in 0..3 {
            let p_start = v_pos[corners[j]];
            let p_end = v_pos[corners[(j + 1) % 3]];
            
            let k_start = [(p_start[0] * 1000.0) as i32, (p_start[1] * 1000.0) as i32, (p_start[2] * 1000.0) as i32];
            let k_end = [(p_end[0] * 1000.0) as i32, (p_end[1] * 1000.0) as i32, (p_end[2] * 1000.0) as i32];

            for &v1 in &pos_map[&k_start] {
                for &v2 in &pos_map[&k_end] {
                    adj[v1].insert(v2);
                    adj[v2].insert(v1);
                }
            }
        }
    }

    for siblings in pos_map.values() {
        for &v1 in siblings {
            for &v2 in siblings {
                if v1 != v2 { adj[v1].insert(v2); }
            }
        }
    }

    planet_data.adjacency = adj.into_iter().map(|set| set.into_iter().collect()).collect();
}

fn planetary_control_system(
    joy: Res<VjoyOutput>,
    time: Res<Time>,
    settings: Res<PlanetSettings>,
    dash_settings: Res<DashSettings>,
    state: Res<DashState>, 
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

    if state.is_active && state.duration_timer >= (dash_settings.dash_duration - dt) {
        let dash_dir = if joy.dir.length() > 0.1 {
            Vec3::new(joy.dir.x, 0.0, -joy.dir.y).normalize()
        } else {
            ball.current_velocity.normalize_or_zero()
        };

        ball.current_velocity += dash_dir * dash_settings.dash_force * size_factor;
    }
    
    if state.is_active {
        ball.current_velocity *= 0.99f32.powf(dt * 60.0);
        
    } else {
        ball.current_velocity *= settings.friction.powf(dt * 60.0);
        
        if ball.current_velocity.length() > effective_max_speed {
            let target_vel = ball.current_velocity.normalize() * effective_max_speed;
            ball.current_velocity = ball.current_velocity.lerp(target_vel, 0.1);
        }
    }

    let current_speed = ball.current_velocity.length();

    if current_speed > 0.1 {
        let rotation_x = Quat::from_rotation_x(ball.current_velocity.z * dt / settings.radius);
        let rotation_z = Quat::from_rotation_z(-ball.current_velocity.x * dt / settings.radius);
        pivot_trans.rotation = pivot_trans.rotation * rotation_x * rotation_z;

        let roll_speed = current_speed * dt / settings.player_radius;
        let roll_axis_raw = Vec3::new(ball.current_velocity.z, 0.0, -ball.current_velocity.x);
        if let Ok(axis) = Dir3::new(roll_axis_raw.normalize_or_zero()) {
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

fn spawn_factories(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>, 
    mut materials: ResMut<Assets<StandardMaterial>>,
    settings: Res<PlanetSettings>,
    enemy_settings: Res<EnemySettings>,
    mut q_planet: Query<(&mut PlanetData, &Mesh3d, &Transform), With<Planet>>,
) {
    let factory_texture = asset_server.load("textures/factory.png");
    let factory_height = 12.0;


    let material_factory = materials.add(StandardMaterial {
        base_color_texture: Some(factory_texture),
        alpha_mode: AlphaMode::Mask(0.5), 
        unlit: true, 
        cull_mode: None, 
        ..default()
    });
    let mesh_2d = meshes.add(Rectangle::new(12.0, factory_height));

    let mut rng = rand::rng();
    let Ok((mut planet_data, mesh_handle, planet_transform)) = q_planet.single_mut() else { return; };
    
    let Some(mesh) = meshes.get_mut(mesh_handle) else { return; };

    for _ in 0..enemy_settings.factory_count {
        let theta = rng.random_range(0.0..std::f32::consts::TAU);
        let phi = (rng.random_range(-1.0..1.0) as f32).acos();
        let normal = Vec3::new(phi.sin() * theta.cos(), phi.sin() * theta.sin(), phi.cos());
        let offset_height = factory_height / 2.0;
        let spawn_pos = normal * (settings.radius + offset_height);

        commands.spawn((
            AlienFactory,
            FactorySpawner { 
                timer: Timer::from_seconds(enemy_settings.machine_spawn_interval, TimerMode::Repeating) 
            },
            Mesh3d(mesh_2d.clone()),
            MeshMaterial3d(material_factory.clone()),
            Transform::from_translation(spawn_pos)
                .looking_at(spawn_pos + normal, Vec3::Y), 
        ));

        let stain_pos = normal * settings.radius;
        pollute_area(
            &mut planet_data, 
            mesh, 
            stain_pos - planet_transform.translation, 
            enemy_settings.pollution_radius / settings.radius,
            enemy_settings.pollution_color
        );
    }
}

fn pollute_area(
    data: &mut PlanetData,
    mesh: &mut Mesh,
    local_pos: Vec3,
    radius_normalized: f32,
    color: [f32; 4]
) {
    let radius_sq = radius_normalized * radius_normalized;
    let Some(bevy::mesh::VertexAttributeValues::Float32x3(v_pos)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else { return; };
    let v_pos = v_pos.clone();

    if let Some(bevy::mesh::VertexAttributeValues::Float32x4(v_col)) = mesh.attribute_mut(Mesh::ATTRIBUTE_COLOR) {
        for i in 0..v_pos.len() {
            let v = Vec3::from(v_pos[i]);
            if v.distance_squared(local_pos.normalize()) < radius_sq {
                data.vertex_states[i] = TileState::Polluted;
                v_col[i] = color;
            }
        }
    }
}

fn pollution_lifecycle_system(
    time: Res<Time>,
    enemy_settings: Res<EnemySettings>,
    settings: Res<PlanetSettings>,
    q_factories: Query<&Transform, With<AlienFactory>>,
    mut q_planet: Query<(&Mesh3d, &mut PlanetData), With<Planet>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut timer: Local<f32>,
) {
    let Ok((mesh_handle, mut planet_data)) = q_planet.single_mut() else { return; };
    if planet_data.adjacency.is_empty() { return; }

    *timer += time.delta_secs();
    if *timer < enemy_settings.spread_tick_rate { return; } 
    *timer = 0.0;

    let Some(mesh) = meshes.get_mut(mesh_handle) else { return; };
    let Some(bevy::mesh::VertexAttributeValues::Float32x3(v_pos_attr)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else { return; };
    
    let factory_positions: Vec<Vec3> = q_factories.iter().map(|t| t.translation).collect();
    let root_radius_sq = (enemy_settings.pollution_radius * 0.5).powi(2);
    let connect_radius_sq = (enemy_settings.pollution_radius * 1.2).powi(2);

    let mut to_infect = std::collections::HashSet::new();
    let mut rng = rand::rng();

    for &f_pos in &factory_positions {
        let mut factory_has_pollution = false;
        let mut root_vertices = Vec::new();

        for (idx, v) in v_pos_attr.iter().enumerate() {
            let world_v_pos = Vec3::from(*v) * settings.radius;
            if f_pos.distance_squared(world_v_pos) < root_radius_sq {
                root_vertices.push(idx);
                if planet_data.vertex_states[idx] == TileState::Polluted {
                    factory_has_pollution = true;
                    break; 
                }
            }
        }

        if !factory_has_pollution && !root_vertices.is_empty() {
            if rand::random::<f32>() < 0.2 {
                for &idx in &root_vertices {
                    to_infect.insert(idx);
                    for &sibling in &planet_data.adjacency[idx] {
                        to_infect.insert(sibling);
                    }
                }
            }
        }
    }

    let mut active_indices = std::collections::VecDeque::new();
    let mut is_active = vec![false; planet_data.vertex_states.len()];

    for (idx, state) in planet_data.vertex_states.iter().enumerate() {
        if *state == TileState::Polluted {
            let world_v_pos = Vec3::from(v_pos_attr[idx]) * settings.radius;
            if factory_positions.iter().any(|f_pos| f_pos.distance_squared(world_v_pos) < connect_radius_sq) {
                is_active[idx] = true;
                active_indices.push_back(idx);
            }
        }
    }

    while let Some(current) = active_indices.pop_front() {
        for &neighbor in &planet_data.adjacency[current] {
            if !is_active[neighbor] && planet_data.vertex_states[neighbor] == TileState::Polluted {
                is_active[neighbor] = true;
                active_indices.push_back(neighbor);
            }
        }
    }

    for (idx, active) in is_active.iter().enumerate() {
        if !*active { continue; }

        let candidates: Vec<&usize> = planet_data.adjacency[idx].iter()
            .filter(|&&n| planet_data.vertex_states[n] != TileState::Polluted)
            .collect();

        if candidates.is_empty() { continue; }
        
        let &target_idx = candidates[rng.random_range(0..candidates.len())];

        if rand::random::<f32>() < enemy_settings.natural_spread_chance {
            to_infect.insert(target_idx);
            for &sibling in &planet_data.adjacency[target_idx] {
                to_infect.insert(sibling);
            }
        }
    }

    if !to_infect.is_empty() {
        if let Some(bevy::mesh::VertexAttributeValues::Float32x4(v_col)) = mesh.attribute_mut(Mesh::ATTRIBUTE_COLOR) {
            for idx in to_infect {
                planet_data.vertex_states[idx] = TileState::Polluted;
                v_col[idx] = enemy_settings.pollution_color;
            }
        }
    }
}

fn factory_spawner_system(
    time: Res<Time>,
    settings: Res<PlanetSettings>,
    enemy_settings: Res<EnemySettings>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    mut q_factories: Query<(&Transform, &mut FactorySpawner), With<AlienFactory>>,
    mut commands: Commands,
    mut local_assets: Local<Option<(Handle<Mesh>, Handle<StandardMaterial>)>>,
) {
    let (machine_mesh, machine_mat) = local_assets.get_or_insert_with(|| {
        let mesh = meshes.add(Rectangle::new(6.0, 6.0));
        let tex = asset_server.load("textures/machine.png");
        let mat = materials.add(StandardMaterial {
            base_color_texture: Some(tex),
            alpha_mode: AlphaMode::Mask(0.5),
            cull_mode: None,
            unlit: true,
            ..default()
        });
        (mesh, mat)
    }).clone();

    for (f_transform, mut spawner) in q_factories.iter_mut() {
        spawner.timer.set_duration(std::time::Duration::from_secs_f32(enemy_settings.machine_spawn_interval));
        spawner.timer.tick(time.delta());

        if spawner.timer.just_finished() {
            let mut rng = rand::rng();
            let factory_pos = f_transform.translation;
            let normal = factory_pos.normalize();

            let random_vec = Vec3::new(
                rng.random_range(-1.0..1.0),
                rng.random_range(-1.0..1.0),
                rng.random_range(-1.0..1.0)
            );
            
            let tangent = (random_vec - normal * random_vec.dot(normal)).normalize();
            let spawn_pos = (factory_pos + tangent * 15.0).normalize() * (settings.radius + 3.0);

            commands.spawn((
                AlienMachine { velocity: Vec3::ZERO },
                Mesh3d(machine_mesh.clone()),
                MeshMaterial3d(machine_mat.clone()),
                Transform::from_translation(spawn_pos),
            ));
        }
    }
}

fn alien_ai_system(
    time: Res<Time>,
    settings: Res<PlanetSettings>,
    enemy_settings: Res<EnemySettings>,
    q_player: Query<&GlobalTransform, With<PlayerBall>>,
    mut q_machines: Query<(&mut Transform, &mut AlienMachine)>,
) {
    let Ok(player_gtrans) = q_player.single() else { return; };
    let player_pos = player_gtrans.translation();
    let dt = time.delta_secs();

    for (mut transform, mut machine) in q_machines.iter_mut() {
        let machine_pos = transform.translation;
        let dist = machine_pos.distance(player_pos);

        if dist < enemy_settings.machine_detection_range && dist > 5.0 {
            let normal = machine_pos.normalize();
            
            let to_player = player_pos - machine_pos;
            let tangent_dir = (to_player - normal * to_player.dot(normal)).normalize();

            machine.velocity += tangent_dir * enemy_settings.machine_acceleration * dt;
        }

        machine.velocity *= 0.95f32.powf(dt * 60.0);
        if machine.velocity.length() > enemy_settings.machine_speed {
            machine.velocity = machine.velocity.normalize() * enemy_settings.machine_speed;
        }

        transform.translation += machine.velocity * dt;
        transform.translation = transform.translation.normalize() * (settings.radius + 3.0);
    }
}

fn billboard_system(
    q_cam: Query<&GlobalTransform, With<BirdEyeCamera>>,
    mut q_billboards: Query<&mut Transform, Or<(With<AlienFactory>, With<AlienMachine>)>>,
) {
    let Ok(cam_gtrans) = q_cam.single() else { return; };
    let cam_pos = cam_gtrans.translation();

    for mut transform in q_billboards.iter_mut() {
        let pos = transform.translation;
        let normal = pos.normalize();
        let to_cam = cam_pos - pos;
        let target_dir = to_cam - normal * to_cam.dot(normal);

        if target_dir.length_squared() > 0.001 {
            let look_target = pos + target_dir;
            *transform = transform.looking_at(look_target, normal);
        }
    }
}

fn enemy_collision_system(
    mut commands: Commands,
    dash_state: Res<DashState>,
    settings: Res<PlanetSettings>,
    q_player: Query<&GlobalTransform, With<PlayerBall>>,
    q_machines: Query<(Entity, &GlobalTransform), With<AlienMachine>>,
    q_factories: Query<(Entity, &GlobalTransform), With<AlienFactory>>,
) {
    if !dash_state.is_active { return; }

    let Ok(player_gtrans) = q_player.single() else { return; };
    let player_pos = player_gtrans.translation();
    let player_radius = settings.player_radius;

    for (entity, machine_gtrans) in q_machines.iter() {
        if player_pos.distance(machine_gtrans.translation()) < player_radius + 3.0 {
            commands.entity(entity).despawn_children();
            commands.entity(entity).despawn();
        }
    }

    for (entity, factory_gtrans) in q_factories.iter() {
        if player_pos.distance(factory_gtrans.translation()) < player_radius + 6.0 {
            commands.entity(entity).despawn_children();
            commands.entity(entity).despawn();
        }
    }
}