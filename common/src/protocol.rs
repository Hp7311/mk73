//! defines structures to be sent between client and server

use bevy::{prelude::*};
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};
use crate::boat::Boat;

/// server asks client to spawn a boat at a specific location (only once per client)
/// 
/// sprite name accessible through `boat.data.file_name`
#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub struct SpawnShip {
    pub position: Vec2,
    pub boat: Boat
}

pub struct SendToClient;
pub struct SendToServer;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum PlayerAction {
    /// move client-owned boat to Vec2
    Move(Vec2),
    /// fire a weapon with rotation f32 (radian)
    Fire(f32)
}

pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        // server -> client
        app
            .register_message::<SpawnShip>()
            .add_direction(NetworkDirection::ServerToClient);

        app
            .add_channel::<SendToClient>(ChannelSettings {
                mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
                ..default()
            })
            .add_direction(NetworkDirection::ServerToClient);

        // client -> server
        app
            .register_message::<PlayerAction>()
            .add_direction(NetworkDirection::ClientToServer);
        
        app
            .add_channel::<SendToServer>(ChannelSettings {
                mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
                ..default()
            })
            .add_direction(NetworkDirection::ClientToServer);
    }
}
