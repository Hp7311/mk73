use bevy::prelude::*;
use common::protocol::{OilRigTransform, PointTransform as Point};

use crate::asset::SpriteMap;

pub(crate) struct OilRigPlugin;

impl Plugin for OilRigPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(spawn_rig)
            .add_observer(spawn_point);
        app.add_systems(Update, sync_point_transform);
    }
}

fn spawn_rig(
    trigger: On<Add, OilRigTransform>,
    rigs: Query<&OilRigTransform>,

    assert_server: Res<AssetServer>,
    mut commands: Commands
) {
    // NOTE client-inserted components get removed when server despawns the replicating entity
    let Ok(rig_info) = rigs.get(trigger.entity) else { panic!() };

    commands.get_entity(trigger.entity).unwrap().insert((
        Transform {
            translation: rig_info.position.extend(OilRigTransform::z_index_transform()),
            rotation: rig_info.rotation.to_quat(),
            ..default()
        },
        Sprite {
            image: assert_server.load(OilRigTransform::file_name()),
            custom_size: Some(OilRigTransform::custom_size()),
            ..default()
        },
        Name::new("Oil rig")
    ));
}

fn spawn_point(
    trigger: On<Add, Point>,
    points: Query<&Point>,
    sprites: Res<SpriteMap>,
    mut commands: Commands
) {
    let point_info = points.get(trigger.entity).unwrap();

    commands.get_entity(trigger.entity).unwrap()
        .insert((
            Sprite {
                // points get spawned VERY frequently
                image: sprites.get_long_lived(point_info.point.file_name()),
                custom_size: Some(Point::custom_size()),
                ..default()
            },
            Transform::from_translation(point_info.to_translation()),
            Name::new("Point")
        ));
}

fn sync_point_transform(
    points: Query<(&Point, &mut Transform), Changed<Point>>,
) {
    for (tf, mut transform) in points {
        // important
        transform.translation = tf.to_translation();
    }
}
