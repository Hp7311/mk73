use bevy::color::palettes::css::LIME;
use bevy::prelude::*;
use lightyear::prelude::*;
use common::primitives::{CursorPos, LastSpeed, MeshBundle, Mk48Rect, Speed, TargetRotation, WeaponCounter, WrapRadian as _};
use common::protocol::{SpawnWeapon, SendToServer, WeaponRollBack, EntityOnServer, EntityOnClient};
use common::util::get_rotate_radian;
use common::{Boat, CIRCLE_HUD, Weapon};
use common::collision::out_of_bound_no_rotation;
use common::world::WorldSize;
use crate::FiresWeapon;

pub(crate) struct WeaponPlugin;

impl Plugin for WeaponPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_observer(fire_weapon)
            .add_observer(spawn_others_weapon)

            .add_systems(Update, rollback)
            .add_systems(FixedUpdate, despawn_on_out_of_bound)
            .add_systems(Update, sync_weapon_marker);
    }
}

fn fire_weapon(
    _: On<FiresWeapon>,
    cursor_pos: Res<CursorPos>,
    mut sender: Single<&mut MessageSender<SpawnWeapon>>,
    boat: Single<(&Transform, &WeaponCounter, &EntityOnServer), (With<Controlled>, With<Boat>)>,
    client_id: Single<&LocalId>,

    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>
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
    
    spawn_weapon_marker(&mut commands, Entity::from_bits(msg.entity_on_client.0),  msg.position.xy(), &mut meshes, &mut materials);
    sender.send::<SendToServer>(msg);
}

fn spawn_others_weapon(
    trigger: On<Add, Weapon>,
    weapons: Query<(&Weapon, &Transform), With<Replicated>>,
    mut commands: Commands,

    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>
) {
    let Ok((weapon, transform)) = weapons.get(trigger.entity) else {
        // not others'
        return;
    };

    commands.get_entity(trigger.entity).unwrap()
        .insert((
            Sprite {
                image: asset_server.load(weapon.file_name()),
                custom_size: Some(weapon.custom_size()),
                ..default()
            },
            Name::new("Other's weapon")
        ));
    
    spawn_weapon_marker(&mut commands, trigger.entity, transform.translation.xy(), &mut meshes, &mut materials);
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
                transform.translation = position;
                transform.rotation = rotation.to_quat();
            }
            WeaponRollBack::Despawn { entity} => if let Ok(mut weapon) = commands.get_entity(Entity::from_bits(entity.0)) {
                weapon.despawn();
            }
        }
    }
}


fn despawn_on_out_of_bound(
    mut commands: Commands,
    weapons: Query<(&Transform, Entity), (With<Weapon>, Changed<Transform>)>,
    world_size: Single<&WorldSize>
) {
    for (transform, id) in weapons {
        if out_of_bound_no_rotation(&world_size, Mk48Rect::from_point(transform.translation.xy())) {
            trace!("Despawned {id} due to outofbounds: {}", transform.translation);
            commands.get_entity(id).unwrap().despawn();
        }
    }
}

const MARKER_OFFSET: Vec2 = vec2(0.0, 40.0);
const MARKER_BOTTOM: Vec2 = vec2(0.0, -17.32);

#[derive(Component, Clone, Copy)]
struct WeaponMarker(Entity);

fn spawn_weapon_marker(commands: &mut Commands, weapon_linked: Entity, weapon_pos: Vec2, meshes: &mut Assets<Mesh>, materials: &mut Assets<ColorMaterial>) {
    commands.spawn((
        Transform::from_translation((weapon_pos + MARKER_OFFSET).extend(*CIRCLE_HUD)),
        MeshBundle {
            mesh: Mesh2d(meshes.add(Triangle2d::new(
                vec2(-10.0, 0.0),
                MARKER_BOTTOM,
                vec2(10.0, 0.0),
            ))),
            materials: MeshMaterial2d(materials.add(ColorMaterial::from_color(LIME))),
        },
        WeaponMarker(weapon_linked),
        Name::new("Weapon Marker")
    ));
}

// can't be bothered with Changed<Transform>, a weapon is always moving
/// make the weapon marker upright
fn sync_weapon_marker(mut commands: Commands, weapon: Query<&Transform, With<Weapon>>, markers: Query<(&mut Transform, &WeaponMarker, Entity), Without<Weapon>>) {
    for (mut marker_transform, &WeaponMarker(parent), marker_id) in markers {
        if let Ok(parent_transform) = weapon.get(parent) {
            let marker_translation = parent_transform.translation + MARKER_OFFSET/* important!! */.extend(*CIRCLE_HUD /* important!! */);
            marker_transform.translation = marker_translation;
        } else {
            // parent weapon despawned
            commands.get_entity(marker_id).unwrap()
                .despawn();
        }
    }
}