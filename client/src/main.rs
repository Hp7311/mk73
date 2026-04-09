use std::f32::consts::PI;
use std::time::Duration;

use bevy::camera_controller::pan_camera::{PanCamera, PanCameraPlugin};
use bevy::color::palettes::css::{GRAY, TEAL};
use bevy::prelude::*;

use common::boat::Boat;
use common::collision::out_of_bounds;
use common::primitives::{
    CircleHud, CursorPos, CustomTransform, DecimalPoint, FlipRadian, MeshBundle, MkRect,
    NormalizeRadian, OutOfBound, Radian, Speed, TargetRotation, TargetSpeed, ToRadian,
    WeaponCounter,
};
use common::protocol::{ActionType, DbgClientInput, MinimalBoat, PlayerAction, PlayerPos, ProtocolPlugin, SendToServer};
use common::util::{
    add_circle_hud, calculate_from_proportion, get_rotate_radian, move_with_rotation,
};
use common::weapon::Weapon;
use common::world::{Background, WorldPlugin, WorldSize};
use common::{
    CIRCLE_HUD, CLIENT_ADDR, MainCamera, PROTOCOL_ID, SERVER_ADDR, WATER_SURFACE, add_debug_systems, print_num,
};

use lightyear::input::client::InputSystems;
use lightyear::link::LinkConditioner;
use lightyear::netcode::auth::Authentication;
use lightyear::netcode::{Key, NetcodeClient};
use lightyear::prelude::client::{ClientConfig, NetcodeConfig, WebSocketClientIo, WebSocketScheme};
use lightyear::prelude::input::native::{ActionState, InputMarker};
use lightyear::prelude::{client::ClientPlugins, *};
use lightyear::websocket::client::WebSocketTarget;

#[cfg(not(target_family = "wasm"))]
// compile_error!{"Should compile by trunk serve"}
const FIX_LATER: &str = "Uncomment above in production, gives ugly warnings in rust-analyzer";

// FIXME client disconnects on switching tabs

const DEFAULT_MAX_ZOOM: f32 = 2.0;
const TIME_TO_LAUNCH_WEAPON: Duration = Duration::from_millis(100);
/// absolute value of minimum radians that must be reached to reverse the Boat
const MINIMUM_REVERSE: f32 = PI * (2. / 3.);

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
    // .init_state::<BoatState>()
    // init
    .add_plugins(WorldPlugin)
    .add_systems(Startup, setup)
    // .add_observer(spawn_boat)
    .add_observer(demo_spawn_sprite)
    .add_systems(Update, demo_update_transform)
    .add_systems(FixedUpdate, (local_simulation, buffer_input.in_set(InputSystems::WriteClientInputs)))
    // .add_systems(Update, update_state)
    // move
    // .add_systems(Update, move_camera)
    // .add_systems(
    //     Update,
    //     (
    //         (rotate_boat, move_boat).run_if(|state: Res<State<BoatState>>| {
    //             matches!(state.get(), BoatState::FreeDir | BoatState::LockedDir)  // consider .run_if(in_state(LockedDir))
    //         }),
    //         boat_to_target.run_if(in_state(BoatState::Released)),
    //         update_transform,
    //     )
    //         .chain(),
    // )
    .add_observer(on_disconnect)
    .add_observer(on_remove_disconnect);
    // .add_systems(Update, update_boat_transform_from_replicate);
    // .add_systems(Update, dbg_transform_sync);

    // add_debug_systems!(&mut app, demo_log);
    // print_num!(&mut app, Predicted);
    // add_debug_systems!(&mut app, count_sprite);

    app.run();
}

fn count_sprite(sprites: Query<&Sprite>) {
    info!("{} sprites", sprites.iter().len());
}
fn demo_log(q: Query<(&PlayerPos, &Confirmed<PlayerPos>), (With<Sprite>, With<Predicted>)>) {
    for (pos, confirmed) in q {
        info!(Predicted = pos.0.to_string(), Confirmed = confirmed.0.0.to_string());
    }
}

fn buffer_input(
    mut query: Query<&mut ActionState<DbgClientInput>>,  // FIXME With<InputMarker<DbgClientInput>>
    keypresses: Res<ButtonInput<KeyCode>>
) {
    let Ok(mut action_state) = query
        .single_mut()
        .inspect_err(|e| warn!("Single: {:?}", e))
    else { return; };

    if keypresses.pressed(KeyCode::KeyW) {
        action_state.0 = DbgClientInput::Move(vec2(10.0, 0.0));
    } else {
        action_state.0 = DbgClientInput::None;
    }
}

