//! plugin to buffer inputs from the client
//!
//! client responsible to add verifier and apply them to [`CustomTransform`] and [`Transform`]
//!
//! ## Steps
//! - buffer player's intended action into [`ActionState`], without checking acceleration or maximum turning degree (may change for bandwidth cost)
//! - server and client both add [MovementPlugin](common::MovementPlugin), it verifies input against constraints and does anti-cheat checks,
//!   then apply the rollback-if-incorrect input to [`CustomTransform`]
//! - client update's the boat's [`Transform`]
//! - any cheating input will be rolled-back once confirmation from the server arrives
//!
//! move-to-target currently implemented as not setting [`ActionState`] to `None`
//! maybe store the move-to-target vals in a client-local component once released LMB

use core::f32;
use std::collections::HashSet;

use bevy::{input::common_conditions::input_pressed, prelude::*};
use common::{Boat, eq, in_one_of_states, primitives::{
    CursorPos, CustomTransform, FlipRadian as _, NormalizeRadian as _, Radian, Speed, WrapRadian as _
}, protocol::{Move, Rotate}, util::{Direction, InputEnabled, KeyboardInputExt, add_circle_hud, calculate_from_proportion, get_rotate_radian, input_not_pressed, not_stopped}};
use lightyear::{
    input::client::InputSystems,
    prelude::{
        Controlled,
        input::native::{ActionState, InputMarker},
    },
};
use common::util::in_states_2;
use crate::{BoatState, ui::AfterUpgradeDontClearMoveState};

pub(crate) struct InputBufferPlugin;

// TODO add keyboard WASD control
impl Plugin for InputBufferPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Reversed>();
        app.init_resource::<KeyBoardInputs>();

        app.add_systems(FixedPreUpdate, 
            update_keyboard_inputs.run_if(resource_changed::<ButtonInput<KeyCode>>)
        );
        // buffering inputs
        // MUST BE FixedPreUpdate and in set WriteClientInputs to avoid not sent inputs
        app.add_systems(
            FixedPreUpdate,
            (
                (
                    buffer_rotate_keyboard.run_if(KeyBoardInputs::should_rotate),
                    buffer_move_keyboard.run_if(KeyBoardInputs::should_move)
                ) 
                .after(update_keyboard_inputs),

                (buffer_rotate, buffer_move)
                    .run_if(in_states_2(
                        BoatState::Moving { locked: true },
                        BoatState::Moving { locked: false }
                    ))
                    .run_if(input_pressed(MouseButton::Left))
                    .normal_input(),
            )
                .chain()
                .run_if(not_stopped)
                .in_set(InputSystems::WriteClientInputs)
        );
        app.add_systems(FixedUpdate, (
            // only reset rotation if reached
            |mut rotate: Single<&mut ActionState<Rotate>, With<InputMarker<Rotate>>>, custom: Single<&CustomTransform, (Changed<CustomTransform>, With<Controlled>)>| {
                if let Rotate(Some(input)) = rotate.0
                    && eq!(input, custom.rotation, ?radian)
                {
                    trace!(old_input = ?input, "Clearing Rotate input");
                    rotate.0 = Rotate(None);
                }
            },

            reset_input::<Move>
                .run_if(in_state(AfterUpgradeDontClearMoveState::NoNeed))
                // don't clear if not reached yet
                .run_if(|input: Single<&ActionState<Move>, With<InputMarker<Move>>>, custom: Single<&CustomTransform, With<Controlled>>| input.0.0.is_some() && eq!(custom.speed.get_raw(), input.0.0.unwrap().get_raw()))
        ).run_if(input_not_pressed(MouseButton::Left)));  // 

        // clear AfterUpgradeDontClearMoveState
        app.add_systems(FixedPreUpdate, (|q: Single<(&CustomTransform, &Boat), With<Controlled>>, mut move_input: Single<&mut ActionState<Move>, With<InputMarker<Move>>>, mut state: ResMut<NextState<AfterUpgradeDontClearMoveState>>| {
            let (custom, boat) = q.into_inner();
            
            if custom.speed > boat.max_speed() {
                // excessive assignment after first...
                move_input.0.0 = Some(boat.max_speed());
            } else if custom.speed < - boat.rev_max_speed() {
                move_input.0.0 = Some(- boat.rev_max_speed());
            } else {
                state.set(AfterUpgradeDontClearMoveState::NoNeed);
            }
        }).run_if(in_state(AfterUpgradeDontClearMoveState::Sure))
        .in_set(InputSystems::WriteClientInputs));
    }
}

