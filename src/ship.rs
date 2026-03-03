//! currently, there are no differentiation between a Ship and a Submarine
//!
//! be mindful of [`Ship::transform`] and [`Transform`] of the [`Ship`] needs to be kept in sync

// doc outdated

use std::f32::consts::PI;
use std::ops::Range;

use bevy::camera_controller::pan_camera::PanCamera;
use bevy::color::palettes::css::*;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use rand::RngExt;
use rand::seq::IndexedRandom;

use crate::constants::*;
use crate::primitives::*;
use crate::util::out_of_bound_no_rotation;
use crate::util::out_of_bounds;
use crate::util::point_in_square;
use crate::util::tiles_around_point;
use crate::util::{
    MainCamera,
    add_circle_hud, calculate_from_proportion, get_cursor_pos, get_rotate_radian,
    move_with_rotation,
    TrimRadian
};

#[derive(Component, Debug, Copy, Clone)]
pub struct Ship;

pub const WORLD_MIN: Vec2 = vec2(4000.0, 2000.0);
pub const WORLD_EXPAND: f32 = 2000.0;

/// absolute value of minimum radians that must be reached to reverse the Ship
const MINIMUM_REVERSE: f32 = PI * (2.0 / 3.0);

// TODO time to modulise
// TODO constants for Z-ordering
pub fn startup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn((
        Camera2d,
        PanCamera {
            min_zoom: 1.0,
            max_zoom: DEFAULT_MAX_ZOOM,
            key_down: None,
            key_left: None,
            key_right: None,
            key_up: None,
            ..default()
        },
        MainCamera,
    ));

    let radius = add_circle_hud(YASEN_RAW_SIZE / 2.0) * DEFAULT_SPRITE_SHRINK;
    commands.spawn((
        ShipBundle::new(
            YASEN_MAX_SPEED,
            YASEN_BACK_SPEED,
            YASEN_ACCELERATION,
            vec2(100.0, 0.0),
            "yasen.png",
            asset_server.clone(),
            YASEN_RAW_SIZE / 2.0,
        ),
        Ship,
    ))
    .with_children(|parent | {
        parent.spawn((
            CircleHudBundle {
                mesh: Mesh2d(meshes.add(Circle::new(radius)
                        .to_ring(3.0),
                )),
                materials: MeshMaterial2d(materials.add(ColorMaterial::from_color(GRAY))),
            },
            Transform::from_translation(vec3(0.0, 0.0, 30.0)),  // relative to parent, circle hud highest Z
            CircleHud { radius, center: vec2(100.0, 0.0) }
        ));
    });

    // in Sprites, translation is the center point of the Sprite
    let world_size = WorldSize::default().0;
    commands.spawn((
        Transform {
            translation: Vec3 {
                z: -1.0,
                ..default()
            },
            ..default()
        },
        Sprite {
            image: asset_server.load("waves.png"),
            color: Color::srgb(0.0, 0.65, 1.03),
            custom_size: Some(world_size.to_vec2()),
            image_mode: SpriteImageMode::Tiled {
                tile_x: true,
                tile_y: true,
                stretch_value: 2.0,
            },
            ..default()
        },
        Background,
    ));

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
        ));
    }

    commands.spawn(WorldSize(world_size));
}

pub fn move_camera(
    mut camera: Single<&mut Transform, With<MainCamera>>,
    ship_pos: Query<&CustomTransform, With<Ship>>,
) {
    // currently ignores possibility of multiple ships
    let Some(ship) = ship_pos.iter().last() else {
        return;
    };

    if ship.position.0 != camera.translation.xy() {
        camera.translation = ship.position.0.extend(0.0);
    }
}

/// modifys [`Transform`] of [`Ship`]
pub fn update_ship(
    buttons: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut queries: ParamSet<(
        Query<(&Transform, &mut CustomTransform, &Radian, &mut TargetRotation, &mut ReleasedAfterReverse), With<Ship>>,
        Query<(&Transform, &mut CustomTransform, &Radian, &TargetRotation, &TargetSpeed, &Acceleration), With<Ship>>,
        Query<(&Transform, &mut CustomTransform, &Radius, &MaxSpeed, &ReverseSpeed, &Acceleration, &mut TargetSpeed), With<Ship>>,
        Query<(&CustomTransform, &mut ReleasedAfterReverse), With<Ship>>
    )>
) {
    if let Some(cursor_pos) = get_cursor_pos(&window, &camera)
        && buttons.pressed(MouseButton::Left)
    {
        rotate_ship(&mut queries.p0(), cursor_pos);
        move_ship(&mut queries.p2(), cursor_pos);
    } else {
        ship_to_target(&mut queries.p1());
    }
    if get_cursor_pos(&window, &camera).is_some()
        && buttons.just_released(MouseButton::Left)
    {
        try_release_after_rev(&mut queries.p3())
    }
}

