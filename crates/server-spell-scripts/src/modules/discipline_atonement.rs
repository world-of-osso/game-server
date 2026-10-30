//! Discipline's native Atonement marks and damage-to-healing transfer.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, AuraApplication, AuraInstance, AuraSource};
use combat::spell_cast::{
    effects::{EffectCtx, apply_critical_transfer_heal},
    script_registry::SpellScriptHook,
};
use combat::spell_info::{SpellInfo, SpellStore};
use shared::components::{ActiveSpec, Health, Player};
use shared::protocol::{CombatLogEvent, CombatLogKind};

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::DeclaredReach(declared_reach),
    SpellScriptHook::AfterAuraApplication(apply_mark),
    SpellScriptHook::OnCast(start_penance),
    SpellScriptHook::LateAuraRegistration(register_shield_discipline),
    SpellScriptHook::NativeAfterCast(master_the_darkness_action),
    SpellScriptHook::AfterAbsorbConsumed(discharge_void_shield),
    SpellScriptHook::HealthChangeObserved(mark_flash_heal),
    SpellScriptHook::SpellHealthImpact(trigger_twist_of_fate),
    SpellScriptHook::ActualDamageMultiplier(twist_damage_multiplier),
    SpellScriptHook::HealingMultiplier(twist_healing_multiplier),
    SpellScriptHook::ResolvedHeal(divine_aegis),
    SpellScriptHook::RawResolvedDamage(transfer_damage),
    SpellScriptHook::RawResolvedDamage(trigger_dark_indulgence),
    SpellScriptHook::DamageAmount(penance_bolt_damage),
    SpellScriptHook::HealingMultiplier(penance_healing_multiplier),
    SpellScriptHook::EffectHit(shadowfiend_death_hit),
    SpellScriptHook::MeleeRawResolved(transfer_pet_melee),
    SpellScriptHook::MeleeResidual(fiend_mana_on_landed_melee),
    SpellScriptHook::SummonInitialized(initialize_fiend),
    SpellScriptHook::AfterNativeCastUpdate(update_voidwraith),
    SpellScriptHook::ConsumedEffectValues(&[(81749, 0)]),
];

fn declared_reach(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
) -> combat::spell_cast::script_registry::ScriptReach {
    use combat::spell_cast::script_registry::{DeclaredReach, ScriptReach, reach_source_selected};
    if !matches!(spell.id, 1253593 | 1253827) || !reach_source_selected(world, caster, 1253827) {
        return ScriptReach::None;
    }
    let discharge = world
        .resource::<SpellStore>()
        .get(1253828)
        .expect("native Void Shield discharge");
    let range = combat::spell_cast::targets::spell_range(discharge, true).1;
    ScriptReach::EngagedArea(DeclaredReach {
        caster_radius: range,
        target_radius: 0.0,
    })
}

#[cfg(test)]
#[test]
fn void_shield_reach_uses_native_discharge_range_and_only_engaged_targets() {
    use combat::spell_cast::script_registry::{DeclaredReach, ScriptReach};
    let store = crate::test_support::world_db()
        .unwrap()
        .spell_store()
        .clone();
    let shield = store.get(1253593).unwrap().clone();
    let discharge_range =
        combat::spell_cast::targets::spell_range(store.get(1253828).unwrap(), true).1;
    let mut world = World::new();
    world.insert_resource(store);
    let caster = world
        .spawn(combat::class_progression::KnownSpells::from_baseline(vec![
            1253827,
        ]))
        .id();
    assert_eq!(
        declared_reach(&world, caster, &shield),
        ScriptReach::EngagedArea(DeclaredReach {
            caster_radius: discharge_range,
            target_radius: 0.0
        })
    );
    assert_eq!(discharge_range, 50_000.0);
}

