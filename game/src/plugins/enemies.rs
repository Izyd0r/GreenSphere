use bevy::prelude::*;

use rand::Rng;

use crate::prelude::planet_settings::*;
use crate::prelude::planet::*;
use crate::prelude::player_ball::*;
use crate::prelude::planet_pivot::*;
use crate::prelude::camera::*;
use crate::prelude::enemy_settings::*;
use crate::prelude::machine::*;
use crate::prelude::factory::*;
use crate::prelude::dash_state::*;
use crate::prelude::ui::*;
use crate::prelude::orbs::*;
use crate::prelude::score::*;
use crate::prelude::session_time::*;
use crate::prelude::player_profile::*;
use crate::prelude::reset_target::*;
use crate::prelude::notification_timer::*;
use crate::prelude::player_settings::*;

use crate::prelude::*;

pub(crate) fn plugin(app: &mut App) {
    app
        .init_resource::<EnemySettings>()
        .register_type::<EnemySettings>()
        .add_systems(OnEnter(GameState::Playing), (
            spawn_factories,
        ).chain())        
        .add_systems(Update, (
            pollution_lifecycle_system,
            factory_spawner_system, 
            alien_ai_system, 
            billboard_system,
            factory_director_system,
        ).run_if(in_state(GameState::Playing)).run_if(any_with_component::<PlayerBall>));
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

    let Ok((mut planet_data, mesh_handle, planet_transform)) = q_planet.single_mut() else { return; };
    let Some(mesh) = meshes.get_mut(mesh_handle) else { return; };
    let mut rng = rand::rng();

    for _ in 0..enemy_settings.factory_count {
        let theta = rng.random_range(0.0..std::f32::consts::TAU);
        let phi = (rng.random_range(-1.0..1.0) as f32).acos();
        let normal = Vec3::new(phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin());
        
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
            SessionUi,
        ));

        let stain_pos = normal * settings.radius;
        pollute_area(
            &mut planet_data, 
            mesh, 
            stain_pos - planet_transform.translation, 
            enemy_settings.pollution_radius / settings.radius,
        );
    }
}

