//! Publish the exact costs/admission already used by normal server casting.
use super::{CostPool, applicable_costs, send_to_owner};
use crate::class_progression::KnownSpells;
use crate::spell_info::SpellStore;
use bevy::prelude::*;
use shared::components::Player;
use shared::protocol::{EffectivePowerCost, SpellCostReading, SpellCostSnapshot};

#[derive(Component)]
struct PublishedCosts(SpellCostSnapshot);

pub(super) fn publish(world: &mut World) {
    let players: Vec<_> = world
        .query_filtered::<Entity, (With<Player>, With<KnownSpells>)>()
        .iter(world)
        .collect();
    for player in players {
        let snapshot = read_snapshot(world, player);
        if world
            .get::<PublishedCosts>(player)
            .is_some_and(|previous| previous.0 == snapshot)
        {
            continue;
        }
        send_to_owner(world, player, snapshot.clone());
        world.entity_mut(player).insert(PublishedCosts(snapshot));
    }
}

fn read_snapshot(world: &World, player: Entity) -> SpellCostSnapshot {
    let store = world.resource::<SpellStore>();
    let spells = world
        .get::<KnownSpells>(player)
        .expect("cost publication requires known spells")
        .iter()
        .map(|spell_id| read_spell_cost(world, player, store, spell_id))
        .collect();
    SpellCostSnapshot { spells }
}

fn read_spell_cost(
    world: &World,
    player: Entity,
    store: &SpellStore,
    spell_id: u32,
) -> SpellCostReading {
    let result = store
        .get(spell_id)
        .ok_or_else(|| format!("spell {spell_id}: metadata absent"))
        .and_then(|spell| {
            applicable_costs(world, player, spell)
                .map_err(|error| format!("spell {spell_id}: {error:?}"))
        });
    let affordable = result
        .as_ref()
        .is_ok_and(|costs| costs.iter().all(|(_, amount, current)| current >= amount));
    let costs = result.map(|readings| readings.into_iter().map(encode_effective_cost).collect());
    SpellCostReading {
        spell_id,
        costs,
        affordable,
    }
}

fn encode_effective_cost((pool, amount, _): (CostPool, f32, f32)) -> EffectivePowerCost {
    let power_type = match pool {
        CostPool::Health => super::POWER_HEALTH,
        CostPool::Power(power) => power as i32,
    };
    EffectivePowerCost { power_type, amount }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spell_cast::test_harness::*;
    use shared::components::PowerType;

    #[test]
    fn cooldown_assist_effective_cost_snapshot_applies_native_base_mana_and_aura_modifier() {
        use shared::components::Mana;
        // Purify527:1.3% of base mana10000. Mental Agility341167 halves it.
        for (known, expected_cost, affordable) in
            [(vec![527], 130.0, false), (vec![527, 341167], 65.0, true)]
        {
            let mut h = harness(Vec::new(), &known, 5).unwrap();
            h.advance(1);
            h.app.world_mut().get_mut::<Mana>(h.caster).unwrap().current = 80.0;
            let snapshot = read_snapshot(h.app.world(), h.caster);
            let reading = snapshot
                .spells
                .iter()
                .find(|reading| reading.spell_id == 527)
                .unwrap();
            assert!((reading.costs.as_ref().unwrap()[0].amount - expected_cost).abs() < 0.001);
            assert_eq!(reading.affordable, affordable);
        }
    }

    #[test]
    fn cooldown_assist_cost_snapshot_matches_native_cost_admission_and_updates() {
        let mut h = harness(Vec::new(), &[12294], 1).unwrap();
        for rage in [200.0, 300.0, 500.0] {
            h.set_power(PowerType::Rage, rage);
            let snapshot = read_snapshot(h.app.world(), h.caster);
            let reading = snapshot
                .spells
                .iter()
                .find(|reading| reading.spell_id == 12294)
                .unwrap();
            assert_eq!(reading.costs.as_ref().unwrap()[0].amount, 300.0);
            assert_eq!(reading.costs.as_ref().unwrap()[0].power_type, 1);
            assert_eq!(reading.affordable, rage >= 300.0);
            let spell = world_db().unwrap().store.get(12294).unwrap();
            assert_eq!(
                reading.affordable,
                super::super::spell_costs(h.app.world(), h.caster, spell).is_ok()
            );
        }
    }
}
