mod util;
mod ship;
use bevy::prelude::*;

use crate::ship::{startup, update_transform, update_ship};

mod constants {
    pub const YASEN_MAX_TURN_DEGREE: f32 = 2.0;
    pub const YASEN_MAX_SPEED: f32 = 1.0;
    pub const YASEN_RAW_SIZE: f32 = 1024.0;
}

pub struct ShipPlugin;

impl Plugin for ShipPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::linear_rgb(0.0, 0.4, 0.8)))
            .add_systems(Startup, startup)
            .add_systems(Update, (update_ship, update_transform).chain());
    }
}
