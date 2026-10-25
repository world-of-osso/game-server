//! Foreground special-execution proc relationships; native data owns rates and ICDs.
use bevy::prelude::*;

use crate::auras::{ActiveAuras, procs};
use crate::spell_info::{SpellInfo, SpellStore};

struct ForegroundProc {
    class_set: u32,
    passive: u32,
    buff: u32,
    resets: &'static [u32],
}

const PROCS: &[ForegroundProc] = &[
    // SimC76f8118 sc_warrior.cpp1710–1718: special foreground executions
    // roll Sudden Death, grant its buff, start its ICD and reset Execute.
    ForegroundProc {
        class_set: 4,
        passive: 29725,
        buff: 52437,
        resets: &[163201, 280735],
    },
];

pub(in crate::spell_cast) fn after_foreground_special(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
) {
    const DEFENSE_MELEE: u32 = 2;
    if spell
        .categories
        .as_ref()
        .is_none_or(|category| category.defense_type != DEFENSE_MELEE)
    {
        return;
    }
    for policy in PROCS {
        if spell
            .class_options
            .as_ref()
            .is_none_or(|options| options.spell_class_set != policy.class_set)
        {
            continue;
        }
        let instance = world.get::<ActiveAuras>(caster).and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.caster == caster && aura.spell_id == policy.passive)
                .map(|aura| aura.instance_id)
        });
        if let Some(instance) = instance {
            if procs::try_native_policy_proc(world, caster, instance) {
                grant_buff_and_reset_actions(world, caster, policy.buff, policy.resets);
            }
        }
    }
}

pub(super) fn grant_buff_and_reset_actions(
    world: &mut World,
    caster: Entity,
    buff: u32,
    resets: &[u32],
) {
    if !crate::spell_cast::try_cast_triggered(world, caster, buff, Some(caster), 1) {
        warn!(?caster, buff, "Native proc buff refused");
        return;
    }
    for action in resets {
        let spell = world
            .resource::<SpellStore>()
            .get(*action)
            .expect("native proc reset action")
            .clone();
        let now = crate::spell_cast::now(world);
        let remaining = world
            .get::<crate::spell_cast::SpellHistory>(caster)
            .map_or(0, |history| history.remaining_cooldown_ms(&spell, now));
        super::reduce_cooldown(world, caster, *action, remaining as f64 / 1000.0);
        let category = spell
            .categories
            .as_ref()
            .map_or(0, |category| category.charge_category);
        crate::spell_cast::restore_charges(world, caster, category, 1);
    }
}
