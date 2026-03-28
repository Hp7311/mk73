// use aeronet_webtransport::client::WebTransportClient;
// use aeronet_webtransport::wtransport::ClientConfig;
use bevy::color::palettes::css::TEAL;
use bevy::prelude::*;
use common::protocol::{ProtocolPlugin, SpawnSprite};
use common::{CLIENT_ADDR, PROTOCOL_ID, SERVER_ADDR};
use lightyear::link::LinkConditioner;
use lightyear::netcode::auth::Authentication;
use lightyear::netcode::{Key, NetcodeClient};
use lightyear::prelude::client::{ClientConfig, NetcodeConfig, WebSocketClientIo, WebSocketScheme};
use lightyear::prelude::{client::ClientPlugins, *};
use lightyear::websocket::client::WebSocketTarget;

// TODO client disconnects on switching tabs


fn main() {
    App::new()
        .add_plugins(
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
        .insert_resource(ClearColor(TEAL.into()))
        .add_systems(Startup, connect_client)
        .add_systems(Update, spawn_sprite)

        .add_systems(Update,dbg_client_disconnected)
        .run();
}

fn connect_client(mut commands: Commands) {
    commands.spawn(Camera2d);

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
            #[cfg(debug_assertions)]  // https://github.com/cBournhonesque/lightyear/blob/main/examples/common/src/client.rs#L102
            config: ClientConfig::default(),
            #[cfg(debug_assertions)]
            target: WebSocketTarget::Addr(WebSocketScheme::Plain)
        },
        MessageReceiver::<SpawnSprite>::default()
    ))
    .id();

    commands.trigger(Connect { entity: client });
}

/// spawns sprite when received command from server
fn spawn_sprite(
    mut recevier: Single<&mut MessageReceiver<SpawnSprite>>,
    asset_server: Res<AssetServer>,
    id: Single<Entity, (With<Client>, Without<Sprite>)>,
    mut commands: Commands,
) {
    for msg in recevier.receive() {
        let yasen = commands.entity(*id)
            .insert((
                Sprite::from_image(asset_server.load(msg.sprite_name)),
                Transform::from_translation(msg.position.extend(0.0))
            ))
            .id();
        info!("Spawned yasen: {yasen}")
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
