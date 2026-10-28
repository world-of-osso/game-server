//! Into the Fray follows actual nearby units, not an assumed raid size.
use super::combat;
use bevy::prelude::*;
use combat::auras::{
    self, AuraInstance, RemoveReason,
    scripts::{AuraScript, AuraScripts},
};
use combat::maps::InMap;
use combat::spell_cast::{
    self,
    script_registry::SpellScriptHook,
    targets::{self, Relation},
};
use shared::components::{Health, Position};

const TALENT: u32 = 202603;
const BUFF: u32 = 202602;
pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::LateAuraRegistration(register)];

fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        TALENT,
        AuraScript {
            on_apply: Some(update),
            on_periodic: Some(tick),
            on_remove: Some(remove),
            ..Default::default()
        },
    );
}

fn tick(world: &mut World, unit: Entity, aura: &AuraInstance, _: u8) {
    update(world, unit, aura);
}

fn update(world: &mut World, unit: Entity, aura: &AuraInstance) {
    let Some(origin) = targets::position(world, unit) else {
        return;
    };
    let Some(map) = world.get::<InMap>(unit).copied() else {
        return;
    };
    let radius = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("Into the Fray requires its native range")
        .amount;
    let spell = world
        .resource::<combat::spell_info::SpellStore>()
        .get(BUFF)
        .expect("Into the Fray buff exists");
    let maximum = auras::fresh_max_stacks(world, unit, spell);
    let count = world
        .query::<(Entity, &Position, &Health, &InMap)>()
        .iter(world)
        .filter(|(entity, position, health, candidate_map)| {
            *entity != unit
                && health.current > 0.0
                && **candidate_map == map
                && origin.distance_squared(Vec3::new(position.x, position.y, position.z))
                    <= radius * radius
                && (targets::relation_matches(world, unit, *entity, Relation::Ally)
                    || targets::relation_matches(world, unit, *entity, Relation::Enemy))
        })
        .take(usize::from(maximum))
        .count() as u8;
    let current = world
        .get::<auras::ActiveAuras>(unit)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == BUFF && aura.caster == unit)
                .map(|aura| aura.stacks)
        })
        .unwrap_or(0);
    if count == current {
        return;
    }
    if count == 0 {
        auras::remove_auras(world, unit, RemoveReason::Consumed, |aura| {
            aura.spell_id == BUFF && aura.caster == unit
        });
    } else {
        if current == 0 {
            if !spell_cast::try_cast_triggered(world, unit, BUFF, Some(unit), 1) {
                warn!(?unit, "Into the Fray haste buff refused");
                return;
            }
        }
        auras::set_self_cast_stacks(world, unit, BUFF, count);
    }
}

fn remove(world: &mut World, unit: Entity, _: &AuraInstance, _: RemoveReason) {
    auras::remove_auras(world, unit, RemoveReason::Consumed, |aura| {
        aura.spell_id == BUFF && aura.caster == unit
    });
}
