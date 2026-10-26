//! Native Haunt RPPM and the source-bound, one-cast Desperate Soul.
use super::combat;
use bevy::prelude::*;
use combat::auras::{ActiveAuras, RemoveReason, procs};
use combat::spell_cast::{
    self,
    effects::EffectCtx,
    script_registry::{EnemyContextHitHook, SpellScriptHook},
    scripts::AfterCastHook,
    targets::{self, Relation},
};
use combat::spell_info::SpellInfo;

const HAUNT: u32 = 48181;
const APEX: u32 = 1261992;
const SUMMON: u32 = 1262094;
const WRATH: u32 = 1278047;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::OwnsPolicyProc(owns_proc),
    SpellScriptHook::EnemyContextHit(haunt_hit),
    SpellScriptHook::NativeAfterCast(after_cast),
    SpellScriptHook::TargetPlan(target_plan),
];

fn owns_proc(spell: u32) -> bool {
    spell == APEX
}

fn haunt_hit(spell: u32) -> Option<EnemyContextHitHook> {
    (spell == HAUNT).then_some(land_haunt)
}

// SimC76f8118 actions2408–2415: the actual landed Haunt replaces this
// caster's previous Haunt, then independently attempts Wrath's native RPPM.
fn land_haunt(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    let previous: Vec<_> = world
        .query::<(Entity, &ActiveAuras)>()
        .iter(world)
        .filter(|(unit, auras)| {
            *unit != target
                && auras
                    .iter()
                    .any(|aura| aura.caster == ctx.caster && aura.spell_id == HAUNT)
        })
        .map(|(unit, _)| unit)
        .collect();
    for unit in previous {
        combat::auras::remove_auras(world, unit, RemoveReason::Replaced, |aura| {
            aura.caster == ctx.caster && aura.spell_id == HAUNT
        });
    }
    try_summon(world, ctx.caster);
}

// SimC76f8118 actions5651–5662: selected rank3 rolls the native hasted
// RPPM stream. Haunt impact and its real pulses share the owning aura timers.
pub(super) fn try_summon(world: &mut World, caster: Entity) {
    let instance = world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.caster == caster && aura.spell_id == APEX)
            .map(|aura| aura.instance_id)
    });
    if let Some(instance) = instance
        && procs::try_native_policy_proc(world, caster, instance)
    {
        spell_cast::cast_triggered(world, caster, SUMMON, Some(caster), 1);
    }
}

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    (spell == WRATH).then_some(expire_soul)
}

// SimC76f8118 pets2547–2563: one actual execution, then dismissal. Mark
// its native expiry instead of destroying the actor inside its cast callback.
fn expire_soul(world: &mut World, caster: Entity, _: &SpellInfo) {
    let now = combat::auras::aura_now(world);
    if let Some(mut summon) = world.get_mut::<spell_cast::summon::SummonedCreature>(caster)
        && summon.spell_id == SUMMON
    {
        summon.expires_at = Some(now);
    }
}

fn target_plan(spell: u32) -> Option<(Relation, spell_cast::script_registry::TargetPlanHook)> {
    (spell == WRATH).then_some((Relation::Enemy, select_wrath_targets))
}

fn owned_haunt(world: &mut World, owner: Entity) -> Option<Entity> {
    let now = combat::auras::aura_now(world);
    world
        .query::<(Entity, &ActiveAuras)>()
        .iter(world)
        .find_map(|(unit, auras)| {
            let active = auras.iter().any(|aura| {
                aura.caster == owner
                    && aura.spell_id == HAUNT
                    && aura.expires_at.is_none_or(|expiry| expiry > now)
            });
            (active && targets::is_alive(world, unit)).then_some(unit)
        })
}

// SimC76f8118 pets2544–2550: a current Haunt overrides the guardian's
// ordinary selected primary. Native1278047 selectors still own the AoE pack.
fn select_wrath_targets(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    _: Option<Entity>,
) -> spell_cast::TargetPlan {
    let owner = world
        .get::<spell_cast::summon::SummonedCreature>(caster)
        .expect("Wrath belongs to an actual native summoned soul")
        .owner;
    let primary = owned_haunt(world, owner)
        .or(spell.cast_primary_target)
        .expect("native Wrath retains its selected primary when no Haunt is active");
    let mut aimed = spell.clone();
    aimed.cast_primary_target = Some(primary);
    aimed
        .effects
        .iter()
        .enumerate()
        .filter(|(_, effect)| effect.effect != 0)
        .map(|(index, _)| plan_wrath_effect(world, caster, &aimed, primary, index))
        .collect()
}

fn plan_wrath_effect(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    primary: Entity,
    index: usize,
) -> spell_cast::PlannedEffect {
    let effect = &spell.effects[index];
    let units = targets::select_targets(world, caster, Some(primary), spell, effect, 0, false)
        .expect("native Wrath AoE selectors are supported");
    spell_cast::PlannedEffect {
        index,
        selected_units: units.clone(),
        units,
        dest: targets::effect_dest(world, caster, Some(primary), effect, None),
        chain_multipliers: Vec::new(),
        blocked_units: Vec::new(),
        prevent_hit_script: false,
    }
}
