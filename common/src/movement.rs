//! plugin to verify inputs and apply them to [`CustomTransform`]
//!
//! note that the client is responsible for translating angle + speed to Euclidean coordinates in [`CustomTransform::position`] and [`Transform`]
//!
//! for client: local prediction
//! for server: confirmation of input and prediction
//!
//! extremely modular :)
#![allow(clippy::type_complexity)]

// note that we're passing owned vals everywhere which doesn't matter for types smaller than 64 bits
use crate::boat::Boat;
use crate::collision::out_of_bound_point;
use crate::primitives::{CustomTransform, LastSpeed, NormalizeRadian, Radian, Size, Speed, TargetRotation, WrapRadian};
use crate::protocol::{Move, Rotate};
use crate::world::WorldSize;

use bevy::prelude::*;
use lightyear::prelude::input::native::ActionState;
use crate::{eq, Weapon};
use crate::util::move_with_rotation;

/// plugin to verify inputs and apply them to [`CustomTransform::rotation`] for `rotate`
/// [`CustomTransform::speed`] and [`CustomTransform::position`] for `move`
///
/// note that the client is responsible for updating [`Transform`]
///
/// for client: local prediction
/// for server: confirmation of input and prediction
/// 
/// also includes weapon moving, can be disabled via `move_weapon`, see [`WeaponMovementPlugin`]
pub struct MovementPlugin {
    pub move_weapon: bool
}

// sometimes jerky movement (not recently observed) (maybe in debug mode?)
// wow, private documented items can be seen from public
impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        // `FixedUpdate` because inputs are tick-synced
        #[cfg(feature = "client")]
        app.add_systems(FixedUpdate, (client::rotate, client::move_));
        #[cfg(feature = "server")]
        app.add_systems(FixedUpdate, (server::rotate, server::move_));
        
        if self.move_weapon {
            app.add_plugins(WeaponMovementPlugin);
        }
    }
}


#[cfg(feature = "server")]
mod server {
    use super::*;

    pub fn rotate(
        query: Query<(&ActionState<Rotate>, &mut CustomTransform, &Boat)>,
    ) {
        for (action, mut custom, boat) in query {
            super::rotate_inner(action, &mut custom, boat)
        }
    }
    pub fn move_(query: Query<(&ActionState<Move>, &mut CustomTransform, &Boat)>, world_size: Single<&WorldSize>) {
        for (action, mut custom, boat) in query {
            super::move_inner(action, &mut custom, boat, &world_size);
        }
    }
}

#[cfg(feature = "client")]
mod client {
    use super::*;
    use lightyear::prelude::Controlled;

    pub fn rotate(
        query: Single<(&ActionState<Rotate>, &mut CustomTransform, &Boat), With<Controlled>>,
    ) {
        let (action, mut custom, boat) = query.into_inner();
        super::rotate_inner(action, &mut custom, boat)
    }
    pub fn move_(query: Single<(&ActionState<Move>, &mut CustomTransform, &Boat), With<Controlled>>, world_size: Single<&WorldSize>) {
        let (action, mut custom, boat) = query.into_inner();
        super::move_inner(action, &mut custom, boat, world_size.into_inner());
    }
}

fn rotate_inner(rotate_input: &ActionState<Rotate>, custom: &mut CustomTransform, boat: &Boat) {
    let Some(mut target) = rotate_input.0.0 else { return; };
    validate_max_turn(&mut target, custom.rotation, boat.max_turn());
    custom.rotation = target;
}

fn move_inner(move_input: &ActionState<Move>, custom: &mut CustomTransform, boat: &Boat, world_size: &WorldSize) {
    // move no matter what to achieve free movement after released LMB
    if !custom.move_position_checked(world_size, boat.render_size()) {
        // maybe UI pop-up
    }
    let Some(mut target) = move_input.0.0 else {
        return;
    };
    validate_acceleration(
        &mut target,
        custom.speed,
        boat.acceleration(),
    );

    // if validate_speed_cheating(&target, boat.max_speed(), boat.rev_max_speed()) == SpeedValidity::Error {
    //     warn!(?boat, target = target.get_knots());
    //     return;
    // }
    custom.speed = target;
}

