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
    SpellScriptHook::DeclaredReach(declared_reach),
    SpellScriptHook::OwnsPolicyProc(owns_proc),
    SpellScriptHook::SnapshotsModifier(snapshots_modifier),
    SpellScriptHook::BeforeDamageSnapshot(convert_km_weapon_school),
    SpellScriptHook::BeforeDamageSnapshot(convert_rime_blast_school),
    SpellScriptHook::TargetContextDamageMultiplier(converted_weapon_multiplier),
    SpellScriptHook::TargetContextDamageMultiplier(breath_secondary_multiplier),
    SpellScriptHook::AfterWhiteHit(after_white_hit),
    SpellScriptHook::AfterElementalLaunch(after_launch),
    SpellScriptHook::AfterElementalLaunch(update_empowered_spender),
    SpellScriptHook::AfterQueuedTriggers(consume_due_killing_machine),
    SpellScriptHook::AfterSpellPayment(roll_runic_empowerment),
    SpellScriptHook::AfterPeriodicDamagePolicies(frost_fever_runic_power),
    SpellScriptHook::AfterPeriodicDamagePolicies(frost_fever_brittle),
    SpellScriptHook::AfterResolvedHits(earn_freezing_tempest),
    SpellScriptHook::AfterResolvedHits(winter_target_bonuses),
    SpellScriptHook::OnCast(reset_freezing_tempest),
    SpellScriptHook::ResolvedDamage(glacial_advance_razorice),
    SpellScriptHook::LateAuraRegistration(register_frost_auras),
    SpellScriptHook::ResolvedDamage(frostreaper_impact),
    SpellScriptHook::ResolvedDamage(hyperpyrexia_impact),
    SpellScriptHook::ResolvedDamage(enduring_strength_impact),
];

const KILLING_MACHINE: u32 = 51128;
const RIME: u32 = 59057;
const OBLITERATION: u32 = 281238;
const PILLAR: u32 = 51271;
const RIME_BUFF: u32 = 59052;
const FROSTBOUND_WILL: u32 = 1238680;
const EMPOWER_RUNE_WEAPON: u32 = 47568;

fn declared_reach(
    _world: &World,
    _caster: Entity,
    spell: &SpellInfo,
) -> combat::spell_cast::script_registry::ScriptReach {
    if !matches!(spell.id, 279302 | 1265384) {
        return Default::default();
    }
    let radius = spell
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Fury radius")
        .base_points;
    combat::spell_cast::script_registry::ScriptReach::caster(radius)
}

#[derive(Component, Default)]
struct KillingMachineAttempts(u32);

pub(super) fn owns_proc(spell: u32) -> bool {
    matches!(
        spell,
        KILLING_MACHINE | RIME | OBLITERATION | 195621 | 1230301 | 1296652 | 377190
    )
}

pub(super) fn snapshots_modifier(spell: u32) -> bool {
    spell == RIME_BUFF
}

fn is_frost(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 251)
}

fn convert_rime_blast_school(world: &World, caster: Entity, spell: &mut SpellInfo) {
    if spell.id != 49184 || !is_frost(world, caster) {
        return;
    }
    let school = world
        .get::<auras::ActiveAuras>(caster)
        .and_then(|active| {
            active
                .iter()
                .find(|aura| aura.spell_id == 443532 && aura.caster == caster)
        })
        .and_then(|aura| aura.effects.iter().find(|effect| effect.index == 0))
        .map(|effect| effect.misc_values[0] as u32);
    if let Some(school) = school {
        spell
            .misc
            .as_mut()
            .expect("Howling Blast school")
            .school_mask = school;
    }
}

fn convert_km_weapon_school(world: &World, caster: Entity, spell: &mut SpellInfo) {
    if !is_frost(world, caster) || !matches!(spell.id, 222024 | 66198 | 325461) {
        return;
    }
    let school = world
        .get::<auras::ActiveAuras>(caster)
        .and_then(|active| {
            active
                .iter()
                .find(|aura| aura.spell_id == 51124 && aura.caster == caster)
        })
        .and_then(|aura| aura.effects.iter().find(|effect| effect.index == 1))
        .map(|effect| effect.misc_values[0] as u32);
    if let Some(school) = school {
        spell
            .misc
            .as_mut()
            .expect("native Obliterate child retains school")
            .school_mask = school;
    }
}

