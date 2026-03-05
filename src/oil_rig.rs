//! functions related to Oil Rigs

use std::{f32::consts::PI, ops::Range};

use bevy::prelude::*;
use enum_dispatch::enum_dispatch;
use rand::{RngExt, rngs::ThreadRng, seq::IndexedRandom};

use crate::{
    DEFAULT_SPRITE_SHRINK, WATER_SURFACE,
    boat::{CircleHud, PlayerScore},
    collision::{out_of_bound_no_rotation, out_of_bounds, square_does_not_intersects},
    primitives::{DecimalPoint, Validated, WidthHeight},
    util::{point_in_square, tiles_around_point},
    world::WorldSize,
};

pub struct OilRigPlugin;

impl Plugin for OilRigPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup).add_systems(
            Update,
            (
                rig_spawn_points,
                move_points,
                points_obsorbed_despawn,
            )
                .chain(),
        );
    }
}

const SPRITE_SIZE: Vec2 = Vec2::splat(1024.0);

#[derive(Component, Debug, Copy, Clone)]
struct OilRig;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let world_size = WorldSize::default();
    let mut rng = rand::rng();
    let oil_rig_image = asset_server.load("oil_platform.png".to_owned());

    let mut spawned_rigs = vec![];
    for _ in 0..10 {
        // temporary
        spawned_rigs.push(spawn_random_rig(
            commands.reborrow(),
            &mut rng,
            &world_size,
            oil_rig_image.clone(),
            &spawned_rigs,
            SPRITE_SIZE * DEFAULT_SPRITE_SHRINK
        ));
    }
}

#[enum_dispatch]
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// an entity that provide an amount of points
enum Point {
    Barrel,
    Coin,
    Scrap,
}

