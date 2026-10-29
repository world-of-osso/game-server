//! Holy Armaments alternate through the native action-bar override aura.
use super::combat;
use bevy::prelude::*;
use combat::auras::{
    AuraApplication, AuraInstance, AuraSource, RemoveReason, scripts::AuraScripts,
};
use combat::class_progression::TraitLoadout;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::{SpellInfo, SpellStore};
use combat::stats::{CharacterStats, EquippedItemModifiers};
use shared::components::{ActiveSpec, Health, PowerType};
use shared::protocol::EquipmentSlot;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::SuccessfulCastRelationships(after_cast),
    SpellScriptHook::LateAuraRegistration(register_bulwark),
    SpellScriptHook::AfterAuraApplication(initial_bulwark),
    SpellScriptHook::AfterAbsorbConsumed(reflection_from_absorb),
    SpellScriptHook::AvoidanceProcHitMask(grand_crusader_avoidance_mask),
    SpellScriptHook::ResolvedDamage(glory_from_shield),
    SpellScriptHook::ResolvedDamage(grant_bulwark_of_righteous_fury),
    SpellScriptHook::OnCast(glory_on_cast),
    SpellScriptHook::EffectHandler(armament_energize_handler),
    SpellScriptHook::EffectHandler(rite_of_sanctification_handler),
    SpellScriptHook::AuraEvent(divine_inspiration_event),
    SpellScriptHook::OwnsPolicyProc(|spell| spell == 432964),
    SpellScriptHook::AfterNativeCastUpdate(bind_blessed_hammer_fields),
    SpellScriptHook::EnemyContextHit(blessed_hammer_hit),
    SpellScriptHook::TargetDamageMultiplier(valor_shield_multiplier),
    SpellScriptHook::TargetDamageMultiplier(ferren_marcus_multiplier),
    SpellScriptHook::TargetDamageMultiplier(focused_enmity_multiplier),
    SpellScriptHook::AfterResolvedHits(consume_valor),
    SpellScriptHook::AfterResolvedHits(consume_bulwark_of_righteous_fury),
    SpellScriptHook::AreaFilter(blaze_area_filter),
    SpellScriptHook::AfterSpenderStacks(advance_afterimage),
];

fn register_bulwark(scripts: &mut AuraScripts) {
    let script = scripts.0.entry(432496).or_default();
    script.on_periodic = Some(periodic_bulwark);
    let grand = scripts.0.entry(85043).or_default();
    grand.check_proc = Some(grand_crusader_is_avoidance);
    grand.on_damage_proc = Some(grand_crusader_from_avoidance);
}

#[derive(Component)]
struct BoundBlessedHammer;

fn bind_blessed_hammer_fields(world: &mut World) {
    use combat::spell_cast::{area_trigger::SpellAreaTrigger, targets::Relation};
    let fields: Vec<_> = world
        .query_filtered::<(Entity, &SpellAreaTrigger), Without<BoundBlessedHammer>>()
        .iter(world)
        .filter(|(_, field)| field.spell_id == 204019)
        .map(|(entity, field)| (entity, field.caster, field.expires_at))
        .collect();
    for (entity, caster, expiry) in fields {
        let Some(expiry) = expiry else {
            warn!(?entity, "Blessed Hammer field lacks native duration");
            continue;
        };
        let duration = {
            let spell = world
                .resource::<SpellStore>()
                .get(204019)
                .expect("Blessed Hammer spell");
            combat::spell_cast::duration_ms(world, caster, spell)
        };
        if duration <= 0 {
            warn!(?caster, "Blessed Hammer duration is invalid");
            continue;
        }
        // SimC76f8118 sc_paladin_protection.cpp358–366,465–490: its default
        // approximates two strikes at 25% and 75% of duration, within 9 yards.
        // Native create-properties6006 are absent, so orbit geometry is unknown.
        let mut field = world
            .get_mut::<SpellAreaTrigger>(entity)
            .expect("live hammer field");
        field.trigger_spell = 204301;
        field.radius = 9.0;
        field.relation = Relation::Enemy;
        field.period_ms = duration as u32 / 2;
        field.next_tick_at = expiry - f64::from(duration) * 0.00075;
        world.entity_mut(entity).insert(BoundBlessedHammer);
    }
}

