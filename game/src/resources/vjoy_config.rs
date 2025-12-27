use bevy::prelude::*;

/// Configuration resource for the Virtual Joystick.
/// 
/// By updating this resource (via code or the `bevy_inspector_egui`), the joystick 
/// will react instantly to changes in size, sensitivity, or appearance.
#[derive(Resource, Reflect, Debug)]
#[reflect(Resource)]
pub struct VjoyConfig {
    /// Multiplier applied to the raw input. 
    /// - `1.0`: Linear mapping (thumb must touch the visual edge for max speed).
    /// - `2.0+`: Easier to pull (reaches max speed much faster).
    pub sensitivity: f32,

    /// Magnitude threshold (0.0 to 1.0) below which input is ignored.
    /// Prevents "drifting" or jitter when a finger is resting near the center.
    pub deadzone: f32,

    /// Transparency of the joystick when it is NOT being touched (0.0 to 1.0).
    pub alpha_idle: f32,

    /// Transparency of the joystick while actively being dragged (0.0 to 1.0).
    pub alpha_active: f32,

    /// Target width/height of the base circle relative to the smaller screen dimension.
    /// 30.0 means 30% of the screen's `VMin`.
    pub base_size_vmin: f32,

    /// Distance from the left edge of the screen in responsive `VMin` units.
    pub pos_left_vmin: f32,

    /// Distance from the bottom edge of the screen in responsive `VMin` units.
    pub pos_bottom_vmin: f32,

    /// Hard ceiling for the base size in physical pixels. 
    /// Prevents the joystick from becoming massive on large 4K monitors.
    pub base_max_px: f32,

    /// Hard floor for the base size in physical pixels.
    /// Ensures the joystick is always big enough for a human thumb on small phones.
    pub base_min_px: f32,

    /// Target size of the moving knob relative to the smaller screen dimension.
    pub knob_size_vmin: f32,

    /// Hard ceiling for the knob size in physical pixels.
    pub knob_max_px: f32,

    /// Hard floor for the knob size in physical pixels.
    pub knob_min_px: f32,

    /// The base color (tint) of the joystick background circle.
    pub base_color: Color,

    /// The color of the inner moving knob.
    pub knob_color: Color,
}


impl Default for VjoyConfig {
    fn default() -> Self {
        Self {
            sensitivity: 2.0,
            deadzone: 0.05,
            alpha_idle: 0.3,
            alpha_active: 0.8,
            base_size_vmin: 30.0,
            pos_left_vmin: 20.0,
            pos_bottom_vmin: 15.0,
            base_max_px: 250.0,
            base_min_px: 120.0,
            knob_size_vmin: 10.0,
            knob_max_px: 80.0,
            knob_min_px: 40.0,
            base_color: Color::srgba(1.0, 1.0, 1.0, 1.0),
            knob_color: Color::WHITE,
        }
    }
}