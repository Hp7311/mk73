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
                .with_no_encryption()  // cfg, clients can only connect via WebSocketScheme::Plain
        },
        // MessageSender::<SpawnSprite>::default()
    )).id();

    commands.trigger(Start { entity: server });
}

/// connecting client
fn handle_new_client(connecting_client: On<Add, LinkOf>, mut commands: Commands) {
    commands.entity(connecting_client.entity).insert((
        ReplicationSender::new(Duration::from_secs(1), SendUpdatesMode::SinceLastAck, false),
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
        .send::<_, SendToClient>(&SpawnSprite {
            position: vec2(30.0, 30.0),
            sprite_name: "yasen.png".to_owned()
        }, *server, &NetworkTarget::Only(vec![*client_id]))
        .expect("Failed to send msg");

    info!("Sent create yasen to clients via `SendToClient");
}
