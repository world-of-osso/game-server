//! Missing Elemental script relationships; native aura records own every amount.
use crate::auras::ActiveAuras;
use crate::class_progression::{ActiveSpec, TraitLoadout};
use crate::random::seq::SliceRandom;
use crate::spell_info::SpellInfo;
use bevy::prelude::*;

const MASTER_OF_ELEMENTS: u32 = 16166;
const MOTE: u32 = 260734;
const ELEMENTAL_BLAST: u32 = 117014;
const MOTE_PRODUCERS: &[u32] = &[51505];
const BLAST_BUFFS: &[u32] = &[118522, 173183, 173184];

#[derive(Clone, Copy)]
enum Consumption {
    Foreground,
    Always,
    Never,
}

struct DamageRule {
    spell: u32,
    consumption: Consumption,
}

// SimC76f8118 sc_shaman.cpp6321,6488,7323,7363,7586,7640,7964,
// 8189,8212,8350,8494,9844,9907: affected_by_master_of_the_elements.
// execute3297–3300 consumes foreground actions, plus background Flame Shock.
const DAMAGE_RULES: &[DamageRule] = &[
    DamageRule {
        spell: 188196,
        consumption: Consumption::Foreground,
    },
    DamageRule {
        spell: 188443,
        consumption: Consumption::Foreground,
    },
    DamageRule {
        spell: 117014,
        consumption: Consumption::Foreground,
    },
    DamageRule {
        spell: 8042,
        consumption: Consumption::Foreground,
    },
    DamageRule {
        spell: 61882,
        consumption: Consumption::Foreground,
    },
    DamageRule {
        spell: 462620,
        consumption: Consumption::Foreground,
    },
    DamageRule {
        spell: 188389,
        consumption: Consumption::Always,
    },
    DamageRule {
        spell: 196840,
        consumption: Consumption::Foreground,
    },
    DamageRule {
        spell: 452201,
        consumption: Consumption::Foreground,
    },
    DamageRule {
        spell: 463351,
        consumption: Consumption::Never,
    },
    DamageRule {
        spell: 45284,
        consumption: Consumption::Never,
    },
    DamageRule {
        spell: 45297,
        consumption: Consumption::Never,
    },
    DamageRule {
        spell: 120588,
        consumption: Consumption::Never,
    },
    DamageRule {
        spell: 381725,
        consumption: Consumption::Never,
    },
];

// SimC76f8118 sc_shaman.cpp7615–7620 grants overload's buff before execute;
// 7700–7704 grants the ordinary/Fusion-of-Elements root's buff afterward.
const BUFF_RULES: &[(u32, bool)] = &[(117014, false), (120588, true)];

// SimC76f8118 sc_shaman.cpp8759: trigger(effectN(5).base_value()) uses.
// Native zero-based effect4 owns the count; a refresh adds uses up to its cap.
const USE_GRANTS: &[(u32, u32, u8)] = &[(191634, 191634, 4)];

fn selected(world: &World, caster: Entity, talent: u32) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 262)
        && world.get::<TraitLoadout>(caster).is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|spell| spell.spell_id == talent)
        })
}

/// Preserve the earned multiplier through ordinary flight, DoTs and fields.
/// SimC76f8118 sc_shaman.cpp3249–3252 reads MOTE's native buff value once.
pub(in crate::spell_cast) fn capture_damage_modifier(
    world: &World,
    caster: Entity,
    spell: &mut SpellInfo,
    foreground: bool,
) -> Option<u32> {
    let rule = DAMAGE_RULES.iter().find(|rule| rule.spell == spell.id)?;
    if !selected(world, caster, MASTER_OF_ELEMENTS) {
        return None;
    }
    let now = super::super::now(world);
    let aura = world.get::<ActiveAuras>(caster)?.iter().find(|aura| {
        aura.spell_id == MOTE && aura.caster == caster && aura.expires_at.is_none_or(|at| at > now)
    })?;
    let percent = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native MOTE effect0 carries the resolved talent amount")
        .amount;
    spell.cast_damage_multiplier =
        Some(spell.cast_damage_multiplier.unwrap_or(1.0) * (1.0 + percent / 100.0));
    let consumes = match rule.consumption {
        Consumption::Always => true,
        Consumption::Foreground => foreground && !spell.cast_parent_owns_modifier_charges,
        Consumption::Never => false,
    };
    consumes.then_some(aura.instance_id)
}

