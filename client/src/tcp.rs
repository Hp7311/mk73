use std::{net::TcpStream, time::Duration};

use bevy::prelude::*;
use common::TCP_ADDR;

/// when connecting under the hood:
/// - system assigns an unused localhost port
/// - request to connect is buffered on server
/// 
/// even if client disconnects before accepted, server-side would still be visible for a short time
pub(crate) struct NetPlugin;

#[derive(Debug, Resource, Deref, DerefMut)]
pub(crate) struct TcpWrapper(pub TcpStream);

const CONNECTION_TIMEOUT: Duration = Duration::from_secs(3);

impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        let socket = TcpStream::connect_timeout(&TCP_ADDR, CONNECTION_TIMEOUT).unwrap();

        info!("Client connected to custom backend at {}", TCP_ADDR);
        app.insert_resource(TcpWrapper(socket));
    }
}
