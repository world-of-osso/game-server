//! SimC76f8118 Arcane execution relationships absent from native trigger rows.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use crate::auras::scripts::{AuraScript, AuraScripts};
use crate::auras::{self, ActiveAuras, AuraInstance};
use crate::class_progression::{KnownSpells, TraitLoadout};
use crate::spell_cast::{self, effects::EffectCtx, queued_triggers};
use crate::spell_info::{SpellInfo, SpellStore};

const CLEARCASTING: u32 = 263725;
const CLEARCASTING_PASSIVE: u32 = 79684;
const SALVO: u32 = 1242974;
const SALVO_TALENT: u32 = 384452;
const CUMULATIVE: u32 = 1296930;
const EXPANDED_MIND: u32 = 1243557;
const PRISMATIC: u32 = 1295942;
const ORB: u32 = 153626;
const MISSILES: u32 = 5143;
const MISSILES_TICK: u32 = 7268;

#[derive(Component, Default)]
struct ClearcastingAttempts(u32);

#[derive(Component)]
struct BarrageSalvo(u8);

#[derive(Component)]
struct BankedOverpowered;

pub(super) fn snapshots_modifier(spell: u32) -> bool {
    // sc_mage.cpp2738/2758/2866/2944: spent modifiers survive projectile flight.
    matches!(spell, 36032 | SALVO | CUMULATIVE | 1223797 | 1277009)
}

fn is_arcane(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 62)
}

fn selected(world: &World, caster: Entity, talent: u32) -> bool {
    world.get::<TraitLoadout>(caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|entry| entry.spell_id == talent)
    })
}

pub(super) fn talent_value(world: &World, caster: Entity, talent: u32, index: u8) -> f32 {
    if selected(world, caster, talent) {
        super::script_effect_value(world, caster, talent, index)
    } else {
        0.0
    }
}

fn roll(percent: f32) -> bool {
    percent >= 100.0
        || (percent > 0.0 && crate::random::random::<f64>() < f64::from(percent) / 100.0)
}

fn grant(world: &mut World, caster: Entity, aura: u32, count: u32) {
    for _ in 0..count {
        auras::apply_aura(
            world,
            &auras::AuraApplication {
                caster,
                target: caster,
                spell_id: aura,
                effect_mask: u32::MAX,
                source: auras::AuraSource::Cast,
            },
        );
    }
}

fn expire(world: &mut World, caster: Entity, spell: u32) {
    auras::remove_auras(world, caster, auras::RemoveReason::Consumed, |aura| {
        aura.caster == caster && aura.spell_id == spell
    });
}

fn stacks(world: &World, caster: Entity, spell: u32) -> u8 {
    world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.caster == caster && aura.spell_id == spell)
        })
        .map_or(0, |aura| aura.stacks)
}

fn grant_salvo(world: &mut World, caster: Entity, count: u32, chance: f32) {
    // sc_mage.cpp7242–7264: talent-gated stacking, native cap and Intuition.
    if !selected(world, caster, SALVO_TALENT) || count == 0 {
        return;
    }
    if !roll(chance) {
        return;
    }
    let before = stacks(world, caster, SALVO);
    grant(world, caster, SALVO, count);
    // sc_mage.cpp7254–7255: only deterministic acquisitions are predicted.
    if chance >= 100.0 {
        #[cfg(any(test, feature = "test-support"))]
        crate::simc_apl_native::reaction::predict(world, caster, SALVO);
    }
    grant_intuition_at_cap(world, caster, before);
}

fn grant_intuition_at_cap(world: &mut World, caster: Entity, before: u8) {
    let cap = world
        .resource::<SpellStore>()
        .get(SALVO)
        .and_then(|spell| spell.aura_options.as_ref())
        .map_or(0, |options| options.cumulative_aura);
    // sc_mage.cpp6404–6406: reaching the cap cannot grant unselected Intuition.
    if selected(world, caster, 1223798)
        && u32::from(stacks(world, caster, SALVO)) == cap
        && u32::from(before) < cap
    {
        grant(world, caster, 1223797, 1);
    }
}