fn blessed_hammer_hit(
    spell: u32,
) -> Option<combat::spell_cast::script_registry::EnemyContextHitHook> {
    (spell == 204301).then_some(grant_blessed_hammer_absorb)
}

fn grant_blessed_hammer_absorb(
    world: &mut World,
    ctx: &combat::spell_cast::effects::EffectCtx<'_>,
    attacker: Entity,
) {
    if !selected(world, ctx.caster, 204019) {
        return;
    }
    // SimC76f8118 sc_paladin_protection.cpp390–395, 815–843: the next hit
    // from this enemy absorbs 30% of snapshotted attack power before shields.
    let attack_power = world
        .get::<combat::stats::CharacterStats>(ctx.caster)
        .expect("Blessed Hammer caster stats")
        .attack_power;
    let percent = combat::spell_cast::scripts::script_effect_value(world, ctx.caster, 204019, 0);
    let duration = ctx
        .spell
        .misc
        .as_ref()
        .and_then(|misc| misc.duration)
        .expect("Blessed Hammer debuff duration")
        .duration_ms;
    combat::auras::source_absorb::grant(
        world,
        ctx.caster,
        attacker,
        204301,
        attack_power * percent / 100.0,
        duration as u32,
    );
}

#[derive(Component)]
struct NextInspirationArmament(u32);

fn divine_inspiration_event(world: &mut World, event: &combat::auras::procs::AuraEvent) {
    use combat::auras::procs;
    const MASK: u32 = procs::PROC_DEAL_MELEE_ABILITY
        | procs::PROC_DEAL_RANGED_ABILITY
        | procs::PROC_DEAL_HARMFUL_ABILITY
        | procs::PROC_DEAL_HARMFUL_SPELL
        | procs::PROC_DEAL_HELPFUL_ABILITY
        | procs::PROC_DEAL_HELPFUL_SPELL
        | procs::PROC_DEAL_HELPFUL_PERIODIC;
    // SimC76f8118 sc_paladin.cpp1361–1376 lets the sourced Edict child
    // participate in ordinary callbacks despite its triggered native cast.
    if (event.can_proc || event.spell_id == Some(1300662))
        && event.actor_mask[0] & MASK != 0
        && (event.damage > 0.0 || event.heal.is_some())
    {
        roll_divine_inspiration(world, event.actor);
    }
}

