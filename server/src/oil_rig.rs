//! functions related to Oil Rigs

use std::{f32::consts::PI, ops::Range};
use std::sync::Arc;
use bevy::ecs::system::command::trigger;
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use lightyear::prelude::{MessageManager, MessageReceiver, MessageSender, NetworkTarget, Replicate, ReplicateLike};
use rand::{RngExt, rngs::ThreadRng, seq::IndexedRandom};

use common::{eq, print_num, OCEAN_SURFACE};
use common::boat::Boat;
use common::collision::{out_of_bound_no_rotation, out_of_bounds, square_does_not_intersects};
use common::primitives::{CircleHud, CustomTransform, DecimalPoint, Mk48Rect, Radian, WidthHeight};
use common::protocol::{OilRigInfo, PlayerScore, PointInfo, SendToClient};
use common::util::{point_in_square, tiles_around_point, InputExt};
use common::world::WorldSize;
// use crate::{
//     DEFAULT_SPRITE_SHRINK, WATER_SURFACE,
//     boat::{CircleHud, PlayerScore},
//     collision::{out_of_bound_point, out_of_bounds, square_does_not_intersects},
//     primitives::{DecimalPoint, MkRect, WidthHeight},
//     util::{eq, point_in_square, tiles_around_point},
//     world::WorldSize,
// };

/// client
pub struct OilRigPlugin;

impl Plugin for OilRigPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(spawn_rigs)  // TODO use system sets
            .add_systems(
                Update,
                (rig_spawn_points, move_points, points_obsorbed_despawn)
            )
            .add_plugins(EguiPlugin::default())
            .add_plugins(WorldInspectorPlugin::default());

        app.world_mut().spawn(Camera2d);
    }
}

const SPRITE_SIZE: Vec2 = Vec2::splat(1024.0 * 0.3);

/// maximum amount of points a rig can spawn
const SPAWN_POINT_AMOUNT_MAX: Range<u16> = 30..40;
/// spawns a point around a rig every x-y seconds
#[cfg(debug_assertions)]
const SPAWN_POINT_SPRITE_P: Range<usize> = 0..2;

/// the maximum radius around a rig which a point can spawn
const SPAWN_POINT_RADIUS_MAX: f32 = 100.0;

/// speed at which a point moves toward a ship's HUD center
const POINT_SPEED: f32 = 2.0;

#[derive(Component, Debug, Copy, Clone)]
struct OilRig;

fn spawn_rigs(trigger: On<Add, WorldSize>, mut commands: Commands, world_size: Single<&WorldSize>) {
    let mut rng = rand::rng();

    let mut spawned_rigs = vec![];
    for _ in 0..10 {
        // temporary 10 rigs
        spawned_rigs.push(spawn_random_rig(
            &mut commands,
            &mut rng,
            &world_size,
            &spawned_rigs,
            SPRITE_SIZE
        ));
    }
}


struct RigInfo {
    center: Vec2,
    width: f32,
}

/// spawns a must-valid rig at [`WATER_SURFACE`], returns the dimensions and position of the spawned rig
/// ### Panics
/// assumes that the rig is a square
/// ### Hangs
/// if there aren't space
fn spawn_random_rig(
    commands: &mut Commands,
    rng: &mut ThreadRng,
    world_size: &WorldSize,
    other_rigs: &[(Vec2, Vec2)],
    rig_dimensions: Vec2
) -> (Vec2, Vec2) {
    // TODO constant?
    assert!(eq!(rig_dimensions.x, rig_dimensions.y));

    let mut rotation;
    let mut center;

    'outer: loop {
        rotation = rng.random_range(-PI..PI);
        center = vec2(
            rng.random_range(-world_size.get_size().x / 2.0..world_size.get_size().x / 2.0),
            rng.random_range(-world_size.get_size().y / 2.0..world_size.get_size().y / 2.0)
        );
        if out_of_bounds(
            world_size,
            Mk48Rect {
                center,
                dimensions: rig_dimensions.into(),
            },
            Quat::from_rotation_z(rotation),
        ) {
            continue;
        }
        let rig = RigInfo {
            center,
            width: rig_dimensions.x,
        };

        for &(dimension, center) in other_rigs {
            let other = RigInfo {
                center,
                width: dimension.x
            };

            // roughly filter out those that may intersect
            if !square_does_not_intersects(rig.center, rig.width, other.center, other.width) {
                continue 'outer;
            }
        }

        break;
    }

    let spawn_transform = Transform {
        translation: center.extend(OCEAN_SURFACE),
        rotation: Quat::from_rotation_z(rotation),
        ..default()
    };

    commands.spawn((
        spawn_transform,
        PointAmount::new(rng),
        OilRig,
        OilRigInfo {
            position: center,
            rotation: Radian(rotation),
            custom_size: rig_dimensions
        },
        Replicate::to_clients(NetworkTarget::All)
    ));

    (rig_dimensions, spawn_transform.translation.xy())
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// an entity that provide an amount of points
enum Point {
    Barrel,
    Coin,
    Scrap,
}

#[derive(Component, Debug, Clone)]
struct ParentRig(Entity);

/// holding the amount of points
#[derive(Component, Debug, Clone, Copy)]
struct PointAmount {
    points: u16,
    max_point: u16,
}