pub(super) fn before_launch(world: &mut World, caster: Entity, spell: u32, depth: u8) {
    grant_blast_buff(world, caster, spell, true, depth);
}

pub(in crate::spell_cast) fn after_launch(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    consumed: Option<u32>,
    depth: u8,
    foreground: bool,
) {
    if let Some(instance) = consumed {
        crate::auras::spend_stack(world, caster, instance);
    }
    if foreground && !spell.cast_parent_owns_modifier_charges {
        grant_mote(world, caster, spell.id, depth);
    }
    grant_blast_buff(world, caster, spell.id, false, depth);
    grant_native_uses(world, caster, spell.id);
}

fn grant_native_uses(world: &mut World, caster: Entity, spell: u32) {
    let Some(&(_, aura_spell, effect)) = USE_GRANTS.iter().find(|rule| rule.0 == spell) else {
        return;
    };
    if !selected(world, caster, spell) {
        return;
    }
    let uses = super::script_effect_value(world, caster, spell, effect) as u8;
    assert!(uses > 0, "native next-spell grant must supply uses");
    let Some(aura) = world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == aura_spell && aura.caster == caster)
    }) else {
        warn!(
            ?caster,
            spell, aura_spell, "Native use-grant aura missing after cast"
        );
        return;
    };
    // Applying the native aura already contributed one stack.
    let stacks = aura.stacks.saturating_add(uses - 1);
    crate::auras::set_self_cast_stacks(world, caster, aura_spell, stacks);
}

// SimC76f8118 sc_shaman.cpp7245–7247: only normal Lava Burst earns MOTE.
// The native16166 generated HIT mask is broad and would re-grant it on Lightning.
pub(crate) fn register(scripts: &mut crate::auras::scripts::AuraScripts) {
    scripts.0.entry(MASTER_OF_ELEMENTS).or_default().check_proc = Some(block_generated_mote_proc);
}

fn block_generated_mote_proc(
    _world: &World,
    _aura: &crate::auras::AuraInstance,
    _event: &crate::auras::procs::AuraEvent,
) -> bool {
    false
}

fn grant_mote(world: &mut World, caster: Entity, spell: u32, depth: u8) {
    if !MOTE_PRODUCERS.contains(&spell) || !selected(world, caster, MASTER_OF_ELEMENTS) {
        return;
    }
    let parent = world
        .resource::<crate::spell_info::SpellStore>()
        .get(MASTER_OF_ELEMENTS)
        .expect("selected native Master of the Elements metadata");
    let child = parent
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Master of the Elements trigger effect0")
        .trigger_spell;
    if !super::super::try_cast_triggered(world, caster, child, Some(caster), depth + 1) {
        warn!(
            ?caster,
            spell, child, "Native Master of the Elements buff cast refused"
        );
    }
}

fn missing_buffs(world: &World, caster: Entity) -> Vec<u32> {
    let now = super::super::now(world);
    let auras = world
        .get::<ActiveAuras>(caster)
        .expect("native class actor retains auras");
    BLAST_BUFFS
        .iter()
        .copied()
        .filter(|spell| {
            !auras
                .iter()
                .any(|aura| aura.spell_id == *spell && aura.expires_at.is_none_or(|at| at > now))
        })
        .collect()
}

// SimC76f8118 sc_shaman.cpp7545–7577: uniformly choose an absent buff;
// when all three are active, refresh one. Native118522/173183/173184 own values.
pub(super) fn grant_blast_buff(
    world: &mut World,
    caster: Entity,
    spell: u32,
    before: bool,
    depth: u8,
) {
    if !BUFF_RULES.contains(&(spell, before)) || !selected(world, caster, ELEMENTAL_BLAST) {
        return;
    }
    let missing = missing_buffs(world, caster);
    let candidates = if missing.is_empty() {
        BLAST_BUFFS
    } else {
        &missing
    };
    let buff = *candidates
        .choose(&mut crate::random::thread_rng())
        .expect("Elemental Blast has three native buffs");
    if !super::super::try_cast_triggered(world, caster, buff, Some(caster), depth + 1) {
        warn!(
            ?caster,
            spell, buff, "Native Elemental Blast buff cast refused"
        );
    }
}
