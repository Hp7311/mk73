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
    MainCamera,
    add_circle_hud, calculate_from_proportion, get_cursor_pos, get_rotate_radian,
    move_with_rotation, check_in_vec2, get_head, get_map_size,
    TrimRadian
};

#[derive(Component, Debug, Copy, Clone)]
pub struct Ship;

const WORLD_MIN: Vec2 = vec2(4000.0, 2000.0);
const WORLD_EXPAND: f32 = 2000.0;

/// absolute value of minimum radians that must be reached to reverse the Ship
const MINIMUM_REVERSE: f32 = PI * (2.0 / 3.0);

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
            YASEN_BACK_SPEED,
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
                mesh: Mesh2d(
                    meshes.add(
                        Circle::new(add_circle_hud(YASEN_RAW_SIZE / 2.0) * DEFAULT_SPRITE_SHRINK)  // TODO
                            .to_ring(3.0),
                    ),
                ),
                materials: MeshMaterial2d(materials.add(ColorMaterial::from_color(GRAY))),
            },
            Transform::from_translation(vec3(0.0, 0.0, 30.0))  // relative to parent, circle hud highest Z
        ));
    });

    // in Sprites, translation is the center point of the Sprite rendered
    let world_size = get_map_size(1, WORLD_MIN, WORLD_EXPAND);  // TODO smart idea: using structs and default values instead of constants!
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
            custom_size: Some(world_size),
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
        let x = rng.random_range(-world_size.x.round() as i32 / 2..world_size.x.round() as i32 / 2) as f32;
        let y = rng.random_range(-world_size.y.round() as i32 / 2..world_size.y.round() as i32 / 2) as f32;
        
        commands.spawn((
            Transform::from_translation(vec3(x, y, 0.0))
                .with_rotation(Quat::from_rotation_z(rotation)),
            Sprite {
                image: oil_rig.clone(),
                ..default()
            },
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
        Query<(&Transform, &mut CustomTransform, &Radian, &mut TargetRotation), With<Ship>>,
        Query<(&Transform, &mut CustomTransform, &Radian, &TargetRotation), With<Ship>>,
        Query<(&Transform, &mut CustomTransform, &Radius, &MaxSpeed, &ReverseSpeed), With<Ship>>,
    )>
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

        let raw_moved = get_rotate_radian(cursor_pos, transform.translation.xy());  // diff from radian 0
        let (_, _, current_rotation) = transform.rotation.to_euler(EulerRot::XYZ);
        let mut target_move = raw_moved;

        let moved = {  // radians to move from current rotation
            let mut moved_from_current = (raw_moved.to_degrees() - current_rotation.to_degrees())
                .to_radians()
                .trim();

            if moved_from_current.abs() > MINIMUM_REVERSE {
                custom_transform.reversed = true;
                // if reversing, adjust return value
                moved_from_current = (moved_from_current + PI).trim();
                target_move = (target_move + PI).trim()

            } else if custom_transform.reversed {
                custom_transform.reversed = false;
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

/// handle moving
fn move_ship(
    datas: &mut Query<(&Transform, &mut CustomTransform, &Radius, &MaxSpeed, &ReverseSpeed), With<Ship>>,
    cursor_pos: Vec2,
) {
    for (transform, mut custom_transform, radius, max_speed, reverse_speed) in datas.iter_mut() {
        let cursor_distance = cursor_pos.distance(transform.translation.xy());
        let speed = if custom_transform.reversed {
            reverse_speed.0
        } else {
            max_speed.0
        };

        let speed = calculate_from_proportion(
            cursor_distance,
            add_circle_hud(radius.default_convert().0),
            speed,
            radius.default_convert().0,
        );

        custom_transform.speed = Speed(speed);
    }
}

// note that we're accepting Query instead of Single for ship everywhere
// and not descriminating Bot/Player

/// remember the last move angle and rotate toward it when button not pressed
fn rotate_ship_to_target(ships: &mut Query<(&Transform, &mut CustomTransform, &Radian, &TargetRotation), With<Ship>>) {
    for (transform, mut custom_transform, max_turn, target) in ships {
        let Some(target) = target.0 else { continue; };

        let (_, _, current_rotation) = transform.rotation.to_euler(EulerRot::XYZ);

        let moved = (target.to_degrees() - current_rotation.to_degrees())
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
    }
}

/// updates [`Ship`]'s [`Transform`] according to its [`CustomTransform`]
pub fn update_transform(
    mut transform_ship: Query<(&mut Transform, &mut CustomTransform, &Radius), With<Ship>>,
    world_size: Single<&WorldSize>
) {
    for (mut transform, mut custom, radius_raw) in transform_ship.iter_mut() {
        let mut translation = custom.position.to_vec3();
        if custom.reversed {
            translation += move_with_rotation(transform.rotation, -custom.speed.0);
        } else {
            translation += move_with_rotation(transform.rotation, custom.speed.0);  // ignores frame lagging temporary
        }

        // TODO use hit box instead of raw head
        // TODO check tail
        if !check_in_vec2(get_head(radius_raw.0 * DEFAULT_SPRITE_SHRINK, translation.xy(), custom.rotation.to_quat()), world_size.0) {  // only checks head
            println!("Out of bounds!!");
            return;
        }
        let target = Transform {
            translation,
            rotation: custom.rotation.to_quat(),
            scale: Vec3::ONE,
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

/// resize [`Sprite`]s by default constant
fn resize_inner<T: Component>(mut sprites: ResizeSprite<T>, assets: Res<Assets<Image>>) {
    for mut sprite in sprites.iter_mut() {
        let Some(image) = assets.get(&mut sprite.image) else {
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