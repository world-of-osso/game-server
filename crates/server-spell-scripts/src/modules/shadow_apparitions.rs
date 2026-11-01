//! SimC76f8118 sc_priest_shadow.cpp291–308/439–445/450–549/1296/2609–2650.
//! Native projectile parents transport their damage payload; only the source parent bursts.
use super::{combat, shadow_link, shadow_priest, shadow_yogg};
use bevy::prelude::*;
use combat::auras;
use combat::spell_cast::script_registry::{ContextHitHook, DamageAmountHook, SpellScriptHook};
use combat::spell_cast::{self, effects::EffectCtx};

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::ContextHit(impact_hook),
    SpellScriptHook::DamageAmount(damage_hook),
    SpellScriptHook::TargetContextDamageMultiplier(shadeburst_scale),
    SpellScriptHook::ConsumedEffectValues(&[
        (VOID_APPARITIONS, 1),
        (SPECTRAL_HORRORS, 0),
        (SHADEBURST_TALENT, 0),
    ]),
];
const MADNESS: u32 = 335467;
const APPARITIONS: u32 = 341491;
const NORMAL_MISSILE: u32 = 148859;
const VOID_MISSILE: u32 = 1264175;
const NORMAL_DAMAGE: u32 = 413231;
const VOID_DAMAGE: u32 = 1264176;
const VOID_APPARITIONS: u32 = 1264104;
const VOID_BOLT: u32 = 1264177;
const SHADEBURST_TALENT: u32 = 73510;
const SHADEBURST: u32 = 1231479;
const SPECTRAL_HORRORS: u32 = 407469;
const AUSPICIOUS_SPIRITS: u32 = 155271;
const AUSPICIOUS_GAIN: u32 = 405746;

#[derive(Component)]
struct AuspiciousProgress {
    passive: u32,
    accumulated: f64,
}

fn impact_hook(spell: u32) -> Option<ContextHitHook> {
    match spell {
        MADNESS => Some(spawn_apparitions),
        NORMAL_MISSILE | VOID_MISSILE => Some(burst_at_parent),
        _ => None,
    }
}

pub(super) fn spawn_apparitions(world: &mut World, ctx: &EffectCtx<'_>, primary: Entity) {
    spawn_cohort(world, ctx, Some(primary));
}

pub(super) fn spawn_parentless_apparitions(world: &mut World, ctx: &EffectCtx<'_>) {
    spawn_cohort(world, ctx, None);
}

fn spawn_cohort(world: &mut World, ctx: &EffectCtx<'_>, primary: Option<Entity>) {
    if !auras::has_aura(world, ctx.caster, APPARITIONS) {
        return;
    }
    let recipients = shadow_link::read_vampiric_touch_targets(world, ctx.caster);
    if recipients.is_empty() {
        return;
    }
    // SimC2628–2632: exactly one Yogg stack for this source cohort, not per VT.
    shadow_yogg::earn_cohort(world, ctx);
    let count = recipients.len() as u32;
    for target in recipients {
        let parent = roll_missile(world, ctx.caster);
        launch_apparition(world, ctx, primary, target, parent, count);
    }
}

fn roll_missile(world: &World, caster: Entity) -> u32 {
    if !auras::has_aura(world, caster, VOID_APPARITIONS) {
        return NORMAL_MISSILE;
    }
    let chance = spell_cast::scripts::script_effect_value(world, caster, VOID_APPARITIONS, 1);
    if combat::random::random::<f32>() * 100.0 < chance {
        VOID_MISSILE
    } else {
        NORMAL_MISSILE
    }
}

fn launch_apparition(
    world: &mut World,
    source: &EffectCtx<'_>,
    primary: Option<Entity>,
    target: Entity,
    parent: u32,
    cohort_size: u32,
) {
    let depth = source.depth + 1;
    let Some(mut missile) = spell_cast::load_triggered_spell(world, parent, depth) else {
        return;
    };
    // SimC450–549: native E0 owns the damage missile. E1 resource generation
    // is conditional Auspicious Spirits; E2 Shadeburst belongs only to the parent.
    missile.effects.retain(|effect| effect.index == 0);
    missile.cast_primary_target = primary;
    missile.cast_projectile_cohort_size = Some(cohort_size);
    let Some(plan) = plan_apparition(world, source.caster, &missile, target, depth) else {
        return;
    };
    if parent == VOID_MISSILE {
        launch_void_bolt(world, source, primary, depth);
    }
    // Keep the original source primary distinct from this missile's recipient.
    spell_cast::launch_planned_spell(world, source.caster, &missile, plan, depth);
}

