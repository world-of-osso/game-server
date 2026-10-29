//! Discipline's native Atonement marks and damage-to-healing transfer.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, AuraApplication, AuraInstance, AuraSource};
use combat::spell_cast::{
    effects::{EffectCtx, apply_fixed_heal},
    script_registry::SpellScriptHook,
};
use combat::spell_info::SpellStore;
use shared::components::ActiveSpec;
use shared::protocol::{CombatLogEvent, CombatLogKind};

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterAuraApplication(apply_mark),
    SpellScriptHook::HealthChangeObserved(mark_flash_heal),
    SpellScriptHook::HealthChangeObserved(transfer_damage),
    SpellScriptHook::ConsumedEffectValues(&[(81749, 0)]),
];

fn apply_mark(world: &mut World, target: Entity, aura: &AuraInstance) {
    if !matches!(aura.spell_id, 17 | 139) || !is_discipline(world, aura.caster) {
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

fn transfer_damage(world: &mut World, event: &CombatLogEvent) {
    if event.kind != CombatLogKind::Damage || event.amount <= 0 {
        return;
    }
    let (Some(caster), Some(target)) = (
        event.source.and_then(Entity::try_from_bits),
        event.target.and_then(Entity::try_from_bits),
    ) else {
        return;
    };
    if caster == target || !is_discipline(world, caster) {
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
        apply_fixed_heal(world, &ctx, ally, event.amount as f32 * fraction);
    }
}
