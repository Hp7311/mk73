//! custom networking backend

use std::net::TcpStream;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::{io::Read, net::TcpListener, sync::mpsc};

use bevy::prelude::*;
use common::protocol::tcp::TcpClientRequest;
use common::{Boat, TCP_ADDR};
use common::util::InputExt;
use common::primitives::ZIndex;

pub(crate) struct NetPlugin;

impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(FixedUpdate, (
                read_request,
                spawn_receiver_for_client,
                move_client_request_rx
            ));
    }
}

/// receives data from a client
/// 
/// - spawned as a random entity via [`SpawnClientRequestReceiver`]
/// - once client sends in associated boat entity, attach to it
/// - receive :)
#[derive(Debug, Component)]
struct ClientRequestReceiver(Receiver<TcpClientRequest>);

#[derive(Debug, Resource)]
struct SpawnClientRequestReceiver(Receiver<ClientRequestReceiver>);

/// provides fancy message on drop
#[derive(Debug, Deref, DerefMut)]
struct StreamWrapper(TcpStream);

// SAFETY: only one system reading
unsafe impl Sync for ClientRequestReceiver {}
unsafe impl Sync for SpawnClientRequestReceiver {}


fn setup(mut commands: Commands) {
    let (spawn_new_channel, rx) = mpsc::channel();
    commands.insert_resource(SpawnClientRequestReceiver(rx));
    
    let (tx, rx) = mpsc::channel();
    commands.insert_resource(MoveClientRequestReceiverRx(rx));
    commands.insert_resource(MoveClientRequestReceiverTx(tx));

    thread::spawn(move || {
        let socket = TcpListener::bind(TCP_ADDR).unwrap();
        let mut buf = [0; 20];

        for stream in socket.incoming() {
            let spawn_new_channel = spawn_new_channel.clone();
            thread::spawn(move || {
                // set up new channel for this specific client
                let (read_tx, read_rx) = mpsc::channel::<TcpClientRequest>();
                spawn_new_channel.send(ClientRequestReceiver(read_rx)).unwrap();

                let mut stream = StreamWrapper(stream.unwrap());
                info!("New client {}", stream.remote_addr());

                while let Ok(amount) = stream.read(&mut buf)
                    && amount != 0
                {
                    let req = TcpClientRequest::try_from_buf(&buf, amount).unwrap();

                    read_tx.send(req).unwrap();
                }
            });
        }
    });
}

/// spawns just the receiver for data from client
fn spawn_receiver_for_client(
    mut commands: Commands,
    spawn: Res<SpawnClientRequestReceiver>
) {
    if let Ok(rx) = spawn.0.try_recv() {
        commands.spawn(rx);
    }
}

#[derive(Debug, Resource)]
struct MoveClientRequestReceiverRx(Receiver<MoveClientRequestReceiver>);
#[derive(Debug, Resource)]
struct MoveClientRequestReceiverTx(Sender<MoveClientRequestReceiver>);

// SAFETY: only one system reading
unsafe impl Sync for MoveClientRequestReceiverRx {}
unsafe impl Sync for MoveClientRequestReceiverTx {}

// #[derive(Debug, Event)]
/// sent via mpsc
struct MoveClientRequestReceiver {
    /// the moving entity
    start: Entity,
    /// the target entity (Boat)
    end: Entity
}

fn read_request(
    rxs: Query<(Entity, &ClientRequestReceiver)>,
    mut z_index: Query<&mut ZIndex, With<Boat>>,
    mut commands: Commands,
    tx: Res<MoveClientRequestReceiverTx>
) {
    for (entity, rx) in rxs {
        match rx.0.try_recv() {
            Ok(TcpClientRequest::NewZIndex(new)) => {
                if let Ok(mut z) = z_index.get_mut(entity) {
                    *z = new;
                }
            },
            Ok(TcpClientRequest::ControlledBoatOnServer(boat)) => {
                // let rx_owned = world.get_entity_mut(entity).unwrap().take::<ClientRequestReceiver>().unwrap();
                tx.0.send(MoveClientRequestReceiver {
                    start: entity,
                    end: Entity::from_bits(boat.0)
                }).unwrap();
            },
            Err(mpsc::TryRecvError::Disconnected) => {
                if let Ok(mut c) = commands.get_entity(entity) {
                    // client diconnected, despawn if not already by lightyear
                    c.despawn();
                }
            }
            Err(mpsc::TryRecvError::Empty) => (),
        }
    }
}

/// move client request receivers to the specified boat entity they're associated with
fn move_client_request_rx(
    world: &mut World
) {
    let rx = world.get_resource::<MoveClientRequestReceiverRx>().unwrap();
    if let Ok(msg) = rx.0.try_recv() {
        let start = world.get_entity_mut(msg.start).unwrap().take::<ClientRequestReceiver>().unwrap();
        info!("Moving request rx from {:?} to {}", msg.start, msg.end);

        if let Ok(mut target) = world.get_entity_mut(msg.end) {
            target.insert(start);
        } else {
            error!("Invalid boat entity specified");
        }
    }
}


impl Drop for StreamWrapper {
    fn drop(&mut self) {
        info!("Client {} disconnected", self.remote_addr());
    }
}
impl StreamWrapper {
    fn remote_addr(&self) -> String {
        self.0.peer_addr().map(|a| a.to_string()).unwrap_or("N/A".to())
    }
}