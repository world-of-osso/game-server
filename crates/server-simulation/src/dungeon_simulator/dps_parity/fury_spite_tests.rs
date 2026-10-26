//! Selected Spite rank data is in tenths of a percent, not whole percent.
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use crate::stats::CharacterStats;
use shared::protocol::CombatLogKind;

const SPITE: u32 = 383852;

fn spite_hit(action: u32, selected: bool, critical: bool) -> (f64, f64) {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(72, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::TraitLoadout>(actor)
            .unwrap()
            .active_talent_spells()
            .any(|talent| talent.spell_id == SPITE),
        "the actual selected native Spite rank must supply the modifier"
    );
    let native = crate::spell_cast::scripts::script_effect_value(run.app.world(), actor, SPITE, 0);
    let native_critical =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), actor, SPITE, 1);
    assert_eq!(
        native, native_critical,
        "native damage and critical ranks match"
    );
    assert!(native > 0.0);
    assert!(crate::auras::has_aura(run.app.world(), actor, SPITE));
    if !selected {
        let instance = run
            .app
            .world()
            .get::<ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == SPITE)
            .unwrap()
            .instance_id;
        for index in [0, 1] {
            crate::auras::set_effect_amount(run.app.world_mut(), actor, instance, index, 0.0);
        }
    }
    // Isolate actual native critical/noncritical outcomes without changing AP or payloads.
    run.app
        .world_mut()
        .get_mut::<CharacterStats>(actor)
        .unwrap()
        .crit_pct = if critical { 1000.0 } else { -1000.0 };
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, action, Some(target), 1);
    for _ in 0..2 {
        run.app.update();
    }
    let hits: Vec<_> = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(action)
                && event.amount > 0
        })
        .collect();
    assert_eq!(hits.len(), 1, "one actual native damage probe for {action}");
    assert_eq!(hits[0].crit, critical);
    (hits[0].amount as f64, f64::from(native) / 10.0)
}

#[test]
#[ignore = "private current native DB and exact spec72 player inputs"]
fn fury_spite_uses_native_tenths_for_damage_and_critical_damage() {
    // SimC76f8118 sc_warrior.cpp7894–7899 explicitly normalizes both effects.
    for action in [96103, 335098, 23881] {
        let (base, native) = spite_hit(action, false, false);
        let (buffed, _) = spite_hit(action, true, false);
        let (critical_base, _) = spite_hit(action, false, true);
        let (critical_buffed, _) = spite_hit(action, true, true);
        let ordinary_ratio = buffed / base;
        let critical_ratio = critical_buffed / critical_base;
        let factor = 1.0 + native / 100.0;
        assert!(
            (ordinary_ratio - factor).abs() < 0.002,
            "native Spite tenths: action={action}, damage={ordinary_ratio}, expected={factor}"
        );
        assert!(
            critical_ratio > ordinary_ratio + 0.005
                && critical_ratio < ordinary_ratio + native / 100.0,
            "both native Spite effects bind in tenths: action={action}, damage={ordinary_ratio}, critical={critical_ratio}, native={native}"
        );
    }
}
