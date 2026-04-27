//! defines structures to be sent between client and server

use std::rc::Rc;
use std::sync::Arc;
use crate::{
    boat::Boat,
    primitives::{CustomTransform, Radian, Speed},
};
use bevy::{ecs::entity::MapEntities, prelude::*};
use lightyear::{
    input::{native::plugin::InputPlugin},
    prelude::{input::native::ActionState, *},
};
use serde::{Deserialize, Serialize};
use crate::primitives::{OutOfBound, Position};
use crate::world::WorldSize;

pub struct SendToClient;
struct SendToServer;

/// ship's head's radian with positive x-axis
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, Reflect, PartialEq)]
pub struct Rotate(pub Option<Radian>);
/// speed is negative on reverse
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, Reflect, PartialEq)]
pub struct Move(pub Option<Speed>);

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

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Component)] // component to store it in server
pub struct OilRigInfo {
    pub position: Vec2,
    pub rotation: Radian,
    pub custom_size: Vec2
}

impl OilRigInfo {
    pub fn file_name(&self) -> &'static str {
        "oil_platform.png"
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Component)]
pub struct PointInfo {
    pub position: Vec2,
    /// currently not doing prediction etc
    pub file_name: Arc<str>
}

impl PointInfo {
    pub fn custom_size() -> Vec2 {
        vec2(5.0, 5.0)
    }
}
#[derive(Debug, Clone, Copy, /*Resource, */ Default, Component, Deserialize, Serialize, PartialEq)]
pub struct PlayerScore(u32);

impl PlayerScore {
    pub fn new(score: u32) -> Self {
        Self(score)
    }
    pub fn add_to_score(&mut self, points: u32) {
        self.0 += points;
    }
    pub fn get_score(&self) -> u32 {
        self.0
    }
}

/// message sender and manager are inserted on every [`ClientOf`] entity on server
pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        // replication
        app.register_component::<WorldSize>().add_delta_compression::<u32>();
        app.register_component::<Boat>();
        app.register_component::<CustomTransform>().add_prediction()
            .add_linear_interpolation();
        app.register_component::<OutOfBound>().add_prediction();

        app.register_component::<OilRigInfo>();
        app.register_component::<PointInfo>();

        app.register_component::<PlayerScore>();

        // MUST register these two for every input
        app.add_plugins(InputPlugin::<Rotate>::default());
        app.add_plugins(InputPlugin::<Move>::default());
        app.register_component::<ActionState<Rotate>>();
        app.register_component::<ActionState<Move>>();

        app.add_channel::<SendToClient>(ChannelSettings {
            mode: ChannelMode::UnorderedReliable(ReliableSettings::default()),
            ..default()
        })
            .add_direction(NetworkDirection::ServerToClient);
    }
}
