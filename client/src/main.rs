#![allow(clippy::type_complexity)]

mod input;
mod dive;
mod weapon;

use std::collections::HashMap;
use std::f32::consts::PI;
use std::time::Duration;

use bevy::camera_controller::pan_camera::{PanCamera, PanCameraPlugin};
use bevy::color::palettes::css::{GRAY, TEAL};
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_inspector_egui::egui::emath::GuiRounding;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use common::primitives::{
    CustomTransform, MeshBundle, NormalizeRadian as _, OutOfBound, TargetRotation, TargetSpeed, WeaponCounter, WrapRadian as _, ZIndex
};
use common::protocol::{Move, NewZIndex, OilRigInfo, PlayerScore, PointInfo, ProtocolPlugin, Rotate};
use common::world::WorldPlugin;
use common::{CIRCLE_HUD, CLIENT_ADDR, MainCamera, MovementPlugin, PROTOCOL_ID, SERVER_ADDR, OCEAN_SURFACE, OILRIG_SPRITE_SIZE, Boat, SubKind};

use lightyear::netcode::{auth::Authentication, Key, NetcodeClient};
use lightyear::prelude::{
    input::native::{ActionState, InputMarker},
    client::{
        ClientPlugins, ClientConfig, NetcodeConfig, WebSocketClientIo, WebSocketScheme
    },
    *
};
use lightyear::websocket::client::WebSocketTarget;
use crate::dive::DivingPlugin;
use crate::input::InputBufferPlugin;
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
        
    .add_plugins(DivingPlugin)
    // init
    .init_state::<BoatState>()
    .add_plugins(WorldPlugin { is_server: false })
    .add_plugins(InputBufferPlugin)

    // init
    .add_systems(Startup, setup)
    .add_observer(spawn_boat) // FIXME
    .add_observer(on_added_actionstate::<Rotate>)
    .add_observer(on_added_actionstate::<Move>)

    .add_systems(Update, update_state)
    .add_systems(Update, move_camera)
    //     Update,
    //     boat_to_target.run_if(in_state(BoatState::Released))
    .add_plugins(MovementPlugin { is_server: false, move_weapon: true })
    .add_systems(Update, sync_transform_from_custom)

    .add_observer(spawn_rig)
        
    .add_observer(spawn_point)
    .add_systems(Update, sync_point_transform)
        
    .add_plugins(WeaponPlugin)
        
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

fn sync_transform_from_custom(
    mut query: Query<(&mut Transform, &CustomTransform), (With<Boat>, Changed<CustomTransform>)>,
) {
    for (mut transform, custom) in query.iter_mut() {
        transform.translation.x = custom.position.x;
        transform.translation.y = custom.position.y;
        transform.rotation = custom.rotation.to_quat();
    }
}

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

#[allow(clippy::too_many_arguments)]
fn spawn_boat(
    trigger: On<Add, CustomTransform>,
    boats: Query<&Boat>,
    customs: Query<(&CustomTransform, &ZIndex)>,//, With<Boat>>,
    controlled: Query<(), (With<Boat>, With<Controlled>)>,

    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let (&custom, &z_index) = customs.get(trigger.entity).unwrap();
    let &boat = boats.get(trigger.entity).unwrap();
    let controls = controlled.get(trigger.entity).is_ok();

    commands
        .get_entity(trigger.entity).unwrap()
        .insert_if_new(BoatBundle {
            boat,
            // TODO WeaponCounter, OutOfBound etc not needed for not controlling boat
            weapon_counter: WeaponCounter {
                weapons: boat.get_armanents(),
                selected_weapon: boat.default_weapon(),
            },
            sprite: Sprite {
                image: asset_server.load(boat.file_name()), // preload assets
                custom_size: Some(boat.sprite_size()),
                ..default()
            },
            transform: Transform {
                translation: custom.position.extend(z_index),
                rotation: custom.rotation.to_quat(),
                ..default()
            },
            custom_transform: custom,
            ..BoatBundle::default()
        })
        .with_children(|parent| {
            if !controls {
                return;
            }
            let circle_hud_radius = boat.circle_hud_radius();

            parent.spawn((
                MeshBundle {
                    mesh: Mesh2d(meshes.add(Circle::new(circle_hud_radius).to_ring(3.0))),
                    materials: MeshMaterial2d(materials.add(ColorMaterial::from_color(GRAY))),
                },
                Transform::from_xyz(0.0, 0.0, *CIRCLE_HUD)
            ))
            .insert(children![
                // reverse indicators
                (
                    Transform::from_xyz(
                        circle_hud_radius * MINIMUM_REVERSE.cos(),
                        circle_hud_radius * MINIMUM_REVERSE.sin(),
                        *CIRCLE_HUD
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
                        *CIRCLE_HUD
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
        })
        .insert(Name::new("Client's boat"));

    commands.insert_resource(BoatType(boat.sub_kind()));
}

fn spawn_rig(
    trigger: On<Add, OilRigInfo>,
    rigs: Query<&OilRigInfo>,
    assert_server: Res<AssetServer>,
    mut commands: Commands
) {
    // NOTE client-inserted components get removed when server despawns the replicating entity
    let Ok(rig_info) = rigs.get(trigger.entity) else { panic!() };

    commands.get_entity(trigger.entity).unwrap().insert((
        Transform {
            translation: rig_info.position.extend(0.0),
            rotation: rig_info.rotation.to_quat(),
            ..default()
        },
        Sprite {
            image: assert_server.load(rig_info.file_name()),
            custom_size: Some(OILRIG_SPRITE_SIZE),
            ..default()
        },
        Name::new("Oil rig")
    ));
}

// TODO we want points to be "below" the boat, unpredictable with all Z-index = 0.0
fn spawn_point(
    trigger: On<Add, PointInfo>,
    points: Query<&PointInfo>,
    asset_server: Res<AssetServer>,
    mut commands: Commands
) {
    let point_info = points.get(trigger.entity).unwrap();

    commands.get_entity(trigger.entity).unwrap()
        .insert((
            Sprite {
                image: asset_server.load((*point_info.file_name).to_owned()),
                custom_size: Some(PointInfo::custom_size()),
                ..default()
            },
            Transform::from_translation(point_info.position),
            Name::new("Point")
        ));
}

fn sync_point_transform(
    points: Query<(&PointInfo, &mut Transform), Changed<PointInfo>>,
) {
    for (info, mut transform) in points {
        transform.translation.x = info.position.x;
        transform.translation.y = info.position.y;
    }
}
/// for performance improvements in diving
#[derive(Resource, PartialEq)]
struct BoatType(SubKind);

#[derive(Bundle, Debug, Clone)]
pub struct BoatBundle {
    /// tranform to update in seperate system
    transform: Transform, // cannot
    /// ship's sprite
    sprite: Sprite, // cannot
    /// whether reversed, speed etc
    custom_transform: CustomTransform, // check
    /// where the user's mouse was facing
    // mouse_target: TargetRotation,
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
            // mouse_target: TargetRotation::default(),
            target_speed: TargetSpeed::default(),
            weapon_counter: WeaponCounter {
                weapons: HashMap::new(),
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
    player_score: Single<&PlayerScore>
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