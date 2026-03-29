use std::f32::consts::PI;
use std::time::Duration;

use bevy::color::palettes::css::{GRAY, TEAL};
use bevy::prelude::*;
use bevy::camera_controller::pan_camera::{PanCamera, PanCameraPlugin};

use common::boat::{Boat, BoatData, SubKind};
use common::collision::out_of_bounds;
use common::primitives::{CursorPos, CustomTransform, DecimalPoint, FlipRadian, MeshBundle, MkRect, NormalizeRadian, OutOfBound, Radian, Speed, TargetRotation, TargetSpeed, ToRadian};
use common::protocol::{ProtocolPlugin, SendToServer,  SpawnShip};
use common::util::{add_circle_hud, calculate_from_proportion, get_rotate_radian, move_with_rotation};
use common::weapon::Weapon;
use common::world::{WorldPlugin, WorldSize};
use common::{CIRCLE_HUD, CLIENT_ADDR, MainCamera, PROTOCOL_ID, SERVER_ADDR, WATER_SURFACE};

use lightyear::link::LinkConditioner;
use lightyear::netcode::auth::Authentication;
use lightyear::netcode::{Key, NetcodeClient};
use lightyear::prelude::client::{ClientConfig, NetcodeConfig, WebSocketClientIo, WebSocketScheme};
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
        .init_state::<BoatState>()
        .init_resource::<CursorPos>()
        .add_plugins(WorldPlugin)

        // init
        .add_systems(Startup, setup)
        .add_systems(Update, spawn_boat)

        // move
        .add_systems(Update, update_state)
        .add_systems(Update, move_camera)
        .add_systems(
            Update,
            (
                (rotate_ship, move_ship).run_if(|state: Res<State<BoatState>>| {
                    matches!(state.get(), BoatState::FreeDir | BoatState::LockedDir)
                }),
                // TODO ship_to_target.run_if(in_state(BoatState::Released)),
                update_transform,
            )
                .chain(),
        );

    add_debug_systems!(&mut app, dbg_client_disconnected);

    app.run();
}

