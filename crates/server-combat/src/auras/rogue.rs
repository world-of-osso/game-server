//! Pinned Rogue relationships; native data owns chances, amounts and payloads.
use bevy::prelude::*;

use super::scripts::{AuraScript, AuraScripts};
use super::{ActiveAuras, AuraInstance, RemoveReason, procs};
use crate::spell_cast::{self, effects::EffectCtx, targets::Relation};
use crate::spell_info::{SpellInfo, SpellStore};
use shared::components::PowerType;

const DEADLY_POISON: u32 = 2823;
const DEADLY_INSTANT: u32 = 113780;
const VENOMOUS_WOUNDS: u32 = 79134;
const VENOMOUS_BLEEDS: &[u32] = &[703, 1943];
const DISPEL_POISON: u8 = 4;

#[derive(Component)]
struct VenomousRemainder(f64);

pub(super) fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        DEADLY_POISON,
        AuraScript {
            on_proc: Some(apply_deadly_poison),
            ..Default::default()
        },
    );
    scripts.0.insert(
        VENOMOUS_WOUNDS,
        AuraScript {
            check_proc: Some(block_deferred_wounds),
            on_remove: Some(clear_venomous_remainder),
            ..Default::default()
        },
    );
}

fn apply_deadly_poison(
    world: &mut World,
    caster: Entity,
    aura: &AuraInstance,
    target: Option<Entity>,
) {
    let Some(target) = target else {
        return;
    };
    if !spell_cast::targets::relation_matches(world, caster, target, Relation::Enemy) {
        return;
    }
    let dot = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Deadly Poison coating must retain effect0")
        .trigger_spell;
    assert_ne!(dot, 0, "native Deadly Poison effect0 must name its DoT");
    let now = spell_cast::now(world);
    let ticking = world.get::<ActiveAuras>(target).is_some_and(|auras| {
        auras.iter().any(|active| {
            active.caster == caster
                && active.spell_id == dot
                && active.expires_at.is_none_or(|at| at > now)
        })
    });
    // SimC76f8118 sc_rogue.cpp2981–3005: the first proc applies the native
    // effect0 DoT; a repeat against this caster's ticking DoT also hits113780.
    // Proc masks/chance, spell modifiers, coefficients and duration remain native.
    if ticking {
        spell_cast::cast_triggered(world, caster, DEADLY_INSTANT, Some(target), 1);
    }
    spell_cast::cast_triggered(world, caster, dot, Some(target), 1);
}

// The native periodic-hit producer below owns this proc before aura expiry.
// Queued generic proc dispatch runs after expiry and cannot recover that count.
fn block_deferred_wounds(_world: &World, _aura: &AuraInstance, _event: &procs::AuraEvent) -> bool {
    false
}

fn clear_venomous_remainder(
    world: &mut World,
    unit: Entity,
    _aura: &AuraInstance,
    _reason: RemoveReason,
) {
    world.entity_mut(unit).remove::<VenomousRemainder>();
}

fn is_poisoned(auras: &ActiveAuras, caster: Entity) -> bool {
    auras
        .iter()
        .any(|aura| aura.caster == caster && aura.dispel_type == DISPEL_POISON)
}

fn read_wounds_values(world: &World, caster: Entity) -> Option<(u32, f64, f64)> {
    let aura = world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.caster == caster && aura.spell_id == VENOMOUS_WOUNDS)?;
    let amount = |index| {
        aura.effects
            .iter()
            .find(|effect| effect.index == index)
            .expect("native Venomous Wounds must retain its refund and diminishing effects")
            .amount
    };
    Some((
        aura.instance_id,
        f64::from(amount(0)),
        f64::from(amount(2)) / 100.0,
    ))
}

fn count_poisoned_bleeds(world: &mut World, caster: Entity, spell: u32) -> usize {
    let mut units = world.query::<&ActiveAuras>();
    units
        .iter(world)
        .filter(|auras| {
            auras
                .iter()
                .any(|aura| aura.caster == caster && aura.spell_id == spell)
                && is_poisoned(auras, caster)
        })
        .count()
}

/// SimC76f8118 sc_rogue.cpp4582–4585/5269–5278/8021–8057:
/// every successful poisoned bleed tick refunds its native diminishing amount.
/// Resolve here before death/expiry, while the producing DoT still exists.
pub(crate) fn after_bleed_tick(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    let caster = ctx.caster;
    let poisoned = world
        .get::<ActiveAuras>(target)
        .is_some_and(|auras| is_poisoned(auras, caster));
    if !VENOMOUS_BLEEDS.contains(&ctx.spell.id) || !poisoned {
        return;
    }
    let Some((instance, base, exponent)) = read_wounds_values(world, caster) else {
        return;
    };
    let Some(talent) = roll_wounds_proc(world, caster, instance) else {
        return;
    };
    let bleeds = count_poisoned_bleeds(world, caster, ctx.spell.id);
    assert!(
        bleeds > 0,
        "a successful poisoned bleed tick must retain its native aura"
    );
    let refund = base / (bleeds as f64).powf(exponent);
    grant_wounds_energy(world, caster, &talent, refund);
}

fn roll_wounds_proc(world: &mut World, caster: Entity, instance: u32) -> Option<SpellInfo> {
    let talent = world
        .resource::<SpellStore>()
        .get(VENOMOUS_WOUNDS)
        .expect("active native Venomous Wounds requires its spell metadata")
        .clone();
    let chance = talent
        .aura_options
        .as_ref()
        .expect("native Venomous Wounds requires proc metadata")
        .proc_chance as f32;
    procs::try_policy_proc(world, caster, instance, chance).then_some(talent)
}

fn grant_wounds_energy(world: &mut World, caster: Entity, talent: &SpellInfo, refund: f64) {
    let previous = world
        .get::<VenomousRemainder>(caster)
        .map_or(0.0, |remainder| remainder.0);
    let accumulated = previous + refund;
    let whole = accumulated.floor();
    world
        .entity_mut(caster)
        .insert(VenomousRemainder(accumulated - whole));
    if whole < 1.0 {
        return;
    }
    let ctx = EffectCtx {
        caster,
        spell: talent,
        depth: 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    spell_cast::effects::energize_unit(world, &ctx, caster, PowerType::Energy, whole as f32, false);
}
