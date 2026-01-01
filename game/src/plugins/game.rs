use bevy::prelude::*;
use bevy::platform::collections::HashSet;

use rand::Rng;

use crate::prelude::session_time;
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
use crate::components::ui::{HealthBarFill, HealthText, DeathMenuRoot, RestartButton, ExitButton, ScoreHudText, TimeHudText, ScoreHud, SessionUi};
use crate::components::orbs::EnergyOrb;
use crate::components::session::SessionEntity;
use crate::resources::score::{Score, ScoreMessage};
use crate::prelude::{
    vjoy_base::VjoyBase, 
    dash::DashButton,
};
use crate::resources::session_time::SessionTime;
use crate::resources::player_profile::PlayerProfile;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default, Reflect)]
pub enum GameState {
    #[default]
    MainMenu,
    Resetting,
    GameOver,
    Playing,
}

#[derive(Component)]
pub struct MainMenuRoot;

#[derive(Component)]
pub struct StartButton;


pub(crate) fn plugin(app: &mut App) {
    app
        .init_state::<GameState>()
        .init_resource::<PlanetSettings>()
        .init_resource::<EnemySettings>()
        .init_resource::<Score>()
        .init_resource::<SessionTime>()
        .init_resource::<PlayerProfile>()
        .add_message::<ScoreMessage>()
        .register_type::<Score>()
        .register_type::<PlanetSettings>()
        .register_type::<EnemySettings>()
        .add_systems(Startup, (setup_planet, build_adjacency).chain())
        .add_systems(OnEnter(GameState::MainMenu), setup_main_menu)
        .add_systems(Update, main_menu_system.run_if(in_state(GameState::MainMenu)))
        .add_systems(OnExit(GameState::MainMenu), cleanup_main_menu)
        .add_systems(OnEnter(GameState::Playing), (
            spawn_session_objects, 
            spawn_factories,
            crate::plugins::vjoy::spawn_joystick,    
            crate::plugins::vjoy::spawn_dash_button,
            spawn_health_bar,
            spawn_score_hud,
        ).chain())        
        .add_systems(Update, (
            (planetary_control_system, sync_visuals).chain(),
            
            (tile_restoration_system, pollution_lifecycle_system),
            
            (factory_spawner_system, alien_ai_system, enemy_collision_system, billboard_system),
            
            (player_health_sync_system, update_health_bar_system, death_system).chain(),
            
            (orb_spawning_system, orb_collection_system, orb_animation_system),

            (player_invincibility_system),
            
            (score_event_handler, update_score_hud_system),

            (track_session_time_system, update_time_hud_system)
        ).run_if(in_state(GameState::Playing)).run_if(any_with_component::<PlayerBall>))
        .add_systems(OnEnter(GameState::GameOver), (setup_death_menu, cleanup_game_ui))
        .add_systems(Update, death_menu_interaction_system.run_if(in_state(GameState::GameOver)))
        .add_systems(OnExit(GameState::GameOver), cleanup_death_menu)
        .add_systems(OnEnter(GameState::Resetting), world_reset_system);
}

fn setup_planet(
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
        Visibility::Inherited,
        InheritedVisibility::default(),
    ))
    .with_children(|parent| {
        parent.spawn((
            BirdEyeCamera,
            Camera3d::default(),
            Transform::from_xyz(0.0, settings.radius + settings.camera_height, 0.0)
                .looking_at(Vec3::new(0.0, settings.radius, 0.0), -Vec3::Z),
        ));
    });
}

fn spawn_session_objects(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    settings: Res<PlanetSettings>,
    q_pivot: Query<Entity, With<PlanetPivot>>,
) {
    let Ok(pivot_entity) = q_pivot.single() else { return; };

    let ball_entity = commands.spawn((
        PlayerBall { current_velocity: Vec3::ZERO, hp: 100.0, invincibility_timer: 0.0 }, 
        Mesh3d(meshes.add(Sphere::new(1.0).mesh().ico(5).unwrap())),
        MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.0, 1.0, 0.5), ..default() })),
        Transform::from_xyz(0.0, settings.radius + settings.player_radius, 0.0)
            .with_scale(Vec3::splat(settings.player_radius)),
    )).id();

    commands.entity(pivot_entity).add_child(ball_entity);
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
    mut score_msg: MessageWriter<ScoreMessage>
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
                let points = match planet_data.vertex_states[i] {
                    TileState::Wasteland => 100,
                    TileState::Polluted => 200,
                    _ => 0,
                };

                if points > 0 {
                    planet_data.vertex_states[i] = TileState::Healthy;
                    v_col[i] = [0.0, 1.0, 0.4, 1.0];
                    score_msg.write(ScoreMessage(points));
                }
            }
        }
    }
}

