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
    vjoy_config::VjoyConfig
};

/// Main entry point for the Virtual Joystick functionality.
/// Call `.add_plugins(vjoy::plugin)` in your App setup.
pub(crate) fn plugin(app: &mut App) {
    app
        .init_resource::<VjoyConfig>()
        .init_resource::<VjoyOutput>()
        .init_resource::<ActiveTouch>()
        .add_systems(Startup, spawn_joystick)
        .add_systems(Update, (
            joystick_input_system, 
            joystick_render_system 
        ).chain());
}

/// Spawns the visual hierarchy of the joystick.
/// The Base is positioned using `VMin` to stay responsive across screen sizes.
fn spawn_joystick(mut commands: Commands, config: Res<VjoyConfig>) {
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
        // Use the color from config
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
        base_node.left = Val::VMin(config.pos_left_vmin);
        base_node.bottom = Val::VMin(config.pos_bottom_vmin);
    }
    knob_node.width = Val::VMin(config.knob_size_vmin);
    knob_node.height = Val::VMin(config.knob_size_vmin);

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