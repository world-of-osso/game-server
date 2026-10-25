//! Native Frost player must use the pinned literal opener, not AssistedCombat4512.

#[test]
#[ignore = "requires private native70448 DB and exact Frost64 inputs"]
fn frost_mage_native_buff_last_trigger_retains_refresh_after_removal() {
    use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts, reaction};
    use server::simc_apl::Facts;
    let mut run = prepare_literal(64, 1).unwrap();
    let bindings = Bindings {
        buffs: [("intellect".into(), 1459)].into(),
        ..default()
    };
    let encounter = Encounter::Fixture {
        started_at: 0.0,
        duration: 300.0,
        style: FightStyle::Patchwerk,
    };
    reaction::configure(
        run.app.world_mut(),
        run.actor,
        reaction::ReactionSettings {
            mean: 0.25,
            stddev: 0.05,
            offset: 0.1,
            nu: 0.15,
            maximum: 1.4,
        },
        &[1459],
    )
    .unwrap();
    let read = |run: &Run| {
        NativeFacts::new(run.app.world(), run.actor, &bindings, encounter, 1)
            .value("buff.intellect.last_trigger")
            .unwrap()
    };
    assert!(
        read(&run) < 0.0,
        "untriggered buff must not look freshly triggered"
    );
    let apply = |run: &mut Run| {
        crate::auras::apply_aura(
            run.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: run.actor,
                target: run.actor,
                spell_id: 1459,
                effect_mask: 1,
                source: crate::auras::AuraSource::Cast,
            },
        );
    };
    apply(&mut run);
    let first = crate::spell_cast::now(run.app.world());
    assert_eq!(read(&run), first);
    run.app.update();
    apply(&mut run);
    let refreshed = crate::spell_cast::now(run.app.world());
    assert!(refreshed > first);
    assert_eq!(read(&run), refreshed);
    crate::auras::remove_auras(
        run.app.world_mut(),
        run.actor,
        crate::auras::RemoveReason::Expired,
        |aura| aura.spell_id == 1459,
    );
    run.app.update();
    assert_eq!(
        read(&run),
        refreshed,
        "expiration must retain last successful trigger"
    );
}

#[test]
#[ignore = "requires private native70448 DB and exact Frost64 inputs"]
fn frost_mage_native_potion_facts_follow_configured_item_cooldown() {
    use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
    use server::simc_apl::Facts;
    let mut run = prepare_literal(64, 1).unwrap();
    let bindings = Bindings::default();
    let encounter = Encounter::Fixture {
        started_at: 0.0,
        duration: 300.0,
        style: FightStyle::Patchwerk,
    };
    let read = |run: &Run, name| {
        NativeFacts::new(run.app.world(), run.actor, &bindings, encounter, 1)
            .value(name)
            .unwrap()
    };
    assert_eq!(read(&run, "potion.potion_of_recklessness"), 1.0);
    assert_eq!(read(&run, "potion.POTION_OF_RECKLESSNESS_2"), 1.0);
    assert_eq!(read(&run, "potion.potion_of_recklessness_1"), 0.0);
    assert_eq!(read(&run, "potion.unconfigured_potion"), 0.0);
    assert_eq!(read(&run, "cooldown.potion.remains"), 0.0);
    let potion = run
        .app
        .world()
        .get::<profile_effects::Potion>(run.actor)
        .unwrap()
        .0;
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Item(potion, run.actor),
    )
    .unwrap();
    let remains = read(&run, "cooldown.potion.remains");
    assert!(remains > 0.0);
    run.app.update();
    assert!(read(&run, "cooldown.potion.remains") < remains);
}

use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private native70448 DB and exact Frost64 inputs"]
fn frost_mage_literal_player_executes_precombat_and_opener() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(64, 1).expect("literal Frost player setup");
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 1459));
    let casts = |run: &Run| {
        run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.kind == CombatLogKind::CastSuccess
            })
            .filter_map(|event| event.spell_id)
            .collect::<Vec<_>>()
    };
    assert!(
        casts(&run).contains(&116),
        "precombat must finish native Frostbolt"
    );
    let started = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started);
    engage(&mut run);
    for _ in 0..400 {
        drive_player(&mut run).expect("literal Frost combat action");
        run.app.update();
    }
    let casts = casts(&run);
    assert!(
        casts.contains(&84714),
        "literal opener must cast Frozen Orb: {casts:?}"
    );
    assert!(
        casts.contains(&205021),
        "literal opener must cast Ray of Frost: {casts:?}"
    );
    assert!(
        run.player_rotation.is_none(),
        "AssistedCombat must not own this fixture"
    );
}
