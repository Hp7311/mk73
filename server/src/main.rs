use std::time::Duration;

use bevy::{
    color::palettes::css::{GRAY, TEAL},
    prelude::*,
};
use common::{
    CIRCLE_HUD, LOCAL_SERVER_ADDR, PROTOCOL_ID,
    boat::Boat,
    primitives::*,
    protocol::{MinimalBoat, PlayerAction, ProtocolPlugin, SendToClient},
    util::add_circle_hud,
    world::{Background, WorldPlugin},
};
use lightyear::websocket::server::Identity;
use lightyear::{
    netcode::NetcodeServer,
    prelude::{
        server::{ClientOf, NetcodeConfig, ServerConfig, ServerPlugins, Start, WebSocketServerIo},
        *,
    },
};

fn main() {
    App::new()
        // headless plugins
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: None,
                    exit_condition: bevy::window::ExitCondition::DontExit,
                    ..default()
                })
        )
        .insert_resource(ClearColor(TEAL.into()))
        .add_plugins(ServerPlugins::default())
        .add_plugins(ProtocolPlugin)
        // init
        .add_plugins(WorldPlugin)
        .add_systems(Startup, setup)
        // handle client req
        .add_observer(handle_new_client)
        .add_observer(handle_connected_client)
        .run();
}

/// starts the server
fn setup(mut commands: Commands) {
    let server = commands
        .spawn((
            NetcodeServer::new(NetcodeConfig::default().with_protocol_id(PROTOCOL_ID).with_client_timeout_secs(30)),
            LocalAddr(LOCAL_SERVER_ADDR),
            WebSocketServerIo {
                config: ServerConfig::builder()
                    .with_bind_address(LOCAL_SERVER_ADDR)
                    .with_identity(from_pem_file("../cert/cert_127.pem", "../cert/key_127.pem")), // _127 include 127.0.0.1 instead of only localhost
            },
        ))
        .id();

    commands.trigger(Start { entity: server });

    // commands.spawn(MessageReceiver::<PlayerAction>::default());  // FIXME use replication instead of manual message syncing
}

/// connecting client
fn handle_new_client(connecting_client: On<Add, LinkOf>, mut commands: Commands) {
    commands
        .entity(connecting_client.entity)
        .insert(ReplicationSender::new(
            Duration::from_millis(100),
            SendUpdatesMode::SinceLastAck,
            false,
        ));
}

/// connected client. setup sprites etc
fn handle_connected_client(
    connected_client: On<Add, Connected>,
    clients: Query<&RemoteId, With<ClientOf>>,
    mut commands: Commands,
) {
    let entity = connected_client.entity;
    let Ok(RemoteId(_client_id)) = clients.get(entity) else {
        warn!("Didn't find the connected client in Query<&RemoteId, With<ClientOf>");
        return;
    };

    let boat = Boat::Yasen;
    let position = vec2(
        rand::random_range(-200.0..200.0),
        rand::random_range(-200.0..200.0),
    );  // TODO

    commands.spawn((
        MinimalBoat {  // TODO consider making an observer that updates CustomTransform and Transform if MinimalBoat changes (potentially through Replication)
            position,
            boat,
            rotation: 90.0_f32.to_radians(),
        },
        Replicate::to_clients(NetworkTarget::All),
        ControlledBy {
            owner: entity,
            lifetime: Lifetime::SessionBased,
        }
    ));
}

use std::fs;
use std::path::Path;

fn from_pem_file(cert_path: impl AsRef<Path>, key_path: impl AsRef<Path>) -> Identity {
    let cert_chain_bytes = fs::read(cert_path).unwrap();
    let key_bytes = fs::read(key_path).unwrap();

    let mut cert_reader = std::io::Cursor::new(cert_chain_bytes);
    let certs = rustls_pemfile::certs(&mut cert_reader)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    let mut key_reader = std::io::Cursor::new(key_bytes);
    let key = rustls_pemfile::private_key(&mut key_reader)
        .unwrap()
        .unwrap();

    Identity::new(certs, key)
}
