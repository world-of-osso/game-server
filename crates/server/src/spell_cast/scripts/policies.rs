//! Explicit missing-script policies; native spell records still own payloads.
use bevy::prelude::*;
use shared::components::{ActiveSpec, PowerType};

use super::AfterCastHook;
use crate::auras::ActiveAuras;
use crate::class_progression::KnownSpells;
use crate::spell_cast::{CostPool, PlannedEffect, TargetPlan, effects::EffectCtx, targets};
use crate::spell_info::{CastDamageRole, CastPolicyContext, SpellInfo, SpellStore};

#[derive(Clone, Copy)]
struct ChannelMomentum {
    channel: u32,
    damage: u32,
    stacks: u32,
    speed: u32,
    haste_effect: u8,
}

#[derive(Clone, Copy)]
enum Relationship {
    SpenderPeriodicExtension {
        resource: PowerType,
        payload: u32,
        impact_effect: u8,
        area_parent: u32,
        area_effect: u8,
    },
    PeriodicSplash {
        parent: u32,
        payload: u32,
        magnitude_spell: u32,
        percent_effect: u8,
        cap_effect: u8,
        radius_slot: u8,
    },
    ApplySelfAuraAfterCast {
        parent: u32,
        payload: u32,
    },
    ConsumeSelfAuraAfterCast {
        parent: u32,
        payload: u32,
    },
    ChannelMomentum(ChannelMomentum),
    ReapplicationExtend {
        payload: u32,
    },
    LinkedAura {
        parent: u32,
        payload: u32,
    },
    CounterEchoAfterCast {
        parent: u32,
        counter: u32,
        payload: u32,
        effectiveness_effect: u8,
    },
    CastEcho {
        parent: u32,
        payload: u32,
        required_auras: &'static [u32],
        effectiveness_effect: u8,
    },
    SecondaryEffect {
        parent: u32,
        effect: u8,
        cap_effect: u8,
    },
    HitPayload {
        parent: u32,
        payload: u32,
        self_target: bool,
        primary_only: bool,
    },
    CounterAfterCast {
        parent: u32,
        counter: u32,
        count_effect: u8,
    },
    SpenderHit {
        resource: PowerType,
        counter: u32,
        payload: u32,
    },
}

struct Policy {
    name: &'static str,
    spec: u32,
    talent: u32,
    relationship: Relationship,
}

