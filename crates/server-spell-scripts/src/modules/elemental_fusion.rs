//! Native Power of the Maelstrom and Fusion of Elements execution relationships.
use bevy::prelude::*;

use super::combat;
use combat::auras;
use combat::class_progression::{ActiveSpec, TraitLoadout};
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::{SpellInfo, SpellStore};

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterElementalLaunch(after_launch),
    SpellScriptHook::OwnsPolicyProc(owns_power_proc),
];

const POWER_TALENT: u32 = 191861;
const POWER: u32 = 191877;
const FUSION: u32 = 462840;
const BLAST: u32 = 117014;
const BUILDERS: &[u32] = &[188196, 188443];

fn selected(world: &World, caster: Entity, talent: u32) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 262)
        && world.get::<TraitLoadout>(caster).is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|entry| entry.spell_id == talent)
        })
}

fn after_launch(world: &mut World, caster: Entity, spell: &SpellInfo, foreground: bool) {
    if !foreground
        || spell.cast_parent_owns_modifier_charges
        || !selected(world, caster, POWER_TALENT)
    {
        return;
    }
    consume_power(world, caster, spell);
    if BUILDERS.contains(&spell.id) {
        roll_power(world, caster);
    }
}

fn owns_power_proc(spell: u32) -> bool {
    matches!(spell, POWER_TALENT | POWER)
}

fn consume_power(world: &mut World, caster: Entity, spell: &SpellInfo) {
    // SimC76f8118 sc_shaman.cpp2582/3303–3311: the native modifier mask decides eligibility.
    let spent = spell
        .cast_charge_mods
        .iter()
        .flatten()
        .any(|modifier| modifier.aura_spell_id == POWER);
    if spent && selected(world, caster, FUSION) {
        cast_fusion(world, caster, spell.cast_primary_target);
    }
}

fn cast_fusion(world: &mut World, caster: Entity, target: Option<Entity>) {
    let Some(target) = target else {
        warn!(?caster, "Fusion lacks its native primary target");
        return;
    };
    let percent = spell_cast::scripts::script_effect_value(world, caster, FUSION, 0);
    let mut blast = world
        .resource::<SpellStore>()
        .get(BLAST)
        .expect("native Fusion Blast")
        .clone();
    blast.cast_damage_multiplier = Some(percent / 100.0);
    blast.cast_overload_multiplier = Some(percent / 100.0);
    blast.cast_primary_target = Some(target);
    blast.cast_parent_owns_modifier_charges = true;
    // SimC76f8118 sc_shaman.cpp7653–7674: instant, free background Blast.
    if !spell_cast::launch_triggered_spell(world, caster, &blast, Some(target), 1) {
        warn!(?caster, ?target, "Native Fusion Blast refused");
    }
}

fn roll_power(world: &mut World, caster: Entity) {
    // SimC76f8118 sc_shaman.cpp6660–6663/7441–7444: one roll per ordinary builder.
    let chance = spell_cast::scripts::script_effect_value(world, caster, POWER_TALENT, 0);
    if combat::random::random::<f32>() * 100.0 >= chance {
        return;
    }
    if !spell_cast::try_cast_triggered(world, caster, POWER, Some(caster), 1) {
        warn!(?caster, "Native Power of the Maelstrom buff refused");
        return;
    }
    let stacks = world
        .resource::<SpellStore>()
        .get(POWER)
        .and_then(|spell| spell.aura_options.as_ref())
        .expect("native Power stack limit")
        .cumulative_aura as u8;
    auras::set_self_cast_stacks(world, caster, POWER, stacks);
}