fn pollute_area(
    data: &mut PlanetData,
    mesh: &mut Mesh,
    local_pos: Vec3,
    radius_normalized: f32,
) {
    let radius_sq = radius_normalized * radius_normalized;
    let Some(bevy::mesh::VertexAttributeValues::Float32x3(v_pos_attr)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else { return; };
    let v_pos = v_pos_attr.clone();

    if let Some(bevy::mesh::VertexAttributeValues::Float32x2(v_uv)) = mesh.attribute_mut(Mesh::ATTRIBUTE_UV_0) {
        for i in 0..v_pos.len() {
            let v = Vec3::from(v_pos[i]);
            if v.distance_squared(local_pos.normalize()) < radius_sq {
                data.vertex_states[i] = TileState::Polluted;
                
                let tri_start = (i / 3) * 3;
                v_uv[tri_start]     = [0.0, 0.5];
                v_uv[tri_start + 1] = [0.5, 0.5];
                v_uv[tri_start + 2] = [0.25, 1.0];
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
                    for &sibling in &planet_data.adjacency[idx] { to_infect.insert(sibling); }
                }
            }
        }
    }

    let mut is_active = vec![false; planet_data.vertex_states.len()];
    let mut active_queue = std::collections::VecDeque::new();
    for (idx, state) in planet_data.vertex_states.iter().enumerate() {
        if *state == TileState::Polluted {
            let world_v_pos = Vec3::from(v_pos_attr[idx]) * settings.radius;
            if factory_positions.iter().any(|f| f.distance_squared(world_v_pos) < connect_radius_sq) {
                is_active[idx] = true;
                active_queue.push_back(idx);
            }
        }
    }
    while let Some(curr) = active_queue.pop_front() {
        for &n in &planet_data.adjacency[curr] {
            if !is_active[n] && planet_data.vertex_states[n] == TileState::Polluted {
                is_active[n] = true;
                active_queue.push_back(n);
            }
        }
    }

    for (idx, active) in is_active.iter().enumerate() {
        if !*active { continue; }
        let neighbors = &planet_data.adjacency[idx];
        let targets: Vec<&usize> = neighbors.iter().filter(|&&n| planet_data.vertex_states[n] != TileState::Polluted).collect();
        if targets.is_empty() { continue; }
        let &target_idx = targets[rng.random_range(0..targets.len())];
        if rand::random::<f32>() < enemy_settings.natural_spread_chance {
            to_infect.insert(target_idx);
            for &sibling in &planet_data.adjacency[target_idx] { to_infect.insert(sibling); }
        }
    }

    if let Some(bevy::mesh::VertexAttributeValues::Float32x2(v_uv)) = mesh.attribute_mut(Mesh::ATTRIBUTE_UV_0) {
        for idx in to_infect {
            planet_data.vertex_states[idx] = TileState::Polluted;
            
            let tri_start = (idx / 3) * 3;
            v_uv[tri_start]     = [0.0, 0.5];
            v_uv[tri_start + 1] = [0.5, 0.5];
            v_uv[tri_start + 2] = [0.25, 1.0];
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

fn factory_director_system(
    time: Res<Time>,
    settings: Res<PlanetSettings>,
    mut enemy_settings: ResMut<EnemySettings>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    mut q_planet: Query<(&mut PlanetData, &Mesh3d, &Transform), With<Planet>>,
    mut q_notice: Query<(&mut Visibility, &mut NotificationTimer, &Children), With<FactoryNotificationText>>,
    mut q_text_color: Query<&mut TextColor>,
    mut local_assets: Local<Option<(Handle<Mesh>, Handle<StandardMaterial>)>>,
) {
    let dt = time.delta_secs();
    
    enemy_settings.difficulty_scale += enemy_settings.difficulty_growth_rate * dt;
    let current_diff = enemy_settings.difficulty_scale;

    enemy_settings.factory_spawn_timer.tick(time.delta().mul_f32(current_diff));

    if enemy_settings.factory_spawn_timer.just_finished() {
        let mut rng = rand::rng();
        let theta = rng.random_range(0.0..std::f32::consts::TAU);
        let phi = (rng.random_range(-1.0..1.0) as f32).acos();
        let normal = Vec3::new(phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin());
        
        let spawn_pos = normal * (settings.radius + 6.0);

        let (f_mesh, f_mat) = local_assets.get_or_insert_with(|| {
            (
                meshes.add(Rectangle::new(12.0, 12.0)), 
                materials.add(StandardMaterial {
                    base_color_texture: Some(asset_server.load("textures/factory.png")),
                    alpha_mode: AlphaMode::Mask(0.5),
                    cull_mode: None, 
                    unlit: true,
                    ..default()
                })
            )
        }).clone();

        commands.spawn((
            AlienFactory,
            FactorySpawner { 
                timer: Timer::from_seconds(10.0 / current_diff, TimerMode::Repeating) 
            },
            Mesh3d(f_mesh),
            MeshMaterial3d(f_mat),
            Transform::from_translation(spawn_pos).looking_at(spawn_pos + normal, Vec3::Y),
            Visibility::Inherited,
            InheritedVisibility::default(),
            SessionUi,
        ));

        if let Ok((mut planet_data, mesh_handle, planet_transform)) = q_planet.single_mut() {
            if let Some(mesh) = meshes.get_mut(mesh_handle) {
                pollute_area(
                    &mut planet_data, 
                    mesh, 
                    spawn_pos - planet_transform.translation, 
                    enemy_settings.pollution_radius / settings.radius,
                );
            }
        }
        
        if let Ok((mut vis, mut timer, children)) = q_notice.single_mut() {
            *vis = Visibility::Inherited;
            timer.0.reset(); 
            for &child in children {
                if let Ok(mut color) = q_text_color.get_mut(child) {
                    color.0 = Color::srgb(1.0, 0.0, 0.0);
                }
            }
        }
    }
}