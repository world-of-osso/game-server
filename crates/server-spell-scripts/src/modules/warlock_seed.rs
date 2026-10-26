//! Actual Seed threshold/original tick → queued native explosion → background DoT.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, AuraInstance, RemoveReason, scripts::AuraScripts};
use combat::class_progression::TraitLoadout;
use combat::random::seq::SliceRandom;
use combat::spell_cast::{
    PlannedEffect, TargetPlan,
    effects::EffectCtx,
    queued_triggers,
    script_registry::{SpellScriptHook, TargetPlanHook},
    targets::{self, Relation},
};
use combat::spell_info::{SpellInfo, SpellStore};
use combat::stats::CharacterStats;
use shared::components::ActiveSpec;

const SEED: u32 = 27243;
const EXPLOSION: u32 = 27285;
const WITHER_TALENT: u32 = 445465;
const SOW_THE_SEEDS: u32 = 196226;
const WITHER_DOT: u32 = 445474;
const CORRUPTION_DOT: u32 = 146739;
const BUDGET_EFFECT: u8 = 2;
const TIMER_EFFECT: u8 = 1;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterAuraApplication(snapshot_seed_budget),
    SpellScriptHook::LandedDamagePolicy(after_landed_damage),
    SpellScriptHook::LateAuraRegistration(register),
    // SimC76f8118 actions1937–1948: refresh cannot postpone the original tick.
    SpellScriptHook::PreservePeriodicPhase(&[SEED]),
    SpellScriptHook::TargetPlan(target_plan),
];

fn has_talent(world: &World, caster: Entity, spell: u32) -> bool {
    world.get::<TraitLoadout>(caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|talent| talent.spell_id == spell)
    })
}

#[derive(Component)]
struct SelectedSeedPrimary(Entity);

/// Read during this Seed's successful-execution callback, before its projectile.
/// The plan below uses this same selection; readers must not draw another target.
pub(super) fn selected_primary(world: &World, caster: Entity) -> Option<Entity> {
    world
        .get::<SelectedSeedPrimary>(caster)
        .map(|primary| primary.0)
}

fn target_plan(spell: u32) -> Option<(Relation, TargetPlanHook)> {
    (spell == SEED).then_some((Relation::Enemy, plan_seed))
}

fn plan_seed(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
) -> TargetPlan {
    let original = explicit.expect("native Seed requires its actual hostile primary");
    let effect = spell
        .effects
        .iter()
        .find(|effect| effect.index == TIMER_EFFECT)
        .expect("native Seed aura owns its candidate selectors");
    let candidates = targets::select_targets(world, caster, explicit, spell, effect, 0, false)
        .expect("native Seed candidate selectors are supported");
    select_and_record_primary(world, caster, original, &candidates);
    let primary = selected_primary(world, caster).expect("Seed plan just recorded its primary");
    let units = select_seed_recipients(world, caster, primary, candidates);
    build_seed_effects(spell, units)
}

fn select_and_record_primary(
    world: &mut World,
    caster: Entity,
    original: Entity,
    candidates: &[Entity],
) {
    // SimC76f8118 actions1851–1879: only actual owned Seed debuffs block a
    // candidate in pinned bugs mode, not seeds in flight. Source fallback is
    // the original target only when no unseeded alternative exists.
    let pool: Vec<_> = candidates
        .iter()
        .copied()
        .filter(|&unit| unit != original && owned_seed(world, caster, unit).is_none())
        .collect();
    let primary = if owned_seed(world, caster, original).is_some() {
        pool.choose(&mut combat::random::thread_rng())
            .copied()
            .unwrap_or(original)
    } else {
        original
    };
    world
        .entity_mut(caster)
        .insert(SelectedSeedPrimary(primary));
}

fn select_seed_recipients(
    world: &World,
    caster: Entity,
    primary: Entity,
    mut candidates: Vec<Entity>,
) -> Vec<Entity> {
    // actions1880–1890: primary first, no duplicates, and seeded secondaries
    // are eligible only if the primary had to fall back to the seeded original.
    let seeded_primary = owned_seed(world, caster, primary).is_some();
    candidates.retain(|&unit| {
        unit != primary && (seeded_primary || owned_seed(world, caster, unit).is_none())
    });
    candidates.shuffle(&mut combat::random::thread_rng());
    candidates.truncate(seed_target_cap(world, caster) - 1);
    candidates.insert(0, primary);
    candidates
}

fn seed_target_cap(world: &World, caster: Entity) -> usize {
    // actions1800–1802: one Seed without selected Sow; native effect0 adds targets.
    if !has_talent(world, caster, SOW_THE_SEEDS) {
        return 1;
    }
    let sow = world
        .resource::<SpellStore>()
        .get(SOW_THE_SEEDS)
        .expect("selected native Sow the Seeds");
    let extra = sow
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Sow target count")
        .base_points;
    1 + extra as usize
}

