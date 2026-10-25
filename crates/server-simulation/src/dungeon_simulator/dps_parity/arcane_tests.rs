//! Literal Arcane profile control and source-defined native producers.
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn stacks(run: &Run, spell: u32) -> u8 {
    run.app
        .world()
        .get::<ActiveAuras>(run.actor)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.caster == run.actor && aura.spell_id == spell)
        })
        .map_or(0, |aura| aura.stacks)
}

fn cast(run: &mut Run, spell: u32) {
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(spell, run.targets[0], Some(run.targets[0])),
    )
    .unwrap_or_else(|error| panic!("ordinary Arcane{spell}: {error}"));
    preparation::finish_pending_setup(run).unwrap();
}

fn grant(run: &mut Run, spell: u32, count: u8) {
    for _ in 0..count {
        crate::auras::apply_aura(
            run.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: run.actor,
                target: run.actor,
                spell_id: spell,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
    }
}

#[test]
#[ignore = "requires private native70448 DB and exact Arcane62 inputs"]
fn arcane_batch_surge_guarantees_clearcasting() {
    let mut run = prepare_literal(62, 1).unwrap();
    assert_eq!(stacks(&run, 263725), 0);
    cast(&mut run, 365350);
    assert!(
        stacks(&run, 263725) > 0,
        "Surge must earn native Clearcasting"
    );
}

#[test]
#[ignore = "requires private native70448 DB and exact Arcane62 inputs"]
fn arcane_batch_missiles_ticks_generate_damage_salvo_and_cumulative_power() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(62, 1).unwrap();
    grant(&mut run, 263725, 1);
    cast(&mut run, 5143);
    let hits = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(7268)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count();
    assert!(
        hits >= 4,
        "ordinary native Missiles must deliver its scheduled ticks: {hits}"
    );
    assert!(stacks(&run, 1242974) >= 4, "one Salvo per native tick");
    assert!(
        stacks(&run, 1296930) >= 4,
        "one Cumulative Power per native tick"
    );
    let charges = crate::spell_cast::powers::power_value(
        run.app.world(),
        run.actor,
        shared::components::PowerType::ArcaneCharges,
    )
    .unwrap()
    .0;
    assert!(
        charges > 0.0,
        "selected High Voltage must earn native charges across ticks"
    );
}

#[test]
#[ignore = "requires private native70448 DB and exact Arcane62 inputs"]
fn arcane_batch_barrage_consumes_salvo_and_rolls_native_orb_and_prismatic() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(62, 1).unwrap();
    // Freeze the actual native max-stack proc as the Barrage input; do not alter its values.
    grant(&mut run, 1242974, 20);
    let mut prism = false;
    let mut orb = false;
    for _ in 0..12 {
        if prism && orb {
            break;
        }
        cast(&mut run, 44425);
        assert_eq!(
            stacks(&run, 1242974),
            3,
            "Barrage consumes Salvo, then native Polished Focus refunds three"
        );
        prism |= stacks(&run, 1295942) > 0;
        orb |= run
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(153640)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            });
        grant(&mut run, 1242974, 20);
    }
    assert!(prism, "earned Salvo must roll native Prismatic Bolt");
    assert!(
        orb,
        "earned Salvo must roll Orb Barrage and its actual native damage child"
    );
}

#[test]
#[ignore = "requires private native70448 DB and exact Arcane62 inputs"]
fn arcane_touch_native_cast_marks_target_and_expires_into_stored_damage() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(62, 1).unwrap();
    cast(&mut run, 321507);
    let owned_touch = run
        .app
        .world()
        .get::<ActiveAuras>(run.targets[0])
        .is_some_and(|auras| {
            auras
                .iter()
                .any(|aura| aura.caster == run.actor && aura.spell_id == 210824)
        });
    assert!(
        owned_touch,
        "actual Touch cast must apply its native accumulation debuff"
    );
    cast(&mut run, 30451);
    let until = crate::spell_cast::now(run.app.world()) + 15.0;
    while crate::spell_cast::now(run.app.world()) < until {
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(210833)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "owned spell damage must discharge into native210833 on expiration"
    );
}

#[test]
#[ignore = "requires private native70448 DB and exact Arcane62 inputs"]
fn arcane_touch_aegwynns_clearcasting_also_grants_selected_brainstorm() {
    let mut run = prepare_literal(62, 1).unwrap();
    cast(&mut run, 321507);
    assert!(
        stacks(&run, 263725) > 0,
        "selected Aegwynn's Technique guarantees Clearcasting"
    );
    assert!(
        stacks(&run, 461531) > 0,
        "every successful Clearcasting also triggers Brainstorm"
    );
}

#[test]
#[ignore = "merge-train shared PRD regression"]
fn arcane_batch_clearcasting_prd_uses_the_pinned_coefficient_cap() {
    let constant = crate::random::accumulated_proc_constant(0.12, 13);
    let mut survival = 1.0;
    let mut expected_attempts = 0.0;
    for attempt in 1..=13 {
        expected_attempts += survival;
        survival *= 1.0 - (constant * f64::from(attempt)).min(1.0);
    }
    assert!((expected_attempts - 1.0 / 0.12).abs() < 1e-10);
    assert_eq!(crate::random::accumulated_proc_constant(0.05, 13), 1e-300);
    assert!(
        (crate::random::accumulated_proc_constant(0.15, 0) - 0.03222091437308769).abs() < 1e-12,
        "uncapped Arms caller unchanged"
    );
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec62 profile/traits"]
fn arcane_literal_fixture_executes_profile_cooldowns_and_filler() {
    let result = measure(62, 1).expect("native Arcane profile fixture");
    assert_eq!(result["driver"], "LITERAL_SIMC_ARCANE_76f8118");
    assert_eq!(result["assisted_decisions"], 0);
    assert!(
        result["arcane_rotation"]["snapshot_stats"]["native"]["spell_power"]
            .as_f64()
            .unwrap()
            > 0.0
    );
    let actions = result["combat"]["actions"].as_array().unwrap();
    for spell in [30451, 365350, 321507] {
        assert!(
            actions
                .iter()
                .any(|action| action["spell_id"] == spell
                    && action["count"].as_u64().unwrap_or(0) > 0),
            "literal profile must execute native spell{spell}: {actions:?}"
        );
    }
}
