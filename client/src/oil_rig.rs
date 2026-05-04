use bevy::prelude::*;
use common::protocol::{OilRigInfo, PointInfo};
// use lightyear::prelude::*;

pub(crate) struct OilRigPlugin;

impl Plugin for OilRigPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(spawn_rig)
            .add_observer(spawn_point);
        app.add_systems(Update, sync_point_transform);
    }
}

fn spawn_rig(
    trigger: On<Add, OilRigInfo>,
    rigs: Query<&OilRigInfo>,
    assert_server: Res<AssetServer>,
    mut commands: Commands
) {
    // NOTE client-inserted components get removed when server despawns the replicating entity
    let Ok(rig_info) = rigs.get(trigger.entity) else { panic!() };

    commands.get_entity(trigger.entity).unwrap().insert((
        Transform {
            translation: rig_info.position.extend(OilRigInfo::z_index_transform()),
            rotation: rig_info.rotation.to_quat(),
            ..default()
        },
        Sprite {
            image: assert_server.load(OilRigInfo::file_name()),
            custom_size: Some(OilRigInfo::custom_size()),
            ..default()
        },
        Name::new("Oil rig")
    ));
}

fn spawn_point(
    trigger: On<Add, PointInfo>,
    points: Query<&PointInfo>,
    asset_server: Res<AssetServer>,
    mut commands: Commands
) {
    let point_info = points.get(trigger.entity).unwrap();

    commands.get_entity(trigger.entity).unwrap()
        .insert((
            Sprite {
                image: asset_server.load((*point_info.file_name).to_owned()),
                custom_size: Some(PointInfo::custom_size()),
                ..default()
            },
            Transform::from_translation(point_info.to_translation()),
            Name::new("Point")
        ));
}

fn sync_point_transform(
    points: Query<(&PointInfo, &mut Transform), Changed<PointInfo>>,
) {
    for (info, mut transform) in points {
        // important
        transform.translation = info.to_translation();
    }
}
