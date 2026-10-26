//! SimC76f8118 sc_demon_hunter.cpp4002–4005,4033–4047,4987–4998.
use super::{combat, havoc::has_talent};
use bevy::prelude::*;
use combat::auras::{ActiveAuras, AuraInstance, scripts::AuraScripts};
use combat::spell_cast::{
    SpellHistory,
    script_registry::{Policy, Relationship, SpellScriptHook},
    scripts::AfterCastHook,
};
use combat::spell_info::{SpellInfo, SpellStore};

const BEAMS: &[u32] = &[198013, 452497];
const CYCLE: u32 = 1214887;
const CYCLE_TALENT: u32 = 258887;
const FURIOUS_GAZE: u32 = 343312;
const GAZE_TALENT: u32 = 343311;
// SimC76f8118 sc_demon_hunter.cpp4984–4985: hard Meta adds its
// native duration to an already-live transformation instead of replacing it.
const POLICIES: &[Policy] = &[Policy {
    name: "SIMC_HAVOC_META_REAPPLICATION_EXTENDS",
    spec: 577,
    talent: 191427,
    relationship: Relationship::ReapplicationExtend { payload: 162264 },
}];
pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::Policies(POLICIES),
    SpellScriptHook::NativeAfterCast(after_cast),
    SpellScriptHook::LateAuraRegistration(register_channels),
];

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    match spell {
        191427 => Some(reset_meta_actions),
        198013 | 452497 => Some(advance_cycle),
        _ => None,
    }
}

fn reset_meta_actions(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    if has_talent(world, caster, 388112) {
        for action in [198013, 452497, 188499, 210152] {
            reset_action(world, caster, action, false);
        }
    }
    if has_talent(world, caster, 452409) {
        reset_action(world, caster, 258920, true);
        reset_action(world, caster, 204596, false);
    }
}

pub(super) fn reset_action(world: &mut World, caster: Entity, id: u32, all_charges: bool) {
    let spell = world
        .resource::<SpellStore>()
        .get(id)
        .expect("native Havoc reset action");
    let category = spell
        .categories
        .as_ref()
        .map(|category| (category.charge_category, category.max_charges));
    let remaining = world.get::<SpellHistory>(caster).map_or(0, |history| {
        history.remaining_cooldown_ms(spell, combat::spell_cast::now(world))
    });
    combat::spell_cast::scripts::reduce_cooldown(world, caster, id, remaining as f64 / 1000.0);
    if let Some((category, maximum)) = category.filter(|(category, _)| *category != 0) {
        let count = if all_charges { maximum } else { 1 };
        combat::spell_cast::restore_charges(
            world,
            caster,
            category,
            u8::try_from(count).expect("native Havoc charge count"),
        );
    }
}

fn advance_cycle(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !has_talent(world, caster, CYCLE_TALENT) {
        return;
    }
    let stacks = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == CYCLE && aura.caster == caster)
        })
        .map_or(0, |aura| aura.stacks);
    let milliseconds =
        combat::spell_cast::scripts::script_effect_value(world, caster, CYCLE_TALENT, 0);
    combat::spell_cast::scripts::reduce_cooldown(
        world,
        caster,
        spell.id,
        f64::from(milliseconds) * f64::from(stacks) / 1000.0,
    );
    combat::spell_cast::cast_triggered(world, caster, CYCLE, Some(caster), 1);
}

fn register_channels(scripts: &mut AuraScripts) {
    for beam in BEAMS {
        let script = scripts.0.entry(*beam).or_default();
        script.on_natural_end = Some(finish_beam);
        script.on_remove = Some(super::havoc_eternal::remove_beam);
    }
}

fn finish_beam(world: &mut World, caster: Entity, aura: &AuraInstance) {
    if aura.caster == caster {
        super::havoc_eternal::finish_beam(world, caster);
    }
    if aura.caster == caster && has_talent(world, caster, GAZE_TALENT) {
        combat::spell_cast::cast_triggered(world, caster, FURIOUS_GAZE, Some(caster), 1);
    }
}
