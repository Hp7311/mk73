//! nukes :D
mod boat;
mod collision;
mod oil_rig;
mod primitives;
mod util;
mod world;

use bevy::camera_controller::pan_camera::PanCamera;
use bevy::prelude::*;

/// # Warning
/// Code will break silently if we use something else
const DEFAULT_SPRITE_SHRINK: f32 = 0.3;

const DEFAULT_MAX_TURN_DEG: f32 = 0.5;
const DEFAULT_MAX_ZOOM: f32 = 2.0;

// --- Z-ordering constants
const WATER_SURFACE: f32 = 0.0;
const CIRCLE_HUD: f32 = 30.0;

pub use boat::BoatPlugin;
pub use oil_rig::OilRigPlugin;
pub use world::WorldPlugin;

#[derive(Component)]
struct MainCamera;

pub fn setup(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        PanCamera {
            min_zoom: 1.0,
            max_zoom: DEFAULT_MAX_ZOOM,
            key_down: None,
            key_left: None,
            key_right: None,
            key_up: None,
            ..default()
        },
        MainCamera,
    ));
}
