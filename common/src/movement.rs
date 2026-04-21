//! plugin to verify inputs and apply them to [`CustomTransform`]
//!
//! note that the client is responsible for translating angle + speed to Euclidean coordinates in [`CustomTransform::position`] and [`Transform`]
//!
//! for client: local prediction
//! for server: confirmation of input and prediction
//!
//! extremely modular :)

use bevy::ecs::schedule::ScheduleLabel;
// note that we're passing owned vals everywhere which doesn't matter for types smaller than 64 bits
use crate::boat::Boat;
use crate::primitives::{CustomTransform, NormalizeRadian, Radian, Speed};
use crate::protocol::{Move, Reversed, Rotate};use crate::collision::out_of_bounds;
use crate::primitives::{MkRect, OutOfBound};
use crate::util::move_with_rotation;
use crate::world::WorldSize;

use bevy::prelude::*;
use lightyear::prelude::input::native::ActionState;

/// plugin to verify inputs and apply them to [`CustomTransform`]
///
/// note that the client is responsible for translating angle + speed to Euclidean coordinates in [`CustomTransform::position`] and [`Transform`]
///
/// for client: local prediction
/// for server: confirmation of input and prediction
pub struct MovementPlugin {
    pub is_server: bool,
}

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        // [`FixedUpdate`] because inputs are tick-synced
        match self.is_server {
            true => {
                app.add_systems(FixedUpdate, (server::rotate, server::move_));
                app.add_systems(FixedUpdate, server::apply_position.after(server::rotate).after(server::move_));
            }
            false => {
                app.add_systems(FixedUpdate, (client::rotate, client::move_));
                app.add_systems(FixedUpdate, client::apply_position.after(client::rotate).after(client::move_));
            }
        }
    }
}

mod server {
    use super::*;

    pub fn rotate(query: Query<(&mut ActionState<Rotate>, &mut CustomTransform, &Boat)>) {
        for (mut action, mut custom, boat) in query {
            validate_max_turn(&mut action, custom.rotation, boat.max_turn());
            apply_rotate(&mut action, &mut custom.rotation);
        }
    }

    pub fn move_(query: Query<(&mut ActionState<Move>, &ActionState<Reversed>, &mut CustomTransform, &Boat)>) {
        for (mut action, reversed, mut custom, boat) in query {
            validate_acceleration(&mut action.0, custom.speed, boat.acceleration());
            validate_speed_cheating(&mut action.0, reversed, boat.max_speed(), boat.rev_max_speed());
            apply_move(&mut action.0, &mut custom.speed);
        }
    }
    pub fn apply_position(
        query: Query<
            (
                &mut CustomTransform,
                &Boat,
                &mut OutOfBound
            ),
            With<Boat>,
        >,
        world_size: Single<&WorldSize>
    ) {
        for (mut custom, boat, mut out_of_bound) in query {
            let mut target = custom.position.to_vec3(0.0);

            target += move_with_rotation(
                custom.rotation,
                custom.speed,
                0.0
            );

            if out_of_bounds(
                &world_size,
                MkRect {
                    center: target.truncate(),
                    dimensions: boat.sprite_size().into(),
                },
                custom.rotation.to_quat(),
            ) {
                out_of_bound.0 |= true;
                return;
            } else if out_of_bound.0 {
                out_of_bound.0 = false;
            }

            custom.position.0 = target.xy();
        }
    }
}
mod client {
    use super::*;
    use lightyear::prelude::Controlled;
    use crate::primitives::OutOfBound;
    use crate::util::move_with_rotation;

    pub fn rotate(
        query: Single<(&mut ActionState<Rotate>, &mut CustomTransform, &Boat), With<Controlled>>,
    ) {
        let (mut action, mut custom, boat) = query.into_inner();
        validate_max_turn(&mut action, custom.rotation, boat.max_turn());
        apply_rotate(&mut action, &mut custom.rotation);
    }
    pub fn move_(query: Single<(&mut ActionState<Move>, &ActionState<Reversed>, &mut CustomTransform, &Boat)>) {
        let (mut action, reversed, mut custom, boat) = query.into_inner();

        validate_acceleration(&mut action.0, custom.speed, boat.acceleration());
        validate_speed_cheating(&mut action.0, reversed, boat.max_speed(), boat.rev_max_speed());
        apply_move(&mut action.0, &mut custom.speed);
    }
    pub fn apply_position(
        query: Single<
            (
                &mut CustomTransform,
                &Boat,
                &mut OutOfBound
            ),
            With<Boat>,
        >,
        world_size: Single<&WorldSize>
    ) {
        let (mut custom, boat, mut out_of_bound) = query.into_inner();

        let mut target = custom.position.to_vec3(0.0);

        target += move_with_rotation(
            custom.rotation,
            custom.speed,
            0.0
        );
        if out_of_bounds(
            &world_size,
            MkRect {
                center: target.truncate(),
                dimensions: boat.sprite_size().into(),
            },
            custom.rotation.to_quat()
        ) {
            out_of_bound.0 |= true;
            return;
        } else if out_of_bound.0 {
            out_of_bound.0 = false;
        }

        custom.position.0 = target.xy();
    }
}

/// validates client input against max turning degree
fn validate_max_turn(rotate: &mut Rotate, current_rotation: Radian, max_turn: Radian) {
    let Some(ref mut target) = rotate.0 else {
        return;
    };
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
/// apply [`ActionState<Rotate>`] to [`CustomTransform`]
///
/// clearing [`Rotate`] to [`None`] when doing so
fn apply_rotate(rotate_input: &mut Rotate, rotation: &mut Radian) {
    let Some(input) = rotate_input.0 else {
        return;
    };
    *rotation = input;

    rotate_input.0 = None;
}

// TODO consider putting these bounds in InputBufferPlugin too for bandwidth, will be easy due to non-bevy funcs
/// check if intended speed greater than acceleration
fn validate_acceleration(move_input: &mut Move, current_speed: Speed, acceleration: Speed) {
    let Some(ref mut target) = move_input.0 else {
        return;
    };
    let diff = *target - current_speed;
    if diff > acceleration {
        *target = current_speed + acceleration;
    } else if diff < -acceleration {
        *target = current_speed - acceleration;
    }
}

/// sanity check: speed upper + lower bound + reversing when can't
/// should be run after validating acceleration
/// - `reverse_max_speed` assumes positive
fn validate_speed_cheating(move_input: &mut Move, reversed: &Reversed, max_speed: Speed, reverse_max_speed: Speed) {
    let Some(ref mut move_input) = move_input.0 else {
        return;
    };
    if *move_input > max_speed {
        error!(
            "Got speed {} greater than max speed {}",
            move_input.get_knots(),
            max_speed.get_knots()
        );
        info!("Setting back");
        *move_input = max_speed;
    } else if *move_input < - reverse_max_speed {
        error!(
            "Got speed {} lesser than reverse max speed {}",
            move_input.get_knots(),
            - reverse_max_speed.get_knots()
        );
        info!("Setting back");
        *move_input = reverse_max_speed;
    }
}

/// clearing to None
fn apply_move(move_input: &mut Move, current_speed: &mut Speed) {
    let Some(input) = move_input.0 else {
        return;
    };
    *current_speed = input;

    move_input.0 = None;
}
