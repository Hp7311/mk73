use bevy::prelude::*;
use lightyear::prelude::*;

use crate::{Boat, primitives::WeaponCounter, protocol::{UpgradeMessage, UpgradeRollback}};


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
        {
            app.configure_sets(FixedUpdate, (
                UpgradeSet::UpdateComponents,
                UpgradeSet::HandleWeapons
            ).chain());
            app.add_systems(FixedUpdate, server::recv_upgrade.in_set(UpgradeSet::UpdateComponents));
        }
    }
}

#[cfg(feature = "server")]
pub use server::UpgradeSet;
#[cfg(feature = "client")]
mod client {
    use crate::boat::{CircleHud, SubKind};
    use crate::{BoatReverseNegative, BoatReversePositive, BoatType, CIRCLE_HUD, circle_hud_mesh};
    use crate::protocol::{EntityOnServer, SendToServerOrdered};
    use crate::primitives::{MaybePushToSurface, PlayerStats, UpgradeEvent, UpgradeRollbackEvent};
    use super::*;

    #[allow(clippy::too_many_arguments)]
    pub(super) fn on_upgrade(
        trigger: On<UpgradeEvent>,

        mut server_sender: Single<&mut MessageSender<UpgradeMessage>>,
        entity_on_server: Single<&EntityOnServer, With<Controlled>>,

        query: Single<(&mut Boat, &mut WeaponCounter, &mut PlayerStats), With<Controlled>>,

        mut meshes: ResMut<Assets<Mesh>>,
        circle_hud: Single<&Mesh2d, With<CircleHud>>,
        mut indicator_positive: Single<&mut Transform, (With<BoatReversePositive>, Without<BoatReverseNegative>)>,
        mut indicator_negative: Single<&mut Transform, With<BoatReverseNegative>>,

        mut boat_type: ResMut<BoatType>,

        mut commands: Commands
    ) {
        let target = trigger.target;
        debug!("Upgrading to {target:?}");
        server_sender.send::<SendToServerOrdered>(UpgradeMessage {
            target,
            entity_on_server: *entity_on_server.into_inner()
        });

        let (mut boat, mut weapon_counter, mut player_stats) = query.into_inner();

        *player_stats.level_mut() = target.level();
        if boat.sub_kind() == SubKind::Submarine && target.sub_kind() != SubKind::Submarine {  // maybe add depth to sub diving
            commands.trigger(MaybePushToSurface { last_boat: *boat });
        }
        upgrade_components(
            target,
            &mut boat,
            &mut weapon_counter
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
    #[allow(clippy::too_many_arguments)]
    pub(super) fn recv_rollback(
        mut reader: Single<&mut MessageReceiver<UpgradeRollback>>,
        query: Single<(&mut Boat, &mut WeaponCounter), With<Controlled>>,

        mut meshes: ResMut<Assets<Mesh>>,
        circle_hud: Single<&Mesh2d, With<CircleHud>>,
        mut indicator_positive: Single<&mut Transform, (With<BoatReversePositive>, Without<BoatReverseNegative>)>,
        mut indicator_negative: Single<&mut Transform, With<BoatReverseNegative>>,

        mut commands: Commands
    ) {
        let (mut boat, mut weapon_counter) = query.into_inner();
        for UpgradeRollback { target } in reader.receive() {
            warn!("Rolling level back to {target:?}");
            commands.trigger(UpgradeRollbackEvent(target));

            upgrade_components(target, &mut boat, &mut weapon_counter);

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
    use crate::{primitives::PlayerStats, protocol::SendToClient, BoatClientId};
    use super::*;

    /// making sure that the WeaponCounter is correct when listens for messsages from client firing weapon
    #[derive(Debug, SystemSet, Hash, Eq, Clone, PartialEq)]
    pub enum UpgradeSet {
        UpdateComponents,
        HandleWeapons,
    }

    pub(super) fn recv_upgrade(
        readers: Query<&mut MessageReceiver<UpgradeMessage>>,
        mut sender: ServerMultiMessageSender,
        server: Single<&Server>,
    
        mut stats: Query<(&mut PlayerStats, &BoatClientId, &mut Boat, &mut WeaponCounter)>
    ) {
        for mut reader in readers {
            for UpgradeMessage { target, entity_on_server } in reader.receive() {
                if let Ok((
                    mut stat,
                    client_id,
                    mut boat, mut weapon_counter
                )) = stats.get_mut(Entity::from_bits(entity_on_server.0)) {
                    if stat.can_upgrade(target) {
                        debug!("Client {client_id:?} upgrading to {target:?}");
                        *stat.level_mut() = target.level();
                        upgrade_components(
                            target,
                            &mut boat,
                            &mut weapon_counter
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
                    info!("Invalid Entity ID of boat on server requested");
                }
            }
        }
    }
}

fn upgrade_components(
    target: Boat,
    boat: &mut Boat,
    weapon_counter: &mut WeaponCounter,
) {
    *boat = target;
    // TODO this would refill weapons every time upgrade
    *weapon_counter = WeaponCounter::from_boat(&target);
}