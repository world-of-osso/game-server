//! Exact-BiS native Havoc hit boundary, independent of the rotation driver.
use super::{Run, engage, prepare};
use crate::auras::ActiveAuras;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::{CombatLogEvent, CombatLogKind};

const HAVOC: u32 = 577;
const BURNING_BLADES: u32 = 452408;
const BURNING_BLADES_DAMAGE: u32 = 453177;
const TEN_SECONDS: usize = 200;

fn read_native_damage(run: &Run) -> Vec<&CombatLogEvent> {
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    events
        .iter()
        .filter(|event| {
            let measured_hit = event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits());
            let positive_damage = event.kind == CombatLogKind::Damage && event.amount > 0;
            measured_hit && positive_damage
        })
        .collect()
}

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
    let own_damage = read_native_damage(&run);
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

#[test]
#[ignore = "opt-in native Havoc BiS/active traits; requires private parity inputs"]
fn havoc_burning_blades_native_auto_budget_expires_without_self_feeding() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare(HAVOC, 1).expect("native Havoc BiS and active traits must load");
    let percent = run
        .app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == BURNING_BLADES)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    engage(&mut run);
    for _ in 0..TEN_SECONDS {
        run.app.update();
        if crate::auras::has_aura(run.app.world(), run.targets[0], BURNING_BLADES_DAMAGE) {
            break;
        }
    }
    assert!(crate::auras::has_aura(
        run.app.world(),
        run.targets[0],
        BURNING_BLADES_DAMAGE
    ));
    crate::combat::attack_stop(run.app.world_mut(), run.actor);
    let swings: Vec<_> = read_native_damage(&run)
        .into_iter()
        .filter(|event| event.spell_id.is_none() && !event.periodic)
        .collect();
    assert!(!swings.is_empty());
    let expected = swings
        .iter()
        .map(|event| f64::from(event.amount))
        .sum::<f64>()
        * f64::from(percent)
        / 100.0;
    let swing_count = swings.len();
    for _ in 0..TEN_SECONDS {
        run.app.update();
    }
    let ticks: Vec<_> = read_native_damage(&run)
        .into_iter()
        .filter(|event| event.spell_id == Some(BURNING_BLADES_DAMAGE) && event.periodic)
        .collect();
    assert!(!ticks.is_empty());
    let paid = ticks
        .iter()
        .map(|event| f64::from(event.amount))
        .sum::<f64>();
    // Receipts quantize each native swing and tick to an integer independently.
    let quantization_bound = (swing_count + ticks.len()) as f64;
    assert!(
        (paid - expected).abs() <= quantization_bound,
        "native share{percent}% expected{expected}, paid{paid}, quantization{quantization_bound}"
    );
    assert!(
        !crate::auras::has_aura(run.app.world(), run.targets[0], BURNING_BLADES_DAMAGE),
        "the residual child must expire after actual swings stop"
    );
}

#[test]
#[ignore = "opt-in native Havoc BiS/active traits; requires private parity inputs"]
fn havoc_burning_blades_native_passive_removal_prevents_child_damage() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare(HAVOC, 1).expect("native Havoc BiS and active traits must load");
    assert!(crate::auras::has_aura(
        run.app.world(),
        run.actor,
        BURNING_BLADES
    ));
    crate::auras::remove_auras(
        run.app.world_mut(),
        run.actor,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == BURNING_BLADES,
    );
    engage(&mut run);
    for _ in 0..TEN_SECONDS {
        run.app.update();
    }
    let damage = read_native_damage(&run);
    assert!(
        damage
            .iter()
            .any(|event| event.spell_id.is_none() && !event.periodic)
    );
    assert!(
        damage
            .iter()
            .all(|event| event.spell_id != Some(BURNING_BLADES_DAMAGE))
    );
}
