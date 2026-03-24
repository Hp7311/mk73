// use aeronet_webtransport::client::WebTransportClient;
// use aeronet_webtransport::wtransport::ClientConfig;
use bevy::color::palettes::css::TEAL;
use bevy::{camera_controller::pan_camera::PanCameraPlugin, prelude::*};
use common::protocol::{ProtocolPlugin, SpawnYasen};
use common::{BoatPlugin, CLIENT_ADDR, OilRigPlugin, SERVER_ADDR, ShadersPlugin, WeaponPlugin, WorldPlugin};
use lightyear::netcode::{Key, NetcodeClient};
use lightyear::prelude::client::{NetcodeConfig, WebTransportClientIo};
use lightyear::prelude::{client::ClientPlugins, *};
use lightyear::netcode::auth::Authentication;
use lightyear::webtransport::client::WebTransportClientPlugin;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                // .set(WindowPlugin {
                //     // primary_window: Some(Window {
                //     //     canvas: Some("#bevy_canvas".to_owned()),
                //     //     fit_canvas_to_parent: true,
                //     //     ..default()
                //     // }),
                //     ..default()
                // })
                // .set(AssetPlugin {
                //     meta_check: bevy::asset::AssetMetaCheck::Never,
                //     ..default()
                // }),
        )
        .add_plugins(ClientPlugins::default())
        .add_plugins(ProtocolPlugin)
        .insert_resource(ClearColor(TEAL.into()))
        .add_systems(Startup, (connect_client, connect).chain())
        .add_systems(Update, spawn_yasen)
        .run();
}

fn connect_client(mut commands: Commands) {
    let auth = Authentication::Manual {
        server_addr: SERVER_ADDR,
        client_id: rand::random_range(0..10),
        private_key: Key::default(),
        protocol_id: 0,
    };
    
    commands
        .spawn((
            Client::default(),
            LocalAddr(CLIENT_ADDR),
            PeerAddr(SERVER_ADDR),
            Link::new(None),
            ReplicationReceiver::default(),
            NetcodeClient::new(auth, NetcodeConfig {
                client_timeout_secs: 3,
                ..default()
            }).unwrap(),
            WebTransportClientIo {
                #[cfg(not(target_family = "wasm"))]
                // certificate_digest: "74d079403eeea48ab063bc93c34c3d3045271ce3c673551f00385b19ef2d9245".to_owned()
                certificate_digest: "".to_owned()
            }
        ));
}

fn connect(mut commands: Commands, client: Single<Entity, With<Client>>) {
    commands.trigger(Connect { entity: client.into_inner() });
    info!("Client connect event triggered")
}
/// spawns sprite when received command from server
fn spawn_yasen(
    asset_server: Res<AssetServer>,
    ids: Query<Entity, (With<SpawnYasen>, Without<Spawned>)>,
    mut commands: Commands
) {
    for id in ids {
        let id = commands.entity(id)
            .insert(Sprite::from_image(asset_server.load("yasen.png")))
            .insert(Spawned)
            .id();
        info!("Spawned yasen: {id}")
    }
}

/// marker to avoid too many sprites
#[derive(Component)]
struct Spawned;