//! SimC76f8118 sc_monk.cpp7232–7243: a dodge consumes Elusive Brawler.
use super::combat;
use bevy::prelude::*;
use combat::auras::{
    self, AuraApplication, AuraInstance, AuraSource, RemoveReason, procs::AuraEvent,
    scripts::AuraScripts,
};
use combat::class_progression::TraitLoadout;
use combat::random::Rng;
use combat::spell_cast::{
    effects::EffectCtx, script_registry::SpellScriptHook, scripts::script_effect_value,
};

const ELUSIVE_BRAWLER: u32 = 195630;
const ONE_WITH_THE_WIND: u32 = 454484;
const ELUSIVE_FOOTWORK: u32 = 387046;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::EarlyAuraRegistration(register),
    SpellScriptHook::AvoidanceProcHitMask(avoidance_mask),
    SpellScriptHook::ResolvedDamage(physical_zero_hit),
    SpellScriptHook::ResolvedDamage(critical_blackout_kick),
    SpellScriptHook::LandedDamagePolicy(physical_bleed_tick),
];

// SimC76f8118 sc_monk.cpp7262–7266 grants mastery on a physical hit even
// when the dummy's physical spell_dot application has zero damage.
fn physical_zero_hit(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, _: bool, amount: f32) {
    if ctx.spell.id != 0
        || ctx.spell.school_mask() != 1
        || amount != 0.0
        || !ctx.spell.effects.iter().any(|effect| effect.mechanic == 15)
    {
        return;
    }
    grant_elusive_brawler(world, target);
}

// The recorded physical spell_dot ticks are ID-zero; Stagger is a self-hit.
// SimC76f8118 sc_monk.cpp1430–1439 adds Elusive Footwork stacks on a critical impact.
fn critical_blackout_kick(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    target: Entity,
    critical: bool,
    _: f32,
) {
    if ctx.spell.id != 205523
        || !critical
        || target == ctx.caster
        || !world.get::<TraitLoadout>(ctx.caster).is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|talent| talent.spell_id == ELUSIVE_FOOTWORK)
        })
    {
        return;
    }
    let extra = script_effect_value(world, ctx.caster, ELUSIVE_FOOTWORK, 1).max(0.0) as u32;
    for _ in 0..extra {
        grant_elusive_brawler(world, ctx.caster);
    }
}

fn physical_bleed_tick(
    world: &mut World,
    source: Entity,
    target: Entity,
    spell: Option<u32>,
    _: f32,
    periodic: bool,
) {
    if periodic && spell == Some(0) && source != target {
        grant_elusive_brawler(world, target);
    }
}

fn grant_elusive_brawler(world: &mut World, target: Entity) {
    if !auras::has_aura(world, target, 117906) {
        return;
    }
    auras::apply_aura(
        world,
        &AuraApplication {
            caster: target,
            target,
            spell_id: ELUSIVE_BRAWLER,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
}

fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(117906).or_default().check_proc = Some(mastery_source);
    let script = scripts.0.entry(ELUSIVE_BRAWLER).or_default();
    script.check_proc = Some(is_dodge);
    script.on_proc = Some(consume_on_dodge);
}

// SimC76f8118 sc_monk.cpp1430–1439,7262–7266: only Blackout Kick
// impacts and incoming physical hits grant the mastery, not other own attacks.
fn mastery_source(world: &World, aura: &AuraInstance, event: &AuraEvent) -> bool {
    if event.actor == aura.caster {
        return event.target != Some(aura.caster) && event.spell_id == Some(205523);
    }
    if event.target != Some(aura.caster) {
        return false;
    }
    match event.spell_id {
        None | Some(0) => true,
        Some(id) => world
            .resource::<combat::spell_info::SpellStore>()
            .get(id)
            .is_some_and(|spell| spell.school_mask() == 1),
    }
}

fn avoidance_mask(spell: u32) -> Option<u32> {
    (spell == ELUSIVE_BRAWLER).then_some(auras::procs::PROC_HIT_DODGE)
}

fn is_dodge(_: &World, _: &AuraInstance, event: &AuraEvent) -> bool {
    event.hit_mask & auras::procs::PROC_HIT_DODGE != 0
}

fn consume_on_dodge(world: &mut World, unit: Entity, _: &AuraInstance, _: Option<Entity>) {
    let selected = world.get::<TraitLoadout>(unit).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|talent| talent.spell_id == ONE_WITH_THE_WIND)
    });
    let preserve = if selected {
        script_effect_value(world, unit, ONE_WITH_THE_WIND, 0) / 100.0
    } else {
        0.0
    };
    if !combat::random::thread_rng().gen_bool(f64::from(preserve.clamp(0.0, 1.0))) {
        auras::remove_auras(world, unit, RemoveReason::Consumed, |aura| {
            aura.spell_id == ELUSIVE_BRAWLER && aura.caster == unit
        });
    }
}
