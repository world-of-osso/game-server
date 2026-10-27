//! Cold Steel, Hot Blood produces Gushing Wound only on critical native impacts.
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use crate::stats::CharacterStats;
use shared::protocol::CombatLogKind;

const COLD_STEEL: u32 = 383959;
const GUSHING_WOUND: u32 = 385042;

fn hit_and_observe(action: u32, critical: bool) -> (bool, bool) {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(72, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    assert!(crate::auras::has_aura(run.app.world(), actor, COLD_STEEL));
    run.app
        .world_mut()
        .get_mut::<CharacterStats>(actor)
        .unwrap()
        .crit_pct = if critical { 1000.0 } else { -1000.0 };
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, action, Some(target), 1);
    for _ in 0..2 {
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let hit = events
        .iter()
        .find(|event| {
            event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(action)
                && event.amount > 0
        })
        .expect("actual native hit");
    assert_eq!(hit.crit, critical);
    let applied = run
        .app
        .world()
        .get::<ActiveAuras>(target)
        .is_some_and(|auras| {
            auras
                .iter()
                .any(|aura| aura.spell_id == GUSHING_WOUND && aura.caster == actor)
        });
    for _ in 0..60 {
        run.app.update();
    }
    let periodic = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .any(|event| {
            event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(GUSHING_WOUND)
                && event.amount > 0
        });
    (applied, periodic)
}

#[test]
#[ignore = "private current native DB and exact spec72 player inputs"]
fn fury_gushing_wound_requires_critical_bloodthirst_or_bloodbath() {
    // SimC76f8118 sc_warrior.cpp2679–2682/2915–2919, including Unhinged hits.
    for action in [23881, 335096] {
        assert_eq!(
            hit_and_observe(action, false),
            (false, false),
            "noncritical action {action}"
        );
        assert_eq!(
            hit_and_observe(action, true),
            (true, true),
            "critical action {action}"
        );
    }
}
