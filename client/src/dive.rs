use crate::BoatType;
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::sprite_render::AlphaMode2d;
use common::primitives::{
    Altitude as _, CustomTransform, DecimalPoint, GetZIndex, MeshBundle, ZIndex,
};
use common::protocol::{EntityOnServer, NewZIndex, SendToServer};
use common::util::{calculate_diving_overlay, in_states_2};
use common::{eq, Boat, MainCamera, SubKind, OCEAN_FLOOR, OCEAN_SURFACE};
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
                update_diving_status.run_if(resource_changed::<ButtonInput<KeyCode>>),
                act_on_state.run_if(in_states_2(DivingStatus::Diving, DivingStatus::Surfacing)),
            )
                .chain()
                .run_if(resource_exists_and_equals(BoatType(SubKind::Submarine))),
        );
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
                        player_pos: vec2(0.0, 0.0), // hm
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
    ship: Single<&CustomTransform, (With<Boat>, With<Controlled>, Changed<CustomTransform>)>,
    transform: Single<&Transform, (With<Boat>, With<Controlled>)>,
    mut diving_overlay_material: ResMut<Assets<DivingOverlayShader>>,
    id: Single<&MeshMaterial2d<DivingOverlayShader>>,
) {
    if let Some(diving_material) = diving_overlay_material.get_mut(*id) {
        diving_material.player_pos = ship.position.0;
        (diving_material.radius, diving_material.darkness) = calculate_diving_overlay(
            transform.translation.z_index(),
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
    buttons: Res<ButtonInput<KeyCode>>,
    transform: Single<&Transform, (With<Controlled>, With<Boat>)>,
) {
    if buttons.just_pressed(KeyCode::KeyR) {
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
}

fn act_on_state(
    ships: Single<(&mut Transform, &mut ZIndex, &Boat, &EntityOnServer), With<Controlled>>,
    diving_status: Res<State<DivingStatus>>,
    mut setter: ResMut<NextState<DivingStatus>>,
    mut new_z: Single<&mut MessageSender<NewZIndex>>,
) {
    let (mut transform, mut z_index, boat, &entity_on_server) = ships.into_inner();

    if boat.sub_kind() != SubKind::Submarine {
        warn!("Should .run_if(resource_exists_and_equals(BoatType(SubKind::Submarine))));")
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
            error!(
                "Should only act_on_state if in DivingStatus::Diving or DivingStatus::Surfacing"
            );
            return;
        }
    }
    new_z.send::<SendToServer>(NewZIndex {
        new_index: *z_index,
        entity_on_server,
    });
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
const DIVING_OVERLAY_SHAPE: Rectangle = Rectangle::from_length(2000.0);
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
