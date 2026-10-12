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
        .map(|spell_id| {
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
            let costs = result.map(|readings| {
                readings
                    .into_iter()
                    .map(|(pool, amount, _)| EffectivePowerCost {
                        power_type: match pool {
                            CostPool::Health => -2,
                            CostPool::Power(power) => power as i32,
                        },
                        amount,
                    })
                    .collect()
            });
            SpellCostReading {
                spell_id,
                costs,
                affordable,
            }
        })
        .collect();
    SpellCostSnapshot { spells }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spell_cast::test_harness::*;
    use shared::components::PowerType;

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