fn sync_visuals(
    settings: Res<PlanetSettings>,
    time: Res<Time>,
    mut q_planet: Query<&mut Transform, (With<Planet>, Without<PlayerBall>, Without<BirdEyeCamera>)>,
    mut q_ball: Query<(&PlayerBall, &mut Transform, &MeshMaterial3d<StandardMaterial>), (Without<Planet>, Without<BirdEyeCamera>)>,
    mut q_cam: Query<&mut Transform, (With<BirdEyeCamera>, Without<Planet>, Without<PlayerBall>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if let Ok(mut t) = q_planet.single_mut() { 
        t.scale = Vec3::splat(settings.radius); 
    }

    if let Ok((player_logic, mut t, mat_handle)) = q_ball.single_mut() { 
        t.scale = Vec3::splat(settings.player_radius); 
        t.translation.y = settings.radius + settings.player_radius;

        if let Some(mat) = materials.get_mut(mat_handle) {
            if player_logic.invincibility_timer > 0.0 {
                let blink = (time.elapsed_secs() * 20.0).sin() > 0.0;
                mat.base_color = if blink {
                    Color::srgba(1.0, 1.0, 1.0, 0.2)
                } else {
                    Color::srgb(0.0, 1.0, 0.5)
                };
            } else {
                mat.base_color = Color::srgb(0.0, 1.0, 0.5);
            }
        }
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
    mut q_player: Query<(&GlobalTransform, &mut PlayerBall)>,
    q_machines: Query<(Entity, &GlobalTransform), With<AlienMachine>>,
    q_factories: Query<(Entity, &GlobalTransform), With<AlienFactory>>,
    mut score_msg: MessageWriter<ScoreMessage>,
) {
    let Ok((player_gtrans, mut player)) = q_player.single_mut() else { return; };
    let player_pos = player_gtrans.translation();
    let player_radius = settings.player_radius;

    for (entity, machine_gtrans) in q_machines.iter() {
        if player_pos.distance(machine_gtrans.translation()) < settings.player_radius + 3.0 {
            if dash_state.is_active {
                commands.entity(entity).despawn_children();
                commands.entity(entity).despawn();
                score_msg.write(ScoreMessage(300));
            } else if !settings.god_mode && player.invincibility_timer <= 0.0 {
                player.hp = (player.hp - 25.0).max(0.0);
                player.invincibility_timer = 5.0; 
                commands.entity(entity).despawn_children();
                commands.entity(entity).despawn();
            }
        }
    }

    if dash_state.is_active {
        for (entity, factory_gtrans) in q_factories.iter() {
            if player_pos.distance(factory_gtrans.translation()) < player_radius + 6.0 {
                commands.entity(entity).despawn_children();
                commands.entity(entity).despawn();
                score_msg.write(ScoreMessage(500));
            }
        }
    }
}

fn player_health_sync_system(
    mut settings: ResMut<PlanetSettings>,
    q_player: Query<&PlayerBall>,
) {
    let Ok(player) = q_player.single() else { return; };
    
    let target_radius = (player.hp / 100.0) * settings.max_hp_radius;
    settings.player_radius = target_radius.max(2.0);
}

fn spawn_health_bar(mut commands: Commands) {
    commands.spawn((
        SessionUi,
        Node {
            position_type: PositionType::Absolute,
            display: Display::Flex,
            flex_direction: FlexDirection::Column, 
            align_items: AlignItems::Center,
            left: Val::Percent(50.0),
            bottom: Val::VMin(18.0),
            margin: UiRect::left(Val::VMin(-15.0)),
            ..default()
        },
        ZIndex(100),
    ))
    .with_children(|parent| {
        parent.spawn((
            Text::new("HEALTH"),
            TextFont { font_size: 14.0, ..default() },
            TextColor(Color::WHITE),
            Node { margin: UiRect::bottom(Val::Px(4.0)), ..default() },
        ));

        parent.spawn((
            Node {
                width: Val::VMin(30.0),
                height: Val::VMin(2.5),
                display: Display::Flex,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.8)),
            BorderRadius::all(Val::Px(4.0)),
        ))
        .with_children(|bar| {
            bar.spawn((
                HealthBarFill,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.0, 1.0, 0.0)),
                BorderRadius::all(Val::Px(4.0)),
            ));

            bar.spawn((
                HealthText,
                Text::new("100 / 100"),
                TextFont { font_size: 14.0, ..default() },
                TextColor(Color::WHITE),
                ZIndex(1),
            ));
        });
    });
}

