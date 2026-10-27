//! SimC76f8118 sc_evoker.cpp6588–6591/8709–8834/11455–11469.
//! Actual earned marks and native443788 proc admission; external attempts are opt-in.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use super::combat;
use combat::auras::procs::AuraEvent;
use combat::auras::scripts::AuraScripts;
use combat::auras::{ActiveAuras, AuraInstance, RemoveReason, has_aura};
use combat::spell_cast::effects::EffectCtx;
use combat::spell_cast::script_registry::{ContextHitHook, DamageAmountHook, SpellScriptHook};
use combat::spell_cast::scripts::{AfterCastHook, script_effect_value};
use combat::spell_info::SpellInfo;

const TALENT: u32 = 434300;
const MARK: u32 = 434473;
const DRIVER: u32 = 443788;
const DAMAGE: u32 = 434481;
const MELT_ARMOR: u32 = 441172;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::ContextHit(hit_hook),
    SpellScriptHook::DamageAmount(damage_lookup),
    SpellScriptHook::MidAuraRegistration(register),
    SpellScriptHook::AfterResourceValues(essence_spender),
];

fn essence_spender(spell: u32) -> Option<AfterCastHook> {
    matches!(spell, 356995 | 357211).then_some(extend_marks)
}

fn extend_marks(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !has_aura(world, caster, 441212)
        || !spell
            .power_costs
            .iter()
            .any(|cost| cost.power_type == 19 && cost.mana_cost > 0)
    {
        return;
    }
    let milliseconds = (script_effect_value(world, caster, 441212, 0) * 1000.0) as u32;
    let targets: Vec<Entity> = world
        .query::<(Entity, &ActiveAuras)>()
        .iter(world)
        .filter(|(_, auras)| {
            auras
                .iter()
                .any(|aura| aura.spell_id == MARK && aura.caster == caster)
        })
        .map(|(target, _)| target)
        .collect();
    // SimC76f8118 sc_evoker.cpp3199–3212/3261–3275: base Essence cost qualifies,
    // including real Essence Burst; only existing owned marks receive native seconds.
    for target in targets {
        combat::auras::extend_owned_duration(world, caster, target, &[MARK], milliseconds);
    }
}

fn damage_lookup(spell: u32) -> Option<DamageAmountHook> {
    (spell == DAMAGE).then_some(melt_armor_amount)
}

/// SimC76f8118 sc_evoker.cpp8281–8284/8888–8891: the caster's Melt Armor debuff value
/// is a Bombardments target multiplier. Native434481 ignores caster modifiers and the
/// native aura271 mask does not select it, so apply the actual debuff value here.
fn melt_armor_amount(world: &World, caster: Entity, target: Entity, amount: f32) -> f32 {
    world
        .get::<ActiveAuras>(target)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == MELT_ARMOR && aura.caster == caster)
        })
        .map_or(amount, |aura| {
            let effect = aura
                .effects
                .iter()
                .find(|effect| effect.index == 1)
                .expect("native Melt Armor retains damage-taken effect1");
            amount * (1.0 + effect.amount / 100.0)
        })
}

fn hit_hook(spell: u32) -> Option<ContextHitHook> {
    (spell == 356995).then_some(mark_primary)
}

fn mark_primary(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    if ctx.spell.cast_primary_target != Some(target)
        || !ctx.spell.cast_launch_auras.contains(&436336)
        || !has_aura(world, ctx.caster, TALENT)
        || !world
            .get::<ActiveSpec>(ctx.caster)
            .is_some_and(|spec| spec.0 == 1467)
    {
        return;
    }
    // sc_evoker.cpp6588–6591: real Mass impact marks chain_target0 only.
    if !combat::spell_cast::try_cast_triggered(world, ctx.caster, MARK, Some(target), ctx.depth + 1)
    {
        warn!(caster=?ctx.caster, ?target, "Native Bombardments mark refused");
    }
}

fn register(scripts: &mut AuraScripts) {
    let mark = scripts.0.entry(MARK).or_default();
    mark.on_apply = Some(arm_driver);
    mark.on_remove = Some(disarm_driver);
    let driver = scripts.0.entry(DRIVER).or_default();
    driver.check_proc = Some(check_damage);
    driver.on_damage_proc = Some(bombard);
}

fn remove_driver(world: &mut World, target: Entity, caster: Entity) {
    combat::auras::remove_auras(world, target, RemoveReason::Cancelled, |aura| {
        aura.spell_id == DRIVER && aura.caster == caster
    });
}

fn arm_driver(world: &mut World, target: Entity, mark: &AuraInstance) {
    // sc_evoker.cpp11464–11469: each actual mark application resets its target ICD.
    remove_driver(world, target, mark.caster);
    if !combat::spell_cast::try_cast_triggered(world, mark.caster, DRIVER, Some(target), 1) {
        warn!(caster=?mark.caster, ?target, "Native Bombardments proc driver refused");
    }
}

fn disarm_driver(world: &mut World, target: Entity, mark: &AuraInstance, _reason: RemoveReason) {
    remove_driver(world, target, mark.caster);
}

fn check_damage(world: &World, aura: &AuraInstance, event: &AuraEvent) -> bool {
    // sc_evoker.cpp8770–8794: the configured simulated cadence replaces live callbacks.
    if world
        .get::<super::super::bombardments::SimulatedBombardments>(aura.caster)
        .is_some()
        || event.spell_id == Some(DAMAGE)
        || event.damage <= 0.0
    {
        return false;
    }
    event.target.is_some_and(|target| {
        world.get::<ActiveAuras>(target).is_some_and(|auras| {
            auras
                .iter()
                .any(|mark| mark.spell_id == MARK && mark.caster == aura.caster)
        })
    })
}

fn bombard(world: &mut World, target: Entity, driver: &AuraInstance, _event: &AuraEvent) {
    if !super::super::bombardments::launch_damage(world, driver.caster, target) {
        warn!(caster=?driver.caster, ?target, "Native Bombardments damage refused");
    }
}
