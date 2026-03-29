//! defines structures to be sent between client and server

use bevy::{prelude::*};
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};
use crate::boat::Boat;

/// server asks client to spawn a boat at a specific location (only one "owned" per client)
/// 
/// sprite name accessible through `boat.data.file_name`
#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub struct SpawnShip {
    pub position: Vec2,
    pub boat: Boat
}

/// sent server -> client
#[derive(Debug, Component, Clone, Default)]
pub struct MultipleSpawnShip(pub Vec<SpawnShip>);

pub struct SendToClient;
pub struct SendToServer;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum PlayerAction {
    /// attempt to move ship to specified position + rotation
    Move {
        position: Vec2,
        rotation: f32  // Z-axis radian
    },
    /// fire a weapon with rotation f32 (radian)
    Fire(f32)
}

/// server's response to a player action,
/// client updates its own stats stored for no serde performance
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum ServerResponse {
    Accept,
    Reject
}

// TODO broadcast changes above to all clients

pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        // server -> client
        app
            .register_message::<SpawnShip>()
            .add_direction(NetworkDirection::ServerToClient);

        app
            .register_message::<ServerResponse>()
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
