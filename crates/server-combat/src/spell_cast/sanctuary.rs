//! TC a352b1fa SpellEffects.cpp3215 / Unit.cpp6029 / Spell.cpp2810.
use bevy::prelude::*;
use shared::components::{Player, ThreatTable};

use super::effects::EffectCtx;
use crate::spell_info::{SpellEffect, SpellInfo, SpellStore};

#[derive(Component)]
struct LastSanctuary(f64);
const IGNORE_SANCTUARY: u32 = 0x80; // SPELL_ATTR8_IGNORE_SANCTUARY.

pub(super) fn apply(world: &mut World, _: &EffectCtx, _: &SpellEffect, target: Entity) {
    let dungeon = world.get::<crate::maps::InMap>(target).is_some_and(|map| {
        world
            .get_resource::<crate::maps::MapStore>()
            .and_then(|store| store.map(map.map_id))
            .is_some_and(|map| map.is_dungeon())
    });
    if world.get::<Player>(target).is_some() && !dungeon {
        stop_attacks(world, target);
        crate::combat_refs::sanctuary(world, target);
    } else {
        reset_target_threat(world, target);
    }
    let now = super::now(world);
    world.entity_mut(target).insert(LastSanctuary(now));
}

fn stop_attacks(world: &mut World, target: Entity) {
    let attackers: Vec<_> = world
        .query::<(Entity, &crate::combat::AutoAttack)>()
        .iter(world)
        .filter(|(_, attack)| attack.victim == target)
        .map(|(unit, _)| unit)
        .collect();
    for attacker in attackers {
        crate::combat::attack_stop(world, attacker);
    }
    crate::combat::attack_stop(world, target);
}

fn reset_target_threat(world: &mut World, target: Entity) {
    let mut tables = world.query::<&mut ThreatTable>();
    for mut table in tables.iter_mut(world) {
        let amount = table.threat_for(target.to_bits());
        if amount != 0.0 {
            table.add_threat(target.to_bits(), -amount);
        }
    }
}

pub(super) fn blocks_hit(world: &World, target: Entity, spell: &SpellInfo, launched: f64) -> bool {
    if super::attributes(spell, 8) & IGNORE_SANCTUARY != 0 {
        return false;
    }
    let protected = world
        .get::<LastSanctuary>(target)
        .is_some_and(|sanctuary| launched <= sanctuary.0);
    protected
        && world
            .get_resource::<SpellStore>()
            .is_some_and(|store| crate::spell_info::positivity::negative_effects(store, spell) != 0)
}
