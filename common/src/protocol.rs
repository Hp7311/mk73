//! defines structures to be sent between client and server

use crate::{boat::Boat, primitives::{CustomTransform, Radian, Speed}};
use bevy::{ecs::entity::MapEntities, prelude::*};
use lightyear::{input::{self, native::plugin::InputPlugin}, prelude::{input::native::ActionState, *}};
use serde::{Deserialize, Serialize};

pub struct SendToClient;
pub struct SendToServer;

/// currently, client sending [`PlayerAction`] to server and server compares the data with existing [`MinimalBoat`], if accepted, server updates
/// the [`MinimalBoat`] in server world and is replicated back to client where an observer catches it
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, Reflect, PartialEq)]
pub struct Rotate(pub Option<Radian>);
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, Reflect, PartialEq)]
pub struct Move(pub Option<Speed>);
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, Reflect, PartialEq, Deref, DerefMut)]
pub struct Reversed(pub bool);
impl Reversed {
    pub fn to_bool(&self) -> bool {
        self.0
    }
}

impl MapEntities for Rotate {
    fn map_entities<E: EntityMapper>(&mut self, _entity_mapper: &mut E) {}
}
impl MapEntities for Move {
    fn map_entities<E: EntityMapper>(&mut self, _entity_mapper: &mut E) {}
}
impl MapEntities for Reversed {
    fn map_entities<E: EntityMapper>(&mut self, _entity_mapper: &mut E) {}
}


// NOTE message rx/sx are automatically spawned on specified direction

/// demo
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Component, Deref, DerefMut)]
struct PlayerPos(pub Vec2);

/// interpolation
impl Ease for PlayerPos {
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
        FunctionCurve::new(Interval::UNIT, move |t| {
            PlayerPos(Vec2::lerp(start.0, end.0, t))
        })
    }
}
pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        // replication
        // app.register_component::<PlayerPos>()
        //     .add_prediction()
        //     .add_linear_interpolation();
        app.register_component::<Boat>();
        app.register_component::<CustomTransform>()
            .add_prediction();

        app.add_channel::<SendToClient>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
            ..default()
        })
        .add_direction(NetworkDirection::ServerToClient);

        app.add_channel::<SendToServer>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
            ..default()
        })
        .add_direction(NetworkDirection::ClientToServer);

        // MUST register these two for every input
        app.add_plugins(InputPlugin::<Rotate>::default());
        app.add_plugins(InputPlugin::<Move>::default());
        app.add_plugins(InputPlugin::<Reversed>::default());
        app.register_component::<ActionState<Rotate>>();
        app.register_component::<ActionState<Move>>();
        app.register_component::<ActionState<Reversed>>();
    }
}
