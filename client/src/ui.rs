use bevy::prelude::*;
use bevy_inspector_egui::egui::emath::GuiRounding;
use common::{MainCamera, primitives::{CustomTransform, DisplayScore, PlayerStats, Level, Percent}, protocol::{Move, Rotate}};
use lightyear::prelude::{input::native::{ActionState, InputMarker}, *};

#[cfg(target_family = "wasm")]
use {
    crate::web_utils::*,
    anyhow::anyhow
};

use crate::BoatState;

pub(crate) struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        // app.add_plugins(DbgPlugin);
        app.add_systems(Startup, spawn_progress_bar.after(crate::setup));
        app.add_systems(Update, recv_stats_update);
    }
}

fn recv_stats_update(
    mut rx: Single<&mut MessageReceiver<DisplayScore>>,
    current: Single<&PlayerStats>,
    #[cfg(not(target_family = "wasm"))]
    bar: Single<(&mut Text, &mut BackgroundGradient), With<normal::ProgressBar>>
) {
    #[cfg(not(target_family = "wasm"))]
    let (mut text, mut background) = bar.into_inner();

    for msg in rx.receive() {
        match msg {
            DisplayScore::NewLevel(level) => {
                info!("New level: {:?}", level);
                // draw upgrade box if doesn't already exist
                // draw boats if not already drawed
            },
            DisplayScore::Percent(p) => {
                update_percent(p, current.level(), &mut text, &mut background);
            }
        }
    }
}

#[cfg(target_family = "wasm")]
use wasm::{spawn_progress_bar, update_percent};
#[cfg(target_family = "wasm")]
mod wasm {
    use super::*;
    pub(super) fn spawn_progress_bar() {
        let div = new_elem("div").unwrap();
        div.set_id(ProgressBar::html_id());
        div.insert_adjacent_text("afterBegin", &ProgressText::new().to_string()).unwrap();
        insert_to_body(div).unwrap();
    }

    pub(super) fn update_percent(new_percent: Percent, current_level: Level) {
        let next_level = current_level + 1;
        let progress_bar = ProgressBar::get_element().unwrap();
        let progress_bar = element_to_html_element(progress_bar).unwrap();

        ProgressText::set(&progress_bar, new_percent, next_level);

        ProgressBar::set_percentage(&progress_bar.style(), new_percent);
    }

    pub(super) fn draw_upgrade(upgrading_level: Level) -> anyhow::Result<()> {
        todo!()
    }
}


#[cfg(not(target_family = "wasm"))]
use normal::{spawn_progress_bar, update_percent};
#[cfg(not(target_family = "wasm"))]
mod normal {
    use common::util::InputExt as _;
    use super::*;

    #[derive(Debug, Component)]
    pub struct ProgressBar;

    pub fn spawn_progress_bar(
        mut commands: Commands,
        camera: Single<Entity, With<MainCamera>>,
        asset_server: Res<AssetServer>
    ) {
        commands.get_entity(camera.into_inner()).unwrap()
            .insert(Node {
                position_type: PositionType::Absolute,
                left: Val::ZERO,
                right: Val::ZERO,
                top: Val::ZERO,
                height: Val::Auto,
                justify_content: JustifyContent::Center,
                ..default()
            })
            .with_child((
                Node {
                    overflow: Overflow::hidden(),
                    position_type: PositionType::Absolute,
                    // left: Val::Percent(50.),  // parent centers all children
                    // height: min-content
                    height: Val::Px(20.4),  // min-content
                    min_width: Val::Percent(30.),
                    min_height: Val::Px(18.), // 1.1rem
                    padding: UiRect::all(Val::Px(3.2)), // 0.2rem
                    border: UiRect {
                        top: Val::ZERO,
                        ..UiRect::all(Val::Px(1.0))
                    },
                    // smaller = less rounding
                    border_radius: BorderRadius {
                        top_left: Val::ZERO,
                        top_right: Val::ZERO,
                        bottom_left: Val::Px(8.0),
                        bottom_right: Val::Px(8.0)
                    },
                    ..default()
                },
                Outline::new(Val::Px(1.0), Val::ZERO, Color::srgb_u8(102, 136, 102)),
                TextLayout {
                    justify: Justify::Center,
                    ..default()
                },
                TextFont {
                    font: asset_server.load("regular.otf"),
                    font_size: 15.0,
                    weight: FontWeight::BOLD,
                    ..default()
                },
                Text("0% to level 2".to_owned()),
                BackgroundGradient(vec![
                    LinearGradient::to_right(vec![
                        // blue
                        ColorStop::percent(Color::srgb_u8(0, 132, 177), 0.),
                        ColorStop::percent(Color::srgb_u8(0, 132, 177), 1.),
                        // brown
                        ColorStop::percent(Color::srgb_u8(62, 51, 51), 1.)
                    ]).into()
                ]),
                ProgressBar
            ));
    }

    pub fn update_percent(
        new_percent: Percent,
        current_level: Level,
        text: &mut Text,
        background: &mut BackgroundGradient
    ) {
        let next_level = current_level + 1;
        text.0 = format!("{new_percent}% to level {next_level}");

        // assumes only one LinearGradient in background
        if let Some(gradient) = background.0.get_mut(0)
            && let Gradient::Linear(linear_gradient) = gradient
            && linear_gradient.stops.len() == 3
        {
            linear_gradient.stops[1].point = Val::Percent(new_percent.to::<f32>());
            linear_gradient.stops[2].point = Val::Percent(new_percent.to::<f32>());
        } else {
            error_once!("Unexpected background configuration");
        }
    }
}


#[allow(dead_code)]
struct DbgPlugin;

impl Plugin for DbgPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, Self::spawn_dbg_gui.after(crate::setup))
            .add_systems(Update, Self::update_dbg_gui);
    }
}

#[allow(dead_code)]
impl DbgPlugin {

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

}