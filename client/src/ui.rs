use bevy::prelude::*;
use bevy_inspector_egui::egui::emath::GuiRounding;
use common::{MainCamera, primitives::{CustomTransform, DisplayScore, PlayerStats}, protocol::{Rotate, Move}};
use lightyear::prelude::{input::native::{ActionState, InputMarker}, *};

use crate::BoatState;

pub(crate) struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        // app.add_plugins(DbgPlugin);
        app.add_systems(Update, recv_stats_update);
        app.add_systems(Startup, spawn_top_status_bar);
    }
}

#[cfg(not(target_family = "wasm"))]
fn spawn_top_status_bar(
    mut commands: Commands,
    camera: Single<Entity, With<MainCamera>>
) {
    commands.get_entity(camera.into_inner()).unwrap()
        .insert((
            // TODO
        ));
}
#[cfg(target_family = "wasm")]
fn spawn_top_status_bar() {
    if let Err(e) = spawn_top_status_bar_inner() {
        error!("{:?}", e);
    }
}
#[cfg(target_family = "wasm")]
fn spawn_top_status_bar_inner() -> anyhow::Result<()> {
    const UNIT_ONE_CSS: f32 = 10000.0;

    use crate::web_utils::*;
    use anyhow::anyhow;

    let outer_div = new_elem("div")?;
    outer_div.set_id("progress_bar");
    
    let outer_styling = get_styling(outer_div.clone());

    outer_styling.set_css_text("
        font-family: Arial,
        sans-serif;
        font-size: calc(7px + 0.8vmin);
        pointer-events: none;
        user-select: none;
        color: white;
        min-width: 30%;
        position: absolute;
        left: 50%;
        text-align: center;
        top: 0;
        margin-top: 0;
        transform: translate(-50%, 0%);
    ");

    let inner_div = new_elem("div")?;
    let text = document().ok_or_else(|| anyhow!("Document absent"))?
        .create_text_node("0% to level 2");
    inner_div.append_child(&text).map_err(to_dbg)?;
    
    let inner_styling = get_styling(inner_div.clone());


    inner_styling.set_css_text(&format!("
        font-family: Arial, sans-serif;
        font-size: calc(7px + 0.8vmin);
        pointer-events: none;
        border-top-left-radius: 0 !important;
        border-top-right-radius: 0 !important;
        border-top-width: 0 !important;
        border-style: solid;
        border-radius: 0.5rem;
        box-sizing: border-box;
        color: white;
        font-weight: bold;
        height: min-content;
        min-height: 1.1rem;
        overflow: hidden;
        padding: 0.2rem;
        text-align: center;
        transition: background-size 0.5s;
        user-select: none;
        width: 100%;
        background: linear-gradient(90deg, #0084b1 0%, #0084b1 1%, #3e3333 1%, #3e3333 100%);
        background-origin: border-box;
        background-size: {}%;
        border-width: 1px;
        border-color: #686868;
    ", (UNIT_ONE_CSS * 0.0) as u32));  // UNIT_ONE_CSS * score to next level

    outer_div.append_child(&inner_div).unwrap();

    insert_to_body(outer_div).unwrap();

    Ok(())
}
fn recv_stats_update(mut rx: Single<&mut MessageReceiver<DisplayScore>>) {
    for msg in rx.receive() {
        match msg {
            DisplayScore::NewLevel(level) => {
                info!("New level: {:?}", level);
                // draw upgrade box if doesn't already exist
                // draw boats if not already drawed
            },
            DisplayScore::Percent(p) => {
                info!("New percent: {}", p);
                // update bottom bare
            }
        }
    }
}

struct DbgPlugin;
impl Plugin for DbgPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_dbg_gui.after(crate::setup))
            .add_systems(Update, update_dbg_gui);
    }
}


fn spawn_dbg_gui(mut commands: Commands, camera: Single<Entity, With<MainCamera>>) {
    commands.get_entity(camera.into_inner()).unwrap().insert(children![(
        Text2d::new("RotateInput: None\nSpeedInput: None\nState: Stopped\nPosition: None\nAltitude: None\nRotation: None\nSpeed: None\nScore: None"),
        TextFont {
            font_size: 15.0,
            ..default()
        },
        Transform::from_xyz(0.0, 280.0, common::OIL_RIG_Z + common::OCEAN_SURFACE.0 + 1.0),
    )]);
}
fn update_dbg_gui(
    mut text: Single<&mut Text2d>,
    rotate: Single<&ActionState<Rotate>, With<InputMarker<Rotate>>>,
    moves: Single<&ActionState<Move>, With<InputMarker<Move>>>,
    state: Res<State<BoatState>>,
    custom: Single<&CustomTransform, With<Controlled>>,
    transform: Single<&Transform, With<Controlled>>,
    player_score: Single<&PlayerStats, With<Controlled>>
) {
    let state = format!("{:?}", state.into_inner()).split("State(").last().unwrap().to_owned();

    let new_text = format!(
        "RotateInput: {}\nSpeedInput: {}\nState: {}\nPosition: {}\nAltitude: {}\nRotation: {}\nSpeed: {}\nScore: {}\nLevel: {:?}",
        rotate.0.0.map(|r| r.to_degrees().round()).unwrap_or(0.0),
        moves.0.0.map(|r| r.get_knots().round()).unwrap_or(0.0),
        state.chars().take(state.len() - 1).collect::<String>(),
        custom.position.0.round(),
        transform.translation.z.round_to_pixels(10.0),
        custom.rotation.to_degrees().round(),
        custom.speed.get_knots().round(),
        player_score.score(),
        player_score.level()
    );

    if new_text != text.0 {
        text.0 = new_text;
    }
}
