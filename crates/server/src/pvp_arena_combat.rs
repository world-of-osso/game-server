//! Match teams, not racial factions, define arena attack/assist relationships.
use super::PlayerPvpState;
use crate::maps::InMap;
use bevy::prelude::*;
use shared::battleground::BgPhase;
use shared::components::UnitSummonedBy;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ArenaRelation {
    Teammate,
    Enemy { can_attack: bool },
    Blocked,
}

pub(crate) fn relation_between(
    attacker: Option<&PlayerPvpState>,
    victim: Option<&PlayerPvpState>,
    attacker_map: Option<InMap>,
    victim_map: Option<InMap>,
) -> Option<ArenaRelation> {
    let attacker = arena_state(attacker);
    let victim = arena_state(victim);
    if attacker.is_none() && victim.is_none() {
        return None;
    }
    let (Some(attacker), Some(victim), Some(map)) = (attacker, victim, attacker_map) else {
        return Some(ArenaRelation::Blocked);
    };
    if victim_map != Some(map)
        || map.map_id != attacker.map_id
        || map.map_id != victim.map_id
        || attacker.match_id != victim.match_id
        || !attacker.accepted
        || !victim.accepted
    {
        return Some(ArenaRelation::Blocked);
    }
    Some(if attacker.team == victim.team {
        ArenaRelation::Teammate
    } else {
        ArenaRelation::Enemy {
            can_attack: attacker.phase == BgPhase::InProgress
                && victim.phase == BgPhase::InProgress,
        }
    })
}
fn arena_state(state: Option<&PlayerPvpState>) -> Option<&shared::protocol::BattlegroundSnapshot> {
    state
        .and_then(|state| state.battleground.as_ref())
        .filter(|bg| bg.arena.is_some())
}
pub(crate) fn relation(world: &World, attacker: Entity, victim: Entity) -> Option<ArenaRelation> {
    let controller = |entity| match world.get::<UnitSummonedBy>(entity) {
        Some(owner) => Entity::try_from_bits(owner.0).expect("server-owned summon controller"),
        None => entity,
    };
    relation_between(
        world.get::<PlayerPvpState>(controller(attacker)),
        world.get::<PlayerPvpState>(controller(victim)),
        world.get::<InMap>(attacker).copied(),
        world.get::<InMap>(victim).copied(),
    )
}