fn build_seed_effects(spell: &SpellInfo, units: Vec<Entity>) -> TargetPlan {
    spell
        .effects
        .iter()
        .enumerate()
        .map(|(index, _)| PlannedEffect {
            index,
            units: units.clone(),
            dest: None,
            chain_multipliers: Vec::new(),
            selected_units: Vec::new(),
            blocked_units: Vec::new(),
            prevent_hit_script: false,
        })
        .collect()
}

fn is_supported_owner(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| matches!(spec.0, 265 | 267))
}

fn register(scripts: &mut AuraScripts) {
    let script = scripts.0.entry(SEED).or_default();
    script.on_periodic = Some(original_tick);
    script.on_remove = Some(seed_removed);
}

fn snapshot_seed_budget(world: &mut World, target: Entity, aura: &AuraInstance) {
    if aura.spell_id != SEED || !is_supported_owner(world, aura.caster) {
        return;
    }
    // SimC76f8118 actions1927–1934 snapshots caster SP×native effect0 percent.
    // Store the script budget in the actual owned dummy effect, never the catalog.
    let power = world
        .get::<CharacterStats>(aura.caster)
        .expect("native Seed caster power")
        .spell_power;
    let percent = world
        .resource::<SpellStore>()
        .get(SEED)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Seed threshold percent")
        .base_points;
    set_budget(world, target, aura.instance_id, power * percent / 100.0);
}

fn set_budget(world: &mut World, target: Entity, instance: u32, budget: f32) {
    if !auras::set_effect_amount(world, target, instance, BUDGET_EFFECT, budget) {
        warn!(?target, instance, "Actual Seed threshold effect missing");
    }
}

fn owned_seed(world: &World, caster: Entity, target: Entity) -> Option<(u32, f32)> {
    let aura = world
        .get::<ActiveAuras>(target)?
        .iter()
        .find(|aura| aura.spell_id == SEED && aura.caster == caster)?;
    let budget = aura
        .effects
        .iter()
        .find(|effect| effect.index == BUDGET_EFFECT)
        .expect("native Seed dummy budget")
        .amount;
    Some((aura.instance_id, budget))
}

fn after_landed_damage(
    world: &mut World,
    caster: Entity,
    target: Entity,
    spell: Option<u32>,
    amount: f32,
    _periodic: bool,
) {
    if !is_supported_owner(world, caster) {
        return;
    }
    if spell == Some(EXPLOSION) {
        // actions1763–1773: a real explosion chains owned seeds, then applies
        // the background DoT. It does not add Wither's direct445468 payload.
        if let Some((instance, _)) = owned_seed(world, caster, target) {
            auras::remove_instance(world, target, instance, RemoveReason::Consumed);
        }
        apply_background_dot(world, caster, target);
        return;
    }
    if amount <= 0.0 {
        return;
    }
    consume_owner_damage(world, caster, target, amount);
}

fn consume_owner_damage(world: &mut World, caster: Entity, target: Entity, amount: f32) {
    let Some((instance, remaining)) = owned_seed(world, caster, target) else {
        return;
    };
    // warlock.cpp380–405: the Seed caster's actual outgoing damage consumes it.
    let remaining = remaining - amount;
    if remaining <= 0.0 {
        auras::remove_instance(world, target, instance, RemoveReason::Consumed);
    } else {
        set_budget(world, target, instance, remaining);
    }
}

fn original_tick(world: &mut World, target: Entity, aura: &AuraInstance, index: u8) {
    if index != TIMER_EFFECT || !is_supported_owner(world, aura.caster) {
        return;
    }
    // actions1937–1948: refresh extends duration but not this original tick.
    // The native scripted periodic boundary replaces the zero-damage tick.
    auras::remove_instance(world, target, aura.instance_id, RemoveReason::Consumed);
}

fn seed_removed(world: &mut World, target: Entity, aura: &AuraInstance, reason: RemoveReason) {
    let detonate = matches!(reason, RemoveReason::Consumed | RemoveReason::Expired);
    if !detonate || !is_supported_owner(world, aura.caster) {
        return;
    }
    let mut spell = world
        .resource::<SpellStore>()
        .get(SEED)
        .expect("native Seed spell")
        .clone();
    spell.cast_item_level = aura.cast_item_level;
    let ctx = EffectCtx {
        caster: aura.caster,
        spell: &spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    // actions1950–1965 encapsulates every explosion as a zero-delay event;
    // removal is complete before a chain can resolve. Native27285 owns damage.
    queued_triggers::schedule_full_trigger(world, &ctx, target, EXPLOSION, 0);
}

fn apply_background_dot(world: &mut World, caster: Entity, target: Entity) {
    let wither = has_talent(world, caster, WITHER_TALENT);
    let dot = if wither { WITHER_DOT } else { CORRUPTION_DOT };
    // actions1710–1724: real native background Wither/Corruption, no cost or
    // initial direct damage; existing native application preserves owned state.
    if !combat::spell_cast::try_cast_triggered(world, caster, dot, Some(target), 1) {
        warn!(
            ?caster,
            ?target,
            dot,
            "Seed explosion background DoT refused"
        );
    }
}
