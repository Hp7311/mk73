mod oil_rig;
mod weapon;
mod tcp;

use std::time::Duration;

#[cfg(not(feature = "gui"))]
use bevy::app::ScheduleRunnerPlugin;
use bevy::{diagnostic::{DiagnosticsPlugin, LogDiagnosticsPlugin}, log::LogPlugin, prelude::*, state::app::StatesPlugin};
use common::{
    Boat, BoatClientId, MovementPlugin, OCEAN_SURFACE, PROTOCOL_ID, SERVER_ADDR, UpgradePlugin, WorldPlugin, primitives::{CustomTransform, PlayerStats, Position, WeaponCounter, ZIndex}, protocol::{Move, ProtocolPlugin, Rotate}
};
use lightyear::{
    prelude::input::native::ActionState, webtransport::server::WebTransportServerIo
};
use lightyear::{
    netcode::NetcodeServer,
    prelude::{
        server::{ClientOf, NetcodeConfig, ServerPlugins, Start},
        *,
    },
};
use common::protocol::{EntityOnServer, ZIndexUpdate};
use crate::{oil_rig::OilRigPlugin};
use crate::weapon::WeaponPlugin;

fn main() {
    let mut app = App::new();
    #[cfg(not(feature = "gui"))]
    app.add_plugins((
        // headless plugins
        MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f32(1.0 / 60.0))),
        DiagnosticsPlugin,
        LogDiagnosticsPlugin::default(),
        LogPlugin::default(),
        StatesPlugin,
    ));
    #[cfg(feature = "gui")]
    app.add_plugins((DefaultPlugins, gui::GuiPlugin));

    app
        .add_plugins(ServerPlugins::default())
        .add_plugins(ProtocolPlugin)
        .add_plugins(OilRigPlugin)
        .add_plugins(WeaponPlugin)
        .add_plugins(UpgradePlugin)
        .add_systems(Startup, setup)
        .add_plugins(WorldPlugin)
        // // handle client action
        .add_plugins(MovementPlugin { move_weapon: true })
        .add_systems(FixedUpdate, recv_new_z_index)

        // handle client req
        .add_observer(handle_new_client)
        .add_observer(handle_connected_client);

    // app.add_plugins(NetPlugin);

    app.run();
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
            // WebSocketServerIo {
            //     #[cfg(debug_assertions)]
            //     config: ServerConfig::builder()
            //         .with_bind_address(SERVER_ADDR)
            //         .with_no_encryption(),
            // },
            WebTransportServerIo {
                certificate: {
                    let runtime = tokio::runtime::Runtime::new().unwrap();
                    runtime.block_on(
                        Identity::load_pemfiles("../cert/cert.pem", "../cert/key.pem")
                    ).expect("Couldn't load certificate")
                }
            },
        ))
        .id();

    commands.trigger(Start { entity: server });

    #[cfg(feature = "gui")]
    {
        commands.spawn(Camera2d);
        commands.insert_resource(ClearColor(Color::WHITE));
    }
}

/// connecting client
fn handle_new_client(connecting_client: On<Add, LinkOf>, mut commands: Commands) {
    commands
        .entity(connecting_client.entity)
        .insert(ReplicationSender::default());
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
        boat,
        OCEAN_SURFACE,
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
        ActionState::<ZIndexUpdate>::default(),

        // children![(
        //     OCEAN_SURFACE,
        //     Replicate::to_clients(NetworkTarget::AllExceptSingle(client_id))
        // )],
        
        ControlledBy {
            owner: entity,
            lifetime: Lifetime::SessionBased
        },
    ));
    entity_commands.insert(EntityOnServer(entity_commands.id().to_bits()));
}

fn recv_new_z_index(
    // rxs: Query<&mut MessageReceiver<NewZIndex>>,
    q: Query<(&ActionState<ZIndexUpdate>, &mut ZIndex)>
) {
    // for mut rx in rxs {
        // for msg in rx.receive() {
        //     let Ok(mut z_index) = z_index.get_mut(Entity::from_bits(msg.entity_on_server.0)) else {
        //         error!("Client sent a non-existent Boat ID");
        //         return;
        //     };
        //     info!(?msg.new_index);
        //     *z_index = msg.new_index;
        // }
    // }
    for (z_update, mut z_index) in q {
        let Some(target) = z_update.0.0 else { return; };
        *z_index = target;
    }
}

/// gives the server a GUI for debugging
#[cfg(feature = "gui")]
mod gui {
    use super::*;
    use common::primitives::Size;
    pub struct GuiPlugin;

    impl Plugin for GuiPlugin {
        fn build(&self, app: &mut App) {
            app.add_observer(spawn_sprite);
            app.add_systems(FixedUpdate, update_transform);
        }
    }
    fn spawn_sprite(
        trigger: On<Add, CustomTransform>,
        boat: Query<(&Boat, &CustomTransform)>,
        asset_server: Res<AssetServer>,
        mut commands: Commands
    ) {
        let (boat, custom) = boat.get(trigger.entity).unwrap();

        commands.get_entity(trigger.entity).unwrap()
            .insert((
                Sprite {
                    custom_size: Some(boat.render_size()),
                    image: asset_server.load("yasen.png"),
                    ..default()
                },
                Transform::from_translation(custom.position.extend(ZIndex(0.0)))
            ));
        info!("Spawned sprite at {:?}", custom.position.0);
    }
    fn update_transform(
        query: Query<(&CustomTransform, &mut Transform), Changed<CustomTransform>>
    ) {
        for (custom, mut transform) in query {
            transform.translation = custom.position.0.extend(0.0);
            transform.rotation = custom.rotation.to_quat();
        }
    }
}