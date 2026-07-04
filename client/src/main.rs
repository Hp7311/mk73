#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

// FIXING server unresponsive after few minutes
mod input;
mod dive;
mod weapon;
mod boat;
mod oil_rig;
mod ui;
mod asset;

use std::env::current_dir;
use std::sync::LazyLock;
use std::time::Duration;

use bevy::camera_controller::pan_camera::{MousePanSettings, PanCamera, PanCameraPlugin};
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use common::util::BlockInput;
use common::{TCP_ADDR, UpgradePlugin};
use common::protocol::ZIndexUpdate;
use common::{
    Boat, CLIENT_ADDR, MainCamera, MovementPlugin, PROTOCOL_ID, SERVER_ADDR, WorldPlugin,
    protocol::{Move, ProtocolPlugin, Rotate},
    primitives::ZIndex
};

use lightyear::netcode::{auth::Authentication, Key, NetcodeClient};
use lightyear::prelude::{
    input::native::{ActionState, InputMarker},
    client::{
        ClientPlugins, NetcodeConfig
    },
    *
};
use lightyear::webtransport::client::WebTransportClientIo;
use crate::asset::AssetPreloadPlugin;
use crate::boat::BoatPlugin;
use crate::dive::DivingPlugin;
use crate::input::InputBufferPlugin;
use crate::oil_rig::OilRigPlugin;
use crate::ui::UiPlugin;
use crate::weapon::WeaponPlugin;

// note that web builds are noticably laggier than native builds

#[cfg(all(not(target_family = "wasm"), not(debug_assertions)))]
compile_error! {"Should compile by trunk serve on production"}


const DEFAULT_MAX_ZOOM: f32 = 2.0;

const TIME_TO_LAUNCH_WEAPON: Duration = Duration::from_millis(200);

fn main() -> AppExit {
    #[cfg(target_family = "wasm")]
    console_error_panic_hook::set_once();
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
            })
            .set(ImagePlugin::default_nearest()),
    )
    .add_plugins(ClientPlugins::default())
    .add_plugins(ProtocolPlugin)
    .add_plugins(PanCameraPlugin)
    .insert_resource(ClearColor(bevy::color::palettes::css::TEAL.into()))
        
    // plugins
    .init_state::<BoatState>()
    .add_plugins(WorldPlugin)
    .add_plugins(OilRigPlugin)
    .add_plugins(BoatPlugin)
    .add_plugins(InputBufferPlugin)
    .add_plugins(MovementPlugin { move_weapon: true })
    .add_plugins(DivingPlugin)
    .add_plugins(WeaponPlugin)
    .add_plugins(UiPlugin) 
    .add_plugins(UpgradePlugin)

    // init
    .add_plugins(AssetPreloadPlugin)
    .add_systems(Startup, setup)
    .add_observer(on_added_actionstate::<Rotate>)
    .add_observer(on_added_actionstate::<Move>)
    .add_observer(on_added_actionstate::<ZIndexUpdate>)

    .add_systems(FixedUpdate, update_state)
    .add_systems(Update, move_camera)

    .add_systems(FixedUpdate, sync_z_index)
        
    .add_plugins(EguiPlugin::default())
    .add_plugins(WorldInspectorPlugin::default())

    .add_observer(on_disconnect)
    .add_observer(on_remove_disconnect);

    app.run()
}

static DIGEST: LazyLock<String> = LazyLock::new(|| {
    if current_dir().unwrap().ends_with("client") {
        std::fs::read_to_string("../cert/digest.txt").unwrap()   
    } else if current_dir().unwrap().ends_with("mk73") {
        std::fs::read_to_string("cert/digest.txt").unwrap()
    } else {
        panic!("Must run from . or ./client, current_dir: {:?}", current_dir().unwrap())
    }
});

fn setup(mut commands: Commands) {
    // let client_id = rand::random_range(0..100);
    let rt = tokio::runtime::Runtime::new().unwrap();
    let client_id = rt.block_on(async {
        let resp = reqwest::get(format!("http://{}/client_id", TCP_ADDR))
            .await.unwrap()
            .bytes()
            .await.unwrap();

        u64::from_be_bytes(resp.as_ref().try_into().unwrap())
    });
    let auth = Authentication::Manual {
        server_addr: SERVER_ADDR,
        client_id,
        private_key: Key::default(),
        protocol_id: PROTOCOL_ID,
    };
    let netcode_config = NetcodeConfig {
        // client_timeout_secs: -1,
        num_disconnect_packets: 50,
        client_timeout_secs: 3,
        token_expire_secs: -1,
        ..default()
    };


    let client = commands
        .spawn((
            Client::default(),
            LocalAddr(CLIENT_ADDR),
            PeerAddr(SERVER_ADDR),
            Link::default(),
            NetcodeClient::new(auth, netcode_config).unwrap(),
            WebTransportClientIo {
                certificate_digest: DIGEST.clone()
            },
            ReplicationReceiver,
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
            mouse_pan_settings: MousePanSettings {
                enabled: false,
                button: MouseButton::Left,
            },
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

fn update_state(
    current_state: Res<State<BoatState>>,
    mut setter: ResMut<NextState<BoatState>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    block_input: Res<BlockInput>,
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
            if mouse_button.just_pressed(MouseButton::Left) && !block_input.0 {
                setter.set(BoatState::FiringWeapon(Duration::ZERO));
            }
        }
        BoatState::FiringWeapon(elapsed) => {
            let duration = *elapsed + time.delta();

            
            // TODO a lot of misses and moving when firing weapon
            if duration > TIME_TO_LAUNCH_WEAPON {
                setter.set(BoatState::Moving { locked: false });
            } else if mouse_button.just_released(MouseButton::Left) {
                commands.trigger(FiresWeapon);  /* 
2026-06-30T16:48:58.481663Z DEBUG client::weapon: Firing BrahMos (3 left)
2026-06-30T16:48:58.481793Z DEBUG client::weapon: Firing BrahMos (2 left) ????? */ 
                setter.set(BoatState::Released);
            } else {
                setter.set(BoatState::FiringWeapon(duration));
            }
        }
    }
}

/// using hack to achieve system to be only triggered when both
/// [`ActionState<T>`] and [`Controlled`] added
// #[deny(unused)]     
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

/// ejected by [`update_state`]
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

fn on_disconnect(trigger: On<Add, Disconnected>, query: Query<&Disconnected>) {
    let disconnected = query.get(trigger.entity).unwrap();
    warn!("Client disconnected because: {:?}", disconnected.reason)
}

fn on_remove_disconnect(_: On<Remove, Disconnected>) {
    info!("Client re-connected")
}