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
        &'static crate::maps::InMap,
        Option<&'static mut crate::combat::KillingBlow>,
    ),
    With<Player>,
>;

#[derive(SystemParam)]
pub(crate) struct PlayerDeathParams<'w, 's> {
    players: PlayerDeathTransitionQuery<'w, 's>,
    connections: Query<'w, 's, (Entity, &'static PlayerLink), With<Authenticated>>,
    state: Res<'w, DeathServerState>,
    time: Res<'w, Time>,
    commands: Commands<'w, 's>,
    senders: Query<'w, 's, &'static mut MessageSender<DeathStateUpdate>>,
    replication_dirty: Option<Res<'w, ReplicationDirtyQueue>>,
    #[cfg(test)]
    counters: ResMut<'w, DeathTransitionCounters>,
}

pub(crate) fn process_player_death_transitions(
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
        {
            params.counters.players_visited += 1;
        }

        let Ok((
            position,
            mut health,
            durability,
            death_state,
            client_id,
            player,
            map,
            killing_blow,
        )) = params.players.get_mut(player_entity)
        else {
            continue;
        };
        // Consume the current lethal cause even if resurrection invalidated its
        // queued transition; a later environmental death must not inherit it.
        let killer = killing_blow.and_then(|mut cause| cause.0.take());
        if !should_process_player_death(&health, death_state) {
            continue;
        }
        health.current = 0.0;
        if let Some(replication_dirty) = params.replication_dirty.as_deref() {
            replication_dirty.mark(player_entity);
            #[cfg(test)]
            {
                params.counters.replication_marks += 1;
            }
        }
        let death_state = build_detected_player_death_state(
            client_id.0,
            map.map_id,
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
        if let Some(killer) = killer {
            params.commands.queue(move |world: &mut World| {
                crate::combat::notify_killed_unit(world, killer, player_entity);
            });
        }
        params.commands.queue(move |world: &mut World| {
            crate::damage_meter::record_player_death_transition(world, player_entity);
            crate::mythic_plus::record_death(world, player_entity);
            crate::achievement::record_player_death(world, player_entity);
            crate::pvp::record_player_death(world, player_entity);
        });
        // `Player::setDeathState` -> `Unit::setDeathState` -> `CombatStop`
        // (AzerothCore Player.cpp:1078, Unit.cpp:11439-11441).
        params
            .commands
            .queue(move |world: &mut World| crate::combat::combat_stop(world, player_entity));
        // `Unit::setDeathState` (Unit.cpp:9218) stands the unit up.
        params.commands.queue(move |world: &mut World| {
            crate::stand_state::set_stand_state(
                world,
                player_entity,
                shared::components::StandState::Stand,
            )
        });
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

pub(crate) fn apply_health_damage(health: &mut Health, damage: f32) -> bool {
    if damage <= 0.0 || health.current <= 0.0 {
        return false;
    }

    health.current = (health.current - damage).max(0.0);
    health.current == 0.0
}

/// Deal environmental `damage` (`Player::EnvironmentalDamage`); the combat log line of a
/// dealt hit (`SMSG_ENVIRONMENTAL_DAMAGE_LOG`, Physical as for every type the server
/// deals), overkill in `overflow`. None when the player was already dead.
pub(crate) fn deal_environmental_damage(
    player: Entity,
    kind: shared::protocol::EnvironmentalKind,
    health: &mut Health,
    damage: f32,
) -> Option<shared::protocol::CombatLogEvent> {
    let before = health.current;
    if damage <= 0.0 || before <= 0.0 {
        return None;
    }
    apply_health_damage(health, damage);
    Some(shared::protocol::CombatLogEvent {
        source: None,
        target: Some(player.to_bits()),
        spell_id: None,
        school_mask: 1,
        amount: damage as i32,
        overflow: (damage - before).max(0.0) as i32,
        absorbed: 0,
        resisted: 0,
        blocked: 0,
        crit: false,
        glancing: false,
        periodic: false,
        extra_spell_id: None,
        timestamp_unix_ms: 0,
        kind: shared::protocol::CombatLogKind::Environmental(kind),
    })
}

fn should_process_player_death(health: &Health, death_state: Option<&PlayerDeathState>) -> bool {
    health.current <= 0.0 && death_state.is_none_or(|state| state.state == DeathState::Alive)
}
