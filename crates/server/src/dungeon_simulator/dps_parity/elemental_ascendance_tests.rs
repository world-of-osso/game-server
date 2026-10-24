//! Native70448 1219480s9/effect8 grants one additional Elemental Overload.
//! Actual Stormkeeper makes the ordinary overload deterministic, without RNG/stat edits.
use super::*;
use crate::simc_apl_native::{Encounter, FightStyle};
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn settle_native_cast(run: &mut Run) {
    for _ in 0..600 {
        let world = run.app.world();
        let active = world.get::<shared::casting::CastState>(run.actor).is_some()
            || world
                .get::<crate::spell_cast::SpellHistory>(run.actor)
                .is_some_and(|history| {
                    history.has_active_gcd(crate::spell_cast::history_now(world))
                });
        if !active {
            return;
        }
        run.app.update();
    }
    panic!("native Elemental cast/GCD did not finish");
}

fn cast_native(run: &mut Run, spell: u32, target: Entity) {
    for _ in 0..600 {
        match crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            spell,
            Some(target),
            false,
        ) {
            Ok(()) => {
                settle_native_cast(run);
                return;
            }
            Err(
                crate::spell_cast::CastFailure::OnGlobalCooldown
                | crate::spell_cast::CastFailure::InProgress,
            ) => run.app.update(),
            Err(reason) => panic!("native spell{spell} rejected: {reason:?}"),
        }
    }
    panic!("native spell{spell} never became ready");
}

fn measure_overloads(ascended: bool, targets: usize, root: u32, overload: u32) -> (usize, usize) {
    let mut run = prepare(262, targets).unwrap();
    let encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: FightStyle::Patchwerk,
    };
    literal_rotation::capture_fixture(run.app.world_mut(), run.actor, encounter).unwrap();
    engage(&mut run);
    settle_native_cast(&mut run);
    let actor = run.actor;
    if ascended {
        cast_native(&mut run, 1219480, actor);
    }
    cast_native(&mut run, 191634, actor);
    let auras = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(actor)
        .unwrap();
    assert!(
        auras
            .iter()
            .any(|aura| aura.spell_id == 168534 && aura.caster == actor),
        "real native Mastery prerequisite"
    );
    assert!(
        auras
            .iter()
            .any(|aura| aura.spell_id == 191634 && aura.caster == actor && aura.charges > 0),
        "actual Stormkeeper must guarantee the ordinary proc"
    );
    if ascended {
        let ascendance = auras
            .iter()
            .find(|aura| aura.spell_id == 1219480 && aura.caster == actor)
            .unwrap();
        assert_eq!(
            ascendance
                .effects
                .iter()
                .find(|effect| effect.index == 8)
                .unwrap()
                .amount,
            1.0
        );
    } else {
        assert!(!auras.iter().any(|aura| aura.spell_id == 1219480));
    }
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        actor,
        root,
        Some(run.targets[0]),
        false,
    )
    .unwrap();
    let mut root_hit_at = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .any(|event| {
            event.source == Some(actor.to_bits())
                && event.spell_id == Some(root)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .then(|| crate::spell_cast::now(run.app.world()));
    for _ in 0..40 {
        run.app.update();
        let now = crate::spell_cast::now(run.app.world());
        let events = &run.app.world().resource::<ObservedCombatEvents>().0;
        if root_hit_at.is_none()
            && events.iter().any(|event| {
                event.source == Some(actor.to_bits())
                    && event.spell_id == Some(root)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
        {
            root_hit_at = Some(now);
        }
        if root_hit_at.is_some_and(|at| now - at < 0.399) {
            assert!(
                !events
                    .iter()
                    .any(|event| event.source == Some(actor.to_bits())
                        && event.spell_id == Some(overload)
                        && event.kind == CombatLogKind::CastSuccess),
                "additional overload must retain the native HIT+400ms boundary"
            );
        }
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let roots = events
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.spell_id == Some(root)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count();
    assert_eq!(
        roots, targets,
        "every ordinary native root target must actually be hit before overload qualification"
    );
    let casts = events
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.spell_id == Some(overload)
                && event.kind == CombatLogKind::CastSuccess
        })
        .count();
    let hits = events
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.spell_id == Some(overload)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count();
    (casts, hits)
}

#[test]
#[ignore = "opt-in native Ascendance additional-overload RED/GREEN pending"]
fn elemental_ascendance_additional_stormkeeper_control_keeps_one_ordinary_overload() {
    let _seed_scope = crate::random::seed_scope(70_448);
    let (casts, hits) = measure_overloads(false, 1, 188196, 45284);
    assert_eq!(casts, 1);
    assert!(hits > 0);
}

#[test]
#[ignore = "opt-in native Ascendance additional-overload RED/GREEN pending"]
fn elemental_ascendance_additional_lightning_has_one_beyond_the_guaranteed_proc() {
    let _seed_scope = crate::random::seed_scope(70_448);
    let (casts, hits) = measure_overloads(true, 1, 188196, 45284);
    assert_eq!(
        casts, 2,
        "1219480effect8 adds one overload beyond Stormkeeper's ordinary proc"
    );
    assert_eq!(hits, 2);
}

#[test]
#[ignore = "opt-in native Ascendance additional-overload RED/GREEN pending"]
fn elemental_ascendance_additional_chain_adds_one_beyond_target_procs() {
    let _seed_scope = crate::random::seed_scope(70_448);
    let (casts, hits) = measure_overloads(true, 3, 188443, 45297);
    assert_eq!(
        casts, 4,
        "three ordinary guaranteed target procs plus one native Ascendance additional cast"
    );
    assert!(
        hits >= 4,
        "actual overload damage is required, not only root cast logs"
    );
}
