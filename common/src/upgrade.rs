use bevy::prelude::*;
use lightyear::prelude::*;

use crate::{Boat, BoatClientId, SubKind, primitives::{WeaponCounter, ZIndex}, protocol::{UpgradeMessage, UpgradeRollback}};


/// - client
///     - listens for [`UpgradeEvent`](crate::primitives::UpgradeEvent) which should be manually triggered
///     - manipulate UI elements manually
/// 
/// - server
///     - receives [`UpgradeMessage`] (automatically managed)
/// 
/// ## Impl details
/// - on [`UpgradeEvent`]
///     - client send a msg to server
///     - upgrate main boat component:
///         - Boat
///         - WeaponCounter  (not used right now)
///         - Sprite
///         - ZIndex possibly
/// 
/// - server on [`UpgradeMessage`]
///     - validate if [`PlayerScore::display`] allow it
///     - if yes, upgrade
///         - Boat
///         - WeaponCounter  (not used right now)
///         - PlayerScore
///         - ZIndex possibly
/// 
/// also push boat to surface if upgrading from sub to ship
pub struct UpgradePlugin;

impl Plugin for UpgradePlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "client")]
        app.add_observer(client::on_upgrade)
            .add_systems(Update, client::recv_rollback);
        #[cfg(feature = "server")]
        app.add_systems(Update, server::recv_upgrade);
    }
}

#[cfg(feature = "client")]
mod client {
    use crate::boat::CircleHud;
    use crate::{BoatReverseNegative, BoatReversePositive, BoatType, CIRCLE_HUD, circle_hud_mesh};
    use crate::protocol::{EntityOnServer, SendToServerOrdered};
    use crate::primitives::UpgradeEvent;
    use super::*;

    #[allow(clippy::too_many_arguments)]
    pub(super) fn on_upgrade(
        trigger: On<UpgradeEvent>,

        mut server_sender: Single<&mut MessageSender<UpgradeMessage>>,
        entity_on_server: Single<&EntityOnServer, With<Controlled>>,

        query: Single<(&mut Boat, &mut WeaponCounter, &mut ZIndex, &mut Sprite), With<Controlled>>,
        // can't access `SpriteMap` in client ...
        asset_server: Res<AssetServer>,

        mut meshes: ResMut<Assets<Mesh>>,
        circle_hud: Single<&Mesh2d, With<CircleHud>>,
        mut indicator_positive: Single<&mut Transform, (With<BoatReversePositive>, Without<BoatReverseNegative>)>,
        mut indicator_negative: Single<&mut Transform, With<BoatReverseNegative>>,

        mut boat_type: ResMut<BoatType>,
    ) {
        let target = trigger.target;
        trace!("Upgrade to {target:?}");
        server_sender.send::<SendToServerOrdered>(UpgradeMessage {
            target,
            entity_on_server: *entity_on_server.into_inner()
        });

        let (mut boat, mut weapon_counter, mut z_index, mut sprite) = query.into_inner();

        sprite.image = asset_server.load(target.file_name().0);
        sprite.custom_size = Some(target.sprite_size());

        upgrade_components(
            target,
            &mut boat,
            &mut weapon_counter,
            &mut z_index
        );

        if let Some(mesh) = meshes.get_mut(*circle_hud) {
            let circle_hud_radius = target.circle_hud_radius();
            *mesh = circle_hud_mesh(circle_hud_radius).into();
            // assume Z is CIRCLE_HUD
            indicator_positive.translation = BoatReversePositive::relative_pos(circle_hud_radius).extend(*CIRCLE_HUD);
            indicator_negative.translation = BoatReverseNegative::relative_pos(circle_hud_radius).extend(*CIRCLE_HUD);
        }

        boat_type.0 = target.sub_kind();
    }
    pub(super) fn recv_rollback(
        mut reader: Single<&mut MessageReceiver<UpgradeRollback>>,
        query: Single<(&mut Boat, &mut WeaponCounter, &mut Sprite), With<Controlled>>,
        asset_server: Res<AssetServer>,

        mut meshes: ResMut<Assets<Mesh>>,
        circle_hud: Single<&Mesh2d, With<CircleHud>>,
        mut indicator_positive: Single<&mut Transform, (With<BoatReversePositive>, Without<BoatReverseNegative>)>,
        mut indicator_negative: Single<&mut Transform, With<BoatReverseNegative>>,
    ) {
        let (mut boat, mut weapon_counter, mut sprite) = query.into_inner();
        for UpgradeRollback { target } in reader.receive() {
            warn!("Rolling level back to {target:?}");
            sprite.image = asset_server.load(target.file_name().0);
            sprite.custom_size = Some(target.sprite_size());

            degrade_components(target, &mut boat, &mut weapon_counter);

            if let Some(mesh) = meshes.get_mut(*circle_hud) {
                let circle_hud_radius = target.circle_hud_radius();
                *mesh = circle_hud_mesh(circle_hud_radius).into();
                // assume Z is CIRCLE_HUD
                indicator_positive.translation = BoatReversePositive::relative_pos(circle_hud_radius).extend(*CIRCLE_HUD);
                indicator_negative.translation = BoatReverseNegative::relative_pos(circle_hud_radius).extend(*CIRCLE_HUD);
            }
        }
    }
}
#[cfg(feature = "server")]
mod server {
    use crate::{primitives::PlayerStats, protocol::SendToClient};
    use super::*;

    pub(super) fn recv_upgrade(
        mut reader: Single<&mut MessageReceiver<UpgradeMessage>>,
        mut sender: ServerMultiMessageSender,
        server: Single<&Server>,
    
        mut stats: Query<(&mut PlayerStats, &BoatClientId, &mut Boat, &mut WeaponCounter, &mut ZIndex)>
    ) {
        for UpgradeMessage { target, entity_on_server } in reader.receive() {
            if let Ok((
                mut stat,
                client_id,
                mut boat, mut weapon_counter, mut z_index
            )) = stats.get_mut(Entity::from_bits(entity_on_server.0)) {
                if stat.can_upgrade(target) {
                    trace!("Client {client_id:?} upgrading to {target:?}");
                    *stat.level_mut() = target.level();
                    upgrade_components(
                        target,
                        &mut boat,
                        &mut weapon_counter,
                        &mut z_index
                    );
                } else {
                    info!("Client {client_id:?}'s upgrade to {target:?} rejected");
                    sender.send::<_, SendToClient>(
                        &UpgradeRollback {
                            target: *boat
                        },
                        &server,
                        &NetworkTarget::Single(client_id.0)
                    ).unwrap();
                }
            } else {
                warn!("Invalid Entity ID");
            }
        }
    }
}

fn upgrade_components(
    target: Boat,
    boat: &mut Boat,
    weapon_counter: &mut WeaponCounter,
    _z_index: &mut ZIndex
) {
    if boat.sub_kind() == SubKind::Submarine && target.sub_kind() != SubKind::Submarine {
        // TODO if sub submerged, push back up
    }
    *boat = target;
    *weapon_counter = WeaponCounter::from_boat(&target);
}

/// [`upgrade_components`] but no `z_index`
#[allow(dead_code)] // rust-analyzer
fn degrade_components(
    target: Boat,
    boat: &mut Boat,
    weapon_counter: &mut WeaponCounter,
) {
    *boat = target;
    *weapon_counter = WeaponCounter::from_boat(&target);
}