#[derive(Debug, Resource, Default, Deref, DerefMut)]
pub(crate) struct KeyBoardInputs {
   pub inputs: HashSet<Direction>
}

fn update_keyboard_inputs(
    mut inputs: ResMut<KeyBoardInputs>,
    keys: Res<ButtonInput<KeyCode>>
) {
    inputs.clear();
    for dir in keys.all_moved() {
        inputs.insert(dir);
    }
}

/// buffer the [`ActionState<Rotate>`] for the target rotation the client wants to go to
/// i.e. not modifying ActionState outside [here](self)
fn buffer_rotate(
    cursor_pos: Res<CursorPos>,
    position: Single<&CustomTransform, (With<Controlled>, With<Boat>)>,
    state: Res<State<BoatState>>,
    mut rotate: Single<&mut ActionState<Rotate>, With<InputMarker<Rotate>>>,
    mut reversed: ResMut<Reversed>
) {
    let BoatState::Moving { locked } = state.get() else {unreachable!()};
    let custom_transform = position.into_inner();

    let current_rotation = custom_transform.rotation;

    let raw_moved = get_rotate_radian(custom_transform.position.0, cursor_pos.0); // diff from positive x-axis

    let moved_after_reverse_check = {
        // radians to move from current rotation
        let mut moved_from_current = (raw_moved - current_rotation.0).normalize();

        // -- adjust for reversed ---
        if moved_from_current.abs() > Boat::MINIMUM_REVERSE.0 && !locked {
            // reversing
            reversed.0 = true;
            moved_from_current = moved_from_current.flip();
        } else if moved_from_current.abs() <= Boat::MINIMUM_REVERSE.0 && reversed.0 && !locked {
            // going forwards
            reversed.0 = false;
        } else if reversed.0 {
            // unable to go forward, haven't released key yet
            moved_from_current = moved_from_current.flip();
        }
        current_rotation.rotate_local_z_ret(moved_from_current.wrap_radian())
    };

    // if no difference in rotation
    if eq!(custom_transform.rotation.0, moved_after_reverse_check.0) {
        return;
    }

    rotate.0.0 = Some(moved_after_reverse_check);
    // target_rotation.0 = Some(target_move.wrap_radian());
}