pub(super) fn effect_enabled(world: &World, caster: Entity, spell: u32, index: u8) -> bool {
    // sc_mage.cpp3114–3143: the secondary native Missiles effect needs an AoE talent.
    spell != MISSILES_TICK
        || index != 1
        || auras::has_aura(world, caster, 1277009)
        || selected(world, caster, 453600)
}

pub(crate) fn register(scripts: &mut AuraScripts) {
    super::arcane_touch::register(scripts);
    scripts.0.insert(
        MISSILES,
        AuraScript {
            on_periodic: Some(missiles_tick),
            on_remove: Some(finish_missiles),
            ..Default::default()
        },
    );
}

fn missiles_tick(world: &mut World, target: Entity, aura: &AuraInstance, index: u8) {
    // sc_mage.cpp3194–3206: the hostile native PERIODIC_DUMMY owns each tick.
    if index == 0 && is_arcane(world, aura.caster) {
        spell_cast::cast_triggered(world, aura.caster, MISSILES_TICK, Some(target), 1);
    }
}

pub(super) fn after_launch(world: &mut World, caster: Entity, spell: &SpellInfo, depth: u8) {
    if !is_arcane(world, caster) {
        return;
    }
    match spell.id {
        MISSILES_TICK => generate_missiles_tick(world, caster),
        ORB => launch_orb(world, caster, spell, depth),
        _ => {}
    }
}

fn generate_missiles_tick(world: &mut World, caster: Entity) {
    // sc_mage.cpp3154–3180: once per tick execution, never once per hit target.
    // sc_mage.cpp6389–6391: only the earned native MID2 four-piece enables it.
    if auras::has_aura(world, caster, 1296582) {
        grant(world, caster, CUMULATIVE, 1);
    }
    grant_salvo(world, caster, 1, 100.0);
    let crystal_chance = talent_value(world, caster, 461257, 0);
    let crystal_count = talent_value(world, caster, 461257, 1) as u32;
    grant_salvo(world, caster, crystal_count, crystal_chance);
    if auras::has_aura(world, caster, 1277009) {
        let count = talent_value(world, caster, 1244329, 1) as u32
            + u32::from(selected(world, caster, 1260616));
        grant_salvo(world, caster, count, 100.0);
    }
    if roll(talent_value(world, caster, 461248, 0)) {
        spell_cast::cast_triggered(world, caster, 461524, Some(caster), 1);
    }
}

fn launch_orb(world: &mut World, caster: Entity, spell: &SpellInfo, depth: u8) {
    let Some(target) = spell.cast_primary_target else {
        return;
    };
    // sc_mage.cpp2648–2662: Orb Mastery emits native extra orbs150ms apart.
    let extra = talent_value(world, caster, 1243435, 0) as u32;
    let ctx = EffectCtx {
        caster,
        spell,
        depth,
        dest: None,
        chain_multiplier: 1.0,
    };
    // sc_mage.cpp2612–2616: each selected enemy receives one native Orb bolt.
    let recipients = super::arcane_area::ranged_recipients(world, caster, spell, target);
    let count = talent_value(world, caster, 444256, 3) as u32;
    let maximum = talent_value(world, caster, 444256, 0) as u32;
    for (index, recipient) in recipients.into_iter().enumerate() {
        // sc_mage.cpp2668–2675: Splintering Orbs limits total conjures by
        // native count-per-impact, not by inventing an AoE damage coefficient.
        if count > 0 && index < (maximum / count) as usize {
            super::script_registry::conjure_from_spell(world, caster, spell, recipient, count);
        }
        queued_triggers::schedule_full_trigger(world, &ctx, recipient, 153640, 0);
        for index in 1..=extra {
            queued_triggers::schedule_full_trigger(world, &ctx, recipient, 153640, index * 150);
        }
    }
    let salvo = talent_value(world, caster, EXPANDED_MIND, 1) as u32;
    grant_salvo(world, caster, salvo, 100.0);
}

