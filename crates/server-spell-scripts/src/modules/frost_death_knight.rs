//! Pinned Frost proc producers; native spells supply their effects and amounts.
use bevy::prelude::*;
use shared::components::{ActiveSpec, WeaponType};

use super::combat;
use combat::{
    auras, random, spell_cast,
    spell_info::{SpellInfo, SpellStore},
    stats::CharacterStats,
};

use combat::spell_cast::script_registry::SpellScriptHook;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::OwnsPolicyProc(owns_proc),
    SpellScriptHook::SnapshotsModifier(snapshots_modifier),
    SpellScriptHook::AfterWhiteHit(after_white_hit),
    SpellScriptHook::AfterElementalLaunch(after_launch),
    SpellScriptHook::AfterQueuedTriggers(consume_due_killing_machine),
    SpellScriptHook::AfterSpellPayment(roll_runic_empowerment),
];

const KILLING_MACHINE: u32 = 51128;
const RIME: u32 = 59057;
const OBLITERATION: u32 = 281238;
const PILLAR: u32 = 51271;
const RIME_BUFF: u32 = 59052;
const FROSTBOUND_WILL: u32 = 1238680;
const EMPOWER_RUNE_WEAPON: u32 = 47568;

#[derive(Component, Default)]
struct KillingMachineAttempts(u32);

pub(super) fn owns_proc(spell: u32) -> bool {
    matches!(spell, KILLING_MACHINE | RIME | OBLITERATION)
}

pub(super) fn snapshots_modifier(spell: u32) -> bool {
    spell == RIME_BUFF
}

fn is_frost(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 251)
}

fn trigger_native_buff(world: &mut World, caster: Entity, source: u32) {
    let buff = world
        .resource::<SpellStore>()
        .get(source)
        .and_then(|spell| spell.effects.iter().find(|effect| effect.index == 0))
        .expect("active Frost passive retains native effect0")
        .trigger_spell;
    if !spell_cast::try_cast_triggered(world, caster, buff, Some(caster), 1) {
        warn!(?caster, source, buff, "Native Frost proc buff refused");
    }
}

pub(super) fn after_white_hit(world: &mut World, caster: Entity, critical: bool) {
    if !critical || !is_frost(world, caster) || !auras::has_aura(world, caster, KILLING_MACHINE) {
        return;
    }
    let Some(stats) = world.get::<CharacterStats>(caster) else {
        return;
    };
    // SimC76f8118 sc_death_knight.cpp7716–7719/14824–14825: critical
    // white swings use accumulated PRD,0.3 DW or1.0 with Might and a2H.
    let two_hand = stats.main_hand.kind == WeaponType::TwoHand;
    let probability = if two_hand && auras::has_aura(world, caster, 81333) {
        1.0
    } else {
        0.3
    };
    let attempts = world
        .get::<KillingMachineAttempts>(caster)
        .map_or(1, |state| state.0 + 1);
    world
        .entity_mut(caster)
        .insert(KillingMachineAttempts(attempts));
    let chance = random::accumulated_proc_constant(probability) * f64::from(attempts);
    if random::random::<f64>() >= chance {
        return;
    }
    world.get_mut::<KillingMachineAttempts>(caster).unwrap().0 = 0;
    trigger_native_buff(world, caster, KILLING_MACHINE);
}

pub(super) fn after_launch(world: &mut World, caster: Entity, spell: &SpellInfo, foreground: bool) {
    if !foreground || !is_frost(world, caster) {
        return;
    }
    if matches!(spell.id, 49143 | 194913 | 49184) {
        trigger_obliteration(world, caster);
    }
    // SimC76f8118 sc_death_knight.cpp10889–10900/11177–11189:
    // after Obliteration, FS/GA roll Rime's native effect2 (not effect1).
    if matches!(spell.id, 49143 | 194913) && auras::has_aura(world, caster, RIME) {
        let chance = spell_cast::scripts::script_effect_value(world, caster, RIME, 1);
        if random::random::<f32>() * 100.0 < chance {
            trigger_native_buff(world, caster, RIME);
        }
    }
    if spell.id == 49184 {
        consume_rime(world, caster, spell);
    }
    schedule_killing_machine(world, caster, spell);
}