fn buffer_rotate_keyboard(
    inputs: Res<KeyBoardInputs>,
    boat_q: Single<(&CustomTransform, &Boat), With<Controlled>>,
    mut rotate: Single<&mut ActionState<Rotate>, With<InputMarker<Rotate>>>,
) {  // note that this impl of taking max turn results in ugly logs since clearing rotate closure clears it every frame
    /*
2026-07-04T22:15:23.361038Z TRACE client::input: Clearing Rotate input old_input=Radian(1.1955502)
[client/src/input.rs:173:5] rotate.0.0 = None
2026-07-04T22:15:23.361436Z TRACE client::input: Clearing Rotate input old_input=Radian(1.2042768)
[client/src/input.rs:173:5] rotate.0.0 = None
2026-07-04T22:15:23.361734Z TRACE client::input: Clearing Rotate input old_input=Radian(1.2130034)
[client/src/input.rs:173:5] rotate.0.0 = None
2026-07-04T22:15:23.362340Z TRACE client::input: Clearing Rotate input old_input=Radian(1.22173)
[client/src/input.rs:173:5] rotate.0.0 = Some(
    Radian(
        1.1955502,
    ),
)*/
    dbg!(rotate.0.0);
    let (custom, boat) = boat_q.into_inner();
    let left_turn = inputs.iter()
        .find(|i| matches!(i, Direction::Left))
        .map(|_| boat.max_turn())
        .unwrap_or(Radian::ZERO);
    let right_turn = inputs.iter()
        .find(|i| matches!(i, Direction::Right))
        .map(|_| - boat.max_turn())
        .unwrap_or(Radian::ZERO);

    let final_res = custom.rotation + left_turn + right_turn;

    rotate.0.0 = Some(final_res.normalize());
}
/// updates actionstate to target speed that the player wants to go
/// i.e. not modifying ActionState outside [here](self)
fn buffer_move(
    query: Single<(&CustomTransform, /*&mut TargetSpeed,*/ &Boat), With<Controlled>>,
    mut move_action: Single<&mut ActionState<Move>, With<InputMarker<Move>>>,
    cursor_pos: Res<CursorPos>,
    reversed: Res<Reversed>,
) {
    let (custom_transform, boat) = query.into_inner();
    let cursor_distance = cursor_pos.0.distance(custom_transform.position.0);
    let max_speed = if reversed.0 {
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

    // todo acceleration buffering
    let speed = Speed::from_raw(speed);
    if eq!(speed, custom_transform.speed) {
        return;
    }
    move_action.0.0 = Some(speed);
}

fn buffer_move_keyboard(
    inputs: Res<KeyBoardInputs>,
    boat_q: Single<(&CustomTransform, &Boat), With<Controlled>>,
    mut move_action: Single<&mut ActionState<Move>, With<InputMarker<Move>>>,
) {
    let (custom, boat) = boat_q.into_inner();
    let up = inputs.iter()
        .find(|i| matches!(i, Direction::Up))
        .map(|_| boat.acceleration())
        .unwrap_or(Speed::ZERO);
    let down = inputs.iter()
        .find(|i| matches!(i, Direction::Down))
        .map(|_| - boat.acceleration())
        .unwrap_or(Speed::ZERO);

    let final_res = custom.speed + up + down;
    let final_res = Speed::from_raw(final_res.clamp(- boat.rev_max_speed().get_raw(), boat.max_speed().get_raw()));

    if eq!(final_res, custom.speed) {
        return;
    }
    move_action.0.0 = Some(final_res);
}
/// indicates whether ship is reversed.
/// 
/// used to communicate between rotate input buffering and moving input buffering
#[derive(Debug, Clone, Copy, Default, PartialEq, Deref, DerefMut, Resource)]
struct Reversed(pub bool);

/// clear [`ActionState<Rotate>`] if reached
/// 
/// note that we're not clearing ActionSpeed<Move> because of moving issues
#[allow(dead_code)]
fn check_reached(
    query: Single<(&CustomTransform, &mut ActionState<Rotate>), (With<Controlled>, Changed<CustomTransform>)>
) {
    let (custom, mut rotate) = query.into_inner();

    if let Some(target) = rotate.0.0
        && eq!(custom.rotation.0, target.0)
    {
        rotate.0.0 = None;
    }
}

/// reset `T`'s actionstate to `T::default`
fn reset_input<T>(
    mut input: Single<&mut ActionState<T>, With<InputMarker<T>>>
) where
    T: Send + Sync + 'static + Default + PartialEq
{
    if input.0 != T::default() {
        input.0 = T::default();
    }
}
/// reset `T` to its default value if `condition` is met
#[allow(dead_code)]
fn reset_input_if<T>(
    mut condition: impl FnMut(&T) -> bool + 'static
) -> impl FnMut(Single<&mut ActionState<T>, With<InputMarker<T>>>)
where 
    T: Send + Sync + 'static + Default
{
    move |mut input| {
        if condition(&input.0) {
            input.0 = T::default();
        }
    }
}

impl KeyBoardInputs {
    fn not_empty(input: Res<KeyBoardInputs>) -> bool {
        !input.is_empty()
    }
    fn should_rotate(input: Res<KeyBoardInputs>) -> bool {
        input.contains(&Direction::Left) || input.contains(&Direction::Right)
    }
    fn should_move(input: Res<KeyBoardInputs>) -> bool {
        input.contains(&Direction::Up) || input.contains(&Direction::Down)
    }
}