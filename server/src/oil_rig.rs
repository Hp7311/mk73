//! functions related to Oil Rigs

use std::{f32::consts::PI, ops::Range};

use bevy::prelude::*;
use lightyear::prelude::{MessageManager, MessageReceiver, MessageSender, NetworkTarget, Replicate};
use rand::{RngExt, rngs::ThreadRng, seq::IndexedRandom};

use common::{eq, print_num, OCEAN_SURFACE};
use common::collision::{out_of_bound_point, out_of_bounds, square_does_not_intersects};
use common::primitives::{CircleHud, DecimalPoint, MkRect, Radian, WidthHeight};
use common::protocol::{OilRigInfo, OilRigMessage, PlayerScore, SendToClient};
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
        app.add_observer(spawn_rigs);  // TODO use system sets
            // .add_systems(
            //     Update,
            //     (rig_spawn_points, move_points, points_obsorbed_despawn).chain(),
            // );
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
            MkRect {
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
    asset_server: Res<AssetServer>,
    mut transforms: Query<(&mut PointAmount, &Transform, &Sprite, Entity), With<OilRig>>,
    world_size: Single<&WorldSize>,
) {
    use Point as P;
    let point_sprites: [(Point, Handle<Image>); 3] = [
        (P::Coin, asset_server.load(P::Coin.file_name())),
        (P::Barrel, asset_server.load(P::Barrel.file_name())),
        (P::Scrap, asset_server.load(P::Scrap.file_name())),
    ]; // load sprites early for performance

    for (mut point_amount, transform, sprite, id) in transforms.iter_mut() {
        let Some(sprite_size) = sprite.custom_size else {
            continue;
        };
        if point_amount.is_max() {
            continue;
        }

        let avaliable_tiles = tiles_around_point(
            transform.translation.xy(),
            sprite_size.x + SPAWN_POINT_RADIUS_MAX,
        );

        let avaliable_tiles: Vec<_> = avaliable_tiles
            .iter()
            .filter(|&tile| !point_in_square(*tile, sprite_size.x, transform.translation.xy()))
            .filter(|&tile| {
                !out_of_bound_point(
                    // okay to not use with rotation outofbound because how small a point is
                    &world_size,
                    MkRect {
                        center: *tile,
                        dimensions: WidthHeight::ZERO,
                    },
                )
            })
            .collect();

        let mut rng = rand::rng();
        let mut spawn_p = vec![false; rng.random_range(SPAWN_POINT_SPRITE_P)];
        spawn_p.push(true);

        if *spawn_p.choose(&mut rng).unwrap() {
            let (chosen_type, chosen_sprite) = point_sprites.choose(&mut rng).unwrap();
            let chosen_tile = avaliable_tiles.choose(&mut rng).unwrap();  // TODO not efficient

            commands.spawn((
                Sprite {
                    image: chosen_sprite.clone(),
                    custom_size: Some(todo!()),
                    ..default()
                },
                Transform {
                    translation: chosen_tile.extend(OCEAN_SURFACE),
                    ..default()
                },
                *chosen_type,
                ParentRig(id),
            ));

            point_amount.add(chosen_type.worth());
        }
    }
}

/// move points toward ships that have a CircleHud overlapping them
fn move_points(
    mut points_transform: Query<&mut Transform, With<Point>>,
    circle_huds: Query<(&CircleHud, &Transform)>,
) {
    for (intersect_huds, mut transform) in points_transform.iter_mut().filter_map(|point_tf| {
        let huds_in_point: Vec<(f32, Vec2)> = circle_huds
            .iter()
            .filter(|(hud, hud_tf)| hud.contains(hud_tf.translation.xy(), point_tf.translation.xy()))
            .map(|(hud, tf)| (hud.radius, tf.translation.xy()))
            .collect();
        if huds_in_point.is_empty() {
            None
        } else {
            Some((huds_in_point, point_tf))
        }
    }) {
        // move the point toward player for those in 1 player's circle hud
        if intersect_huds.len() == 1 {
            transform.translation = transform.translation.move_towards(
                intersect_huds.first().unwrap().1.extend(OCEAN_SURFACE),
                POINT_SPEED,
            );
            continue;
        }

        // calculate the distance and make the point go to the nearest ship
        let Some((_, hud_transform)) = intersect_huds.iter().min_by_key(|(_, hud_tf)| {
            transform
                .translation
                .distance_squared(hud_tf.extend(OCEAN_SURFACE))
                .round() as u64  // pixles are too small to be noticeable
        }) else {
            return;
        };

        transform.translation = transform
            .translation
            .move_towards(hud_transform.extend(OCEAN_SURFACE), POINT_SPEED);
    }
}

/// increment player's score and despawning the Point if absorbed
fn points_obsorbed_despawn(
    mut commands: Commands,
    points_transform: Query<(&Transform, &Point, &ParentRig, Entity)>,
    circle_huds: Query<(&CircleHud, &Transform)>,
    mut oil_rigs: Query<&mut PointAmount, With<OilRig>>,
    mut player_score: ResMut<PlayerScore>,
) {
    for (point_transform, point, parent_rig, id) in points_transform.iter() {
        if circle_huds
            .iter()
            .any(|(_, tf)| CircleHud::at_center(tf.translation.xy(), point_transform.translation.xy(), DecimalPoint::Zero))
        {
            commands.get_entity(id).unwrap().despawn();
            let mut point_amount = oil_rigs.get_mut(parent_rig.0).unwrap();
            point_amount.remove(point.worth());

            player_score.add_to_score(point.worth() as u32);
        }
    }
}


impl Point {
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
