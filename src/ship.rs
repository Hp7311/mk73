//! currently, there are no differentiation between a Ship and a Submarine
//!
//! be mindful of [`Ship::transform`] and [`Transform`] of the [`Ship`] needs to be kept in sync

// doc outdated

use std::f32::consts::PI;

use bevy::camera_controller::pan_camera::PanCamera;
use bevy::color::palettes::css::*;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use rand::RngExt;

use crate::constants::*;
use crate::primitives::*;
use crate::util::{
    MainCamera, add_circle_hud, calculate_from_proportion, get_cursor_pos, get_rotate_radian,
    move_with_rotation,
};

#[derive(Component, Debug, Copy, Clone)]
pub struct Ship;

const WORLD_SIZE: Vec2 = vec2(4000.0, 2000.0);

// TODO time to modulise

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

    commands.spawn((
        ShipBundle::new(
            YASEN_MAX_SPEED,
            vec2(100.0, 0.0),
            "yasen.png",
            asset_server.clone(),
            YASEN_RAW_SIZE / 2.0,
        ),
        Ship,
    ))
    .with_children(|parent | {
        parent.spawn(CircleHudBundle {
            mesh: Mesh2d(
                meshes.add(
                    Circle::new(add_circle_hud(YASEN_RAW_SIZE / 2.0) * DEFAULT_SPRITE_SHRINK)
                        .to_ring(3.0),
                ),
            ),
            materials: MeshMaterial2d(materials.add(ColorMaterial::from_color(RED))),
        });
    });

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
            custom_size: Some(WORLD_SIZE), // TODO spawn sprites from edge of screen / world map (multiplayer)
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

    for _ in 0..10 {
        // TODO hard coded
        let x = rng.random_range(-WORLD_SIZE.x.round() as i32..WORLD_SIZE.x.round() as i32) as f32;
        let y = rng.random_range(-WORLD_SIZE.y.round() as i32..WORLD_SIZE.y.round() as i32) as f32;
        
        commands.spawn((
            Transform::from_translation(vec3(x, y, 0.0)),
            Sprite {
                image: oil_rig.clone(),
                ..default()
            },
            // Text2d("I'm here".to_owned()),
            OilRig,
        ));
    }
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
        Query<(&Transform, &mut CustomTransform, &Radian, &mut TargetRotation), With<Ship>>,
        Query<(&Transform, &mut CustomTransform, &Radian, &TargetRotation), With<Ship>>,
        Query<(&Transform, &mut CustomTransform, &Radius, &Speed), With<Ship>>,
    )>,
) {
    if let Some(cursor_pos) = get_cursor_pos(window, camera)
        && buttons.pressed(MouseButton::Left)
    {
        rotate_ship(&mut queries.p0(), cursor_pos);
        move_ship(&mut queries.p2(), cursor_pos);
    } else {
        rotate_ship_to_target(&mut queries.p1());
    }
}

/// handle rotation
fn rotate_ship(
    transforms: &mut Query<(&Transform, &mut CustomTransform, &Radian, &mut TargetRotation), With<Ship>>,
    cursor_pos: Vec2,
) {
    for (transform, mut custom_transform, max_turn, mut target_rotation) in transforms.iter_mut() {
        // TODO consider subtracting this into the system

        let raw_moved = get_rotate_radian(cursor_pos, transform.translation.xy());  // diff from radian 0
        let (_, _, current_rotation) = transform.rotation.to_euler(EulerRot::XYZ);

        let moved = {  // radians to move from current rotation
            let mut raw_moved =
                (raw_moved.to_degrees() - current_rotation.to_degrees()).to_radians();
            if raw_moved > PI {
                raw_moved -= 2.0 * PI;
            } else if raw_moved < -PI {
                raw_moved += 2.0 * PI;
            }
            raw_moved
        };

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

        *target_rotation = Some(raw_moved).into();  // when moving, raw_moved will differ with mouse on same pos
    }
}

