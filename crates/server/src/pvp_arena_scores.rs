//! TC Unit.cpp:975 damageDone (not health-clamped),6570 effective healing;
//! Battleground kill credit. Ignore ATTR7_DO_NOT_COUNT_FOR_PVP_SCOREBOARD.
use super::{PlayerPvpState, PvpServerState};
use bevy::prelude::*;
use shared::battleground::BgPhase;
use shared::components::UnitSummonedBy;
use shared::protocol::{CombatLogEvent, CombatLogKind};
const DO_NOT_COUNT_FOR_PVP_SCOREBOARD: u32 = 0x10000000;

fn controller(world: &World, entity: Entity) -> Entity {
    match world.get::<UnitSummonedBy>(entity) {
        Some(owner) => Entity::try_from_bits(owner.0).expect("server-owned summon controller"),
        None => entity,
    }
}
pub(crate) fn record(world: &mut World, event: &CombatLogEvent) {
    if !matches!(
        event.kind,
        CombatLogKind::Damage | CombatLogKind::Heal | CombatLogKind::Death
    ) {
        return;
    }
    let Some(source) = event
        .source
        .and_then(Entity::try_from_bits)
        .map(|entity| controller(world, entity))
    else {
        return;
    };
    let Some(target) = event
        .target
        .and_then(Entity::try_from_bits)
        .map(|entity| controller(world, entity))
    else {
        return;
    };
    let Some(bg) = world
        .get::<PlayerPvpState>(source)
        .and_then(|state| state.battleground.as_ref())
        .filter(|bg| bg.arena.is_some())
    else {
        return;
    };
    let match_id = bg.match_id;
    let same_match = world
        .get::<PlayerPvpState>(target)
        .and_then(|state| state.battleground.as_ref())
        .is_some_and(|target| target.match_id == match_id && target.arena.is_some());
    if !same_match {
        return;
    }
    let excluded = event
        .spell_id
        .and_then(|id| {
            world
                .get_resource::<crate::spell_info::SpellStore>()?
                .get(id)
        })
        .and_then(|spell| spell.misc.as_ref())
        .is_some_and(|misc| misc.attributes[7] & DO_NOT_COUNT_FOR_PVP_SCOREBOARD != 0);
    if excluded {
        return;
    }
    let Some(id) = world
        .get::<crate::persistence::ClientId>(source)
        .map(|id| id.0)
    else {
        return;
    };
    // Cast-only test apps do not install the PvP plugin or its match ledger.
    let Some(mut state) = world.get_resource_mut::<PvpServerState>() else {
        return;
    };
    let Some(matched) = state.battleground_matches.get_mut(&match_id) else {
        return;
    };
    if matched.instance.phase != BgPhase::InProgress {
        return;
    }
    let Some(score) = matched
        .scoreboard
        .iter_mut()
        .find(|score| score.character_id == id)
    else {
        return;
    };
    match event.kind {
        CombatLogKind::Damage => {
            score.damage_done += u64::try_from(event.amount).expect("nonnegative arena damage")
        }
        CombatLogKind::Heal => {
            score.healing_done += u64::try_from(event.amount.saturating_sub(event.overflow))
                .expect("nonnegative effective arena healing")
        }
        CombatLogKind::Death if source != target => score.killing_blows += 1,
        _ => return,
    }
    matched
        .arena
        .as_mut()
        .expect("arena combat ledger")
        .score_dirty = true;
}
