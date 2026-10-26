//! Actual native applications and ticks for the shared Hellcaller counter lifecycle.
use super::*;
use crate::auras::{ActiveAuras, AuraApplication, AuraSource};
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

const WITHER: u32 = 445474;
const UA: u32 = 1259790;
const COLLAPSE: u32 = 445731;
const BLACKENED: u32 = 445736;
const SEEDS: u32 = 440055;

fn owned(run: &Run, target: Entity, spell: u32) -> Option<(u8, Option<f64>)> {
    run.app
        .world()
        .get::<ActiveAuras>(target)?
        .iter()
        .find(|aura| aura.spell_id == spell && aura.caster == run.actor)
        .map(|aura| (aura.stacks, aura.expires_at))
}

fn apply(run: &mut Run, target: Entity, spell: u32) {
    crate::auras::apply_aura(
        run.app.world_mut(),
        &AuraApplication {
            caster: run.actor,
            target,
            spell_id: spell,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
    assert!(
        owned(run, target, spell).is_some(),
        "actual native aura{spell} must be stored"
    );
}

fn native_value(run: &Run, spell: u32, index: u8) -> f32 {
    // SimC reads effectN(n).base_value(), not a damage-modified spell amount.
    run.app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(spell)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == index)
        .unwrap()
        .base_points
}

fn apply_counter_state(run: &mut Run, stacks: u8) {
    // Explicit native SPELLVALUE_AURA_STACK precondition, not an invented producer.
    let mut spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(WITHER)
        .unwrap()
        .clone();
    spell.cast_aura_stacks = Some(stacks);
    assert!(crate::spell_cast::launch_triggered_spell(
        run.app.world_mut(),
        run.actor,
        &spell,
        Some(run.targets[0]),
        1
    ));
    for _ in 0..10 {
        if owned(run, run.targets[0], WITHER).is_some() {
            break;
        }
        run.app.update();
    }
    assert_eq!(owned(run, run.targets[0], WITHER).unwrap().0, stacks);
}

fn blackened_hits(run: &Run) -> usize {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(BLACKENED)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count()
}

#[test]
#[ignore = "requires private265 native player inputs"]
fn hellcaller_actual_ua_application_and_refresh_add_owned_wither_without_refreshing_it() {
    // SimC76f8118 actions1544–1565,5534–5546: actual UA application is a producer.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 1).unwrap();
    engage(&mut run);
    let target = run.targets[0];
    apply(&mut run, target, WITHER);
    let before = owned(&run, target, WITHER).unwrap();
    apply(&mut run, target, UA);
    let first = owned(&run, target, WITHER).unwrap();
    assert!(
        (before.0 + 1..=before.0 + 2).contains(&first.0),
        "UA must add its native base counter, with at most one Bleakheart bonus: {before:?}→{first:?}"
    );
    assert_eq!(first.1, before.1, "a counter grant must not refresh Wither");
    apply(&mut run, target, UA);
    let second = owned(&run, target, WITHER).unwrap();
    assert!(
        (first.0 + 1..=first.0 + 2).contains(&second.0),
        "an actual reapplication must also produce a counter: {first:?}→{second:?}"
    );
    assert_eq!(second.1, before.1);
}

#[test]
#[ignore = "requires private267 native player inputs"]
fn hellcaller_actual_destruction_hits_add_owned_wither_counters() {
    // Native triggered impacts exercise the hit boundary, including background
    // copies. Resource payment is separately covered by Incinerate/pool tests.
    // SimC76f8118 actions4328–4331/4807–4810: CB/Shadowburn impact producers.
    for spell_id in [116858, 17877] {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_literal(267, 1).unwrap();
        engage(&mut run);
        let target = run.targets[0];
        if spell_id == 17877 {
            let mut health = run
                .app
                .world_mut()
                .get_mut::<shared::components::Health>(target)
                .unwrap();
            health.current = health.max * 0.19;
        }
        apply(&mut run, target, WITHER);
        let before = owned(&run, target, WITHER).unwrap();
        let spell = run
            .app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(spell_id)
            .unwrap()
            .clone();
        assert!(crate::spell_cast::launch_triggered_spell(
            run.app.world_mut(),
            run.actor,
            &spell,
            Some(target),
            1
        ));
        let mut landed = false;
        for _ in 0..60 {
            run.app.update();
            landed = run
                .app
                .world()
                .resource::<ObservedCombatEvents>()
                .0
                .iter()
                .any(|event| {
                    event.source == Some(run.actor.to_bits())
                        && event.spell_id == Some(spell_id)
                        && event.kind == CombatLogKind::Damage
                        && event.amount > 0
                });
            if landed {
                break;
            }
        }
        assert!(landed, "actual native impact{spell_id} required");
        let after = owned(&run, target, WITHER).unwrap();
        assert!(
            (before.0 + 1..=before.0 + 2).contains(&after.0),
            "native impact{spell_id} must produce its base counter: {before:?}→{after:?}"
        );
        assert!(after.1 <= before.1, "counter grant must not extend Wither");
    }
}

#[test]
#[ignore = "requires private265 native player inputs"]
fn hellcaller_ua_without_owned_wither_creates_no_counter_or_collapse() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 1).unwrap();
    engage(&mut run);
    let target = run.targets[0];
    apply(&mut run, target, UA);
    assert!(owned(&run, target, WITHER).is_none());
    assert!(owned(&run, target, COLLAPSE).is_none());
}