fn local_simulation(
    mut query: Query<(&mut PlayerPos, &ActionState<DbgClientInput>), (With<Predicted>, With<Controlled>)>
) {
    let Ok((mut pos, action)) = query.single_mut() else { return };
    
    if let DbgClientInput::Move(move_by) = action.0 {
        info!("Moving right {}", move_by);
        pos.0 += move_by;
    }
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
            Link::new(Some(LinkConditioner::new(
                LinkConditionerConfig::average_condition(),
            ))),
            ReplicationReceiver::default(),
            PredictionManager::default(),
            NetcodeClient::new(auth, NetcodeConfig::default()).unwrap(),
            WebSocketClientIo {
                // https://github.com/cBournhonesque/lightyear/blob/main/examples/common/src/client.rs#L102
                config: ClientConfig::default(),
                target: WebSocketTarget::Addr(WebSocketScheme::Plain),
            },
        ))
        .id();

    info!(?client);

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
    /// locked in a direction (LMB pressed)
    LockedDir,
    /// middle state between `LockedDir` and `Released`, can change direction
    FreeDir,
    /// LMB not pressed
    Released,
}

fn update_state(
    current_state: Res<State<BoatState>>,
    mut setter: ResMut<NextState<BoatState>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
) {
    match current_state.get() {
        BoatState::Stopped => {
            if mouse_button.just_pressed(MouseButton::Left) {
                // may not be correct
                setter.set(BoatState::FreeDir);
            }
        }
        BoatState::LockedDir => {
            if mouse_button.just_released(MouseButton::Left) {
                setter.set(BoatState::Released);
            }
        }
        BoatState::FreeDir => {
            // allow 1 frame in freedir
            setter.set(BoatState::LockedDir);
        }
        BoatState::Released => {
            if mouse_button.just_pressed(MouseButton::Left) {
                setter.set(BoatState::FiringWeapon(Duration::ZERO));
            }
        }
        BoatState::FiringWeapon(elapsed) => {
            let duration = *elapsed + time.delta();

            if duration > TIME_TO_LAUNCH_WEAPON {
                setter.set(BoatState::FreeDir);
            } else if mouse_button.just_released(MouseButton::Left) {
                info!("Firing weapon ->>>>>"); // TODO
                setter.set(BoatState::Released);
            } else {
                setter.set(BoatState::FiringWeapon(duration));
            }
        }
    }
}

fn dbg_transform_sync(
    mut query: Query<(&mut Transform, &CustomTransform), With<Boat>>
) {
    for (mut transform, custom) in query.iter_mut() {
        transform.translation.x = custom.position.x;
        transform.translation.y = custom.position.y;
        transform.rotation = custom.rotation.to_quat();
    }
}

// does not descriminate controlled
fn update_boat_transform_from_replicate(
    mut query: Query<(&MinimalBoat, &mut Transform, &CustomTransform)>,
) {
    for (template, mut transform, custom) in query.iter_mut() {
        info!("Position: {:?}\nSpeed: {}\nRotation: {}", custom.position.0, custom.speed.get_knots(), custom.rotation.to_degrees());
        transform.translation.x = template.position.x;
        transform.translation.y = template.position.y;
        transform.rotation = template.rotation.to_quat();

        // custom.position.0 = template.position;
        // custom.rotation.0 = template.rotation;
    }
}

// TODO targetrotation & targetspeed
/// handle rotation (manipulate [`CustomTransform`])
fn rotate_boat(
    query: Single<(&mut CustomTransform, &mut TargetRotation, &Boat), With<Controlled>>,
    state: Res<State<BoatState>>,
    cursor_pos: Res<CursorPos>,
) {
    let state = *state.get();

    let (mut custom_transform, mut target_rotation, boat) = query.into_inner();
    let raw_moved = get_rotate_radian(custom_transform.position.0, cursor_pos.0); // diff from radian 0
    // let (.., current_rotation) = transform.rotation.to_euler(EulerRot::XYZ);
    let current_rotation = custom_transform.rotation.0;
    let mut target_move = raw_moved;

    let moved = {
        // radians to move from current rotation
        let mut moved_from_current = (raw_moved - current_rotation).normalize();

        // -- adjust for reversed ---
        if moved_from_current.abs() > MINIMUM_REVERSE && state == BoatState::FreeDir {
            // reversing
            custom_transform.reversed = true;
            moved_from_current = moved_from_current.flip();
            target_move = target_move.flip()
        } else if moved_from_current.abs() <= MINIMUM_REVERSE
            && custom_transform.reversed
            && state == BoatState::FreeDir
        {
            // going forwards
            custom_transform.reversed = false;
        } else if custom_transform.reversed {
            // unable to go forward, haven't released key yet
            moved_from_current = moved_from_current.flip();
            target_move = target_move.flip()
        }

        moved_from_current
    };

    // turning degree bigger than maximum
    let max_turn = boat.max_turn().to_radians();

    if moved.abs() > max_turn {
        if moved > 0.0 {
            custom_transform.rotate_local_z(max_turn.to_radian_unchecked());
        } else if moved < 0.0 {
            custom_transform.rotate_local_z(-max_turn.to_radian_unchecked());
        }
    } else if moved != 0.0 {
        // normal
        custom_transform.rotate_local_z(moved.to_radian_unchecked());
    }

    target_rotation.0 = Some(target_move);
}

