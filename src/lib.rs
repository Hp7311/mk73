mod primitives;
mod ship;
mod util;
mod collision;
mod world;
mod oil_rig;

use bevy::prelude::*;
use bevy::camera_controller::pan_camera::PanCamera;

/// # Warning
/// Code will break silently if we use something else
const DEFAULT_SPRITE_SHRINK: f32 = 0.3;

const DEFAULT_MAX_TURN_DEG: f32 = 0.5;
const DEFAULT_MAX_ZOOM: f32 = 2.0;

pub use ship::ShipPlugin;
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