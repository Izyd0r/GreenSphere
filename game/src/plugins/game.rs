use bevy::prelude::*;

use rand::Rng;

use crate::prelude::planet_settings::*;
use crate::prelude::planet::*;
use crate::prelude::player_ball::*;
use crate::prelude::planet_pivot::*;
use crate::prelude::camera::*;
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
        .init_state::<GameState>()
        .init_resource::<PlanetSettings>()
        .init_resource::<Score>()
        .init_resource::<SessionTime>()
        .init_resource::<PlayerProfile>()
        .init_resource::<ResetTarget>()
        .add_message::<ScoreMessage>()
        .register_type::<Score>()
        .register_type::<PlanetSettings>()
        .add_systems(Startup, (setup_planet, build_adjacency).chain())
        .add_systems(OnEnter(GameState::Playing), (
            spawn_session_objects, 
        ).chain())        
        .add_systems(Update, (
            (tile_restoration_system, ),
            (orb_spawning_system, orb_collection_system, orb_animation_system),
        ).run_if(in_state(GameState::Playing)).run_if(any_with_component::<PlayerBall>))
        .add_systems(OnEnter(GameState::Resetting), world_reset_system);
}

fn setup_planet(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    planet_settings: Res<PlanetSettings>,
    player_settings: Res<PlayerSettings>,
) {
    commands.spawn((
        DirectionalLight { illuminance: 7000.0, shadows_enabled: true, ..default() },
        Transform::from_xyz(100.0, 100.0, 100.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    let mut mesh = Sphere::new(1.0).mesh().ico(planet_settings.subdivisions as u32).unwrap();
    mesh.duplicate_vertices();
    mesh.compute_flat_normals();
    let vertex_count = mesh.count_vertices();

    let mut uvs = vec![[0.0, 0.0]; vertex_count];
    for i in (0..vertex_count).step_by(3) {
        uvs[i]     = [0.0, 0.0];
        uvs[i + 1] = [0.5, 0.0];
        uvs[i + 2] = [0.25, 0.5];
    }
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    
    mesh.asset_usage = bevy::asset::RenderAssetUsages::default();

    let atlas_handle = asset_server.load("textures/tiles.png");

    commands.spawn((
        Planet,
        PlanetData { 
            vertex_states: vec![TileState::Wasteland; vertex_count],
            adjacency: Vec::new(), 
        },
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(materials.add(StandardMaterial { 
            base_color_texture: Some(atlas_handle),
            perceptual_roughness: 0.9,
            ..default() 
        })),
        Transform::from_scale(Vec3::splat(planet_settings.radius)),
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
            Transform::from_xyz(0.0, planet_settings.radius + player_settings.camera_height, 0.0)
                .looking_at(Vec3::new(0.0, planet_settings.radius, 0.0), -Vec3::Z),
        ));
    });
}

fn spawn_session_objects(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    planet_settings: Res<PlanetSettings>,
    player_settings: Res<PlayerSettings>,
    q_pivot: Query<Entity, With<PlanetPivot>>,
) {
    let Ok(pivot_entity) = q_pivot.single() else { return; };

    let ball_material = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load("textures/moss_ball.png")),
        base_color: Color::WHITE,
        alpha_mode: AlphaMode::Blend, 
        perceptual_roughness: 0.8,
        ..default()
    });

    let ball_entity = commands.spawn((
        PlayerBall { current_velocity: Vec3::ZERO, hp: 100.0, invincibility_timer: 0.0 }, 
        Mesh3d(meshes.add(Sphere::new(1.0).mesh().ico(5).unwrap())),
        MeshMaterial3d(ball_material),
        Transform::from_xyz(0.0, planet_settings.radius + player_settings.player_radius, 0.0)
            .with_scale(Vec3::splat(player_settings.player_radius)),
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

fn tile_restoration_system(
    planet_settings: Res<PlanetSettings>,
    player_settings: Res<PlayerSettings>,
    q_player: Query<&GlobalTransform, With<PlayerBall>>,
    mut q_planet: Query<(&Transform, &Mesh3d, &mut PlanetData), With<Planet>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut score_msg: MessageWriter<ScoreMessage>,
) {
    let Ok(player_gtrans) = q_player.single() else { return; };
    let Ok((planet_trans, mesh_handle, mut planet_data)) = q_planet.single_mut() else { return; };
    let Some(mesh) = meshes.get_mut(mesh_handle) else { return; };

    let player_rel_pos = (player_gtrans.translation() - planet_trans.translation) / planet_settings.radius;
    let brush_sq = ((player_settings.player_radius * 1.2) / planet_settings.radius).powi(2);

    let Some(bevy::mesh::VertexAttributeValues::Float32x3(v_pos)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else { return; };
    let v_pos_local = v_pos.clone(); 

    if let Some(bevy::mesh::VertexAttributeValues::Float32x2(v_uv)) = mesh.attribute_mut(Mesh::ATTRIBUTE_UV_0) {
        for i in 0..v_pos_local.len() {
            if planet_data.vertex_states[i] == TileState::Healthy { continue; }
            let v = Vec3::from(v_pos_local[i]);
            if (v.x - player_rel_pos.x).abs() > 0.1 { continue; }
            
            if v.distance_squared(player_rel_pos) < brush_sq {
                let points = if planet_data.vertex_states[i] == TileState::Polluted { 200 } else { 100 };
                planet_data.vertex_states[i] = TileState::Healthy;
                score_msg.write(ScoreMessage(points));

                let tri_start = (i / 3) * 3;
                v_uv[tri_start]     = [0.5, 0.0];
                v_uv[tri_start + 1] = [1.0, 0.0];
                v_uv[tri_start + 2] = [0.75, 0.5];
            }
        }
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
    planet_settings: Res<PlanetSettings>,
    player_settings: Res<PlayerSettings>,
    mut q_player: Query<(&GlobalTransform, &mut PlayerBall)>,
    q_orbs: Query<(Entity, &GlobalTransform), With<EnergyOrb>>,
) {
    let Ok((player_gtrans, mut player)) = q_player.single_mut() else { return; };
    let player_pos = player_gtrans.translation();

    for (orb_entity, orb_gtrans) in q_orbs.iter() {
        let orb_pos = orb_gtrans.translation();
        
        if player_pos.distance(orb_pos) < player_settings.player_radius + 3.0 {
            player.hp = (player.hp + planet_settings.orb_hp_gain).min(100.0);
            
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

fn world_reset_system(
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    reset_target: Res<ResetTarget>,
    q_cleanup: Query<Entity, Or<(With<PlayerBall>, With<AlienFactory>, With<AlienMachine>, With<EnergyOrb>, With<SessionUi>)>>,
    mut q_planet: Query<(&mut PlanetData, &Mesh3d), With<Planet>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut dash_state: ResMut<DashState>,
    mut score: ResMut<Score>,
    mut time: ResMut<SessionTime>,
) {
    for entity in q_cleanup.iter() {
        if let Ok(mut entity_cmds) = commands.get_entity(entity) {
            entity_cmds.despawn_children();
            entity_cmds.despawn();
        }
    }

    if let Ok((mut planet_data, mesh_handle)) = q_planet.single_mut() {
        planet_data.vertex_states.fill(TileState::Wasteland);

        if let Some(mesh) = meshes.get_mut(mesh_handle) {
            if let Some(bevy::mesh::VertexAttributeValues::Float32x2(v_uv)) = mesh.attribute_mut(Mesh::ATTRIBUTE_UV_0) {
                for i in (0..v_uv.len()).step_by(3) {
                    v_uv[i]     = [0.0, 0.0];
                    v_uv[i + 1] = [0.5, 0.0];
                    v_uv[i + 2] = [0.25, 0.5];
                }
            }
        }
    }

    *dash_state = DashState::default();
    dash_state.current_energy = 100.0;
    score.current = 0;
    time.elapsed = 0.0;

    next_state.set(reset_target.0.clone());
}