// Missing edges are explicit SimC policies, never invented DB2/TDB records.
const POLICIES: &[Policy] = &[
    // SimC76f8118 sc_hunter.cpp5089/4972,3843–3846: successful Barbed/Cobra
    // arms selected Nature's Ally; native1276720 owns its value and lifetime.
    Policy {
        name: "SIMC_BM_BARBED_NATURES_ALLY",
        spec: 253,
        talent: 1273126,
        relationship: Relationship::ApplySelfAuraAfterCast {
            parent: 217200,
            payload: 1276720,
        },
    },
    Policy {
        name: "SIMC_BM_COBRA_NATURES_ALLY",
        spec: 253,
        talent: 1273126,
        relationship: Relationship::ApplySelfAuraAfterCast {
            parent: 193455,
            payload: 1276720,
        },
    },
    // SimC76f8118 sc_hunter.cpp6633: consume only after Kill Command executes.
    Policy {
        name: "SIMC_BM_KILL_COMMAND_NATURES_ALLY",
        spec: 253,
        talent: 1273126,
        relationship: Relationship::ConsumeSelfAuraAfterCast {
            parent: 34026,
            payload: 1276720,
        },
    },
    // SimC76f8118 sc_monk.cpp3970–3998: selected Zenith Stomp fires on Zenith.
    Policy {
        name: "SIMC_WW_ZENITH_STOMP_CHILD",
        spec: 269,
        talent: 1272694,
        relationship: Relationship::HitPayload {
            parent: 1249625,
            payload: 1272696,
            self_target: false,
            primary_only: false,
        },
    },
    // SimC76f8118 sc_warrior.cpp3685–3686: Cleave applies the selected Rend
    // periodic payload on each landed target; native388539 owns all values.
    Policy {
        name: "SIMC_ARMS_CLEAVE_REND",
        spec: 71,
        talent: 772,
        relationship: Relationship::HitPayload {
            parent: 845,
            payload: 388539,
            self_target: false,
            primary_only: false,
        },
    },
    // SimC76f8118 sc_warrior.cpp4452–4456: only Fury Execute main-hand
    // applies Deep Wounds. Native262115 retains its coefficient and rolling pool.
    Policy {
        name: "SIMC_FURY_EXECUTE_DEEP_WOUNDS",
        spec: 72,
        talent: 1261060,
        relationship: Relationship::HitPayload {
            parent: 280849,
            payload: 262115,
            self_target: false,
            primary_only: false,
        },
    },
    // Momentum Boost tooltip: https://warcraft.wiki.gg/wiki/Momentum_Boost
    // SimC76f8118 sc_monk.cpp1699–1704/1730–1734/1839–1841: haste damage,
    // FoF-only stacks, reset and native60%/8s speed on end (also dot.cpp75–81).
    Policy {
        name: "SIMC_WW_MOMENTUM_BOOST",
        spec: 269,
        talent: 451294,
        relationship: Relationship::ChannelMomentum(ChannelMomentum {
            channel: 113656,
            damage: 117418,
            stacks: 451297,
            speed: 451298,
            haste_effect: 0,
        }),
    },
    // SimC76f8118 sc_paladin.hpp1492–1496/1581–1594: every spender
    // impact extends owned Dawnlight; primary Storm adds its multi-DoT bonus.
    Policy {
        name: "SIMC_RET_ENDLESS_GLEAM",
        spec: 70,
        talent: 1263787,
        relationship: Relationship::SpenderPeriodicExtension {
            resource: PowerType::HolyPower,
            payload: 431380,
            impact_effect: 1,
            area_parent: 53385,
            area_effect: 2,
        },
    },
    // SimC76f8118 sc_paladin.cpp2990–2998/3048–3053: resolved tick
    // times native percent, secondaries only; source also uses effect1 for cap.
    Policy {
        name: "SIMC_RET_DAWNLIGHT_PERIODIC_SPLASH",
        spec: 70,
        talent: 431377,
        relationship: Relationship::PeriodicSplash {
            parent: 431380,
            payload: 431399,
            magnitude_spell: 431581,
            percent_effect: 0,
            cap_effect: 0,
            radius_slot: 1,
        },
    },
    // SimC76f8118 sc_paladin.cpp3016–3026: DOT_EXTEND;
    // source note: "per bolas test Aug 21 2024". Native duration/phase stay owned.
    Policy {
        name: "SIMC_RET_DAWNLIGHT_REAPPLICATION_EXTEND",
        spec: 70,
        talent: 431377,
        relationship: Relationship::ReapplicationExtend { payload: 431380 },
    },
    // SimC76f8118 sc_paladin.cpp314–315: Hammer mode follows wings.
    Policy {
        name: "SIMC_RET_WRATH_HAMMER_MODE",
        spec: 70,
        talent: 1241288,
        relationship: Relationship::LinkedAura {
            parent: 31884,
            payload: 1277026,
        },
    },
    Policy {
        name: "SIMC_RET_BACKGROUND_WRATH_HAMMER_MODE",
        spec: 70,
        talent: 1241288,
        relationship: Relationship::LinkedAura {
            parent: 454351,
            payload: 1277026,
        },
    },
    // SimC76f8118 sc_paladin.cpp1571–1575,1633–1642,1738–1743:
    // only foreground Judgment/HoW spends one native Resonance stack; its
    // background echo must not classify/consume another stack.
    Policy {
        name: "SIMC_RET_RESONANCE_JUDGMENT",
        spec: 70,
        talent: 384027,
        relationship: Relationship::CounterEchoAfterCast {
            parent: 20271,
            counter: 1266308,
            payload: 20271,
            effectiveness_effect: 1,
        },
    },
    Policy {
        name: "SIMC_RET_RESONANCE_HAMMER",
        spec: 70,
        talent: 384027,
        relationship: Relationship::CounterEchoAfterCast {
            parent: 24275,
            counter: 1266308,
            payload: 24275,
            effectiveness_effect: 1,
        },
    },
    // SimC76f8118 sc_paladin.cpp1745–1748; Ret1126–1127:
    // HoW during wings mirrors BoJ at native talent effectiveness and ICD.
    Policy {
        name: "SIMC_RET_WALK_INTO_LIGHT",
        spec: 70,
        talent: 1263782,
        relationship: Relationship::CastEcho {
            parent: 24275,
            payload: 184575,
            required_auras: &[31884, 454351],
            effectiveness_effect: 2,
        },
    },
    Policy {
        name: "SIMC_RET_TOLL_WALK_INTO_LIGHT",
        spec: 70,
        talent: 1263782,
        relationship: Relationship::CastEcho {
            parent: 1279408,
            payload: 184575,
            required_auras: &[31884, 454351],
            effectiveness_effect: 2,
        },
    },
    // SimC76f8118 sc_paladin.cpp1481–1486,1550–1564,1689–1698:
    // primary uses effect1; Blessed Champion adds only secondary recipients.
    Policy {
        name: "SIMC_RET_JUDGMENT_SECONDARIES",
        spec: 70,
        talent: 403010,
        relationship: Relationship::SecondaryEffect {
            parent: 20271,
            effect: 2,
            cap_effect: 0,
        },
    },
    Policy {
        name: "SIMC_RET_HAMMER_SECONDARIES",
        spec: 70,
        talent: 403010,
        relationship: Relationship::SecondaryEffect {
            parent: 24275,
            effect: 2,
            cap_effect: 3,
        },
    },
    // SimC76f8118 sc_paladin.cpp1502–1521,1580–1590: Toll Judgment earns HP.
    Policy {
        name: "SIMC_RET_TOLL_JUDGMENT_HOLY_POWER",
        spec: 70,
        talent: 315867,
        relationship: Relationship::HitPayload {
            parent: 406957,
            payload: 220637,
            self_target: true,
            primary_only: true,
        },
    },
    // SimC76f8118 sc_paladin.cpp888–916: each successful primary strike
    // grants the native406834 effect0, independently of the old stack counter.
    Policy {
        name: "SIMC_RET_CRUSADING_STRIKE_HOLY_POWER",
        spec: 70,
        talent: 404542,
        relationship: Relationship::HitPayload {
            parent: 408385,
            payload: 406834,
            self_target: true,
            primary_only: true,
        },
    },
    // SimC76f8118 sc_paladin_retribution.cpp432–436: BoJ impact applies
    // Expurgation. Native404358 carries the secondary BoJ target damage.
    Policy {
        name: "SIMC_RET_BLADE_EXPURGATION",
        spec: 70,
        talent: 383344,
        relationship: Relationship::HitPayload {
            parent: 184575,
            payload: 383346,
            self_target: false,
            primary_only: false,
        },
    },
    Policy {
        name: "SIMC_RET_BLADE_SECONDARY_EXPURGATION",
        spec: 70,
        talent: 383344,
        relationship: Relationship::HitPayload {
            parent: 404358,
            payload: 383346,
            self_target: false,
            primary_only: false,
        },
    },
    // sc_paladin_retribution.cpp846–876; native raw2 + specialization1 wins.
    Policy {
        name: "SIMC_RET_DAWNLIGHT_WAKE_PRODUCER",
        spec: 70,
        talent: 431377,
        relationship: Relationship::CounterAfterCast {
            parent: 255937,
            counter: 431522,
            count_effect: 0,
        },
    },
    // sc_paladin.hpp1512–1535: successful foreground first hit, one use.
    Policy {
        name: "SIMC_RET_DAWNLIGHT_SPENDER_CONSUMER",
        spec: 70,
        talent: 431377,
        relationship: Relationship::SpenderHit {
            resource: PowerType::HolyPower,
            counter: 431522,
            payload: 431380,
        },
    },
];