pub(in crate::spell_cast) fn after_foreground(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
) {
    if !is_arcane(world, caster) {
        return;
    }
    // sc_mage.cpp1855–1863/2694/2841/3205: only eligible executions roll.
    if matches!(spell.id, 30451 | 44425 | MISSILES | ORB | 1449 | 321507) {
        roll_clearcasting(world, caster);
    }
    match spell.id {
        321507 => {
            let count = talent_value(world, caster, 470021, 0) as u32;
            conjure_at_primary(world, caster, spell, count);
            if selected(world, caster, 1243507) {
                trigger_clearcasting(world, caster, 100.0, true);
            }
        }
        365350 => trigger_clearcasting(world, caster, 100.0, true), // sc_mage.cpp3363
        30451 | 1295924 => finish_filler(world, caster, spell.id),
        44425 => finish_barrage(world, caster, spell),
        _ => {}
    }
}

fn trigger_clearcasting(world: &mut World, caster: Entity, chance: f32, allow_predict: bool) {
    // sc_mage.cpp7363–7406: successful Clearcasting also grants Brainstorm
    // and rolls Overpowered; an active Missiles channel banks that proc.
    if !roll(chance) {
        return;
    }
    grant(world, caster, CLEARCASTING, 1);
    if chance >= 100.0 && allow_predict {
        #[cfg(any(test, feature = "test-support"))]
        crate::simc_apl_native::reaction::predict(world, caster, CLEARCASTING);
    }
    if selected(world, caster, 461261) {
        grant(world, caster, 461531, 1);
    }
    roll_overpowered_missiles(world, caster);
}

fn roll_overpowered_missiles(world: &mut World, caster: Entity) {
    if !roll(talent_value(world, caster, 1244329, 0)) {
        return;
    }
    let channeling = world
        .get::<shared::casting::CastState>(caster)
        .is_some_and(|cast| {
            cast.spell_id == MISSILES && cast.cast_type == shared::casting::CastType::Channel
        });
    if channeling {
        world.entity_mut(caster).insert(BankedOverpowered);
    } else {
        grant(world, caster, 1277009, 1);
    }
}

fn finish_missiles(world: &mut World, _: Entity, aura: &AuraInstance, _: auras::RemoveReason) {
    // sc_mage.cpp3214–3223: only the hostile channel application owns finish.
    if !aura
        .effects
        .iter()
        .any(|effect| effect.index == 0 && effect.period_ms > 0)
    {
        return;
    }
    expire(world, aura.caster, 1277009);
    if world
        .entity_mut(aura.caster)
        .take::<BankedOverpowered>()
        .is_some()
    {
        grant(world, aura.caster, 1277009, 1);
    }
}

fn roll_clearcasting(world: &mut World, caster: Entity) {
    let learned = world
        .get::<KnownSpells>(caster)
        .is_some_and(|known| known.contains(CLEARCASTING_PASSIVE));
    if !learned {
        return;
    }
    let chance = f64::from(super::script_effect_value(
        world,
        caster,
        CLEARCASTING_PASSIVE,
        1,
    )) / 100.0;
    // sc_mage.cpp6626–6631: bugs=true uses13 only for finding C; no roll cap.
    let constant = crate::random::capped_accumulated_proc_constant(chance, 13);
    if world.get::<ClearcastingAttempts>(caster).is_none() {
        world
            .entity_mut(caster)
            .insert(ClearcastingAttempts::default());
    }
    let mut attempts = world.get_mut::<ClearcastingAttempts>(caster).unwrap();
    attempts.0 += 1;
    let success = crate::random::random::<f64>() < constant * f64::from(attempts.0);
    if success {
        attempts.0 = 0;
    }
    if success {
        // sc_mage.cpp1860: accumulated-random acquisitions are not predicted.
        trigger_clearcasting(world, caster, 100.0, false);
    }
}

