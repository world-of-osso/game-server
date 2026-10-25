//! Deathstalker lifecycle through the literal profile and native combat.
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires prepared259 private native DB and player actor"]
fn assassination_hero_native_rupture_and_kingsbane_children_deal_damage() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(259, 1).unwrap();
    engage(&mut run);
    let started_at = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started_at);
    for _ in 0..800 {
        drive_player(&mut run).unwrap();
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let kingsbane = events
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(385627)
                && event.kind == CombatLogKind::CastSuccess
        })
        .count();
    assert!(
        kingsbane > 0,
        "the real literal Kingsbane must execute first"
    );
    for child in [1265794, 1265795] {
        let damage: Vec<_> = events
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(run.targets[0].to_bits())
                    && event.spell_id == Some(child)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
            .collect();
        assert_eq!(
            damage.len(),
            kingsbane * 5,
            "each Kingsbane must deliver its five native Implacable child hits for{child}"
        );
    }
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(381628)
                && event.kind == CombatLogKind::Damage
                && event.periodic
                && event.amount > 0),
        "native Rupture must apply the selected Internal Bleeding payload"
    );
}

#[test]
#[ignore = "requires prepared259 private native DB and player actor"]
fn assassination_hero_mark_depletion_earns_darkest_night_and_critical_envenom() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(259, 1).unwrap();
    engage(&mut run);
    let started_at = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started_at);
    let mut marked = false;
    let mut darkest = false;
    let mut empowered_envenoms = 0;
    for _ in 0..1600 {
        let before_dark = crate::auras::has_aura(run.app.world(), run.actor, 457280);
        let before_combo = crate::spell_cast::powers::power_value(
            run.app.world(),
            run.actor,
            shared::components::PowerType::ComboPoints,
        )
        .unwrap();
        let before_events = run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .len();
        drive_player(&mut run).unwrap();
        run.app.update();
        marked |= crate::auras::has_aura(run.app.world(), run.targets[0], 457129);
        darkest |= crate::auras::has_aura(run.app.world(), run.actor, 457280);
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        if before_dark && before_combo.0 >= before_combo.1 {
            for event in events[before_events..].iter().filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(32645)
                    && event.kind == CombatLogKind::Damage
            }) {
                assert!(
                    event.crit,
                    "earned Darkest Night must guarantee the max-CP Envenom crit"
                );
                empowered_envenoms += 1;
            }
        }
    }
    assert!(marked, "native Garrote must establish Deathstalker's Mark");
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events.iter().any(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(457157)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        }),
        "actual finishers must consume marks and deal their native damage"
    );
    assert!(darkest, "depleting the real mark must earn Darkest Night");
    assert!(
        empowered_envenoms > 1,
        "Darkest Night must be consumed and renewed across real mark cycles"
    );
}