#[enum_dispatch(Point)]
trait PointData {
    fn worth(&self) -> u16;
    fn get_parent_rig(&self) -> Option<Entity>;
    fn fill_spawned_by(&mut self, spawned_by: Entity);
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
struct Barrel {
    spawned_by: Option<Entity>,
}
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
struct Coin {
    spawned_by: Option<Entity>,
}
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
struct Scrap {
    spawned_by: Option<Entity>,
}

impl PointData for Coin {
    fn worth(&self) -> u16 {
        3
    }
    fn get_parent_rig(&self) -> Option<Entity> {
        self.spawned_by
    }
    fn fill_spawned_by(&mut self, spawned_by: Entity) {
        self.spawned_by = Some(spawned_by);
    }
}
impl PointData for Barrel {
    fn worth(&self) -> u16 {
        2
    }
    fn get_parent_rig(&self) -> Option<Entity> {
        self.spawned_by
    }
    fn fill_spawned_by(&mut self, spawned_by: Entity) {
        self.spawned_by = Some(spawned_by);
    }
}
impl PointData for Scrap {
    fn worth(&self) -> u16 {
        1
    }
    fn get_parent_rig(&self) -> Option<Entity> {
        self.spawned_by
    }
    fn fill_spawned_by(&mut self, spawned_by: Entity) {
        self.spawned_by = Some(spawned_by);
    }
}

/// holding the amount of points
#[derive(Component, Debug, Clone, Copy)]
struct PointAmount {
    points: u16,
    max_point: u16,
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


/// maximum amount of points a rig can spawn
const SPAWN_POINT_AMOUNT_MAX: Range<u16> = 30..40;
/// spawns a point around a rig every x-y seconds
#[cfg(debug_assertions)]
const SPAWN_POINT_SPRITE_P: Range<usize> = 0..2;
#[cfg(not(debug_assertions))]
const SPAWN_POINT_SPRITE_P: Range<usize> = 15 * 60..30 * 60;

/// the maximum radius around a rig which a point can spawn
const SPAWN_POINT_RADIUS_MAX: f32 = 100.0;

/// speed at which a point moves toward a ship's HUD center
const POINT_SPEED: f32 = 2.0;

fn rig_spawn_points(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut transforms: Query<(&mut PointAmount, &Transform, &Sprite, Entity), With<OilRig>>,
    world_size: Single<&WorldSize>,
) {
    let point_sprites: [(Point, Handle<Image>); 3] = [
        (Coin::default().into(), asset_server.load("coin.png")),
        (Barrel::default().into(), asset_server.load("barrel.png")),
        (Scrap::default().into(), asset_server.load("scrap.png")),
    ];

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
            .filter(|&tile| !out_of_bound_no_rotation(&world_size, WidthHeight::ZERO, tile))
            .collect();

        let mut rng = rand::rng();
        let mut spawn_p = vec![false; rng.random_range(SPAWN_POINT_SPRITE_P)];
        spawn_p.push(true);

        if *spawn_p.choose(&mut rng).unwrap() {
            let (mut chosen_type, chosen_sprite) = point_sprites.choose(&mut rng).unwrap().clone();
            chosen_type.fill_spawned_by(id);
            let chosen_tile = avaliable_tiles.choose(&mut rng).unwrap();

            commands.spawn((
                Sprite::from_image(chosen_sprite),
                Transform {
                    translation: chosen_tile.extend(WATER_SURFACE),
                    scale: Vec2::splat(DEFAULT_SPRITE_SHRINK.powi(2)).extend(0.0),
                    ..default()
                },
                chosen_type,
            ));

            point_amount.add(chosen_type.worth());
        }
    }
}

/// move points toward ships that have a CircleHud overlapping them
fn move_points(
    mut points_transform: Query<&mut Transform, With<Point>>,
    circle_huds: Query<&CircleHud>,
) {
    for (intersect_huds, mut transform) in points_transform.iter_mut().filter_map(|transform| {
        let huds_in_point = circle_huds
            .iter()
            .filter(|hud| hud.contains(transform.translation.xy()))
            .collect::<Vec<_>>();

        if huds_in_point.is_empty() {
            None
        } else {
            Some((huds_in_point, transform))
        }
    }) {
        // move the point toward player for those in 1 player's circle hud
        if intersect_huds.len() == 1 {
            transform.translation = transform.translation.move_towards(
                intersect_huds.first().unwrap().center.extend(WATER_SURFACE),
                POINT_SPEED,
            );
            continue;
        }

        // calculate the distance and make the point go to the nearest ship
        let Some(closest_hud) = intersect_huds.iter().min_by(|a, b| {
            let a_distance = transform
                .translation
                .distance_squared(a.center.extend(WATER_SURFACE));
            let b_distance = transform
                .translation
                .distance_squared(b.center.extend(WATER_SURFACE));
            a_distance.total_cmp(&b_distance)
        }) else {
            return;
        };

        transform.translation = transform
            .translation
            .move_towards(closest_hud.center.extend(WATER_SURFACE), POINT_SPEED);
    }
}

fn points_obsorbed_despawn(
    mut commands: Commands,
    points_transform: Query<(&Transform, &Point, Entity)>,
    circle_huds: Query<&CircleHud>,
    mut oil_rigs: Query<&mut PointAmount, With<OilRig>>,
    mut player_score: ResMut<PlayerScore>,
) {
    for (point_transform, point, id) in points_transform.iter() {
        if circle_huds
            .iter()
            .any(|hud| hud.at_center(point_transform.translation.xy(), DecimalPoint::Zero))
        {
            commands.get_entity(id).unwrap().despawn();
            let mut point_amount = oil_rigs.get_mut(point.get_parent_rig().unwrap()).unwrap();

            point_amount.remove(point.worth());

            player_score.add_to_score(point.worth().into());
        }
    }
}

#[derive(Bundle, Debug, Clone)]
struct OilRigBundle {
    sprite: Sprite,
    point_amount: PointAmount,
    oil_rig: OilRig,
    validated: Validated,
}

impl OilRigBundle {
    fn new(sprite: Sprite, rng: &mut ThreadRng) -> Self {

        OilRigBundle {
            sprite,
            point_amount: PointAmount::new(rng),
            oil_rig: OilRig,
            validated: Validated(false),
        }
    }
}

struct RigInfo {
    center: Vec2,
    width: f32,
}


/// spawns a must-valid rig, returns the dimensions and Transform of the spawned rig
/// ### Panics
/// assumes that the rig is a square
/// ### Hangs
/// if there aren't space
fn spawn_random_rig(
    mut commands: Commands,
    rng: &mut ThreadRng,
    world_size: &WorldSize,
    image: Handle<Image>,
    other_rigs: &[(Vec2, Transform)],
    rig_dimensions: Vec2
) -> (Vec2, Transform) {
    assert!((rig_dimensions.x - rig_dimensions.y).abs() < 0.001);
    let sprite = Sprite {
        image,
        custom_size: Some(rig_dimensions),
        ..default()
    };
    let mut rotation;
    let mut x;
    let mut y;

    'outer: loop {
        rotation = rng.random_range(-PI..PI);
        x = rng.random_range(-world_size.0.width / 2.0..world_size.0.width / 2.0);
        y = rng.random_range(-world_size.0.height / 2.0..world_size.0.height / 2.0);
        if out_of_bounds(
            world_size,
            rig_dimensions.into(),
            vec2(x, y),
            Quat::from_rotation_z(rotation),
        ) {
            continue;
        }
        let rig = RigInfo {
            center: vec2(x, y),
            width: rig_dimensions.x,
        };

        for (dimension, transform) in other_rigs {
            let other = RigInfo {
                center: transform.translation.xy(),
                width: dimension.x,
            };

            if !square_does_not_intersects(rig.center, rig.width, other.center, other.width) {
                continue 'outer;
            }
        }

        break;
    }

    let spawn_transform = Transform {
        translation: vec3(x, y, WATER_SURFACE),
        rotation: Quat::from_rotation_z(rotation),
        ..default()
    };
    commands.spawn((
        spawn_transform,
        OilRigBundle::new(sprite, rng),
    ));

    (
        rig_dimensions,
        spawn_transform
    )
}
