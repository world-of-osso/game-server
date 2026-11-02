//! Dimensional Rift's accumulated proc and native guardian actions.
use super::combat;
use bevy::prelude::*;
use combat::auras::AuraStatMods;
use combat::class_progression::TraitLoadout;
use combat::random::{self as rand, Rng, seq::SliceRandom};
use combat::spell_cast::effects::EffectCtx;
use combat::spell_cast::script_registry::{DamageAmountHook, ScriptReach, SpellScriptHook};
use combat::spell_cast::summon::SummonedCreature;
use combat::spell_cast::{self, scripts::script_effect_value};
use combat::spell_info::SpellInfo;
use combat::stats::CharacterStats;
use shared::components::Npc;

const TALENT: u32 = 1280868;
const AVATAR: u32 = 1245089;
const SHADOWY: u32 = 394235;
const UNSTABLE: u32 = 387979;
const CHAOS: u32 = 394243;
const OVERFIEND: u32 = 434587;
const CHAOS_BOLT: u32 = 394246;

#[derive(Component, Default)]
struct RiftState {
    attempts: u32,
    deck: Vec<u32>,
}

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::DeclaredReach(shadowy_reach),
    SpellScriptHook::DeclaredReach(unstable_reach),
    SpellScriptHook::DeclaredReach(chaos_reach),
    SpellScriptHook::DeclaredReach(overfiend_reach),
    SpellScriptHook::BeforeDamageSnapshot(unstable_barrage_child),
    SpellScriptHook::ResolvedDamage(roll_rift),
    SpellScriptHook::DamageAmount(damage_lookup),
    SpellScriptHook::CriticalChance(chaos_crit),
    SpellScriptHook::ActualDamageMultiplier(roll_mastery),
    SpellScriptHook::ExpectedDamageMultiplier(expected_mastery),
];

fn reach(world: &World, caster: Entity, spell: &SpellInfo, child: u32) -> ScriptReach {
    if matches!(spell.id, 116858 | 17877)
        && combat::spell_cast::script_registry::reach_source_selected(world, caster, TALENT)
    {
        ScriptReach::Child(child)
    } else {
        ScriptReach::None
    }
}
fn shadowy_reach(world: &World, caster: Entity, spell: &SpellInfo) -> ScriptReach {
    reach(world, caster, spell, SHADOWY)
}
fn unstable_reach(world: &World, caster: Entity, spell: &SpellInfo) -> ScriptReach {
    reach(world, caster, spell, UNSTABLE)
}
fn chaos_reach(world: &World, caster: Entity, spell: &SpellInfo) -> ScriptReach {
    reach(world, caster, spell, CHAOS)
}
fn overfiend_reach(world: &World, caster: Entity, spell: &SpellInfo) -> ScriptReach {
    reach(world, caster, spell, OVERFIEND)
}

fn selected(world: &World, caster: Entity, talent: u32) -> bool {
    world.get::<TraitLoadout>(caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|spell| spell.spell_id == talent)
    })
}

// SimC76f8118 sc_warlock_actions.cpp442–445,4293–4295,4781–4782;
// sc_warlock_init.cpp1345–1376: landed spenders use accumulated PRD and a
// shuffled 2/2/2 deck, with one extra Overfiend card when Avatar is selected.
fn roll_rift(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, _: bool, amount: f32) {
    if !matches!(ctx.spell.id, 116858 | 17877)
        || amount <= 0.0
        || !selected(world, ctx.caster, TALENT)
    {
        return;
    }
    let probability = f64::from(script_effect_value(world, ctx.caster, TALENT, 0)) / 100.0;
    let constant = rand::accumulated_proc_constant(probability);
    let avatar = selected(world, ctx.caster, AVATAR);
    let mut state = world
        .entity_mut(ctx.caster)
        .take::<RiftState>()
        .unwrap_or_default();
    state.attempts += 1;
    if rand::random::<f64>() >= constant * f64::from(state.attempts) {
        world.entity_mut(ctx.caster).insert(state);
        return;
    }
    state.attempts = 0;
    if state.deck.is_empty() {
        state.deck = vec![SHADOWY, SHADOWY, UNSTABLE, UNSTABLE, CHAOS, CHAOS];
        if avatar {
            state.deck.push(OVERFIEND);
        }
        state.deck.shuffle(&mut rand::thread_rng());
    }
    let summon = state.deck.pop().expect("refilled dimensional rift deck");
    world.entity_mut(ctx.caster).insert(state);
    let recipient = if summon == OVERFIEND {
        ctx.caster
    } else {
        target
    };
    if !spell_cast::try_cast_triggered(world, ctx.caster, summon, Some(recipient), ctx.depth + 1) {
        warn!(caster = ?ctx.caster, ?target, summon, "Dimensional Rift summon refused");
    }
}

