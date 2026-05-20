mod oil_rig;
mod weapon;

use std::time::Duration;

use bevy::{diagnostic::{DiagnosticsPlugin, LogDiagnosticsPlugin}, log::LogPlugin, prelude::*, state::app::StatesPlugin};
use common::{
    Boat, BoatClientId, MovementPlugin, OCEAN_SURFACE, PROTOCOL_ID, SERVER_ADDR, UpgradePlugin, WorldPlugin, primitives::{CustomTransform, PlayerStats, Position, WeaponCounter, ZIndex}, protocol::{Move, ProtocolPlugin, Rotate, SetupServer, SystemSetPlugin}
};
use lightyear::{
    prelude::input::native::ActionState,
};
use lightyear::{
    netcode::NetcodeServer,
    prelude::{
        server::{ClientOf, NetcodeConfig, ServerConfig, ServerPlugins, Start, WebSocketServerIo},
        *,
    },
};
use common::protocol::{EntityOnServer, NewZIndex};
use crate::oil_rig::OilRigPlugin;
use crate::weapon::WeaponPlugin;

// FIXME server disconnects after few minutes
fn main() {
    App::new()
        .add_plugins((
            // headless plugins
            MinimalPlugins,
            DiagnosticsPlugin,
            LogDiagnosticsPlugin::default(),
            LogPlugin::default(),
            StatesPlugin,
        ))
        .add_plugins(ServerPlugins::default())
        .add_plugins(ProtocolPlugin)
        .add_plugins(SystemSetPlugin { is_server: true })
        .add_plugins(OilRigPlugin)
        .add_plugins(WeaponPlugin)
        .add_plugins(UpgradePlugin)
        .add_systems(Startup, setup.in_set(SetupServer::Io))
        .add_plugins(WorldPlugin)
        // handle client action
        .add_plugins(MovementPlugin { move_weapon: true })
        .add_systems(FixedUpdate, recv_new_z_index)

        // handle client req
        .add_observer(handle_new_client)
        .add_observer(handle_connected_client)

        .run();
}

/// starts the server
fn setup(mut commands: Commands) {
    let netcode_config = NetcodeConfig {
        protocol_id: PROTOCOL_ID,
        num_disconnect_packets: 50,
        // client_timeout_secs: -1,
        ..Default::default()
    };
    let server = commands
        .spawn((
            NetcodeServer::new(netcode_config),
            LocalAddr(SERVER_ADDR),
            WebSocketServerIo {
                #[cfg(debug_assertions)]
                config: ServerConfig::builder()
                    .with_bind_address(SERVER_ADDR)
                    .with_no_encryption(),
            },
        ))
        .id();

    commands.trigger(Start { entity: server });
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

// TODO seperate CustomTransform?

/// connected client. spawns the main boat entity
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
    );

    let mut entity_commands = commands.spawn((
        CustomTransform {
            position: Position(position),
            ..CustomTransform::default()
        },
        OCEAN_SURFACE,
        boat,
        WeaponCounter {
            weapons: boat.armanents(),
            selected_weapon: boat.default_weapon()
        },
        PlayerStats::new(0),
        
        BoatClientId(client_id),
        
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
    entity_commands.insert(EntityOnServer(entity_commands.id().to_bits()));
}

fn recv_new_z_index(
    rxs: Query<&mut MessageReceiver<NewZIndex>>,
    mut z_index: Query<&mut ZIndex>
) {
    for mut rx in rxs {
        for msg in rx.receive() {
            let Ok(mut z_index) = z_index.get_mut(Entity::from_bits(msg.entity_on_server.0)) else {
                error!("Client sent a non-existent Boat ID");
                return;
            };
            *z_index = msg.new_index;
        }
    }
}

#[cfg(test)]
#[allow(dead_code)]
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