fn conjure_at_primary(world: &mut World, caster: Entity, spell: &SpellInfo, count: u32) {
    if let Some(target) = spell.cast_primary_target {
        super::script_registry::conjure_from_spell(world, caster, spell, target, count);
    }
}

pub(super) fn after_splinter_hit(world: &mut World, ctx: &EffectCtx) {
    if ctx.spell.id != 443763 || !is_arcane(world, ctx.caster) {
        return;
    }
    // sc_mage.cpp5406–5407: Infused Splinters rolls once per actual impact.
    let chance = talent_value(world, ctx.caster, 1261080, 0);
    let count = talent_value(world, ctx.caster, 1261080, 2) as u32;
    grant_salvo(world, ctx.caster, count, chance);
}

fn finish_filler(world: &mut World, caster: Entity, spell: u32) {
    // sc_mage.cpp2860/2866/2942–2950: native grant amounts, then clear spent buffs.
    let effect = if spell == 30451 { 0 } else { 2 };
    let salvo = talent_value(world, caster, EXPANDED_MIND, effect) as u32;
    grant_salvo(world, caster, salvo, 100.0);
    expire(world, caster, CUMULATIVE);
    if spell == 1295924 {
        expire(world, caster, PRISMATIC);
        let chance = talent_value(world, caster, 1295944, 0);
        trigger_clearcasting(world, caster, chance, true);
    }
}

pub(in crate::spell_cast) fn before_cast(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    depth: u8,
) {
    if !is_arcane(world, caster) {
        return;
    }
    if spell.id == 365350 {
        // sc_mage.cpp3348: conjure before the new Surge doubling window exists.
        let count = talent_value(world, caster, 443783, 0) as u32;
        conjure_at_primary(world, caster, spell, count);
    }
    if spell.id != 44425 {
        return;
    }
    // sc_mage.cpp2719–2731: Orb Barrage executes before charge spending/launch.
    let old_salvo = stacks(world, caster, SALVO);
    world.entity_mut(caster).insert(BarrageSalvo(old_salvo));
    let chance = f32::from(old_salvo) * talent_value(world, caster, 384858, 0);
    if roll(chance) {
        if let Some(target) = spell.cast_primary_target {
            spell_cast::cast_triggered(world, caster, ORB, Some(target), depth + 1);
        }
    }
}

fn finish_barrage(world: &mut World, caster: Entity, spell: &SpellInfo) {
    // sc_mage.cpp2740–2759: use earned Salvo, roll Prismatic, then consume Salvo.
    let salvo = stacks(world, caster, SALVO);
    if roll(f32::from(salvo) * talent_value(world, caster, 1295923, 0)) {
        grant(world, caster, PRISMATIC, 1);
    }
    expire(world, caster, 1223797);
    if !auras::has_aura(world, caster, 451038) {
        expire(world, caster, SALVO);
    }
    // sc_mage.cpp2760–2761: Force of Will consumes the actual earned Salvo count.
    let per_splinter = talent_value(world, caster, 444719, 0) as u32;
    if per_splinter > 0 {
        conjure_at_primary(world, caster, spell, u32::from(salvo) / per_splinter);
    }
    // sc_mage.cpp2719–2721/2762–2764: Polished Focus uses pre-Orb Salvo.
    let old_salvo = world
        .entity_mut(caster)
        .take::<BarrageSalvo>()
        .map_or(0, |old| old.0);
    let threshold = talent_value(world, caster, 1261082, 0);
    if threshold > 0.0 && f32::from(old_salvo) >= threshold {
        let refund = talent_value(world, caster, 1261082, 1) as u32;
        grant_salvo(world, caster, refund, 100.0);
    }
}
