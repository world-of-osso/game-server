//! MID2 Seed creates native UA applications with provenance on real async stacks.
use super::combat;
use bevy::prelude::*;
use combat::auras::{ActiveAuras, AuraInstance};
use combat::class_progression::KnownSpells;
use combat::spell_cast::{
    self, effects::EffectCtx, script_registry::SpellScriptHook, scripts::AfterCastHook,
};
use combat::spell_info::SpellInfo;
use std::cmp::Ordering;
use std::collections::HashMap;

const SEED: u32 = 27243;
const UA: u32 = 1259790;
const MID2: u32 = 1296569;
const INSTABILITY: u32 = 1260269;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::NativeAfterCast(after_cast),
    SpellScriptHook::AfterAuraApplication(record_application),
    SpellScriptHook::TargetContextDamageMultiplier(damage_multiplier),
];

#[derive(Component)]
struct ApplyingSeedUa;

#[derive(Default)]
struct SeedApplications {
    instance: u32,
    deadlines: Vec<f64>,
}

#[derive(Component, Default)]
struct SeedUaStacks(HashMap<Entity, SeedApplications>);

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    (spell == SEED).then_some(finish_seed)
}

fn finish_seed(world: &mut World, caster: Entity, spell: &SpellInfo) {
    apply_seed_ua(world, caster);
    // SimC76f8118 actions1913–1924: root and tier UA spend at most one
    // admitted use; an Instability proc earned during a hardcast is retained.
    let admitted = world.get::<ActiveAuras>(caster).is_some_and(|auras| {
        auras.iter().any(|aura| {
            aura.caster == caster
                && aura.spell_id == INSTABILITY
                && spell.cast_proc_mods.contains(&aura.instance_id)
        })
    });
    if admitted {
        super::cast_relationships::decrement(world, caster, INSTABILITY);
    }
}

// SimC76f8118 actions1799–1813/1903–1910: once at successful Seed
// execution on the exact smart primary already selected by the real Seed plan.
fn apply_seed_ua(world: &mut World, caster: Entity) {
    let selected = world.get::<ActiveAuras>(caster).is_some_and(|auras| {
        auras
            .iter()
            .any(|aura| aura.caster == caster && aura.spell_id == MID2)
    });
    let knows_ua = world
        .get::<KnownSpells>(caster)
        .is_some_and(|known| known.contains(UA));
    if !selected || !knows_ua {
        return;
    }
    let Some(primary) = super::warlock_seed::selected_primary(world, caster) else {
        warn!(
            ?caster,
            "Successful Seed has no selected primary for MID2 UA"
        );
        return;
    };
    world.entity_mut(caster).insert(ApplyingSeedUa);
    let applied = spell_cast::try_cast_triggered(world, caster, UA, Some(primary), 1);
    world.entity_mut(caster).remove::<ApplyingSeedUa>();
    if !applied {
        warn!(?caster, ?primary, "Native MID2 Seed UA refused");
    }
}

// Sorted multiset intersection retains only identities still in the actual
// native async scheduler. No second timer runs; the final paid tick sees the
// stack until the existing scheduler actually removes it.
fn intersect_deadlines(seed: &[f64], native: &[f64]) -> Vec<f64> {
    let (mut seed, mut native) = (seed.iter().peekable(), native.iter().peekable());
    let mut retained = Vec::new();
    while let (Some(left), Some(right)) = (seed.peek(), native.peek()) {
        match left.total_cmp(right) {
            Ordering::Less => {
                seed.next();
            }
            Ordering::Greater => {
                native.next();
            }
            Ordering::Equal => {
                retained.push(**left);
                seed.next();
                native.next();
            }
        }
    }
    retained
}

fn record_application(world: &mut World, target: Entity, aura: &AuraInstance) {
    if aura.spell_id != UA {
        return;
    }
    let from_seed = world.get::<ApplyingSeedUa>(aura.caster).is_some();
    if world.get::<SeedUaStacks>(target).is_none() {
        if !from_seed {
            return;
        }
        world.entity_mut(target).insert(SeedUaStacks::default());
    }
    let mut state = world.get_mut::<SeedUaStacks>(target).unwrap();
    let applications = state.0.entry(aura.caster).or_default();
    if applications.instance != aura.instance_id {
        applications.instance = aura.instance_id;
        applications.deadlines.clear();
    }
    applications.deadlines = intersect_deadlines(&applications.deadlines, aura.stack_expirations());
    if from_seed {
        let deadline = *aura
            .stack_expirations()
            .last()
            .expect("native UA has independent stack deadlines");
        applications.deadlines.push(deadline);
        applications.deadlines.sort_by(f64::total_cmp);
    }
}

fn seed_stacks(world: &World, caster: Entity, target: Entity, aura: &AuraInstance) -> usize {
    let Some(applications) = world
        .get::<SeedUaStacks>(target)
        .and_then(|state| state.0.get(&caster))
    else {
        return 0;
    };
    if applications.instance != aura.instance_id {
        return 0;
    }
    intersect_deadlines(&applications.deadlines, aura.stack_expirations()).len()
}

pub(super) fn has_seed_stack(world: &World, caster: Entity, target: Entity) -> bool {
    world.get::<ActiveAuras>(target).is_some_and(|auras| {
        auras.iter().any(|aura| {
            aura.spell_id == UA
                && aura.caster == caster
                && seed_stacks(world, caster, target, aura) > 0
        })
    })
}

// SimC76f8118 warlock.cpp169–172/actions1652–1664: visible full stack
// count is rescaled to ordinary + native20%-effective Seed applications.
// Grasp's extra UA payload deliberately keeps visible stacks (actions2212).
fn damage_multiplier(world: &World, ctx: &EffectCtx<'_>, target: Entity, periodic: bool) -> f32 {
    if ctx.spell.id != UA || !periodic {
        return 1.0;
    }
    let Some(aura) = world.get::<ActiveAuras>(target).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == UA && aura.caster == ctx.caster && aura.stacks > 0)
    }) else {
        return 1.0;
    };
    let seeds = seed_stacks(world, ctx.caster, target, aura);
    if seeds == 0 {
        return 1.0;
    }
    let weight = spell_cast::scripts::script_effect_value(world, ctx.caster, MID2, 0) / 100.0;
    1.0 - (1.0 - weight) * seeds as f32 / f32::from(aura.stacks)
}
