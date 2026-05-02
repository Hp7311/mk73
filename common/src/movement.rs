//! plugin to verify inputs and apply them to [`CustomTransform`]
//!
//! note that the client is responsible for translating angle + speed to Euclidean coordinates in [`CustomTransform::position`] and [`Transform`]
//!
//! for client: local prediction
//! for server: confirmation of input and prediction
//!
//! extremely modular :)

// note that we're passing owned vals everywhere which doesn't matter for types smaller than 64 bits
use crate::boat::Boat;
use crate::primitives::{CustomTransform, LastSpeed, NormalizeRadian, Radian, Speed, TargetRotation, WrapRadian};
use crate::protocol::{Move, Rotate};
use crate::primitives::OutOfBound;
use crate::world::WorldSize;

use bevy::prelude::*;
use lightyear::prelude::input::native::ActionState;
use crate::{eq, Weapon};
use crate::util::move_with_rotation;

/// plugin to verify inputs and apply them to [`CustomTransform`]
///
/// note that the client is responsible for translating angle + speed to Euclidean coordinates in [`CustomTransform::position`] and [`Transform`]
///
/// for client: local prediction
/// for server: confirmation of input and prediction
/// 
/// also includes weapon moving, can be disabled via `move_weapon`, see [`WeaponMovementPlugin`]
pub struct MovementPlugin {
    pub is_server: bool,
    pub move_weapon: bool
}

// wow, private documented items can be seen from public
impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        // `FixedUpdate` because inputs are tick-synced
        if self.is_server  {
            app.add_systems(FixedUpdate, (server::rotate, server::move_));
        } else {
            app.add_systems(FixedUpdate, (client::rotate, client::move_));
        }
        
        if self.move_weapon {
            app.add_plugins(WeaponMovementPlugin);
        }
    }
}

mod server {
    use bevy::prelude::*;
    use lightyear::prelude::input::native::ActionState;
    use crate::boat::Boat;
    use crate::primitives::*;
    use crate::protocol::{Rotate, Move};
    use crate::world::WorldSize;

    pub fn rotate(
        query: Query<(&ActionState<Rotate>, &mut CustomTransform, &Boat)>,
    ) {
        for (action, mut custom, boat) in query {
            super::rotate(action, &mut custom, boat)
        }
    }
    pub fn move_(query: Query<(&ActionState<Move>, &mut CustomTransform, &mut OutOfBound, &Boat)>, world_size: Single<&WorldSize>) {
        for (action, mut custom, mut out_of_bound, boat) in query {
            super::move_(action, &mut custom, boat, &world_size, &mut out_of_bound);
        }
    }
}
mod client {
    use bevy::prelude::*;
    use lightyear::prelude::Controlled;
    use lightyear::prelude::input::native::ActionState;
    use crate::boat::Boat;
    use crate::primitives::*;
    use crate::protocol::{Rotate, Move};
    use crate::world::WorldSize;

    pub fn rotate(
        query: Single<(&ActionState<Rotate>, &mut CustomTransform, &Boat), With<Controlled>>,
    ) {
        let (action, mut custom, boat) = query.into_inner();
        super::rotate(action, &mut custom, boat)
    }
    pub fn move_(query: Single<(&ActionState<Move>, &mut CustomTransform, &mut OutOfBound, &Boat), With<Controlled>>, world_size: Single<&WorldSize>) {
        let (action, mut custom, mut out_of_bound, boat) = query.into_inner();
        super::move_(action, &mut custom, boat, world_size.into_inner(), &mut out_of_bound);
    }
}

fn rotate(rotate: &ActionState<Rotate>, custom: &mut CustomTransform, boat: &Boat) {
    let Some(mut target) = rotate.0.0 else { return; };
    validate_max_turn(&mut target, custom.rotation, boat.max_turn());
    custom.rotation = target;
}

fn move_(move_input: &ActionState<Move>, custom: &mut CustomTransform, boat: &Boat, world_size: &WorldSize, out_of_bound: &mut OutOfBound) {
    let Some(mut target) = move_input.0.0 else { return; };
    validate_acceleration(&mut target, custom.speed, boat.acceleration());

    if validate_speed_cheating(&target, boat.max_speed(), boat.rev_max_speed()) == PlayerValidity::PotentialCheating {
        return;
    }
    custom.speed = target;

    if !custom.move_position_checked(world_size, boat.sprite_size()) {
        out_of_bound.0 = true;
    }
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

// TODO should we clear to None after applying? if we don't we can use it for moving to target but maybe bandwidth

// TODO consider putting these bounds in InputBufferPlugin too for bandwidth, will be easy due to non-bevy funcs
/// check if intended speed greater than acceleration
fn validate_acceleration(target: &mut Speed, current_speed: Speed, acceleration: Speed) {
    let diff = *target - current_speed;
    if diff > acceleration {
        *target = current_speed + acceleration;
    } else if diff < -acceleration {
        *target = current_speed - acceleration;
    }
}

#[derive(PartialEq)]
enum PlayerValidity {
    PotentialCheating,
    Normal
}

/// sanity check: speed upper + lower bound
/// should be run after validating acceleration
/// - `reverse_max_speed` assumes positive
#[must_use]
fn validate_speed_cheating(target: &Speed, max_speed: Speed, reverse_max_speed: Speed) -> PlayerValidity {
    if *target > max_speed {
        error!(
            "Got speed {} greater than max speed {}",
            target.get_knots(),
            max_speed.get_knots()
        );
        PlayerValidity::PotentialCheating
    } else if *target < - reverse_max_speed {
        error!(
            "Got speed {} lesser than reverse max speed {}",
            target.get_knots(),
            - reverse_max_speed.get_knots()
        );
        PlayerValidity::PotentialCheating
    } else {
        PlayerValidity::Normal
    }
}

/// requires each weapon entity to have:
/// - `Transform`
/// - `TargetRotation`
/// - `Weapon`
/// - `LastSpeed`
struct WeaponMovementPlugin;

impl Plugin for WeaponMovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, (rotate_weapon, move_weapon).chain());  // chain and FixedUpdate for minimal diff between server and cient
    }
}


fn rotate_weapon(q: Query<(&mut Transform, &TargetRotation, &Weapon)>) {
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
        let move_by = move_with_rotation(transform.rotation.wrap_radian(), speed, transform.translation.z);
        transform.translation += move_by;
    }
}