fn spender_extensions(spell: &SpellInfo) -> impl Iterator<Item = &'static Policy> + '_ {
    POLICIES.iter().filter(move |policy| {
        let Relationship::SpenderPeriodicExtension { resource, .. } = policy.relationship else {
            return false;
        };
        spell.cast_damage_role != CastDamageRole::Secondary
            && spell
                .power_costs
                .iter()
                .any(|cost| cost.power_type == resource as i32)
    })
}

pub(in crate::spell_cast) fn has_impact_policy(spell: &SpellInfo) -> bool {
    spender_extensions(spell).next().is_some()
}

pub(in crate::spell_cast) fn before_impact(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    for policy in spender_extensions(ctx.spell) {
        if selected(world, ctx.caster, policy) {
            extend_spender_impact(world, ctx.caster, target, policy);
        }
    }
}

fn extend_spender_impact(world: &mut World, caster: Entity, target: Entity, policy: &Policy) {
    let Relationship::SpenderPeriodicExtension {
        payload,
        impact_effect,
        ..
    } = policy.relationship
    else {
        return;
    };
    if !owns_periodic(world, caster, target, payload) {
        return;
    }
    if let Some(milliseconds) = read_extension_ms(world, caster, policy, impact_effect) {
        crate::auras::extend_owned_duration(world, caster, target, &[payload], milliseconds);
    }
}

pub(in crate::spell_cast) fn after_impacts(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    targets: &[Entity],
) {
    // Ret495–532 echo bypasses the consumer execute; its impact still extends.
    if ctx.spell.cast_damage_role != CastDamageRole::Primary {
        return;
    }
    for policy in spender_extensions(ctx.spell) {
        let Relationship::SpenderPeriodicExtension { area_parent, .. } = policy.relationship else {
            continue;
        };
        if ctx.spell.id == area_parent && selected(world, ctx.caster, policy) {
            extend_spender_area(world, ctx.caster, targets, policy);
        }
    }
}

fn read_owned_targets(
    world: &World,
    caster: Entity,
    payload: u32,
    targets: &[Entity],
) -> Vec<Entity> {
    targets
        .iter()
        .copied()
        .filter(|&target| owns_periodic(world, caster, target, payload))
        .collect()
}

fn extend_spender_area(world: &mut World, caster: Entity, targets: &[Entity], policy: &Policy) {
    let Relationship::SpenderPeriodicExtension {
        payload,
        area_effect,
        ..
    } = policy.relationship
    else {
        return;
    };
    let owned = read_owned_targets(world, caster, payload, targets);
    if owned.len() < 2 {
        return;
    }
    let Some(milliseconds) = read_extension_ms(world, caster, policy, area_effect) else {
        return;
    };
    for target in owned {
        crate::auras::extend_owned_duration(world, caster, target, &[payload], milliseconds);
    }
}

