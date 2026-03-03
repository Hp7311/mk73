//! functions related to Oil Rigs

use std::{f32::consts::PI, ops::Range};

use bevy::prelude::*;
use enum_dispatch::enum_dispatch;
use rand::{RngExt, rngs::ThreadRng, seq::IndexedRandom};

use crate::{DEFAULT_SPRITE_SHRINK, collision::{out_of_bound_no_rotation, out_of_bounds}, primitives::{DecimalPoint, Dimensions, RectIntersect, WidthHeight}, ship::CircleHud, util::{fill_dimensions, point_in_square, resize_inner, tiles_around_point}, world::WorldSize};

pub struct OilRigPlugin;

impl Plugin for OilRigPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, (
                resize_rigs,
                validate_rigs,
                rig_spawn_points,
                move_points,
                despawn_points
            ).chain());
    }
}

#[derive(Component, Debug, Copy, Clone)]
struct OilRig;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let world_size = WorldSize::default().0;
    let mut rng = rand::rng();
    let oil_rig = asset_server.load("oil_platform.png".to_owned());

    for _ in 0..10 {  // temporary
        let rotation = rng.random_range(-PI..PI);
        let x = rng.random_range(-world_size.width.round() as i32 / 2..world_size.width.round() as i32 / 2) as f32;
        let y = rng.random_range(-world_size.height.round() as i32 / 2..world_size.height.round() as i32 / 2) as f32;
        
        commands.spawn((
            Transform::from_translation(vec3(x, y, 0.0))
                .with_rotation(Quat::from_rotation_z(rotation)),
            Sprite {
                image: oil_rig.clone(),
                ..default()
            },
            Dimensions(None),
            PointAmount::new(&mut rng),
            OilRig,
        ));  // TODO bundle it
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
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Barrel;
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Coin;
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Scrap;

impl PointData for Coin {
    fn worth(&self) -> u16 {
        3
    }
}
impl PointData for Barrel {
    fn worth(&self) -> u16 {
        2
    }
}
impl PointData for Scrap {
    fn worth(&self) -> u16 {
        1
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

        PointAmount { points: 0, max_point }
    }
    /// add given amount to points
    fn add(&mut self, points: u16) {
        self.points += points
    }
    /// if exceeds max range
    fn is_max(&self) -> bool {
        self.points >= self.max_point
    }
}

fn resize_rigs(
    mut queries: ParamSet<(
        Query<&mut Sprite, With<OilRig>>,
        Query<(&Sprite, &mut Dimensions), With<OilRig>>
    )>,
    assets: Res<Assets<Image>>
) {
    resize_inner(queries.p0(), &assets);
    fill_dimensions(queries.p1(), &assets);
}

/// despawn rigs that intersect with another rig
fn validate_rigs(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    sprites: Query<(&Dimensions, &Transform, Entity), With<OilRig>>,
    world_size: Single<&WorldSize>
) {
    let mut rigs = vec![];
    
    for (dimension, transform, id) in sprites.iter().filter(|(d, ..)| d.0.is_some()) {
        let WidthHeight { width, height } = dimension.0.unwrap();

        rigs.push((
            WidthHeight {
                width,
                height,
            }.to_rect(transform.translation.xy()),
            transform.rotation,
            id
        ));
    }

    let despawning = validate_rig_raw(rigs, *world_size);
    for id in despawning.iter() {
        commands.get_entity(*id).unwrap()
            .despawn();
    }

    let mut rng = rand::rng();
    let oil_rig = asset_server.load("oil_platform.png");
    for _ in 0..despawning.len() {
        let rotation = rng.random_range(-PI..PI);
        let x = rng.random_range(-world_size.0.width.round() as i32 / 2..world_size.0.width.round() as i32 / 2) as f32;
        let y = rng.random_range(-world_size.0.height.round() as i32 / 2..world_size.0.height.round() as i32 / 2) as f32;
        
        commands.spawn((
            Transform::from_translation(vec3(x, y, 0.0))
                .with_rotation(Quat::from_rotation_z(rotation)),
            Sprite {
                image: oil_rig.clone(),
                ..default()
            },
            Dimensions(None),
            PointAmount::new(&mut rng),
            OilRig,
        ));
        // here, we don't need to validate again because the systems is run every Update, so next frame will call again
    }
}

/// returns vector of Entities to despawn
fn validate_rig_raw(rigs: Vec<(Rect, Quat, Entity)>, world_size: &WorldSize) -> Vec<Entity> {
    let mut despawning_id = vec![];
    for (rect, rotation, id) in rigs.iter() {
        if rigs.iter()
            .filter(|(target, ..)| target != rect)
            .any(|(target, ..)| rect.intersects_with(target))
            || out_of_bounds(&world_size, rect.size().into(), rect.center(), *rotation)
        {
            despawning_id.push(*id);
        }
        // TODO create a Rect-like structure instead of operating with WidthHeight and Vec2 etc.
    }

    despawning_id
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
    mut transforms: Query<(&mut PointAmount, &Transform, &Sprite), With<OilRig>>,
    world_size: Single<&WorldSize>
) {
    let point_sprites: [(Point, _); 3] = [
        (Coin.into(), asset_server.load("coin.png")),
        (Barrel.into(), asset_server.load("barrel.png")),
        (Scrap.into(), asset_server.load("scrap.png"))
    ];

    for (mut point_amount, transform, sprite) in transforms.iter_mut() {
        let Some(sprite_size) = sprite.custom_size else { continue };

        let avaliable_tiles = tiles_around_point(
            transform.translation.xy(),
            sprite_size.x + SPAWN_POINT_RADIUS_MAX
        );
        let avaliable_tiles: Vec<_> = avaliable_tiles
            .iter()
            .filter(|&tile| !point_in_square(*tile, sprite_size.x, transform.translation.xy()))
            .filter(|&tile| !out_of_bound_no_rotation(&world_size, WidthHeight::ZERO, tile))
            .collect();

        if point_amount.is_max() {
            continue;
        }

        let mut rng = rand::rng();
        let mut spawn_p = vec![false; rng.random_range(SPAWN_POINT_SPRITE_P)];
        spawn_p.push(true);

        if *spawn_p.choose(&mut rng).unwrap() {
            let (chosen_type, chosen_sprite) = point_sprites.choose(&mut rng).unwrap().clone();
            commands.spawn((
                Sprite::from_image(chosen_sprite),
                Transform {
                    translation: avaliable_tiles.choose(&mut rng).unwrap().extend(0.0),
                    scale: Vec2::splat(DEFAULT_SPRITE_SHRINK.powi(2)).extend(0.0),
                    ..default()
                },
                chosen_type
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
    for (intersect_huds, mut transform) in points_transform
        .iter_mut()
        .filter_map(|transform| {
            let huds_in_point = circle_huds
                .iter()
                .filter(|hud| hud.contains(transform.translation.xy()))
                .collect::<Vec<_>>();

            if huds_in_point.is_empty() {
                None
            } else {
                Some((huds_in_point, transform))
            }
        })
    {
        // move the point toward player for those in 1 player's circle hud
        if intersect_huds.len() == 1 {
            transform.translation = transform.translation.move_towards(
                intersect_huds.first().unwrap()
                    .center
                    .extend(0.0),
                POINT_SPEED
            );
            continue;
        }

        // calculate the distance and make the point go to the nearest ship
        let Some(closest_hud) = intersect_huds.iter()
            .min_by(|a, b| {
                let a_distance = transform.translation.distance_squared(a.center.extend(0.0));
                let b_distance = transform.translation.distance_squared(b.center.extend(0.0));
                a_distance.total_cmp(&b_distance)
            }) else { return };

        transform.translation = transform.translation.move_towards(
            closest_hud.center.extend(0.0),
            POINT_SPEED
        );
    }
}

// TODO add to player's score
fn despawn_points(
    mut commands: Commands,
    points_transform: Query<(&Transform, Entity), With<Point>>,
    circle_huds: Query<&CircleHud>,
) {
    for (point_transform, id) in points_transform {
        if circle_huds.iter().any(|hud| hud.at_center(point_transform.translation.xy(), DecimalPoint::Zero)) {
            commands.get_entity(id).unwrap().despawn();
        }
    }
}