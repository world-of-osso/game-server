//! Bloodbath's native bleed is applied and extended by real landed impacts.
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

const BLOODBATH: u32 = 335096;
const BLEED: u32 = 113344;

fn bleed_deadline(run: &Run) -> Option<f64> {
    run.app
        .world()
        .get::<ActiveAuras>(run.targets[0])?
        .iter()
        .find(|aura| aura.spell_id == BLEED && aura.caster == run.actor)?
        .expires_at
}

#[test]
#[ignore = "private current native DB and exact spec72 player inputs"]
fn fury_bloodbath_landed_hit_applies_and_extends_its_native_bleed() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(72, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    let duration = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(BLEED)
        .unwrap()
        .misc
        .as_ref()
        .unwrap()
        .duration
        .as_ref()
        .unwrap()
        .duration_ms;
    assert!(duration > 0);
    assert!(bleed_deadline(&run).is_none());
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, BLOODBATH, Some(target), 1);
    for _ in 0..2 {
        run.app.update();
    }
    let first = bleed_deadline(&run).expect("landed Bloodbath applies the owned native bleed");
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, BLOODBATH, Some(target), 1);
    for _ in 0..2 {
        run.app.update();
    }
    let second = bleed_deadline(&run).expect("repeat Bloodbath retains the owned native bleed");
    assert!(
        (second - first - f64::from(duration) / 1000.0).abs() < 0.1,
        "repeat Bloodbath extends by one native duration: first={first}, second={second}, duration={duration}"
    );
    for _ in 0..80 {
        run.app.update();
    }
    assert!(
        run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(BLEED)
                && event.amount > 0),
        "the native bleed must deal actual periodic damage"
    );
}