fn launch_void_bolt(world: &mut World, source: &EffectCtx<'_>, primary: Option<Entity>, depth: u8) {
    // SimC543–547/2468: every Void Apparition executes native1264177.
    let target = match primary {
        Some(target) => Some(target),
        // Parentless Yogg keeps null Shadeburst identity; its default Bolt action
        // still uses the priest's actual selected enemy (2850–2858).
        None => world
            .get::<combat::networking::CombatTarget>(source.caster)
            .and_then(|target| target.0),
    };
    spell_cast::cast_triggered(world, source.caster, VOID_BOLT, target, depth);
}

fn plan_apparition(
    world: &mut World,
    caster: Entity,
    missile: &combat::spell_info::SpellInfo,
    target: Entity,
    depth: u8,
) -> Option<spell_cast::TargetPlan> {
    match spell_cast::plan_triggered_spell(world, caster, missile, Some(target), depth) {
        Ok(plan) => Some(plan),
        Err(reason) => {
            warn!(?reason, "Apparition native recipient refused");
            None
        }
    }
}

fn burst_at_parent(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    earn_auspicious_spirits(world, ctx);
    let parent_hit = ctx.primary_target() == Some(target);
    if parent_hit && auras::has_aura(world, ctx.caster, SHADEBURST_TALENT) {
        // SimC sc_priest_shadow.cpp439–445 executes the area after impact,
        // even when the damage child killed the parent target.
        let Some(position) = spell_cast::targets::position(world, target) else {
            warn!(?target, "Shadeburst primary position unavailable");
            return;
        };
        let Some(mut spell) = spell_cast::load_triggered_spell(world, SHADEBURST, ctx.depth + 1)
        else {
            return;
        };
        spell.cast_destination = Some(position.to_array());
        spell_cast::launch_triggered_spell_with_primary(
            world,
            ctx.caster,
            &spell,
            None,
            Some(target),
            ctx.depth + 1,
        );
    }
}

fn earn_auspicious_spirits(world: &mut World, ctx: &EffectCtx<'_>) {
    let passive = world
        .get::<auras::ActiveAuras>(ctx.caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.caster == ctx.caster && aura.spell_id == AUSPICIOUS_SPIRITS)
                .map(|aura| aura.instance_id)
        });
    let Some(passive) = passive else { return };
    let Some(count) = ctx.spell.cast_projectile_cohort_size else {
        return;
    };
    if count == 0 {
        return;
    }
    if world
        .get::<AuspiciousProgress>(ctx.caster)
        .is_none_or(|state| state.passive != passive)
    {
        world.entity_mut(ctx.caster).insert(AuspiciousProgress {
            passive,
            accumulated: combat::random::random::<f64>(),
        });
    }
    // SimC76f8118 sc_priest_shadow.cpp415–424/2373–2386 snapshots cohort size.
    let mut progress = world.get_mut::<AuspiciousProgress>(ctx.caster).unwrap();
    progress.accumulated += 0.8 / f64::from(count).powf(0.8);
    if progress.accumulated < 1.0 {
        return;
    }
    progress.accumulated -= 1.0;
    // Native155271 effect1 modifies 405746 via label2560.
    spell_cast::cast_triggered(
        world,
        ctx.caster,
        AUSPICIOUS_GAIN,
        Some(ctx.caster),
        ctx.depth + 1,
    );
}

fn shadeburst_scale(world: &World, ctx: &EffectCtx<'_>, _: Entity, _: bool) -> f32 {
    if ctx.spell.id != SHADEBURST {
        return 1.0;
    }
    // SimC291–308: native talent E0 supplies the reduced-AoE soft cap.
    let cap = spell_cast::scripts::script_effect_value(world, ctx.caster, SHADEBURST_TALENT, 0);
    let count = spell_cast::resolving_targets(world, ctx)
        .expect("Shadeburst native area selection")
        .len() as f32;
    (cap / count).min(1.0).sqrt()
}

fn damage_hook(spell: u32) -> Option<DamageAmountHook> {
    match spell {
        NORMAL_DAMAGE => Some(spectral_damage),
        VOID_DAMAGE => Some(void_damage),
        _ => None,
    }
}

fn spectral_damage(world: &World, caster: Entity, target: Entity, amount: f32) -> f32 {
    let spectral = auras::has_aura(world, caster, SPECTRAL_HORRORS)
        && shadow_priest::owns_dot(world, caster, target, MADNESS);
    if spectral {
        let percent = spell_cast::scripts::script_effect_value(world, caster, SPECTRAL_HORRORS, 0);
        amount * (1.0 + percent / 100.0)
    } else {
        amount
    }
}

fn void_damage(world: &World, caster: Entity, target: Entity, amount: f32) -> f32 {
    let percent = spell_cast::scripts::script_effect_value(world, caster, VOID_APPARITIONS, 1);
    spectral_damage(world, caster, target, amount) * (1.0 + percent / 100.0)
}
