use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{DEFAULT_MAX_TURN_DEG, DEFAULT_SPRITE_SHRINK, primitives::{CircleHud, CustomTransform, OutOfBound, Speed}, protocol::{Reversed, Rotate}, weapon::Weapon, world::WorldSize};

#[derive(Component, Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Boat {
    Yasen,
}

#[derive(Component, Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubKind {
    Submarine,
    SurfaceShip,
}

impl Boat {
    pub fn sub_kind(&self) -> SubKind {
        match self {
            Self::Yasen => SubKind::Submarine,
        }
    }
    pub fn file_name(&self) -> &'static str {
        match self {
            Self::Yasen => "yasen.png",
        }
    }
    pub fn get_armanents(&self) -> Vec<Weapon> {
        match self {
            Self::Yasen => vec![Weapon::Set65],
        }
    }
    pub fn default_weapon(&self) -> Option<Weapon> {
        match self {
            Self::Yasen => Some(Weapon::Set65),
        }
    }
    pub fn max_speed(&self) -> Speed {
        Speed::from_knots(match self {
            Self::Yasen => 35.0,
        })
    }
    pub fn rev_max_speed(&self) -> Speed {
        Speed::from_knots(match self {
            Self::Yasen => 21.0,
        })
    }
    pub fn diving_speed(&self) -> Speed {
        Speed::from_raw(match self {
            Self::Yasen => 0.004,
        })
    }
    pub fn acceleration(&self) -> Speed {
        Speed::from_knots(match self {
            Self::Yasen => 2.0,
        })
    }
    /// raw file size * [`DEFAULT_SPRITE_SHRINK`]
    pub fn sprite_size(&self) -> Vec2 {
        (match self {
            Self::Yasen => vec2(1024.0, 156.0),
        }) * DEFAULT_SPRITE_SHRINK
    }
    /// max turn in degrees
    pub fn max_turn(&self) -> f32 {
        DEFAULT_MAX_TURN_DEG
    }
    /// radius
    pub fn radius(&self) -> f32 {
        self.sprite_size().x / 2.0
    }
}

use lightyear::prelude::*;
// movements

/// extract or return
macro_rules! extract {
    ($in:expr, Option) => {
        match $in {
            Some(x) => x,
            None => return
        }
    };
    ($in:expr, Result) => {
        match $in {
            Ok(x) => x,
            Err(e) => {
                error!("Unwrapping on Err({:?})", e);
                return
            }
        }
    }
}
/// pass on inputs with world bound checking
fn act_on_inputs(
    world_size: &WorldSize,
    custom: &mut CustomTransform,
    rotate: &Rotate,
    moves: &Move,
    reversed: &Reversed
) {
    // validate

    custom.reversed = reversed.0;
    custom.rotation = extract!(rotate.0, Option)
}

// FIXME
// fn update_transform(
//     query: Single<
//         (
//             &mut CustomTransform,
//             &Children,
//             &Sprite,
//             &mut OutOfBound,
//         ),
//         (With<Boat>, With<Controlled>),
//     >,
//     custom: &mut CustomTransform,
//     world_size: &WorldSize,
//     rotate: &Rotate,
//     moves: &Move,
//     reversed: &Reversed
// ) {
//     let (mut transform, mut custom, children, sprite, mut out_of_bound) = query.into_inner();

//     let Some(custom_size) = sprite.custom_size else {
//         return;
//     };

//     let mut translation = custom.position.to_vec3(transform.translation.z);

//     translation += move_with_rotation(custom.rotation.to_quat(), custom.speed.get_raw()); // ignores frame lagging temporary

//     custom.position.0 = translation.xy();

//     if out_of_bounds(
//         &world_size,
//         MkRect {
//             center: custom.position.0,
//             dimensions: custom_size.into(),
//         },
//         custom.rotation.to_quat(),
//     ) {
//         custom.position.0 = transform.translation.truncate(); // changes have no effect
//         out_of_bound.0 = true;
//         return;
//     } else if out_of_bound.0 {
//         out_of_bound.0 = false;
//     }

//     // sender.send::<SendToServer>(PlayerAction {
//     //     action: ActionType::Rotate(custom.rotation),
//     //     client: client_id.to_bits()
//     // });
//     // // TODO more info?
//     // sender.send::<SendToServer>(PlayerAction {
//     //     action: ActionType::Move(custom.position.0),
//     //     client: client_id.to_bits()
//     // });

//     let target = Transform {
//         translation,
//         rotation: custom.rotation.to_quat(),
//         scale: Vec3::ONE,
//     };
//     *transform = target;

//     // ^^^^^^^^^^^^^^ only do these if server says so through replication

//     // TODO seperate function
//     for &child in children {
//         if child == circle_huds.0 {
//             circle_huds.1.center = translation.xy();
//             break;
//         }
//     }
// }