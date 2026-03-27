use std::time::Duration;

use bevy::{color::palettes::css::TEAL, log::LogPlugin, prelude::*};
use common::{
    LOCAL_SERVER_ADDR, PROTOCOL_ID,
    protocol::{ProtocolPlugin, SendToClient, SpawnSprite},
};
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
        .add_systems(Startup, (setup, start).chain())

        .add_observer(handle_new_client)
        .add_observer(handle_connected_client)
        .add_systems(Update, verify_clients)
        .run();
}

fn verify_clients(clients: Query<Entity, With<Client>>) {
    if clients.iter().len() != 0 {
        info!("Client number: {}", clients.iter().len());
    }
}

/// starts the server
fn setup(mut commands: Commands) {

    commands.spawn((
        NetcodeServer::new(NetcodeConfig::default().with_protocol_id(PROTOCOL_ID)),
        LocalAddr(LOCAL_SERVER_ADDR),
        WebSocketServerIo {
            #[cfg(debug_assertions)]
            config: ServerConfig::builder()
                .with_bind_address(LOCAL_SERVER_ADDR)
                .with_no_encryption()  // cfg
        }
    ));
}

fn start(mut commands: Commands, server: Single<Entity, With<Server>>) {
    info!("Server started");

    commands.trigger(Start { entity: *server });
}

/// connecting client
fn handle_new_client(connecting_client: On<Add, LinkOf>, mut commands: Commands) {
    commands.entity(connecting_client.entity).insert((
        ReplicationSender::new(Duration::from_secs(1), SendUpdatesMode::SinceLastAck, false),
        
        // MessageReceiver::<SpawnSprite>::default()
    ));
}

/// connected client. setup sprites etc
fn handle_connected_client(
    connected_client: On<Add, Connected>,
    query: Query<&RemoteId, With<ClientOf>>
) {
    let Ok(RemoteId(client_id)) = query.get(connected_client.entity) else {
        return;
    };
    
    MessageSender::default()
        .send::<SendToClient>(SpawnSprite {
            position: vec2(30.0, 30.0),
            sprite_name: "yasen.png".to_owned()
        });

    info!("Sent create yasen for client {client_id}");
}