fn read_extension_ms(world: &World, caster: Entity, policy: &Policy, effect: u8) -> Option<u32> {
    let amount = super::script_effect_value(world, caster, policy.talent, effect);
    if !amount.is_finite() || !(0.0..=u32::MAX as f32).contains(&amount) {
        warn!(
            policy = policy.name,
            effect, amount, "Native periodic extension invalid"
        );
        return None;
    }
    Some(amount.round() as u32)
}

fn splash_policies(parent: u32) -> impl Iterator<Item = &'static Policy> {
    POLICIES.iter().filter(move |policy| {
        matches!(policy.relationship,
        Relationship::PeriodicSplash { parent: id, .. } if id == parent)
    })
}

pub(in crate::spell_cast) fn after_periodic_damage(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    target: Entity,
    amount: f32,
) {
    for policy in splash_policies(ctx.spell.id) {
        if !selected(world, ctx.caster, policy) {
            continue;
        }
        let accepted = cast_periodic_splash(world, ctx, target, amount, policy);
        if !accepted {
            warn!(
                policy = policy.name,
                ?target,
                "Native periodic splash refused"
            );
        }
    }
}

fn cast_periodic_splash(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    target: Entity,
    amount: f32,
    policy: &Policy,
) -> bool {
    let Relationship::PeriodicSplash {
        payload,
        magnitude_spell,
        percent_effect,
        ..
    } = policy.relationship
    else {
        unreachable!("periodic splash policy");
    };
    let percent = super::script_effect_value(world, ctx.caster, magnitude_spell, percent_effect);
    // cpp3052–3053 supplies computed tick damage to the native child, not a tuned coefficient.
    let overrides = super::super::TriggeredSpellOverrides {
        effect0_amount: Some(amount * percent / 100.0),
        ..Default::default()
    };
    super::super::cast_triggered_with_overrides(
        world,
        ctx.caster,
        payload,
        Some(target),
        ctx.depth.saturating_add(1),
        overrides,
    )
}

fn splash_policy(payload: u32) -> Option<&'static Policy> {
    POLICIES.iter().find(|policy| {
        matches!(policy.relationship,
        Relationship::PeriodicSplash { payload: id, .. } if id == payload)
    })
}

pub(super) fn target_plan_hook(spell: u32) -> Option<(targets::Relation, super::TargetPlanHook)> {
    splash_policy(spell).map(|_| {
        (
            targets::Relation::Enemy,
            splash_target_plan as super::TargetPlanHook,
        )
    })
}

fn read_splash_geometry(world: &World, caster: Entity, spell: &SpellInfo) -> (f32, f32) {
    let policy = splash_policy(spell.id).expect("registered periodic splash policy");
    let Relationship::PeriodicSplash {
        magnitude_spell,
        cap_effect,
        radius_slot,
        ..
    } = policy.relationship
    else {
        unreachable!("periodic splash policy");
    };
    // Native destination is slot0; its area selector/radius is slot1.
    let radius = targets::radius(&spell.effects[0], usize::from(radius_slot));
    let cap = super::script_effect_value(world, caster, magnitude_spell, cap_effect);
    (radius, cap)
}

fn soft_cap_multiplier(count: usize, cap: f32) -> f32 {
    // SimC76f8118 action.cpp1562–1579; max_aoe_enemies defaults to20.
    const MAX_AOE_ENEMIES: usize = 20;
    let reduced = if cap > 0.0 && count as f32 > cap {
        (cap / count.min(MAX_AOE_ENEMIES) as f32).sqrt()
    } else {
        1.0
    };
    let capped = if count > MAX_AOE_ENEMIES {
        MAX_AOE_ENEMIES as f32 / count as f32
    } else {
        1.0
    };
    reduced * capped
}

fn splash_target_plan(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
) -> TargetPlan {
    let primary = explicit.expect("native periodic splash requires its actual primary");
    let center = targets::position(world, primary).expect("periodic splash primary position");
    let (radius, cap) = read_splash_geometry(world, caster, spell);
    let mut units = targets::area_units(world, caster, center, radius, targets::Relation::Enemy, 0);
    units.retain(|&unit| unit != primary);
    let reduced = soft_cap_multiplier(units.len(), cap);
    let chain_multipliers = units.iter().map(|&unit| (unit, reduced)).collect();
    vec![PlannedEffect {
        index: 0,
        units,
        dest: None,
        chain_multipliers,
        selected_units: Vec::new(),
        blocked_units: Vec::new(),
        prevent_hit_script: false,
    }]
}

fn channel_policies() -> impl Iterator<Item = (&'static Policy, ChannelMomentum)> {
    POLICIES
        .iter()
        .filter_map(|policy| match policy.relationship {
            Relationship::ChannelMomentum(channel) => Some((policy, channel)),
            _ => None,
        })
}

