//! nukes :D
mod boat;
mod collision;
mod oil_rig;
mod primitives;
pub mod protocol;
mod shaders;
mod util;
mod weapons;
mod world;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use bevy::camera_controller::pan_camera::PanCamera;
use bevy::prelude::*;

pub const SERVER_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8000);
/// in server
// pub const LOCAL_SERVER_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8000);
pub const LOCAL_SERVER_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8000);
pub const CLIENT_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8001);
pub const PROTOCOL_ID: u64 = 0;

/// # Warning
/// Code will break silently if we use something else
const DEFAULT_SPRITE_SHRINK: f32 = 0.3;

const DEFAULT_MAX_TURN_DEG: f32 = 0.5;
const DEFAULT_MAX_ZOOM: f32 = 2.0;

// --- Z-ordering constants
const WATER_SURFACE: f32 = 0.0;
const OCEAN_FLOOR: f32 = -0.4;
const CIRCLE_HUD: f32 = 30.0;
const DIVING_OVERLAY: f32 = 35.0;

pub use boat::BoatPlugin;
pub use oil_rig::OilRigPlugin;
pub use shaders::ShadersPlugin;
pub use weapons::WeaponPlugin;
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
            key_rotate_ccw: None,
            key_rotate_cw: None,
            ..default()
        },
        MainCamera,
    ));
}
