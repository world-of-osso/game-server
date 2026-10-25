use super::*;

/// Publish the authoritative death/release/resurrection state, including persisted ghosts.
/// TrinityCore a352b1fa: Player::setDeathState and HandleAuraGhost's PLAYER_FLAGS_GHOST.
/// One changed-state projection covers every handler without inferring release from HP.
pub(super) fn sync_replicated_life_state(
    players: Query<(Entity, &PlayerDeathState, Option<&DeathState>), Changed<PlayerDeathState>>,
    mut commands: Commands,
    interest: Option<Res<InterestDirtyQueue>>,
    replication: Option<Res<ReplicationDirtyQueue>>,
) {
    for (entity, death, replicated) in &players {
        if replicated == Some(&death.state) {
            continue;
        }
        commands.entity(entity).insert(death.state);
        if let Some(queue) = interest.as_deref() {
            queue.mark(entity);
        }
        if let Some(queue) = replication.as_deref() {
            queue.mark(entity);
        }
    }
}

pub(super) fn send_update(
    conn_entity: Entity,
    position: &Position,
    death_state: PlayerDeathState,
    message: Option<String>,
    error: Option<String>,
    senders: &mut Query<&mut MessageSender<DeathStateUpdate>>,
) {
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<DeathChannel>(DeathStateUpdate {
        snapshot: Some(build_snapshot(position, &death_state)),
        message,
        error,
    });
}

pub(super) fn send_error(
    conn_entity: Entity,
    senders: &mut Query<&mut MessageSender<DeathStateUpdate>>,
    error: &str,
) {
    let Ok(mut sender) = senders.get_mut(conn_entity) else {
        return;
    };
    sender.send::<DeathChannel>(DeathStateUpdate {
        snapshot: None,
        message: None,
        error: Some(error.into()),
    });
}

pub(super) fn current_seconds(time: &Time) -> u64 {
    time.elapsed_secs_f64().floor() as u64
}

pub(super) fn build_snapshot(position: &Position, death_state: &PlayerDeathState) -> DeathSnapshot {
    DeathSnapshot {
        state: map_state_snapshot(death_state.state),
        corpse: death_state.corpse.as_ref().map(snapshot_from_corpse),
        graveyard: death_state.graveyard.as_ref().map(snapshot_from_graveyard),
        can_resurrect_at_corpse: death_state
            .corpse
            .as_ref()
            .is_some_and(|corpse| in_corpse_range(position.x, position.y, position.z, corpse)),
        spirit_healer_available: death_state.graveyard.as_ref().is_some_and(|graveyard| {
            let dx = position.x - graveyard.x;
            let dy = position.y - graveyard.y;
            let dz = position.z - graveyard.z;
            dx * dx + dy * dy + dz * dz
                <= shared::death::SPIRIT_HEALER_RANGE * shared::death::SPIRIT_HEALER_RANGE
        }),
    }
}

fn map_state_snapshot(state: DeathState) -> DeathStateSnapshot {
    match state {
        DeathState::Alive => DeathStateSnapshot::Alive,
        DeathState::Dead => DeathStateSnapshot::Dead,
        DeathState::Ghost => DeathStateSnapshot::Ghost,
        DeathState::Resurrecting => DeathStateSnapshot::Resurrecting,
    }
}

fn snapshot_from_corpse(corpse: &Corpse) -> DeathPositionSnapshot {
    DeathPositionSnapshot {
        map_id: corpse.map_id,
        x: corpse.x,
        y: corpse.y,
        z: corpse.z,
    }
}

fn snapshot_from_graveyard(graveyard: &Graveyard) -> DeathPositionSnapshot {
    DeathPositionSnapshot {
        map_id: graveyard.map_id,
        x: graveyard.x,
        y: graveyard.y,
        z: graveyard.z,
    }
}
