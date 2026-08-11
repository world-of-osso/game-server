use std::collections::HashSet;

use bevy::ecs::system::SystemParam;

use super::*;

#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlayerDeathTransition {
    pub(crate) player: Entity,
}

type PlayerDeathTransitionQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Position,
        &'static mut Health,
        Option<&'static mut EquippedDurability>,
        Option<&'static PlayerDeathState>,
        &'static crate::persistence::ClientId,
        &'static Player,
    ),
    With<Player>,
>;

#[derive(SystemParam)]
pub(super) struct PlayerDeathParams<'w, 's> {
    players: PlayerDeathTransitionQuery<'w, 's>,
    connections: Query<'w, 's, (Entity, &'static PlayerLink), With<Authenticated>>,
    state: Res<'w, DeathServerState>,
    time: Res<'w, Time>,
    commands: Commands<'w, 's>,
    senders: Query<'w, 's, &'static mut MessageSender<DeathStateUpdate>>,
    replication_dirty: Option<Res<'w, ReplicationDirtyQueue>>,
}

pub(super) fn process_player_death_transitions(
    mut transitions: MessageReader<PlayerDeathTransition>,
    mut params: PlayerDeathParams,
) {
    let now = current_seconds(&params.time);
    let processed_players = transitions
        .read()
        .map(|transition| transition.player)
        .collect::<HashSet<_>>();
    for player_entity in processed_players {
        #[cfg(test)]
        DEATH_PLAYERS_VISITED.fetch_add(1, Ordering::SeqCst);

        let Ok((position, mut health, durability, death_state, client_id, player)) =
            params.players.get_mut(player_entity)
        else {
            continue;
        };
        if !should_process_player_death(&health, death_state) {
            continue;
        }
        health.current = 0.0;
        mark_player_death_replication(params.replication_dirty.as_deref(), player_entity);
        let death_state = build_detected_player_death_state(
            client_id.0,
            position,
            player.race,
            &params.state.graveyards,
            now,
        );
        if let Some(mut durability) = durability {
            crate::durability::apply_death_loss(&mut durability);
        }
        params
            .commands
            .entity(player_entity)
            .insert(death_state.clone());
        if let Some(conn_entity) = connection_for_player(player_entity, &params.connections) {
            send_update(
                conn_entity,
                position,
                death_state,
                Some("you died".into()),
                None,
                &mut params.senders,
            );
        }
    }
}

fn mark_player_death_replication(
    replication_dirty: Option<&ReplicationDirtyQueue>,
    player_entity: Entity,
) {
    let Some(replication_dirty) = replication_dirty else {
        return;
    };
    #[cfg(test)]
    DEATH_REPLICATION_MARKS.fetch_add(1, Ordering::SeqCst);
    replication_dirty.mark(player_entity);
}

pub(crate) fn apply_health_damage(health: &mut Health, damage: f32) -> bool {
    if damage <= 0.0 || health.current <= 0.0 {
        return false;
    }

    health.current = (health.current - damage).max(0.0);
    health.current == 0.0
}

fn should_process_player_death(health: &Health, death_state: Option<&PlayerDeathState>) -> bool {
    health.current <= 0.0 && death_state.is_none_or(|state| state.state == DeathState::Alive)
}
