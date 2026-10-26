use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private native70448 DB and exact Frost64 inputs"]
fn frost_mage_native_orb_bolts_roll_fingers_once_per_real_area_pulse() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(64, 3).unwrap();
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 385167));
    let primary = *run
        .app
        .world()
        .get::<shared::components::Position>(run.targets[0])
        .unwrap();
    let mut bolt = crate::spell_cast::load_triggered_spell(run.app.world(), 84721, 1).unwrap();
    bolt.cast_destination = Some([primary.x, primary.y, primary.z]);
    let mut earned_pulses = 0;
    for _ in 0..1000 {
        crate::auras::remove_auras(
            run.app.world_mut(),
            run.actor,
            crate::auras::RemoveReason::Consumed,
            |aura| aura.spell_id == 44544,
        );
        assert!(crate::spell_cast::launch_triggered_spell(
            run.app.world_mut(),
            run.actor,
            &bolt,
            Some(run.targets[0]),
            1
        ));
        let fingers = run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(run.actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 44544 && aura.caster == run.actor)
            .map_or(0, |aura| aura.stacks);
        assert!(
            fingers <= 1,
            "one real three-target Orb pulse must not roll separately for each recipient"
        );
        earned_pulses += usize::from(fingers > 0);
    }
    let hits = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(84721)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count();
    assert_eq!(
        hits, 3000,
        "the exact native pulses must hit all three live recipients"
    );
    assert!(
        earned_pulses > 0,
        "actual native Orb impacts must roll selected Everlasting Frost"
    );
}