fn consume_rime(world: &mut World, caster: Entity, spell: &SpellInfo) {
    let used_rime = spell
        .cast_charge_mods
        .iter()
        .flatten()
        .any(|modifier| modifier.aura_spell_id == RIME_BUFF);
    if !used_rime {
        return;
    }
    // SimC76f8118 sc_death_knight.cpp11637–11665: an earned Rime HB
    // adjusts ERW by native Frostbound Will effect1, then decrements Rime.
    if auras::has_aura(world, caster, FROSTBOUND_WILL) {
        let milliseconds =
            spell_cast::scripts::script_effect_value(world, caster, FROSTBOUND_WILL, 0);
        spell_cast::scripts::reduce_cooldown(
            world,
            caster,
            EMPOWER_RUNE_WEAPON,
            f64::from(-milliseconds) / 1000.0,
        );
    }
    let instance = world.get::<auras::ActiveAuras>(caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == RIME_BUFF && aura.caster == caster)
            .map(|aura| aura.instance_id)
    });
    if let Some(instance) = instance {
        auras::spend_stack(world, caster, instance);
    }
}

fn trigger_obliteration(world: &mut World, caster: Entity) {
    // sc_death_knight.cpp10889–10897/11177–11186/11614–11622:
    // actual FS/GA/HB during Pillar grants KM and rolls the native rune gain.
    if !auras::has_aura(world, caster, OBLITERATION) || !auras::has_aura(world, caster, PILLAR) {
        return;
    }
    if auras::has_aura(world, caster, KILLING_MACHINE) {
        trigger_native_buff(world, caster, KILLING_MACHINE);
        // sc_death_knight.cpp13356–13364: guaranteed producers predict KM now.
        #[cfg(any(test, feature = "test-support"))]
        combat::simc_apl_native::reaction::predict(world, caster, 51124);
    }
    let chance = spell_cast::scripts::script_effect_value(world, caster, OBLITERATION, 1);
    if random::random::<f32>() * 100.0 >= chance {
        return;
    }
    if !spell_cast::try_cast_triggered(world, caster, 281327, Some(caster), 1) {
        warn!(?caster, "Native Obliteration rune grant refused");
    }
}

fn roll_runic_empowerment(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    costs: &[(spell_cast::CostPool, f32)],
) {
    use shared::components::PowerType;
    if !is_frost(world, caster) {
        return;
    }
    let actual = costs.iter().find_map(|&(pool, amount)| {
        (pool == spell_cast::CostPool::Power(PowerType::RunicPower)).then_some(amount)
    });
    let Some(actual) = actual else { return };
    let base = spell
        .power_costs
        .iter()
        .filter(|cost| cost.power_type == PowerType::RunicPower as i32)
        .map(|cost| cost.mana_cost as f32)
        .fold(0.0, f32::max);
    // SimC76f8118 sc_death_knight.cpp13128–13146/13424–13439:
    // max(base,actual) RP cost, native chance percent times0.1 per displayed RP.
    // DB2 stores RP in tenths; divide by10 before applying that chance.
    let raw_cost = actual.max(base);
    if raw_cost <= 0.0 {
        return;
    }
    let displayed_cost = raw_cost / 10.0;
    let chance = spell_cast::scripts::script_effect_value(world, caster, 81229, 0) / 100.0
        * 0.1
        * displayed_cost;
    if random::random::<f32>() < chance {
        cast_buff_stacks(world, caster, 193486, 1);
    }
}

#[derive(Component, Default)]
struct PendingConsumption(Vec<(f64, Entity)>);

fn schedule_killing_machine(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !matches!(spell.id, 49020 | 207230) || !auras::has_aura(world, caster, 51124) {
        return;
    }
    let Some(target) = spell.cast_primary_target else {
        return;
    };
    // SimC76f8118 sc_death_knight.cpp10585–10588/11908–11915/11970–11971:
    // FSC waits50ms; Obliterate waits the sum of its native weapon delays.
    let delay_ms = if spell.id == 207230 {
        50
    } else {
        let dual = world
            .get::<CharacterStats>(caster)
            .is_some_and(|stats| stats.off_hand.is_some());
        spell
            .effects
            .iter()
            .filter(|effect| {
                if dual {
                    matches!(effect.index, 1 | 2)
                } else {
                    effect.index == 3
                }
            })
            .map(|effect| effect.misc_values[0])
            .sum::<i32>()
    };
    let due = auras::aura_now(world) + f64::from(delay_ms) / 1000.0;
    if let Some(mut pending) = world.get_mut::<PendingConsumption>(caster) {
        pending.0.push((due, target));
    } else {
        world
            .entity_mut(caster)
            .insert(PendingConsumption(vec![(due, target)]));
    }
}

fn consume_due_killing_machine(world: &mut World) {
    let now = auras::aura_now(world);
    let mut due = Vec::new();
    for (caster, mut pending) in world
        .query::<(Entity, &mut PendingConsumption)>()
        .iter_mut(world)
    {
        pending.0.retain(|&(at, target)| {
            if at > now {
                return true;
            }
            due.push((caster, target));
            false
        });
    }
    for (caster, target) in due {
        consume_killing_machine(world, caster, target);
    }
}

