#![allow(clippy::type_complexity)]

mod input;

use std::f32::consts::PI;
use std::time::Duration;

use bevy::camera_controller::pan_camera::{PanCamera, PanCameraPlugin};
use bevy::color::palettes::css::{GRAY, RED, TEAL};
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use common::boat::Boat;
use common::collision::out_of_bounds;
use common::primitives::{CircleHud, CursorPos, CustomTransform, FlipRadian, MeshBundle, MkRect, NormalizeRadian, OutOfBound, Speed, TargetRotation, TargetSpeed, WeaponCounter, WidthHeight, WrapRadian};
use common::protocol::{Move, ProtocolPlugin, Reversed, Rotate};
use common::util::{add_circle_hud, calculate_from_proportion, get_rotate_radian, move_with_rotation, InputExt};
use common::weapon::Weapon;
use common::world::{Background, WorldPlugin, WorldSize};
use common::{
    CIRCLE_HUD, CLIENT_ADDR, MainCamera, MovementPlugin, PROTOCOL_ID, SERVER_ADDR, WATER_SURFACE,
    add_dbg_app, print_num,
};

use lightyear::netcode::auth::Authentication;
use lightyear::netcode::{Key, NetcodeClient};
use lightyear::prelude::client::{ClientConfig, NetcodeConfig, WebSocketClientIo, WebSocketScheme};
use lightyear::prelude::input::native::{ActionState, InputMarker};
use lightyear::prelude::{client::ClientPlugins, *};
use lightyear::websocket::client::WebSocketTarget;

use crate::input::InputBufferPlugin;

#[cfg(all(not(target_family = "wasm"), not(debug_assertions)))]
compile_error! {"Should compile by trunk serve on production"}

// FIXME client disconnects on switching tabs

const DEFAULT_MAX_ZOOM: f32 = 2.0;
const TIME_TO_LAUNCH_WEAPON: Duration = Duration::from_millis(100);
/// absolute value of minimum radians that must be reached to reverse the Boat
const MINIMUM_REVERSE: f32 = PI * (2. / 3.);

#[cfg(all(not(debug_assertions), target_family = "wasm"))]
compile_error!("Web app");

fn main() {
    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    canvas: Some("#bevy_canvas".to_owned()),
                    fit_canvas_to_parent: true,
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                meta_check: bevy::asset::AssetMetaCheck::Never,
                ..default()
            }),
    )
    .add_plugins(ClientPlugins::default())
    .add_plugins(ProtocolPlugin)
    .add_plugins(PanCameraPlugin)
    .insert_resource(ClearColor(TEAL.into()))
    // init
    .init_state::<BoatState>()
    .add_plugins(WorldPlugin { is_server: false })
    .add_plugins(InputBufferPlugin)
    .add_plugins(MovementPlugin { is_server: false })

    // init
    .add_systems(Startup, setup)
    .add_observer(spawn_boat) // FIXME
    .add_observer(on_added_actionstate::<Rotate>)
    .add_observer(on_added_actionstate::<Move>)
    .add_observer(on_added_actionstate::<Reversed>)
    .add_systems(Update, update_state)
    .add_systems(Update, move_camera)
    // .add_systems(
    //     Update,
    //     (
    //         boat_to_target.run_if(in_state(BoatState::Released)),
    //         update_transform,
    //     )
    //         .chain(),
    // )
    .add_systems(Update, (sync_transform_from_custom, move_camera))

    .add_observer(on_disconnect)
    .add_observer(on_remove_disconnect)
        .add_plugins(EguiPlugin::default())
        .add_plugins(WorldInspectorPlugin::default());

    app.add_systems(Startup, spawn_gui);
    app.add_systems(Update, update_gui);

    app.run();
}


// TODO performance problem on client num > 1

