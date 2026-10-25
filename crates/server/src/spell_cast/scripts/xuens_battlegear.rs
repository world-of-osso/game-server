//! SIMC_WW_XUENS_BATTLEGEAR_FOF_RECOVERY, approved named policy.
//! SimC76f8118 sc_monk.cpp954–958/1143–1150/1541–1546/1596–1609.
//! DB2 owns the dummy time budgets; TC/DB2 supply no conditional consumer.
use bevy::prelude::*;
use std::collections::HashSet;

use crate::auras::{ActiveAuras, AuraInstance};
use crate::class_progression::TraitLoadout;
use crate::spell_cast::effects::EffectCtx;
use crate::spell_info::SpellEffect;

const TALENT: u32 = 392993;
const FOF: u32 = 113656;
pub(super) const SCK: u32 = 101546;
pub(super) const SCK_TICK: u32 = 107270;
pub(super) const RSK_DAMAGE: u32 = 185099;
pub(super) const RWK_DAMAGE: u32 = 468179;
const RSK_TIME: u8 = 1;
const SCK_TIME: u8 = 3;
const SCK_MAXIMUM: u8 = 4;

#[derive(Component)]
struct SckImpacts {
    aura_instance: u32,
    targets: HashSet<Entity>,
}

fn has_selected_talent(world: &World, caster: Entity) -> bool {
    world.get::<TraitLoadout>(caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|spell| spell.spell_id == TALENT)
    })
}

fn read_native_seconds(world: &World, caster: Entity, index: u8) -> f64 {
    f64::from(super::script_effect_value(world, caster, TALENT, index)) / 1000.0
}

/// `chain_target==0`, not the first successful damage or mutable combat focus.
pub(super) fn reduce_fof_on_critical_impact(
    world: &mut World,
    ctx: &EffectCtx,
    target: Entity,
    critical: bool,
) {
    let eligible = critical && has_selected_talent(world, ctx.caster);
    if !eligible {
        return;
    }
    let Some(primary) =
        super::super::resolving_targets(world, ctx).and_then(|targets| targets.first())
    else {
        error!(caster=?ctx.caster, spell=ctx.spell.id, "Xuen critical impact lacks native TargetInfo context");
        return;
    };
    if *primary != target {
        return;
    }
    let seconds = read_native_seconds(world, ctx.caster, RSK_TIME);
    super::reduce_cooldown(world, ctx.caster, FOF, seconds);
}

fn find_parent_instance(world: &World, unit: Entity, caster: Entity) -> Option<u32> {
    world
        .get::<ActiveAuras>(unit)?
        .iter()
        .find(|aura| aura.spell_id == SCK && aura.caster == caster)
        .map(|aura| aura.instance_id)
}

/// One actual parent self-hit follows aura application, including refreshes.
pub(super) fn capture_sck_parent(world: &mut World, ctx: &EffectCtx, unit: Entity) {
    if !has_selected_talent(world, ctx.caster) {
        world.entity_mut(unit).remove::<SckImpacts>();
        return;
    }
    let Some(aura_instance) = find_parent_instance(world, unit, ctx.caster) else {
        world.entity_mut(unit).remove::<SckImpacts>();
        error!(caster=?ctx.caster, ?unit, "Xuen SCK parent lacks its native aura instance");
        return;
    };
    world.entity_mut(unit).insert(SckImpacts {
        aura_instance,
        targets: HashSet::new(),
    });
}

/// SimC inserts every impact, without a result predicate. Native TargetInfos
/// include misses, while an empty/out-of-range native plan contributes nothing.
pub(super) fn record_sck_impacts(world: &mut World, ctx: &EffectCtx, _effect: &SpellEffect) {
    let Some(state) = world.get::<SckImpacts>(ctx.caster) else {
        return;
    };
    if find_parent_instance(world, ctx.caster, ctx.caster) != Some(state.aura_instance) {
        return;
    }
    let targets = super::super::resolving_targets(world, ctx)
        .expect("native SCK impact retains its resolving TargetInfos")
        .to_vec();
    world
        .get_mut::<SckImpacts>(ctx.caster)
        .expect("matching native SCK retains its impact set")
        .targets
        .extend(targets);
}

pub(crate) fn register_xuens_battlegear(scripts: &mut crate::auras::scripts::AuraScripts) {
    // Retain independent on_remove consumers (including Unbroken Rhythm).
    scripts.0.entry(SCK).or_default().on_expire = Some(recover_on_sck_expiry);
}

fn recover_on_sck_expiry(world: &mut World, unit: Entity, aura: &AuraInstance) {
    let Some(state) = world
        .get::<SckImpacts>(unit)
        .filter(|state| state.aura_instance == aura.instance_id)
    else {
        return;
    };
    let count = state.targets.len();
    world.entity_mut(unit).remove::<SckImpacts>();
    if count == 0 || !has_selected_talent(world, aura.caster) {
        return;
    }
    let per_actor = read_native_seconds(world, aura.caster, SCK_TIME);
    let maximum = read_native_seconds(world, aura.caster, SCK_MAXIMUM);
    assert!(
        per_actor > 0.0,
        "native Xuen SCK per-actor time must be positive"
    );
    let actors = count.min((maximum / per_actor) as usize);
    super::reduce_cooldown(world, aura.caster, FOF, per_actor * actors as f64);
}
