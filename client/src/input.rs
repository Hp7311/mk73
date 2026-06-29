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

use bevy::{input::common_conditions::input_pressed, prelude::*};
use common::{Boat, eq, primitives::{
    CursorPos, CustomTransform, FlipRadian as _, NormalizeRadian as _, Speed, WrapRadian as _
}, protocol::{Move, Rotate}, util::{InputEnabled, add_circle_hud, calculate_from_proportion, get_rotate_radian, input_not_pressed}};
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

impl Plugin for InputBufferPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Reversed>();

        // buffering inputs
        // MUST BE FixedPreUpdate and in set WriteClientInputs to avoid jerky movement
        app.add_systems(
            FixedPreUpdate,
            (buffer_rotate, buffer_move)
                .run_if(in_states_2(
                    BoatState::Moving { locked: true },
                    BoatState::Moving { locked: false }
                ))
                .run_if(input_pressed(MouseButton::Left))
                .normal_input()
                .in_set(InputSystems::WriteClientInputs)
        );
        app.add_systems(FixedPreUpdate, (
            // only reset rotation if reached
            |mut rotate: Single<&mut ActionState<Rotate>, With<InputMarker<Rotate>>>, custom: Single<&CustomTransform, (Changed<CustomTransform>, With<Controlled>)>| {
                if let Rotate(Some(input)) = rotate.0
                    && eq!(input, custom.rotation, ?radian)
                {
                    trace!(old_input = ?input, "Clearing Rotate input");
                    rotate.0 = Rotate(None);
                }
            },
            reset_input::<Move>.run_if(in_state(AfterUpgradeDontClearMoveState::NoNeed))
        ).run_if(input_not_pressed(MouseButton::Left)));


        app.add_systems(FixedPreUpdate, (|q: Single<(&CustomTransform, &Boat), With<Controlled>>, mut move_input: Single<&mut ActionState<Move>, With<InputMarker<Move>>>, mut state: ResMut<NextState<AfterUpgradeDontClearMoveState>>| {
            let (custom, boat) = q.into_inner();
            
            if custom.speed > boat.max_speed() {  // excessive assignment after first...
                move_input.0.0 = Some(boat.max_speed())
            } else if custom.speed < - boat.rev_max_speed() {
                move_input.0.0 = Some(- boat.rev_max_speed());
            } else {
                state.set(AfterUpgradeDontClearMoveState::NoNeed);
            }
        }).run_if(in_state(AfterUpgradeDontClearMoveState::Sure))
        .in_set(InputSystems::WriteClientInputs));
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

    if eq!(custom_transform.rotation.0, moved_after_reverse_check.0) {
        return;
    }

    rotate.0.0 = Some(moved_after_reverse_check);
    // target_rotation.0 = Some(target_move.wrap_radian());
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

    if eq!(speed, custom_transform.speed.get_raw()) {
        return;
    }
    // target_speed.0 = Speed::from_raw(speed);
    let speed = Speed::from_raw(speed);
    move_action.0.0 = Some(speed);
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