fn update_health_bar_system(
    q_player: Query<&PlayerBall, Changed<PlayerBall>>,
    mut q_fill: Query<(&mut Node, &mut BackgroundColor), With<HealthBarFill>>,
    mut q_text: Query<&mut Text, With<HealthText>>,
) {
    let Ok(player) = q_player.single() else { return; };
    
    if let Ok((mut node, mut color)) = q_fill.single_mut() {
        node.width = Val::Percent(player.hp);
        let hp_ratio = player.hp / 100.0;
        color.0 = Color::srgba(1.0 - hp_ratio, hp_ratio, 0.0, 1.0);
    }

    if let Ok(mut text) = q_text.single_mut() {
        text.0 = format!("{:.0} / 100", player.hp);
    }
}

fn player_invincibility_system(
    time: Res<Time>,
    mut q_player: Query<&mut PlayerBall>,
) {
    let Ok(mut player) = q_player.single_mut() else { return; };
    if player.invincibility_timer > 0.0 {
        player.invincibility_timer = (player.invincibility_timer - time.delta_secs()).max(0.0);
    }
}

fn orb_spawning_system(
    mut commands: Commands,
    settings: Res<PlanetSettings>,
    mut meshes: ResMut<Assets<Mesh>>, 
    mut materials: ResMut<Assets<StandardMaterial>>,
    q_planet: Query<(&PlanetData, &Mesh3d), With<Planet>>,
    q_orbs: Query<Entity, With<EnergyOrb>>,
    mut local_assets: Local<Option<(Handle<Mesh>, Handle<StandardMaterial>)>>,
) {
    if q_orbs.iter().count() >= settings.max_orbs { return; }
    
    let mut rng = rand::rng();
    if rng.random::<f32>() > settings.orb_spawn_chance { return; }

    if local_assets.is_none() {
        let m = meshes.add(Sphere::new(2.0).mesh().ico(4).unwrap());
        let mat = materials.add(StandardMaterial {
            base_color: Color::srgba(0.0, 5.0, 1.0, 1.0),
            emissive: LinearRgba::GREEN * 10.0,
            ..default()
        });
        *local_assets = Some((m, mat));
    }
    let (orb_mesh, orb_mat) = local_assets.as_ref().unwrap().clone();

    let Ok((planet_data, mesh_handle)) = q_planet.single() else { return; };
    let Some(mesh) = meshes.get(mesh_handle) else { return; };

    let healthy_indices: Vec<usize> = planet_data.vertex_states.iter()
        .enumerate()
        .filter(|(_, state)| **state == TileState::Healthy)
        .map(|(idx, _)| idx)
        .collect();

    if healthy_indices.is_empty() { return; }

    let random_idx = healthy_indices[rng.random_range(0..healthy_indices.len())];

    if let Some(bevy::mesh::VertexAttributeValues::Float32x3(v_pos)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        let normal = Vec3::from(v_pos[random_idx]).normalize();
        
        let spawn_pos = normal * (settings.radius + 5.0);

        commands.spawn((
            EnergyOrb,
            Mesh3d(orb_mesh),
            MeshMaterial3d(orb_mat),
            Transform::from_translation(spawn_pos),
        ));
    }
}

fn orb_collection_system(
    mut commands: Commands,
    settings: Res<PlanetSettings>,
    mut q_player: Query<(&GlobalTransform, &mut PlayerBall)>,
    q_orbs: Query<(Entity, &GlobalTransform), With<EnergyOrb>>,
) {
    let Ok((player_gtrans, mut player)) = q_player.single_mut() else { return; };
    let player_pos = player_gtrans.translation();

    for (orb_entity, orb_gtrans) in q_orbs.iter() {
        let orb_pos = orb_gtrans.translation();
        
        if player_pos.distance(orb_pos) < settings.player_radius + 3.0 {
            player.hp = (player.hp + settings.orb_hp_gain).min(100.0);
            
            commands.entity(orb_entity).despawn_children();
            commands.entity(orb_entity).despawn();
        }
    }
}

fn orb_animation_system(
    time: Res<Time>,
    mut q_orbs: Query<&mut Transform, With<EnergyOrb>>,
) {
    let t = time.elapsed_secs();
    for mut transform in q_orbs.iter_mut() {
        transform.rotate_y(0.05);
        let offset = (t * 2.0).sin() * 0.5;
        let normal = transform.translation.normalize();
        transform.translation += normal * offset * 0.1;
    }
}

