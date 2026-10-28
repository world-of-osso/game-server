//! Blood San'layn's cooldown-driven Gift of the San'layn.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, RemoveReason};
use combat::class_progression::KnownSpells;
use combat::spell_cast::{self, TriggeredSpellOverrides, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;
use combat::spell_info::SpellStore;
use shared::components::{ActiveSpec, Health};

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterForegroundSpecial(grant_gift),
    SpellScriptHook::AfterForegroundSpecial(vampiric_proc),
];

// SimC76f8118 sc_death_knight.cpp7279–7300/9531–9536: Vampiric Blood
// and Dancing Rune Weapon grant Gift for their native duration.
fn grant_gift(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !matches!(spell.id, 49028 | 55233)
        || !world
            .get::<ActiveSpec>(caster)
            .is_some_and(|spec| spec.0 == 250)
        || !world
            .get::<KnownSpells>(caster)
            .is_some_and(|known| known.contains(434152))
    {
        return;
    }
    let duration = spell
        .misc
        .as_ref()
        .and_then(|misc| misc.duration)
        .expect("Blood cooldown duration")
        .duration_ms;
    spell_cast::cast_triggered_with_overrides(
        world,
        caster,
        434153,
        Some(caster),
        1,
        TriggeredSpellOverrides {
            duration_ms: Some(duration),
            ..Default::default()
        },
    );
    spell_cast::cast_triggered(world, caster, 433899, Some(caster), 1);
}

// SimC76f8118 sc_death_knight.cpp10131–10134/13835–13853: Death Strike
// may earn a replacement outside Gift; the replacement consumes the proc.
fn vampiric_proc(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 250)
    {
        return;
    }
    let gift = auras::has_aura(world, caster, 434153);
    if spell.id == 433895 && !gift {
        auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
            aura.spell_id == 433899 && aura.caster == caster
        });
        return;
    }
    if spell.id != 49998
        || gift
        || !world
            .get::<KnownSpells>(caster)
            .is_some_and(|known| known.contains(433901))
    {
        return;
    }
    let store = world.resource::<SpellStore>();
    let effect = |id, index| {
        store
            .get(id)
            .expect("Sanlayn spell")
            .effects
            .iter()
            .find(|effect| effect.index == index)
            .expect("Sanlayn effect")
            .base_points
    };
    let mut chance = effect(433901, 0);
    if world
        .get::<KnownSpells>(caster)
        .is_some_and(|known| known.contains(434263))
        && spell
            .cast_primary_target
            .and_then(|target| world.get::<Health>(target))
            .is_some_and(|health| {
                health.max > 0.0 && health.current * 100.0 <= health.max * effect(434263, 0)
            })
    {
        chance += effect(434263, 1);
    }
    if combat::random::random::<f32>() * 100.0 < chance {
        spell_cast::cast_triggered(world, caster, 433899, Some(caster), 1);
    }
}
