//! defines structures to be sent between client and server

use crate::{
    boat::Boat,
    primitives::{CustomTransform, Radian, Speed},
};
use bevy::{ecs::entity::MapEntities, prelude::*};
use lightyear::{
    input::{self, native::plugin::InputPlugin},
    prelude::{input::native::ActionState, *},
};
use serde::{Deserialize, Serialize};
use crate::primitives::{OutOfBound, Position};
use crate::world::WorldSize;

pub struct SendToClient;
pub struct SendToServer;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, Reflect, PartialEq)]
pub struct Input {
    pub reversed: bool,
    pub rotate: Radian,
    pub speed: Speed
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, Reflect, PartialEq)]
pub enum ClientInput {
    Exists(Input),
    #[default]
    None
}

impl ClientInput {
    pub fn unwrap(&self) -> Option<Input> {
        match self {
            Self::Exists(input) => Some(*input),
            Self::None => None
        }
    }
}

/// ship's head's radian with positive x-axis
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, Reflect, PartialEq)]
pub struct Rotate(pub Option<Radian>);
/// speed is negative on reverse
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, Reflect, PartialEq)]
pub struct Move(pub Option<Speed>);
/// indicates whether ship is reversed.
/// 
/// used to communicate between rotate input buffering and moving input buffering
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Deref, DerefMut, Component
)]
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

impl Ease for CustomTransform {
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
        FunctionCurve::new(Interval::UNIT, move |t| {
            Self {
                position: Position(Vec2::lerp(start.position.0, end.position.0, t)),
                rotation: Radian(f32::lerp(start.rotation.0, end.rotation.0, t)),
                speed: end.speed
            }
        })
    }
}

pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        // replication
        app.register_component::<WorldSize>().add_delta_compression::<u32>();
        app.register_component::<Boat>();
        app.register_component::<CustomTransform>().add_prediction()
            .add_linear_interpolation();
        app.register_component::<OutOfBound>().add_prediction();

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
        app.register_component::<ActionState<Rotate>>();
        app.register_component::<ActionState<Move>>();
    }
}
