//! SimC76f8118 sc_warlock_actions.cpp1474–1487 and sc_warlock_init.cpp1163–1187.
use super::combat;
use bevy::prelude::*;
use combat::auras::{ActiveAuras, AuraInstance, RemoveReason};
use combat::spell_cast::{
    effects::{self, EffectCtx},
    script_registry::SpellScriptHook,
};
use combat::spell_info::SpellStore;
use shared::components::{ActiveSpec, PowerType};

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::EarlyAuraRegistration(register),
    SpellScriptHook::PeriodicDamageFinished(landed_tick),
];
const AFFLICTION: u32 = 265;
const AGONY: u32 = 980;
const ENERGIZE: u32 = 17941;
const CREEPING_DEATH: u32 = 264000;
// SimC76f8118 sc_warlock.hpp1064: measured threshold increment, not a damage coefficient.
const INCREMENT_MAX: f64 = 0.370;

#[derive(Component)]
struct ShardProgress(f64);

fn register(scripts: &mut combat::auras::scripts::AuraScripts) {
    scripts.0.entry(AGONY).or_default().on_remove = Some(reset_last_agony);
}

fn active_agonies(world: &mut World, caster: Entity) -> usize {
    world
        .query::<&ActiveAuras>()
        .iter(world)
        .filter(|auras| {
            auras
                .iter()
                .any(|aura| aura.spell_id == AGONY && aura.caster == caster)
        })
        .count()
}

fn reset_last_agony(
    world: &mut World,
    _target: Entity,
    aura: &AuraInstance,
    _reason: RemoveReason,
) {
    // SimC76f8118 actions1433–1437: only losing the last owned Agony resets progress.
    if active_agonies(world, aura.caster) == 0 {
        world.entity_mut(aura.caster).remove::<ShardProgress>();
    }
}

fn landed_tick(world: &mut World, ctx: &EffectCtx<'_>) {
    let affliction = world
        .get::<ActiveSpec>(ctx.caster)
        .is_some_and(|spec| spec.0 == AFFLICTION);
    if ctx.spell.id != AGONY || !affliction {
        return;
    }
    let count = active_agonies(world, ctx.caster);
    assert!(count > 0, "landed Agony retains its owned periodic aura");
    let increment_max =
        INCREMENT_MAX * creeping_normalization(world, ctx.caster) * (count as f64).powf(-2.0 / 3.0);
    let initial = world
        .get::<ShardProgress>(ctx.caster)
        .map_or_else(combat::random::random::<f64>, |state| state.0);
    // SimC76f8118 proc_rng.cpp221–273: uniform initial state, threshold1 and rollover.
    let (remaining, proc) =
        advance_progress(initial, combat::random::random::<f64>() * increment_max);
    world
        .entity_mut(ctx.caster)
        .insert(ShardProgress(remaining));
    if proc {
        energize_shard(world, ctx);
    }
}

fn creeping_normalization(world: &World, caster: Entity) -> f64 {
    let Some(aura) = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| auras.iter().find(|aura| aura.spell_id == CREEPING_DEATH))
    else {
        return 1.0;
    };
    let percent = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Creeping Death timing effect")
        .amount;
    let first_rank = world
        .resource::<SpellStore>()
        .get(CREEPING_DEATH)
        .expect("native Creeping Death")
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Creeping Death first-rank effect")
        .base_points;
    // SimC76f8118 init1165–1172: rank2 uses half its negative timing effect (the
    // pinned bug), so both native ranks normalize by the native first-rank value.
    1.0 + f64::from(percent.max(first_rank)) / 100.0
}

fn advance_progress(progress: f64, increment: f64) -> (f64, bool) {
    let total = progress + increment;
    let proc = total >= 1.0;
    (if proc { total - 1.0 } else { total }, proc)
}

fn energize_shard(world: &mut World, parent: &EffectCtx<'_>) {
    let spell = world
        .resource::<SpellStore>()
        .get(ENERGIZE)
        .expect("native Agony energize17941")
        .clone();
    let amount =
        combat::spell_cast::scripts::script_effect_value(world, parent.caster, ENERGIZE, 0);
    let ctx = EffectCtx {
        caster: parent.caster,
        spell: &spell,
        depth: parent.depth + 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    effects::energize_unit(world, &ctx, ctx.caster, PowerType::SoulShards, amount, true);
}

#[cfg(test)]
mod tests {
    use super::advance_progress;
    #[test]
    fn threshold_rollover_preserves_unspent_progress() {
        assert_eq!(advance_progress(0.25, 0.125), (0.375, false));
        assert_eq!(advance_progress(0.875, 0.25), (0.125, true));
        assert_eq!(advance_progress(0.75, 0.25), (0.0, true));
    }
}
