//! Demon Hunter reactive damage from hostile hits.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use shared::protocol::{CombatLogEvent, CombatLogKind};

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::HealthChangeObserved(observe)];

fn selected(world: &World, actor: Entity, spell: u32) -> bool {
    world
        .get::<combat::class_progression::TraitLoadout>(actor)
        .is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|talent| talent.spell_id == spell)
        })
}

fn observe(world: &mut World, event: &CombatLogEvent) {
    if event.kind != CombatLogKind::Damage || event.periodic {
        return;
    }
    let (Some(attacker), Some(defender)) = (
        event.source.and_then(Entity::try_from_bits),
        event.target.and_then(Entity::try_from_bits),
    ) else {
        return;
    };
    if !spell_cast::targets::is_hostile(world, defender, attacker) {
        return;
    }
    // SimC76f8118 sc_demon_hunter.cpp12072–12086 uses landed direct hits.
    if event.school_mask & 1 != 0 && selected(world, defender, 320331) {
        let stacks = world
            .get::<combat::auras::ActiveAuras>(defender)
            .and_then(|auras| {
                auras
                    .iter()
                    .find(|aura| aura.spell_id == 258920 && aura.caster == defender)
            })
            .map_or(0, |aura| aura.stacks.max(1));
        for _ in 0..stacks {
            if !spell_cast::try_cast_triggered(world, defender, 320334, Some(attacker), 1) {
                error!(?defender, ?attacker, "Infernal Armor damage refused");
            }
        }
    }
    if event.spell_id.is_none()
        && selected(world, defender, 389729)
        && combat::auras::has_aura(world, defender, 203819)
        && !spell_cast::try_cast_triggered(world, defender, 391159, Some(attacker), 1)
    {
        error!(?defender, ?attacker, "Retaliation damage refused");
    }
}
