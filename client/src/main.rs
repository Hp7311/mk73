#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

mod input;
mod dive;
mod weapon;
mod boat;
mod oil_rig;

use std::f32::consts::PI;
use std::time::Duration;

use bevy::camera_controller::pan_camera::{PanCamera, PanCameraPlugin};
use bevy::color::palettes::css::TEAL;
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_inspector_egui::egui::emath::GuiRounding;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use common::primitives::{
    CustomTransform, ZIndex
};
use common::protocol::{Move, PlayerScore, ProtocolPlugin, Rotate};
use common::world::WorldPlugin;
use common::{Boat, CLIENT_ADDR, MainCamera, MovementPlugin, PROTOCOL_ID, SERVER_ADDR, SubKind};

use lightyear::netcode::{auth::Authentication, Key, NetcodeClient};
use lightyear::prelude::{
    input::native::{ActionState, InputMarker},
    client::{
        ClientPlugins, ClientConfig, NetcodeConfig, WebSocketClientIo, WebSocketScheme
    },
    *
};
use lightyear::websocket::client::WebSocketTarget;
use crate::boat::BoatPlugin;
use crate::dive::DivingPlugin;
use crate::input::InputBufferPlugin;
use crate::oil_rig::OilRigPlugin;
use crate::weapon::WeaponPlugin;

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
        
    // plugins
    .init_state::<BoatState>()
    .add_plugins(WorldPlugin { is_server: false })
    .add_plugins(OilRigPlugin)
    .add_plugins(BoatPlugin)
    .add_plugins(InputBufferPlugin)
    .add_plugins(MovementPlugin { is_server: false, move_weapon: true })
    .add_plugins(DivingPlugin)
    .add_plugins(WeaponPlugin)

    // init
    .add_systems(Startup, setup)
    .add_observer(on_added_actionstate::<Rotate>)
    .add_observer(on_added_actionstate::<Move>)

    .add_systems(Update, update_state)
    .add_systems(Update, move_camera)
    //     Update,
    //     boat_to_target.run_if(in_state(BoatState::Released))

    .add_systems(FixedUpdate, sync_z_index)
        
    .add_plugins(EguiPlugin::default())
    .add_plugins(WorldInspectorPlugin::default())

    .add_observer(on_disconnect)
    .add_observer(on_remove_disconnect);

    app.add_systems(Startup, spawn_gui);
    app.add_systems(Update, update_gui);

    app.run();
}

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
    /// potentially fire a weapon
    FiringWeapon(Duration),
    /// maybe locked in a direction (LMB pressed), unlocked for one frame only
    ///
    /// always transition here with locked: false if can change direction (forward/reverse)
    Moving { locked: bool },
    /// LMB not pressed
    #[default]
    Released,
}

// potential bug: NextState lagging
fn update_state(
    current_state: Res<State<BoatState>>,
    mut setter: ResMut<NextState<BoatState>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    mut commands: Commands
) {
    match current_state.get() {
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
                setter.set(BoatState::Moving { locked: false });
            } else if mouse_button.just_released(MouseButton::Left) {
                commands.trigger(FiresWeapon);
                setter.set(BoatState::Released);
            } else {
                setter.set(BoatState::FiringWeapon(duration));
            }
        }
    }
}

#[derive(Event)]
struct FiresWeapon;

// targetrotation & targetspeed achieved by not clearing ActionState

fn move_camera(
    mut camera: Single<&mut Transform, (With<MainCamera>, Without<Boat>)>,
    ship: Single<&Transform, (With<Boat>, With<Controlled>)>,
) {
    if ship.translation.xy() != camera.translation.xy() {
        camera.translation.x = ship.translation.x;
        camera.translation.y = ship.translation.y;
    }
}

/// here, we assumeee the modified ZIndex is the correct val to use for rendering
fn sync_z_index(
    query: Query<(&ZIndex, &mut Transform), (With<Boat>, Without<Controlled>, Changed<ZIndex>)>
) {
    for (z_index, mut transform) in query {
        transform.translation.z = z_index.0;
    }
}

/// for performance improvements in diving
#[derive(Resource, PartialEq)]
struct BoatType(SubKind);

fn on_disconnect(trigger: On<Add, Disconnected>, query: Query<&Disconnected>) {
    let disconnected = query.get(trigger.entity).unwrap();
    warn!("Client disconnected because: {:?}", disconnected.reason)
}

fn on_remove_disconnect(_: On<Remove, Disconnected>) {
    info!("Client re-connected")
}

fn spawn_gui(mut commands: Commands) {
    commands.spawn((
        Text2d::new("RotateInput: None\nSpeedInput: None\nState: Stopped\nPosition: None\nAltitude: None\nRotation: None\nSpeed: None\nScore: None"),
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
    custom: Single<&CustomTransform, With<Controlled>>,
    transform: Single<&Transform, With<Controlled>>,
    player_score: Single<&PlayerScore, With<Controlled>>
) {
    let state = format!("{:?}", state.into_inner()).split("State(").last().unwrap().to_owned();

    let new_text = format!(
        "RotateInput: {}\nSpeedInput: {}\nState: {}\nPosition: {}\nAltitude: {}\nRotation: {}\nSpeed: {}\nScore: {}",
        rotate.0.0.map(|r| r.to_degrees().round()).unwrap_or(0.0),
        moves.0.0.map(|r| r.get_knots().round()).unwrap_or(0.0),
        state.chars().take(state.len() - 1).collect::<String>(),
        custom.position.0.round(),
        transform.translation.z.round_to_pixels(10.0),
        custom.rotation.to_degrees().round(),
        custom.speed.get_knots().round(),
        player_score.get_score()
    );

    if new_text != text.0 {
        text.0 = new_text;
    }
}

// FIXME if a client leaves -> border shrinks and another client's boat in the shrunk area will lag