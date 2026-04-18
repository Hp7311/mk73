//! plugin to verify inputs and apply them to [`CustomTransform`]
//! 
//! note that the client is responsible for translating angle + speed to Euclidean coordinates in [`CustomTransform::position`] and [`Transform`]
//!
//! for client: local prediction
//! for server: confirmation of input and prediction
//! 
//! extremely modular :)

// note that we're passing owned vals everywhere which doesn't matter for types smaller than 64 bits
use bevy::prelude::*;
use crate::primitives::{CustomTransform, NormalizeRadian, Position, Radian, Speed};
use crate::protocol::{Move, Rotate};
use lightyear::prelude::input::native::ActionState;
use crate::boat::Boat;

/// plugin to verify inputs and apply them to [`CustomTransform`]
///
/// note that the client is responsible for translating angle + speed to Euclidean coordinates in [`CustomTransform::position`] and [`Transform`]
///
/// for client: local prediction
/// for server: confirmation of input and prediction
pub struct MovementPlugin {
    pub is_server: bool
}

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        // [`FixedUpdate`] because inputs are tick-synced
        match self.is_server {
            true => {
                app.add_systems(FixedUpdate, (server::rotate, server::move_));
            },
            false => {
                app.add_systems(FixedUpdate, (client::rotate, client::move_));
            }
        }
    }
}

mod server {
    use super::*;

    pub fn rotate(
        query: Query<(&mut ActionState<Rotate>, &mut CustomTransform, &Boat)>
    ) {
        for (mut action, mut custom, boat) in query {
            validate_max_turn(
                &mut action,
                custom.rotation,
                boat.max_turn()
            );
            apply_rotate(
                &mut action,
                &mut custom.rotation
            );
        }
    }

    pub fn move_(
        query: Query<(&mut ActionState<Move>, &mut CustomTransform, &Boat)>
    ) {
        for (mut action, mut custom, boat) in query {
            validate_acceleration(
                &mut action.0,
                custom.speed,
                boat.acceleration()
            );
            validate_speed_cheating(
                &mut action.0,
                boat.max_speed(),
                boat.rev_max_speed()
            );
            apply_move(
                &mut action.0,
                &mut custom.speed
            );
        }
    }
}
mod client {
    use lightyear::prelude::Controlled;
    use super::*;

    pub fn rotate(
        query: Single<(&mut ActionState<Rotate>, &mut CustomTransform, &Boat), With<Controlled>>
    ) {
        let (mut action, mut custom, boat) = query.into_inner();
        validate_max_turn(
            &mut action,
            custom.rotation,
            boat.max_turn(),
        );
        apply_rotate(
            &mut action,
            &mut custom.rotation
        );
    }
    pub fn move_(
        query: Single<(&mut ActionState<Move>, &mut CustomTransform, &Boat)>
    ) {
        let (mut action, mut custom, boat) = query.into_inner();
        
        validate_acceleration(
            &mut action.0,
            custom.speed,
            boat.acceleration()
        );
        validate_speed_cheating(
            &mut action.0,
            boat.max_speed(),
            boat.rev_max_speed()
        );
        apply_move(
            &mut action.0,
            &mut custom.speed
        );
    }
}

/// validates client input against max turning degree
fn validate_max_turn(
    rotate: &mut Rotate,
    current_rotation: Radian,
    max_turn: Radian
) {
    let Some(ref mut target) = rotate.0 else { return; };
    let diff = (*target - current_rotation).normalize();
    if diff > max_turn {
        if diff.0 > 0.0 {
            *target = current_rotation.rotate_local_z_ret(max_turn);
        } else if diff.0 < 0.0 {
            *target = current_rotation.rotate_local_z_ret(- max_turn);
        }
    }
}

// TODO should we clear to None after applying? if we don't we can use it for moving to target but maybe bandwidth
/// apply [`ActionState<Rotate>`] to [`CustomTransform`]
///
/// clearing [`Rotate`] to [`None`] when doing so
fn apply_rotate(
    rotate_input: &mut Rotate,
    rotation: &mut Radian
) {
    let Some(input) = rotate_input.0 else { return; };
    *rotation = input;

    rotate_input.0 = None;
}

// TODO consider putting these bounds in InputBufferPlugin too for bandwidth, will be easy due to non-bevy funcs
/// check if intended speed greater than acceleration
fn validate_acceleration(
    move_input: &mut Move,
    current_speed: Speed,
    acceleration: Speed
) {
    let Some(ref mut target) = move_input.0 else { return; };
    let diff = *target - current_speed;
    if diff > acceleration {
        *target = current_speed + acceleration;
    } else if diff < -acceleration {
        *target = current_speed - acceleration;
    }
}

/// sanity check
/// should be run after validating acceleration
fn validate_speed_cheating(
    move_input: &mut Move,
    max_speed: Speed,
    min_speed: Speed
) {
    let Some(ref mut move_input) = move_input.0 else { return; };
    if *move_input > max_speed {
        error!("Got speed {} greater than max speed {}", move_input.get_knots(), max_speed.get_knots());
        info!("Setting back");
        *move_input = max_speed;
    } else if *move_input < min_speed {
        error!("Got speed {} lesser than minimum speed {}", move_input.get_knots(), min_speed.get_knots());
        info!("Setting back");
        *move_input = min_speed;
    }
}

/// clearing to None
fn apply_move(
    move_input: &mut Move,
    current_speed: &mut Speed
) {
    let Some(input) = move_input.0 else { return; };
    *current_speed = input;

    move_input.0 = None;
}