fn roll_divine_inspiration(world: &mut World, caster: Entity) {
    if !world.get::<TraitLoadout>(caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|talent| talent.spell_id == 432964)
    }) {
        return;
    }
    let Some(id) = world
        .get::<combat::auras::ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == 432964 && aura.caster == caster)
                .map(|aura| aura.instance_id)
        })
    else {
        return;
    };
    // SimC76f8118 sc_paladin.cpp4116–4147: .55 RPPM, haste and bad-luck scaling.
    if !combat::auras::procs::try_source_hasted_rppm(world, caster, id, 0.55) {
        return;
    }
    #[cfg(any(test, feature = "test-support"))]
    {
        world
            .entity_mut(caster)
            .entry::<combat::test_support::DivineInspirationProcs>()
            .or_default()
            .get_mut()
            .0 += 1;
    }
    let next = world
        .get::<NextInspirationArmament>(caster)
        .map(|value| value.0)
        .unwrap_or_else(|| {
            if combat::random::random::<f32>() < 0.5 {
                432496
            } else {
                432502
            }
        });
    combat::auras::apply_aura(
        world,
        &AuraApplication {
            caster,
            target: caster,
            spell_id: next,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
    world
        .entity_mut(caster)
        .insert(NextInspirationArmament(if next == 432496 {
            432502
        } else {
            432496
        }));
}

fn rite_of_sanctification_handler(
    spell: u32,
    index: u8,
) -> Option<combat::spell_cast::effects::EffectHandler> {
    (spell == 433568 && index == 0).then_some(apply_rite_of_sanctification)
}

fn apply_rite_of_sanctification(
    world: &mut World,
    ctx: &combat::spell_cast::effects::EffectCtx<'_>,
    effect: &combat::spell_info::SpellEffect,
    _: Entity,
) {
    // SimC76f8118 sc_paladin.cpp:1999–2020; DB2 effect54 selects enchant7143.
    let Some(mut items) = world.get_mut::<EquippedItemModifiers>(ctx.caster) else {
        warn!("Rite of Sanctification requires equipped item modifiers");
        return;
    };
    if items
        .temporary_enchants
        .iter()
        .any(|(slot, _)| *slot == EquipmentSlot::MainHand)
    {
        warn!("Rite of Sanctification cannot replace a temporary weapon enchant");
        return;
    }
    items
        .temporary_enchants
        .push((EquipmentSlot::MainHand, effect.misc_values[0] as u32));
}

fn armament_energize_handler(
    spell: u32,
    index: u8,
) -> Option<combat::spell_cast::effects::EffectHandler> {
    (matches!(spell, 432459 | 432472) && index == 1).then_some(energize_holy_armament)
}

fn energize_holy_armament(
    world: &mut World,
    ctx: &combat::spell_cast::effects::EffectCtx<'_>,
    effect: &combat::spell_info::SpellEffect,
    target: Entity,
) {
    // SimC76f8118 sc_paladin.cpp2623–2625: Protection gains no Holy Power here.
    if world
        .get::<ActiveSpec>(ctx.caster)
        .is_some_and(|spec| spec.0 == 66)
    {
        return;
    }
    let amount =
        combat::spell_cast::effects::modded_base_points(world, ctx.caster, ctx.spell, effect);
    combat::spell_cast::effects::energize_unit(
        world,
        ctx,
        target,
        PowerType::HolyPower,
        amount,
        false,
    );
}

fn selected(world: &World, caster: Entity, talent: u32) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 66)
        && world.get::<TraitLoadout>(caster).is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|entry| entry.spell_id == talent)
        })
}

fn focused_enmity_multiplier(world: &World, caster: Entity, _: Entity, spell: &SpellInfo) -> f32 {
    if spell.id != 31935
        || !selected(world, caster, 378845)
        || combat::combat_refs::active_opponents(world, caster)
            .take(2)
            .count()
            != 1
    {
        return 1.0;
    }
    // SimC76f8118 sc_paladin_protection.cpp: count encounter enemies, not chain hits.
    1.0 + combat::spell_cast::scripts::script_effect_value(world, caster, 378845, 0) / 100.0
}

fn ferren_marcus_multiplier(
    world: &World,
    caster: Entity,
    target: Entity,
    spell: &SpellInfo,
) -> f32 {
    if spell.id != 31935
        || spell.cast_primary_target != Some(target)
        || !selected(world, caster, 378762)
    {
        return 1.0;
    }
    // SimC76f8118 sc_paladin_protection.cpp:273–280 applies the selected rank to the primary hit.
    1.0 + combat::spell_cast::scripts::script_effect_value(world, caster, 378762, 0) / 100.0
}

fn valor_shield_multiplier(
    world: &World,
    caster: Entity,
    target: Entity,
    spell: &SpellInfo,
) -> f32 {
    if spell.id != 53600 || spell.cast_primary_target != Some(target) {
        return 1.0;
    }
    let stacks = world
        .get::<combat::auras::ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == 1269179 && aura.caster == caster)
        })
        .map_or(0, |aura| aura.stacks);
    if stacks == 0 {
        return 1.0;
    }
    1.0 + f32::from(stacks)
        * combat::spell_cast::scripts::script_effect_value(world, caster, 1269179, 0)
        / 100.0
}

fn blaze_area_filter(
    spell: u32,
    index: u8,
) -> Option<combat::spell_cast::script_registry::AreaTargetFilter> {
    (spell == 1269224 && index == 0).then_some(filter_blaze_targets)
}

fn filter_blaze_targets(
    world: &World,
    _spell: &SpellInfo,
    primary: Option<Entity>,
    targets: &mut Vec<Entity>,
) {
    targets.retain(|target| Some(*target) != primary);
    let cap = world
        .resource::<SpellStore>()
        .get(1267215)
        .and_then(|source| source.effects.iter().find(|effect| effect.index == 0))
        .expect("Glory apex Blaze target count")
        .base_points
        .max(0.0) as usize;
    targets.truncate(cap);
}

