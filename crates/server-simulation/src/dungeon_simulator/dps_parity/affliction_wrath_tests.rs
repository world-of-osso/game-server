//! Real native Desperate Souls, native Haunt sources and one-cast lifetime.
use super::*;
use crate::spell_cast::effects::factor_capture::FactorCapture;
use shared::protocol::CombatLogKind;

const HAUNT: u32 = 48181;
const APEX: u32 = 1261992;
const SUMMON: u32 = 1262094;
const WRATH: u32 = 1278047;
const SOUL: u32 = 254505;

fn summons(run: &Run) -> Vec<u64> {
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(SUMMON)
                && event.kind == CombatLogKind::Summon
        })
        .map(|event| event.target.expect("native summon event names its actor"))
        .collect()
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_wrath_native_soul_casts_once_at_haunt_and_dismisses() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 3).unwrap();
    engage(&mut run);
    let owner = run.actor;
    let (selected, haunted, nearby) = (run.targets[0], run.targets[1], run.targets[2]);
    let origin = *run
        .app
        .world()
        .get::<shared::components::Position>(owner)
        .unwrap();
    for (target, distance) in [(selected, 60.0), (haunted, 20.0), (nearby, 21.0)] {
        let mut position = run
            .app
            .world_mut()
            .get_mut::<shared::components::Position>(target)
            .unwrap();
        position.x = origin.x + distance;
        position.y = origin.y;
        position.z = origin.z;
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), owner, HAUNT, Some(haunted), false)
        .unwrap();
    for _ in 0..30 {
        run.app.update();
    }
    assert!(crate::auras::has_aura(run.app.world(), haunted, HAUNT));
    run.app
        .world_mut()
        .entity_mut(owner)
        .insert(crate::networking::CombatTarget(Some(selected)));
    crate::spell_cast::cast_triggered(run.app.world_mut(), owner, SUMMON, Some(owner), 0);
    let soul = run
        .app
        .world_mut()
        .query::<(
            Entity,
            &crate::spell_cast::summon::SummonedCreature,
            &shared::components::Npc,
        )>()
        .iter(run.app.world())
        .find(|(_, summon, npc)| summon.owner == owner && npc.template_id == SOUL)
        .map(|(entity, _, _)| entity)
        .expect("native1262094 must create actual254505");
    let expected = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(owner)
        .unwrap()
        .spell_power;
    run.app
        .world_mut()
        .insert_resource(FactorCapture::new(soul, haunted, WRATH, "wrath", 1));
    for _ in 0..40 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let hits: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(soul.to_bits())
                && event.spell_id == Some(WRATH)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .collect();
    // SimC76f8118 pets2516–2563: actual Haunt recipient, native AoE,
    // exactly one Wrath per soul, then dismissal rather than repeated casts.
    assert_eq!(
        hits.len(),
        2,
        "one real Wrath must hit the Haunt-centered native pack"
    );
    for target in [haunted, nearby] {
        assert_eq!(
            hits.iter()
                .filter(|hit| hit.target == Some(target.to_bits()))
                .count(),
            1
        );
    }
    assert!(
        run.app.world().get_entity(soul).is_err(),
        "the one-cast soul must dismiss"
    );
    let capture = run.app.world().resource::<FactorCapture>().export();
    let formula = capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event"] == "direct_amount" && event["spell_id"] == WRATH)
        .unwrap();
    assert!(
        (formula["factors"]["s_power"].as_f64().unwrap() - f64::from(expected)).abs() < 1.0,
        "the actual soul damage must inherit native full owner SP"
    );
}

#[test]
#[ignore = "requires private265 profile and native70448 world DB"]
fn affliction_wrath_real_haunt_sources_roll_selected_native_rppm() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 1).unwrap();
    engage(&mut run);
    let (caster, target) = (run.actor, run.targets[0]);
    assert!(crate::auras::has_aura(run.app.world(), caster, APEX));
    for _ in 0..10 {
        crate::spell_cast::creature_cast(run.app.world_mut(), caster, HAUNT, Some(target), false)
            .unwrap();
        for _ in 0..360 {
            run.app.update();
        }
    }
    for _ in 0..40 {
        run.app.update();
    }
    let earned = summons(&run);
    assert!(
        !earned.is_empty(),
        "actual Haunt impacts/pulses must earn native Desperate Souls"
    );
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for &soul in &earned {
        assert_eq!(
            events
                .iter()
                .filter(|event| event.source == Some(soul)
                    && event.spell_id == Some(WRATH)
                    && event.target == Some(target.to_bits())
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0)
                .count(),
            1,
            "each earned soul must land exactly one actual Wrath"
        );
        assert!(run.app.world().get_entity(Entity::from_bits(soul)).is_err());
    }
    crate::auras::remove_auras(
        run.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Consumed,
        |aura| aura.caster == caster && aura.spell_id == APEX,
    );
    crate::spell_cast::creature_cast(run.app.world_mut(), caster, HAUNT, Some(target), false)
        .unwrap();
    for _ in 0..300 {
        run.app.update();
    }
    assert!(crate::auras::has_aura(run.app.world(), target, HAUNT));
    assert_eq!(
        summons(&run),
        earned,
        "unselected rank3 cannot roll from fresh real Haunt sources"
    );
}
