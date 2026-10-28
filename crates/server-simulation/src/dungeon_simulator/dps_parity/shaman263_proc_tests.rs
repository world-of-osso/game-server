//! Real earned Tempest charges and native linked Strike payloads, independent of APL selection.
use super::enhancement_generation_tests::{cast, earn_tempest_by_spending};
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn damage_hits(run: &Run, spell: u32, target: Entity) -> usize {
    let expected_route = (
        Some(run.actor.to_bits()),
        Some(target.to_bits()),
        Some(spell),
    );
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            let actual_route = (event.source, event.target, event.spell_id);
            let dealt_damage = event.kind == CombatLogKind::Damage && event.amount > 0;
            actual_route == expected_route && dealt_damage
        })
        .count()
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn shaman263_ascendance_replaces_both_real_swing_hands_with_windlash() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(263, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::AutoAttack(target),
    )
    .unwrap();
    cast(&mut run, 114051, actor);
    assert!(crate::auras::has_aura(run.app.world(), actor, 114051));
    for _ in 0..180 {
        if damage_hits(&run, 114089, target) > 0 && damage_hits(&run, 114093, target) > 0 {
            break;
        }
        run.app.update();
    }
    assert!(
        damage_hits(&run, 114089, target) > 0,
        "main-hand swing becomes actual Windlash"
    );
    assert!(
        damage_hits(&run, 114093, target) > 0,
        "off-hand swing becomes actual Windlash"
    );
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn shaman263_static_accumulation_stops_when_its_last_real_mode_expires() {
    // SimC76f8118 sc_shaman.cpp12840–12860: mode lifetime, not raw Static's15s, owns this aura.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(263, 1).unwrap();
    let actor = run.actor;
    cast(&mut run, 384352, actor);
    assert!(crate::auras::has_aura(run.app.world(), actor, 466772));
    assert!(crate::auras::has_aura(run.app.world(), actor, 384437));
    for _ in 0..1200 {
        if !crate::auras::has_aura(run.app.world(), actor, 466772) {
            break;
        }
        run.app.update();
    }
    assert!(
        !crate::auras::has_aura(run.app.world(), actor, 466772),
        "real mode must finish its native lifetime"
    );
    assert!(
        !crate::auras::has_aura(run.app.world(), actor, 384437),
        "Static cannot keep generating after its last actual mode ends"
    );
}

fn tempest_procs(run: &Run) -> u8 {
    run.app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 454015 && aura.caster == run.actor)
        .map_or(0, |aura| aura.stacks)
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn shaman263_tempest_cast_spends_one_of_two_actually_earned_procs() {
    // SimC76f8118 sc_shaman.cpp9966 decrements one, before the spender can earn its next proc.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(263, 1).unwrap();
    earn_tempest_by_spending(&mut run);
    assert_eq!(tempest_procs(&run), 1);
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .remove::<crate::combat::AutoAttack>();
    let actor = run.actor;
    cast(&mut run, 114051, actor);
    assert_eq!(
        tempest_procs(&run),
        2,
        "actual Ascendance earns the second proc through selected Descending Skies"
    );
    let target = run.targets[0];
    cast(&mut run, 452201, target);
    assert_eq!(
        tempest_procs(&run),
        1,
        "real Tempest must not discard the other earned proc"
    );
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn shaman263_real_strike_dispatches_ride_the_lightning_to_native_chain_targets() {
    // SimC76f8118 sc_shaman.cpp5647/10757–10766/12532–12551 uses native211094, not another foreground CL.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(263, 3).unwrap();
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::KnownSpells>(run.actor)
            .unwrap()
            .contains(289874)
    );
    engage(&mut run);
    let target = run.targets[0];
    cast(&mut run, 17364, target);
    for _ in 0..60 {
        run.app.update();
    }
    for &target in &run.targets {
        assert_eq!(
            damage_hits(&run, 211094, target),
            1,
            "one real Strike emits one native Ride the Lightning bounce per actual target"
        );
    }
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn shaman263_overcharge_scales_real_nature_crit_with_live_owner_crit() {
    use crate::spell_cast::effects::factor_capture::FactorCapture;
    let mut run = prepare_literal(263, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    assert!(crate::auras::has_aura(run.app.world(), actor, 1251026));
    let measure = |run: &mut Run, crit: f32| {
        run.app
            .world_mut()
            .get_mut::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .crit_pct = crit;
        run.app
            .insert_resource(FactorCapture::new(actor, target, 469270, "doom", 1));
        crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 469270, Some(target), 1);
        for _ in 0..40 {
            let capture = run.app.world().resource::<FactorCapture>().export();
            if let Some(event) = capture["events"]
                .as_array()
                .unwrap()
                .iter()
                .find(|event| event["event"] == "result" && event["spell_id"] == 469270)
            {
                assert_eq!(event["factors"]["result"], "crit");
                return 1.0 + event["factors"]["crit_bonus"].as_f64().unwrap();
            }
            run.app.update();
        }
        panic!("real Doom Winds impact must occur");
    };
    let with_100 = measure(&mut run, 100.0);
    let with_150 = measure(&mut run, 150.0);
    crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 1251026,
    );
    let without = measure(&mut run, 100.0);
    let ordinary = run
        .app
        .world()
        .get::<crate::auras::stat_mods::AuraStatMods>(actor)
        .unwrap()
        .critical_damage(9) as f64;
    // Native spell crit-bonus modifiers apply after the whole-crit multiplier.
    let bonus_multiplier = (without - 1.0) / (2.0 * ordinary - 1.0);
    let expected = |overcharge: f64| 1.0 + (2.0 * ordinary * overcharge - 1.0) * bonus_multiplier;
    assert!(
        (with_100 - expected(1.2)).abs() < 0.001,
        "100% crit: {with_100} vs {}",
        expected(1.2)
    );
    assert!(
        (with_150 - expected(1.3)).abs() < 0.001,
        "150% crit: {with_150} vs {}",
        expected(1.3)
    );
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn shaman263_real_doom_winds_emits_its_native_area_pulse_on_apply_and_periodically() {
    // SimC76f8118 sc_shaman.cpp8782–8790/12848–12869: actual mode aura owns these ticks.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(263, 3).unwrap();
    engage(&mut run);
    let actor = run.actor;
    cast(&mut run, 384352, actor);
    assert!(crate::auras::has_aura(run.app.world(), actor, 466772));
    let initial: Vec<usize> = run
        .targets
        .iter()
        .map(|&target| damage_hits(&run, 469270, target))
        .collect();
    assert!(
        initial.iter().all(|hits| *hits > 0),
        "actual Doom Winds applies its native area damage immediately"
    );
    for _ in 0..40 {
        run.app.update();
    }
    for (&target, before) in run.targets.iter().zip(initial) {
        assert!(
            damage_hits(&run, 469270, target) > before,
            "native mode period must emit another real area pulse"
        );
    }
}
