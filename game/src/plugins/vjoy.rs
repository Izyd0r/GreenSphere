//! # Virtual Joystick Plugin
//! 
//! Provides a responsive 2D virtual joystick for cross-platform mouse and touch input.
//! 
//! This plugin manages:
//! 1. Spawning the UI elements.
//! 2. Capturing input and calculating a normalized `Vec2`.
//! 3. Rendering the visual "Knob" movement.
//!
//! ## Requirements
//! - Requires a `Camera2d` or `Camera3d` to be present in the world for UI rendering.

use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;

use crate::prelude::{
    active_touch::ActiveTouch, 
    vjoy_base::VjoyBase, 
    vjoy_knob::VjoyKnob, 
    vjoy_output::VjoyOutput,
    vjoy_config::VjoyConfig,
    dash::DashButton,
    dash::DashMeter,
    dash_settings::DashSettings,
    dash_state::DashState
};

use crate::components::ui::*;

/// Main entry point for the Virtual Joystick functionality.
/// Call `.add_plugins(vjoy::plugin)` in your App setup.
pub(crate) fn plugin(app: &mut App) {
    app
        .init_resource::<VjoyConfig>()
        .init_resource::<VjoyOutput>()
        .init_resource::<ActiveTouch>()
        .init_resource::<DashState>()
        .init_resource::<DashSettings>()
        .register_type::<VjoyConfig>()
        .register_type::<VjoyOutput>()
        .register_type::<DashState>()
        .register_type::<DashSettings>() 
        .add_systems(Update, (
            joystick_input_system.run_if(any_with_component::<VjoyBase>), 
            joystick_render_system.run_if(any_with_component::<VjoyBase>),
            dash_input_system.run_if(any_with_component::<DashButton>),
        ).chain());
}

/// Spawns the visual hierarchy of the joystick.
/// The Base is positioned using `VMin` to stay responsive across screen sizes.
pub fn spawn_joystick(mut commands: Commands, config: Res<VjoyConfig>) {
    commands.spawn((
        VjoyBase::default(),
        Interaction::default(),
        Node {
            width: Val::VMin(config.base_size_vmin),
            height: Val::VMin(config.base_size_vmin),
            max_width: Val::Px(config.base_max_px),
            max_height: Val::Px(config.base_max_px),
            min_width: Val::Px(config.base_min_px),
            min_height: Val::Px(config.base_min_px),
            position_type: PositionType::Absolute,
            left: Val::VMin(config.pos_left_vmin),
            bottom: Val::VMin(config.pos_bottom_vmin),
            display: Display::Flex,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(config.base_color.with_alpha(config.alpha_idle)),
        BorderRadius::all(Val::Percent(50.0)),
        ZIndex(100),
    ))
    .insert(RelativeCursorPosition::default())
    .with_children(|parent| {
        parent.spawn((
            VjoyKnob,
            Node {
                width: Val::VMin(config.knob_size_vmin),
                height: Val::VMin(config.knob_size_vmin),
                max_width: Val::Px(config.knob_max_px),
                max_height: Val::Px(config.knob_max_px),
                min_width: Val::Px(config.knob_min_px),
                min_height: Val::Px(config.knob_min_px),
                position_type: PositionType::Relative, 
                ..default()
            },
            BackgroundColor(config.knob_color.with_alpha(config.alpha_idle)),
            BorderRadius::all(Val::Percent(50.0)),
        ));
    });
}

/// Reads mouse and touch input to update the [VjoyOutput] resource.
/// 
/// ### Constraints:
/// - **Deadzone:** Input smaller than 5% is ignored (returns `Vec2::ZERO`).
/// - **Clamping:** The output vector length is capped at `1.0`.
/// - **Sensitivity:** A multiplier of `2.0` is applied to make pulling the knob easier.
fn joystick_input_system(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    config: Res<VjoyConfig>,
    mut q_base: Query<(&Interaction, &RelativeCursorPosition, &mut VjoyBase, &ComputedNode)>,
    mut vjoy_output: ResMut<VjoyOutput>,
    mut active_touch: ResMut<ActiveTouch>,
) {
    let Ok((interaction, relative_cursor, mut base_info, computed_node)) = q_base.single_mut() else { return; };

    let actual_radius = computed_node.size().x / 2.0;
    base_info.radius = actual_radius;

    if active_touch.id.is_none() {
        if *interaction == Interaction::Pressed {
            active_touch.id = Some(u64::MAX);
        }
        for touch in touches.iter_just_pressed() {
            if relative_cursor.cursor_over() {
                active_touch.id = Some(touch.id());
            }
        }
    }

    let mut input_released = false;
    if let Some(id) = active_touch.id {
        if id == u64::MAX {
            if !mouse_buttons.pressed(MouseButton::Left) { input_released = true; }
        } else if touches.get_released(id).is_some() {
            input_released = true;
        }

        if !input_released {
            if let Some(pos) = relative_cursor.normalized {
                let joystick_vec = Vec2::new(pos.x * config.sensitivity, -pos.y * config.sensitivity);
                vjoy_output.dir = joystick_vec.clamp(Vec2::splat(-1.0), Vec2::splat(1.0));
                if vjoy_output.dir.length() < config.deadzone { vjoy_output.dir = Vec2::ZERO; }
            }
        }
    }

    if input_released {
        active_touch.id = None;
        vjoy_output.dir = Vec2::ZERO;
    }
}

/// Updates the visual position of the knob and the opacity of the joystick.
fn joystick_render_system(
    vjoy_output: Res<VjoyOutput>,
    active_touch: Res<ActiveTouch>,
    config: Res<VjoyConfig>,
    q_base: Query<(&ComputedNode, Entity), (With<VjoyBase>, Without<VjoyKnob>)>,
    mut q_base_node: Query<&mut Node, (With<VjoyBase>, Without<VjoyKnob>)>,
    mut q_knob: Query<(&mut Node, &ComputedNode), With<VjoyKnob>>,
    mut q_base_col: Query<&mut BackgroundColor, (With<VjoyBase>, Without<VjoyKnob>)>,
    mut q_knob_col: Query<&mut BackgroundColor, (With<VjoyKnob>, Without<VjoyBase>)>,
) {
    let Ok((base_computed, _)) = q_base.single() else { return; };
    let Ok((mut knob_node, knob_computed)) = q_knob.single_mut() else { return; };
    
    if let Ok(mut base_node) = q_base_node.single_mut() {
        base_node.width = Val::VMin(config.base_size_vmin);
        base_node.height = Val::VMin(config.base_size_vmin);
        base_node.max_width = Val::Px(config.base_max_px);
        base_node.max_height = Val::Px(config.base_max_px);
        base_node.min_width = Val::Px(config.base_min_px);
        base_node.min_height = Val::Px(config.base_min_px);
        base_node.left = Val::VMin(config.pos_left_vmin);
        base_node.bottom = Val::VMin(config.pos_bottom_vmin);
    }
    knob_node.width = Val::VMin(config.knob_size_vmin);
    knob_node.height = Val::VMin(config.knob_size_vmin);
    knob_node.max_width = Val::Px(config.knob_max_px);
    knob_node.max_height = Val::Px(config.knob_max_px);
    knob_node.min_width = Val::Px(config.knob_min_px);
    knob_node.min_height = Val::Px(config.knob_min_px);

    let base_radius = base_computed.size().x / 2.0;
    let knob_radius = knob_computed.size().x / 2.0;
    let max_move: f32 = (base_radius - knob_radius).max(0.0);
    
    knob_node.left = Val::Px(vjoy_output.dir.x * max_move);
    knob_node.top = Val::Px(-vjoy_output.dir.y * max_move);

    let target_alpha = if active_touch.id.is_some() { 
        config.alpha_active 
    } else { 
        config.alpha_idle 
    };

    if let Ok(mut c) = q_base_col.single_mut() { c.0.set_alpha(target_alpha); }
    if let Ok(mut c) = q_knob_col.single_mut() { c.0.set_alpha(target_alpha); }
}

pub fn spawn_dash_button(mut commands: Commands) {
    let vertical_level = Val::VMin(15.0); 
    let horizontal_offset = Val::VMin(25.0); 

    commands.spawn((
        DashButton,
        SessionUi,
        Interaction::default(),
        Node {
            width: Val::VMin(15.0),
            height: Val::VMin(15.0),
            position_type: PositionType::Absolute,
            right: horizontal_offset, 
            bottom: vertical_level,
            display: Display::Flex,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.2)),
        BorderRadius::all(Val::Percent(50.0)),
        ZIndex(100),
    ))
    .with_children(|parent| {
        parent.spawn((
            DashMeter,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 1.0, 1.0, 0.3)),
            BorderRadius::all(Val::Percent(50.0)),
        ));

        parent.spawn((
            DashButtonText,
            Text::new("DASH"),
            TextFont { 
                font_size: 20.0,
                ..default() 
            },
            TextColor(Color::WHITE),
            ZIndex(1),
        ));
    });
}

