use bevy::prelude::*;

pub struct WeaponPlugin;

impl Plugin for WeaponPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SpawnWeaponMessage>()
            .add_systems(Update, spawn_weapon);
    }
}

#[derive(Debug, Component, Clone, Copy)]
pub(crate) enum Weapon {
    Set65  // TODO seperate torp/shell/etc
}

impl Weapon {
    fn file_name(&self) -> &'static str {
        match self {
            Weapon::Set65 => "Set65.png"
        }
    }
    fn custom_size(&self) -> Vec2 {
        match self {
            Weapon::Set65 => vec2(25.6, 2.0)
        }
    }
}

#[derive(Debug, Message)]
pub(crate) struct SpawnWeaponMessage{
    pub weapon: Weapon,
    pub position: Vec2
}

fn spawn_weapon(mut commands: Commands, mut reader: MessageReader<SpawnWeaponMessage>, asset_server: Res<AssetServer>) {
    for SpawnWeaponMessage {weapon, position} in reader.read() {
        commands.spawn((
            Sprite {
                image: asset_server.load(weapon.file_name()),
                custom_size: Some(weapon.custom_size()),
                ..default()
            },
            Transform::from_translation(position.extend(0.0))
        ));
        println!("Spawned one")
    }
}