/// handle moving (manipulate [`CustomTransform`]'s [`Speed`])
fn move_boat(
    query: Single<(&mut CustomTransform, &mut TargetSpeed, &Boat), With<Controlled>>,
    cursor_pos: Res<CursorPos>
) {
    let (mut custom_transform, mut target_speed, boat) = query.into_inner();
    let cursor_distance = cursor_pos.0.distance(custom_transform.position.0);
    let max_speed = if custom_transform.reversed {
        - boat.rev_max_speed().get_raw()
    } else {
        boat.max_speed().get_raw()
    };

    let speed = calculate_from_proportion(
        cursor_distance,
        add_circle_hud(boat.sprite_size().x / 2.0),
        max_speed,
        boat.sprite_size().x / 2.0,
    );

    target_speed.0 = Speed::from_raw(speed);

    // adjust for acceleration
    let speed_diff = speed - custom_transform.speed.get_raw();
    let acceleration = boat.acceleration();

    if speed_diff > acceleration.get_raw() {
        // accelerating too much forwards
        custom_transform.speed.add_raw(acceleration.get_raw());
    } else if speed_diff < -acceleration.get_raw() {
        // accelerating too much backwards
        custom_transform.speed.subtract_raw(acceleration.get_raw());
    }
    // not exceeding acceleration
    else if speed_diff.abs() > 0.1 {
        custom_transform.speed.overwrite_with_raw(speed);
    }
}

// send messages to server
fn update_transform(
    query: Single<
        (
            &Transform,
            &mut CustomTransform,
            &Children,
            &Sprite,
            &mut OutOfBound,
        ),
        (With<Boat>, With<Controlled>)
    >,
    mut circle_huds: Single<(Entity, &mut CircleHud)>,
    world_size: Single<&WorldSize>,
    mut sender: Single<&mut MessageSender<PlayerAction>>,
    client_id: Single<&LocalId>
) {
    let (transform, mut custom, children, sprite, mut out_of_bound) = query.into_inner();

    let Some(custom_size) = sprite.custom_size else {
        return;
    };

    let mut translation = custom.position.to_vec3(transform.translation.z);

    translation += move_with_rotation(custom.rotation.to_quat(), custom.speed.get_raw()); // ignores frame lagging temporary

    custom.position.0 = translation.xy();

    if out_of_bounds(
        &world_size,
        MkRect {
            center: custom.position.0,
            dimensions: custom_size.into(),
        },
        custom.rotation.to_quat(),
    ) {
        custom.position.0 = transform.translation.truncate();  // changes have no effect
        out_of_bound.0 = true;
        return;
    } else if out_of_bound.0 {
        out_of_bound.0 = false;
    }

    sender.send::<SendToServer>(PlayerAction {
        action: ActionType::Rotate(custom.rotation),
        client: client_id.to_bits()
    });
    // TODO more info?
    sender.send::<SendToServer>(PlayerAction {
        action: ActionType::Move(custom.position.0),
        client: client_id.to_bits()
    });

    // let target = Transform {
    //     translation,
    //     rotation: custom.rotation.to_quat(),
    //     scale: Vec3::ONE,
    // };
    // *transform = target;

    // ^^^^^^^^^^^^^^ only do these if server says so through replication

    // TODO seperate function
    for &child in children {
        if child == circle_huds.0 {
            circle_huds.1.center = translation.xy();
            break;
        }
    }

    info!("Speed: {} knots", custom.speed.get_knots());
}

