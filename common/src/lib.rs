use bevy::prelude::*;
use std::net::{Ipv4Addr, SocketAddr};
use crate::primitives::ZIndex;
use crate::util::ip_addr;

mod boat;
mod movement;
mod weapon;
mod world;
mod upgrade;
#[cfg(feature = "client")]
mod shaders;

// TODO smaller raw size to knots, smaller turning speed etc
// TODO test these

pub mod collision;
pub mod primitives;
pub mod protocol;
pub mod util;
pub use movement::MovementPlugin;
#[cfg(feature = "server")]
pub use upgrade::UpgradeSet;
pub use upgrade::UpgradePlugin;
pub use weapon::{Weapon, WeaponType};
pub use boat::{Boat, SubKind, CircleHud, BoatClientId, BoatReverseNegative, BoatReversePositive};
#[cfg(feature = "client")]
pub use boat::BoatType;
pub use world::{WorldPlugin, WorldSize};

pub use macros::BoatImpl;

pub const SERVER_ADDR: SocketAddr = ip_addr(Ipv4Addr::LOCALHOST, SERVER_PORT);
#[cfg(feature = "client")]
pub const CLIENT_ADDR: SocketAddr = ip_addr(Ipv4Addr::LOCALHOST, CLIENT_PORT);
pub const PROTOCOL_ID: u64 = 0;

pub fn circle_hud_mesh(radius: f32) -> Ring<Circle> {
    Circle::new(radius).to_ring(3.0)
}

// --- Z-ordering constants
/// primarily for the main [`Boat`] on the surface
pub const OCEAN_SURFACE: ZIndex = ZIndex(0.0);
pub const OCEAN_FLOOR: ZIndex = ZIndex(-0.4);
/// position.z += self when passing to Transform
pub const POINTS_Z: f32 = -0.1;
/// position.z += self when passing to Transform
pub const OIL_RIG_Z: f32 = 0.1;
/// circle-hud + weapon marker
pub const CIRCLE_HUD: ZIndex = ZIndex(30.0);

#[derive(Component)]
pub struct MainCamera;

const SERVER_PORT: u16 = 8000;
#[cfg(feature = "client")]
const CLIENT_PORT: u16 = 8001;

pub const TCP_ADDR: SocketAddr = ip_addr(Ipv4Addr::LOCALHOST, 9000);
pub const TCP_WS_ADDR: &str = "ws://127.0.0.1:9000";

const DEFAULT_MAX_TURN_DEG: crate::primitives::Radian = crate::primitives::Radian::from_deg(0.5);

#[cfg(all(not(debug_assertions), feature = "client", feature = "server"))]
// not erroring in debug to look good to rust-analyzer
compile_error!("Client and Server features mutually exclusive");

// TODO wrapping of the World like the Earth, suggested by OceanForceYT