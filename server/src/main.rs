use std::time::Duration;

use bevy::{color::palettes::css::TEAL, log::LogPlugin, prelude::*};
use common::{
    LOCAL_SERVER_ADDR, PROTOCOL_ID, boat, protocol::{ProtocolPlugin, SendToClient, SpawnShip}
};
#[cfg(debug_assertions)]
use lightyear::websocket::server::Identity;
use lightyear::{
    netcode::NetcodeServer,
    prelude::{
        server::{ClientOf, NetcodeConfig, ServerConfig, ServerPlugins, Start, WebSocketServerIo},
        *,
    }
};

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins(LogPlugin::default())
        .insert_resource(ClearColor(TEAL.into()))
        .add_plugins(ServerPlugins::default())
        .add_plugins(ProtocolPlugin)

        .add_systems(Startup, setup)

        .add_observer(handle_new_client)
        .add_observer(handle_connected_client)
        .run();
}

/// starts the server
fn setup(mut commands: Commands) {
    let server = commands.spawn((
        NetcodeServer::new(NetcodeConfig::default().with_protocol_id(PROTOCOL_ID)),
        LocalAddr(LOCAL_SERVER_ADDR),
        WebSocketServerIo {
            #[cfg(debug_assertions)]
            config: ServerConfig::builder()
                .with_bind_address(LOCAL_SERVER_ADDR)
                .with_identity(from_pem_file("../cert/cert_127.pem", "../cert/key_127.pem"))  // _127 include 127.0.0.1 instead of only localhost
        }
    )).id();

    commands.trigger(Start { entity: server });
}

/// connecting client
fn handle_new_client(connecting_client: On<Add, LinkOf>, mut commands: Commands) {
    commands.entity(connecting_client.entity).insert((
        ReplicationSender::new(Duration::from_secs(1), SendUpdatesMode::SinceLastAck, false),
        MessageReceiver::<SpawnShip>::default(),  // enables receiving message
    ));
}

/// connected client. setup sprites etc
fn handle_connected_client(
    connected_client: On<Add, Connected>,
    query: Query<&RemoteId, With<ClientOf>>,
    server: Single<&Server>,
    mut sender: ServerMultiMessageSender
) {
    let Ok(RemoteId(client_id)) = query.get(connected_client.entity) else {
        info!("Didn't find the connected client in Query<&RemoteId, With<ClientOf>");
        return;
    };
    
    sender
        .send::<_, SendToClient>(
            &SpawnShip {
                position: vec2(30.0, 30.0),
                boat: boat::Boat {
                    data: boat::BoatData::Yasen,
                    subkind: boat::SubKind::Submarine
                }
            },
            *server,
            &NetworkTarget::Only(vec![*client_id])
        )
        .expect("Failed to send spawn ship");

    info!("Sent create yasen to clients via `SendToClient");
}

use std::path::Path;
use std::fs;

fn from_pem_file(
    cert_path: impl AsRef<Path>, 
    key_path: impl AsRef<Path>
) -> Identity {
    let cert_chain_bytes = fs::read(cert_path).unwrap();
    let key_bytes = fs::read(key_path).unwrap();

    let mut cert_reader = std::io::Cursor::new(cert_chain_bytes);
    let certs = rustls_pemfile::certs(&mut cert_reader)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    let mut key_reader = std::io::Cursor::new(key_bytes);
    let key = rustls_pemfile::private_key(&mut key_reader).unwrap().unwrap();

    // 4. Construct the Lightyear Identity
    Identity::new(certs, key)
}