// SimC76f8118 pets.cpp2232–2263 uses 387985 for every Chaos Barrage tick;
// the 387984 native periodic row does not contain that trigger id.
fn unstable_barrage_child(_: &World, _: Entity, spell: &mut SpellInfo) {
    if spell.id == 387984 {
        spell
            .effects
            .iter_mut()
            .find(|effect| effect.index == 0)
            .expect("native Chaos Barrage effect")
            .trigger_spell = 387985;
    }
}

fn chaos_owner(world: &World, caster: Entity) -> Option<Entity> {
    let summon = world.get::<SummonedCreature>(caster)?;
    let npc = world.get::<Npc>(caster)?;
    (summon.spell_id == CHAOS && npc.template_id == 198555).then_some(summon.owner)
}

fn damage_lookup(spell: u32) -> Option<DamageAmountHook> {
    (spell == CHAOS_BOLT).then_some(chaos_damage)
}

// SimC76f8118 pets.cpp2281–2333: guaranteed crit scales by live owner crit
// and an independent Chaotic Energies roll.
fn chaos_damage(world: &World, caster: Entity, _: Entity, amount: f32) -> f32 {
    let Some(owner) = chaos_owner(world, caster) else {
        return amount;
    };
    let crit = world
        .get::<CharacterStats>(owner)
        .expect("rift owner stats")
        .crit_pct
        + world
            .get::<AuraStatMods>(owner)
            .map_or(0.0, |mods| mods.spell_crit_pct);
    amount * (1.0 + crit / 100.0)
}
fn chaos_crit(world: &World, caster: Entity, _: Entity, spell: u32) -> Option<f32> {
    (spell == CHAOS_BOLT && chaos_owner(world, caster).is_some()).then_some(100.0)
}
fn mastery_bounds(world: &World, caster: Entity, spell: &SpellInfo) -> Option<(f32, f32)> {
    if spell.id != CHAOS_BOLT {
        return None;
    }
    let owner = chaos_owner(world, caster)?;
    if !combat::auras::has_aura(world, owner, 77220) {
        return None;
    }
    let mastery = world
        .get::<CharacterStats>(owner)
        .expect("rift owner stats")
        .mastery_pct
        / 100.0;
    let minimum = if selected(world, owner, 387275) {
        script_effect_value(world, owner, 387275, 0) / 100.0
    } else {
        0.5
    };
    Some((1.0 + minimum * mastery, 1.0 + mastery))
}
fn roll_mastery(world: &mut World, ctx: &EffectCtx<'_>, _: Entity) -> f32 {
    let Some((minimum, maximum)) = mastery_bounds(world, ctx.caster, ctx.spell) else {
        return 1.0;
    };
    if minimum == maximum {
        minimum
    } else {
        rand::thread_rng().gen_range(minimum..=maximum)
    }
}
fn expected_mastery(world: &World, caster: Entity, _: Entity, spell: &SpellInfo) -> f32 {
    mastery_bounds(world, caster, spell).map_or(1.0, |(minimum, maximum)| (minimum + maximum) / 2.0)
}