/// handle rotation
fn rotate_ship(
    transforms: &mut Query<(&Transform, &mut CustomTransform, &Radian, &mut TargetRotation, &mut ReleasedAfterReverse), With<Ship>>,
    cursor_pos: Vec2,
) {
    for (transform, mut custom_transform, max_turn, mut target_rotation, mut released_after_reverse) in transforms.iter_mut() {

        let raw_moved = get_rotate_radian(cursor_pos, transform.translation.xy());  // diff from radian 0
        let (.., current_rotation) = transform.rotation.to_euler(EulerRot::XYZ);
        let mut target_move = raw_moved;

        let moved = {  // radians to move from current rotation
            let mut moved_from_current = (raw_moved.to_degrees() - current_rotation.to_degrees())
                .to_radians()
                .trim();

            // if reversing, adjust return value
            if moved_from_current.abs() > MINIMUM_REVERSE {
                custom_transform.reversed = true;
                moved_from_current = moved_from_current.flip();
                target_move = target_move.flip()

            } else if custom_transform.reversed && released_after_reverse.0 {  // free to forward again
                custom_transform.reversed = false;
                released_after_reverse.0 = false;  // reset. setting to true is done in `try_release_after_rev`
            } else if custom_transform.reversed {  // unable to go forward, haven't released key yet
                moved_from_current = moved_from_current.flip();
                target_move = target_move.flip()
            }

            moved_from_current
        };

        // turning degree bigger than maximum
        if moved.abs() > max_turn.0 {
            let ship_max_turn = max_turn.0;
            if moved > 0.0 {
                custom_transform.rotate_local_z(ship_max_turn.to_radian_unchecked());
            } else if moved < 0.0 {
                custom_transform.rotate_local_z(-ship_max_turn.to_radian_unchecked());
            }
        } else { // normal
            custom_transform.rotate_local_z(moved.to_radian_unchecked());
        }

        
        *target_rotation = Some(target_move).into();
    }
}

fn try_release_after_rev(query: &mut Query<(&CustomTransform, &mut ReleasedAfterReverse), With<Ship>>) {
    for (CustomTransform { reversed, ..}, mut release) in query {
        if !reversed { continue; }

        release.0 = true;
    }
}

/// handle moving
fn move_ship(
    datas: &mut Query<(&Transform, &mut CustomTransform, &Radius, &MaxSpeed, &ReverseSpeed, &Acceleration, &mut TargetSpeed), With<Ship>>,
    cursor_pos: Vec2,
) {
    for (transform, mut custom_transform, radius, max_speed, reverse_speed, acceleration, mut target_speed) in datas.iter_mut() {
        let cursor_distance = cursor_pos.distance(transform.translation.xy());
        let speed = if custom_transform.reversed {
            reverse_speed.0
        } else {
            max_speed.0
        };

        let mut speed = calculate_from_proportion(
            cursor_distance,
            add_circle_hud(radius.default_convert().0),
            speed,
            radius.default_convert().0,
        );

        target_speed.0 = speed;

        // adjust for acceleration
        let speed_diff = speed - custom_transform.speed.0;
        if speed_diff > acceleration.0 {
            speed = custom_transform.speed.0 + acceleration.0;
        } else if speed_diff < -acceleration.0 {
            speed = custom_transform.speed.0 - acceleration.0;
        }

        custom_transform.speed = Speed(speed);
    }
}

// note that we're accepting Query instead of Single for ship everywhere
// and not descriminating Bot/Player

/// remember the last move angle and rotate toward it when button not pressed
fn ship_to_target(ships: &mut Query<(&Transform, &mut CustomTransform, &Radian, &TargetRotation, &TargetSpeed, &Acceleration), With<Ship>>) {
    for (transform, mut custom_transform, max_turn, target_rotation, target_speed, acceleration) in ships {
        // ------ rotation
        let Some(target_rotation) = target_rotation.0 else { continue; };

        let (.., current_rotation) = transform.rotation.to_euler(EulerRot::XYZ);

        let moved = (target_rotation.to_degrees() - current_rotation.to_degrees())
            .to_radians()
            .trim();

        if moved.abs() > max_turn.0 {
            let ship_max_turn = max_turn.0;
            if moved > 0.0 {
                custom_transform.rotate_local_z(ship_max_turn.to_radian_unchecked());
            } else if moved < 0.0 {
                custom_transform.rotate_local_z(-ship_max_turn.to_radian_unchecked());
            }
        } else {
            custom_transform.rotate_local_z(moved.to_radian_unchecked());
        }
        // ------ speed
        let speed_diff = target_speed.0 - custom_transform.speed.0;
        if speed_diff > acceleration.0 {
            custom_transform.speed.0 = custom_transform.speed.0 + acceleration.0;
        } else if speed_diff < -acceleration.0 {
            custom_transform.speed.0 = custom_transform.speed.0 - acceleration.0;
        }
    }
}