fn cast_buff_stacks(world: &mut World, caster: Entity, spell: u32, stacks: u8) {
    if !spell_cast::cast_triggered_with_overrides(
        world,
        caster,
        spell,
        Some(caster),
        1,
        spell_cast::TriggeredSpellOverrides {
            // SimC76f8118 sc_death_knight.cpp16207–16209: KS divides effect1 by10.
            effect0_amount: (spell == 1230916)
                .then(|| spell_cast::scripts::script_effect_value(world, caster, spell, 0) / 10.0),
            aura_stacks: Some(stacks),
            ..Default::default()
        },
    ) {
        warn!(?caster, spell, "Native Frost callback refused");
    }
}

fn consume_killing_machine(world: &mut World, caster: Entity, target: Entity) {
    let aura = world.get::<auras::ActiveAuras>(caster).and_then(|active| {
        active
            .iter()
            .find(|aura| aura.spell_id == 51124 && aura.caster == caster)
            .cloned()
    });
    let Some(aura) = aura else { return };
    // SimC76f8118 sc_death_knight.cpp13372–13412: consume current earned
    // stacks after impacts, then trigger each selected callback per consumption.
    let streak = auras::has_aura(world, caster, 1230153);
    let spent = if streak { aura.stacks } else { 1 };
    for _ in 0..spent {
        auras::spend_stack(world, caster, aura.instance_id);
    }
    if streak {
        cast_buff_stacks(world, caster, 1230916, spent);
    }
    extend_breath(world, caster, spent);
    trigger_bonegrinder(world, caster, spent);
    schedule_arctic_assault(world, caster, target, spent);
    if auras::has_aura(world, caster, 207061) {
        let chance = spell_cast::scripts::script_effect_value(world, caster, 207061, 0);
        for _ in 0..spent {
            if random::random::<f32>() * 100.0 < chance {
                cast_buff_stacks(world, caster, 207062, 1);
            }
        }
    }
}

fn extend_breath(world: &mut World, caster: Entity, spent: u8) {
    if !auras::has_aura(world, caster, 1249658) {
        return;
    }
    // SimC76f8118 sc_death_knight.cpp13385–13389: native effect3 is milliseconds.
    let milliseconds = spell_cast::scripts::script_effect_value(world, caster, 1249658, 2);
    auras::extend_owned_duration(
        world,
        caster,
        caster,
        &[1249658],
        (milliseconds * f32::from(spent)) as u32,
    );
}

fn trigger_bonegrinder(world: &mut World, caster: Entity, spent: u8) {
    if !auras::has_aura(world, caster, 377098) || auras::has_aura(world, caster, 377103) {
        return;
    }
    // SimC76f8118 sc_death_knight.cpp13460–13485: native cap minus one
    // converts the crit counter into the selected talent's frost-damage buff.
    cast_buff_stacks(world, caster, 377101, spent);
    let counter = world
        .get::<auras::ActiveAuras>(caster)
        .and_then(|active| active.iter().find(|aura| aura.spell_id == 377101).cloned());
    if let Some(counter) = counter.filter(|aura| aura.stacks >= aura.max_stacks.saturating_sub(1)) {
        let amount = spell_cast::scripts::script_effect_value(world, caster, 377098, 0);
        if !spell_cast::cast_triggered_with_overrides(
            world,
            caster,
            377103,
            Some(caster),
            1,
            spell_cast::TriggeredSpellOverrides {
                effect0_amount: Some(amount),
                ..Default::default()
            },
        ) {
            warn!(?caster, "Native Bonegrinder frost buff refused");
        }
        auras::remove_instance(
            world,
            caster,
            counter.instance_id,
            auras::RemoveReason::Consumed,
        );
    }
}

fn schedule_arctic_assault(world: &mut World, caster: Entity, target: Entity, spent: u8) {
    if !auras::has_aura(world, caster, 456230) {
        return;
    }
    // SimC76f8118 sc_death_knight.cpp11059–11076/13395–13404:
    // native GA child, native talent multiplier, consumed stack count,500ms delay.
    let multiplier = spell_cast::scripts::script_effect_value(world, caster, 456230, 0) / 100.0
        * f32::from(spent);
    let spell = world
        .resource::<SpellStore>()
        .get(195975)
        .expect("native Arctic Assault GA payload")
        .clone();
    let ctx = spell_cast::effects::EffectCtx {
        caster,
        spell: &spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    spell_cast::queued_triggers::schedule_scaled_full_trigger(
        world,
        &ctx,
        spell_cast::queued_triggers::ScaledTrigger {
            target,
            spell_id: 195975,
            delay_ms: 500,
            multiplier,
        },
    );
}
