//! SimC76f8118 sc_shaman.cpp4536–4598/5597–5689/6102–6105/12554–12600:
//! native strike payloads and Thorim's actual last foreground lightning selection.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use crate::auras::procs::AuraEvent;
use crate::auras::scripts::{AuraScript, AuraScripts};
use crate::auras::{self, ActiveAuras, AuraInstance};
use crate::class_progression::KnownSpells;
use crate::spell_cast::{self, effects::EffectCtx, targets};
use crate::spell_info::{SpellInfo, SpellStore};

const CRASH: u32 = 187874;
const CRASH_BUFF: u32 = 187878;
const CRASH_STACKS: u32 = 1252415;
const CRASH_DAMAGE: u32 = 195592;
const STORM_UNLEASHED: u32 = 1262713;
const THORIM: u32 = 384444;
const RIDE_THE_LIGHTNING: u32 = 289874;
const RIDE_DAMAGE: u32 = 211094;
const DOOM_WINDS: u32 = 466772;
const ASCENDANCE: u32 = 114051;
const BOLT: u32 = 188196;
const CHAIN: u32 = 188443;
const TEMPEST: u32 = 452201;
const TEMPEST_PROC: u32 = 454015;

#[derive(Component)]
struct ThorimSelection(u32);

fn enhancement(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 263)
}

fn learned(world: &World, caster: Entity, spell: u32) -> bool {
    world
        .get::<KnownSpells>(caster)
        .is_some_and(|known| known.contains(spell))
}

pub(super) fn register(scripts: &mut AuraScripts) {
    // SimC5597–5598/5273–5275 qualifies the landed strike root, not arbitrary
    // native generic proc flags or both individual weapon children.
    // SimC10280–10283 grants Descending Skies only on actual Ascendance;
    // 5610/12249–12261 rolls Awakening Storms only for strike roots.
    scripts.0.insert(
        1264688,
        AuraScript {
            check_proc: Some(check_descending_skies),
            ..Default::default()
        },
    );
    scripts.0.insert(
        455129,
        AuraScript {
            check_proc: Some(check_awakening_storms),
            ..Default::default()
        },
    );
    scripts.0.insert(
        CRASH_STACKS,
        AuraScript {
            check_proc: Some(strike_hook_owns_payload),
            ..Default::default()
        },
    );
}

fn check_descending_skies(_world: &World, _aura: &AuraInstance, event: &AuraEvent) -> bool {
    matches!(event.spell_id, Some(114051 | 1219480))
}

fn check_awakening_storms(_world: &World, _aura: &AuraInstance, event: &AuraEvent) -> bool {
    matches!(event.spell_id, Some(17364 | 115356))
}

pub(super) fn grant_elemental_assault(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    // SimC76f8118 sc_shaman.cpp5225/5605–5608/12023–12039: normal roots, including misses.
    const ELEMENTAL_ASSAULT: u32 = 210853;
    if !enhancement(world, caster) || !learned(world, caster, ELEMENTAL_ASSAULT) {
        return;
    }
    let chance = super::script_effect_value(world, caster, ELEMENTAL_ASSAULT, 2);
    if crate::random::random::<f32>() * 100.0 >= chance {
        return;
    }
    let count = super::script_effect_value(world, caster, ELEMENTAL_ASSAULT, 1) as u8;
    super::maelstrom_weapon::generate(world, caster, count);
}

fn strike_hook_owns_payload(_world: &World, _aura: &AuraInstance, _event: &AuraEvent) -> bool {
    false
}

pub(in crate::spell_cast) fn check_cast(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
) -> Result<(), spell_cast::CastFailure> {
    // SimC sc_shaman.cpp10041–10049: the selected talent alone never readies Tempest.
    if enhancement(world, caster)
        && spell.id == TEMPEST
        && !auras::has_aura(world, caster, TEMPEST_PROC)
    {
        return Err(spell_cast::CastFailure::CasterAuraState);
    }
    Ok(())
}

pub(super) fn after_crash(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    // SimC12758–12763: Storm Unleashed owns a different, native async counter.
    let buff = if learned(world, caster, STORM_UNLEASHED) {
        CRASH_STACKS
    } else {
        CRASH_BUFF
    };
    spell_cast::cast_triggered(world, caster, buff, Some(caster), 1);
}

pub(super) fn on_strike(world: &mut World, ctx: &EffectCtx, target: Entity) {
    if !enhancement(world, ctx.caster) {
        return;
    }
    // Crash is AoE, but Thorim is invoked only for its actual selected primary.
    if ctx.primary_target() != Some(target) {
        return;
    }
    if matches!(ctx.spell.id, 17364 | 115356 | 60103) {
        cast_crash_payload(world, ctx.caster, target, ctx.depth);
    }
    let invoke = match ctx.spell.id {
        115356 => true,
        17364 => auras::has_aura(world, ctx.caster, DOOM_WINDS),
        CRASH => {
            auras::has_aura(world, ctx.caster, DOOM_WINDS)
                || auras::has_aura(world, ctx.caster, ASCENDANCE)
        }
        _ => false,
    };
    if invoke {
        invoke_thorim(world, ctx.caster, target, ctx.depth);
    }
    super::enhancement_spending::consume_for_strike(world, ctx);
    super::enhancement_unleashed::schedule_for_landed_root(world, ctx, target);
}

