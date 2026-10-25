//! Real admitted WDP casts; no child/aura/power/cooldown fixture grants.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::{CombatLogEvent, CombatLogKind};
use std::collections::{BTreeMap, BTreeSet};

type Run = super::super::Run;
const ROOT: u32 = 152175;
const SINGLE: u32 = 451767;
const AREA: u32 = 158221;

fn prepare_first_cast(targets: usize) -> Run {
    let mut run = super::super::prepare_measurement(269, targets).unwrap();
    let start = run.app.world().resource::<Time>().elapsed();
    super::super::install_fixture_lifetimes(&mut run, start);
    super::super::engage(&mut run);
    let mut rotation = (run.spec_driver.take().unwrap() as Box<dyn std::any::Any>).downcast::<Rotation>().unwrap();
    for _ in 0..6000 {
        rotation
            .drive(run.app.world_mut(), run.actor, &run.targets)
            .unwrap();
        if events(&run, ROOT)
            .iter()
            .any(|event| event.kind == CombatLogKind::CastSuccess)
        {
            return run;
        }
        run.app.update();
    }
    panic!("literal source must actually cast native WDP within its300s fixture");
}

fn events(run: &Run, spell: u32) -> Vec<CombatLogEvent> {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| event.source == Some(run.actor.to_bits()) && event.spell_id == Some(spell))
        .cloned()
        .collect()
}

fn deliveries(run: &Run, spell: u32) -> Vec<CombatLogEvent> {
    events(run, spell)
        .into_iter()
        .filter(|event| matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_)))
        .collect()
}

fn read_root_aura(run: &Run) -> crate::auras::AuraInstance {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == ROOT && aura.caster == run.actor)
        .unwrap()
        .clone()
}

#[test]
#[ignore = "native admitted WDP emits one immediate single-target child at its actual primary"]
fn literal_ww_native_wdp_children_cast_once_at_primary_not_the_self_aura_unit() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_first_cast(1);
    let root_events = events(&run, ROOT);
    let root = root_events
        .iter()
        .find(|event| event.kind == CombatLogKind::CastSuccess)
        .unwrap();
    assert_eq!(root.target, Some(run.targets[0].to_bits()));
    assert!(
        root_events
            .iter()
            .any(|event| event.kind == CombatLogKind::AuraApplied
                && event.target == Some(run.actor.to_bits())),
        "native WDP aura really lands on its caster"
    );
    eprintln!(
        "NATIVE_WDP_CAST_CONTEXT {}",
        json!({"actor":run.actor.to_bits(),
        "primary":root.target,"aura_unit":run.actor.to_bits(),"time":crate::spell_cast::now(run.app.world())})
    );
    let immediate = deliveries(&run, SINGLE);
    assert_eq!(
        immediate.len(),
        1,
        "admitted152175 must emit exactly one native451767 delivery immediately"
    );
    assert_eq!(
        immediate[0].target, root.target,
        "primary target must not become the self aura unit"
    );
    assert!(
        immediate[0].kind == CombatLogKind::Damage && immediate[0].amount > 0,
        "native single-target child must actually damage this concrete seeded target"
    );
    assert!(
        deliveries(&run, AREA).is_empty(),
        "the cast relationship must not manually emit an area tick before native aura processing"
    );
    for _ in 0..24 {
        run.app.update();
    }
    assert_eq!(
        deliveries(&run, SINGLE).len(),
        1,
        "periodic ticks must never repeat the single-target child"
    );
}

#[test]
#[ignore = "native WDP periodic dummy dispatches its area child at each native250ms tick through expiry"]
fn literal_ww_native_wdp_children_follow_all_native_aura_ticks_and_three_recipients() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_first_cast(3);
    let aura = read_root_aura(&run);
    assert_eq!(
        aura.duration_ms, 1000,
        "native root lifetime, not the enabler override"
    );
    let effect = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap();
    assert_eq!(
        effect.period_ms, 250,
        "native aura226 period is authoritative"
    );
    let cast_at = crate::spell_cast::now(run.app.world());
    assert!(
        deliveries(&run, AREA).is_empty(),
        "native aura system has not advanced yet"
    );
    let root = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(ROOT)
        .unwrap();
    let attributes = &root.misc.as_ref().unwrap().attributes;
    const EXTRA_INITIAL_PERIOD: u32 = 0x200;
    let initial_tick = attributes[5] & EXTRA_INITIAL_PERIOD != 0;
    assert!(
        initial_tick,
        "CURRENT DB2 explicitly authors an initial native tick"
    );
    let first_frame_ms = run.app.world().resource::<Time>().delta().as_millis() as u32;
    let mut observed = BTreeMap::<u32, Vec<CombatLogEvent>>::new();
    let mut count = 0;
    for _ in 0..24 {
        run.app.update();
        let elapsed = ((crate::spell_cast::now(run.app.world()) - cast_at) * 1000.0).round() as u32;
        let delivered = deliveries(&run, AREA);
        if delivered.len() > count {
            observed.insert(elapsed, delivered[count..].to_vec());
        }
        count = delivered.len();
    }
    let mut expected: Vec<u32> = (1..=aura.duration_ms / effect.period_ms)
        .map(|tick| tick * effect.period_ms)
        .collect();
    if initial_tick {
        expected.insert(0, first_frame_ms);
    }
    assert_eq!(
        observed.keys().copied().collect::<Vec<_>>(),
        expected,
        "each actual native periodic tick must emit158221, including the final expiry tick"
    );
    let targets: BTreeSet<u64> = run.targets.iter().map(|target| target.to_bits()).collect();
    for (elapsed, rows) in observed {
        let actual: BTreeSet<u64> = rows.iter().filter_map(|event| event.target).collect();
        assert_eq!(
            rows.len(),
            3,
            "one delivery per eligible native target at{elapsed}ms"
        );
        assert_eq!(
            actual, targets,
            "native child must choose all three real eligible targets"
        );
    }
    assert!(
        !crate::auras::has_aura(run.app.world(), run.actor, ROOT),
        "root aura must expire normally"
    );
    assert!(
        deliveries(&run, AREA)
            .iter()
            .any(|event| event.kind == CombatLogKind::Damage && event.amount > 0),
        "native coefficients must deliver actual damage, not a script-only counter"
    );
}
