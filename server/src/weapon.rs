use std::sync::Arc;
use lightyear::prelude::*;
use bevy::prelude::*;
use common::primitives::{LastSpeed, Radian, Speed, TargetRotation};
use common::protocol::{SendToClient, SpawnWeapon, WeaponCustomTransform};

/// note that we're NOT using replication etc to sync weapon position and rotation due to small diff
///
/// msgs usually take 0.01-0.02 secs
pub(crate) struct WeaponPlugin;

impl Plugin for WeaponPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, recv_spawning);
    }
}

/// spawns server's independent copy of Weapon, locally moved if validation passes
fn recv_spawning(mut rx: Single<&mut MessageReceiver<SpawnWeapon>>, mut commands: Commands) {
    for msg in rx.receive() {
        commands.spawn((
            Transform {
                translation: msg.position,
                // follows boat rotation
                rotation: msg.starting_rotation.to_quat(),
                ..default()
            },
            TargetRotation(msg.end_rotation),
            LastSpeed(Speed::ZERO),

            msg.weapon,

            WeaponCustomTransform {
                position: msg.position,
                rotation: msg.starting_rotation
            },
            Replicate::to_clients(NetworkTarget::All/*Except(msg.client_id)*/)  // FIXME this works if only 1 client
        ));
        // rollback_tx.send::<SendToClient>(WeaponRollBack::Transform { position: Vec2::ZERO, rotation: rand::random(), entity: msg.entity_on_client})
    }
}
