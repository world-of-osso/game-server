//! HAVOC_BURNING_BLADES_SIMC_POLICY: missing child dispatch only.
//! SimC76f8118 sc_demon_hunter.cpp3088–3112/6878/7040/7455/8258/8506.
//! DB2 owns proc masks/chance, percentage and the child's rolling-periodic clock.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use super::AuraInstance;
use super::procs::AuraEvent;

pub(super) const BURNING_BLADES: u32 = 452408;
const BURNING_BLADES_DAMAGE: u32 = 453177;
const HAVOC: u32 = 577;
const ELIGIBLE_HITS: &[u32] = &[
    222031, 199547, // Chaos Strike.
    227518, 201428, // Annihilation.
    391374, 391378, 393055, 393054, // First Blood, not the ordinary area strikes.
    337819, 393035, // Throw Glaive and Furious Throws.
];

pub(super) fn check_hit(world: &World, _aura: &AuraInstance, event: &AuraEvent) -> bool {
    if event.damage <= 0.0 || event.periodic {
        return false;
    }
    let is_havoc = world
        .get::<ActiveSpec>(event.actor)
        .is_some_and(|spec| spec.0 == HAVOC);
    let eligible = event.spell_id.is_none_or(|id| ELIGIBLE_HITS.contains(&id));
    is_havoc && eligible && event.target.is_some()
}

pub(super) fn apply_hit(world: &mut World, unit: Entity, aura: &AuraInstance, event: &AuraEvent) {
    let Some(percent) = aura.effects.iter().find(|effect| effect.index == 0) else {
        error!("Native Burning Blades452408 has no effect0 percentage");
        return;
    };
    let Some(ticks) = super::periodic::damage_tick_count(world, BURNING_BLADES_DAMAGE) else {
        error!("Native Burning Blades453177 has no positive periodic tick count");
        return;
    };
    let damage_share = event.damage * percent.amount / 100.0;
    crate::spell_cast::cast_triggered_with_effect0_amount(
        world,
        unit,
        BURNING_BLADES_DAMAGE,
        event.target,
        1,
        damage_share / ticks as f32,
    );
}