pub(crate) fn register_channel_policies(scripts: &mut crate::auras::scripts::AuraScripts) {
    for (policy, channel) in channel_policies() {
        // The synchronous damage consumer owns this producer, not broad DB2 masks.
        scripts.0.entry(policy.talent).or_default().check_proc = Some(block_metadata_proc);
        scripts.0.entry(channel.channel).or_default().on_remove = Some(end_channel_policy);
    }
}

fn block_metadata_proc(
    _world: &World,
    _aura: &crate::auras::AuraInstance,
    _event: &crate::auras::procs::AuraEvent,
) -> bool {
    false
}

pub(super) fn after_damage(world: &mut World, ctx: &EffectCtx<'_>) {
    for (policy, channel) in
        channel_policies().filter(|(_, channel)| channel.damage == ctx.spell.id)
    {
        if selected(world, ctx.caster, policy) {
            crate::spell_cast::cast_triggered(
                world,
                ctx.caster,
                channel.stacks,
                Some(ctx.caster),
                ctx.depth + 1,
            );
        }
    }
}

pub(super) fn damage_hook(spell: u32) -> Option<super::DamageAmountHook> {
    channel_policies()
        .any(|(_, channel)| channel.damage == spell)
        .then_some(channel_haste_damage)
}

fn channel_haste_damage(world: &World, caster: Entity, _target: Entity, amount: f32) -> f32 {
    let Some((policy, channel)) =
        channel_policies().find(|(policy, _)| selected(world, caster, policy))
    else {
        return amount;
    };
    let percent = super::script_effect_value(world, caster, policy.talent, channel.haste_effect);
    let haste = world
        .get::<crate::stats::CharacterStats>(caster)
        .expect("selected class damage policy requires native stats")
        .haste_pct;
    let bonus = haste / 100.0 * percent / 100.0;
    amount * (1.0 + bonus)
}

fn end_channel_policy(
    world: &mut World,
    unit: Entity,
    aura: &crate::auras::AuraInstance,
    reason: crate::auras::RemoveReason,
) {
    for (policy, channel) in
        channel_policies().filter(|(_, channel)| channel.channel == aura.spell_id)
    {
        if unit != aura.caster || !selected(world, unit, policy) {
            continue;
        }
        crate::auras::remove_auras(world, unit, crate::auras::RemoveReason::Consumed, |buff| {
            buff.caster == unit && buff.spell_id == channel.stacks
        });
        if matches!(
            reason,
            crate::auras::RemoveReason::ChannelEnded | crate::auras::RemoveReason::Expired
        ) {
            crate::spell_cast::cast_triggered(world, unit, channel.speed, Some(unit), 1);
        }
    }
    super::windwalker_children::channel_ended(world, unit, aura, reason);
}

pub(crate) fn extends_reapplication(world: &World, caster: Entity, payload: u32) -> bool {
    POLICIES.iter().any(|policy| {
        let matches_payload = matches!(policy.relationship,
            Relationship::ReapplicationExtend { payload: id } if id == payload);
        matches_payload && selected(world, caster, policy)
    })
}

pub(crate) fn register_lifecycle_policies(scripts: &mut crate::auras::scripts::AuraScripts) {
    for policy in POLICIES {
        if let Relationship::LinkedAura { parent, .. } = policy.relationship {
            scripts.0.insert(
                parent,
                crate::auras::scripts::AuraScript {
                    on_apply: Some(apply_linked_aura),
                    on_remove: Some(remove_linked_aura),
                    ..Default::default()
                },
            );
        }
    }
}

fn apply_linked_aura(world: &mut World, unit: Entity, aura: &crate::auras::AuraInstance) {
    for policy in POLICIES {
        let Relationship::LinkedAura { parent, payload } = policy.relationship else {
            continue;
        };
        if parent == aura.spell_id && selected(world, unit, policy) {
            let depth = super::super::next_trigger_depth(world);
            super::super::cast_triggered(world, unit, payload, Some(unit), depth);
        }
    }
}

fn remove_linked_aura(
    world: &mut World,
    unit: Entity,
    aura: &crate::auras::AuraInstance,
    reason: crate::auras::RemoveReason,
) {
    for policy in POLICIES {
        let Relationship::LinkedAura { parent, payload } = policy.relationship else {
            continue;
        };
        if parent == aura.spell_id {
            crate::auras::remove_auras(world, unit, reason, |child| {
                child.spell_id == payload && child.caster == aura.caster
            });
        }
    }
}

pub(crate) fn filter_secondary_targets(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    effect: u8,
    primary: Option<Entity>,
    units: &mut Vec<Entity>,
) {
    for policy in POLICIES {
        let Relationship::SecondaryEffect {
            parent,
            effect: index,
            cap_effect,
        } = policy.relationship
        else {
            continue;
        };
        if parent != spell.id || index != effect {
            continue;
        }
        filter_policy_secondaries(world, caster, policy, cap_effect, primary, units);
    }
}

