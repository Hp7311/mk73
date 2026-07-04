use bevy::{color::palettes::css::GRAY, prelude::*};
use common::{
    Boat, BoatReverseNegative, BoatReversePositive, BoatType, CIRCLE_HUD, CircleHud, OCEAN_SURFACE,
    circle_hud_mesh,
    primitives::{CustomTransform, MeshBundle, Size, WeaponCounter},
    util::OrderedHashMap,
};
use lightyear::prelude::*;

use crate::asset::SpriteMap;

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
    boats: Query<(&Boat, &CustomTransform)>, //  &EntityOnServer // legacy custom TCP impl
    controlled: Query<(), (With<Boat>, With<Controlled>)>,

    mut commands: Commands,
    sprites: Res<SpriteMap>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let (&boat, &custom) = boats.get(trigger.entity).unwrap();
    let controls = controlled.get(trigger.entity).is_ok();

    info!("new boat");
    if !controls {
        info!("doesnt control");
        commands
            .get_entity(trigger.entity)
            .unwrap()
            .insert(BoatBundleNotControl {
                transform: Transform {
                    translation: custom.position.0.extend(*OCEAN_SURFACE),
                    rotation: custom.rotation.to_quat(),
                    ..default()
                },
                sprite: Sprite {
                    image: sprites.image(),
                    custom_size: Some(boat.render_size()),
                    texture_atlas: sprites.get(boat),
                    ..default()
                },
            });
        return;
    }

    commands
        .get_entity(trigger.entity)
        .unwrap()
        // .insert(OCEAN_SURFACE)
        .insert(BoatBundle {
            boat,
            weapon_counter: WeaponCounter {
                weapons: boat.armanents(),
                selected_weapon: boat.default_weapon(),
            },
            sprite: Sprite {
                image: sprites.image(), // preload assets
                custom_size: Some(boat.render_size()),
                texture_atlas: sprites.get(boat),
                ..default()
            },
            transform: Transform {
                translation: custom.position.extend(OCEAN_SURFACE),
                rotation: custom.rotation.to_quat(),
                ..default()
            },
        })
        .with_children(|parent| {
            let circle_hud_radius = boat.circle_hud_radius();
            let reverse_indicator_length = 10.0;

            parent
                .spawn((
                    MeshBundle {
                        mesh: Mesh2d(meshes.add(circle_hud_mesh(circle_hud_radius))),
                        materials: MeshMaterial2d(materials.add(ColorMaterial::from_color(GRAY))),
                    },
                    Transform::from_xyz(0.0, 0.0, *CIRCLE_HUD),
                    CircleHud,
                ))
                .insert(children![
                    // reverse indicators
                    (
                        Transform::from_translation(
                            BoatReversePositive::relative_pos(circle_hud_radius)
                                .extend(*CIRCLE_HUD)
                        ),
                        MeshBundle {
                            mesh: Mesh2d(
                                meshes.add(BoatReversePositive::mesh(reverse_indicator_length))
                            ),
                            materials: MeshMaterial2d(
                                materials.add(ColorMaterial::from_color(GRAY))
                            )
                        },
                        BoatReversePositive
                    ),
                    (
                        Transform::from_translation(
                            BoatReverseNegative::relative_pos(circle_hud_radius)
                                .extend(*CIRCLE_HUD)
                        ),
                        MeshBundle {
                            mesh: Mesh2d(
                                meshes.add(BoatReverseNegative::mesh(reverse_indicator_length))
                            ),
                            materials: MeshMaterial2d(
                                materials.add(ColorMaterial::from_color(GRAY))
                            )
                        },
                        BoatReverseNegative
                    )
                ]);
        })
        .insert(Name::new("Client's boat"));

    // associate socket with boat
    // let amount = tcp.write(&TcpClientRequest::ControlledBoatOnServer(*entity_on_server).to_bytes()).unwrap();
    // assert_eq!(amount, 8);

    commands.insert_resource(BoatType(boat.sub_kind()));
}

// is it better to directly manipulate Transform
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
    // custom_transform: CustomTransform, // check
    // /// where the user's mouse was facing
    // mouse_target: TargetRotation,
    // /// the target speed of the Boat
    // target_speed: TargetSpeed,
    // out_of_bound: OutOfBound,
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
            // custom_transform: CustomTransform::default(),
            // out_of_bound: OutOfBound(false),
            // mouse_target: TargetRotation::default(),
            // target_speed: TargetSpeed::default(),
            weapon_counter: WeaponCounter {
                weapons: OrderedHashMap::new(),
                selected_weapon: None,
            },
            boat: Boat::Yasen, // should be G5
        }
    }
}
