//! defines structures to be sent between client and server

use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

/// server asks client to spawn a sprite at a specific location
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct SpawnSprite {
    pub position: Vec2,
    pub sprite_name: String
}

pub struct SendToClient;

pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        app
            .register_message::<SpawnSprite>()
            .add_direction(NetworkDirection::ServerToClient);

        app
            .add_channel::<SendToClient>(ChannelSettings {
                mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
                ..default()
            })
            .add_direction(NetworkDirection::ServerToClient);
    }
}