/// updates [`Ship`]'s [`Transform`] according to its [`CustomTransform`]
pub fn update_transform(
    mut transform_ship: Query<(&mut Transform, &mut CustomTransform, &Children, &Dimensions), With<Ship>>,
    mut circle_huds: Query<&mut CircleHud>,
    world_size: Single<&WorldSize>,
) {
    for (mut transform, mut custom, children, dimension) in transform_ship.iter_mut().filter(|(.., dimension)| dimension.0.is_some()) {
        let mut translation = custom.position.to_vec3();
        if custom.reversed {
            translation += move_with_rotation(transform.rotation, -custom.speed.0);
        } else {
            translation += move_with_rotation(transform.rotation, custom.speed.0);  // ignores frame lagging temporary
        }

        
        if out_of_bounds(&world_size, dimension.0.unwrap(), translation.xy(), custom.rotation.to_quat()) {
            println!("Out of bounds");
            return;
        }
        let target = Transform {
            translation,
            rotation: custom.rotation.to_quat(),
            scale: Vec3::ONE,
        };
        *transform = target;

        // sync position
        custom.position = Position(translation.xy());

        for child in children {
            if let Ok(mut hud) = circle_huds.get_mut(*child) {
                hud.center = translation.xy();
                break;
            }
        }
    }
}


pub fn resize_ship(
    mut queries: ParamSet<(
        Query<&mut Sprite, With<Ship>>,
        Query<(&Sprite, &mut Dimensions), With<Ship>>
    )>,
    assets: Res<Assets<Image>>
) {
    resize_inner(queries.p0(), &assets);
    fill_dimensions(queries.p1(), &assets);
}

pub fn resize_rigs(
    mut queries: ParamSet<(
        Query<&mut Sprite, With<OilRig>>,
        Query<(&Sprite, &mut Dimensions), With<OilRig>>
    )>,
    assets: Res<Assets<Image>>
) {
    resize_inner(queries.p0(), &assets);
    fill_dimensions(queries.p1(), &assets);
}

/// resize [`Sprite`]s by default constant
fn resize_inner<T: Component>(mut sprites: Query<&mut Sprite, With<T>>, assets: &Res<Assets<Image>>) {
    for mut sprite in sprites.iter_mut() {

        let Some(image) = assets.get(&sprite.image) else {
            continue;
        };
        if sprite.custom_size.is_some() {
            continue;
        }

        sprite.custom_size = Some(vec2(
            image.width() as f32 * DEFAULT_SPRITE_SHRINK,
            image.height() as f32 * DEFAULT_SPRITE_SHRINK,
        ));
    }
}

/// fill in the dimensions of a Sprite using it's `custom_size`
fn fill_dimensions<T: Component>(mut query: Query<(&Sprite, &mut Dimensions), With<T>>, images: &Res<Assets<Image>>) {
    for (sprite, mut dimension) in query.iter_mut() {
        if images.get(&sprite.image).is_none() {
            continue;
        };
        let Some(size) = sprite.custom_size else {
            continue;
        };

        *dimension = Dimensions(Some(WidthHeight {
            width: size.x,
            height: size.y
        }));
    }
}

/// despawn rigs that intersect with another rig
pub fn validate_rigs(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    sprites: Query<(&Dimensions, &Transform, Entity), With<OilRig>>,
    world_size: Single<&WorldSize>
) {
    let mut rigs = vec![];
    
    for (dimension, transform, id) in sprites.iter().filter(|(d, ..)| d.0.is_some()) {
        let WidthHeight { width, height } = dimension.0.unwrap();
        let pos = transform.translation.xy();

        rigs.push((
            WidthHeight {
                width,
                height,
            }.to_rect(pos),
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
pub const SPAWN_POINT_AMOUNT_MAX: Range<u16> = 30..40;
/// spawns a point around a rig every x-y seconds
#[cfg(debug_assertions)]
pub const SPAWN_POINT_SPRITE_P: Range<usize> = 0..2;
#[cfg(not(debug_assertions))]
pub const SPAWN_POINT_SPRITE_P: Range<usize> = 15 * 60..30 * 60;

/// the maximum radius around a rig which a point can spawn
pub const SPAWN_POINT_RADIUS_MAX: f32 = 100.0;

/// speed at which a point moves toward a ship's HUD center
const POINT_SPEED: f32 = 2.0;

pub fn rig_spawn_points(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut transforms: Query<(&mut PointAmount, &Transform, &Sprite), With<OilRig>>,
    world_size: Single<&WorldSize>
) {
    let point_sprites: [(PointType, Handle<Image>); 3] = [
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
pub fn move_points(
    mut points_transform: Query<&mut Transform, With<PointType>>,
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
pub fn despawn_points(
    mut commands: Commands,
    points_transform: Query<(&Transform, Entity), With<PointType>>,
    circle_huds: Query<&CircleHud>,
) {
    for (point_transform, id) in points_transform {
        if circle_huds.iter().any(|hud| hud.at_center(point_transform.translation.xy(), DecimalPoint::Zero)) {
            commands.get_entity(id).unwrap().despawn();
        }
    }
}