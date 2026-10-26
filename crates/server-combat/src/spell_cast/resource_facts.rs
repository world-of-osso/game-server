//! Read-only resource-dependent spell values, without cast admission or payment.
use bevy::prelude::*;
use shared::components::PowerType;

use super::{CastFailure, CostPool, applicable_costs, resources};
use crate::spell_info::{SpellInfo, SpellStore};

/// SimC76f8118 sc_druid.cpp4673–4676: fresh Rip duration uses the same
/// current pre-payment combo points as the actual cast, not its raw base duration.
pub fn read_prepared_spell(
    world: &World,
    caster: Entity,
    id: u32,
) -> Result<SpellInfo, CastFailure> {
    let spell = world
        .resource::<SpellStore>()
        .get(id)
        .ok_or(CastFailure::UnknownSpell)?;
    // Only combo-point duration/value preparation is qualified here. Ordinary
    // dot metadata must not acquire unrelated affordability or resource requirements.
    if !spell
        .power_costs
        .iter()
        .any(|cost| cost.power_type == PowerType::ComboPoints as i32)
    {
        return Ok(spell.clone());
    }
    let costs = applicable_costs(world, caster, spell)?
        .into_iter()
        .map(|(pool, amount, current)| {
            // A metadata query remains valid below the minimum finisher admission cost.
            let amount = if pool == CostPool::Power(PowerType::ComboPoints) {
                amount.min(current)
            } else {
                amount
            };
            (pool, amount)
        })
        .collect::<Vec<_>>();
    Ok(resources::snapshot(world, caster, spell, &costs))
}
