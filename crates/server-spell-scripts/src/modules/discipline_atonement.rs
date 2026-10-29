//! Discipline's native Atonement marks and damage-to-healing transfer.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, AuraApplication, AuraInstance, AuraSource};
use combat::spell_cast::{
    effects::{EffectCtx, apply_critical_transfer_heal},
    script_registry::SpellScriptHook,
};
use combat::spell_info::{SpellInfo, SpellStore};
use shared::components::ActiveSpec;
use shared::protocol::{CombatLogEvent, CombatLogKind};

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterAuraApplication(apply_mark),
    SpellScriptHook::NativeAfterCast(master_the_darkness_action),
    SpellScriptHook::AfterAbsorbConsumed(discharge_void_shield),
    SpellScriptHook::HealthChangeObserved(mark_flash_heal),
    SpellScriptHook::RawResolvedDamage(transfer_damage),
    SpellScriptHook::EffectHit(shadowfiend_death_hit),
    SpellScriptHook::MeleeRawResolved(transfer_pet_melee),
    SpellScriptHook::SummonInitialized(initialize_fiend),
    SpellScriptHook::AfterNativeCastUpdate(update_voidwraith),
    SpellScriptHook::ConsumedEffectValues(&[(81749, 0)]),
];

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
    matches!(spell, 47540 | 8092 | 1253593).then_some(after_darkness_action)
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
    let is_enemy = |world: &World, target| {
        combat::spell_cast::targets::is_alive(world, target)
            && combat::spell_cast::targets::relation_matches(
                world,
                caster,
                target,
                combat::spell_cast::targets::Relation::Enemy,
            )
    };
    let target = attacker
        .filter(|&target| is_enemy(world, target))
        .or_else(|| {
            let enemies: Vec<_> = world
                .query::<Entity>()
                .iter(world)
                .filter(|&target| is_enemy(world, target))
                .collect();
            if enemies.is_empty() {
                None
            } else {
                Some(enemies[(combat::random::random::<f32>() * enemies.len() as f32) as usize])
            }
        });
    let Some(target) = target else {
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
    if event.kind != CombatLogKind::Heal || event.spell_id != Some(2061) {
        return;
    }
    let (Some(caster), Some(target)) = (
        event.source.and_then(Entity::try_from_bits),
        event.target.and_then(Entity::try_from_bits),
    ) else {
        return;
    };
    if is_discipline(world, caster) {
        // SimC 76f8118 sc_priest.cpp:1782-1792: Flash Heal applies Atonement on impact.
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
