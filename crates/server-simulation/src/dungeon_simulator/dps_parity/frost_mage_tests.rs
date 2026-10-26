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

#[test]
#[ignore = "requires private native70448 DB and exact Arcane62/Frost64 inputs"]
fn spellslinger_native_arcane_and_frost_casts_conjure_their_own_splinters() {
    for (spec, source, payload) in [(62, 30451, 443763), (64, 116, 443722)] {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_literal(spec, 1).unwrap();
        crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            source,
            Some(run.targets[0]),
            false,
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        for _ in 0..40 {
            run.app.update();
        }
        let events = &run.app.world().resource::<ObservedCombatEvents>().0;
        assert!(
            events
                .iter()
                .any(|event| event.source == Some(run.actor.to_bits())
                    && event.target == Some(run.targets[0].to_bits())
                    && event.spell_id == Some(payload)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0),
            "ordinary selected spec{spec} cast{source} must launch native Splinter{payload}"
        );
    }
}

#[test]
#[ignore = "requires private native70448 DB and exact Frost64 inputs"]
fn spellslinger_native_ray_ticks_conjure_splinters_and_open_splinterstorm() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(64, 1).unwrap();
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        205021,
        Some(run.targets[0]),
        false,
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..20 {
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let hits = |spell| {
        events
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(spell)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
            .count()
    };
    let ticks = hits(205021);
    assert!(
        ticks > 0,
        "native Ray must really tick before testing its producer"
    );
    assert!(
        hits(443722) >= ticks,
        "each selected Ray tick must conjure its native Splinter"
    );
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 1247908),
        "native Ray must leave its earned Splinterstorm window"
    );
}

#[test]
#[ignore = "requires private native70448 DB and exact Frost64 inputs"]
fn frost_mage_native_frozen_orb_pays_thirty_delayed_area_pulses() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(64, 3).unwrap();
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(84714, run.actor, Some(run.targets[0])),
    )
    .unwrap();
    for _ in 0..8 {
        run.app.update();
    }
    let count_bolts = |run: &Run| {
        run.app
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
            .filter_map(|event| event.target)
            .collect::<Vec<_>>()
    };
    assert!(
        count_bolts(&run).is_empty(),
        "Orb cannot pulse before its500ms activation"
    );
    for _ in 0..352 {
        run.app.update();
    }
    let bolts = count_bolts(&run);
    for target in &run.targets {
        assert_eq!(
            bolts
                .iter()
                .filter(|&&unit| unit == target.to_bits())
                .count(),
            30,
            "native Orb must pay30 area pulses to each actual in-radius enemy"
        );
    }
    assert_eq!(
        bolts.len(),
        90,
        "one field must not duplicate the area hit per recipient"
    );
}

#[test]
#[ignore = "requires private native70448 DB and exact Arcane62/Frost64 inputs"]
fn spellslinger_native_impacts_reduce_actual_specialization_orb_recharge() {
    use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
    use server::simc_apl::Facts;
    for (spec, orb, source, payload, effect) in
        [(62, 153626, 30451, 443763, 0), (64, 84714, 116, 443722, 1)]
    {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_literal(spec, 1).unwrap();
        let bindings = Bindings {
            actions: [("orb".into(), orb)].into(),
            ..default()
        };
        let encounter = Encounter::Fixture {
            started_at: 0.0,
            duration: 300.0,
            style: FightStyle::Patchwerk,
        };
        let recharge = |run: &Run| {
            NativeFacts::new(run.app.world(), run.actor, &bindings, encounter, 1)
                .value("cooldown.orb.full_recharge_time")
                .unwrap()
        };
        let unit = if spec == 64 {
            run.actor
        } else {
            run.targets[0]
        };
        literal_rotation::execution::execute(
            run.app.world_mut(),
            run.actor,
            literal_rotation::execution::Execution::Spell(orb, unit, Some(run.targets[0])),
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        let before = recharge(&run);
        let event_cursor = run.app.world().resource::<ObservedCombatEvents>().0.len();
        assert!(
            before > 0.0,
            "native spec{spec} Orb recharge must really run"
        );
        let started = crate::spell_cast::now(run.app.world());
        crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            source,
            Some(run.targets[0]),
            false,
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        for _ in 0..40 {
            run.app.update();
        }
        let hits = run
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .skip(event_cursor)
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(payload)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
            .count();
        assert!(
            hits > 0,
            "native Splinters must land before checking their cooldown consumer"
        );
        let native_ms = run
            .app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(444986)
            .unwrap()
            .effects
            .iter()
            .find(|row| row.index == effect)
            .unwrap()
            .base_points;
        let elapsed = crate::spell_cast::now(run.app.world()) - started;
        let reduction = before - recharge(&run) - elapsed;
        assert!(
            (reduction - hits as f64 * f64::from(native_ms) / 1000.0).abs() < 0.005,
            "spec{spec}: {hits} actual Splinter impacts must reduce native recharge; got{reduction}"
        );
    }
}

#[test]
#[ignore = "requires private native70448 DB and exact Frost64 inputs"]
fn frost_mage_native_periodic_icicles_enable_and_pay_glacial_spike() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(64, 1).unwrap();
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 1246832),
        "the actual selected Icicles passive must exist before testing its periodic producer"
    );
    for _ in 0..800 {
        run.app.update();
    }
    let stacks = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.caster == run.actor && aura.spell_id == 205473)
        .map_or(0, |aura| aura.stacks);
    assert_eq!(
        stacks, 5,
        "native hasted Icicles must accumulate without casting Frostbolt"
    );
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 1222865),
        "the native counter cap must enable Glacial Spike"
    );
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(199786, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..20 {
        run.app.update();
    }
    assert!(
        !crate::auras::has_aura(run.app.world(), run.actor, 1222865),
        "a successful Spike must consume its actual enabler"
    );
    assert!(
        run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(228600)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "the real Spike must deliver native228600 rather than a zero-damage Dummy cast"
    );
}

#[test]
#[ignore = "requires private native70448 DB and exact Frost64 inputs"]
fn frost_mage_native_impacts_build_freezing_and_lance_pays_owned_shatter() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(64, 1).unwrap();
    for spell in [116, 116, 44614] {
        literal_rotation::execution::execute(
            run.app.world_mut(),
            run.actor,
            literal_rotation::execution::Execution::Spell(
                spell,
                run.targets[0],
                Some(run.targets[0]),
            ),
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        for _ in 0..20 {
            run.app.update();
        }
    }
    let freezing = |run: &Run| {
        run.app
            .world()
            .get::<crate::auras::ActiveAuras>(run.targets[0])
            .unwrap()
            .iter()
            .find(|aura| aura.caster == run.actor && aura.spell_id == 1221389)
            .map_or(0, |aura| aura.stacks)
    };
    let before = freezing(&run);
    assert!(
        before >= 4,
        "actual Frostbolt/Flurry impacts must earn native owned Freezing"
    );
    // This case isolates owned-debuff payment. Earned Fingers has a separate
    // real Orb/flight test and intentionally does not consume target stacks.
    crate::auras::remove_auras(
        run.app.world_mut(),
        run.actor,
        crate::auras::RemoveReason::Consumed,
        |aura| aura.spell_id == 44544,
    );
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(30455, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..20 {
        run.app.update();
    }
    assert!(
        freezing(&run) < before,
        "successful Lance must consume its actual target's stacks"
    );
    let hits = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(1246949)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count();
    assert_eq!(
        hits, 1,
        "native Shatter primary must hit once, not once per primary/secondary effect"
    );
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
