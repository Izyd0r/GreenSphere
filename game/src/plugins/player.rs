use bevy::prelude::*;

use crate::prelude::player_settings::*;
use crate::prelude::vjoy_output::*;
use crate::prelude::planet_settings::*;
use crate::prelude::planet::*;
use crate::prelude::player_ball::*;
use crate::prelude::planet_pivot::*;
use crate::prelude::camera::*;
use crate::prelude::machine::*;
use crate::prelude::factory::*;
use crate::prelude::dash_settings::*;
use crate::prelude::dash_state::*;
use crate::prelude::ui::*;
use crate::prelude::score::*;

use crate::prelude::*;

pub(crate) fn plugin(app: &mut App) {
    app
        .init_resource::<PlayerSettings>()
        .register_type::<PlayerSettings>()    
        .add_systems(Update, (
            (planetary_control_system, sync_visuals).chain(),
            (enemy_collision_system),
            (player_health_sync_system, update_health_bar_system, death_system).chain(),
            (player_invincibility_system),
        ).run_if(in_state(GameState::Playing)).run_if(any_with_component::<PlayerBall>));
}

fn planetary_control_system(
    joy: Res<VjoyOutput>,
    time: Res<Time>,
    settings: Res<PlayerSettings>,
    dash_settings: Res<DashSettings>,
    planet_settings: Res<PlanetSettings>,
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
        ball.current_velocity *= planet_settings.friction.powf(dt * 60.0);
        
        if ball.current_velocity.length() > effective_max_speed {
            let target_vel = ball.current_velocity.normalize() * effective_max_speed;
            ball.current_velocity = ball.current_velocity.lerp(target_vel, 0.1);
        }
    }

    let current_speed = ball.current_velocity.length();

    if current_speed > 0.1 {
        let rotation_x = Quat::from_rotation_x(ball.current_velocity.z * dt / planet_settings.radius);
        let rotation_z = Quat::from_rotation_z(-ball.current_velocity.x * dt / planet_settings.radius);
        pivot_trans.rotation = pivot_trans.rotation * rotation_x * rotation_z;

        let roll_speed = current_speed * dt / settings.player_radius;
        let roll_axis_raw = Vec3::new(ball.current_velocity.z, 0.0, -ball.current_velocity.x);
        if let Ok(axis) = Dir3::new(roll_axis_raw.normalize_or_zero()) {
            ball_trans.rotate_axis(axis, roll_speed);
        }
    }
}

fn sync_visuals(
    player_settings: Res<PlayerSettings>,
    planet_settings: Res<PlanetSettings>,
    time: Res<Time>,
    mut q_planet: Query<&mut Transform, (With<Planet>, Without<PlayerBall>, Without<BirdEyeCamera>)>,
    mut q_ball: Query<(&PlayerBall, &mut Transform, &MeshMaterial3d<StandardMaterial>), (Without<Planet>, Without<BirdEyeCamera>)>,
    mut q_cam: Query<&mut Transform, (With<BirdEyeCamera>, Without<Planet>, Without<PlayerBall>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if let Ok(mut t) = q_planet.single_mut() { 
        t.scale = Vec3::splat(planet_settings.radius); 
    }

    if let Ok((player_logic, mut t, mat_handle)) = q_ball.single_mut() { 
        t.scale = Vec3::splat(player_settings.player_radius); 
        t.translation.y = planet_settings.radius + player_settings.player_radius;

        if let Some(mat) = materials.get_mut(mat_handle) {
            if player_logic.invincibility_timer > 0.0 {
                let blink = (time.elapsed_secs() * 30.0).sin() > 0.0;
                if blink {
                    mat.base_color = Color::srgba(2.0, 2.0, 2.0, 0.4); 
                } else {
                    mat.base_color = Color::WHITE;
                }
            } else {
                mat.base_color = Color::WHITE;
            }
        }
    }

    if let Ok(mut t) = q_cam.single_mut() {
        let dynamic_zoom = player_settings.camera_height + (player_settings.player_radius * 5.0);
        t.translation.y = planet_settings.radius + dynamic_zoom;
        let target = Vec3::new(0.0, planet_settings.radius, 0.0);
        t.look_at(target, -Vec3::Z);
    }
}

fn enemy_collision_system(
    mut commands: Commands,
    dash_state: Res<DashState>,
    player_settings: Res<PlayerSettings>,
    mut q_player: Query<(&GlobalTransform, &mut PlayerBall)>,
    q_machines: Query<(Entity, &GlobalTransform), With<AlienMachine>>,
    q_factories: Query<(Entity, &GlobalTransform), With<AlienFactory>>,
    mut score_msg: MessageWriter<ScoreMessage>,
) {
    let Ok((player_gtrans, mut player)) = q_player.single_mut() else { return; };
    let player_pos = player_gtrans.translation();
    let player_radius = player_settings.player_radius;

    for (entity, machine_gtrans) in q_machines.iter() {
        if player_pos.distance(machine_gtrans.translation()) < player_settings.player_radius + 3.0 {
            if dash_state.is_active {
                commands.entity(entity).despawn_children();
                commands.entity(entity).despawn();
                score_msg.write(ScoreMessage(300));
            } else if !player_settings.god_mode && player.invincibility_timer <= 0.0 {
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
    mut player_settings: ResMut<PlayerSettings>,
    q_player: Query<&PlayerBall>,
) {
    let Ok(player) = q_player.single() else { return; };
    
    let target_radius = (player.hp / 100.0) * player_settings.max_hp_radius;
    player_settings.player_radius = target_radius.max(2.0);
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
    for mut player in q_player.iter_mut() {
        if player.invincibility_timer > 0.0 {
            player.invincibility_timer -= time.delta_secs();
            if player.invincibility_timer < 0.0 {
                player.invincibility_timer = 0.0;
            }
        }
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