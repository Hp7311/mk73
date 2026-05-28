use std::time::Duration;
use std::{f32::consts::PI, ops::Range};
use std::sync::LazyLock;
use bevy::prelude::*;
use lightyear::link::server::Server;
use lightyear::prelude::{InterpolationTarget, NetworkTarget, Replicate, ServerMultiMessageSender};
use rand::{RngExt, rngs::ThreadRng, seq::IndexedRandom};

use common::{Boat, OCEAN_SURFACE, eq};
use common::collision::{out_of_bound_point, out_of_bounds, square_does_not_intersects};
use common::primitives::{CustomTransform, Mk48Rect, PlayerStats, Point, Radian, ZIndex, in_range};
use common::protocol::{OilRigTransform as OilRig, PointTransform, SendToClient};
use common::util::{avaliable_cords, point_in_square};
use common::WorldSize;

use common::BoatClientId;

/// Replicated for OilRig entity:
/// - [`OilRigInfo`]
/// 
/// Replicated for Point entity:
/// - [`PointTransform`]
pub struct OilRigPlugin;

// TODO randomly despawning rigs

impl Plugin for OilRigPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(RigTimer::new_rand(&mut rand::rng()))
            .add_systems(Update, spawn_rigs)
            .add_systems(
                FixedUpdate,
                (rig_spawn_points, move_points, points_obsorbed_despawn)
            );
    }
}
/// use [`rand::rng`] to determine whether to spawn a new [`Point`]
static SPAWN_POINT_VEC: LazyLock<Vec<bool>> = LazyLock::new(|| {
    #[cfg(debug_assertions)]
    let false_vec = [false; 4];
    #[cfg(not(debug_assertions))]
    let false_vec = [false; 40];
    false_vec.into_iter().chain([true]).collect::<Vec<bool>>()
});

/// the maximum radius around a rig which a point can spawn
const SPAWN_POINT_RADIUS_MAX: f32 = 100.0;

/// speed at which a point moves toward a ship's HUD center
const POINT_SPEED: f32 = 2.0;


#[derive(Resource, Deref, DerefMut)]
struct RigTimer(Timer);

#[cfg(debug_assertions)]
static mut DEBUG_SPAWN: bool = true;

/// spawns rig if the timer is reached, then setting the timer to a random val
fn spawn_rigs(
    mut timer: ResMut<RigTimer>,
    time: Res<Time>,

    mut commands: Commands,
    world_size: Single<&WorldSize>,
    spawned_rigs: Query<&PointTransform>
) {
    timer.tick(time.delta());

    if timer.is_finished() {
        let mut rng = rand::rng();

        spawn_random_rig(
            &mut commands,
            &mut rng,
            &world_size,
            &spawned_rigs.iter().map(|i| i.position).collect::<Vec<Vec2>>()
        );

        *timer = RigTimer::new_rand(&mut rng);
    }

    if unsafe { DEBUG_SPAWN } {
        commands.spawn((
            OilRig {
                position: vec2(0.0, 0.0),
                rotation: Radian::ZERO
            },
            PointAmount::new(&mut rand::rng()),
            Replicate::to_clients(NetworkTarget::All)
        ));

        unsafe { DEBUG_SPAWN = false; }
    }
}

/// spawns a must-valid rig at [`OCEAN_SURFACE`], returns the center of the spawned rig
/// 
/// ### Hangs
/// if there aren't space
/// 
/// ### Params
/// - `world_size`: the [`Single<WorldSize>`]
/// - `other_rigs`: all other rigs' centers
/// 
/// ### Spawns
/// - [`OilRigTransform`](OilRig) (consider changing name)
/// - [`PointAmount`]
/// - Replicated to all
/// 
/// uses [`SPRITE_SIZE`]
fn spawn_random_rig(
    commands: &mut Commands,
    rng: &mut ThreadRng,
    world_size: &WorldSize,
    other_rigs: &[Vec2]
) -> Vec2 {
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
            Mk48Rect::new(center, Vec2::splat(OilRig::SPRITE_SIZE)),
            Radian(rotation),
        ) {
            continue;
        }

        for &other_center in other_rigs {
            // roughly filter out those that may intersect
            if !square_does_not_intersects(center, OilRig::SPRITE_SIZE, other_center, OilRig::SPRITE_SIZE) {
                continue 'outer;
            }
        }

        break;
    }

    commands.spawn((
        OilRig {
            position: center,
            rotation: Radian(rotation)
        },
        PointAmount::new(rng),
        Replicate::to_clients(NetworkTarget::All)
    ));

    center
}

