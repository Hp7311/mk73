use bevy::input::keyboard::Key;
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::sprite_render::AlphaMode2d;
use lightyear::prelude::Controlled;
use common::boat::{Boat, SubKind};
use common::{eq, MainCamera, OCEAN_FLOOR, OCEAN_SURFACE};
use common::primitives::{CustomTransform, DecimalPoint, MeshBundle, Altitude as _};
use common::util::calculate_diving_overlay;

pub struct DivingPlugin;

impl Plugin for DivingPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<DivingStatus>();
        app.add_plugins(bevy::sprite_render::Material2dPlugin::<DivingOverlayShader>::default());
        app.add_systems(Startup, spawn_diving_overlay.after(crate::setup));
        app.add_systems(Update, update_diving_overlay);
        app.add_systems(Update, (
            update_diving_status.run_if(resource_changed::<ButtonInput<Key>>),
            act_on_state
        ).chain());
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
            transform.translation.z,
            OCEAN_FLOOR,
            DIVING_OVERLAY_MIN_RADIUS,
            DIVING_OVERLAY_MAX_RADIUS,
            DIVING_OVERLAY_MAX_DARKNESS,
        )
    }
}

#[derive(States, Default, Copy, Clone, PartialEq, Eq, Hash, Debug)]
enum DivingStatus {
    #[default]
    None,
    Surfacing,
    Diving
}
fn update_diving_status(
    mut setter: ResMut<NextState<DivingStatus>>,
    getter: Res<State<DivingStatus>>,
    buttons: Res<ButtonInput<Key>>,
    transform: Single<&Transform, (With<Controlled>, With<Boat>)>,
) {
    if buttons.just_pressed(Key::Character("r".into()))
        || buttons.just_pressed(Key::Character("R".into()))
    {
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

fn act_on_state(ships: Single<(&mut Transform, &Boat), With<Controlled>>, diving_status: Res<State<DivingStatus>>, mut setter: ResMut<NextState<DivingStatus>>) {
    let (mut transform, boat) = ships.into_inner();

    if boat.sub_kind() != SubKind::Submarine {
        return;
    }

    match diving_status.get() {
        DivingStatus::Diving => {
            transform.decrease_with_limit(boat.diving_speed().get_raw(), OCEAN_FLOOR);
            if transform.reached(OCEAN_FLOOR, DecimalPoint::Three) {
                setter.set(DivingStatus::None);
            }
        }
        DivingStatus::Surfacing => {
            transform.increase_with_limit(boat.diving_speed().get_raw(), OCEAN_SURFACE);
            if transform.reached(OCEAN_SURFACE, DecimalPoint::Three) {
                setter.set(DivingStatus::None);
            }
        }
        DivingStatus::None => ()
    }
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Copy, Debug, Default)]
struct DivingOverlayShader {
    #[uniform(0)]
    pub radius: f32,
    #[cfg(target_arch = "wasm32")]#[uniform(0)]pub _r_padding: Vec3,
    #[uniform(1)]
    pub player_pos: Vec2,
    #[cfg(target_arch = "wasm32")]#[uniform(1)]pub _p_padding: Vec2,
    #[uniform(2)]
    pub darkness: f32,
    #[cfg(target_arch = "wasm32")]#[uniform(2)]pub _d_padding: Vec3,
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

impl DivingStatus {
    /// substitute for `run_if` not working on multiple states
    fn in_state_2(first: Self, second: Self) -> impl Fn(Res<State<DivingStatus>>) -> bool {
        move |state| *state.get() == first || *state.get() == second
    }
}