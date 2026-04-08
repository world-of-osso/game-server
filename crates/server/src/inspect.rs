use bevy::prelude::*;
use lightyear::prelude::*;
use shared::class_spec::SelectedTalents;
use shared::components::{EquipmentAppearance, Player};
use shared::protocol::{InspectChannel, InspectSnapshot, InspectStateUpdate, QueryInspectTarget};

use crate::auth::Authenticated;
use crate::networking::PlayerLink;

pub struct InspectPlugin;

impl Plugin for InspectPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, process_inspect_queries);
    }
}

fn process_inspect_queries(
    mut receivers: Query<
        (
            Entity,
            &PlayerLink,
            &mut MessageReceiver<QueryInspectTarget>,
        ),
        With<Authenticated>,
    >,
    players: Query<(&Player, &EquipmentAppearance, &SelectedTalents)>,
    mut senders: Query<&mut MessageSender<InspectStateUpdate>>,
) {
    for (conn_entity, link, mut receiver) in &mut receivers {
        for request in receiver.receive() {
            let target = request.target_entity.map(Entity::from_bits);
            let is_player_target = target.is_some_and(|entity| players.get(entity).is_ok());
            let target_entity = match inspect_target_entity(link.0, target, is_player_target) {
                Ok(target_entity) => target_entity,
                Err(error) => {
                    send_inspect_error(conn_entity, &mut senders, error);
                    continue;
                }
            };
            let Ok((target_player, equipment_appearance, talents)) = players.get(target_entity)
            else {
                send_inspect_error(conn_entity, &mut senders, "inspect target data unavailable");
                continue;
            };
            send_inspect_snapshot(
                conn_entity,
                &mut senders,
                build_inspect_snapshot(target_player, equipment_appearance, talents),
            );
        }
    }
}

fn send_inspect_snapshot(
    conn_entity: Entity,
    senders: &mut Query<&mut MessageSender<InspectStateUpdate>>,
    snapshot: InspectSnapshot,
) {
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<InspectChannel>(InspectStateUpdate {
        snapshot: Some(snapshot),
        message: Some("inspect ready".into()),
        error: None,
    });
}

fn send_inspect_error(
    conn_entity: Entity,
    senders: &mut Query<&mut MessageSender<InspectStateUpdate>>,
    error: impl Into<String>,
) {
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<InspectChannel>(InspectStateUpdate {
        snapshot: None,
        message: None,
        error: Some(error.into()),
    });
}

fn inspect_target_entity(
    player_entity: Entity,
    target_entity: Option<Entity>,
    is_player_target: bool,
) -> Result<Entity, &'static str> {
    let Some(target_entity) = target_entity else {
        return Err("no inspect target selected");
    };
    if target_entity == player_entity {
        return Err("inspect target must be another player");
    }
    if !is_player_target {
        return Err("inspect target is not a player");
    }
    Ok(target_entity)
}

fn build_inspect_snapshot(
    target: &Player,
    equipment_appearance: &EquipmentAppearance,
    talents: &SelectedTalents,
) -> InspectSnapshot {
    InspectSnapshot {
        target_name: target.name.clone(),
        equipment_appearance: equipment_appearance.clone(),
        talents: crate::talent::build_talent_snapshot(talents),
    }
}

#[cfg(test)]
#[path = "inspect_tests.rs"]
mod tests;
