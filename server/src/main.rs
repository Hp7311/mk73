use std::{collections::HashMap, time::Duration};

use bevy::{
    color::palettes::css::{GRAY, TEAL}, diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin}, prelude::*
};
use common::{
    CIRCLE_HUD, LOCAL_SERVER_ADDR, PROTOCOL_ID,
    boat::Boat,
    primitives::*,
    protocol::{ActionType, DbgClientInput, MinimalBoat, PlayerAction, PlayerPos, ProtocolPlugin, SendToClient},
    util::add_circle_hud,
    world::{Background, WorldPlugin},
};
use lightyear::{prelude::input::native::ActionState, websocket::server::Identity};
use lightyear::{
    netcode::NetcodeServer,
    prelude::{
        server::{ClientOf, NetcodeConfig, ServerConfig, ServerPlugins, Start, WebSocketServerIo},
        *,
    },
};
use rand::seq::IndexedRandom;

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

        .add_systems(Startup, setup)
        // handle client req
        .add_observer(handle_new_client)
        .add_observer(handle_connected_client)

        // handle client action
        // .add_systems(Update, recv_player_action)
        // .add_systems(Update, dbg_recv_client)
        .add_systems(FixedUpdate, reverse_player_pos)
        .run();
}

// fn dbg_recv_client(
//     mut recv: Single<&mut MessageReceiver<DbgClientAction>>,
//     mut player_pos: Single<&mut PlayerPos>
// ) {
//     for msg in recv.receive() {
//         match msg {
//             DbgClientAction::Move(by) => player_pos.0 += by
//         }
//     }
// }
// fn recv_player_action(
//     mut receiver: Single<&mut MessageReceiver<PlayerAction>>,
//     mut templates: Query<&mut MinimalBoat>,
//     client_map: Single<&ClientMap>
// ) {
//     for PlayerAction { action, client } in receiver.receive() {
//         let entity = client_map.0.get(&client).unwrap();
//         let mut template = templates.get_mut(*entity).unwrap();

//         info!("Accepted: {:?}", action);
//         match action {
//             ActionType::Fire(_) => todo!(),
//             ActionType::Move(target) => {
//                 template.position = target;  // accept all
//             }
//             ActionType::Rotate(rotation) => {
//                 template.rotation = rotation;
//             }
//         }
//     }
// }

/// starts the server
fn setup(mut commands: Commands) {
    let server = commands
        .spawn((
            NetcodeServer::new(NetcodeConfig::default().with_protocol_id(PROTOCOL_ID).with_client_timeout_secs(30)),
            LocalAddr(LOCAL_SERVER_ADDR),
            WebSocketServerIo {
                #[cfg(debug_assertions)]
                config: ServerConfig::builder()
                    .with_bind_address(LOCAL_SERVER_ADDR)
                    .with_no_encryption()
            },
        ))
        .id();

    commands.trigger(Start { entity: server });

    commands.spawn(ClientMap::default());
}

/// identify the main struct storing [`MinimalBoat`] which is replicated and used to accept clients' request
/// 
/// see [`common::protocol::PlayerAction`]
#[derive(Debug, Clone, Component, Default)]
struct ClientMap(HashMap<u64, Entity>);

/// connecting client
fn handle_new_client(connecting_client: On<Add, LinkOf>, mut commands: Commands) {
    commands
        .entity(connecting_client.entity)
        .insert(
            ReplicationSender::new(
                Duration::from_millis(100),
                SendUpdatesMode::SinceLastAck,
                false,
            )
        );
}

fn reverse_player_pos(query: Query<&mut PlayerPos>) {
    for mut player_pos in query {
        let mut possibility = [false; 10].to_vec();
        possibility.push(true);

        if *possibility.choose(&mut rand::rng()).unwrap() {
            player_pos.0 += vec2(1.0, 1.0);
        }
    }
}
/// connected client. setup sprites etc
fn handle_connected_client(
    connected_client: On<Add, Connected>,
    clients: Query<&RemoteId, With<ClientOf>>,
    mut commands: Commands,
    mut client_map: Single<&mut ClientMap>
) {
    let entity = connected_client.entity;  // NOT equal to client id or Client entity in client's world
    let Ok(RemoteId(client_id)) = clients.get(entity) else {
        warn!("Didn't find the connected client in Query<&RemoteId, With<ClientOf>");
        return;
    };

    let boat = Boat::Yasen;
    let position = vec2(
        rand::random_range(-200.0..200.0),
        rand::random_range(-200.0..200.0),
    );  // TODO

    let boat_id = commands.spawn((
        // MinimalBoat {  // TODO consider making an observer that updates CustomTransform and Transform if MinimalBoat changes (potentially through Replication)
        //     position,
        //     boat,
        //     rotation: Radian::from_deg(rand::random_range(-180.0..180.0))
        // },
        PlayerPos(Vec2::default()),
        
        Replicate::to_clients(NetworkTarget::All),
        PredictionTarget::to_clients(NetworkTarget::Single(*client_id)),
        ActionState::<DbgClientInput>::default(),

        ControlledBy {
            owner: entity,
            lifetime: Lifetime::SessionBased,
        }
    )).id();

    
    client_map.0.insert(client_id.to_bits(), boat_id);
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
