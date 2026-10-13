//! TC a352b1fa Unit.cpp:1607–1657: DAMAGE_SHIELD15 after damaging melee.
//! Snapshot effects before retaliation because damage can remove or replace auras.
use bevy::prelude::*;

use super::{ActiveAuras, handlers};
use crate::spell_cast::{self, effects::EffectCtx};
use crate::spell_info::SpellStore;

pub(crate) fn retaliate(world: &mut World, owner: Entity, attacker: Entity) {
    let shields = snapshot_shields(world, owner);
    for (spell_id, aura_caster, amount) in shields {
        let spell = world
            .resource::<SpellStore>()
            .get(spell_id)
            .expect("active damage shield has admitted spell metadata")
            .clone();
        if !spell_cast::damage_shield_hits(world, owner, &spell, attacker) {
            continue;
        }
        let ctx = EffectCtx {
            caster: owner,
            spell: &spell,
            depth: 0,
            dest: None,
            chain_multiplier: 1.0,
        };
        spell_cast::effects::deal_damage_shield(world, &ctx, aura_caster, attacker, amount);
    }
}

fn snapshot_shields(world: &World, owner: Entity) -> Vec<(u32, Entity, f32)> {
    let Some(auras) = world.get::<ActiveAuras>(owner) else {
        return Vec::new();
    };
    auras
        .auras
        .iter()
        .flat_map(|aura| {
            aura.effects
                .iter()
                .filter(|effect| effect.aura_type == handlers::DAMAGE_SHIELD)
                .map(|effect| {
                    (
                        aura.spell_id,
                        aura.caster,
                        effect.amount * aura.stack_multiplier(),
                    )
                })
        })
        .collect()
}