fn death_system(
    mut next_state: ResMut<NextState<GameState>>,
    q_player: Query<&PlayerBall>,
) {
    let Ok(player) = q_player.single() else { return; };

    if player.hp <= 0.0 {
        next_state.set(GameState::GameOver);
    }
}

fn setup_death_menu(mut commands: Commands, score: Res<Score>, time: Res<SessionTime>) {
    commands.spawn((
        DeathMenuRoot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
        ZIndex(200),
    ))
    .with_children(|parent| {
        parent.spawn((
            Text::new("GAME OVER!"),
            TextFont { font_size: 80.0, ..default() },
            TextColor(Color::srgb(1.0, 0.2, 0.2)),
            Node { margin: UiRect::bottom(Val::Px(20.0)), ..default() },
        ));

        parent.spawn((
            Text::new(format!("FINAL SCORE: {}", score.current)),
            TextFont { font_size: 32.0, ..default() },
            TextColor(Color::WHITE),
            Node { margin: UiRect::bottom(Val::Px(40.0)), ..default() },
        ));

        parent.spawn((
            Text::new(format!("SURVIVED FOR: {}", time.format())),
            TextFont { font_size: 28.0, ..default() },
            TextColor(Color::WHITE),
            Node { margin: UiRect::bottom(Val::Px(40.0)), ..default() },
        ));

        parent.spawn((
            RestartButton,
            Interaction::default(),
            Node {
                width: Val::Px(200.0),
                height: Val::Px(50.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                margin: UiRect::bottom(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
            BorderRadius::all(Val::Px(10.0)),
        )).with_children(|btn| {
            btn.spawn((Text::new("RESTART"), TextFont { font_size: 20.0, ..default() }, TextColor(Color::WHITE)));
        });

        parent.spawn((
            ExitButton,
            Interaction::default(),
            Node {
                width: Val::Px(200.0),
                height: Val::Px(50.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
            BorderRadius::all(Val::Px(10.0)),
        )).with_children(|btn| {
            btn.spawn((Text::new("EXIT"), TextFont { font_size: 20.0, ..default() }, TextColor(Color::WHITE)));
        });
    });
}

fn death_menu_interaction_system(
    mut next_state: ResMut<NextState<GameState>>,
    q_restart: Query<&Interaction, (Changed<Interaction>, With<RestartButton>)>,
    q_exit: Query<&Interaction, (Changed<Interaction>, With<ExitButton>)>,
    mut app_exit_events: MessageWriter<AppExit>,
) {
    if let Ok(Interaction::Pressed) = q_restart.single() {
        next_state.set(GameState::Resetting);
    }

    if let Ok(Interaction::Pressed) = q_exit.single() {
        app_exit_events.write(AppExit::Success);
    }
}

fn cleanup_death_menu(mut commands: Commands, q_root: Query<Entity, With<DeathMenuRoot>>) {
    if let Ok(entity) = q_root.single() {
        commands.entity(entity).despawn_children();
        commands.entity(entity).despawn();
    }
}

fn world_reset_system(
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    q_cleanup: Query<Entity, Or<(With<PlayerBall>, With<AlienFactory>, With<AlienMachine>, With<EnergyOrb>)>>,
    mut q_planet: Query<(&mut PlanetData, &Mesh3d), With<Planet>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut dash_state: ResMut<DashState>,
    mut score: ResMut<Score>,
    mut time: ResMut<SessionTime>,
) {
    for entity in q_cleanup.iter() {
        commands.entity(entity).despawn_children();
        commands.entity(entity).despawn();
    }

    if let Ok((mut planet_data, mesh_handle)) = q_planet.single_mut() {
        planet_data.vertex_states.fill(TileState::Wasteland);
        if let Some(mesh) = meshes.get_mut(mesh_handle) {
            if let Some(bevy::mesh::VertexAttributeValues::Float32x4(v_col)) = mesh.attribute_mut(Mesh::ATTRIBUTE_COLOR) {
                v_col.fill([0.15, 0.15, 0.15, 1.0]);
            }
        }
    }

    *dash_state = DashState::default();
    score.current = 0;
    time.elapsed = 0.0;

    next_state.set(GameState::Playing);
}

fn score_event_handler(
    mut messages: MessageReader<ScoreMessage>,
    mut score: ResMut<Score>,
) {
    for msg in messages.read() {
        score.current += msg.0;
    }
}

fn spawn_score_hud(mut commands: Commands) {
    commands.spawn((
        ScoreHud, 
        SessionUi,
        Node {
            position_type: PositionType::Absolute,
            top: Val::VMin(2.0),
            width: Val::Percent(100.0),
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            column_gap: Val::VMin(10.0),
            ..default()
        },
        ZIndex(100),
    ))
    .with_children(|parent| {
        parent.spawn((
            ScoreHudText,
            Text::new("SCORE: 0"),
            TextFont { font_size: 24.0, ..default() },
            TextColor(Color::WHITE),
        ));

        parent.spawn((
            TimeHudText,
            Text::new("TIME: 00:00"),
            TextFont { font_size: 24.0, ..default() },
            TextColor(Color::srgb(0.8, 0.8, 1.0)),
        ));
    });
}

fn update_score_hud_system(
    score: Res<Score>,
    mut q_text: Query<&mut Text, With<ScoreHudText>>,
) {
    if score.is_changed() {
        if let Ok(mut text) = q_text.single_mut() {
            text.0 = format!("SCORE: {}", score.current);
        }
    }
}

fn cleanup_game_ui(
    mut commands: Commands,
    q_ui: Query<Entity, Or<(With<VjoyBase>, With<DashButton>, With<HealthBarFill>, With<ScoreHud>, With<DeathMenuRoot>, With<SessionUi>)>>,
) {
    for entity in q_ui.iter() {
        if let Ok(mut entity_cmds) = commands.get_entity(entity) {
            entity_cmds.despawn();
        }
    }
}

fn track_session_time_system(
    time: Res<Time>,
    mut session_time: ResMut<SessionTime>,
) {
    session_time.elapsed += time.delta_secs();
}

fn update_time_hud_system(
    session_time: Res<SessionTime>,
    mut q_text: Query<&mut Text, With<TimeHudText>>,
) {
    if let Ok(mut text) = q_text.single_mut() {
        text.0 = format!("TIME: {}", session_time.format());
    }
}

fn setup_main_menu(mut commands: Commands, profile: Res<PlayerProfile>) {
    commands.spawn((
        MainMenuRoot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.9).into()),
        ZIndex(200),
    ))
    .with_children(|parent| {
        parent.spawn((
            Text::new("GREEN SPHERE"),
            TextFont { font_size: 80.0, ..default() },
            TextColor(Color::srgb(0.0, 1.0, 0.5)),
            Node { margin: UiRect::bottom(Val::VMin(5.0)), ..default() },
        ));

        parent.spawn((
            Node {
                width: Val::Px(300.0),
                height: Val::Px(40.0),
                align_items: AlignItems::Center,
                padding: UiRect::left(Val::Px(10.0)),
                margin: UiRect::bottom(Val::VMin(3.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.1, 0.1, 0.1).into()),
            BorderRadius::all(Val::Px(5.0)),
        )).with_children(|p| {
            p.spawn((
                Text::new(format!("USER: {}", profile.username)),
                TextFont { font_size: 18.0, ..default() },
                TextColor(Color::srgb(0.6, 0.6, 0.6)),
            ));
        });

        parent.spawn((
            StartButton,
            Interaction::default(),
            Node {
                width: Val::Px(250.0),
                height: Val::Px(60.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                margin: UiRect::bottom(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.2, 0.2, 0.2).into()),
            BorderRadius::all(Val::Px(10.0)),
        )).with_children(|btn| {
            btn.spawn((Text::new("START"), TextFont { font_size: 25.0, ..default() }, TextColor(Color::WHITE)));
        });

        parent.spawn((
            ExitButton,
            Interaction::default(),
            Node {
                width: Val::Px(250.0),
                height: Val::Px(60.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.2, 0.1, 0.1).into()),
            BorderRadius::all(Val::Px(10.0)),
        )).with_children(|btn| {
            btn.spawn((Text::new("EXIT"), TextFont { font_size: 25.0, ..default() }, TextColor(Color::WHITE)));
        });
    });
}

fn main_menu_system(
    mut next_state: ResMut<NextState<GameState>>,
    q_start: Query<&Interaction, (Changed<Interaction>, With<StartButton>)>,
    q_exit: Query<&Interaction, (Changed<Interaction>, With<ExitButton>)>,
    mut exit_events: MessageWriter<AppExit>,
) {
    if let Ok(Interaction::Pressed) = q_start.single() {
        next_state.set(GameState::Playing);
    }
    if let Ok(Interaction::Pressed) = q_exit.single() {
        exit_events.write(AppExit::Success);
    }
}

fn cleanup_main_menu(mut commands: Commands, q: Query<Entity, With<MainMenuRoot>>) {
    if let Ok(e) = q.single() {
        commands.entity(e).despawn();
    }
}