//! Named SimC76f8118 Warrior policies; TC a352b1fa has no corresponding scripts.
//! Relationships: sc_warrior.cpp9176–9193/9324–9336. Amounts remain native DB2.
use bevy::prelude::*;
use shared::components::Player;

use super::ActiveAuras;
use crate::class_progression::ActiveSpec;
use crate::followers::Follower;
use crate::stats::StatModifiers;

const ARMORED_TO_THE_TEETH: u32 = 384124;
const BATTLEFIELD_COMMANDER: u32 = 424742;

pub(super) fn add_policies(world: &World, unit: Entity, stats: &mut StatModifiers) {
    let (class, spec) = match (world.get::<Player>(unit), world.get::<Follower>(unit)) {
        (Some(player), _) => (
            player.class,
            world.get::<ActiveSpec>(unit).map(|spec| spec.0),
        ),
        (None, Some(follower)) => (follower.profile.class_id, Some(follower.profile.spec_id)),
        _ => return,
    };
    if class != 1 {
        return;
    }
    let Some(auras) = world.get::<ActiveAuras>(unit) else {
        return;
    };
    // WARRIOR_ARMORED_TO_THE_TEETH_SIMC_POLICY: effect2 for Protection, effect1 otherwise.
    let index = if spec == Some(73) { 2 } else { 1 };
    stats.strength_from_armor_pct += native_amount(auras, ARMORED_TO_THE_TEETH, index, 226);
    // WARRIOR_BATTLEFIELD_COMMANDER_SIMC_POLICY: compose with existing AP percentages.
    let amount = native_amount(auras, BATTLEFIELD_COMMANDER, 5, 4);
    stats.attack_power_pct += amount * (1.0 + stats.attack_power_pct / 100.0);
}

fn native_amount(auras: &ActiveAuras, spell: u32, index: u8, aura_type: u32) -> f32 {
    auras
        .iter()
        .filter(|aura| aura.spell_id == spell)
        .map(|aura| {
            aura.effects
                .iter()
                .filter(|effect| effect.index == index && effect.aura_type == aura_type)
                .map(|effect| effect.amount * aura.stack_multiplier())
                .sum::<f32>()
        })
        .sum()
}
