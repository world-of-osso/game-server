//! Native aura chances and ICDs drive repeating-melee charge-reset relationships.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use super::combat;
use combat::auras::{ActiveAuras, procs};
use combat::spell_cast::{self, effects::EffectCtx};
use combat::spell_info::SpellStore;
use combat::spell_cast::script_registry::SpellScriptHook;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterWhiteHit(roll_white_hit),
    SpellScriptHook::ReplacementDirectDamage(roll_replacement_hit),
    SpellScriptHook::OwnsPolicyProc(owns_proc),
];

struct MeleeReset {
    spec: u32,
    passive: u32,
    buff: u32,
    reset: u32,
    replacement: u32,
    chance_effect: u8,
    crit_bonus_effect: u8,
}

const RESETS: &[MeleeReset] = &[
    // SimC76f8118 sc_paladin.cpp999–1014: repeating landed autos roll Art of
    // War, criticals multiply its chance, then buff+ICD+Blade reset(true).
    // sc_paladin.cpp949–958: Crusading Strike replaces the repeating swing.
    // cooldown.hpp63/cooldown.cpp370–379: reset(true) restores ONE charge.
    MeleeReset {
        spec: 70,
        passive: 406064,
        buff: 406086,
        reset: 184575,
        replacement: 408385,
        chance_effect: 0,
        crit_bonus_effect: 1,
    },
];

/// SimC76f8118 sc_paladin.cpp999–1014: only a repeating landed auto may
/// attempt this passive. Raw Dummy proc hints must not spend its native ICD.
pub(super) fn owns_proc(spell: u32) -> bool {
    RESETS.iter().any(|reset| reset.passive == spell)
}

pub(crate) fn roll_white_hit(world: &mut World, caster: Entity, critical: bool) {
    for reset in RESETS {
        roll_reset(world, caster, reset, critical);
    }
}

pub(super) fn roll_replacement_hit(
    world: &mut World,
    ctx: &EffectCtx,
    target: Entity,
    critical: bool,
) {
    // Blessed Champion's secondary recipients are not additional white swings.
    if ctx.spell.cast_primary_target != Some(target) {
        return;
    }
    for reset in RESETS
        .iter()
        .filter(|reset| reset.replacement == ctx.spell.id)
    {
        roll_reset(world, ctx.caster, reset, critical);
    }
}

fn read_chance(
    world: &World,
    caster: Entity,
    reset: &MeleeReset,
    critical: bool,
) -> Option<(u32, f32)> {
    if world.get::<ActiveSpec>(caster)?.0 != reset.spec {
        return None;
    }
    let aura = world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.caster == caster && aura.spell_id == reset.passive)?;
    let amount = |index| {
        aura.effects
            .iter()
            .find(|effect| effect.index == index)
            .map(|effect| effect.amount)
    };
    let chance = amount(reset.chance_effect)?;
    let multiplier = if critical {
        1.0 + amount(reset.crit_bonus_effect)? / 100.0
    } else {
        1.0
    };
    Some((aura.instance_id, chance * multiplier))
}

fn roll_reset(world: &mut World, caster: Entity, reset: &MeleeReset, critical: bool) {
    let Some((instance, chance)) = read_chance(world, caster, reset, critical) else {
        return;
    };
    if !procs::try_policy_proc(world, caster, instance, chance) {
        return;
    }
    if !spell_cast::try_cast_triggered(world, caster, reset.buff, Some(caster), 1) {
        warn!(
            ?caster,
            passive = reset.passive,
            "Native melee reset buff refused"
        );
        return;
    }
    let category = world
        .resource::<SpellStore>()
        .get(reset.reset)
        .and_then(|spell| spell.categories.as_ref())
        .expect("native melee reset charge category")
        .charge_category;
    spell_cast::restore_charges(world, caster, category, 1);
}
