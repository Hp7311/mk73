use bevy::prelude::*;
use lightyear::prelude::*;
use common::primitives::{CursorPos, LastSpeed, Mk48Rect, NormalizeRadian as _, Radian, Speed, TargetRotation, WeaponCounter, WrapRadian as _};
use common::protocol::{SpawnWeapon, SendToServer, WeaponRollBack, EntityOnServer, EntityOnClient, WeaponCustomTransform};
use common::util::get_rotate_radian;
use common::{Boat, Weapon};
use common::collision::out_of_bound_no_rotation;
use common::world::WorldSize;
use crate::FiresWeapon;

pub(crate) struct WeaponPlugin;

impl Plugin for WeaponPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(fire_weapon)
            .add_observer(spawn_others_weapon)

            .add_systems(Update, rollback)
            .add_systems(Update, despawn_on_out_of_bound)

            .add_systems(Update, sync_others_transform);
    }
}

fn fire_weapon(
    _: On<FiresWeapon>,
    cursor_pos: Res<CursorPos>,
    mut sender: Single<&mut MessageSender<SpawnWeapon>>,
    boat: Single<(&Transform, &WeaponCounter, &EntityOnServer), (With<Controlled>, With<Boat>)>,
    client_id: Single<&LocalId>,

    mut commands: Commands,
    asset_server: Res<AssetServer>
) {
    let (transform, weapon_counter, &entity_on_server) = boat.into_inner();
    let mut msg = SpawnWeapon {
        weapon: weapon_counter.selected_weapon.unwrap(),
        position: transform.translation,  // currently starts at centre of boat
        starting_rotation: transform.rotation.wrap_radian(),
        end_rotation: get_rotate_radian(transform.translation.xy(), cursor_pos.0).wrap_radian(),
        entity_on_client: EntityOnClient(u64::MAX),
        entity_on_server,
        client_id: client_id.0
    };

    msg.entity_on_client.0 = commands.spawn((
        Sprite {
            image: asset_server.load(msg.weapon.file_name()),
            custom_size: Some(msg.weapon.custom_size()),
            ..default()
        },
        Transform {
            translation: transform.translation,
            // follows boat rotation
            rotation: transform.rotation,
            ..default()
        },
        TargetRotation(msg.end_rotation),
        LastSpeed(Speed::ZERO),
        msg.weapon,

        Name::new("Controlled weapon")
    )).id().to_bits();

    // at back to prevent use-after-move
    sender.send::<SendToServer>(msg);
}

fn spawn_others_weapon(
    trigger: On<Add, WeaponCustomTransform>,
    q: Query<(&WeaponCustomTransform, &Weapon)>,
    mut commands: Commands,
    asset_server: Res<AssetServer>
) {
    // println!("HEY");
    let Ok((custom, weapon)) = q.get(trigger.entity) else { error!("Should spawn Weapon alongside"); return };
    commands.get_entity(trigger.entity).unwrap()
        .insert((
            Transform {
                translation: custom.position,
                rotation: custom.rotation.to_quat(),
                ..default()
            },
            Sprite {
                image: asset_server.load(weapon.file_name()),
                custom_size: Some(weapon.custom_size()),
                ..default()
            },
            Name::new("Other's weapon")
        ));
}

fn sync_others_transform(query: Query<(&WeaponCustomTransform, &mut Transform), Changed<WeaponCustomTransform>>) {
    for (custom, mut transform) in query {
        transform.translation = custom.position;
        transform.rotation = custom.rotation.to_quat();
    }
}

fn rollback(mut reader: Single<&mut MessageReceiver<WeaponRollBack>>, mut commands: Commands, mut weapons: Query<&mut Transform, With<Weapon>>) {
    for msg in reader.receive() {
        match msg {
            WeaponRollBack::Transform {
                position,
                rotation,
                entity
            } => {
                let Ok(mut transform) = weapons.get_mut(Entity::from_bits(entity.0)) else { return };
                transform.translation.x = position.x;
                transform.translation.y = position.y;
                transform.rotation = rotation.to_quat();
            }
            WeaponRollBack::Despawn { entity} => if let Ok(mut weapon) = commands.get_entity(Entity::from_bits(entity.0)) {
                weapon.despawn();
            }
        }
    }
}

// FIXME other client's weapons not in client world
// FIXME when rendering, Z-indexes of other client's boats are potentially incorrect

fn despawn_on_out_of_bound(
    mut commands: Commands,
    weapons: Query<(&Transform, Entity), (With<Weapon>, Changed<Transform>)>,
    world_size: Single<&WorldSize>
) {
    for (transform, id) in weapons {
        if out_of_bound_no_rotation(&world_size, Mk48Rect::from_point(transform.translation.xy())) {
            commands.get_entity(id).unwrap().despawn();
        }
    }
}