pub(super) fn cast_ride_the_lightning(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    depth: u8,
) {
    // SimC76f8118 sc_shaman.cpp5278/5647/5690/12532–12551: one linked CL per strike execution.
    let strike = matches!(spell.id, 17364 | 115356 | 60103);
    let selected =
        strike && enhancement(world, caster) && learned(world, caster, RIDE_THE_LIGHTNING);
    if !selected {
        return;
    }
    let Some(target) = spell.cast_primary_target else {
        return;
    };
    let mut payload = world
        .resource::<SpellStore>()
        .get(RIDE_DAMAGE)
        .expect("native Ride the Lightning payload")
        .clone();
    // SimC6587–6597: this background variant never benefits from or spends MW.
    payload.cast_charge_mods = Some(
        auras::spell_mods::snapshot_charged_modifiers(world, caster, &payload)
            .into_iter()
            .filter(|modifier| modifier.aura_spell_id != super::maelstrom_weapon::MODIFIER)
            .collect(),
    );
    spell_cast::launch_triggered_spell(world, caster, &payload, Some(target), depth + 1);
}

fn crash_stacks(world: &World, caster: Entity) -> u8 {
    world.get::<ActiveAuras>(caster).map_or(0, |auras| {
        auras
            .iter()
            .filter(|aura| {
                aura.caster == caster && matches!(aura.spell_id, CRASH_BUFF | CRASH_STACKS)
            })
            .map(|aura| aura.stacks)
            .max()
            .unwrap_or(0)
    })
}

fn count_targets(world: &mut World, caster: Entity, target: Entity, spell: &SpellInfo) -> usize {
    // These native payloads use deterministic area/cone selectors, not random targets.
    let effect = spell
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native lightning payload effect0");
    targets::select_targets(world, caster, Some(target), spell, effect, 0, true)
        .unwrap_or_else(|selector| {
            panic!("native lightning payload {} selector {selector}", spell.id)
        })
        .len()
}

fn cast_crash_payload(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    let stacks = crash_stacks(world, caster);
    if stacks == 0 {
        return;
    }
    let mut payload = world
        .resource::<SpellStore>()
        .get(CRASH_DAMAGE)
        .expect("native crash payload")
        .clone();
    let count = count_targets(world, caster, target, &payload);
    if count == 0 {
        return;
    }
    // SimC4545/4553–4558: split total damage among native recipients, scaled by stacks.
    payload.cast_damage_multiplier = Some(f32::from(stacks) / count as f32);
    spell_cast::launch_triggered_spell(world, caster, &payload, Some(target), depth + 1);
}

fn invoke_thorim(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    if !learned(world, caster, THORIM) || !auras::has_aura(world, caster, 344179) {
        return;
    }
    let selected = select_thorim_spell(world, caster);
    let mut payload = world
        .resource::<SpellStore>()
        .get(selected)
        .expect("native Thorim payload")
        .clone();
    payload.cast_proc_mods = super::snapshot_proc_modifiers(world, caster, &payload);
    payload.cast_charge_mods = Some(auras::spell_mods::snapshot_charged_modifiers(
        world, caster, &payload,
    ));
    if spell_cast::launch_triggered_spell(world, caster, &payload, Some(target), depth + 1) {
        // SimC2626–2629 makes this background variant an explicit MW consumer.
        // SimC9966: spend this proc before MW spending can earn the next one.
        consume_tempest(world, caster, selected);
        super::maelstrom_weapon::consume(world, caster, &payload);
    }
}

fn select_thorim_spell(world: &World, caster: Entity) -> u32 {
    // SimC12566–12579: a real Tempest proc supersedes the primer; initially LB.
    if auras::has_aura(world, caster, TEMPEST_PROC) {
        return TEMPEST;
    }
    world
        .get::<ThorimSelection>(caster)
        .map_or(BOLT, |selection| selection.0)
}

fn consume_tempest(world: &mut World, caster: Entity, spell: u32) {
    if spell != TEMPEST {
        return;
    }
    // SimC9966 decrements one earned charge, not the entire native two-stack aura.
    let instance = world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == TEMPEST_PROC && aura.caster == caster)
            .map(|aura| aura.instance_id)
    });
    if let Some(instance) = instance {
        auras::spend_stack(world, caster, instance);
    }
}

pub(super) fn after_lightning(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !enhancement(world, caster) {
        return;
    }
    consume_tempest(world, caster, spell.id);
    let selection = match spell.id {
        BOLT | CHAIN => spell.id,
        TEMPEST => {
            let Some(target) = spell.cast_primary_target else {
                return;
            };
            if count_targets(world, caster, target, spell) > 1 {
                CHAIN
            } else {
                BOLT
            }
        }
        _ => return,
    };
    world.entity_mut(caster).insert(ThorimSelection(selection));
}