fn trigger_dark_indulgence(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    _target: Entity,
    _crit: bool,
    _amount: f32,
) {
    if ctx.spell.id != 8092
        || !world
            .get::<combat::class_progression::KnownSpells>(ctx.caster)
            .is_some_and(|known| known.contains(372972))
    {
        return;
    }
    // SimC76f8118 sc_priest.cpp:273–277 grants Power of the Dark Side on Mind Blast impact.
    auras::apply_aura(
        world,
        &AuraApplication {
            caster: ctx.caster,
            target: ctx.caster,
            spell_id: 198069,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
}

fn is_priest_spell(spell: &SpellInfo) -> bool {
    spell
        .class_options
        .as_ref()
        .is_some_and(|options| options.spell_class_set == 6)
}

fn trigger_twist_of_fate(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    target: Entity,
    before: f32,
    _healing: bool,
) {
    let caster = ctx.caster;
    if !is_priest_spell(ctx.spell)
        || !world
            .get::<combat::class_progression::KnownSpells>(caster)
            .is_some_and(|known| known.contains(390972))
    {
        return;
    }
    let Some(health) = world.get::<Health>(target) else {
        return;
    };
    let threshold = combat::spell_cast::scripts::script_effect_value(world, caster, 390972, 2);
    if before * 100.0 >= health.max * threshold {
        return;
    }
    auras::apply_aura(
        world,
        &AuraApplication {
            caster,
            target: caster,
            spell_id: 390978,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
}

fn twist_multiplier(world: &World, caster: Entity, spell: &SpellInfo) -> f32 {
    if is_priest_spell(spell) && auras::has_aura(world, caster, 390978) {
        1.0 + combat::spell_cast::scripts::script_effect_value(world, caster, 390972, 0) / 100.0
    } else {
        1.0
    }
}

fn twist_damage_multiplier(world: &mut World, ctx: &EffectCtx<'_>, _target: Entity) -> f32 {
    twist_multiplier(world, ctx.caster, ctx.spell)
}

fn twist_healing_multiplier(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    _periodic: bool,
) -> f32 {
    twist_multiplier(world, caster, spell)
}

#[derive(Component)]
struct PenanceBolts {
    completed: u32,
    dark_side_multiplier: f32,
}

fn start_penance(world: &mut World, caster: Entity, spell: u32, _depth: u8) {
    if spell != 47540 || !is_discipline(world, caster) {
        return;
    }
    // SimC76f8118 sc_priest_discipline.cpp:579-582 resets Holy Ray at channel start.
    auras::remove_auras(world, caster, auras::RemoveReason::Consumed, |aura| {
        aura.spell_id == 1235193 && aura.caster == caster
    });
    let dark_side_multiplier = if auras::has_aura(world, caster, 198069) {
        1.0 + combat::spell_cast::scripts::script_effect_value(world, caster, 198069, 0) / 100.0
    } else {
        1.0
    };
    auras::remove_auras(world, caster, auras::RemoveReason::Consumed, |aura| {
        aura.spell_id == 198069 && aura.caster == caster
    });
    world.entity_mut(caster).insert(PenanceBolts {
        completed: 0,
        dark_side_multiplier,
    });
}

fn register_shield_discipline(scripts: &mut auras::scripts::AuraScripts) {
    for spell in [17, 1253593] {
        scripts.0.entry(spell).or_default().on_remove = Some(shield_discipline);
    }
}

fn shield_discipline(
    world: &mut World,
    _target: Entity,
    aura: &AuraInstance,
    _reason: auras::RemoveReason,
) {
    let caster = aura.caster;
    // SimC76f8118 sc_priest.cpp:50-61,114-125: only an early shield expiration returns mana.
    if !auras::has_aura(world, caster, 197045)
        || !aura
            .expires_at
            .is_some_and(|expiry| expiry > combat::spell_cast::now(world))
    {
        return;
    }
    let percent = combat::spell_cast::scripts::script_effect_value(world, caster, 47755, 0) / 100.0;
    combat::spell_cast::cast_triggered_with_effect0_amount(
        world,
        caster,
        47755,
        Some(caster),
        1,
        percent,
    );
}

#[derive(Component)]
struct VoidwraithAttack {
    target: Entity,
    next: f64,
}

fn initialize_fiend(world: &mut World, actor: Entity) {
    let Some(summon) = world.get::<combat::spell_cast::summon::SummonedCreature>(actor) else {
        return;
    };
    let (owner, spell) = (summon.owner, summon.spell_id);
    #[cfg(any(test, feature = "test-support"))]
    let expires = summon.expires_at;
    if !is_discipline(world, owner) || !matches!(spell, 1280172 | 123040 | 200174 | 451235) {
        return;
    }
    #[cfg(any(test, feature = "test-support"))]
    {
        let started = combat::spell_cast::now(world);
        let owner_spell_power = world
            .get::<combat::stats::CharacterStats>(owner)
            .expect("fiend owner stats")
            .spell_power;
        if let Some(mut receipts) =
            world.get_resource_mut::<combat::spell_cast::test_harness::FiendAtonementReceipts>()
        {
            receipts
                .1
                .push(combat::spell_cast::test_harness::FiendSummonReceipt {
                    actor,
                    owner,
                    spell,
                    started,
                    expires,
                    owner_spell_power,
                    landed_raw: Vec::new(),
                });
        }
    }
    let Some(target) = world
        .get::<combat::networking::CombatTarget>(owner)
        .and_then(|target| target.0)
    else {
        warn!(?owner, spell, "Discipline summon has no selected enemy");
        return;
    };
    if spell == 451235 {
        let now = combat::spell_cast::now(world);
        world
            .entity_mut(actor)
            .insert(VoidwraithAttack { target, next: now });
    } else {
        combat::spell_cast::summon::start_melee_attack(world, actor, target);
    }
}

fn update_voidwraith(world: &mut World) {
    let now = combat::spell_cast::now(world);
    let due: Vec<_> = world
        .query::<(
            Entity,
            &VoidwraithAttack,
            &combat::spell_cast::summon::SummonedCreature,
        )>()
        .iter(world)
        .filter(|(_, state, _)| state.next <= now)
        .map(|(actor, state, summon)| (actor, state.target, summon.owner))
        .collect();
    for (actor, target, owner) in due {
        if !combat::spell_cast::targets::is_alive(world, target) {
            continue;
        }
        let haste = world
            .get::<combat::stats::CharacterStats>(owner)
            .map_or(1.0, |stats| 1.0 + stats.haste_pct / 100.0);
        world
            .get_mut::<VoidwraithAttack>(actor)
            .expect("live Voidwraith")
            .next = now + 1.5 / f64::from(haste.max(0.01));
        // SimC76f8118 sc_priest_pets.cpp:500-555: Voidwraith casts Void Flay.
        if !combat::spell_cast::try_cast_triggered(world, actor, 451435, Some(target), 1) {
            warn!(?actor, ?target, "Voidwraith Void Flay could not be cast");
        }
    }
}

#[derive(Component)]
struct DarknessDeck {
    cards: u8,
    success: bool,
}

fn master_the_darkness_action(spell: u32) -> Option<combat::spell_cast::scripts::AfterCastHook> {
    match spell {
        472433 => Some(after_evangelism),
        47540 | 8092 | 1253593 => Some(after_darkness_action),
        _ => None,
    }
}

fn after_evangelism(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !is_discipline(world, caster) {
        return;
    }
    let Some(target) = spell.cast_primary_target else {
        return;
    };
    let effectiveness =
        combat::spell_cast::scripts::script_effect_value(world, caster, 472433, 1) / 100.0;
    let mut radiance = world
        .resource::<SpellStore>()
        .get(194509)
        .expect("Evangelism requires Radiance")
        .clone();
    for effect in &mut radiance.effects {
        if effect.effect == 10 {
            effect.base_points *= effectiveness;
            effect.bonus_coefficient *= effectiveness;
            effect.bonus_coefficient_from_ap *= effectiveness;
        }
    }
    // SimC76f8118 sc_priest_discipline.cpp:164-194: Evangelism triggers scaled Radiance.
    combat::spell_cast::launch_triggered_spell(world, caster, &radiance, Some(target), 1);
}

fn after_darkness_action(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !is_discipline(world, caster) {
        return;
    }
    if spell.id == 1253593 {
        auras::remove_auras(world, caster, auras::RemoveReason::Consumed, |aura| {
            aura.spell_id == 1253591 && aura.caster == caster
        });
        return;
    }
    let proc = if spell.id == 8092 {
        auras::has_aura(world, caster, 1253827)
    } else if auras::has_aura(world, caster, 1253590) {
        draw_darkness_card(world, caster)
    } else {
        false
    };
    if proc {
        // SimC76f8118 sc_priest_discipline.cpp:588-594,793-796; sc_priest.cpp:216-218.
        auras::apply_aura(
            world,
            &AuraApplication {
                caster,
                target: caster,
                spell_id: 1253591,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
    }
}

fn draw_darkness_card(world: &mut World, caster: Entity) -> bool {
    if world
        .get::<DarknessDeck>(caster)
        .is_none_or(|deck| deck.cards == 0)
    {
        let chance = combat::spell_cast::scripts::script_effect_value(world, caster, 1253590, 0);
        assert!(
            chance > 0.0 && chance <= 100.0,
            "Master the Darkness requires a native proc chance"
        );
        let cards = (100.0 / chance).round() as u8;
        world.entity_mut(caster).insert(DarknessDeck {
            cards,
            success: true,
        });
    }
    let mut deck = world.get_mut::<DarknessDeck>(caster).unwrap();
    let drawn = deck.success && combat::random::random::<f32>() < 1.0 / f32::from(deck.cards);
    deck.cards -= 1;
    if drawn {
        deck.success = false;
    }
    drawn
}

fn discharge_void_shield(
    world: &mut World,
    caster: Entity,
    _victim: Entity,
    attacker: Option<Entity>,
    spell: u32,
    amount: f32,
) {
    if spell != 1253593 || amount <= 0.0 || !auras::has_aura(world, caster, 1253827) {
        return;
    }
    let Some(target) = void_discharge_target(world, caster, attacker) else {
        return;
    };
    let percent = combat::spell_cast::scripts::script_effect_value(world, caster, 1253827, 0);
    // SimC76f8118 sc_priest.cpp:84-105: each actual absorbed contribution discharges.
    combat::spell_cast::cast_triggered_with_effect0_amount(
        world,
        caster,
        1253828,
        Some(target),
        1,
        amount * percent / 100.0,
    );
}

fn void_discharge_target(
    world: &mut World,
    caster: Entity,
    attacker: Option<Entity>,
) -> Option<Entity> {
    use combat::spell_cast::targets::{self, Relation};
    use shared::components::CombatStatus;
    let spell = world
        .resource::<SpellStore>()
        .get(1253828)
        .expect("native Void Shield discharge");
    let range = targets::spell_range(spell, true).1;
    let origin = targets::position(world, caster)?;
    let valid = |world: &World, target: Entity| {
        targets::is_alive(world, target)
            && targets::same_map(world, caster, target)
            && targets::relation_matches(world, caster, target, Relation::Enemy)
            && targets::position(world, target)
                .is_some_and(|position| origin.distance(position) <= range)
    };
    if let Some(attacker) = attacker.filter(|&target| valid(world, target)) {
        return Some(attacker);
    }
    // SimC76f8118 sc_priest.cpp:86-102 chooses only a non-sleeping enemy
    // when the attacker is absent or invalid. CombatStatus represents that
    // active-enemy boundary in the server; native1253828 supplies the range.
    let enemies: Vec<_> = world
        .query::<(Entity, &CombatStatus)>()
        .iter(world)
        .filter(|(target, status)| status.0 && valid(world, *target))
        .map(|(target, _)| target)
        .collect();
    if enemies.is_empty() {
        return None;
    }
    Some(enemies[(combat::random::random::<f32>() * enemies.len() as f32) as usize])
}

fn apply_mark(world: &mut World, target: Entity, aura: &AuraInstance) {
    if !matches!(aura.spell_id, 17 | 139 | 1253593) || !is_discipline(world, aura.caster) {
        return;
    }
    // SimC76f8118 sc_priest.cpp:2003-2011: Shield applies Atonement on impact.
    // Renew also applies the normal-duration mark for Discipline.
    auras::apply_aura(
        world,
        &AuraApplication {
            caster: aura.caster,
            target,
            spell_id: 194384,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
}

fn mark_flash_heal(world: &mut World, event: &CombatLogEvent) {
    if event.kind != CombatLogKind::Heal || !matches!(event.spell_id, Some(2061 | 186263)) {
        return;
    }
    let (Some(caster), Some(target)) = (
        event.source.and_then(Entity::try_from_bits),
        event.target.and_then(Entity::try_from_bits),
    ) else {
        return;
    };
    if is_discipline(world, caster) {
        // Flash Heal applies Atonement in SimC; WCL VxFtGRafyAbKmnd9 fight 10
        // shows the selected Shadow Mend override applies it on the same recipient.
        auras::apply_aura(
            world,
            &AuraApplication {
                caster,
                target,
                spell_id: 194384,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
    }
}

fn is_discipline(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 256)
}

fn transfer_damage(
    world: &mut World,
    damage: &EffectCtx<'_>,
    target: Entity,
    crit: bool,
    raw: f32,
) {
    // SimC 76f8118 priest_spell_t sets triggers_atonement on these damage actions.
    const TRIGGERS: &[u32] = &[
        585, 589, 8092, 32379, 47666, 373130, 390832, 120696, 390964, 132157, 450215, 447448,
        448405, 1280134, 1253828, 421543,
    ];
    let source = damage.caster;
    let (caster, pet_spell) =
        if let Some(summon) = world.get::<combat::spell_cast::summon::SummonedCreature>(source) {
            if !matches!(summon.spell_id, 1280172 | 123040 | 200174 | 451235)
                || !matches!(damage.spell.id, 451435 | 373442)
            {
                return;
            }
            (summon.owner, true)
        } else {
            if !TRIGGERS.contains(&damage.spell.id) {
                return;
            }
            (source, false)
        };
    transfer_atonement(
        world,
        caster,
        target,
        crit,
        raw,
        damage.spell.school_mask(),
        pet_spell,
        false,
    );
    if damage.spell.id == 47666 && is_discipline(world, caster) {
        extend_pain_from_penance(world, caster, target);
        finish_penance_bolt(world, caster);
    }
}

fn extend_pain_from_penance(world: &mut World, caster: Entity, target: Entity) {
    if !world
        .get::<combat::class_progression::KnownSpells>(caster)
        .is_some_and(|known| known.contains(390686))
    {
        return;
    }
    // SimC76f8118 sc_priest_discipline.cpp:291-295 extends only the caster's Pain on impact.
    let milliseconds =
        combat::spell_cast::scripts::script_effect_value(world, caster, 390686, 0).max(0.0) as u32;
    auras::extend_owned_duration(world, caster, target, &[589], milliseconds);
}

fn penance_bolt_damage(
    spell: u32,
) -> Option<combat::spell_cast::script_registry::DamageAmountHook> {
    (spell == 47666).then_some(apply_penance_bolt_modifiers)
}

fn apply_penance_bolt_modifiers(
    world: &World,
    caster: Entity,
    _target: Entity,
    amount: f32,
) -> f32 {
    let Some(bolts) = world.get::<PenanceBolts>(caster) else {
        return amount;
    };
    let prompt = bolts.completed == 2
        && world
            .get::<combat::class_progression::KnownSpells>(caster)
            .is_some_and(|known| known.contains(1246799));
    // SimC76f8118 sc_priest_discipline.cpp:445-448,490-494 snapshots Dark Side and boosts tick 2.
    let bonus = if prompt {
        combat::spell_cast::scripts::script_effect_value(world, caster, 1246799, 0)
    } else {
        0.0
    };
    amount * bolts.dark_side_multiplier * (1.0 + bonus / 100.0)
}

fn penance_healing_multiplier(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    _periodic: bool,
) -> f32 {
    if spell.id != 47750 {
        return 1.0;
    }
    world
        .get::<PenanceBolts>(caster)
        .map_or(1.0, |bolts| bolts.dark_side_multiplier)
}

fn select_twinsight_recipient(world: &mut World, caster: Entity) -> Option<Entity> {
    let map = *world.get::<combat::maps::InMap>(caster)?;
    let candidates: Vec<_> = world
        .query::<(
            Entity,
            &Health,
            &combat::maps::InMap,
            Option<&Player>,
            Option<&combat::followers::Follower>,
        )>()
        .iter(world)
        .filter(|(_, health, unit_map, player, follower)| {
            health.current > 0.0 && **unit_map == map && (player.is_some() || follower.is_some())
        })
        .map(|(unit, _, _, _, _)| unit)
        .filter(|&unit| {
            combat::spell_cast::targets::relation_matches(
                world,
                caster,
                unit,
                combat::spell_cast::targets::Relation::Ally,
            )
        })
        .collect();
    if candidates.is_empty() {
        return None;
    }
    Some(candidates[(combat::random::random::<f32>() * candidates.len() as f32) as usize])
}

fn finish_penance_bolt(world: &mut World, caster: Entity) {
    let Some(mut bolts) = world.get_mut::<PenanceBolts>(caster) else {
        return;
    };
    bolts.completed += 1;
    let bolt = bolts.completed;
    let selected = |world: &World, talent| {
        world
            .get::<combat::class_progression::KnownSpells>(caster)
            .is_some_and(|known| known.contains(talent))
    };
    let twinsight = bolt <= 4 && selected(world, 440742);
    let mut stacks = 1;
    if twinsight && let Some(target) = select_twinsight_recipient(world, caster) {
        // SimC76f8118 sc_priest_discipline.cpp:463-484 schedules one healing and one damage bolt.
        let mut spell = world
            .resource::<SpellStore>()
            .get(47750)
            .expect("Twinsight Penance heal must exist")
            .clone();
        for effect in &mut spell.effects {
            if effect.implicit_targets[0] == 77 {
                effect.implicit_targets[0] = 21;
            }
        }
        if combat::spell_cast::launch_triggered_spell(world, caster, &spell, Some(target), 1) {
            stacks += 1;
        }
    }
    if selected(world, 372969) {
        // SimC76f8118 sc_priest_discipline.cpp:304-310,350-358: each bolt advances Holy Ray.
        for _ in 0..stacks {
            auras::apply_aura(
                world,
                &AuraApplication {
                    caster,
                    target: caster,
                    spell_id: 1235193,
                    effect_mask: u32::MAX,
                    source: AuraSource::Cast,
                },
            );
        }
    }
}

fn shadowfiend_death_hit(
    spell: u32,
    index: u8,
) -> Option<combat::spell_cast::script_registry::HitHook> {
    (spell == 32379 && index == 1).then_some(proc_shadowfiend)
}

fn proc_shadowfiend(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    if !is_discipline(world, caster)
        || !world
            .get::<combat::class_progression::KnownSpells>(caster)
            .is_some_and(|known| known.contains(34433))
    {
        return;
    }
    let threshold = combat::spell_cast::scripts::script_effect_value(world, caster, 32379, 2);
    let Some(health) = world.get::<shared::components::Health>(target) else {
        return;
    };
    if health.max <= 0.0 || health.current * 100.0 > health.max * threshold {
        return;
    }
    // Native Discipline passive 137032 adds 60 points to Shadowfiend's effect 3.
    let chance = combat::spell_cast::scripts::script_effect_value(world, caster, 34433, 2);
    let roll = combat::random::random::<f32>() * 100.0;
    #[cfg(any(test, feature = "test-support"))]
    {
        let now = combat::spell_cast::now(world);
        if let Some(mut receipts) =
            world.get_resource_mut::<combat::spell_cast::test_harness::FiendAtonementReceipts>()
        {
            receipts.2.push((caster, now, roll, chance));
        }
    }
    if roll < chance {
        combat::spell_cast::cast_triggered(world, caster, 1280172, Some(target), depth + 1);
    }
}

fn divine_aegis(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    target: Entity,
    _raw: f32,
    effective: f32,
    crit: bool,
    _periodic: bool,
) {
    // SimC76f8118 priest_heal_t triggers on critical direct priest heals, except Atonement.
    if !is_discipline(world, ctx.caster)
        || !crit
        || effective <= 0.0
        || !auras::has_aura(world, ctx.caster, 47515)
        || !matches!(
            ctx.spell.id,
            2061 | 186263 | 47750 | 1232567 | 281469 | 270501 | 194509 | 421544
        )
    {
        return;
    }
    let percent = combat::spell_cast::scripts::script_effect_value(world, ctx.caster, 47515, 0);
    combat::spell_cast::cast_triggered_with_effect0_amount(
        world,
        ctx.caster,
        47753,
        Some(target),
        ctx.depth + 1,
        effective * percent / 100.0,
    );
}

fn fiend_mana_on_landed_melee(world: &mut World, source: Entity, _target: Entity, _raw: f32) {
    let Some(summon) = world.get::<combat::spell_cast::summon::SummonedCreature>(source) else {
        return;
    };
    let (owner, summon_spell) = (summon.owner, summon.spell_id);
    if !is_discipline(world, owner) {
        return;
    }
    let Some(known) = world.get::<combat::class_progression::KnownSpells>(owner) else {
        return;
    };
    // SimC76f8118 sc_priest_pets.cpp:587-588,627-628,745-762: landed fiend swings return mana.
    let (spell, divisor) = match summon_spell {
        1280172 if known.contains(34433) => (343727, 10.0),
        123040 | 200174 if known.contains(123040) || known.contains(200174) => (123051, 100.0),
        _ => return,
    };
    let percent =
        combat::spell_cast::scripts::script_effect_value(world, owner, spell, 0) / divisor;
    combat::spell_cast::cast_triggered_with_effect0_amount(
        world,
        source,
        spell,
        Some(owner),
        1,
        percent,
    );
}

fn transfer_pet_melee(world: &mut World, source: Entity, target: Entity, raw: f32, crit: bool) {
    let Some(summon) = world.get::<combat::spell_cast::summon::SummonedCreature>(source) else {
        return;
    };
    if !matches!(summon.spell_id, 1280172 | 123040 | 200174) {
        return;
    }
    let owner = summon.owner;
    #[cfg(any(test, feature = "test-support"))]
    if let Some(mut receipts) =
        world.get_resource_mut::<combat::spell_cast::test_harness::FiendAtonementReceipts>()
        && let Some(receipt) = receipts
            .1
            .iter_mut()
            .find(|receipt| receipt.actor == source)
    {
        receipt.landed_raw.push(raw);
    }
    transfer_atonement(world, owner, target, crit, raw, 1, false, true);
}

fn transfer_atonement(
    world: &mut World,
    caster: Entity,
    target: Entity,
    crit: bool,
    raw: f32,
    school: u32,
    pet_spell: bool,
    pet_melee: bool,
) {
    if caster == target || !is_discipline(world, caster) {
        return;
    }
    #[cfg(any(test, feature = "test-support"))]
    if (pet_melee || pet_spell)
        && let Some(mut receipts) =
            world.get_resource_mut::<combat::spell_cast::test_harness::FiendAtonementReceipts>()
    {
        receipts.0.entry(caster).or_default().0 += f64::from(raw);
    }
    let now = combat::spell_cast::now(world);
    let recipients: Vec<_> = world
        .query::<(Entity, &ActiveAuras, &shared::components::Health)>()
        .iter(world)
        .filter(|(_, auras, health)| {
            health.current > 0.0
                && auras.iter().any(|aura| {
                    aura.spell_id == 194384
                        && aura.caster == caster
                        && aura.expires_at.is_none_or(|expiry| expiry > now)
                })
        })
        .map(|(entity, _, health)| (entity, health.current >= health.max))
        .collect();
    if recipients.is_empty() {
        return;
    }
    let store = world.resource::<SpellStore>();
    // SimC76f8118 sc_priest.hpp:1547-1549 and sc_priest.cpp:4233-4252.
    let fraction = store
        .get(81749)
        .expect("Atonement passive")
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("Atonement share")
        .base_points
        / 100.0;
    let pet_bonus = store
        .get(81749)
        .expect("Atonement passive")
        .effects
        .iter()
        .find(|effect| effect.index == 2)
        .expect("Atonement pet share")
        .base_points
        / 100.0;
    let cap = store
        .get(81749)
        .expect("Atonement passive")
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .expect("Atonement target cap")
        .base_points;
    let spell = store.get(94472).expect("Atonement heal").clone();
    let mastery = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == 271534 && aura.caster == caster)
        })
        .and_then(|aura| aura.effects.iter().find(|effect| effect.index == 0))
        .map_or(0.0, |effect| effect.amount);
    // SimC76f8118 sc_priest.hpp:1418-1425: Grace increases healing on Atoned allies.
    let mut transfer = raw * fraction * (1.0 + mastery / 100.0);
    if pet_melee {
        transfer *= 1.0 + pet_bonus;
    }
    // SimC76f8118 sc_priest.hpp:1547-1565 applies these source-school bonuses.
    for (aura, effect, applies) in [
        (
            373054,
            0,
            !pet_melee
                && if pet_spell {
                    school & 32 == 0
                } else {
                    school & 32 != 0
                },
        ),
        (1272359, 3, !pet_spell && !pet_melee && school & 2 != 0),
        (449887, 1, true),
    ] {
        if applies && auras::has_aura(world, caster, aura) {
            let percent =
                combat::spell_cast::scripts::script_effect_value(world, caster, aura, effect);
            transfer *= 1.0 + percent / 100.0;
        }
    }
    let ctx = EffectCtx {
        caster,
        spell: &spell,
        depth: 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    let targets = recipients.len() as f32;
    let injured = recipients.iter().filter(|(_, full)| !full).count() as f32;
    for (ally, full) in recipients {
        // SimC76f8118 sc_priest.cpp:2250-2259: full-health and injured targets
        // use separate reduced-AoE populations.
        let population = if full && targets >= cap {
            targets
        } else {
            injured
        };
        let factor = if population > cap {
            (cap / population).powf(0.75)
        } else {
            1.0
        };
        #[cfg(any(test, feature = "test-support"))]
        let before = world
            .get::<shared::components::Health>(ally)
            .map(|health| health.current);
        apply_critical_transfer_heal(world, &ctx, ally, transfer * factor, crit);
        #[cfg(any(test, feature = "test-support"))]
        if (pet_melee || pet_spell)
            && let Some(before) = before
        {
            let after = world
                .get::<shared::components::Health>(ally)
                .expect("healed ally")
                .current;
            if let Some(mut receipts) =
                world.get_resource_mut::<combat::spell_cast::test_harness::FiendAtonementReceipts>()
            {
                let receipt = receipts.0.entry(caster).or_default();
                receipt.1 += f64::from((transfer * factor).max(0.0).round());
                receipt.2 += f64::from(after - before);
            }
        }
    }
}