/// replicate [`PointTransform`]s to client which is equivalent of `CustomTransform` for Points
/// 
/// interpolation enabled
fn rig_spawn_points(
    mut commands: Commands,
    rigs: Query<(&mut PointAmount, &OilRig, Entity)>,
    world_size: Single<&WorldSize>,
) {
    let mut rng = rand::rng();

    for (mut point_amount, rig, id) in rigs {
        if point_amount.is_max() {
            continue;
        }

        if *SPAWN_POINT_VEC.choose(&mut rng).unwrap() {
            let cords = avaliable_cords(rig.position, OilRig::SPRITE_SIZE + SPAWN_POINT_RADIUS_MAX);

            let chosen_tile =  loop {
                let chosen = vec2(rng.random_range(cords.0.clone()), rng.random_range(cords.1.clone()));

                if point_in_square(chosen, OilRig::SPRITE_SIZE, rig.position)
                    || out_of_bound_point(&world_size, chosen)
                {
                    continue;
                }
                break chosen;
            };

            let &chosen_type = Point::VARIANTS.choose(&mut rng).unwrap();

            commands.spawn((
                chosen_type,
                ParentRig(id),

                PointTransform {
                    position: chosen_tile,
                    // default spawns on water surface
                    depth: OCEAN_SURFACE,
                    point: chosen_type
                },
                Replicate::to_clients(NetworkTarget::All),
                InterpolationTarget::to_clients(NetworkTarget::All)
            ));

            point_amount.add(chosen_type.worth());
        }
    }
}

/// move points toward ships that have a circle hud overlapping them
/// 
/// ignoring moving in the Z-axis
/// 
/// takes ~0.01 milliseconds to run once in a small map with 1 boat
fn move_points(
    mut points_transform: Query<&mut PointTransform, With<Point>>,
    boats: Query<(&CustomTransform, &Boat, &ZIndex)>,
) {
    for (boats_in_range, mut point) in points_transform.iter_mut().filter_map(|point_info| {
        let boats_in_range = boats.iter()
            .filter(|&(CustomTransform { position: boat_pos, ..}, boat, boat_depth)| {
                // info!(?boat_depth, ?point_info.depth);
                in_range(boat_pos.0, point_info.position, boat.circle_hud_radius())
                    // TODO points should "lock in" to a boat once it starts to dive
                    && eq!(*boat_depth, point_info.depth, ?precision = 0.05)
            })
            .map(|(CustomTransform { position, ..}, ..)| position.0)
            .collect::<Vec<_>>();

        if boats_in_range.is_empty() {
            None
        } else {
            Some((boats_in_range, point_info))
        }
    }) {
        // move the point toward player for those in 1 player's circle hud
        if boats_in_range.len() == 1 {
            point.position = point.position.move_towards(
                // safety: we know from the len check above
                *unsafe { boats_in_range.get_unchecked(0) },
                POINT_SPEED,
            );
            continue;
        }

        // safety: iterator won't run if boats empty
        let boat_position = unsafe { boats_in_range.iter().min_by_key(|&boat_position| {
            Vec2::distance_squared(point.position, *boat_position) as u32
        }).unwrap_unchecked() };

        point.position = point.position.move_towards(*boat_position, POINT_SPEED);
    }
}

/// increment player's score and despawning the Point if absorbed
fn points_obsorbed_despawn(
    mut commands: Commands,
    points_transform: Query<(&PointTransform, &Point, &ParentRig, Entity)>,
    mut boats: Query<(&CustomTransform, &ZIndex, &mut PlayerStats, &BoatClientId), With<Boat>>,
    mut point_amounts: Query<&mut PointAmount, With<OilRig>>,

    mut sender: ServerMultiMessageSender,
    server: Single<&Server>
) {
    for (point_transform, point, parent_rig, id) in points_transform.iter() {
        if let Some((mut player_stats, client_id)) = boats
            .iter_mut()
            .find(|&(custom, z_index, ..)| eq!(custom.position.extend(*z_index), point_transform.to_actual_translation(), ?vec3))
            .map(|(_, _, stats, client_id)| (stats, client_id))
        {
            commands.get_entity(id).unwrap().despawn();

            player_stats.add_to_score(point.worth().into());
            
            // client spawns UI and collects user input
            // TODO is this pointless? we're doing this to avoid checking display() every frame on client
            sender.send::<_, SendToClient>(
                &player_stats.display(),
                &server,
                &NetworkTarget::Single(client_id.0)
            ).unwrap();

            let mut point_amount = point_amounts.get_mut(parent_rig.0).unwrap();
            point_amount.remove(point.worth());
        }
    }
}


#[derive(Component, Debug, Clone)]
struct ParentRig(Entity);

/// holding the amount of points
#[derive(Component, Debug, Clone, Copy)]
struct PointAmount {
    points: u16,
    max_point: u16,
}


impl PointAmount {
    /// maximum amount of points a rig can spawn
    const SPAWN_POINT_AMOUNT_MAX: Range<u16> = 30..40;
    /// generates a max point from default
    fn new(rng: &mut ThreadRng) -> Self {
        let max_point = rng.random_range(Self::SPAWN_POINT_AMOUNT_MAX);

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

impl RigTimer {
    const DURATION_RANGE: Range<Duration> = Duration::from_secs(10)..Duration::from_secs(120);
    /// random duration with [`TimerMode::Once`]
    fn new_rand(rng: &mut ThreadRng) -> Self {
        Self(Timer::new(rng.random_range(Self::DURATION_RANGE), TimerMode::Once))
    }
}