#[test]
#[ignore = "requires private265 native player inputs"]
fn hellcaller_malevolence_adds_native_six_only_to_owned_wither_and_grants_its_buff() {
    // SimC76f8118 actions1295–1317,5534–5611: native effect0 grants6;
    // Malevolence does not independently roll the ordinary collapse chance.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 3).unwrap();
    engage(&mut run);
    let targets = run.targets.clone();
    for target in &targets[..2] {
        apply(&mut run, *target, WITHER);
    }
    let expected = 1 + native_value(&run, 430014, 0) as u8;
    assert_eq!(expected, 7, "native70448 Malevolence fixture");
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(430014, run.actor, Some(targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    for target in &targets[..2] {
        assert_eq!(
            owned(&run, *target, WITHER).unwrap().0,
            expected,
            "actual Malevolence must grant the native stack amount"
        );
        assert!(
            owned(&run, *target, COLLAPSE).is_none(),
            "7stacks/full health must not collapse from Malevolence alone"
        );
    }
    assert!(owned(&run, targets[2], WITHER).is_none());
    assert!(
        owned(&run, run.actor, 442726).is_some(),
        "actual native Malevolence buff required"
    );
}

#[test]
#[ignore = "requires private265 native player inputs"]
fn hellcaller_seed_stack_gate_starts_a_permanent_counter_and_native_blackened_ticks() {
    // SimC76f8118 actions5581–5606; warlock.cpp103–126: native period,
    // zero-duration frozen-stack counter, not the unused native2s buff lifetime.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 1).unwrap();
    engage(&mut run);
    let target = run.targets[0];
    let threshold = native_value(&run, SEEDS, 0) as u8;
    assert_eq!(threshold, 8, "native Seeds threshold");
    apply_counter_state(&mut run, threshold - 1);
    let expiry = owned(&run, target, WITHER).unwrap().1;
    apply(&mut run, target, UA);
    let initial = owned(&run, target, WITHER).unwrap().0;
    assert!(initial >= threshold);
    let collapse =
        owned(&run, target, COLLAPSE).expect("Seeds threshold must start native collapse");
    assert_eq!(
        collapse,
        (initial, None),
        "collapse budget equals real stacks and is permanent until consumed"
    );
    for _ in 0..65 {
        run.app.update();
    }
    let hits = blackened_hits(&run);
    assert_eq!(
        hits, 3,
        "native1000ms period must continue beyond native2s aura duration"
    );
    assert_eq!(
        owned(&run, target, WITHER).unwrap(),
        (initial - hits as u8, expiry)
    );
    assert_eq!(
        owned(&run, target, COLLAPSE).unwrap().0,
        initial - hits as u8
    );
    crate::auras::remove_auras(
        run.app.world_mut(),
        target,
        crate::auras::RemoveReason::Consumed,
        |aura| aura.spell_id == WITHER,
    );
    for _ in 0..25 {
        run.app.update();
    }
    assert_eq!(
        blackened_hits(&run),
        hits,
        "no ghost Blackened damage after Wither removal"
    );
    assert!(owned(&run, target, COLLAPSE).is_none());
}

#[test]
#[ignore = "requires private265 native player inputs"]
fn hellcaller_seed_health_gate_collapses_to_one_then_stops() {
    // SimC76f8118 actions1232–1272,5579–5606: health gate, one stack
    // consumed per real Blackened hit, expire collapse when Wither reaches1.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(265, 1).unwrap();
    engage(&mut run);
    let target = run.targets[0];
    let percentage = native_value(&run, SEEDS, 1);
    assert_eq!(percentage, 20.0);
    {
        let mut health = run
            .app
            .world_mut()
            .get_mut::<shared::components::Health>(target)
            .unwrap();
        health.current = health.max * (percentage - 1.0) / 100.0;
    }
    apply(&mut run, target, WITHER);
    let expiry = owned(&run, target, WITHER).unwrap().1;
    apply(&mut run, target, UA);
    let initial = owned(&run, target, WITHER).unwrap().0;
    assert!((2..=3).contains(&initial));
    assert!(
        owned(&run, target, COLLAPSE).is_some(),
        "low-health Seeds gate must start collapse"
    );
    for _ in 0..70 {
        run.app.update();
    }
    assert_eq!(blackened_hits(&run), usize::from(initial - 1));
    assert_eq!(owned(&run, target, WITHER).unwrap(), (1, expiry));
    assert!(owned(&run, target, COLLAPSE).is_none());
}