fn dash_input_system(
    time: Res<Time>,
    settings: Res<DashSettings>,
    mut state: ResMut<DashState>,
    mut q_button: Query<(&Interaction, &mut BackgroundColor), (With<DashButton>, Without<DashMeter>)>,
    mut q_meter: Query<(&mut Node, &mut BackgroundColor), (With<DashMeter>, Without<DashButton>)>,
) {
    let dt = time.delta_secs();
    
    let Ok((interaction, mut btn_bg)) = q_button.single_mut() else { return; };
    let Ok((mut meter_node, mut meter_bg)) = q_meter.single_mut() else { return; };

    state.current_energy = (state.current_energy + settings.regen_rate * dt).min(settings.max_energy);
    state.cooldown_timer = (state.cooldown_timer - dt).max(0.0);

    if state.duration_timer > 0.0 {
        state.duration_timer -= dt;
        if state.duration_timer <= 0.0 {
            state.is_active = false; 
        }
    }

    let is_ready = state.current_energy >= settings.max_energy && state.cooldown_timer <= 0.0;

    if state.is_active {
        btn_bg.0 = Color::srgba(0.0, 1.0, 1.0, 1.0); // Solid Cyan
    } else if is_ready {
        // Ready to dash
        btn_bg.0 = Color::srgba(1.0, 1.0, 1.0, 0.9);
        meter_bg.0 = Color::srgba(0.0, 1.0, 1.0, 0.8);
    } else {
        // Recharging
        btn_bg.0 = Color::srgba(0.2, 0.2, 0.2, 0.2);
        meter_bg.0 = Color::srgba(0.0, 0.5, 0.5, 0.3);
    }

    if *interaction == Interaction::Pressed && is_ready {
        state.is_active = true;
        state.duration_timer = settings.dash_duration;
        state.current_energy = 0.0;
        state.cooldown_timer = settings.cooldown_secs;
    }

    let energy_pct = state.current_energy / settings.max_energy;
    meter_node.width = Val::Percent(energy_pct * 100.0);
    meter_node.height = Val::Percent(energy_pct * 100.0);
}

pub fn sync_dash_text_size(
    q_window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut q_text: Query<&mut TextFont, With<DashButtonText>>,
) {
    let Ok(window) = q_window.single() else { return; };
    let Ok(mut font) = q_text.single_mut() else { return; };

    let vmin = window.width().min(window.height());

    font.font_size = vmin * 0.035; 
}