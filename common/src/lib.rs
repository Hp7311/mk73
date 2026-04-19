use bevy::prelude::*;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

pub mod boat;
pub mod cert;
pub mod collision;
mod movement;
pub mod primitives;
pub mod protocol;
pub mod util;
pub mod weapon;
pub mod world;

use crate::primitives::Radian;
pub use movement::MovementPlugin;

pub const SERVER_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), SERVER_PORT);
pub const LOCAL_SERVER_ADDR: SocketAddr =
    SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), SERVER_PORT);
pub const CLIENT_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), CLIENT_PORT);
pub const PROTOCOL_ID: u64 = 0;

// --- Z-ordering constants
pub const WATER_SURFACE: f32 = 0.0;
pub const OCEAN_FLOOR: f32 = -0.4;
pub const CIRCLE_HUD: f32 = 30.0;
pub const DIVING_OVERLAY: f32 = 35.0;

const SERVER_PORT: u16 = 8000;
const CLIENT_PORT: u16 = 8001;

const DEFAULT_MAX_TURN_DEG: Radian = Radian::from_deg(0.5);

/// # Warning
/// Code will break silently if we use something else
const DEFAULT_SPRITE_SHRINK: f32 = 0.3;

#[derive(Component)]
pub struct MainCamera;