/// using hack to achieve system to be only triggered when both
/// [`ActionState<T>`] and [`Controlled`] added
#[deny(unused)]
fn on_added_actionstate<T>(
    trigger: On<Add, ActionState<T>>,
    controlled_action_states: Query<(), (With<ActionState<T>>, With<Controlled>)>,
    mut commands: Commands,
) where
    T: Default + Send + Sync + 'static,
{
    if controlled_action_states.get(trigger.entity).is_err() {
        // other client's
        return;
    }

    let id = commands
        .get_entity(trigger.entity)
        .unwrap()
        .insert(InputMarker::<T>::default())
        .id();
    info!(
        "Added InputMarker for this client (only once): {}, ID: {}",
        std::any::type_name::<T>()
            .split("::")
            .last()
            .unwrap_or_default(),
        id
    );
}

fn setup(mut commands: Commands) {
    let client_id = rand::random_range(0..100);
    let auth = Authentication::Manual {
        server_addr: SERVER_ADDR,
        client_id,
        private_key: Key::default(),
        protocol_id: PROTOCOL_ID,
    };


    let client = commands
        .spawn((
            Client::default(),
            LocalAddr(CLIENT_ADDR),
            PeerAddr(SERVER_ADDR),
            Link::default(),
            NetcodeClient::new(auth, NetcodeConfig::default()).unwrap(),
            WebSocketClientIo {
                // https://github.com/cBournhonesque/lightyear/blob/main/examples/common/src/client.rs#L102
                config: ClientConfig::default(),
                #[cfg(debug_assertions)]
                target: WebSocketTarget::Addr(WebSocketScheme::Plain),
            },
            ReplicationReceiver::default(),
            PredictionManager::default()
        ))
        .id();

    commands.trigger(Connect { entity: client });

    info!("Client {client_id} is requesting");

    commands.spawn((
        Camera2d,
        PanCamera {
            min_zoom: 1.0,
            max_zoom: DEFAULT_MAX_ZOOM,
            key_down: None,
            key_left: None,
            key_right: None,
            key_up: None,
            key_rotate_ccw: None,
            key_rotate_cw: None,
            ..default()
        },
        MainCamera,
    ));
}

#[derive(Debug, States, Clone, Copy, Hash, PartialEq, Eq, Default)]
enum BoatState {
    /// start state
    #[default]
    Stopped,
    /// potentially fire a weapon
    FiringWeapon(Duration),
    /// maybe locked in a direction (LMB pressed), unlocked for one frame only
    ///
    /// always transition here with locked: false if can change direction (forward/reverse)
    Moving { locked: bool },
    /// LMB not pressed
    Released,
}

// potential bug: NextState lagging
fn update_state(
    current_state: Res<State<BoatState>>,
    mut setter: ResMut<NextState<BoatState>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
) {
    match current_state.get() {
        BoatState::Stopped => {
            if mouse_button.just_pressed(MouseButton::Left) {
                setter.set(BoatState::Moving { locked: false });
            }
        }
        BoatState::Moving { locked } => {
            if !locked {
                setter.set(BoatState::Moving { locked: true })
            }

            if !mouse_button.pressed(MouseButton::Left) {
                // not just_released for countering rare bug
                setter.set(BoatState::Released);
            }
        }
        BoatState::Released => {
            if mouse_button.just_pressed(MouseButton::Left) {
                setter.set(BoatState::FiringWeapon(Duration::ZERO));
            }
        }
        BoatState::FiringWeapon(elapsed) => {
            let duration = *elapsed + time.delta();

            if duration > TIME_TO_LAUNCH_WEAPON {
                setter.set(BoatState::Moving { locked: false }); // TODO true?
            } else if mouse_button.just_released(MouseButton::Left) {
                info!("Firing weapon ->>>>>"); // TODO
                setter.set(BoatState::Released);
            } else {
                setter.set(BoatState::FiringWeapon(duration));
            }
        }
    }
}

fn sync_transform_from_custom(
    mut query: Query<(&mut Transform, &CustomTransform), (With<Boat>, Changed<CustomTransform>)>,
) {
    for (mut transform, custom) in query.iter_mut() {
        transform.translation.x = custom.position.x;
        transform.translation.y = custom.position.y;
        transform.rotation = custom.rotation.to_quat();
    }
}

// TODO targetrotation & targetspeed

