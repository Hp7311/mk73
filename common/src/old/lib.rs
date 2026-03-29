//! nukes :D
mod boat;
// mod collision;
// mod oil_rig;
// mod primitives;
pub mod protocol;
// mod shaders;
// mod util;
// mod weapons;
// mod world;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use bevy::prelude::*;

const SERVER_PORT: u16 = 8000;
const CLIENT_PORT: u16 = 8001;

pub const SERVER_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), SERVER_PORT);
pub const LOCAL_SERVER_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), SERVER_PORT);
pub const CLIENT_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), CLIENT_PORT);
pub const PROTOCOL_ID: u64 = 0;

/// # Warning
/// Code will break silently if we use something else
const DEFAULT_SPRITE_SHRINK: f32 = 0.3;

const DEFAULT_MAX_TURN_DEG: f32 = 0.5;


// pub use boat::BoatPlugin;
// pub use oil_rig::OilRigPlugin;
// pub use shaders::ShadersPlugin;
// pub use weapons::WeaponPlugin;
// pub use world::WorldPlugin;