fn consume_valor(world: &mut World, ctx: &combat::spell_cast::effects::EffectCtx<'_>) {
    if ctx.spell.id != 53600 || !combat::auras::has_aura(world, ctx.caster, 1269179) {
        return;
    }
    // SimC76f8118 sc_paladin.cpp2955–2971: the action's primary damage
    // uses Valor, then the owned buff is spent and Blaze targets secondaries.
    if let Some(target) = ctx.primary_target() {
        combat::spell_cast::cast_triggered(world, ctx.caster, 1269224, Some(target), ctx.depth + 1);
    }
    combat::auras::remove_auras(world, ctx.caster, RemoveReason::Consumed, |aura| {
        aura.spell_id == 1269179 && aura.caster == ctx.caster
    });
}

fn roll_vanguard(world: &mut World, caster: Entity, spell: u32) {
    // SimC76f8118 sc_paladin.cpp1518–1521: Judgment rolls 20%.
    if spell == 275779 && selected(world, caster, 1267203) && combat::random::random::<f32>() < 0.2
    {
        combat::auras::apply_aura(
            world,
            &AuraApplication {
                caster,
                target: caster,
                spell_id: 1268810,
                effect_mask: 1,
                source: AuraSource::Cast,
            },
        );
    }
}

fn grant_bulwark_of_righteous_fury(
    world: &mut World,
    ctx: &combat::spell_cast::effects::EffectCtx<'_>,
    _: Entity,
    _: bool,
    amount: f32,
) {
    if ctx.spell.id != 31935 || amount <= 0.0 || !selected(world, ctx.caster, 386653) {
        return;
    }
    // SimC76f8118 sc_paladin_protection.cpp:228–229 grants a stack per Shield impact.
    combat::auras::apply_aura(
        world,
        &AuraApplication {
            caster: ctx.caster,
            target: ctx.caster,
            spell_id: 386652,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
}

fn consume_bulwark_of_righteous_fury(
    world: &mut World,
    ctx: &combat::spell_cast::effects::EffectCtx<'_>,
) {
    if ctx.spell.id == 53600 {
        // SimC76f8118 sc_paladin.cpp:3927 consumes the modifier after Shield's damage.
        combat::auras::remove_auras(world, ctx.caster, RemoveReason::Consumed, |aura| {
            aura.spell_id == 386652 && aura.caster == ctx.caster
        });
    }
}

fn glory_from_shield(
    world: &mut World,
    ctx: &combat::spell_cast::effects::EffectCtx<'_>,
    target: Entity,
    _: bool,
    amount: f32,
) {
    if ctx.spell.id != 31935
        || ctx.primary_target() != Some(target)
        || !selected(world, ctx.caster, 1267203)
    {
        return;
    }
    // SimC76f8118 sc_paladin_protection.cpp248–258: wings at apex 3
    // bypass the Vanguard requirement; otherwise consume its stack.
    let apex = selected(world, ctx.caster, 1267215)
        && [31884, 389539]
            .iter()
            .any(|id| combat::auras::has_aura(world, ctx.caster, *id));
    if !apex && !combat::auras::has_aura(world, ctx.caster, 1268810) {
        return;
    }
    if !apex {
        combat::auras::remove_auras(world, ctx.caster, RemoveReason::Consumed, |aura| {
            aura.spell_id == 1268810 && aura.caster == ctx.caster
        });
    }
    let percent = combat::spell_cast::scripts::script_effect_value(world, ctx.caster, 1267203, 0);
    combat::spell_cast::queued_triggers::schedule_amount_full_trigger(
        world,
        ctx,
        target,
        1269175,
        300,
        amount * percent / 100.0,
    );
}

fn glory_on_cast(world: &mut World, caster: Entity, spell: u32, depth: u8) {
    if spell != 1269175 {
        return;
    }
    // SimC76f8118 sc_paladin_protection.cpp128–137: the child grants power
    // and Valor when the corresponding apex talents are selected.
    if selected(world, caster, 1267211) {
        let amount = combat::spell_cast::scripts::script_effect_value(world, caster, 1267211, 1);
        let source = world
            .resource::<SpellStore>()
            .get(1269175)
            .expect("Glory child")
            .clone();
        let ctx = combat::spell_cast::effects::EffectCtx {
            caster,
            spell: &source,
            depth,
            dest: None,
            chain_multiplier: 1.0,
        };
        combat::spell_cast::effects::energize_unit(
            world,
            &ctx,
            caster,
            PowerType::HolyPower,
            amount,
            false,
        );
    }
    if selected(world, caster, 1267215) {
        combat::auras::apply_aura(
            world,
            &AuraApplication {
                caster,
                target: caster,
                spell_id: 1269179,
                effect_mask: 1,
                source: AuraSource::Cast,
            },
        );
    }
}

fn grand_crusader_avoidance_mask(spell: u32) -> Option<u32> {
    // SimC76f8118 sc_paladin.cpp4929–4932: Grand Crusader rolls on each avoided melee.
    (spell == 85043).then_some(
        combat::auras::procs::PROC_HIT_MISS
            | combat::auras::procs::PROC_HIT_DODGE
            | combat::auras::procs::PROC_HIT_PARRY,
    )
}

fn grand_crusader_is_avoidance(
    _: &World,
    _: &AuraInstance,
    event: &combat::auras::procs::AuraEvent,
) -> bool {
    event.hit_mask != 0
}

fn grand_crusader_from_avoidance(
    world: &mut World,
    caster: Entity,
    _: &AuraInstance,
    _: &combat::auras::procs::AuraEvent,
) {
    roll_grand_crusader(world, caster);
}

fn initial_bulwark(world: &mut World, target: Entity, aura: &AuraInstance) {
    if aura.spell_id == 432496 {
        apply_bulwark_absorb(world, target, aura.caster, 1, true);
    }
}

fn periodic_bulwark(world: &mut World, target: Entity, aura: &AuraInstance, index: u8) {
    if index == 1 {
        apply_bulwark_absorb(world, target, aura.caster, 3, false);
    }
}

fn apply_bulwark_absorb(
    world: &mut World,
    target: Entity,
    caster: Entity,
    index: u8,
    initial: bool,
) {
    // SimC76f8118 sc_paladin.hpp:990–999, 1066–1072: percent/10 of maximum health;
    // versatility applies to the initial grant, not the periodic increment.
    let store = world.resource::<SpellStore>();
    let source = store.get(432496).expect("Holy Bulwark source spell");
    let percent = source
        .effects
        .iter()
        .find(|effect| effect.index == index)
        .expect("Holy Bulwark shield amount")
        .base_points;
    let mut shield = store
        .get(432607)
        .expect("Holy Bulwark absorb spell")
        .clone();
    let health = world
        .get::<Health>(target)
        .expect("living Holy Bulwark target")
        .max;
    let versatility = if initial {
        1.0 + world
            .get::<CharacterStats>(caster)
            .expect("Holy Bulwark caster stats")
            .versatility_pct
            / 100.0
    } else {
        1.0
    };
    shield
        .effects
        .iter_mut()
        .find(|effect| effect.index == 0)
        .expect("Holy Bulwark absorb effect")
        .base_points = health * percent / 1000.0 * versatility;
    combat::auras::apply_aura_with_spell(
        world,
        &AuraApplication {
            caster,
            target,
            spell_id: 432607,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
        &shield,
    );
}

fn advance_shining_light(world: &mut World, caster: Entity, cast: &SpellInfo) {
    if !world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 66)
    {
        return;
    }
    let spell = cast.id;
    if spell == 85673 {
        combat::auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
            aura.spell_id == 327510 && aura.caster == caster
        });
        if consumed_shining_light(cast) {
            apply_valiance(world, caster);
        }
        return;
    }
    if spell != 53600 {
        return;
    }
    // SimC76f8118 sc_paladin.cpp2940–2948: every third hardcast grants a free Word of Glory.
    let stacks = world
        .get::<combat::auras::ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == 182104 && aura.caster == caster)
        })
        .map_or(0, |aura| aura.stacks);
    let id = if stacks >= 2 {
        combat::auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
            aura.spell_id == 182104 && aura.caster == caster
        });
        327510
    } else {
        182104
    };
    let mask = if id == 182104 { 3 } else { 1 };
    combat::auras::apply_aura(
        world,
        &AuraApplication {
            caster,
            target: caster,
            spell_id: id,
            effect_mask: mask,
            source: AuraSource::Cast,
        },
    );
}

