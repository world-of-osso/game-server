//! SimC76f8118 sc_druid.cpp:3210–3220, 3268–3273, 3305–3342, 7482–7514, 8199–8219.
use super::combat;
use bevy::prelude::*;
use combat::auras::{AuraInstance, scripts::AuraScripts};
use combat::class_progression::KnownSpells;
use combat::random::seq::SliceRandom;
use combat::spell_cast::{
    self, effects::EffectCtx, queued_triggers, script_registry::SpellScriptHook, targets,
};
use combat::spell_info::SpellStore;

const TALENT: u32 = 1261566;
const LUNAR: u32 = 48518;
const SOLAR: u32 = 48517;
const LUNAR_BOLT: u32 = 1261700;
const LUNAR_DAMAGE: u32 = 1263137;
const SOLAR_BOLT: u32 = 1261573;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::LateAuraRegistration(register),
    SpellScriptHook::CriticalChance(force_critical),
    SpellScriptHook::TargetContextDamageMultiplier(lunar_soft_cap),
];

#[derive(Component, Default)]
struct BoltCooldowns([f64; 2]);

fn register(scripts: &mut AuraScripts) {
    for spell in [LUNAR, SOLAR] {
        let script = scripts.0.entry(spell).or_default();
        script.on_apply = Some(on_eclipse);
        script.on_refresh = Some(on_eclipse);
    }
}

fn force_critical(_: &World, _: Entity, _: Entity, spell: u32) -> Option<f32> {
    matches!(spell, LUNAR_DAMAGE | SOLAR_BOLT).then_some(100.0)
}

fn on_eclipse(world: &mut World, caster: Entity, aura: &AuraInstance) {
    if aura.caster != caster
        || !world
            .get::<KnownSpells>(caster)
            .is_some_and(|known| known.contains(TALENT))
    {
        return;
    }
    let now = spell_cast::now(world);
    let index = usize::from(aura.spell_id == SOLAR);
    if world
        .get::<BoltCooldowns>(caster)
        .is_some_and(|cooldowns| now < cooldowns.0[index])
    {
        return;
    }
    let Some(target) = world
        .get::<combat::networking::CombatTarget>(caster)
        .and_then(|target| target.0)
    else {
        return;
    };
    let (cooldown_ms, count) = {
        let talent = world
            .resource::<SpellStore>()
            .get(TALENT)
            .expect("Ascendant Eclipses talent");
        let cooldown = talent
            .aura_options
            .as_ref()
            .expect("native bolt cooldown")
            .proc_category_recovery_ms;
        let index = if aura.spell_id == LUNAR { 2 } else { 1 };
        let count = talent
            .effects
            .iter()
            .find(|effect| effect.index == index)
            .expect("native bolt count")
            .base_points as u32;
        (cooldown, count)
    };
    world
        .entity_mut(caster)
        .entry::<BoltCooldowns>()
        .or_default()
        .get_mut()
        .0[index] = now + f64::from(cooldown_ms) / 1000.0;
    let source = world
        .resource::<SpellStore>()
        .get(aura.spell_id)
        .expect("Eclipse source")
        .clone();
    let ctx = EffectCtx {
        caster,
        spell: &source,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    for ordinal in 0..count {
        queued_triggers::schedule_callback(
            world,
            &ctx,
            target,
            ordinal * 200,
            if aura.spell_id == LUNAR {
                cast_lunar
            } else {
                cast_solar
            },
        );
    }
}

fn cast_lunar(world: &mut World, caster: Entity, _: Entity, depth: u8) {
    let Some(origin) = targets::position(world, caster) else {
        return;
    };
    let radius = spell_cast::scripts::script_effect_value(world, caster, TALENT, 0);
    let mut candidates =
        targets::area_units(world, caster, origin, radius, targets::Relation::Enemy, 0);
    candidates.shuffle(&mut combat::random::thread_rng());
    if let Some(target) = candidates.first().copied() {
        spell_cast::cast_triggered(world, caster, LUNAR_BOLT, Some(target), depth);
    }
}

fn cast_solar(world: &mut World, caster: Entity, _: Entity, depth: u8) {
    if let Some(target) = world
        .get::<combat::networking::CombatTarget>(caster)
        .and_then(|target| target.0)
    {
        spell_cast::cast_triggered(world, caster, SOLAR_BOLT, Some(target), depth);
    }
}

fn lunar_soft_cap(world: &World, ctx: &EffectCtx, _: Entity, _: bool) -> f32 {
    if ctx.spell.id != LUNAR_DAMAGE {
        return 1.0;
    }
    let count = spell_cast::resolving_targets(world, ctx)
        .expect("Lunar Bolt damage targets")
        .iter()
        .filter(|target| **target != ctx.caster)
        .count();
    if count > 5 {
        (5.0 / count.min(20) as f32).sqrt()
    } else {
        1.0
    }
}