fn rig_spawn_points(
    mut commands: Commands,
    rigs: Query<(&mut PointAmount, &Transform, Entity), With<OilRig>>,
    world_size: Single<&WorldSize>,
) {
    let mut rng = rand::rng();
    let mut spawn_p = vec![false; rng.random_range(SPAWN_POINT_SPRITE_P)];
    spawn_p.push(true);

    for (mut point_amount, transform, id) in rigs {
        if point_amount.is_max() {
            continue;
        }

        if *spawn_p.choose(&mut rng).unwrap() {
            let available_tiles: Vec<_> = tiles_around_point(
                    transform.translation.xy(),
                    SPRITE_SIZE.x + SPAWN_POINT_RADIUS_MAX,
                )
                .iter()
                .filter(|&&tile| !point_in_square(tile, SPRITE_SIZE.x, transform.translation.xy()))
                .filter(|&&tile| {
                    !out_of_bound_no_rotation(
                        // okay to not use with rotation outofbound because how small a point is
                        &world_size,
                        Mk48Rect {
                            center: tile,
                            dimensions: WidthHeight::ZERO,
                        },
                    )
                })
                .copied()
                .collect();

            let &chosen_type = Point::ALL.choose(&mut rng).unwrap();
            let &chosen_tile = available_tiles.choose(&mut rng).unwrap();  // TODO not efficient

            commands.spawn((
                Transform { // TODO consider not spawning Transform due to presence of PointInfo, same for rig
                    translation: chosen_tile.extend(OCEAN_SURFACE),
                    ..default()
                },
                chosen_type,
                ParentRig(id),

                PointInfo {
                    position: chosen_tile,
                    file_name: Arc::from(chosen_type.file_name())
                },
                Replicate::to_clients(NetworkTarget::All)
            ));

            point_amount.add(chosen_type.worth());
        }
    }
}

/// move points toward ships that have a CircleHud overlapping them
fn move_points(
    mut points_transform: Query<&mut PointInfo, With<Point>>,
    circle_huds: Query<(&Boat, &CustomTransform), Without<Point>>,
) {
    for (intersect_huds, mut transform) in points_transform.iter_mut().filter_map(|point_tf| {
        let huds_in_point: Vec<(f32, Vec2)> = circle_huds
            .iter()
            .filter(|(boat, hud_tf)| CircleHud::contains(&CircleHud { radius: boat.circle_hud_radius()}, hud_tf.position.0, point_tf.position))
            .map(|(boat, tf)| (boat.circle_hud_radius(), tf.position.0))
            .collect();
        if huds_in_point.is_empty() {
            None
        } else {
            Some((huds_in_point, point_tf))
        }
    }) {
        // move the point toward player for those in 1 player's circle hud
        if intersect_huds.len() == 1 {
            transform.position = transform.position.move_towards(
                intersect_huds.first().unwrap().1,
                POINT_SPEED,
            );
            continue;
        }

        // calculate the distance and make the point go to the nearest ship
        let Some((_, hud_transform)) = intersect_huds.iter().min_by_key(|(_, hud_tf)| {
            transform
                .position
                .distance_squared(*hud_tf)
                .round() as u64  // pixles are too small to be noticeable
        }) else {
            return;
        };

        transform.position = transform
            .position
            .move_towards(*hud_transform, POINT_SPEED);
    }
}

/// increment player's score and despawning the Point if absorbed
fn points_obsorbed_despawn(
    mut commands: Commands,
    points_transform: Query<(&PointInfo, &Point, &ParentRig, Entity)>,
    mut circle_huds: Query<(&Boat, &CustomTransform, &mut PlayerScore)>,
    mut oil_rigs: Query<&mut PointAmount, With<OilRig>>,
) {
    for (point_transform, point, parent_rig, id) in points_transform.iter() {
        if let Some((_, _, mut player_score)) = circle_huds
            .iter_mut()
            .find(|(_, custom, _)| (custom.position.0 - point_transform.position).abs().x < 10.0 && (custom.position.0 - point_transform.position).abs().y < 10.0)
        {
            commands.get_entity(id).unwrap().despawn();
            let mut point_amount = oil_rigs.get_mut(parent_rig.0).unwrap();
            point_amount.remove(point.worth());

            player_score.add_to_score(point.worth() as u32);
            // info!("Player's score after adding {}: {}", player_score.get_score(), point.worth() as u32);
        }
    }
}


impl Point {
    const ALL: [Self; 3] = [Self::Barrel, Self::Coin, Self::Scrap];

    fn worth(&self) -> u16 {
        match self {
            Self::Barrel => 2,
            Self::Coin => 3,
            Self::Scrap => 1,
        }
    }
    fn file_name(&self) -> &'static str {
        match self {
            Self::Barrel => "barrel.png",
            Self::Coin => "coin.png",
            Self::Scrap => "scrap.png",
        }
    }
    fn custom_size() -> Vec2 {
        vec2(5.0, 5.0)
    }
}

impl PointAmount {
    /// generates a max point from default
    fn new(rng: &mut ThreadRng) -> Self {
        let max_point = rng.random_range(SPAWN_POINT_AMOUNT_MAX);

        PointAmount {
            points: 0,
            max_point,
        }
    }
    /// add given amount to points
    fn add(&mut self, points: u16) {
        self.points += points
    }
    /// remove given amount from self
    fn remove(&mut self, points: u16) {
        self.points -= points;
    }
    /// if exceeds max range
    fn is_max(&self) -> bool {
        self.points >= self.max_point
    }
}
