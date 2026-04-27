mod oil_rig;

use std::time::Duration;

use bevy::{
    color::palettes::css::{GRAY, TEAL},
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};
use bevy_inspector_egui::egui::NUM_POINTER_BUTTONS;
use common::{LOCAL_SERVER_ADDR, PROTOCOL_ID, boat::Boat, primitives::*, protocol::{Move, ProtocolPlugin, Rotate, SendToClient}, util::add_circle_hud, world::{Background, WorldPlugin}, MovementPlugin};
use lightyear::{
    prelude::input::native::ActionState,
    websocket::server::Identity,
};
use lightyear::{
    netcode::NetcodeServer,
    prelude::{
        server::{ClientOf, NetcodeConfig, ServerConfig, ServerPlugins, Start, WebSocketServerIo},
        *,
    },
};
use common::protocol::{OilRigInfo, PlayerScore};
use crate::oil_rig::OilRigPlugin;

fn main() {
    App::new()
        .add_plugins(
            // headless plugins
            DefaultPlugins.set(WindowPlugin {
                primary_window: None,
                exit_condition: bevy::window::ExitCondition::DontExit,
                ..default()
            }),
        )
        .add_plugins(ServerPlugins::default())
        .add_plugins(ProtocolPlugin)
        .add_plugins(OilRigPlugin)
        .add_systems(Startup, setup)
        .add_plugins(WorldPlugin { is_server: true })
        // handle client action
        .add_plugins(MovementPlugin { is_server: true })
        // handle client req
        .add_observer(handle_new_client)
        .add_observer(handle_connected_client)

        .run();
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
                    .with_no_encryption(),
            },
        ))
        .id();

    commands.trigger(Start { entity: server });

    commands.spawn((
        OilRigInfo {
            position: default(),
            rotation: Radian(0.0),
            custom_size: Vec2::ZERO
        },
        Replicate::to_clients(NetworkTarget::All)
    ));
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

// TODO seperate CUstomTransform?

/// connected client. setup sprites etc
fn handle_connected_client(
    connected_client: On<Add, Connected>,
    clients: Query<&RemoteId, With<ClientOf>>,
    mut commands: Commands,
) {
    let entity = connected_client.entity; // NOT equal to client id or Client entity in client's world
    let Ok(&RemoteId(client_id)) = clients.get(entity) else {
        warn!("Didn't find the connected client in Query<&RemoteId, With<ClientOf>");
        return;
    };

    let boat = Boat::Yasen;
    let position = vec2(
        rand::random_range(-200.0..200.0),
        rand::random_range(-200.0..200.0),
    ); // TODO

    commands.spawn((
        CustomTransform {
            position: Position(position),
            ..CustomTransform::default()
        },
        boat,
        OutOfBound(false),
        PlayerScore::new(0),
        
        Replicate::to_clients(NetworkTarget::All),
        PredictionTarget::to_clients(NetworkTarget::Single(client_id)),
        InterpolationTarget::to_clients(NetworkTarget::AllExceptSingle(client_id)),
        
        ActionState::<Rotate>::default(),
        ActionState::<Move>::default(),
        
        ControlledBy {
            owner: entity,
            lifetime: Lifetime::SessionBased
        },
    ));
}

/// webtransport certificate
fn from_pem_file(
    cert_path: impl AsRef<std::path::Path>,
    key_path: impl AsRef<std::path::Path>,
) -> Identity {
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

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use bevy::log::LogPlugin;
    use bevy::prelude::*;
    use lightyear::netcode::{NetcodeClient, NetcodeServer};
    use lightyear::netcode::server_plugin::NetcodeConfig;
    use lightyear::prelude::*;
    use lightyear::prelude::client::{ClientPlugins};
    use lightyear::prelude::server::{ServerPlugins, ServerUdpIo, Start};
    use lightyear::prelude::UdpIo;
    use serde::{Deserialize, Serialize};

    // #[test]
    #[ignore]
    fn child_spawning() {
        const SERVER_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8000);
        const CLIENT_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8001);

        let mut server = App::new();
        server.add_plugins((MinimalPlugins, LogPlugin::default(), ProtocolPlugin, ServerPlugins::default()));
        let spawn_id = server.world_mut()
            .spawn((
                NetcodeServer::new(NetcodeConfig::default()),
                LocalAddr(SERVER_ADDR),
                ServerUdpIo::default(),
            ))
            .id();
        server.world_mut().trigger(Start { entity: spawn_id });

        server.add_observer(|trigger: On<Add, LinkOf>, mut commands: Commands| {
            commands.get_entity(trigger.entity).unwrap().insert((
                ReplicationSender::default(),
                Name::from("Client"),
            ));
            println!("New")
        });

        let mut client = App::new();
        client.add_plugins((MinimalPlugins, LogPlugin::default(), ProtocolPlugin, ClientPlugins::default()));

        let spawn_id = client.world_mut().spawn((
            Client::default(),
            LocalAddr(CLIENT_ADDR),
            PeerAddr(SERVER_ADDR),
            Link::new(None),
            ReplicationReceiver::default(),
            PredictionManager::default(),
            NetcodeClient::new(Authentication::Manual { server_addr: SERVER_ADDR, client_id: 0, private_key: default(), protocol_id: 0}, default()).unwrap(),
            UdpIo::default()
        )).id();
        client.world_mut().trigger(Connect { entity: spawn_id });

        for _ in 0..300 {
            server.update();
            client.update();
        }


        server.world_mut().spawn((
            ParentComponent(8),
            Replicate::to_clients(NetworkTarget::All),
            // children![
            //     ChildComponent(1)
            // ]
        ));

        info!("Spawned before update");
        for _ in 0..6000 {
            server.update();
            client.update();
        }
        client.add_systems(Update, |q: Query<&Replicated>| {
            assert_eq!(q.iter().len(), 1);
            println!("Passed");
        });
        server.add_systems(Update, |q: Query<&ChildComponent>| {
            assert_eq!(q.iter().len(), 1);
        });

        client.update();
        server.update();
    }

    #[derive(Component, Deserialize, Serialize, PartialEq)]
    struct ParentComponent(u128);
    #[derive(Component, Deserialize, Serialize, PartialEq)]
    struct ChildComponent(u8);

    struct ProtocolPlugin;

    impl Plugin for ProtocolPlugin {
        fn build(&self, app: &mut App) {
            app.register_component::<ParentComponent>();
            app.register_component::<ChildComponent>();
        }
    }
}