/// remember the last move angle and rotate toward it when button not pressed
fn boat_to_target(
    boat: Single<(
        &Transform,
        &mut CustomTransform,
        &TargetRotation,
        &TargetSpeed,
        &Boat,
    ), With<Controlled>>,
) {
    let (transform, mut custom_transform, target_rotation, target_speed, boat) = boat.into_inner();
    
    // ------ rotation
    let Some(target_rotation) = target_rotation.0 else {
        return;
    };

    let (.., current_rotation) = transform.rotation.to_euler(EulerRot::XYZ);

    let moved = (target_rotation - current_rotation).normalize();

    let ship_max_turn = boat.max_turn().to_radians();
    if moved.abs() > ship_max_turn {
        if moved > 0.0 {
            custom_transform.rotate_local_z(ship_max_turn.to_radian_unchecked());
        } else if moved < 0.0 {
            custom_transform.rotate_local_z(-ship_max_turn.to_radian_unchecked());
        }
    } else {
        custom_transform.rotate_local_z(moved.to_radian_unchecked());
    }
    // ------ speed
    let speed_diff = target_speed.get_raw() - custom_transform.speed.get_raw();
    let acceleration = boat.acceleration();
    if speed_diff > acceleration.get_raw() {
        custom_transform.speed.add_raw(acceleration.get_raw());
    } else if speed_diff < -acceleration.get_raw() {
        custom_transform.speed.subtract_raw(acceleration.get_raw());
    } else {
        custom_transform
            .speed
            .overwrite_with_raw(target_speed.get_raw());
    }

}

fn move_camera(
    mut camera: Single<&mut Transform, With<MainCamera>>,
    ship: Single<&Transform, (With<Boat>, With<Controlled>)>,
) {
    if ship.translation.xy() != camera.translation.xy() {
        camera.translation = ship.translation.with_z(WATER_SURFACE);
    }
}

fn demo_update_transform(
    mut query: Query<(&PlayerPos, &mut Transform), Changed<PlayerPos>>
) {
    for (pos, mut transform) in query.iter_mut() {
        transform.translation = pos.0.extend(0.0);
    }
}
fn demo_spawn_sprite(
    trigger: On<Add, (PlayerPos, Predicted)>,
    player_pos: Query<&PlayerPos, With<Predicted>>,
    asset_server: Res<AssetServer>,
    mut commands: Commands
) {
    info!("New predicted playerpos");
    let Ok(player_pos) = player_pos.get(trigger.entity) else { return; };  // seems to be triggered twice for the same spawning?
    commands
        .get_entity(trigger.entity).unwrap()
        .insert((
           Sprite::from_image(asset_server.load("yasen.png")),
           Transform::from_translation(player_pos.0.extend(0.0))
        ));
}
fn spawn_boat(
    trigger: On<Add, MinimalBoat>,
    templates: Query<&MinimalBoat>,
    controlled: Query<(), (With<MinimalBoat>, With<Controlled>)>,

    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let template = templates.get(trigger.entity).unwrap();
    let boat = template.boat;
    let circle_hud_radius = add_circle_hud(boat.sprite_size().x / 2.0);
    let controls = controlled.get(trigger.entity).is_ok();

    commands
        .get_entity(trigger.entity)
        .unwrap()
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
                translation: template.position.extend(WATER_SURFACE),
                rotation: template.rotation.to_quat(),
                ..default()
            },
            custom_transform: CustomTransform {
                position: template.position.into(),
                rotation: template.rotation.to_radian_unchecked(),
                ..default()
            },
            ..BoatBundle::default()
        })
        .with_children(|parent| {
            let mut circle_hud = parent.spawn((
                MeshBundle {
                    mesh: Mesh2d(meshes.add(Circle::new(circle_hud_radius).to_ring(3.0))),
                    materials: MeshMaterial2d(materials.add(ColorMaterial::from_color(GRAY))),
                },
                Transform::from_xyz(0.0, 0.0, CIRCLE_HUD),
                CircleHud {
                    radius: circle_hud_radius,
                    center: template.position,
                },
            ));

            // hide circle hud if not client's ship
            if !controls {
                circle_hud.insert(Visibility::Hidden);
            }
        });
    // insert circle hud if controlls
}

#[derive(Bundle, Debug, Clone)]
pub struct BoatBundle {
    /// tranform to update in seperate system
    transform: Transform,
    /// ship's sprite
    sprite: Sprite,
    /// whether reversed, speed etc
    custom_transform: CustomTransform,
    /// where the user's mouse was facing
    mouse_target: TargetRotation,
    /// the target speed of the Boat
    target_speed: TargetSpeed,
    out_of_bound: OutOfBound,
    weapon_counter: WeaponCounter,
    boat: Boat,
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
    info!(
        "Client disconnected because: {}",
        disconnected.reason.as_ref().map(|s| s.as_str()).unwrap_or("None")
    )
}

fn on_remove_disconnect(_: On<Remove, Disconnected>) {
    info!("Client re-connected")
}