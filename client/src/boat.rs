use std::collections::HashMap;

use bevy::{color::palettes::css::GRAY, prelude::*};
use common::{Boat, CIRCLE_HUD, OCEAN_SURFACE, primitives::{CustomTransform, MeshBundle, OutOfBound, WeaponCounter, WrapRadian as _}};
use lightyear::prelude::*;

use crate::{BoatType, MINIMUM_REVERSE};

pub(crate) struct BoatPlugin;

impl Plugin for BoatPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(spawn_boat)
            .add_systems(FixedUpdate, sync_transform_from_custom);
    }
}
/// spawn controlled/not controlled boat
#[allow(clippy::too_many_arguments)]
fn spawn_boat(
    trigger: On<Add, CustomTransform>,
    boats: Query<(&Boat, &CustomTransform)>,
    controlled: Query<(), (With<Boat>, With<Controlled>)>,

    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let (&boat, &custom) = boats.get(trigger.entity).inspect_err(|_| error!("Spawn custom along with boat")).unwrap();
    let controls = controlled.get(trigger.entity).is_ok();

    if !controls {
        commands.get_entity(trigger.entity).unwrap()
            .insert(BoatBundleNotControl {
                transform: Transform {
                    translation: custom.position.0.extend(*OCEAN_SURFACE),
                    rotation: custom.rotation.to_quat(),
                    ..default()
                },
                sprite: Sprite {
                    image: asset_server.load(boat.file_name()),
                    custom_size: Some(boat.sprite_size()),
                    ..default()
                }
            });
        return;
    }

    commands
        .get_entity(trigger.entity).unwrap()
        .insert_if_new(BoatBundle {
            boat,
            // TODO WeaponCounter, OutOfBound etc not needed for not controlling boat
            weapon_counter: WeaponCounter {
                weapons: boat.get_armanents(),
                selected_weapon: boat.default_weapon(),
            },
            sprite: Sprite {
                image: asset_server.load(boat.file_name()), // preload assets
                custom_size: Some(boat.sprite_size()),
                ..default()
            },
            transform: Transform {
                translation: custom.position.extend(OCEAN_SURFACE),
                rotation: custom.rotation.to_quat(),
                ..default()
            },
            custom_transform: custom,
            ..BoatBundle::default()
        })
        .with_children(|parent| {
            let circle_hud_radius = boat.circle_hud_radius();

            parent.spawn((
                MeshBundle {
                    mesh: Mesh2d(meshes.add(Circle::new(circle_hud_radius).to_ring(3.0))),
                    materials: MeshMaterial2d(materials.add(ColorMaterial::from_color(GRAY))),
                },
                Transform::from_xyz(0.0, 0.0, *CIRCLE_HUD)
            ))
            .insert(children![
                // reverse indicators
                (
                    Transform::from_xyz(
                        circle_hud_radius * MINIMUM_REVERSE.cos(),
                        circle_hud_radius * MINIMUM_REVERSE.sin(),
                        *CIRCLE_HUD
                    ),
                    MeshBundle {
                        mesh: Mesh2d(meshes.add(Segment2d::from_ray_and_length(
                            Ray2d::new(Vec2::ZERO, Dir2::new(MINIMUM_REVERSE.wrap_radian().to_vec()).unwrap()),
                            10.0
                        ))),
                        materials: MeshMaterial2d(materials.add(ColorMaterial::from_color(GRAY)))
                    }
                ),
                (
                    Transform::from_xyz(
                        circle_hud_radius * (-MINIMUM_REVERSE).cos(),
                        circle_hud_radius * (-MINIMUM_REVERSE).sin(),
                        *CIRCLE_HUD
                    ),
                    MeshBundle {
                        mesh: Mesh2d(meshes.add(Segment2d::from_ray_and_length(
                            Ray2d::new(Vec2::ZERO, Dir2::new((-MINIMUM_REVERSE).wrap_radian().to_vec()).unwrap()),
                            10.0
                        ))),
                        materials: MeshMaterial2d(materials.add(ColorMaterial::from_color(GRAY)))
                    }
                )
            ]);
        })
        .insert(Name::new("Client's boat"));

    commands.insert_resource(BoatType(boat.sub_kind()));
}


// TODO is it better to directly manipulate Transform
/// for all boats regardless of control
fn sync_transform_from_custom(
    mut query: Query<(&mut Transform, &CustomTransform), (With<Boat>, Changed<CustomTransform>)>,
) {
    for (mut transform, custom) in query.iter_mut() {
        transform.translation.x = custom.position.x;
        transform.translation.y = custom.position.y;
        transform.rotation = custom.rotation.to_quat();
    }
}


#[derive(Bundle, Debug, Clone)]
struct BoatBundle {
    /// tranform to update in seperate system
    transform: Transform, // cannot
    /// ship's sprite
    sprite: Sprite, // cannot
    /// whether reversed, speed etc
    custom_transform: CustomTransform, // check
    // /// where the user's mouse was facing
    // mouse_target: TargetRotation,
    // /// the target speed of the Boat
    // target_speed: TargetSpeed,
    out_of_bound: OutOfBound,
    weapon_counter: WeaponCounter,
    boat: Boat, // check
}

#[derive(Bundle)]
struct BoatBundleNotControl {
    transform: Transform,
    sprite: Sprite,
}

impl Default for BoatBundle {
    /// Should be overwritten:
    /// - `boat`
    /// - `weapon_counter`
    /// - `sprite`
    /// - `transform`
    /// - `custom_transform`
    fn default() -> Self {
        BoatBundle {
            transform: Transform::default(),
            sprite: Sprite::default(),
            custom_transform: CustomTransform::default(),
            out_of_bound: OutOfBound(false),
            // mouse_target: TargetRotation::default(),
            // target_speed: TargetSpeed::default(),
            weapon_counter: WeaponCounter {
                weapons: HashMap::new(),
                selected_weapon: None,
            },
            boat: Boat::Yasen, // should be G5
        }
    }
}
