use std::collections::HashMap;
use std::time::{Duration, Instant};

use common::{Boat, BoatClientId, UpgradeEventCommonFinished, UpgradeSet, Weapon};
use lightyear::prelude::*;
use bevy::prelude::*;
use common::primitives::{LastSpeed, Speed, TargetRotation, WeaponCounter};
use common::protocol::{ReloadWeapon, SendToClient, SendToClientOrdered, SpawnWeapon, WeaponRollBack};

use crate::FPS;

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
        app.add_systems(FixedUpdate, recv_spawning.in_set(UpgradeSet::AfterRecvUpgrade))
            .add_observer(on_upgrade)
            .add_systems(FixedUpdate, reload_weapons.in_set(UpgradeSet::AfterRecvUpgrade));
    }
}

/// spawns server's independent copy of Weapon, locally moved if validation passes
/// 
/// we're taking 1 away from selected from weaponcounter here, [`WeaponCounter::selected`] is ignored on server
fn recv_spawning(
    rxs: Query<&mut MessageReceiver<SpawnWeapon>>,
    mut commands: Commands,
    mut boat_q: Query<(&BoatClientId, &mut WeaponCounter, &mut LastReloaded)>,
    mut sender: ServerMultiMessageSender,
    server: Single<&Server>,
) {
    for mut rx in rxs {
        for msg in rx.receive() {
            let (_, mut counter, mut reload_map) = boat_q.iter_mut().find(|(c, ..)| c.0 == msg.client_id).unwrap();
            let Some(count) =  counter.weapons.get_mut(&msg.weapon) else { panic!("{:?}", counter) };
            if count.avaliable == 0 {
                sender.send::<_, SendToClient>(&WeaponRollBack::Despawn { entity: msg.entity_on_client }, &server, &NetworkTarget::Single(msg.client_id)).unwrap();
                info!("Client sent a weapon request but they don't have enough weapons. Should be caught");
                continue;
            }

            if count.avaliable == count.max {
                let reload = reload_map.get_mut(&msg.weapon).unwrap();
                *reload = Some(Instant::now());  // set latest "reload" time at fire time
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

                // replicated to all but the controlling client, controlling client
                // simulates weapon locally without server intervention
                Replicate::to_clients(NetworkTarget::AllExceptSingle(msg.client_id))
            ));
        }
    }
}

/// when was a counter last reloaded (hashmap)
/// 
/// Some means should reload, None means ignore
#[derive(Debug, Component, Deref, DerefMut)]
pub(crate) struct LastReloaded(pub HashMap<Weapon, Option<Instant>>);

fn on_upgrade(
    trigger: On<UpgradeEventCommonFinished>,
    mut query: Query<(&mut LastReloaded, &Boat), With<WeaponCounter>>
) {
    if let Ok((mut last_reloaded, boat)) = query.get_mut(trigger.entity) {
        last_reloaded.clear();

        info!("Cleared");
        for weapon in boat.armanents().keys() {
            last_reloaded.insert(*weapon, None);
        }
    } else {
        error!("WeaponCounter/boat entity not found");
    }
}

fn reload_weapons(
    query: Query<(&mut WeaponCounter, &mut LastReloaded, &BoatClientId)>,
    mut txs: Query<(&mut MessageSender<ReloadWeapon>, &RemoteId)>
) {
    for (mut counter, mut reload_map, client_id) in query {
        for (weapon, last_reloaded) in reload_map.iter_mut().filter(|(w, i)| i.is_some_and(|i| i.elapsed() > w.reload())) {
            let data = counter.weapons.get_mut(weapon).unwrap();

            // FIXME messages not receiving properly
            if data.avaliable == data.max {
                info!("{weapon:?} reloading complete");
                *last_reloaded = None;
                continue;
            }

            info!("Reloaded {weapon:?}");
            data.avaliable += 1;

            *last_reloaded = Some(Instant::now());

            let (mut sender, _) = txs.iter_mut().find(|(_, id)| id.0 == client_id.0).unwrap();
            sender.send::<SendToClientOrdered>(ReloadWeapon { weapon: *weapon });
        }
    }
}

// reloading mechanisms:
//      - reload starts when a slot is non empty, filling the timer
//      - 