/// handle moving
fn move_ship(
    datas: &mut Query<(&Transform, &mut CustomTransform, &Radius, &Speed), With<Ship>>,
    cursor_pos: Vec2,
) {
    for (transform, mut custom_transform, radius, max_speed) in datas.iter_mut() {
        let cursor_distance = cursor_pos.distance(transform.translation.xy());
        let speed = calculate_from_proportion(
            cursor_distance,
            add_circle_hud(radius.default_convert().0),
            max_speed.0,
            radius.default_convert().0,
        );

        println!("Speed: {}", speed);

        custom_transform.speed = Speed(speed); // TODO currently not using Speed in custom
    }
}

// note that we're accepting Query instead of Single for ship everywhere
// and not descriminating Bot/Player

fn rotate_ship_to_target(ships: &mut Query<(&Transform, &mut CustomTransform, &Radian, &TargetRotation), With<Ship>>) {
    for (transform, mut custom_transform, max_turn, target) in ships {
        let Some(target) = target.0 else { continue; };

        let (_, _, current_rotation) = transform.rotation.to_euler(EulerRot::XYZ);

        let moved = {  // radians to move from current rotation
            let mut raw_moved =
                (target.to_degrees() - current_rotation.to_degrees()).to_radians();
            if raw_moved > PI {
                raw_moved -= 2.0 * PI;
            } else if raw_moved < -PI {
                raw_moved += 2.0 * PI;
            }
            raw_moved
        };

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
    }
}

/// updates [`Ship`]'s [`Transform`] along with Circle HUD
pub fn update_transform(
    mut transform_ship: Query<(&mut Transform, &mut CustomTransform), With<Ship>>,
) {
    for (mut transform, mut custom) in transform_ship.iter_mut() {
        let mut translation = custom.position.to_vec3();
        translation += move_with_rotation(transform.rotation, custom.speed.0);

        let target = Transform {
            translation,
            rotation: custom.rotation.to_quat(),
            scale: Vec3::splat(1.0),
        };
        *transform = target;

        custom.position = Position(translation.xy()); // sync position
    }
}

type ResizeSprite<'a, 'w, 's, T> = Query<'w, 's, &'static mut Sprite, With<T>>;

pub fn resize_ship(sprites: ResizeSprite<Ship>, assets: Res<Assets<Image>>) {
    resize_inner(sprites, assets);
}

pub fn resize_rigs(sprites: ResizeSprite<OilRig>, assets: Res<Assets<Image>>) {
    resize_inner(sprites, assets);
}

fn resize_inner<T: Component>(mut sprites: ResizeSprite<T>, assets: Res<Assets<Image>>) {
    for mut sprite in sprites.iter_mut() {
        let Some(image) = assets.get(&mut sprite.image) else {
            continue;
        };
        if sprite.custom_size.is_some() {
            continue;
        }

        println!("Changing size..");
        sprite.custom_size = Some(vec2(
            image.width() as f32 * DEFAULT_SPRITE_SHRINK,
            image.height() as f32 * DEFAULT_SPRITE_SHRINK,
        ));
    }
}

// TODO add the raw size of sprite to Entity to calculate hit box

pub fn validate_rigs(mut commands: Commands, sprites: Query<(&Sprite, &Transform, Entity), With<OilRig>>) {
    let mut bounding_boxes = vec![];
    
    for (sprite, transform, id) in sprites.iter().filter(|(sprite, _, _)| sprite.custom_size.is_some()) {
        let sprite_size = sprite.custom_size.unwrap();
        let pos = transform.translation.xy();

        bounding_boxes.push((
            RectWithWh {
                pos,
                w_h: sprite_size,
            },
            id
        ));
    }

    for (rect, id) in bounding_boxes.iter() {
        if bounding_boxes
            .iter()
            .filter(|(target, _)| target != rect)
            .any(|(target, _)| rect.intersects_with(target))
        {
            commands.get_entity(*id).unwrap().despawn();
        }
    }
}