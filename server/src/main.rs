use std::time::Duration;

use aeronet_webtransport::{server::{SessionRequest, SessionResponse}};
use bevy::{color::palettes::css::TEAL, prelude::*};
#[cfg(debug_assertions)]
use common::LOCALHOSTS;
use common::{SERVER_ADDR, protocol::{ProtocolPlugin, SpawnYasen}};
use lightyear::{netcode::NetcodeServer, prelude::{server::{ClientOf, NetcodeConfig, ServerPlugins, Start, WebTransportServerIo}, *}, webtransport::server::WebTransportServerPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(
            WindowPlugin {
                primary_window: None,
                exit_condition: bevy::window::ExitCondition::DontExit,  // headless
                ..default()
            }
        ))
        .insert_resource(ClearColor(TEAL.into()))
        .add_plugins(ServerPlugins::default())
        .add_plugins(ProtocolPlugin)
        .add_systems(Startup, (setup, start).chain())
        .add_observer(accept_request)
        .add_observer(handle_new_client)
        .add_observer(handle_connected_client)

        .add_systems(Update, verify)
        .add_systems(Update, verify_clients)
        .run();
}

fn verify(server: Single<Entity, With<Server>>) {
    info!("Server is active")
}
fn verify_clients(clients: Query<Entity, With<Client>>) {
    info!("Client number: {}", clients.iter().len());
}
/// starts the server
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands
        .spawn((
            NetcodeServer::new(NetcodeConfig::default()),
            LocalAddr(SERVER_ADDR),
            WebTransportServerIo {
                #[cfg(debug_assertions)]
                certificate: Identity::self_signed(LOCALHOSTS).unwrap()
            },
            Server::default()
        ));
}

fn start(mut commands: Commands, server: Single<Entity, With<Server>>) {
    info!("Server started");
    
    commands.trigger(Start { entity: *server });
}
/// accept all requests via WebTransport
fn accept_request(mut request: On<SessionRequest>) {
    let client = request.event_target();
    info!("Accepted client {}", client);
    request.respond(SessionResponse::Accepted);
}

/// connecting client
fn handle_new_client(connecting_client: On<Add, LinkOf>, mut commands: Commands) {
    commands.entity(connecting_client.entity)
        .insert((
            ReplicationSender::new(Duration::from_secs(1), SendUpdatesMode::SinceLastAck, false),
            Name::from("Client")
        ));
    info!("added LinkOf");
}

/// connected client. setup sprites etc
fn handle_connected_client(
    connected_client: On<Add, Connected>,
    query: Query<&RemoteId, With<ClientOf>>,
    mut commands: Commands
) {
    let Ok(RemoteId(client_id)) = query.get(connected_client.entity) else {
        return;
    };
    let entity = commands.spawn((
        Replicate::to_clients(NetworkTarget::Only(vec![*client_id])),  // draw to all clients
        SpawnYasen,
        Transform::from_xyz(10.0, 10.0, 0.0)
    )).id();

    info!("Sent create yasen: {entity} for client {client_id}");
}

fn handle_disconnect(
    disconnected: On<Add, Disconnected>
) {
    info!("Client {} disconnected", disconnected.entity)
}