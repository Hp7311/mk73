use common::{BoatClientId, UpgradeSet};
use lightyear::prelude::*;
use bevy::prelude::*;
use common::primitives::{LastSpeed, Speed, TargetRotation, WeaponCounter};
use common::protocol::{SendToClient, SpawnWeapon, WeaponRollBack};

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
        app.add_systems(FixedUpdate, recv_spawning.in_set(UpgradeSet::HandleWeapons));
    }
}

/// spawns server's independent copy of Weapon, locally moved if validation passes
/// 
/// we're taking 1 away from selected from weaponcounter here, [`WeaponCounter::selected`] is ignored on server
fn recv_spawning(
    rxs: Query<&mut MessageReceiver<SpawnWeapon>>,
    mut commands: Commands,
    mut boat_q: Query<(&BoatClientId, &mut WeaponCounter)>,
    mut sender: ServerMultiMessageSender,
    server: Single<&Server>
) {
    for mut rx in rxs {
        for msg in rx.receive() {
            let (_, mut counter) = boat_q.iter_mut().find(|(c, _)| c.0 == msg.client_id).unwrap();
            let Some(count) =  counter.weapons.get_mut(&msg.weapon) else { panic!("{:?}", counter) };
            if count.avaliable == 0 {
                sender.send::<_, SendToClient>(&WeaponRollBack::Despawn { entity: msg.entity_on_client }, &server, &NetworkTarget::Single(msg.client_id)).unwrap();
                info!("Client sent a weapon request but they don't have enough weapons. Should be caught");
                continue;
            }
            count.avaliable -= 1;

            commands.spawn((
                // transform replicated to other clients
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
        }
    }
}
