//! custom networking backend

use std::net::TcpStream;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::{io::Read, net::TcpListener, sync::mpsc};

use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;
use common::{Boat, TCP_ADDR};
use common::util::InputExt;
use common::primitives::ZIndex;
use common::protocol::EntityOnServer;

pub(crate) struct NetPlugin;

impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(FixedUpdate, (read_request, spawn_receiver_for_client))
            .add_observer(move_client_request_rx);
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
    rx: ClientRequestReceiver,
    /// the target entity (Boat)
    end: Entity
}

fn read_request(
    // rxs: Query<(Entity, &ClientRequestReceiver)>,
    // mut z_index: Query<&mut ZIndex>,
    world: &mut World
) {
    let mut rxs = world.query::<(Entity, &ClientRequestReceiver)>();
    for (entity, rx) in rxs.iter(world) {
        match rx.0.try_recv() {
            Ok(TcpClientRequest::NewZIndex(new)) => info!(?new),
            Ok(TcpClientRequest::ControlledBoatOnServer(boat)) => {
                let rx_owned = world.get_entity_mut(entity).unwrap().take::<ClientRequestReceiver>().unwrap();
                let tx = world.get_resource::<MoveClientRequestReceiverTx>().unwrap();
                tx.0.send(MoveClientRequestReceiver {
                    rx: rx_owned,
                    end: Entity::from_bits(boat.0)
                }).unwrap();
            },
            Err(mpsc::TryRecvError::Disconnected) => {
                if let Ok(c) = world.get_entity_mut(entity) {
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
    rx: Res<MoveClientRequestReceiverRx>,
    mut commands: Commands
) {
    if let Ok(msg) = rx.0.try_recv() {
        info!("Moving request rx {:?} to {}", msg.rx, msg.end);

        if let Ok(mut target) = commands.get_entity(msg.end){
            target.insert(msg.rx);
        } else {
            error!("Invalid boat entity specified");
        }
    }
}

// TODO common with different interface impl for client and server
enum TcpClientRequest {
    /// marker: 4 bytes read
    NewZIndex(ZIndex),
    /// to identify a TCP stream on server with the client
    /// 
    /// marker: 8 bytes read
    ControlledBoatOnServer(EntityOnServer)
}

impl TcpClientRequest {
    /// buf is a buffer of current read, ideally a Vec that is cleared every read but that's not possible due to read_to_end waiting for EOF
    /// 
    /// ### Panics
    /// if `read_len > buf.len()`
    fn try_from_buf(buf: &[u8], read_len: usize) -> Option<Self> {
        let full_buf = buf.split_at(read_len).0;
        match read_len {
            4 => Some(Self::NewZIndex(ZIndex(
                f32::from_be_bytes(full_buf.try_into().unwrap())
            ))),
            8 => Some(Self::ControlledBoatOnServer(EntityOnServer(
                u64::from_be_bytes(full_buf.try_into().unwrap())
            ))),
            _ => None
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