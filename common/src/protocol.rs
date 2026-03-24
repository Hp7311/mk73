//! defines structures to be sent between client and server

use bevy::prelude::*;
use lightyear::prelude::{*};
use serde::{Deserialize, Serialize};
use wtransport::tls::{Certificate, CertificateChain, PrivateKey};

#[derive(Debug, Deserialize, Serialize, Clone, Component, PartialEq)]
pub struct SpawnYasen;

pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.register_component::<SpawnYasen>();
    }
}

pub fn self_signed() -> Identity {
    let cert = CertificateChain::new(vec![Certificate::from_der(
        std::fs::read("../cert.der").unwrap()
    ).unwrap()]);
    let key = PrivateKey::from_der_pkcs8(
        std::fs::read("../key.der").unwrap()
    );
    Identity::new(cert, key)
}