/// remember the last move angle and rotate toward it when button not pressed
fn boat_to_target(
    boat: Single<
        (
            &Transform,
            &mut CustomTransform,
            &TargetRotation,
            &TargetSpeed,
            &Boat,
        ),
        With<Controlled>,
    >,
) {
    let (transform, mut custom_transform, target_rotation, target_speed, boat) = boat.into_inner();

    // ------ rotation
    let Some(target_rotation) = target_rotation.0 else {
        return;
    };

    let (.., current_rotation) = transform.rotation.to_euler(EulerRot::XYZ);

    let moved = (target_rotation - current_rotation).normalize();

    let ship_max_turn = boat.max_turn();
    if moved.abs() > ship_max_turn.0 {
        if moved > 0.0 {
            custom_transform.rotate_local_z(ship_max_turn);
        } else if moved < 0.0 {
            custom_transform.rotate_local_z(-ship_max_turn);
        }
    } else {
        custom_transform.rotate_local_z(moved.wrap_radian());
    }
    // ------ speed
    let speed_diff = target_speed.get_raw() - custom_transform.speed.get_raw();
    let acceleration = boat.acceleration();
    if speed_diff > acceleration.get_raw() {
        custom_transform.speed.add_raw(acceleration.get_raw());
    } else if speed_diff < -acceleration.get_raw() {
        custom_transform.speed.subtract_raw(acceleration.get_raw());
    } else {
        custom_transform.speed.overwrite(target_speed.0);
    }
}