fn filter_policy_secondaries(
    world: &World,
    caster: Entity,
    policy: &Policy,
    cap_effect: u8,
    primary: Option<Entity>,
    units: &mut Vec<Entity>,
) {
    if !selected(world, caster, policy) {
        units.clear();
        return;
    }
    units.retain(|unit| Some(*unit) != primary);
    let cap = super::script_effect_value(world, caster, policy.talent, cap_effect);
    if !cap.is_finite() || cap < 0.0 {
        warn!(
            policy = policy.name,
            cap, "Native secondary target cap invalid"
        );
        units.clear();
        return;
    }
    units.truncate(cap as usize);
}

pub(in crate::spell_cast) fn after_triggered_cast(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    depth: u8,
) {
    // SimC's common HoW execute also mirrors on Toll/Resonance/Sunrise.
    // Counter consumers remain foreground-only, so same-ID echoes cannot recurse.
    for policy in producers(spell.id) {
        if matches!(policy.relationship, Relationship::CastEcho { .. })
            && selected(world, caster, policy)
        {
            cast_echo(world, caster, spell, policy, depth.saturating_add(1));
        }
    }
}

fn cast_counter_echo(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    policy: &Policy,
    depth: u8,
) {
    let Relationship::CounterEchoAfterCast { counter, .. } = policy.relationship else {
        return;
    };
    let Some(target) = spell.cast_primary_target else {
        warn!(
            policy = policy.name,
            "Native counter echo primary unavailable"
        );
        return;
    };
    let Some(instance) = read_counter_instance(world, caster, counter) else {
        return;
    };
    if !crate::auras::procs::try_policy_proc(world, caster, instance, 100.0) {
        return;
    }
    if launch_scaled_payload(world, caster, target, policy, depth) {
        crate::auras::spend_stack(world, caster, instance);
    }
}

fn cast_echo(world: &mut World, caster: Entity, spell: &SpellInfo, policy: &Policy, depth: u8) {
    let Relationship::CastEcho { required_auras, .. } = policy.relationship else {
        return;
    };
    let Some(target) = spell.cast_primary_target else {
        warn!(policy = policy.name, "Native cast echo primary unavailable");
        return;
    };
    if !try_echo_proc(world, caster, policy, required_auras) {
        return;
    }
    launch_scaled_payload(world, caster, target, policy, depth);
}

fn try_echo_proc(
    world: &mut World,
    caster: Entity,
    policy: &Policy,
    required_auras: &[u32],
) -> bool {
    if !required_auras
        .iter()
        .any(|id| crate::auras::has_aura(world, caster, *id))
    {
        return false;
    }
    let Some(instance) = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| auras.iter().find(|aura| aura.spell_id == policy.talent))
        .map(|aura| aura.instance_id)
    else {
        return false;
    };
    crate::auras::procs::try_policy_proc(world, caster, instance, 100.0)
}

fn read_echo_payload(world: &World, caster: Entity, policy: &Policy) -> (u32, f32) {
    let (payload, source, effect) = match policy.relationship {
        Relationship::CastEcho {
            payload,
            effectiveness_effect,
            ..
        } => (payload, policy.talent, effectiveness_effect),
        Relationship::CounterEchoAfterCast {
            counter,
            payload,
            effectiveness_effect,
            ..
        } => (payload, counter, effectiveness_effect),
        _ => unreachable!("scaled echo policy"),
    };
    let effectiveness = super::script_effect_value(world, caster, source, effect);
    (payload, effectiveness / 100.0)
}

fn launch_scaled_payload(
    world: &mut World,
    caster: Entity,
    target: Entity,
    policy: &Policy,
    depth: u8,
) -> bool {
    let (payload, multiplier) = read_echo_payload(world, caster, policy);
    let Some(mut child) = super::super::load_triggered_spell(world, payload, depth) else {
        return false;
    };
    child.cast_damage_multiplier = Some(multiplier);
    let accepted = super::super::launch_triggered_spell(world, caster, &child, Some(target), depth);
    if !accepted {
        warn!(
            policy = policy.name,
            ?caster,
            ?target,
            "Native cast echo refused"
        );
    }
    accepted
}

// CastEcho owns its source event predicate and the native shared ICD. Raw
// CAST_SUCCESSFUL hints must not consume that ICD on unrelated/self phases.
pub(crate) fn owns_policy_proc(spell: u32) -> bool {
    POLICIES.iter().any(|policy| match policy.relationship {
        Relationship::CastEcho { .. } => policy.talent == spell,
        Relationship::CounterEchoAfterCast { counter, .. } => counter == spell,
        _ => false,
    })
}

fn hit_payloads(parent: u32) -> impl Iterator<Item = &'static Policy> {
    POLICIES.iter().filter(move |policy| {
        matches!(policy.relationship,
            Relationship::HitPayload { parent: id, .. } if id == parent)
    })
}

