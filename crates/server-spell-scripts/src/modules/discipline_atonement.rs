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
    SpellScriptHook::ConsumedEffectValues(&[(81749, 0)]),
];

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
        auras::apply_aura(world, &AuraApplication {
            caster, target: caster, spell_id: 1253591, effect_mask: u32::MAX, source: AuraSource::Cast,
        });
    }
}

fn draw_darkness_card(world: &mut World, caster: Entity) -> bool {
    if world.get::<DarknessDeck>(caster).is_none_or(|deck| deck.cards == 0) {
        world.entity_mut(caster).insert(DarknessDeck { cards: 4, success: true });
    }
    let mut deck = world.get_mut::<DarknessDeck>(caster).unwrap();
    let drawn = deck.success && combat::random::random::<f32>() < 1.0 / f32::from(deck.cards);
    deck.cards -= 1;
    if drawn { deck.success = false; }
    drawn
}

fn discharge_void_shield(world: &mut World, caster: Entity, _victim: Entity, attacker: Option<Entity>, spell: u32, amount: f32) {
    if spell != 1253593 || amount <= 0.0 || !auras::has_aura(world, caster, 1253827) {
        return;
    }
    let is_enemy = |world: &World, target| {
        combat::spell_cast::targets::is_alive(world, target)
            && combat::spell_cast::targets::relation_matches(world, caster, target, combat::spell_cast::targets::Relation::Enemy)
    };
    let target = attacker.filter(|&target| is_enemy(world, target)).or_else(|| {
        let enemies: Vec<_> = world.query::<Entity>().iter(world).filter(|&target| is_enemy(world, target)).collect();
        if enemies.is_empty() { None } else { Some(enemies[(combat::random::random::<f32>() * enemies.len() as f32) as usize]) }
    });
    let Some(target) = target else { return; };
    let percent = combat::spell_cast::scripts::script_effect_value(world, caster, 1253827, 0);
    // SimC76f8118 sc_priest.cpp:84-105: each actual absorbed contribution discharges.
    combat::spell_cast::cast_triggered_with_effect0_amount(world, caster, 1253828, Some(target), 1, amount * percent / 100.0);
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
    let caster = damage.caster;
    if caster == target || !is_discipline(world, caster) || !TRIGGERS.contains(&damage.spell.id) {
        return;
    }
    let now = combat::spell_cast::now(world);
    let recipients: Vec<_> = world
        .query::<(Entity, &ActiveAuras)>()
        .iter(world)
        .filter(|(_, auras)| {
            auras.iter().any(|aura| {
                aura.spell_id == 194384
                    && aura.caster == caster
                    && aura.expires_at.is_none_or(|expiry| expiry > now)
            })
        })
        .map(|(entity, _)| entity)
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
    let spell = store.get(94472).expect("Atonement heal").clone();
    let ctx = EffectCtx {
        caster,
        spell: &spell,
        depth: 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    for ally in recipients {
        apply_critical_transfer_heal(world, &ctx, ally, raw * fraction, crit);
    }
}
