//! plugin to buffer inputs from the client
//!
//! client responsible to add verifier and apply them to [`CustomTransform`] and [`Transform`]

use bevy::prelude::*;
use common::{
    boat::Boat,
    primitives::{
        CursorPos, CustomTransform, FlipRadian as _, NormalizeRadian as _, Radian, Speed,
        WrapRadian as _,
    },
    protocol::{Move, Reversed, Rotate},
    util::{add_circle_hud, calculate_from_proportion, get_rotate_radian},
};
use lightyear::{
    input::client::InputSystems,
    prelude::{
        Controlled,
        input::native::{ActionState, InputMarker},
    },
};

use crate::{BoatState, MINIMUM_REVERSE};

pub struct InputBufferPlugin;

impl Plugin for InputBufferPlugin {
    fn build(&self, app: &mut App) {
        // inputs
        // MUST BE FixedPreUpdate and in set WriteClientInputs to avoid jerky movement
        app.add_systems(
            FixedPreUpdate,
            (buffer_rotate, buffer_move)
                .in_set(InputSystems::WriteClientInputs)
                .run_if(resource_changed::<CursorPos>)
                .run_if(BoatState::in_state_2(
                    BoatState::Moving { locked: true },
                    BoatState::Moving { locked: false },
                )),
        );
    }
}

// TODO should we put bound checking (max turn deg, acceleration, max speed) to handling input for anti-cheat?

/// buffer the [`ActionState<Rotate>`]
///
/// checks about max turn etc
fn buffer_rotate(
    cursor_pos: Res<CursorPos>,
    position: Single<&CustomTransform, (With<Controlled>, With<Boat>)>,
    state: Res<State<BoatState>>,
    mut rotate: Single<&mut ActionState<Rotate>, With<InputMarker<Rotate>>>,
    mut reversed: Single<&mut ActionState<Reversed>, With<InputMarker<Reversed>>>,
    boat: Single<&Boat, With<Controlled>>,
) {
    let BoatState::Moving { locked } = state.get() else {
        unreachable!()
    };
    let custom_transform = position.into_inner();

    let mut current_rotation = custom_transform.rotation;

    let raw_moved = get_rotate_radian(custom_transform.position.0, cursor_pos.0); // diff from + x axis

    let moved_from_current = {
        // radians to move from current rotation
        let mut moved_from_current = (raw_moved - current_rotation.0).normalize();

        // -- adjust for reversed ---
        if moved_from_current.abs() > MINIMUM_REVERSE && !locked {
            // reversing
            // custom_transform.reversed = true;
            *reversed.0 |= true;
            moved_from_current = moved_from_current.flip();
        } else if moved_from_current.abs() <= MINIMUM_REVERSE && reversed.to_bool() && !locked {
            // going forwards
            // custom_transform.reversed = false;
            *reversed.0 = false;
        } else if reversed.to_bool() {
            // unable to go forward, haven't released key yet
            moved_from_current = moved_from_current.flip();
        }
        moved_from_current
    };

    // rotate.0.0 = Some(target_move.wrap_radian());
    // return;
    // target_rotation.0 = Some(target_move.wrap_radian());

    let max_turn = boat.max_turn();

    // indirection
    if moved_from_current.abs() > max_turn.0 {
        // turning degree bigger than maximum
        if moved_from_current > 0.0 {
            current_rotation.rotate_local_z(max_turn);
        } else if moved_from_current < 0.0 {
            current_rotation.rotate_local_z(-max_turn);
        }
    } else if moved_from_current != 0.0 {
        // normal
        current_rotation.rotate_local_z(moved_from_current.wrap_radian());
    }

    rotate.0.0 = Some(current_rotation);
}

fn buffer_move(
    query: Single<(&CustomTransform, /*&mut TargetSpeed,*/ &Boat), With<Controlled>>,
    mut move_action: Single<&mut ActionState<Move>, With<InputMarker<Move>>>,
    cursor_pos: Res<CursorPos>,
    reversed: Single<&ActionState<Reversed>, With<InputMarker<Reversed>>>,
) {
    let (custom_transform, boat) = query.into_inner();
    let cursor_distance = cursor_pos.0.distance(custom_transform.position.0);
    let max_speed = if *reversed.0 {
        -boat.rev_max_speed().get_raw()
    } else {
        boat.max_speed().get_raw()
    };

    let speed = calculate_from_proportion(
        cursor_distance,
        add_circle_hud(boat.radius()),
        max_speed,
        boat.radius(),
    );

    // target_speed.0 = Speed::from_raw(speed);
    let mut speed = Speed::from_raw(speed);

    // adjust for acceleration
    let speed_diff = speed - custom_transform.speed;
    let acceleration = boat.acceleration();

    if speed_diff > acceleration {
        // accelerating too much forwards
        // custom_transform.speed += acceleration;
        speed += acceleration;
    } else if speed_diff < -acceleration {
        // accelerating too much backwards
        // custom_transform.speed -= acceleration;
        speed -= acceleration;
    }
    // not exceeding acceleration
    // else if speed_diff.abs() > 0.1 {
    // custom_transform.speed.overwrite(speed);
    // }
    move_action.0.0 = Some(speed);
}
