//! Native coating procs must produce their sourced damage through real attacks.

#[test]
#[ignore = "requires prepared259 private native DB and player actor"]
fn rogue_kingsbane_earned_poison_hits_stack_native_ramp_until_dot_ends() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(259, 1).unwrap();
    engage(&mut run);
    let started_at = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started_at);
    let mut peak_stacks = 0;
    for _ in 0..800 {
        drive_player(&mut run).unwrap();
        run.app.update();
        if let Some(auras) = run.app.world().get::<crate::auras::ActiveAuras>(run.actor) {
            for aura in auras.iter().filter(|aura| aura.spell_id == 394095) {
                peak_stacks = peak_stacks.max(aura.stacks);
                let dot = run
                    .app
                    .world()
                    .get::<crate::auras::ActiveAuras>(run.targets[0])
                    .unwrap()
                    .iter()
                    .find(|dot| dot.spell_id == 385627 && dot.caster == run.actor)
                    .expect("Kingsbane ramp cannot outlive its owned dot");
                assert_eq!(aura.expires_at, dot.expires_at);
            }
        }
    }
    assert!(
        peak_stacks > 5,
        "earned lethal poison hits must stack native394095, got {peak_stacks}"
    );
    assert!(
        !crate::auras::has_aura(run.app.world(), run.actor, 394095),
        "ramp must end with Kingsbane rather than persist to the next cast"
    );
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let all_ticks: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(385627)
                && event.kind == CombatLogKind::Damage
                && event.periodic
        })
        .collect();
    let ticks: Vec<_> = all_ticks.iter().filter(|event| !event.crit).collect();
    assert!(ticks.len() > 3, "actual Kingsbane ticks required");
    assert!(
        ticks.iter().map(|tick| tick.amount).max().unwrap() > ticks[0].amount * 2,
        "the actual periodic damage must use the earned ramp, not only display stacks: ticks(amount,crit)={:?}, peak_stacks={peak_stacks}",
        all_ticks
            .iter()
            .map(|tick| (tick.amount, tick.crit))
            .collect::<Vec<_>>()
    );
}
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires prepared259 private native DB and player actor"]
fn rogue_envenom_consumes_earned_native_amplifying_poison_threshold() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(259, 1).unwrap();
    engage(&mut run);
    let started_at = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started_at);
    let actor = run.actor;
    let target = run.targets[0];
    let stacks = move |world: &World| {
        world
            .get::<crate::auras::ActiveAuras>(target)
            .unwrap()
            .iter()
            .find(|aura| aura.caster == actor && aura.spell_id == 383414)
            .map_or(0, |aura| aura.stacks)
    };
    for _ in 0..1600 {
        let before = stacks(run.app.world());
        let events_before = run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .len();
        drive_player(&mut run).unwrap();
        run.app.update();
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        let envenom = events[events_before..].iter().any(|event| {
            event.source == Some(actor.to_bits())
                && event.spell_id == Some(32645)
                && event.kind == CombatLogKind::CastSuccess
        });
        if envenom && before >= 10 {
            assert!(
                stacks(run.app.world()) < before,
                "ordinary Envenom must consume the earned native threshold before same-cast poison procs"
            );
            return;
        }
    }
    panic!("rotation must earn Amplifying Poison's native threshold and cast Envenom");
}

#[test]
#[ignore = "requires prepared259 private native DB and player actor"]
fn rogue_instant_coating_produces_its_native_damage_on_real_attacks() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(259, 1).unwrap();
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(315584, run.actor, None),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 315584));
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
    assert!(
        events.iter().any(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id.is_none()
                && event.amount > 0
        }),
        "actual weapon hits must reach the coating boundary"
    );
    assert!(
        events.iter().any(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(315585)
                && event.amount > 0
        }),
        "Instant Poison must dispatch its native effect0 child, independently of Rogue spec"
    );
}

#[test]
#[ignore = "requires prepared259 private native DB and player actor"]
fn assassination_deadly_coating_produces_dot_and_repeat_instant() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(259, 1).unwrap();
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 2823));
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
    let damage = |spell| {
        events
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(run.targets[0].to_bits())
                    && event.kind == CombatLogKind::Damage
                    && event.spell_id == spell
            })
            .map(|event| event.amount)
            .sum::<i32>()
    };
    assert!(
        damage(None) > 0,
        "real weapon swings must reach the coating's proc boundary"
    );
    assert!(
        damage(Some(2818)) > 0,
        "native Deadly Poison must apply and tick from the earned coating procs"
    );
    assert!(
        damage(Some(113780)) > 0,
        "later Deadly procs against the owned ticking poison must also deal instant damage"
    );
}
