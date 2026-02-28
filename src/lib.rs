mod primitives;
mod ship;
mod util;

use bevy::camera_controller::pan_camera::PanCameraPlugin;
use bevy::prelude::*;

use crate::ship::{move_camera, resize_rigs, resize_ship, startup, update_ship, update_transform, validate_rigs};

mod constants {

    pub const DEFAULT_MAX_TURN_DEG: f32 = 0.5;
    pub const DEFAULT_MAX_ZOOM: f32 = 2.0;
    /// # Warning
    /// Code will break silently if we use something else
    pub const DEFAULT_SPRITE_SHRINK: f32 = 0.3;

    pub const YASEN_MAX_SPEED: f32 = 1.5;  // using HashMap?
    pub const YASEN_BACK_SPEED: f32 = 0.9;
    pub const YASEN_RAW_SIZE: f32 = 1024.0;
}

pub struct ShipPlugin;

impl Plugin for ShipPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PanCameraPlugin)
            .insert_resource(ClearColor(Color::linear_rgb(0.0, 0.1, 0.6)))
            .add_systems(Startup, startup)
            .add_systems(Update, (update_ship, update_transform).chain())
            .add_systems(Update, move_camera)
            .add_systems(Update, (resize_rigs, resize_ship, validate_rigs));
    }
}
