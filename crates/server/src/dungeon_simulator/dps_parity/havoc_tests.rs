//! Exact-BiS native Havoc hit boundary, independent of the rotation driver.
use super::{engage, prepare};
use crate::auras::ActiveAuras;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

const HAVOC: u32 = 577;
const BURNING_BLADES: u32 = 452408;
const BURNING_BLADES_DAMAGE: u32 = 453177;
const TEN_SECONDS: usize = 200;

#[test]
#[ignore = "opt-in native Havoc BiS/active traits; requires private parity inputs"]
fn havoc_burning_blades_native_bis_auto_attack_earns_periodic_damage() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare(HAVOC, 1).expect("native Havoc BiS and active traits must load");
    assert!(
        run.app
            .world()
            .get::<ActiveAuras>(run.actor)
            .unwrap()
            .iter()
            .any(|aura| aura.spell_id == BURNING_BLADES),
        "actual selected Burning Blades passive must exist before attacking"
    );
    engage(&mut run);
    for _ in 0..TEN_SECONDS {
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let own_damage: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .collect();
    assert!(
        own_damage
            .iter()
            .any(|event| event.spell_id.is_none() && !event.periodic),
        "ordinary native auto-attack must land before testing its consequence"
    );
    assert!(
        own_damage
            .iter()
            .any(|event| { event.spell_id == Some(BURNING_BLADES_DAMAGE) && event.periodic }),
        "native452408 must turn actual melee damage into periodic453177 damage; events={own_damage:?}"
    );
}