pub(super) fn hit_hook(parent: u32) -> Option<super::ContextHitHook> {
    hit_payloads(parent)
        .next()
        .map(|_| cast_hit_payload as super::ContextHitHook)
}

fn cast_hit_payload(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    // Self auras can resolve before a hostile missile; that local phase's
    // first TargetInfo is not the original action primary.
    let primary = ctx.spell.cast_primary_target;
    for policy in hit_payloads(ctx.spell.id) {
        let Relationship::HitPayload {
            payload,
            self_target,
            primary_only,
            ..
        } = policy.relationship
        else {
            unreachable!("hit-payload policy iterator");
        };
        if !selected(world, ctx.caster, policy) || (primary_only && primary != Some(target)) {
            continue;
        }
        let recipient = if self_target { ctx.caster } else { target };
        super::super::cast_triggered(world, ctx.caster, payload, Some(recipient), ctx.depth + 1);
    }
}

fn producers(parent: u32) -> impl Iterator<Item = &'static Policy> {
    POLICIES.iter().filter(move |policy| {
        matches!(policy.relationship,
        Relationship::ApplySelfAuraAfterCast { parent: trigger, .. }
        | Relationship::ConsumeSelfAuraAfterCast { parent: trigger, .. }
        | Relationship::CounterAfterCast { parent: trigger, .. }
        | Relationship::CastEcho { parent: trigger, .. }
        | Relationship::CounterEchoAfterCast { parent: trigger, .. } if trigger == parent)
    })
}

fn consumers(powers: &[PowerType]) -> impl Iterator<Item = &'static Policy> + '_ {
    POLICIES.iter().filter(move |policy| {
        matches!(policy.relationship,
        Relationship::SpenderHit { resource, .. } if powers.contains(&resource))
    })
}

pub(super) fn after_cast_hook(parent: u32) -> Option<AfterCastHook> {
    producers(parent)
        .next()
        .map(|_| apply_after_cast as AfterCastHook)
}

fn selected(world: &World, caster: Entity, policy: &Policy) -> bool {
    let correct_spec = world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == policy.spec);
    // Learned native abilities include specialization grants as well as selected traits.
    let owns_talent = world
        .get::<KnownSpells>(caster)
        .is_some_and(|known| known.contains(policy.talent));
    correct_spec && owns_talent
}

fn apply_after_cast(world: &mut World, caster: Entity, spell: &SpellInfo) {
    for policy in producers(spell.id) {
        if !selected(world, caster, policy) {
            continue;
        }
        match policy.relationship {
            Relationship::ApplySelfAuraAfterCast { payload, .. } => {
                let depth = super::super::next_trigger_depth(world);
                super::super::cast_triggered(world, caster, payload, Some(caster), depth);
            }
            Relationship::ConsumeSelfAuraAfterCast { payload, .. } => {
                crate::auras::remove_auras(
                    world,
                    caster,
                    crate::auras::RemoveReason::Consumed,
                    |aura| aura.spell_id == payload && aura.caster == caster,
                );
            }
            Relationship::CounterAfterCast { .. } => grant_counter(world, caster, policy),
            Relationship::CastEcho { .. } => {
                let depth = super::super::next_trigger_depth(world);
                cast_echo(world, caster, spell, policy, depth);
            }
            Relationship::CounterEchoAfterCast { .. } => {
                let depth = super::super::next_trigger_depth(world);
                cast_counter_echo(world, caster, spell, policy, depth);
            }
            _ => unreachable!("after-cast policy iterator"),
        }
    }
}

fn read_counter_amount(world: &World, caster: Entity, policy: &Policy, effect: u8) -> Option<u8> {
    let count = super::script_effect_value(world, caster, policy.talent, effect);
    if !count.is_finite() || !(1.0..=f32::from(u8::MAX)).contains(&count) {
        warn!(
            policy = policy.name,
            ?caster,
            count,
            "Native policy counter amount invalid"
        );
        return None;
    }
    Some(count as u8)
}

fn grant_counter(world: &mut World, caster: Entity, policy: &Policy) {
    let Relationship::CounterAfterCast {
        counter,
        count_effect,
        ..
    } = policy.relationship
    else {
        return;
    };
    let Some(count) = read_counter_amount(world, caster, policy, count_effect) else {
        return;
    };
    launch_counter(world, caster, policy, counter, count);
}

fn launch_counter(world: &mut World, caster: Entity, policy: &Policy, counter: u32, count: u8) {
    let Some(mut spell) = world.resource::<SpellStore>().get(counter).cloned() else {
        warn!(
            policy = policy.name,
            spell_id = counter,
            "Native policy counter unavailable"
        );
        return;
    };
    spell.cast_aura_stacks = Some(count);
    if !super::super::launch_triggered_spell(world, caster, &spell, Some(caster), 1) {
        warn!(
            policy = policy.name,
            ?caster,
            "Native policy counter refused"
        );
    }
}

