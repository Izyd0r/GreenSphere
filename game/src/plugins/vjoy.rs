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
    vjoy_output::VjoyOutput
};

/// Main entry point for the Virtual Joystick functionality.
/// Call `.add_plugins(vjoy::plugin)` in your App setup.
pub(crate) fn plugin(app: &mut App) {
    app
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
fn spawn_joystick(mut commands: Commands) {
    let base_size = Val::VMin(30.0); 
    let knob_size = Val::VMin(10.0);

    commands.spawn((
        VjoyBase { radius: 0.0 },
        Interaction::default(), 
        Node {
            width: base_size,
            height: base_size,
            max_width: Val::Px(250.0),
            max_height: Val::Px(250.0),
            min_width: Val::Px(120.0),
            min_height: Val::Px(120.0),
            position_type: PositionType::Absolute,
            left: Val::VMin(20.0),
            bottom: Val::VMin(15.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.3)),
        BorderRadius::all(Val::Percent(50.0)),
        ZIndex(100),
    ))
    .insert(RelativeCursorPosition::default())
    .with_children(|parent| {
        parent.spawn((
            VjoyKnob,
            Node {
                width: knob_size,
                height: knob_size,
                max_width: Val::Px(80.0),
                max_height: Val::Px(80.0),
                min_width: Val::Px(40.0),
                min_height: Val::Px(40.0),
                position_type: PositionType::Relative, 
                ..default()
            },
            BackgroundColor(Color::WHITE),
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
                let sensitivity = 2.0;
                let joystick_vec = Vec2::new(pos.x * sensitivity, -pos.y * sensitivity);
                vjoy_output.dir = joystick_vec.clamp(Vec2::splat(-1.0), Vec2::splat(1.0));
                if vjoy_output.dir.length() < 0.05 { vjoy_output.dir = Vec2::ZERO; }
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
    q_base: Query<&ComputedNode, (With<VjoyBase>, Without<VjoyKnob>)>,
    mut q_knob: Query<&mut Node, With<VjoyKnob>>,
    mut q_base_col: Query<&mut BackgroundColor, (With<VjoyBase>, Without<VjoyKnob>)>,
    mut q_knob_col: Query<&mut BackgroundColor, (With<VjoyKnob>, Without<VjoyBase>)>,
) {
    let Ok(base_node) = q_base.single() else { return; };
    let Ok(mut knob_node) = q_knob.single_mut() else { return; };
    
    let radius = base_node.size().x / 2.0;
    
    knob_node.left = Val::Px(vjoy_output.dir.x * radius);
    knob_node.top = Val::Px(-vjoy_output.dir.y * radius);

    let alpha = if active_touch.id.is_some() { 0.7 } else { 0.2 };
    if let Ok(mut c) = q_base_col.single_mut() { c.0.set_alpha(alpha); }
    if let Ok(mut c) = q_knob_col.single_mut() { c.0.set_alpha(alpha); }
}