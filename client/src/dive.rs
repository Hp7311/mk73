use bevy::input::common_conditions::input_just_pressed;
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::sprite_render::AlphaMode2d;
use common::primitives::{
    Altitude as _, CustomTransform, DecimalPoint, GetZIndex, MaybePushToSurface, MeshBundle, ZIndex
};
use common::protocol::{EntityOnServer, ZIndexUpdate};
use common::util::{calculate_diving_overlay, in_states_2, not_in_state};
use common::{Boat, BoatType, MainCamera, OCEAN_FLOOR, OCEAN_SURFACE, SubKind, eq};
use lightyear::input::client::InputSystems;
use lightyear::prelude::input::native::{ActionState, InputMarker};
use lightyear::prelude::Controlled;

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
                update_diving_status.run_if(input_just_pressed(KeyCode::KeyR)).run_if(resource_exists_and_equals(BoatType(SubKind::Submarine))),
                act_on_state.run_if(not_in_state(DivingStatus::None)),
            )
                .chain()
        );
        app.add_systems(FixedPreUpdate, (
            act_on_state.run_if(in_states_2(DivingStatus::Diving, DivingStatus::Surfacing)),
            clear_z_update.run_if(in_state(DivingStatus::None))
        ).in_set(InputSystems::WriteClientInputs));
        app.add_observer(push_to_surface_on_upgrade);
        app.add_systems(FixedUpdate, dbg_just_pressed.run_if(input_just_pressed(KeyCode::KeyR)));
    }
}

fn dbg_just_pressed() {
    info!("Just pressed R!");  /* sometimes doesn't dive even though pressed (maybe 2 presses for one like this
    
2026-06-07T16:38:55.877286Z  INFO client::dive: Just pressed R!
2026-06-07T16:38:55.877467Z  INFO client::dive: Just pressed R!
) */
}
#[allow(clippy::needless_update)]  // webgl paddings
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
                    }))
                },
                DivingOverlay,
                Name::from("Diving overlay"),
            ));
        });
    }
}

fn update_diving_overlay(
    boat_tf: Single<&Transform, (With<Boat>, With<Controlled>, Changed<CustomTransform>)>,
    mut diving_overlay_material: ResMut<Assets<DivingOverlayShader>>,
    overlay: Single<&MeshMaterial2d<DivingOverlayShader>, (With<DivingOverlay>, Without<Boat>)>,
) {
    let material_handle = overlay.into_inner();
    if let Some(diving_material) = diving_overlay_material.get_mut(material_handle) {

        diving_material.player_pos = boat_tf.translation.xy();
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
    /// when a submarine upgrades to a surface ship
    PushingToSurface(Boat), // due to f32 not Eq, can't store Speed
}

fn update_diving_status(
    mut setter: ResMut<NextState<DivingStatus>>,
    getter: Res<State<DivingStatus>>,
    transform: Single<&Transform, (With<Controlled>, With<Boat>)>,
) {
    debug!("Just pressed R");
    // if buttons.just_pressed(KeyCode::KeyR) {  already set in .run_if
        let target = match getter.get() {
            DivingStatus::None => {
                if eq!(transform.translation.z, 0.0) {
                    DivingStatus::Diving
                } else {
                    DivingStatus::Surfacing
                }
            }
            DivingStatus::Surfacing => DivingStatus::Diving,
            DivingStatus::Diving => DivingStatus::Surfacing,
            DivingStatus::PushingToSurface(_) => {
                warn!("Should not run the system");
                return;
            }
        };

        setter.set(target);
}

/// modifies `Transform::z` and sends new ZIndex to server
fn act_on_state(
    ships: Single<(&mut Transform, &Boat, &mut ZIndex, &EntityOnServer), With<Controlled>>,
    diving_status: Res<State<DivingStatus>>,
    mut setter: ResMut<NextState<DivingStatus>>,
    // mut tcp_wrapper: ResMut<TcpWrapper>
    // mut sender: Single<&mut MessageSender<NewZIndex>>,
    mut z_update: Single<&mut ActionState<ZIndexUpdate>, With<InputMarker<ZIndexUpdate>>>
) {
    let (mut transform, boat, mut z_index, _e) = ships.into_inner();

    match diving_status.get() {
        DivingStatus::Diving => {
            // local simulation
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
        DivingStatus::PushingToSurface(target) => {
            *z_index = transform.increase_with_limit(target.diving_speed().get_raw(), OCEAN_SURFACE);

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

    trace!("State: {:?}, Depth: {:?}", diving_status.get(), z_index);
    // let amount = tcp_wrapper.write(&TcpClientRequest::NewZIndex(*z_index).to_bytes()).unwrap();
    // assert_eq!(amount, 4);

    // sender.send::<SendToServerOrdered>(NewZIndex { new_index: **z_index, entity_on_server: *e });

    // info!(?z_index);
    z_update.0 = ZIndexUpdate(Some(*z_index));
    // unsafe { FRAME_TO_CLEAR = Some(50); }
}

#[expect(clippy::partialeq_to_none)]
fn clear_z_update(mut z_update: Single<&mut ActionState<ZIndexUpdate>, With<InputMarker<ZIndexUpdate>>>) {
    if z_update.0.0 == None {
        return;
    }

    // ActionState is not accurate therefore uses ?precision = 0.05 in z index comparisons
    unsafe {
        if FRAME_TO_CLEAR == None {
            FRAME_TO_CLEAR = Some(80);
            return;
        }
        let Some(ref mut f) = FRAME_TO_CLEAR else { unreachable!() };

        *f -= 1;

        if *f == 0 {
            z_update.0 = ZIndexUpdate(None);
            debug!("Cleared ZIndexUpdate after 80 frames");
            FRAME_TO_CLEAR = None;
        }
    }
}

static mut FRAME_TO_CLEAR: Option<u8> = None;

fn push_to_surface_on_upgrade(
    trigger: On<MaybePushToSurface>,
    transform: Single<&Transform, With<Controlled>>,
    mut setter: ResMut<NextState<DivingStatus>>
) {
    if !transform.reached(OCEAN_SURFACE, DecimalPoint::Three)
    {
        debug!("Surfacing after upgrading to a ship");
        setter.set(DivingStatus::PushingToSurface(trigger.last_boat));
    }
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