/// Only ordinary successful launch snapshots enter this path; FULL_MASK does not.
/// Retain applicable pools even at zero final cost, and original selected targets.
pub(in crate::spell_cast) fn snapshot(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    plan: &TargetPlan,
    costs: &[(CostPool, f32)],
) -> Option<Box<CastPolicyContext>> {
    let powers = costs
        .iter()
        .filter_map(|(pool, _)| match pool {
            CostPool::Power(power) => Some(*power),
            CostPool::Health => None,
        })
        .collect::<Vec<_>>();
    if spell.cast_parent_owns_modifier_charges
        || !consumers(&powers).any(|policy| selected(world, caster, policy))
    {
        return None;
    }
    let targets = capture_primary_targets(caster, spell, plan)?;
    Some(Box::new(CastPolicyContext { powers, targets }))
}

fn capture_primary_targets(
    caster: Entity,
    spell: &SpellInfo,
    plan: &TargetPlan,
) -> Option<Vec<Entity>> {
    let Some(primary) = spell.cast_primary_target else {
        warn!(
            ?caster,
            spell = spell.id,
            "NATIVE_POLICY_PRIMARY_UNAVAILABLE: no request context"
        );
        return None;
    };
    let selected = super::super::selected_units(plan);
    if !selected.contains(&primary) {
        warn!(
            ?caster,
            spell = spell.id,
            ?primary,
            "NATIVE_POLICY_PRIMARY_UNAVAILABLE: not selected"
        );
        return None;
    }
    Some(
        std::iter::once(primary)
            .chain(selected.into_iter().filter(|&target| target != primary))
            .collect(),
    )
}

pub(in crate::spell_cast) fn on_hit(world: &mut World, ctx: &EffectCtx<'_>, hit: Entity) {
    let Some(context) = &ctx.spell.cast_policy_context else {
        return;
    };
    if context.targets.first() != Some(&hit) {
        return;
    }
    for policy in consumers(&context.powers) {
        if selected(world, ctx.caster, policy) {
            consume_on_hit(world, ctx.caster, policy, context);
        }
    }
}

fn read_counter_instance(world: &World, caster: Entity, counter: u32) -> Option<u32> {
    world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == counter && aura.caster == caster && aura.stacks > 0)
        .map(|aura| aura.instance_id)
}

fn consume_on_hit(world: &mut World, caster: Entity, policy: &Policy, context: &CastPolicyContext) {
    let Relationship::SpenderHit {
        counter, payload, ..
    } = policy.relationship
    else {
        return;
    };
    let Some(instance) = read_counter_instance(world, caster, counter) else {
        return;
    };
    let Some(targets) = read_payload_targets(world, caster, policy, payload, &context.targets)
    else {
        return;
    };
    let Some(target) = choose_target(world, caster, payload, &targets) else {
        return;
    };
    if !super::super::try_cast_triggered(world, caster, payload, Some(target), 1) {
        warn!(
            policy = policy.name,
            ?caster,
            ?target,
            "Native policy payload refused"
        );
        return;
    }
    crate::auras::spend_stack(world, caster, instance);
}

fn read_payload_targets(
    world: &World,
    caster: Entity,
    policy: &Policy,
    payload: u32,
    selected: &[Entity],
) -> Option<Vec<Entity>> {
    let Some(spell) = world.resource::<SpellStore>().get(payload) else {
        warn!(
            policy = policy.name,
            payload, "Native policy payload unavailable"
        );
        return None;
    };
    let relation = targets::explicit_requirement(spell.effects.iter());
    let accepts = |&unit: &Entity| {
        relation.is_none_or(|relation| targets::relation_matches(world, caster, unit, relation))
            && (spell.allows_dead_target() || targets::is_alive(world, unit))
    };
    let primary = selected.first()?;
    if !accepts(primary) {
        warn!(
            policy = policy.name,
            ?primary,
            "NATIVE_POLICY_PRIMARY_REFUSED: no target substitute"
        );
        return None;
    }
    Some(
        selected
            .iter()
            .filter(|unit| accepts(unit))
            .copied()
            .collect(),
    )
}

fn owns_periodic(world: &World, caster: Entity, target: Entity, payload: u32) -> bool {
    world.get::<ActiveAuras>(target).is_some_and(|auras| {
        auras.iter().any(|aura| {
            aura.caster == caster
                && aura.spell_id == payload
                && aura.effects.iter().any(|effect| effect.period_ms > 0)
        })
    })
}

fn choose_target(
    world: &World,
    caster: Entity,
    payload: u32,
    targets: &[Entity],
) -> Option<Entity> {
    let primary = *targets.first()?;
    if targets.len() == 1 || !owns_periodic(world, caster, primary, payload) {
        return Some(primary);
    }
    targets
        .iter()
        .copied()
        .find(|&target| !owns_periodic(world, caster, target, payload))
}
