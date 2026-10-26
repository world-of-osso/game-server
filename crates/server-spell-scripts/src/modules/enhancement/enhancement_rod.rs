//! SimC76f8118 sc_shaman.cpp3423–3486/4471–4485/6546–6564/6698/7514–7530/10028–10039/11478:
//! resolved native casts mirror one accumulated amount into existing caster-owned Rods before application.
use bevy::prelude::*;
use shared::components::{ActiveSpec, Health};

use super::combat;
use combat::auras::ActiveAuras;
use combat::class_progression::KnownSpells;
use combat::random::Rng;
use combat::spell_cast::{self, effects::EffectCtx, targets};
use combat::spell_info::{SpellInfo, SpellStore};

const CONDUCTIVE_ENERGY: u32 = 455123;
const ROD: u32 = 197209;
const ROD_DAMAGE: u32 = 197568;
const ROD_VALUES: u32 = 210689;
const BOLT: u32 = 188196;
const CHAIN: u32 = 188443;
const RIDE_CHAIN: u32 = 211094;
const TEMPEST: u32 = 452201;

fn read_native_coefficient(world: &World) -> f32 {
    world
        .resource::<SpellStore>()
        .get(ROD_VALUES)
        .expect("native Lightning Rod coefficient spell")
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .expect("native Lightning Rod coefficient effect1")
        .base_points
}

pub(crate) fn snapshot_rod_amount(world: &World, spell: &SpellInfo, index: u8, amount: f32) -> f32 {
    // SimC2243–2244 sets the debuff value from its11478 native coefficient, including refreshes.
    if (spell.id, index) == (ROD, 0) {
        read_native_coefficient(world)
    } else {
        amount
    }
}

fn has_conductive_energy(world: &World, caster: Entity) -> bool {
    let enhancement = world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 263);
    let conductive = world
        .get::<KnownSpells>(caster)
        .is_some_and(|known| known.contains(CONDUCTIVE_ENERGY));
    enhancement && conductive
}

fn owns_rod(world: &World, target: Entity, caster: Entity) -> bool {
    world.get::<ActiveAuras>(target).is_some_and(|auras| {
        auras
            .iter()
            .any(|aura| aura.spell_id == ROD && aura.caster == caster)
    })
}

pub(super) fn after_resolution(world: &mut World, ctx: &EffectCtx) {
    if !matches!(ctx.spell.id, BOLT | CHAIN | RIDE_CHAIN | TEMPEST) {
        return;
    }
    if !has_conductive_energy(world, ctx.caster) {
        return;
    }
    let actual_targets = spell_cast::resolving_targets(world, ctx)
        .expect("native Conductive Energy retains its actual TargetInfos");
    let selected: Vec<Entity> = actual_targets
        .iter()
        .copied()
        // SimC6546–6564 uses the actual attackable chain list, including neutral enemies.
        .filter(|target| {
            targets::relation_matches(world, ctx.caster, *target, targets::Relation::Enemy)
        })
        .collect();
    let damage: f32 = spell_cast::resolving_damage(world, ctx)
        .expect("native Conductive Energy retains its resolved damage")
        .iter()
        .map(|(_, amount)| amount)
        .sum();
    mirror_existing_rods(world, ctx, damage);
    if let Some(target) = choose_application_target(world, ctx, &selected) {
        let delay_ms = combat::random::thread_rng()
            .gen_range(10.0_f64..100.0)
            .ceil() as u32;
        spell_cast::queued_triggers::schedule_full_trigger(world, ctx, target, ROD, delay_ms);
    }
}

fn choose_application_target(
    world: &World,
    ctx: &EffectCtx,
    selected: &[Entity],
) -> Option<Entity> {
    let primary = ctx
        .primary_target()
        .filter(|target| selected.contains(target))?;
    if matches!(ctx.spell.id, BOLT | TEMPEST) {
        return Some(primary);
    }
    // SimC6546–6564 keeps the primary when all actual chain targets already have a Rod.
    let mut target = primary;
    for &candidate in selected {
        if !owns_rod(world, candidate, ctx.caster) {
            target = candidate;
            break;
        }
    }
    Some(target)
}

fn mirror_existing_rods(world: &mut World, ctx: &EffectCtx, damage: f32) {
    if damage <= 0.0 {
        return;
    }
    let mut query = world.query::<(Entity, &ActiveAuras, &Health)>();
    let rods: Vec<Entity> = query
        .iter(world)
        .filter_map(|(unit, auras, health)| {
            let owned = auras
                .iter()
                .any(|aura| aura.spell_id == ROD && aura.caster == ctx.caster);
            let alive = health.current > 0.0;
            (owned && alive).then_some(unit)
        })
        .collect();
    let amount = damage * read_native_coefficient(world) / 100.0;
    for target in rods {
        // Native197568 cannot crit and ignores caster damage modifiers; target modifiers remain native.
        spell_cast::cast_triggered_with_effect0_amount(
            world,
            ctx.caster,
            ROD_DAMAGE,
            Some(target),
            ctx.depth + 1,
            amount,
        );
    }
}
