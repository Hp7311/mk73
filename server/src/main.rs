use std::{collections::HashMap, time::Duration};

use bevy::{
    color::palettes::css::{GRAY, TEAL}, diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin}, prelude::*
};
use common::{
    CIRCLE_HUD, LOCAL_SERVER_ADDR, PROTOCOL_ID,
    boat::Boat,
    primitives::*,
    protocol::{Move, ProtocolPlugin, Reversed, Rotate, SendToClient},
    util::add_circle_hud,
    world::{Background, WorldPlugin},
};
use lightyear::{input::{native::plugin::InputPlugin, server::{ServerInputConfig, ServerInputPlugin}}, prelude::input::native::ActionState, websocket::server::Identity};
use lightyear::{
    netcode::NetcodeServer,
    prelude::{
        server::{ClientOf, NetcodeConfig, ServerConfig, ServerPlugins, Start, WebSocketServerIo},
        *,
    },
};

fn main() {
    App::new()
        .add_plugins(
            // headless plugins
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
        // .add_systems(Update, dbg_recv_client)
        .add_systems(FixedUpdate, handle_input)
        .run();
}



fn handle_input(rotate: Query<(&mut CustomTransform, &ActionState<Rotate>, &ActionState<Move>)>) {
    // for (mut custom, action) in query {
        // if let ClientInput::Move(move_by) = action.0 {
        //     custom.position.0 += move_by;
        // }
    // }
}

/// starts the server
fn setup(mut commands: Commands) {
    let server = commands
        .spawn((
            NetcodeServer::new(NetcodeConfig::default().with_protocol_id(PROTOCOL_ID)),
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
}

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

// TODO seperate CUstomTransform?

/// connected client. setup sprites etc
fn handle_connected_client(
    connected_client: On<Add, Connected>,
    clients: Query<&RemoteId, With<ClientOf>>,
    mut commands: Commands
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

    commands.spawn((
        CustomTransform {
            position: Position(position),
            ..CustomTransform::default()
        },
        boat,
        
        Replicate::to_clients(NetworkTarget::All),

        PredictionTarget::to_clients(NetworkTarget::Single(*client_id)),
        InterpolationTarget::to_clients(NetworkTarget::AllExceptSingle(*client_id)),

        ActionState::<Rotate>::default(),
        ActionState::<Move>::default(),
        ActionState::<Reversed>::default(),

        ControlledBy {
            owner: entity,
            lifetime: Lifetime::SessionBased,
        }
    ));
}

/// webtransport certificate
fn from_pem_file(cert_path: impl AsRef<std::path::Path>, key_path: impl AsRef<std::path::Path>) -> Identity {
    use std::fs;

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
