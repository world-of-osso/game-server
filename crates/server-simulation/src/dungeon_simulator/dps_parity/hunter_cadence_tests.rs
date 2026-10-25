//! Next native MM gaps: source-qualified cadence, residual and cleave boundaries.
use super::marksmanship_mechanics_tests::{cast, owned_buff, player};
use super::*;
use shared::protocol::CombatLogKind;

fn stacks(run: &Run, spell: u32) -> Option<(u8, u8)> {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.caster == run.actor && aura.spell_id == spell)
        .map(|aura| (aura.stacks, aura.max_stacks))
}

#[test]
#[ignore = "requires private exact254 profile/traits and native world data"]
fn native_stalk_and_strike_chakram_grants_lock_and_load() {
    // SimC76f8118 sc_hunter.cpp4482–4485.
    let mut run = player(1);
    cast(&mut run, 288613, true);
    assert!(!owned_buff(&run, 194594));
    cast(&mut run, 1264949, false);
    assert!(
        owned_buff(&run, 194594),
        "selected Stalk and Strike grants native Lock and Load"
    );
}

#[test]
#[ignore = "requires private exact254 profile/traits and native world data"]
fn native_bulletstorm_rapid_fire_fills_stacks_and_aimed_spends_one() {
    // SimC76f8118 sc_hunter.cpp5803–5806/5596.
    let mut run = player(1);
    cast(&mut run, 257044, false);
    let (earned, maximum) = stacks(&run, 389020).expect("actual Rapid Fire grants Bulletstorm");
    assert!(maximum > 0, "native Bulletstorm has a positive stack limit");
    assert_eq!(
        earned, maximum,
        "Rapid Fire grants the actual native maximum"
    );
    cast(&mut run, 19434, false);
    let remaining = (maximum > 1).then(|| (maximum - 1, maximum));
    assert_eq!(
        stacks(&run, 389020),
        remaining,
        "actual Aimed Shot spends one native Bulletstorm use"
    );
}

#[test]
#[ignore = "requires private exact254 profile/traits and native world data"]
fn native_ranged_critical_damage_starts_master_marksman_bleed() {
    // SimC76f8118 sc_hunter.cpp8583–8626: actual crit amount feeds residual269576.
    let _seed = crate::random::seed_scope(70_448);
    let mut run = player(1);
    cast(&mut run, 75, false);
    for _ in 0..200 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events.iter().any(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(75)
                && event.kind == CombatLogKind::Damage
                && event.crit
                && event.amount > 0
        }),
        "a real native ranged critical strike supplies the bleed budget"
    );
    assert!(
        events.iter().any(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(269576)
                && event.kind == CombatLogKind::Damage
                && event.periodic
                && event.amount > 0
        }),
        "the actual critical strike produces native Master Marksman bleed damage"
    );
}

#[test]
#[ignore = "requires private exact254 profile/traits and native world data"]
fn native_volley_trick_shots_cleave_aimed_to_actual_extra_targets() {
    // SimC76f8118 sc_hunter.cpp6993–6994/5354–5374.
    let mut run = player(3);
    cast(&mut run, 260243, false);
    cast(&mut run, 19434, false);
    for _ in 0..30 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for target in &run.targets {
        assert!(
            events.iter().any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(19434)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            }),
            "actual Volley-enabled Aimed Shot reaches {target:?}"
        );
    }
}
