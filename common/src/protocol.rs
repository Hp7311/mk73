//! defines structures to be sent between client and server

use crate::{boat::Boat, primitives::Radian};
use bevy::{ecs::entity::MapEntities, prelude::*};
use lightyear::{input, prelude::{input::native::ActionState, *}};
use serde::{Deserialize, Serialize};

pub struct SendToClient;
pub struct SendToServer;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct PlayerAction {
    pub action: ActionType,
    pub client: u64
}

/// currently, client sending [`PlayerAction`] to server and server compares the data with existing [`MinimalBoat`], if accepted, server updates
/// the [`MinimalBoat`] in server world and is replicated back to client where an observer catches it
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum ActionType {
    /// attempt to move ship to specified position
    Move(Vec2),
    /// rotates to rotation f32 (radian)
    Rotate(Radian),
    /// fire a weapon with rotation f32 (radian)
    Fire(Radian)  // TODO weapon counter etc
}

/// server replicates this to client
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Component)]
pub struct MinimalBoat {
    pub position: Vec2,
    pub boat: Boat,
    /// radians along the Z-axis
    pub rotation: Radian,
}

// NOTE message rx/sx are automatically spawned on specified direction


#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Component, Deref, DerefMut)]
pub struct PlayerPos(pub Vec2);
pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        // replication
        // app.register_component::<MinimalBoat>();
        app.register_component::<PlayerPos>()
            .add_prediction();

        // server -> client
        app.add_channel::<SendToClient>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
            ..default()
        })
        .add_direction(NetworkDirection::ServerToClient);

        // client -> server
        app.register_message::<PlayerAction>()
            .add_direction(NetworkDirection::ClientToServer);

        app.add_channel::<SendToServer>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
            ..default()
        })
        .add_direction(NetworkDirection::ClientToServer);

        app.add_plugins(input::native::plugin::InputPlugin::<DbgClientInput>::default());
        app.register_component::<ActionState<DbgClientInput>>();
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Default, Reflect)]
pub enum DbgClientInput {
    /// relative to client pos
    Move(Vec2),
    #[default]
    None
}

impl MapEntities for DbgClientInput {
    fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {}
}