fn breath_secondary_multiplier(
    world: &World,
    ctx: &spell_cast::effects::EffectCtx,
    target: Entity,
    _: bool,
) -> f32 {
    if ctx.spell.id != 155166 || !is_frost(world, ctx.caster) {
        return 1.0;
    }
    let targets = spell_cast::resolving_targets(world, ctx).expect("Breath targets");
    let index = targets
        .iter()
        .position(|candidate| *candidate == target)
        .expect("Breath target ordinal");
    // SimC76f8118 sc_death_knight.cpp9204–9207/9244–9251 and action.cpp1564–1568.
    if index == 0 {
        1.0
    } else {
        0.8 * (1.0 / targets.len().min(20) as f32).sqrt()
    }
}

fn converted_weapon_multiplier(
    world: &World,
    ctx: &spell_cast::effects::EffectCtx,
    target: Entity,
    _: bool,
) -> f32 {
    if !is_frost(world, ctx.caster)
        || !matches!(ctx.spell.id, 222024 | 66198 | 325461)
        || ctx.spell.school_mask() != 16
        || !world
            .get::<combat::class_progression::KnownSpells>(ctx.caster)
            .is_some_and(|known| known.contains(317214))
    {
        return 1.0;
    }
    // SimC76f8118 sc_death_knight.cpp11807–11828: the converted physical
    // child manually gains Frozen Heart and its caster's Razorice stacks.
    let mastery = world
        .get::<CharacterStats>(ctx.caster)
        .expect("Frost caster stats")
        .mastery_pct
        / 100.0;
    let razorice = world
        .get::<auras::ActiveAuras>(target)
        .and_then(|active| {
            active
                .iter()
                .find(|aura| aura.spell_id == 51714 && aura.caster == ctx.caster)
        })
        .and_then(|aura| {
            aura.effects
                .iter()
                .find(|effect| effect.index == 0)
                .map(|effect| effect.amount * f32::from(aura.stacks) / 100.0)
        })
        .unwrap_or(0.0);
    (1.0 + mastery) * (1.0 + razorice)
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

fn hyperpyrexia_impact(
    world: &mut World,
    ctx: &combat::spell_cast::effects::EffectCtx,
    target: Entity,
    _: bool,
    damage: f32,
) {
    if damage <= 0.0
        || !matches!(ctx.spell.id, 222026 | 66196 | 195975 | 155166)
        || !is_frost(world, ctx.caster)
        || !auras::has_aura(world, ctx.caster, 456238)
    {
        return;
    }
    let chance = world
        .resource::<SpellStore>()
        .get(456238)
        .and_then(|spell| spell.aura_options.as_ref())
        .expect("Hyperpyrexia proc chance")
        .proc_chance as f32;
    if random::random::<f32>() * 100.0 < chance {
        let percent = spell_cast::scripts::script_effect_value(world, ctx.caster, 456238, 0);
        super::marksmanship::bank_residual(world, ctx, target, 458169, percent, damage);
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
    extend_winter_for_runes(world, caster, spell);
    trigger_rune_mastery(world, caster, spell);
    if spell.id == EMPOWER_RUNE_WEAPON && auras::has_aura(world, caster, KILLING_MACHINE) {
        // SimC76f8118 sc_death_knight.cpp10236–10243: a real ready ERW grants KM.
        trigger_native_buff(world, caster, KILLING_MACHINE);
        #[cfg(any(test, feature = "test-support"))]
        combat::simc_apl_native::reaction::predict(world, caster, 51124);
    }
    if spell.id == 49143 {
        consume_frostreaper(world, caster, spell.cast_primary_target);
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
    // SimC76f8118 sc_death_knight.cpp12050–12060/15341–15342:
    // selected Frozen Dominion starts its native Winter aura from real Pillar.
    if spell.id == PILLAR && auras::has_aura(world, caster, 377226) {
        cast_buff_stacks(world, caster, 1233152, 1);
    }
    if spell.id == 279302 && auras::has_aura(world, caster, 1265637) {
        let delay = world
            .resource::<SpellStore>()
            .get(1265640)
            .and_then(|spell| spell.misc.as_ref())
            .and_then(|misc| misc.duration)
            .expect("Chosen recall delay")
            .duration_ms as u32;
        let ctx = combat::spell_cast::effects::EffectCtx {
            caster,
            spell,
            depth: 0,
            dest: None,
            chain_multiplier: 1.0,
        };
        combat::spell_cast::queued_triggers::schedule_callback(
            world,
            &ctx,
            caster,
            delay,
            enable_chosen_recall,
        );
    }
    if matches!(spell.id, 279302 | 1265384) {
        launch_fury_damage(world, caster, spell);
    }
    if spell.id == 1265384 {
        auras::remove_auras(world, caster, auras::RemoveReason::Consumed, |aura| {
            aura.spell_id == 1265639 && aura.caster == caster
        });
    }
    if matches!(spell.id, 279302 | 1265384) && auras::has_aura(world, caster, 1265632) {
        let duration = world
            .resource::<SpellStore>()
            .get(1265630)
            .and_then(|spell| spell.misc.as_ref())
            .and_then(|misc| misc.duration)
            .expect("Chosen haste duration")
            .duration_ms as u32;
        let scale = if spell.id == 1265384 {
            spell_cast::scripts::script_effect_value(world, caster, 1265637, 1) / 100.0
        } else {
            1.0
        };
        let duration = (duration as f32 * scale) as u32;
        if auras::has_aura(world, caster, 1265630) {
            auras::extend_owned_duration(world, caster, caster, &[1265630], duration);
        } else {
            spell_cast::cast_triggered_with_overrides(
                world,
                caster,
                1265630,
                Some(caster),
                1,
                spell_cast::TriggeredSpellOverrides {
                    duration_ms: Some(duration as i32),
                    ..Default::default()
                },
            );
        }
    }
    if matches!(spell.id, 279302 | 1265384) && auras::has_aura(world, caster, 1265633) {
        // SimC76f8118 sc_death_knight.cpp10659–10660/10694–10695:
        // the actual Fury extends an already active Pillar by native effect1.
        let milliseconds = spell_cast::scripts::script_effect_value(world, caster, 1265633, 0);
        let scale = if spell.id == 1265384 {
            spell_cast::scripts::script_effect_value(world, caster, 1265637, 1) / 100.0
        } else {
            1.0
        };
        auras::extend_owned_duration(
            world,
            caster,
            caster,
            &[PILLAR],
            (milliseconds * scale) as u32,
        );
    }
}

fn enable_chosen_recall(world: &mut World, caster: Entity, _: Entity, depth: u8) {
    spell_cast::cast_triggered(world, caster, 1265639, Some(caster), depth);
}

fn launch_fury_damage(world: &mut World, caster: Entity, source: &SpellInfo) {
    let Some(primary) = source.cast_primary_target else {
        return;
    };
    let Some(mut damage) = spell_cast::load_triggered_spell(world, 279303, 1) else {
        return;
    };
    let radius = source
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("Fury radius")
        .base_points;
    let center =
        combat::spell_cast::targets::position(world, caster).expect("Fury caster position");
    let targets = combat::spell_cast::targets::area_units(
        world,
        caster,
        center,
        radius,
        combat::spell_cast::targets::Relation::Enemy,
        0,
    );
    let recall = if source.id == 1265384 {
        spell_cast::scripts::script_effect_value(world, caster, 1265637, 1) / 100.0
    } else {
        1.0
    };
    for target in targets {
        let primary_bonus = if target == primary && auras::has_aura(world, caster, 1265632) {
            1.0 + spell_cast::scripts::script_effect_value(world, caster, 1265632, 0) / 100.0
        } else {
            1.0
        };
        damage.cast_damage_multiplier = Some(recall * primary_bonus);
        spell_cast::launch_triggered_spell(world, caster, &damage, Some(target), 1);
    }
}

fn update_empowered_spender(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    foreground: bool,
) {
    const ERW_BUFF: u32 = 1230959;
    const OBLITERATION_EFFECT: u32 = 207256;
    if !is_frost(world, caster) {
        return;
    }
    if spell.id == ERW_BUFF
        && auras::has_aura(world, caster, PILLAR)
        && auras::has_aura(world, caster, OBLITERATION)
    {
        let instance = world
            .get::<auras::ActiveAuras>(caster)
            .and_then(|active| {
                active
                    .iter()
                    .find(|aura| aura.spell_id == ERW_BUFF && aura.caster == caster)
                    .map(|aura| aura.instance_id)
            })
            .expect("actual Empower Rune Weapon applies its native aura");
        // SimC76f8118 sc_death_knight.cpp10236–10243/16965: real ERW in
        // Pillar uses native Obliteration trigger effects on its spender aura.
        for (destination, source) in [(1, 0), (2, 1), (3, 2)] {
            let amount = spell_cast::scripts::script_effect_value(
                world,
                caster,
                OBLITERATION_EFFECT,
                source,
            );
            assert!(auras::set_effect_amount(
                world,
                caster,
                instance,
                destination,
                amount
            ));
        }
    } else if foreground && matches!(spell.id, 207230 | 49020) {
        let instance = world.get::<auras::ActiveAuras>(caster).and_then(|active| {
            active
                .iter()
                .find(|aura| {
                    aura.spell_id == ERW_BUFF
                        && aura.caster == caster
                        && aura
                            .effects
                            .iter()
                            .any(|effect| effect.index == 1 && effect.amount < 0.0)
                })
                .map(|aura| aura.instance_id)
        });
        if let Some(instance) = instance {
            // sc_death_knight.cpp10581–10594/11906–11913: one actual spender consumes it.
            auras::remove_instance(world, caster, instance, auras::RemoveReason::Consumed);
        }
    }
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
    // SimC76f8118 sc_death_knight.cpp11630–11665: earned Rime HB grants
    // Rage's native RP, extends active Breath, adjusts ERW, then spends Rime.
    if auras::has_aura(world, caster, 377076)
        && !spell_cast::try_cast_triggered(world, caster, 341725, Some(caster), 1)
    {
        warn!(
            ?caster,
            "Native Rage of the Frozen Champion energize refused"
        );
    }
    extend_breath(world, caster, 1);
    trigger_dark_talons(world, caster);
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

fn frostreaper_impact(
    world: &mut World,
    ctx: &spell_cast::effects::EffectCtx,
    target: Entity,
    _: bool,
    damage: f32,
) {
    if damage <= 0.0
        || !matches!(
            ctx.spell.id,
            222024 | 66198 | 325461 | 1264082 | 1264083 | 1264084
        )
        || !is_frost(world, ctx.caster)
    {
        return;
    }
    let instance = world
        .get::<auras::ActiveAuras>(ctx.caster)
        .and_then(|active| {
            active
                .iter()
                .find(|aura| aura.spell_id == 1230301 && aura.caster == ctx.caster)
                .map(|aura| aura.instance_id)
        });
    let Some(instance) = instance else { return };
    // SimC76f8118 sc_death_knight.cpp11854–11857: each real Obliterate
    // strike attempts the native Frostreaper RPPM on its actual target.
    if !auras::procs::try_native_policy_proc(world, ctx.caster, instance) {
        return;
    }
    if owned_frostreaper(world, ctx.caster, target) {
        spell_cast::cast_triggered(world, ctx.caster, 1233619, Some(target), ctx.depth + 1);
    }
    auras::apply_aura(
        world,
        &auras::AuraApplication {
            caster: ctx.caster,
            target,
            spell_id: 1233351,
            effect_mask: 1,
            source: auras::AuraSource::Cast,
        },
    );
}

fn owned_frostreaper(world: &World, caster: Entity, target: Entity) -> bool {
    world
        .get::<auras::ActiveAuras>(target)
        .is_some_and(|active| {
            active
                .iter()
                .any(|aura| aura.spell_id == 1233351 && aura.caster == caster)
        })
}

fn consume_frostreaper(world: &mut World, caster: Entity, target: Option<Entity>) {
    let Some(target) = target.filter(|&target| owned_frostreaper(world, caster, target)) else {
        return;
    };
    // sc_death_knight.cpp10875–10883: Frost Strike detonates an earned mark.
    spell_cast::cast_triggered(world, caster, 1233619, Some(target), 1);
    auras::remove_auras(world, target, auras::RemoveReason::Consumed, |aura| {
        aura.spell_id == 1233351 && aura.caster == caster
    });
}

fn trigger_rune_mastery(world: &mut World, caster: Entity, spell: &SpellInfo) {
    let rune_action = spell.power_costs.iter().any(|cost| {
        cost.power_type == 5
            && cost.mana_cost > 0
            && (cost.required_aura_spell_id == 0
                || auras::has_aura(world, caster, cost.required_aura_spell_id))
    });
    // SimC76f8118 sc_death_knight.cpp13081–13098/15831–15833: each rune-cost action,
    // including a free one, rolls the source's explicitly observed 15% chance once.
    if rune_action && auras::has_aura(world, caster, 374574) && random::random::<f32>() < 0.15 {
        spell_cast::cast_triggered(world, caster, 374585, Some(caster), 1);
    }
}

fn extend_winter_for_runes(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if spell.id == 196770 || !auras::has_aura(world, caster, 194912) {
        return;
    }
    let winter = [196770, 1233152];
    if !winter.iter().any(|&id| auras::has_aura(world, caster, id)) {
        return;
    }
    let base_cost: i32 = spell
        .power_costs
        .iter()
        .filter(|cost| {
            cost.power_type == 5
                && cost.mana_cost > 0
                && (cost.required_aura_spell_id == 0
                    || auras::has_aura(world, caster, cost.required_aura_spell_id))
        })
        .map(|cost| cost.mana_cost)
        .sum();
    if base_cost == 0 {
        return;
    }
    // SimC76f8118 sc_death_knight.cpp13086–13095: use base rune cost,
    // including free casts, for both stacks and the Winter extension.
    let tenths = spell_cast::scripts::script_effect_value(world, caster, 194912, 0);
    auras::extend_owned_duration(
        world,
        caster,
        caster,
        &winter,
        (tenths * 100.0 * base_cost as f32) as u32,
    );
    cast_buff_stacks(world, caster, 211805, base_cost as u8);
}

fn register_frost_auras(scripts: &mut auras::scripts::AuraScripts) {
    scripts.0.entry(440031).or_default().check_proc = Some(bind_native_proc_allowed);
    scripts.0.entry(RIME_BUFF).or_default().on_apply = Some(rime_applied);
    scripts.0.entry(RIME_BUFF).or_default().on_remove = Some(rime_ended);
    scripts.0.entry(PILLAR).or_default().on_apply = Some(pillar_applied);
    scripts.0.entry(PILLAR).or_default().on_remove = Some(pillar_ended);
    scripts.0.entry(443595).or_default().on_remove = Some(dark_talons_ended);
    scripts.0.entry(194879).or_default().on_apply = Some(icy_talons_started);
    scripts.0.entry(194879).or_default().on_remove = Some(icy_talons_ended);
    scripts.0.entry(1249658).or_default().on_remove = Some(breath_ended);
    for winter in [196770, 1233152] {
        scripts.0.entry(winter).or_default().on_remove = Some(winter_ended);
    }
}

#[derive(Component)]
struct EnduringStrengthCooldown(f64);

fn enduring_strength_impact(
    world: &mut World,
    ctx: &spell_cast::effects::EffectCtx,
    _: Entity,
    critical: bool,
    damage: f32,
) {
    if !critical
        || damage <= 0.0
        || !matches!(
            ctx.spell.id,
            207230 | 222024 | 66198 | 325461 | 1264082 | 1264083 | 1264084
        )
        || !is_frost(world, ctx.caster)
        || !auras::has_aura(world, ctx.caster, 377190)
        || !auras::has_aura(world, ctx.caster, PILLAR)
    {
        return;
    }
    let now = spell_cast::now(world);
    if world
        .get::<EnduringStrengthCooldown>(ctx.caster)
        .is_some_and(|cooldown| now < cooldown.0)
    {
        return;
    }
    let milliseconds = world
        .resource::<SpellStore>()
        .get(377192)
        .and_then(|spell| spell.aura_options.as_ref())
        .expect("Enduring Strength internal cooldown")
        .proc_category_recovery_ms;
    world
        .entity_mut(ctx.caster)
        .insert(EnduringStrengthCooldown(
            now + f64::from(milliseconds) / 1000.0,
        ));
    cast_buff_stacks(world, ctx.caster, 377192, 1);
}

fn pillar_ended(
    world: &mut World,
    unit: Entity,
    aura: &auras::AuraInstance,
    reason: auras::RemoveReason,
) {
    if aura.caster != unit
        || !is_frost(world, unit)
        || !auras::has_aura(world, unit, 377190)
        || !matches!(
            reason,
            auras::RemoveReason::Expired
                | auras::RemoveReason::Consumed
                | auras::RemoveReason::Cancelled
        )
    {
        return;
    }
    let stacks = world
        .get::<auras::ActiveAuras>(unit)
        .and_then(|active| {
            active
                .iter()
                .find(|a| a.spell_id == 377192 && a.caster == unit)
        })
        .map_or(0, |a| a.stacks);
    spell_cast::cast_triggered(world, unit, 377195, Some(unit), 1);
    let per_stack = spell_cast::scripts::script_effect_value(world, unit, 377190, 1);
    let max = spell_cast::scripts::script_effect_value(world, unit, 377190, 3);
    auras::extend_owned_duration(
        world,
        unit,
        unit,
        &[377195],
        (per_stack * f32::from(stacks)).min(max) as u32,
    );
    auras::remove_auras(world, unit, auras::RemoveReason::Consumed, |a| {
        a.spell_id == 377192 && a.caster == unit
    });
}

fn bind_native_proc_allowed(
    world: &World,
    aura: &auras::AuraInstance,
    _: &auras::procs::AuraEvent,
) -> bool {
    !is_frost(world, aura.caster)
}

fn rime_applied(world: &mut World, unit: Entity, aura: &auras::AuraInstance) {
    if aura.caster == unit && is_frost(world, unit) && auras::has_aura(world, unit, 440031) {
        spell_cast::cast_triggered(world, unit, 443532, Some(unit), 1);
    }
}

fn rime_ended(world: &mut World, unit: Entity, aura: &auras::AuraInstance, _: auras::RemoveReason) {
    if aura.caster == unit && is_frost(world, unit) {
        auras::remove_auras(world, unit, auras::RemoveReason::Consumed, |buff| {
            buff.spell_id == 443532 && buff.caster == unit
        });
    }
}

fn pillar_applied(world: &mut World, unit: Entity, aura: &auras::AuraInstance) {
    if aura.caster != unit || !is_frost(world, unit) {
        return;
    }
    // SimC76f8118 sc_death_knight.cpp7151–7158/16942–16970 uses Pillar
    // for strength but does not parse its direct damage modifier for Frost.
    assert!(auras::set_effect_amount(
        world,
        unit,
        aura.instance_id,
        4,
        0.0
    ));
}

fn winter_ended(
    world: &mut World,
    unit: Entity,
    aura: &auras::AuraInstance,
    _: auras::RemoveReason,
) {
    if aura.caster == unit {
        auras::remove_auras(world, unit, auras::RemoveReason::Cancelled, |buff| {
            buff.spell_id == 211805 && buff.caster == unit
        });
    }
}

fn breath_ended(
    world: &mut World,
    unit: Entity,
    aura: &auras::AuraInstance,
    reason: auras::RemoveReason,
) {
    if aura.caster != unit
        || !is_frost(world, unit)
        || matches!(
            reason,
            auras::RemoveReason::Death
                | auras::RemoveReason::CasterGone
                | auras::RemoveReason::Evade
                | auras::RemoveReason::Unlearned
        )
    {
        return;
    }
    // SimC76f8118 sc_death_knight.cpp7135–7140: ending Breath grants
    // the native 1234304 rune amount, except during encounter teardown.
    spell_cast::cast_triggered(world, unit, 1234304, Some(unit), 1);
}

fn earn_freezing_tempest(world: &mut World, ctx: &spell_cast::effects::EffectCtx) {
    if ctx.spell.id != 196771
        || !is_frost(world, ctx.caster)
        || !auras::has_aura(world, ctx.caster, 1296652)
    {
        return;
    }
    let hit = spell_cast::resolving_damage(world, ctx)
        .expect("Winter pulse retains its resolved hits")
        .iter()
        .any(|(_, damage)| *damage > 0.0);
    // SimC76f8118 sc_death_knight.cpp12381–12387: one stack per pulse with any hit.
    if hit && !spell_cast::try_cast_triggered(world, ctx.caster, 1297365, Some(ctx.caster), 1) {
        warn!(caster = ?ctx.caster, "Native Freezing Tempest buff refused");
    }
}

#[derive(Component, Default)]
struct WinterTargets {
    biting_cold: bool,
    dominion_started: bool,
    dominion_targets: std::collections::HashSet<Entity>,
}

fn winter_target_bonuses(world: &mut World, ctx: &spell_cast::effects::EffectCtx) {
    if ctx.spell.id != 196771 || !is_frost(world, ctx.caster) {
        return;
    }
    let hits: Vec<Entity> = spell_cast::resolving_damage(world, ctx)
        .expect("Winter pulse hits")
        .iter()
        .filter(|(_, damage)| *damage > 0.0)
        .map(|(target, _)| *target)
        .collect();
    if hits.is_empty() {
        return;
    }
    let biting = auras::has_aura(world, ctx.caster, 377056);
    let threshold = spell_cast::scripts::script_effect_value(world, ctx.caster, 377056, 0) as usize;
    let dominion = auras::has_aura(world, ctx.caster, 377226);
    let mut dominion_active = auras::has_aura(world, ctx.caster, 377253);
    let mut state = world
        .entity_mut(ctx.caster)
        .take::<WinterTargets>()
        .unwrap_or_default();
    if biting && hits.len() >= threshold && !state.biting_cold {
        state.biting_cold = true;
        trigger_native_buff(world, ctx.caster, RIME);
    }
    if dominion {
        for target in hits {
            if (!state.dominion_started || dominion_active) && state.dominion_targets.insert(target)
            {
                state.dominion_started = true;
                if dominion_active {
                    let stacks = world
                        .get::<auras::ActiveAuras>(ctx.caster)
                        .and_then(|active| {
                            active
                                .iter()
                                .find(|aura| aura.spell_id == 377253 && aura.caster == ctx.caster)
                        })
                        .expect("active Frozen Dominion")
                        .stacks;
                    auras::set_owned_stacks(
                        world,
                        ctx.caster,
                        ctx.caster,
                        377253,
                        stacks.saturating_add(1),
                    );
                } else {
                    spell_cast::cast_triggered(world, ctx.caster, 377253, Some(ctx.caster), 1);
                    dominion_active = auras::has_aura(world, ctx.caster, 377253);
                }
            }
        }
    }
    world.entity_mut(ctx.caster).insert(state);
}

fn reset_freezing_tempest(world: &mut World, caster: Entity, spell: u32, _: u8) {
    if matches!(spell, 196770 | 1233152) && is_frost(world, caster) {
        world.entity_mut(caster).insert(WinterTargets::default());
        // SimC76f8118 sc_death_knight.cpp12443–12450: a new Winter clears the previous stack.
        auras::remove_auras(world, caster, auras::RemoveReason::Consumed, |aura| {
            aura.spell_id == 1297365 && aura.caster == caster
        });
    }
}

fn frost_fever_brittle(world: &mut World, ctx: &spell_cast::effects::EffectCtx, target: Entity) {
    if ctx.spell.id != 55095
        || !is_frost(world, ctx.caster)
        || !auras::has_aura(world, ctx.caster, 374504)
    {
        return;
    }
    let chance = world
        .resource::<SpellStore>()
        .get(374504)
        .and_then(|spell| spell.aura_options.as_ref())
        .expect("Brittle has native proc chance")
        .proc_chance;
    // SimC76f8118 sc_death_knight.cpp7814–7822: each actual disease tick rolls Brittle.
    if random::random::<f32>() * 100.0 < chance as f32
        && !spell_cast::try_cast_triggered(world, ctx.caster, 374557, Some(target), 1)
    {
        warn!(caster = ?ctx.caster, ?target, "Native Brittle debuff refused");
    }
}

fn glacial_advance_razorice(
    world: &mut World,
    ctx: &spell_cast::effects::EffectCtx,
    target: Entity,
    _: bool,
    damage: f32,
) {
    if ctx.spell.id != 195975 || damage <= 0.0 || !is_frost(world, ctx.caster) {
        return;
    }
    // SimC76f8118 sc_death_knight.cpp11109–11122: each real Glacial Advance impact grants Razorice.
    if !spell_cast::try_cast_triggered(world, ctx.caster, 51714, Some(target), 1) {
        warn!(caster = ?ctx.caster, ?target, "Native Razorice debuff refused");
    }
}

fn frost_fever_runic_power(
    world: &mut World,
    ctx: &spell_cast::effects::EffectCtx,
    _target: Entity,
) {
    if ctx.spell.id != 55095
        || !is_frost(world, ctx.caster)
        || !auras::has_aura(world, ctx.caster, 195621)
    {
        return;
    }
    let count = world
        .query::<&auras::ActiveAuras>()
        .iter(world)
        .filter(|active| {
            active
                .iter()
                .any(|aura| aura.spell_id == 55095 && aura.caster == ctx.caster)
        })
        .count();
    if count == 0 {
        return;
    }
    // SimC76f8118 sc_death_knight.cpp8006–8022: 30% divided by the
    // square root of this caster's active Frost Fever target count.
    let chance = 0.30 / (count as f32).sqrt();
    if random::random::<f32>() < chance {
        spell_cast::cast_triggered(world, ctx.caster, 195617, Some(ctx.caster), ctx.depth + 1);
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
    trigger_dark_talons(world, caster);
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

fn trigger_dark_talons(world: &mut World, caster: Entity) {
    if !auras::has_aura(world, caster, 436687) || !auras::has_aura(world, caster, 194878) {
        return;
    }
    let chance = spell_cast::scripts::script_effect_value(world, caster, 436687, 0);
    if random::random::<f32>() * 100.0 >= chance {
        return;
    }
    let active = auras::has_aura(world, caster, 443595);
    if !spell_cast::try_cast_triggered(world, caster, 443595, Some(caster), 1) {
        warn!(?caster, "Native Dark Talons aura refused");
        return;
    }
    if !active {
        let stacks = spell_cast::scripts::script_effect_value(world, caster, 436687, 1) as u8;
        if !auras::adjust_owned_stack_cap(world, caster, 194879, stacks as i8) {
            cast_buff_stacks(world, caster, 194879, stacks);
        }
    }
}

fn dark_talons_ended(
    world: &mut World,
    unit: Entity,
    aura: &auras::AuraInstance,
    _: auras::RemoveReason,
) {
    let amount = spell_cast::scripts::script_effect_value(world, aura.caster, 436687, 1) as u8;
    let stacks = world
        .get::<auras::ActiveAuras>(unit)
        .and_then(|active| {
            active
                .iter()
                .find(|a| a.spell_id == 194879 && a.caster == aura.caster)
        })
        .map(|a| a.stacks);
    if stacks.is_some_and(|stacks| stacks <= amount) {
        auras::remove_auras(world, unit, auras::RemoveReason::Consumed, |a| {
            a.spell_id == 194879 && a.caster == aura.caster
        });
    } else {
        auras::adjust_owned_stack_cap(world, unit, 194879, -(amount as i8));
    }
}

fn icy_talons_started(world: &mut World, unit: Entity, aura: &auras::AuraInstance) {
    if auras::has_aura(world, aura.caster, 436687) {
        spell_cast::cast_triggered(world, aura.caster, 443586, Some(unit), 1);
    }
}

fn icy_talons_ended(
    world: &mut World,
    unit: Entity,
    aura: &auras::AuraInstance,
    _: auras::RemoveReason,
) {
    auras::remove_auras(world, unit, auras::RemoveReason::Consumed, |a| {
        matches!(a.spell_id, 443595 | 443586) && a.caster == aura.caster
    });
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