fn move_camera(
    mut camera: Single<&mut Transform, (With<MainCamera>, Without<Boat>)>,
    ship: Single<&Transform, (With<Boat>, With<Controlled>)>,
) {
    if ship.translation.xy() != camera.translation.xy() {
        camera.translation = ship.translation.with_z(WATER_SURFACE);
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_boat(
    trigger: On<Add, CustomTransform>,
    boats: Query<&Boat>,
    customs: Query<&CustomTransform>,//, With<Boat>>,
    controlled: Query<(), (With<Boat>, With<Controlled>)>,

    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let &custom = customs.get(trigger.entity).unwrap();
    println!("Passed");

    let &boat = boats.get(trigger.entity).unwrap();

    commands
        .get_entity(trigger.entity)
        .unwrap()
        .insert(Reversed(false))
        .insert(BoatBundle {
            boat,
            weapon_counter: WeaponCounter {
                aval_weapons: boat.get_armanents(),
                selected_weapon: boat.default_weapon(),
            },
            sprite: Sprite {
                image: asset_server.load(boat.file_name()), // TODO preload assets
                custom_size: Some(boat.sprite_size()),
                ..default()
            },
            transform: Transform {
                translation: custom.position.extend(WATER_SURFACE),
                rotation: custom.rotation.to_quat(),
                ..default()
            },
            custom_transform: custom,
            ..BoatBundle::default()
        })
        .with_children(|parent| {
            // required for other clients to have a CircleHud for rig's point attraction
            let circle_hud_radius = add_circle_hud(boat.radius());
            let controls = controlled.get(trigger.entity).is_ok();

            let mut hud = parent.spawn((
                Transform::from_xyz(0.0, 0.0, CIRCLE_HUD),
                CircleHud {
                    radius: circle_hud_radius
                },
            ));
            // not client's ship
            if !controls {
                return;
            }

            hud.insert(MeshBundle {
                mesh: Mesh2d(meshes.add(Circle::new(circle_hud_radius).to_ring(3.0))),
                materials: MeshMaterial2d(materials.add(ColorMaterial::from_color(GRAY))),
            })
            .insert(children![
                // reverse indicators
                (
                    Transform::from_xyz(
                        circle_hud_radius * MINIMUM_REVERSE.cos(),
                        circle_hud_radius * MINIMUM_REVERSE.sin(),
                        CIRCLE_HUD
                    ),
                    MeshBundle {
                        mesh: Mesh2d(meshes.add(Segment2d::from_ray_and_length(
                            Ray2d::new(Vec2::ZERO, Dir2::new(MINIMUM_REVERSE.wrap_radian().to_vec()).unwrap()),
                            10.0
                        ))),
                        materials: MeshMaterial2d(materials.add(ColorMaterial::from_color(GRAY)))
                    }
                ),
                (
                    Transform::from_xyz(
                        circle_hud_radius * (-MINIMUM_REVERSE).cos(),
                        circle_hud_radius * (-MINIMUM_REVERSE).sin(),
                        CIRCLE_HUD
                    ),
                    MeshBundle {
                        mesh: Mesh2d(meshes.add(Segment2d::from_ray_and_length(
                            Ray2d::new(Vec2::ZERO, Dir2::new((-MINIMUM_REVERSE).wrap_radian().to_vec()).unwrap()),
                            10.0
                        ))),
                        materials: MeshMaterial2d(materials.add(ColorMaterial::from_color(GRAY)))
                    }
                )
            ]);
        });
}

#[derive(Bundle, Debug, Clone)]
pub struct BoatBundle {
    /// tranform to update in seperate system
    transform: Transform, // cannot
    /// ship's sprite
    sprite: Sprite, // cannot
    /// whether reversed, speed etc
    custom_transform: CustomTransform, // check
    /// where the user's mouse was facing
    mouse_target: TargetRotation,
    /// the target speed of the Boat
    target_speed: TargetSpeed,
    out_of_bound: OutOfBound,
    weapon_counter: WeaponCounter,
    boat: Boat, // check
}

impl Default for BoatBundle {
    /// Should be overwritten:
    /// - `boat`
    /// - `weapon_counter`
    /// - `sprite`
    /// - `transform`
    /// - `custom_transform`
    fn default() -> Self {
        BoatBundle {
            transform: Transform::default(),
            sprite: Sprite::default(),
            custom_transform: CustomTransform::default(),
            out_of_bound: OutOfBound(false),
            mouse_target: TargetRotation::default(),
            target_speed: TargetSpeed::default(),
            weapon_counter: WeaponCounter {
                aval_weapons: vec![],
                selected_weapon: None,
            },
            boat: Boat::Yasen, // should be G5
        }
    }
}

fn on_disconnect(trigger: On<Add, Disconnected>, query: Query<&Disconnected>) {
    let disconnected = query.get(trigger.entity).unwrap();
    warn!("Client disconnected because: {:?}", disconnected.reason)
}

fn on_remove_disconnect(_: On<Remove, Disconnected>) {
    info!("Client re-connected")
}

impl BoatState {
    /// substitute for `run_if` not working on multiple states
    fn in_state_2(first: Self, second: Self) -> impl Fn(Res<State<BoatState>>) -> bool {
        move |state| *state.get() == first || *state.get() == second
    }
}

fn spawn_gui(mut commands: Commands) {
    commands.spawn((
        Text2d::new("RotateInput: None\nSpeedInput: None\nState: Stopped\nPosition: None\nRotation: None\nSpeed: None"),
        TextFont {
            font_size: 30.0,
            ..default()
        },
        Transform::from_xyz(-200.0, 200.0, 0.0),
    ));
}
fn update_gui(
    mut text: Single<&mut Text2d>,
    rotate: Single<&ActionState<Rotate>, With<InputMarker<Rotate>>>,
    moves: Single<&ActionState<Move>, With<InputMarker<Move>>>,
    state: Res<State<BoatState>>,
    custom: Single<&CustomTransform, With<Controlled>>
) {
    let state = format!("{:?}", state.into_inner()).split("State(").last().unwrap().to_owned();

    let new_text = format!(
        "RotateInput: {}\nSpeedInput: {}\nState: {}\nPosition: {}\nRotation: {}\nSpeed: {}",
        rotate.0.0.map(|r| r.to_degrees().round()).unwrap_or(0.0),
        moves.0.0.map(|r| r.get_knots().round()).unwrap_or(0.0),
        state.chars().take(state.len() - 1).collect::<String>(),
        custom.position.0.round(),
        custom.rotation.to_degrees().round(),
        custom.speed.get_knots().round()
    );

    if new_text != text.0 {
        text.0 = new_text;
    }
}

// FIXME if a client leaves -> border shrinks and another client's boat in the shrunk area will lag