/// The cast's charge modifiers record the Shining Light aura that paid for it.
fn consumed_shining_light(cast: &SpellInfo) -> bool {
    cast.cast_charge_mods
        .iter()
        .flatten()
        .any(|modifier| modifier.aura_spell_id == 327510)
}

/// SimC76f8118 sc_paladin.cpp1263–1268: selected Valiance shortens Holy Armaments
/// whenever a Word of Glory executes with free Shining Light.
fn apply_valiance(world: &mut World, caster: Entity) {
    if !selected(world, caster, 432919) {
        return;
    }
    let milliseconds = combat::spell_cast::scripts::script_effect_value(world, caster, 432919, 0);
    combat::spell_cast::scripts::reduce_cooldown(
        world,
        caster,
        432459,
        f64::from(milliseconds) / 1000.0,
    );
}

fn owned_stacks(world: &World, caster: Entity, spell: u32) -> u8 {
    world
        .get::<combat::auras::ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == spell && aura.caster == caster)
        })
        .map_or(0, |aura| aura.stacks)
}

fn set_afterimage_stacks(world: &mut World, caster: Entity, stacks: u8) {
    if stacks == 0 {
        combat::auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
            aura.spell_id == 400745 && aura.caster == caster
        });
        return;
    }
    if owned_stacks(world, caster, 400745) == 0 {
        combat::auras::apply_aura(
            world,
            &AuraApplication {
                caster,
                target: caster,
                spell_id: 400745,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
    }
    combat::auras::set_self_cast_stacks(world, caster, 400745, stacks);
}

/// SimC76f8118 sc_paladin.hpp1545–1568,1747–1757: each foreground spender adds its
/// Holy Power cost to Afterimage (free spenders count their base cost). A Word of
/// Glory that starts at the threshold spends it and echoes onto the caster at the
/// talent's effectiveness; the echo is a Word of Glory and also triggers Valiance.
fn advance_afterimage(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !selected(world, caster, 385414) {
        return;
    }
    let Some(spent) = spell
        .power_costs
        .iter()
        .find(|cost| cost.power_type == PowerType::HolyPower as i32)
        .map(|cost| cost.mana_cost.clamp(0, i32::from(u8::MAX)) as u8)
    else {
        return;
    };
    let before = owned_stacks(world, caster, 400745);
    if spent > 0 {
        set_afterimage_stacks(world, caster, before.saturating_add(spent));
    }
    let threshold =
        combat::spell_cast::scripts::script_effect_value(world, caster, 385414, 2) as u8;
    if spell.id != 85673 || before < threshold {
        return;
    }
    let remaining = owned_stacks(world, caster, 400745).saturating_sub(threshold);
    set_afterimage_stacks(world, caster, remaining);
    let effectiveness =
        combat::spell_cast::scripts::script_effect_value(world, caster, 385414, 0) / 100.0;
    combat::spell_cast::cast_triggered_with_overrides(
        world,
        caster,
        85673,
        Some(caster),
        1,
        combat::spell_cast::TriggeredSpellOverrides {
            healing_multiplier: Some(effectiveness),
            ..Default::default()
        },
    );
    if consumed_shining_light(spell) {
        apply_valiance(world, caster);
    }
}

fn advance_divine_guidance(world: &mut World, caster: Entity, spell: u32) {
    if !world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 66)
        || !world.get::<TraitLoadout>(caster).is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|talent| talent.spell_id == 433106)
        })
    {
        return;
    }
    match spell {
        // SimC76f8118 sc_paladin.hpp1738–1740: successful Holy Power spenders grant a stack.
        53600 | 85673 => grant_divine_guidance(world, caster),
        // sc_paladin.cpp562–569: a hardcast consumes the accumulated stacks.
        26573 => consume_divine_guidance(world, caster),
        _ => {}
    }
}

