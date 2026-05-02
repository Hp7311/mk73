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
use crate::primitives::{OutOfBound, Position, WeaponCounter, ZIndex};
use crate::weapon::Weapon;
use crate::world::WorldSize;

pub struct SendToClient;
pub struct SendToServer;

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
    pub rotation: Radian
}

impl OilRigInfo {
    pub fn file_name(&self) -> &'static str {
        "oil_platform.png"
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Component)]
pub struct PointInfo {
    /// may be point depth fluctuations
    pub position: Vec3,
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

/// currently implemented as a message, to-server
#[derive(Debug, Deserialize, Serialize)]
pub struct SpawnWeapon {
    pub weapon: Weapon,
    pub position: Vec3,
    // currently no turrets etc
    pub starting_rotation: Radian,
    pub end_rotation: Radian,
    /// to identify the weapon on client-side if server doesn't approve
    pub entity_on_client: EntityOnClient,
    /// identifing the Boat
    ///
    /// replicated by server on spawning the main boat entity
    pub entity_on_server: EntityOnServer,
    /// required to replicate weapon to other clients
    pub client_id: PeerId
}

/// replicated by server on spawning the main boat entity
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Component, Copy, Reflect)]
pub struct EntityOnServer(pub u64);
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Component, Copy, Reflect)]
pub struct EntityOnClient(pub u64);

/// if server doesn't approve
///
/// some thoughts:
///
#[derive(Debug, Deserialize, Serialize)]
pub enum WeaponRollBack {
    Transform {
        position: Vec2,
        rotation: Radian,
        entity: EntityOnClient
    },
    Despawn {
        entity: EntityOnClient
    }
}

/// client sends updates to controlling boat's Z-index to the server as a Message
///
/// requires [`EntityOnServer`] to be given and correctly represent the boat's entity on the server world
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct NewZIndex {
    pub new_index: ZIndex,
    pub entity_on_server: EntityOnServer
}

/// not controlled
///
/// we update this when Transform of a weapon in server is updated, then replicated into other client's worlds
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Component, Copy)]
pub struct WeaponCustomTransform {
    pub position: Vec3,
    pub rotation: Radian
}

/// message sender and manager are inserted on every [`ClientOf`] entity on server
pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        // replication
        app.register_component::<WorldSize>().add_delta_compression::<u32>();
        app.register_component::<Boat>();
        app.register_component::<WeaponCounter>();
        app.register_component::<CustomTransform>().add_prediction()
            .add_linear_interpolation();
        app.register_component::<ZIndex>();
        app.register_component::<OutOfBound>();

        app.register_component::<EntityOnServer>();
        app.register_message::<NewZIndex>().add_direction(NetworkDirection::ClientToServer);

        app.register_component::<OilRigInfo>();
        app.register_component::<PointInfo>();

        app.register_component::<PlayerScore>();

        // MUST register these two for every input
        app.add_plugins(InputPlugin::<Rotate>::default());
        app.add_plugins(InputPlugin::<Move>::default());
        app.register_component::<ActionState<Rotate>>();
        app.register_component::<ActionState<Move>>();

        app.register_message::<SpawnWeapon>().add_direction(NetworkDirection::ClientToServer);
        app.register_message::<WeaponRollBack>().add_direction(NetworkDirection::ServerToClient);

        app.register_component::<WeaponCustomTransform>();
        app.register_component::<Weapon>();

        app.add_channel::<SendToClient>(ChannelSettings {
            mode: ChannelMode::UnorderedReliable(ReliableSettings::default()),
            ..default()
        })
            .add_direction(NetworkDirection::ServerToClient);
        app.add_channel::<SendToServer>(ChannelSettings {
            mode: ChannelMode::UnorderedReliable(ReliableSettings::default()),
            ..default()
        })
            .add_direction(NetworkDirection::ClientToServer);
    }
}
