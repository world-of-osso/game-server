//! Accepted battleground/arena teams, not racial factions, define match relationships.
//! TC a352b1fa Object.cpp:2470-2478 allows PvP for affecting players; match entry
//! establishes their teams. Open-world players retain the ordinary faction rules.
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
    let attacker = match_state(attacker);
    let victim = match_state(victim);
    match (attacker, victim) {
        (None, None) => return None,
        // Battleground NPCs have no match snapshot: retain source faction rules.
        // Arena outsider checks remain strict; ordinary player PvP still rejects
        // unmatched players through the existing faction attack rules.
        (Some(state), None) | (None, Some(state)) if state.arena.is_none() => return None,
        _ => {}
    }
    let (Some(attacker), Some(victim), Some(map)) = (attacker, victim, attacker_map) else {
        return Some(ArenaRelation::Blocked);
    };
    if victim_map != Some(map)
        || map.map_id != attacker.map_id
        || map.map_id != victim.map_id
        || attacker.match_id != victim.match_id
        || attacker.phase != victim.phase
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
fn match_state(state: Option<&PlayerPvpState>) -> Option<&shared::protocol::BattlegroundSnapshot> {
    state.and_then(|state| state.battleground.as_ref())
}
/// TC GetControllingPlayer: a charm's controller precedes summon ownership. The
/// entity without either relationship remains its own permission principal.
pub(crate) fn controller(world: &World, entity: Entity) -> Entity {
    if let Some(charm) = world.get::<shared::components::UnitCharm>(entity) {
        return Entity::try_from_bits(charm.charmer).expect("server-owned charm controller");
    }
    match world.get::<UnitSummonedBy>(entity) {
        Some(owner) => Entity::try_from_bits(owner.0).expect("server-owned summon controller"),
        None => crate::game_objects::seaforium_bombs::find_caster_controller(world, entity)
            .unwrap_or(entity),
    }
}

pub(crate) fn relation(world: &World, attacker: Entity, victim: Entity) -> Option<ArenaRelation> {
    relation_between(
        world.get::<PlayerPvpState>(controller(world, attacker)),
        world.get::<PlayerPvpState>(controller(world, victim)),
        world.get::<InMap>(attacker).copied(),
        world.get::<InMap>(victim).copied(),
    )
}