fn grant_divine_guidance(world: &mut World, caster: Entity) {
    let mut buff = world
        .resource::<SpellStore>()
        .get(433106)
        .expect("Divine Guidance spell")
        .clone();
    // SimC76f8118 sc_paladin.cpp3718: this source buff has five stacks.
    buff.aura_options.get_or_insert_default().cumulative_aura = 5;
    combat::auras::apply_aura_with_spell(
        world,
        &AuraApplication {
            caster,
            target: caster,
            spell_id: 433106,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
        &buff,
    );
}

fn consume_divine_guidance(world: &mut World, caster: Entity) {
    let stacks = world
        .get::<combat::auras::ActiveAuras>(caster)
        .and_then(|auras| {
            auras.iter().find(|aura| {
                aura.spell_id == 433106 && aura.caster == caster && aura.source == AuraSource::Cast
            })
        })
        .map_or(0, |aura| aura.stacks);
    if stacks == 0 {
        return;
    }
    let store = world.resource::<SpellStore>();
    let talent = store.get(433106).expect("Divine Guidance talent");
    let coefficient = talent
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("Divine Guidance AP coefficient")
        .base_points
        / 10.0
        * f32::from(stacks);
    let target = world
        .get::<combat::networking::CombatTarget>(caster)
        .and_then(|target| target.0);
    if let Some(target) = target {
        cast_guidance_effect(world, caster, target, 433808, coefficient);
    }
    cast_guidance_effect(world, caster, caster, 433807, coefficient * 0.3);
    combat::auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
        aura.spell_id == 433106 && aura.caster == caster && aura.source == AuraSource::Cast
    });
}

