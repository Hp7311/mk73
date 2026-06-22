//! defines structures to be sent between client and server

use std::f32::consts::{FRAC_PI_2, PI};
use crate::{
    OCEAN_SURFACE, OIL_RIG_Z, POINTS_Z, boat::Boat, primitives::{CustomTransform, DisplayScore, LastSpeed, PlayerStats, Point, Radian, Size, Speed, TargetRotation}
};
use bevy::{ecs::entity::MapEntities, prelude::*};
use lightyear::{
    input::{native::plugin::InputPlugin},
    prelude::{input::native::ActionState, *},
};
use macros::{FetchSprite};
use serde::{Deserialize, Serialize};
use crate::primitives::{Position, ZIndex};
use crate::weapon::Weapon;
use crate::world::WorldSize;

/// unordered reliable
pub struct SendToClient;
pub struct SendToServer;
// ordered reliable
pub struct SendToClientOrdered;
pub struct SendToServerOrdered;

/// ship's head's radian with positive x-axis
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, Reflect, PartialEq)]
pub struct Rotate(pub Option<Radian>);
/// speed is negative on reverse
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, Reflect, PartialEq)]
pub struct Move(pub Option<Speed>);
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, Reflect, PartialEq)]
pub struct ZIndexUpdate(pub Option<ZIndex>);

impl MapEntities for Rotate {
    fn map_entities<E: EntityMapper>(&mut self, _entity_mapper: &mut E) {}
}
impl MapEntities for Move {
    fn map_entities<E: EntityMapper>(&mut self, _entity_mapper: &mut E) {}
}
impl MapEntities for ZIndexUpdate {
    fn map_entities<E: EntityMapper>(&mut self, _entity_mapper: &mut E) {}
}

impl Ease for CustomTransform {
    /// when lerping over the negative X-axis in rotation, it will "snap" the boat by interpolating in the opposite direction if goes by default
    /// 
    /// assumes CustomTransform's rotation normalized
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
        FunctionCurve::new(Interval::UNIT, move |t| {
            let rotation = 
            // if turning over the axis, adjust the starting rotation, using 90 degrees for clarity
            if start.rotation.0 < -FRAC_PI_2 && end.rotation.0 > FRAC_PI_2{
                f32::lerp(start.rotation.0 + 2.0 * PI, end.rotation.0, t)
            } else if start.rotation.0 > FRAC_PI_2 && end.rotation.0 < -FRAC_PI_2 {
                f32::lerp(start.rotation.0 - 2.0 * PI, end.rotation.0, t)
            } else {
                f32::lerp(start.rotation.0, end.rotation.0, t)
            };

            Self {
                position: Position(Vec2::lerp(start.position.0, end.position.0, t)),
                rotation: Radian(rotation),
                speed: end.speed
            }
        })
    }
}

#[derive(FetchSprite, Serialize, Deserialize, Debug, PartialEq, Clone, Component)] // component to store it in server
#[json = "Hq"]  // or OilPlatform
pub struct OilRigTransform {
    pub position: Vec2,
    pub rotation: Radian
}

impl Size for OilRigTransform {
    fn size(&self) -> Vec2 {
        vec2(100.0, 100.0)  // inferred from being approx half the size of 055
    }
}
impl OilRigTransform {
    pub const SPRITE_SIZE: f32 = 1024.0 * 0.3;
    /// ### For [`Transform`] only
    pub fn z_index_transform() -> f32 {
        *OCEAN_SURFACE + OIL_RIG_Z
    }
    pub fn custom_size() -> Vec2 {
        Vec2::splat(Self::SPRITE_SIZE)
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Component)]
pub struct PointTransform {
    /// Vec3.z is the theoretical, NOT necessarily rendering
    pub position: Vec2,
    pub depth: ZIndex,
    /// currently not doing prediction etc
    pub point: Point
}

impl PointTransform {
    pub const PRECISION_TO_BOAT_Z: f32 = 0.05;
    pub fn new(position: Vec2, depth: ZIndex, point: Point) -> Self {
        Self {
            position,
            depth,
            point
        }
    }
    pub fn custom_size() -> Vec2 {  // Size?
        vec2(5.0, 5.0)
    }
    /// ## use for [`Transform`]
    /// don't use for physics
    pub fn to_translation(&self) -> Vec3 {
        self.position.extend(*self.depth + POINTS_Z)
    }
    /// ## DO NOT use this for [`Transform`],  
    /// only for physics
    pub fn to_actual_translation(&self) -> Vec3 {
        self.position.extend(*self.depth)
    }
}

impl Ease for PointTransform {
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
        FunctionCurve::new(Interval::UNIT, move |t| {
            Self {
                position: Vec2::lerp(start.position, end.position, t),
                depth: ZIndex(f32::lerp(start.depth.0, end.depth.0, t)),
                point: end.point
            }
        })
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
    /// required to replicate weapon to other clients and identifying the [`Boat`] entity on the server
    pub client_id: PeerId
}

/// replicated by server on spawning the main boat entity
/// 
/// mainly used by client to specify the [`Boat`] entity targeting on server in a message
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Component, Copy, Reflect)]
pub struct EntityOnServer(pub u64);
/// specifying Weapon to rollback on client for now
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Component, Copy, Reflect)]
pub struct EntityOnClient(pub u64);

