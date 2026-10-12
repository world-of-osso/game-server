//! TC a352b1fa Battleground.cpp:1047-1050,416: source preparation aura,
//! removed when the arena opens. No synthetic duration replaces the match clock.
use super::{PlayerPvpState, PvpServerState};
use crate::maps::InMap;
use bevy::prelude::*;
const PREPARATION_SPELL: u32 = 32727;

pub(super) fn sync_preparation(world: &mut World) {
    let mut query = world.query::<(Entity, &PlayerPvpState, &InMap)>();
    let actors: Vec<_> = query
        .iter(world)
        .filter_map(|(entity, state, map)| {
            let bg = state
                .battleground
                .as_ref()
                .filter(|bg| bg.arena.is_some() && bg.accepted)?;
            let matched = world
                .resource::<PvpServerState>()
                .battleground_matches
                .get(&bg.match_id)?;
            (matched.map == Some(*map)).then_some((
                entity,
                matched.instance.phase == shared::battleground::BgPhase::Waiting,
            ))
        })
        .collect();
    for (entity, preparing) in actors {
        let applied = world
            .get::<crate::auras::ActiveAuras>(entity)
            .is_some_and(|auras| auras.iter().any(|aura| aura.spell_id == PREPARATION_SPELL));
        match (preparing, applied) {
            (true, false) => crate::auras::apply_aura(
                world,
                &crate::auras::AuraApplication {
                    caster: entity,
                    target: entity,
                    spell_id: PREPARATION_SPELL,
                    effect_mask: u32::MAX,
                    source: crate::auras::AuraSource::Cast,
                },
            ),
            (false, true) => clear_preparation(world, entity),
            _ => {}
        }
    }
}
pub(super) fn clear_preparation(world: &mut World, entity: Entity) {
    crate::auras::remove_auras(world, entity, crate::auras::RemoveReason::Expired, |aura| {
        aura.spell_id == PREPARATION_SPELL
    });
}