fn cast_guidance_effect(
    world: &mut World,
    caster: Entity,
    target: Entity,
    id: u32,
    coefficient: f32,
) {
    let mut heal = world
        .resource::<SpellStore>()
        .get(id)
        .expect("Divine Guidance effect spell")
        .clone();
    heal.effects
        .iter_mut()
        .find(|effect| effect.index == 0)
        .expect("Divine Guidance effect")
        .bonus_coefficient_from_ap = coefficient;
    let amount = combat::spell_cast::effects::scaled_amount(world, caster, &heal, &heal.effects[0]);
    let cast = combat::spell_cast::cast_triggered_with_overrides(
        world,
        caster,
        id,
        Some(target),
        1,
        combat::spell_cast::TriggeredSpellOverrides {
            effect0_amount: Some(amount),
            ..Default::default()
        },
    );
    if !cast {
        warn!("Divine Guidance spell {id} could not be cast");
    }
}

fn extend_zealots_paragon(world: &mut World, caster: Entity, spell: u32) {
    if !matches!(spell, 275779 | 1241413)
        || !world
            .get::<ActiveSpec>(caster)
            .is_some_and(|spec| spec.0 == 66)
        || !world.get::<TraitLoadout>(caster).is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|talent| talent.spell_id == 391142)
        })
    {
        return;
    }
    // SimC76f8118 sc_paladin.cpp1414–1429: actual Judgment/HoW casts extend live wings.
    let duration = combat::spell_cast::scripts::script_effect_value(world, caster, 391142, 0);
    combat::auras::extend_owned_duration(world, caster, caster, &[31884, 389539], duration as u32);
}

fn trigger_grand_crusader(world: &mut World, caster: Entity, spell: u32) {
    if spell == 204019 {
        roll_grand_crusader(world, caster);
    }
}

fn roll_grand_crusader(world: &mut World, caster: Entity) {
    if !world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 66)
        || !world.get::<TraitLoadout>(caster).is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|talent| talent.spell_id == 85043)
        })
    {
        return;
    }
    // SimC76f8118 sc_paladin_protection.cpp448–452,903–923: eligible
    // events roll the selected native Grand Crusader chance.
    let chance = combat::spell_cast::scripts::script_effect_value(world, caster, 85043, 0);
    if combat::random::random::<f32>() * 100.0 >= chance {
        return;
    }
    reset_avengers_shield(world, caster);
}