/// if server doesn't approve
#[derive(Debug, Deserialize, Serialize)]
pub enum WeaponRollBack {
    Transform {
        position: Vec3,
        rotation: Radian,
        entity: EntityOnClient
    },
    /// client should +1 on the weaponcounter
    Despawn {
        entity: EntityOnClient
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct UpgradeMessage {
    pub target: Boat,
    pub entity_on_server: EntityOnServer
}
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct UpgradeRollback {
    pub target: Boat
}

// client sends updates to controlling boat's Z-index(physical depth) to the server as a Message
//
// requires [`EntityOnServer`] to be given and correctly represent the boat's entity on the server world
// #[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
// pub struct NewZIndex {
//     pub new_index: ZIndex,
//     pub entity_on_server: EntityOnServer
// }

/// message sender and manager are inserted on every [`ClientOf`] entity on server
pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        // replication
        app.register_component::<WorldSize>();
        app.register_component::<Boat>();
        app.register_component::<CustomTransform>()
            .add_prediction()
            .add_linear_interpolation();
        app.register_component::<ZIndex>();

        app.register_component::<EntityOnServer>();

        // app.register_message::<NewZIndex>().add_direction(NetworkDirection::ClientToServer);

        app.register_component::<OilRigTransform>();
        app.register_component::<PointTransform>().add_linear_interpolation();

        app.register_component::<PlayerStats>();
        app.register_message::<DisplayScore>().add_direction(NetworkDirection::ServerToClient);

        // MUST register these two for every input
        app.add_plugins(InputPlugin::<Rotate>::default());
        app.add_plugins(InputPlugin::<Move>::default());
        app.add_plugins(InputPlugin::<ZIndexUpdate>::default());
        app.register_component::<ActionState<Rotate>>();
        app.register_component::<ActionState<Move>>();
        app.register_component::<ActionState<ZIndexUpdate>>();

        app.register_message::<SpawnWeapon>().add_direction(NetworkDirection::ClientToServer);
        app.register_message::<WeaponRollBack>().add_direction(NetworkDirection::ServerToClient);

        app.register_message::<UpgradeMessage>().add_direction(NetworkDirection::ClientToServer);
        app.register_message::<UpgradeRollback>().add_direction(NetworkDirection::ServerToClient);

        app.register_component::<Weapon>();
        app.register_component::<Transform>();
        app.register_component::<LastSpeed>();
        app.register_component::<TargetRotation>();

        app.add_channel::<SendToClient>(ChannelSettings {
            mode: ChannelMode::UnorderedReliable(ReliableSettings::default()),
            ..default()
        })
            .add_direction(NetworkDirection::ServerToClient);
        app.add_channel::<SendToClientOrdered>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
            ..default()
        })
            .add_direction(NetworkDirection::ServerToClient);

        app.add_channel::<SendToServer>(ChannelSettings {
            mode: ChannelMode::UnorderedReliable(ReliableSettings::default()),
            ..default()
        })
            .add_direction(NetworkDirection::ClientToServer);
        app.add_channel::<SendToServerOrdered>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
            ..default()
        })
            .add_direction(NetworkDirection::ClientToServer);
    }
}

pub mod tcp {
    use tungstenite::Message;


    /// more strongly typed request-parsing and writing
    pub enum TcpClientRequest {
        AvaliableClientId
    }

    pub enum TcpServerResponse {
        AvaliableClientId(u64)
    }

    pub enum ParseError {
        InvalidMessage(String),
        InvalidHeader(Message),
        /// invalid message type
        NotExpected
    }
    impl TcpClientRequest {
        const NEXT_CLIENT_ID_IDENTIFIER: &str = "next_client_id";
        /// buf is a buffer of current read, ideally a Vec that is cleared every read but that's not possible due to read_to_end waiting for EOF
        /// 
        /// ### Panics
        /// if `read_len > buf.len()`
        #[cfg(feature = "server")]
        pub fn try_parse(message: Message) -> Option<Self> {
            
            bevy::prelude::debug!(?message);
            if let Message::Text(text) = message
                && text.as_str() == Self::NEXT_CLIENT_ID_IDENTIFIER
            {
                Some(Self::AvaliableClientId)
            } else {
                None
            }
        }
        pub fn to_msg(&self) -> Message {
            match self {
                Self::AvaliableClientId => Message::Text(Self::NEXT_CLIENT_ID_IDENTIFIER.into())
            }
        }
    }
    impl TcpServerResponse {
        const ID_HEADER: &str = "next_client_id";
        pub fn try_parse(message: Message) -> Result<Self, ParseError> {

            bevy::prelude::debug!(?message);
            if let Message::Text(ref text) = message
                && let text = text.as_str()
            {
                if text.starts_with(Self::ID_HEADER) {
                    let resp = text.split_at(Self::ID_HEADER.len()).1;
                    let id = resp.parse().map_err(|_| ParseError::InvalidMessage("Expected u64".to_owned()))?;

                    Ok(Self::AvaliableClientId(id))
                } else {
                    Err(ParseError::InvalidHeader(message))
                }
            } else {
                Err(ParseError::NotExpected)
            }
        }
        pub fn to_msg(&self) -> Message {
            match self {
                Self::AvaliableClientId(id) => Message::Text(format!("{}{id}", Self::ID_HEADER).into())
            }
        }
    }
}