/// validates client input against max turning degree
/// - `target`: `let Some(target) = rotate_input else { return }`
fn validate_max_turn(target: &mut Radian, current_rotation: Radian, max_turn: Radian) {
    let diff = (*target - current_rotation).normalize();
    if diff.abs() > max_turn {
        if diff.0 > 0.0 {
            *target = current_rotation.rotate_local_z_ret(max_turn);
        } else if diff.0 < 0.0 {
            *target = current_rotation.rotate_local_z_ret(-max_turn);
        }
    }
}

// should we clear to None after applying? if we don't we can use it for moving to target but maybe bandwidth

/// check if intended speed greater than acceleration
fn validate_acceleration(
    target: &mut Speed,
    current_speed: Speed,
    acceleration: Speed,
) {
    let diff = *target - current_speed;
    if diff > acceleration {
        *target = current_speed + acceleration;
    } else if diff < -acceleration {
        *target = current_speed - acceleration;
    }
}
// FIXME playerscore clearing to 0 on upgrade with points around sometimes (???????)

#[derive(PartialEq)]
#[allow(dead_code)]
enum SpeedValidity {
    Error,
    Normal
}

/// sanity check: speed upper + lower bound
/// should be run after validating acceleration
/// - `reverse_max_speed` assumes positive from [`Boat`]
#[must_use = "Result may be a err value which should be handled"]
#[allow(dead_code)]
fn validate_speed_cheating(target: &Speed, max_speed: Speed, reverse_max_speed: Speed) -> SpeedValidity {
    if *target > max_speed {
        warn!(
            "Got speed {} greater than max speed {}",
            target.get_knots(),
            max_speed.get_knots()
        );
        SpeedValidity::Error
    } else if *target < - reverse_max_speed {
        warn!(
            "Got speed {} lesser than reverse max speed {}",
            target.get_knots(),
            - reverse_max_speed.get_knots()
        );
        SpeedValidity::Error
    } else {
        SpeedValidity::Normal
    }
}

// TODO more advanced moving
// - going up/down in depth
// - tracking

/// requires each weapon entity to have:
/// - `Transform`
/// - `TargetRotation`
/// - `Weapon`
/// - `LastSpeed`
struct WeaponMovementPlugin;

impl Plugin for WeaponMovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, (rotate_weapon, move_weapon).chain());  // chain and FixedUpdate for minimal diff between server and cient
        app.add_systems(FixedUpdate, despawn_weapon_out_of_bound);
    }
}

fn rotate_weapon(q: Query<(&mut Transform, &TargetRotation, &Weapon)>) {  // simply don't add TargetRotation for weapons e.g. shells
    for (mut transform, target, weapon) in q {
        let max_turn_radian = weapon.max_turn_radian();
        let current_rotation = transform.rotation.wrap_radian();

        let moved_from_current = (target.0 - current_rotation).normalize();

        if eq!(moved_from_current.0, 0.0) {
            continue;
        }

        if moved_from_current.abs() > max_turn_radian {
            if moved_from_current.0 < 0.0 {
                transform.rotate_local_z(-max_turn_radian.0);
            } else {
                transform.rotate_local_z(max_turn_radian.0);
            }
        } else {
            transform.rotate_local_z(moved_from_current.0);
        }
    }
}

fn move_weapon(query: Query<(&mut Transform, &Weapon, &mut LastSpeed)>) {
    for (mut transform, weapon, mut last_speed) in query {
        let mut speed = last_speed.0;
        let speed_diff = weapon.max_speed() - last_speed.0;
        let acceleration = weapon.acceleration();

        if speed_diff > acceleration {
            speed += acceleration;
        } else if speed_diff < -acceleration {
            speed -= acceleration;
        } else if speed_diff.abs() > 0.1 {
            speed = weapon.max_speed();
        } // weapons don't have dynamic speeds. therefore it'll always try to go at max speed

        last_speed.0 = speed;

        // update transform
        let move_by = move_with_rotation(transform.rotation.wrap_radian(), speed);
        transform.translation += move_by;
    }
}

fn despawn_weapon_out_of_bound(
    mut commands: Commands,
    weapons: Query<(&Transform, Entity), (With<Weapon>, Changed<Transform>)>,
    world_size: Single<&WorldSize>
) {
    for (transform, id) in weapons {
        if out_of_bound_point(&world_size, transform.translation.xy()) {
            commands.get_entity(id).unwrap()
                .despawn();
        }
    }
}