fn reflection_from_absorb(
    world: &mut World,
    caster: Entity,
    victim: Entity,
    _attacker: Option<Entity>,
    spell: u32,
    amount: f32,
) {
    if caster != victim
        || spell != 432607
        || amount <= 0.0
        || !world
            .get::<ActiveSpec>(caster)
            .is_some_and(|spec| spec.0 == 66)
        || !world.get::<TraitLoadout>(caster).is_some_and(|traits| {
            let mut talents = traits.active_talent_spells();
            talents.any(|talent| talent.spell_id == 1271466)
                && traits
                    .active_talent_spells()
                    .any(|talent| talent.spell_id == 85043)
        })
    {
        return;
    }
    // SimC76f8118 sc_paladin.hpp777–780,1002–1012: a self Bulwark absorb
    // rolls the 20% solo chance; fake allied Solidarity stacks are not present.
    if combat::random::random::<f32>() < 0.2 {
        reset_avengers_shield(world, caster);
    }
}

fn reset_avengers_shield(world: &mut World, caster: Entity) {
    let shield = world
        .resource::<SpellStore>()
        .get(31935)
        .expect("Avenger's Shield")
        .clone();
    let now = combat::spell_cast::now(world);
    let remaining = world
        .get::<combat::spell_cast::SpellHistory>(caster)
        .map_or(0, |history| history.remaining_cooldown_ms(&shield, now));
    combat::spell_cast::scripts::reduce_cooldown(world, caster, 31935, remaining as f64 / 1000.0);
    // SimC76f8118 sc_paladin_protection.cpp946–948: every successful Grand
    // Crusader also advances the shared Judgment/Hammer of Wrath charge.
    if selected(world, caster, 204023) {
        let milliseconds =
            combat::spell_cast::scripts::script_effect_value(world, caster, 204023, 1);
        combat::spell_cast::scripts::reduce_cooldown(
            world,
            caster,
            275779,
            f64::from(milliseconds) / 1000.0,
        );
    }
}

fn after_cast(world: &mut World, caster: Entity, spell: &SpellInfo) {
    // SimC76f8118 sc_paladin.hpp:1730–1735: Protection SotR grants selected Faith's Armor.
    if spell.id == 53600 && selected(world, caster, 406101) {
        combat::auras::apply_aura(
            world,
            &AuraApplication {
                caster,
                target: caster,
                spell_id: 379017,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
    }
    roll_vanguard(world, caster, spell.id);
    if spell.id == 204019
        && spell.cast_primary_target.is_some_and(|target| {
            combat::spell_cast::targets::relation_matches(
                world,
                caster,
                target,
                combat::spell_cast::targets::Relation::Enemy,
            )
        })
    {
        // SimC76f8118 treats the zero-damage hit on the action target as the proc source.
        roll_divine_inspiration(world, caster);
    }
    trigger_grand_crusader(world, caster, spell.id);
    extend_zealots_paragon(world, caster, spell.id);
    advance_shining_light(world, caster, spell);
    advance_divine_guidance(world, caster, spell.id);
    // Native70448 spell432478 effect0 is aura332: misc432459 -> amount432472.
    // SimC76f8118 sc_paladin.cpp2748–2752 alternates only on actual hardcasts.
    let Some(override_spell) = world.resource::<SpellStore>().get(432478) else {
        if matches!(spell.id, 432459 | 432472) {
            warn!("Holy Armaments override aura432478 is missing");
        }
        return;
    };
    let Some(effect) = override_spell
        .effects
        .iter()
        .find(|effect| effect.aura == 332)
    else {
        if matches!(spell.id, 432459 | 432472) {
            warn!("Holy Armaments override aura432478 lacks aura332");
        }
        return;
    };
    let override_id = override_spell.id;
    let base = effect.misc_values[0] as u32;
    let replacement = effect.base_points as u32;
    if spell.id == base {
        combat::auras::apply_aura(
            world,
            &AuraApplication {
                caster,
                target: caster,
                spell_id: override_id,
                effect_mask: 1,
                source: AuraSource::Cast,
            },
        );
    } else if spell.id == replacement {
        combat::auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
            aura.spell_id == override_id && aura.caster == caster
        });
    }
}
