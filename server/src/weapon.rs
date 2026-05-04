use common::collision::out_of_bound_no_rotation;
use common::world::WorldSize;
use lightyear::prelude::*;
use bevy::prelude::*;
use common::primitives::{LastSpeed, Mk48Rect, Speed, TargetRotation};
use common::Weapon;
use common::protocol::SpawnWeapon;

/// note that we're NOT using replication etc to sync weapon position and rotation due to small diff
///
/// Replicated to all but client "controlling" the weapon:
/// - [`Transform`] directly
/// - [`Weapon`]
pub(crate) struct WeaponPlugin;

impl Plugin for WeaponPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, recv_spawning);
        app.add_systems(FixedUpdate, despawn_out_of_bounds);
    }
}

/// spawns server's independent copy of Weapon, locally moved if validation passes
fn recv_spawning(rxs: Query<&mut MessageReceiver<SpawnWeapon>>, mut commands: Commands) {
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
            // rollback_tx.send::<SendToClient>(WeaponRollBack::Transform { position: Vec2::ZERO, rotation: rand::random(), entity: msg.entity_on_client})
        }
    }
}

fn despawn_out_of_bounds(weapons: Query<(&Transform, Entity), With<Weapon>>, mut commands: Commands, world_size: Single<&WorldSize>) {
    for (transform, id) in weapons {
        if out_of_bound_no_rotation(&world_size, Mk48Rect::from_point(transform.translation.xy())) {
            commands.get_entity(id).unwrap()
                .despawn();
            trace!("Despawned {id} due to outofbounds: {}", transform.translation);
        }
    }
}