use std::io::Write;

use crate::{BoatType, tcp::TcpWrapper};
use bevy::input::common_conditions::input_just_pressed;
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::sprite_render::AlphaMode2d;
use common::primitives::{
    Altitude as _, CustomTransform, DecimalPoint, GetZIndex, MeshBundle, ZIndex,
};
use common::protocol::{EntityOnServer, NewZIndex, SendToServerOrdered};
use common::util::{calculate_diving_overlay, in_states_2};
use common::{Boat, MainCamera, OCEAN_FLOOR, OCEAN_SURFACE, SubKind, debug_component, eq, print_num};
use lightyear::prelude::{Controlled, MessageSender};

pub(crate) struct DivingPlugin;

impl Plugin for DivingPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<DivingStatus>();
        app.add_plugins(bevy::sprite_render::Material2dPlugin::<DivingOverlayShader>::default());
        app.add_systems(Startup, spawn_diving_overlay.after(crate::setup));
        app.add_systems(Update, update_diving_overlay);
        app.add_systems(
            FixedUpdate,
            (
                update_diving_status.run_if(input_just_pressed(KeyCode::KeyR)),
                act_on_state.run_if(in_states_2(DivingStatus::Diving, DivingStatus::Surfacing)),
            )
                .chain()
                .run_if(resource_exists_and_equals(BoatType(SubKind::Submarine))),
        );

        // app.add_systems(Update, debug_component!(ZIndex, With<Controlled>, |z: &ZIndex| z.0 != 0.0));
    }
}

#[allow(clippy::needless_update)]
fn spawn_diving_overlay(
    mut commands: Commands,
    mut diving_overlay_material: ResMut<Assets<DivingOverlayShader>>,
    mut meshes: ResMut<Assets<Mesh>>,
    camera: Single<Entity, With<MainCamera>>,
) {
    if let Ok(mut camera) = commands.get_entity(*camera) {
        camera.with_children(|parent| {
            parent.spawn((
                Transform::from_xyz(0.0, 0.0, DIVING_OVERLAY_Z),
                MeshBundle {
                    mesh: Mesh2d(meshes.add(DIVING_OVERLAY_SHAPE)),
                    materials: MeshMaterial2d(diving_overlay_material.add(DivingOverlayShader {
                        radius: DIVING_OVERLAY_MAX_RADIUS,
                        player_pos: vec2(0.0, 0.0),
                        darkness: 0.0,
                        ..default()
                    })),
                },
                DivingOverlay,
                Name::from("Diving overlay"),
            ));
        });
    }
}

fn update_diving_overlay(
    boat: Single<(&CustomTransform, &Transform), (With<Boat>, With<Controlled>, Changed<CustomTransform>)>,
    mut diving_overlay_material: ResMut<Assets<DivingOverlayShader>>,
    overlay: Single<(&MeshMaterial2d<DivingOverlayShader>, &mut Transform), (With<DivingOverlay>, Without<Boat>)>,
) {
    let (material_handle, mut transform) = overlay.into_inner();
    if let Some(diving_material) = diving_overlay_material.get_mut(material_handle) {
        let (custom, boat_tf) = *boat;
        diving_material.player_pos = custom.position.0;
        transform.translation.x = boat_tf.translation.x;
        transform.translation.y = boat_tf.translation.y;
        (diving_material.radius, diving_material.darkness) = calculate_diving_overlay(
            boat_tf.translation.z_index(),
            OCEAN_FLOOR,
            DIVING_OVERLAY_MIN_RADIUS,
            DIVING_OVERLAY_MAX_RADIUS,
            DIVING_OVERLAY_MAX_DARKNESS,
        );
    }
}

#[derive(States, Default, Copy, Clone, PartialEq, Eq, Hash, Debug)]
enum DivingStatus {
    #[default]
    None,
    Surfacing,
    Diving,
}
fn update_diving_status(
    mut setter: ResMut<NextState<DivingStatus>>,
    getter: Res<State<DivingStatus>>,
    transform: Single<&Transform, (With<Controlled>, With<Boat>)>,
) {
    // if buttons.just_pressed(KeyCode::KeyR) {  already set in .run_if
        let mut target = *getter.get();
        match target {
            DivingStatus::None => {
                if eq!(transform.translation.z, 0.0) {
                    target = DivingStatus::Diving
                } else {
                    target = DivingStatus::Surfacing;
                }
            }
            DivingStatus::Surfacing => target = DivingStatus::Diving,
            DivingStatus::Diving => target = DivingStatus::Surfacing,
        }

        setter.set(target);
}

fn act_on_state(
    ships: Single<(&mut Transform, &mut ZIndex, &Boat), With<Controlled>>,
    diving_status: Res<State<DivingStatus>>,
    mut setter: ResMut<NextState<DivingStatus>>,
    mut tcp_wrapper: ResMut<TcpWrapper>
) {
    let (mut transform, mut z_index, boat) = ships.into_inner();

    #[cfg(debug_assertions)]
    if boat.sub_kind() != SubKind::Submarine {
        warn!("Should .run_if(resource_exists_and_equals(BoatType(SubKind::Submarine))));");
        return;
    }

    match diving_status.get() {
        DivingStatus::Diving => {
            *z_index = transform.decrease_with_limit(boat.diving_speed().get_raw(), OCEAN_FLOOR);

            if transform.reached(OCEAN_FLOOR, DecimalPoint::Three) {
                setter.set(DivingStatus::None);
            }
        }
        DivingStatus::Surfacing => {
            *z_index = transform.increase_with_limit(boat.diving_speed().get_raw(), OCEAN_SURFACE);

            if transform.reached(OCEAN_SURFACE, DecimalPoint::Three) {
                setter.set(DivingStatus::None);
            }
        }
        DivingStatus::None => {
            warn!(
                "Should only act_on_state if in DivingStatus::Diving or DivingStatus::Surfacing"
            );
            return;
        }
    }
    info!(?z_index);
    // new_z.send::<SendToServerOrdered>(NewZIndex {
    //     new_index: *z_index,
    //     entity_on_server,
    // });
    let amount = tcp_wrapper.write(&z_index.to_be_bytes()).unwrap();
    assert_eq!(amount, 4);
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Copy, Debug, Default)]
struct DivingOverlayShader {
    #[uniform(0)]
    pub radius: f32,
    #[cfg(target_arch = "wasm32")]
    #[uniform(0)]
    pub _r_padding: Vec3,
    #[uniform(1)]
    pub player_pos: Vec2,
    #[cfg(target_arch = "wasm32")]
    #[uniform(1)]
    pub _p_padding: Vec2,
    #[uniform(2)]
    pub darkness: f32,
    #[cfg(target_arch = "wasm32")]
    #[uniform(2)]
    pub _d_padding: Vec3,
}

const DIVING_OVERLAY_MIN_RADIUS: f32 = 800.0;
/// must cover the whole screen. 4000 * 2000 is pretty big
const DIVING_OVERLAY_SHAPE: Rectangle = Rectangle::new(4000.0, 2000.0);
const DIVING_OVERLAY_MAX_RADIUS: f32 = 1000.0;
const DIVING_OVERLAY_MAX_DARKNESS: f32 = 0.6;
const DIVING_OVERLAY_Z: f32 = 35.0;

#[derive(Component)]
struct DivingOverlay;

impl bevy::sprite_render::Material2d for DivingOverlayShader {
    fn fragment_shader() -> ShaderRef {
        "shaders/diving_overlay.wgsl".into()
    }
    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}
