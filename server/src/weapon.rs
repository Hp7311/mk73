use lightyear::prelude::*;
use bevy::prelude::*;
use common::primitives::{LastSpeed, Speed, TargetRotation};
use common::protocol::SpawnWeapon;

/// note that we're NOT using replication etc to sync weapon position and rotation due to small diff
///
/// Replicated to all but client "controlling" the weapon:
/// - [`Transform`] directly
/// - [`Weapon`](common::Weapon)
/// 
/// Spawned locally for movement:
/// - [`TargetRotation`]
/// - [`LastSpeed`]
pub(crate) struct WeaponPlugin;

impl Plugin for WeaponPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, recv_spawning);
    }
}

/// spawns server's independent copy of Weapon, locally moved if validation passes
fn recv_spawning(
    rxs: Query<&mut MessageReceiver<SpawnWeapon>>,
    mut commands: Commands,
    // mut sender: ServerMultiMessageSender,
    // server: Single<&Server>
) {
    for mut rx in rxs {
        for msg in rx.receive() {
            commands.spawn((
                // transform replicated to clients
                Transform {
                    translation: msg.position,
                    // follows boat rotation
                    rotation: msg.starting_rotation.to_quat(),
                    ..default()
                },
                msg.weapon,

                TargetRotation(msg.end_rotation),
                LastSpeed(Speed::ZERO),

                Replicate::to_clients(NetworkTarget::AllExceptSingle(msg.client_id))
            ));
            // sender.send::<_, SendToClient>(&WeaponRollBack::Transform { position: Vec3::ZERO, rotation: rand::random(), entity: msg.entity_on_client}, &server, &NetworkTarget::Single(msg.client_id)).unwrap();
        }
    }
}
