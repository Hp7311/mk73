use bevy::prelude::*;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

// TODO feature client/server
mod boat;
pub mod collision;
mod movement;
pub mod primitives;
pub mod protocol;
pub mod util;
mod weapon;
pub mod world;

pub use movement::MovementPlugin;
pub use weapon::Weapon;
pub use boat::Boat;
pub use boat::SubKind;

pub const SERVER_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), SERVER_PORT);
pub const LOCAL_SERVER_ADDR: SocketAddr =
    SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), SERVER_PORT);
pub const CLIENT_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), CLIENT_PORT);
pub const PROTOCOL_ID: u64 = 0;

// --- Z-ordering constants
use crate::primitives::ZIndex;
pub const OCEAN_SURFACE: ZIndex = ZIndex(0.0);
pub const OCEAN_FLOOR: ZIndex = ZIndex(-0.4);
pub const CIRCLE_HUD: ZIndex = ZIndex(30.0);

const SERVER_PORT: u16 = 8000;
const CLIENT_PORT: u16 = 8001;

/// unified `custom_size` on the Sprite of an oil rig. may be upgrades
pub const OILRIG_SPRITE_SIZE: Vec2 = Vec2::splat(1024.0 * 0.3);

const DEFAULT_MAX_TURN_DEG: crate::primitives::Radian = crate::primitives::Radian::from_deg(0.5);

/// # Warning
/// Code will break silently if we use something else
const DEFAULT_SPRITE_SHRINK: f32 = 0.3;

#[derive(Component)]
pub struct MainCamera;
