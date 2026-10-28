//! Deathstalker lifecycle through the literal profile and native combat.
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires prepared259 native profile and world data"]
fn assassination_mass_casualty_hits_only_secondary_owned_rupture() {
    use shared::components::PowerType;
    let mut run = prepare_measurement(259, 3).unwrap();
    engage(&mut run);
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 1273035));
    for (spell, target) in [(457129, run.targets[0]), (1943, run.targets[1])] {
        crate::auras::apply_aura(
            run.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: run.actor,
                target,
                spell_id: spell,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
    }
    assert!(crate::auras::set_owned_stacks(
        run.app.world_mut(),
        run.actor,
        run.targets[0],
        457129,
        3
    ));
    {
        let mut pools = run
            .app
            .world_mut()
            .get_mut::<crate::spell_cast::powers::PowerPools>(run.actor)
            .unwrap();
        for power in [PowerType::Energy, PowerType::ComboPoints] {
            let pool = pools.get_mut(power).unwrap();
            pool.current = pool.max;
        }
    }
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(32645, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for (index, target) in run.targets.iter().enumerate() {
        let hits = events
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(1273061)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
            .count();
        assert_eq!(
            hits,
            usize::from(index == 1),
            "only secondary owned Rupture receives Mass Casualty"
        );
    }
}

#[test]
#[ignore = "requires prepared259 private native DB and player actor"]
fn assassination_hero_garrote_window_allows_real_sequential_target_spread() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(259, 3).unwrap();
    engage(&mut run);
    let started_at = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started_at);
    let started = crate::spell_cast::now(run.app.world());
    for target in run.targets.clone() {
        literal_rotation::execution::execute(
            run.app.world_mut(),
            run.actor,
            literal_rotation::execution::Execution::Spell(703, target, Some(target)),
        )
        .expect("actual Improved Garrote must permit another target after only the real GCD");
        preparation::finish_pending_setup(&mut run).unwrap();
        let aura = run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(target)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 703 && aura.caster == run.actor);
        assert!(
            aura.is_some(),
            "the native Garrote must actually land on{target:?}"
        );
    }
    assert!(
        crate::spell_cast::now(run.app.world()) - started < 6.0,
        "three actual GCDs must not wait for the ordinary native six-second recovery"
    );
}

#[test]
#[ignore = "requires prepared259 private native DB and player actor"]
fn assassination_hero_garrote_earns_native_post_stealth_window() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(259, 1).unwrap();
    assert!(
        run.app
            .world()
            .get::<crate::auras::ActiveAuras>(run.actor)
            .unwrap()
            .has_type(crate::auras::handlers::MOD_STEALTH)
    );
    engage(&mut run);
    let started_at = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started_at);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(703, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    assert!(
        crate::auras::has_aura(run.app.world(), run.targets[0], 703),
        "ordinary Garrote must be the earned stealth exit"
    );
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 392401),
        "leaving real stealth must grant the native Improved Garrote window"
    );
    for _ in 0..200 {
        run.app.update();
    }
    assert!(
        !crate::auras::has_aura(run.app.world(), run.actor, 392401),
        "native window must expire without a new stealth exit"
    );
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(703)
                && event.kind == CombatLogKind::Damage
                && event.periodic
                && event.amount > 0)
    );
}

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

#[test]
#[ignore = "requires prepared259 private native DB and player actor"]
fn assassination_hero_unshakeable_drive_earned_and_delayed_use() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(259, 1).unwrap();
    engage(&mut run);
    let started_at = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started_at);
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 1248774));
    for _ in 0..2000 {
        drive_player(&mut run).unwrap();
        run.app.update();
        if crate::auras::has_aura(run.app.world(), run.actor, 1248775) {
            break;
        }
    }
    let stacks = |run: &Run| {
        run.app
            .world()
            .get::<crate::auras::ActiveAuras>(run.actor)
            .unwrap()
            .iter()
            .find(|a| a.spell_id == 1248775 && a.caster == run.actor)
            .map_or(0, |a| a.stacks)
    };
    let before = stacks(&run);
    assert!(before > 0, "spending a real mark must earn Drive");
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        run.actor,
        1329,
        Some(run.targets[0]),
        0
    ));
    assert_eq!(
        stacks(&run),
        before,
        "Mutilate retains Drive through its hands"
    );
    run.app
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_millis(100),
        ));
    run.app.update();
    assert_eq!(stacks(&run), before, "Drive must survive until 201ms");
    run.app
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_millis(150),
        ));
    run.app.update();
    assert_eq!(
        stacks(&run),
        before - 1,
        "one completed Mutilate consumes exactly one use"
    );
}

#[test]
#[ignore = "requires prepared261 private native DB and player actor"]
fn subtlety_hero_unshakeable_drive_earned_and_delayed_use() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(261, 1).unwrap();
    engage(&mut run);
    let started_at = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started_at);
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 1248774));
    for _ in 0..2000 {
        drive_player(&mut run).unwrap();
        run.app.update();
        if crate::auras::has_aura(run.app.world(), run.actor, 1248775) {
            break;
        }
    }
    let stacks = |run: &Run| {
        run.app
            .world()
            .get::<crate::auras::ActiveAuras>(run.actor)
            .unwrap()
            .iter()
            .find(|a| a.spell_id == 1248775 && a.caster == run.actor)
            .map_or(0, |a| a.stacks)
    };
    let before = stacks(&run);
    assert!(before > 0, "spending a real mark must earn Drive");
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        run.actor,
        185438,
        Some(run.targets[0]),
        0
    ));
    assert_eq!(
        stacks(&run),
        before,
        "Shadowstrike retains Drive during its impact"
    );
    run.app
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_millis(100),
        ));
    run.app.update();
    assert_eq!(stacks(&run), before, "Drive must survive until 201ms");
    run.app
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_millis(150),
        ));
    run.app.update();
    assert_eq!(
        stacks(&run),
        before - 1,
        "one completed Shadowstrike consumes exactly one use"
    );
}

#[test]
#[ignore = "requires prepared259 native profile and world data"]
fn assassination_hero_quietus_spends_new_mark_with_native_chance() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(259, 1).unwrap();
    engage(&mut run);
    let started_at = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started_at);
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 1273017));
    let mark_stacks = |run: &Run| {
        run.app.world().get::<crate::auras::ActiveAuras>(run.targets[0]).and_then(|auras| {
            auras.iter().find(|a| a.spell_id == 457129 && a.caster == run.actor).map(|a| a.stacks)
        })
    };
    let mut spent = 0;
    let mut retained = 0;
    for _ in 0..6000 {
        let before = mark_stacks(&run);
        drive_player(&mut run).unwrap();
        run.app.update();
        if before.is_none() {
            match mark_stacks(&run) {
                Some(2) => spent += 1,
                Some(3) => retained += 1,
                _ => {}
            }
        }
    }
    assert!(spent > 0 && retained > 0, "new marks must both lose and retain a stack: spent={spent}, retained={retained}");
}