fn setup(mut commands: Commands) {
    let auth = Authentication::Manual {
        server_addr: SERVER_ADDR,
        client_id: rand::random_range(0..100),
        private_key: Key::default(),
        protocol_id: PROTOCOL_ID,
    };

    let client = commands.spawn((
        Client::default(),
        LocalAddr(CLIENT_ADDR),
        PeerAddr(SERVER_ADDR),
        Link::new(Some(LinkConditioner::new(LinkConditionerConfig::average_condition()))),
        ReplicationReceiver::default(),
        NetcodeClient::new(auth, NetcodeConfig::default()).unwrap(),
        WebSocketClientIo {
            // https://github.com/cBournhonesque/lightyear/blob/main/examples/common/src/client.rs#L102
            config: ClientConfig::default(),
            target: WebSocketTarget::Addr(WebSocketScheme::Secure)
        }
    )).id();

    commands.trigger(Connect { entity: client });

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
    time: Res<Time>
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

/// handle rotation
fn rotate_ship(
    query: Single<(&Transform, &mut CustomTransform, &mut TargetRotation, &Boat)>,
    state: Res<State<BoatState>>,
    cursor_pos: Res<CursorPos>,
) {
    let state = *state.get();

    let (transform, mut custom_transform, mut target_rotation, boat) = query.into_inner();
    let raw_moved = get_rotate_radian(transform.translation.xy(), cursor_pos.0); // diff from radian 0
    let (.., current_rotation) = transform.rotation.to_euler(EulerRot::XYZ);
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
    if moved.abs() > boat.data.max_turn().to_radians() {
        let ship_max_turn = boat.data.max_turn().to_radians();
        if moved > 0.0 {
            custom_transform.rotate_local_z(ship_max_turn.to_radian_unchecked());
        } else if moved < 0.0 {
            custom_transform.rotate_local_z(-ship_max_turn.to_radian_unchecked());
        }
    } else {
        // normal
        custom_transform.rotate_local_z(moved.to_radian_unchecked());
    }

    target_rotation.0 = Some(target_move);
}

/// handle moving
fn move_ship(
    query: Single<(&Transform, &mut CustomTransform, &mut TargetSpeed, &Boat)>,
    cursor_pos: Res<CursorPos>,
) {
    let (transform, mut custom_transform, mut target_speed, boat) = query.into_inner();
    let cursor_distance = cursor_pos.0.distance(transform.translation.xy());
    let max_speed = if custom_transform.reversed {
        -boat.data.rev_max_speed().get_raw()
    } else {
        boat.data.max_speed().get_raw()
    };

    let speed = calculate_from_proportion(
        cursor_distance,
        add_circle_hud(boat.data.sprite_size().x / 2.0),
        max_speed,
        boat.data.sprite_size().x / 2.0,
    );

    target_speed.0 = Speed::from_raw(speed);

    // adjust for acceleration
    let speed_diff = speed - custom_transform.speed.get_raw();
    let acceleration = boat.data.acceleration();

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

fn update_transform(
    query: Single<
        (
            &mut Transform,
            &mut CustomTransform,
            &Children,
            &Sprite,
            &mut OutOfBound,
        ),
        With<Boat>,
    >,
    mut circle_huds: Query<&mut CircleHud>,
    world_size: Single<&WorldSize>,
) {
    let (mut transform, mut custom, children, sprite, mut out_of_bound) = query.into_inner();

    let Some(custom_size) = sprite.custom_size else {
        return;
    };

    let mut translation = custom.position.to_vec3(transform.translation.z);

    translation += move_with_rotation(transform.rotation, custom.speed.get_raw()); // ignores frame lagging temporary

    if out_of_bounds(
        &world_size,
        MkRect {
            center: translation.xy(),
            dimensions: custom_size.into(),
        },
        custom.rotation.to_quat(),
    ) {
        custom.position.0 = transform.translation.truncate();
        out_of_bound.0 = true;
        return;
    } else if out_of_bound.0 {
        out_of_bound.0 = false;
    }

    let target = Transform {
        translation,
        rotation: custom.rotation.to_quat(),
        scale: Vec3::ONE,
    };
    *transform = target;

    // sync position
    custom.position = translation.xy().into();

    for child in children {
        if let Ok(mut hud) = circle_huds.get_mut(*child) {
            hud.center = translation.xy();
            break;
        }
    }

    // println!("Speed: {} knots", custom.speed.get_knots());
}


fn move_camera(
    mut camera: Single<&mut Transform, With<MainCamera>>,
    ship_pos: Query<&CustomTransform, With<Boat>>,
) {
    // currently ignores possibility of multiple ships
    let Some(ship) = ship_pos.iter().last() else {
        return;
    };

    if ship.position.0 != camera.translation.xy() {
        camera.translation = ship.position.0.extend(WATER_SURFACE);
    }
}

/// spawns boat bundle which is seperated from [`Client`] entity when received command from server
fn spawn_boat(
    mut recevier: Single<&mut MessageReceiver<SpawnShip>>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    boat_sprite: Query<&Sprite, With<Boat>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>
) {
    for msg in recevier.receive() {
        assert_eq!(boat_sprite.iter().len(), 0); // shouldn't spawn two `Boat`s of the same client
        let boat = msg.boat;
        let circle_hud_radius = add_circle_hud(boat.data.sprite_size().x / 2.0);
        let position = msg.position;

        commands
            .spawn(BoatBundle {
                boat: msg.boat,
                weapon_counter: WeaponCounter {
                    aval_weapons: boat.data.get_armanents(),
                    selected_weapon: boat.data.default_weapon()
                },
                sprite: Sprite {
                    image: asset_server.load(boat.data.file_name()),
                    custom_size: Some(boat.data.sprite_size()),
                    ..default()
                },
                transform: Transform::from_translation(msg.position.extend(0.0)),
                custom_transform: CustomTransform {
                    position: msg.position.into(),
                    rotation: Radian::from_deg(90.0),
                    ..default()
                },
                ..BoatBundle::default()
            })
            .with_children(|parent| {
                parent.spawn((
                    MeshBundle {
                        mesh: Mesh2d(meshes.add(Circle::new(circle_hud_radius).to_ring(3.0))),
                        materials: MeshMaterial2d(materials.add(ColorMaterial::from_color(GRAY))),
                    },
                    Transform::from_xyz(0.0, 0.0, CIRCLE_HUD),
                    CircleHud {
                        radius: circle_hud_radius,
                        center: position,
                    },
                ));
            });
        info!("Spawned boat: {:?}", msg.boat)
    }
}

fn dbg_client_disconnected(dis: Query<&Disconnected>) {
    for d in dis {
        info!(
            "Client disconnected because: {}",
            d.reason.as_ref().unwrap_or(&"None".to_owned())
        )
    }
}

#[derive(Bundle, Debug, Clone)]
pub(crate) struct BoatBundle {
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
            boat: Boat {
                data: BoatData::Yasen,
                subkind: SubKind::SurfaceShip
            }
        }
    }
}

/// helper struct for accessing the [`Boat`]'s circle HUD
#[derive(Debug, Component, Copy, Clone)]
pub(crate) struct CircleHud {
    pub radius: f32,
    pub center: Vec2,
}

impl CircleHud {
    /// whether `point` is in the Circle HUD
    pub(crate) fn contains(&self, point: Vec2) -> bool {
        point.distance_squared(self.center) < self.radius.powi(2)
    }
    /// whether a point is at HUD's center
    ///
    /// adjusted for decimal-point precision
    pub(crate) fn at_center(&self, point: Vec2, decimal_point: DecimalPoint) -> bool {
        let x_diff = (point.x - self.center.x).abs();
        let y_diff = (point.y - self.center.y).abs();

        x_diff < decimal_point.to_f32() && y_diff < decimal_point.to_f32()
    }
}

#[derive(Debug, Component, Clone)]
pub(crate) struct WeaponCounter {
    aval_weapons: Vec<Weapon>,       // FIXME and maybe HashMap<Weapon, u16>
    selected_weapon: Option<Weapon>, // potential terry fox
}

/// adds the specified systems to the [`Update`] schedule in the app
/// ### Example
/// ```rust,norun
/// fn example_debug_system() {
///     println!("This is a system that runs on Update!")
/// }
/// app.add_systems(Update, example_debug_system);
/// // is equivalent to
/// add_debug_systems(&mut app, example_debug_system);
/// ```
#[macro_export]
macro_rules! add_debug_systems {
    ( $app:expr, $( $system:expr ),+ ) => {
        $app.add_systems(Update, $(
            $system
        )+);
    };
}