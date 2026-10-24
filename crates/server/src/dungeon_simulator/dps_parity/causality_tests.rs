//! TC a352b1fa spell_evoker.cpp215–239: real Disintegrate ticks, earned passive,
//! ordinary empower recovery. No history writes or invented recovery durations.
use super::*;
use crate::auras::{ActiveAuras, AuraApplication, AuraSource, RemoveReason};
use crate::simc_apl_native::{Encounter, FightStyle};
use crate::spell_cast::test_harness::ObservedCombatEvents;
use crate::spell_cast::{CastFailure, SpellHistory};
use crate::spell_info::SpellStore;
use shared::casting::CastState;
use shared::protocol::CombatLogKind;

const CAUSALITY: u32 = 375777;
const DISINTEGRATE: u32 = 356995;
const FIRE_BREATH: u32 = 357208;
const ETERNITY_SURGE: u32 = 359073;
const DRAGONRAGE: u32 = 375087;
const WATCHED: &[u32] = &[FIRE_BREATH, ETERNITY_SURGE, DRAGONRAGE];

struct Sample {
    at: f64,
    remaining: BTreeMap<u32, f64>,
    ticks: usize,
}

fn causal_amount(run: &Run) -> f64 {
    let aura = run
        .app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == CAUSALITY && aura.caster == run.actor)
        .expect("selected native Causality must actually be active");
    let effect = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap();
    let milliseconds = effect.amount as i32; // TC GetAmountAsInt, not effect1.
    assert!(milliseconds < 0);
    -f64::from(milliseconds) / 1000.0
}

fn start_native(run: &mut Run, spell: u32, stage: Option<u8>) {
    for _ in 0..TICKS {
        let actor = run.actor;
        let target = if spell == DISINTEGRATE {
            run.targets[0]
        } else {
            actor
        };
        let result = if let Some(stage) = stage {
            crate::spell_cast::creature_cast_empowered(
                run.app.world_mut(),
                actor,
                spell,
                Some(target),
                stage,
            )
        } else {
            crate::spell_cast::creature_cast(run.app.world_mut(), actor, spell, Some(target), false)
        };
        match result {
            Ok(()) => return,
            Err(CastFailure::OnGlobalCooldown | CastFailure::InProgress) => run.app.update(),
            Err(reason) => panic!("ordinary native spell{spell}: {reason:?}"),
        }
    }
    panic!("ordinary native spell{spell} never became available");
}

fn finish_channel(run: &mut Run) {
    for _ in 0..TICKS {
        run.app.update();
        if run.app.world().get::<CastState>(run.actor).is_none() {
            return;
        }
    }
    panic!("native cast never completed");
}

fn sample(run: &Run) -> Sample {
    let world = run.app.world();
    let remaining = WATCHED
        .iter()
        .map(|&id| {
            let spell = world.resource::<SpellStore>().get(id).unwrap();
            let seconds =
                crate::spell_cast::native_apl::cooldown_value(world, run.actor, spell, "remains")
                    .unwrap();
            (id, seconds)
        })
        .collect();
    let ticks = world
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(DISINTEGRATE)
                && event.periodic
                && event.kind == CombatLogKind::Damage
        })
        .count();
    Sample {
        at: crate::spell_cast::history_now(world),
        remaining,
        ticks,
    }
}

fn prepared() -> Run {
    let mut run = prepare(1467, 1).unwrap();
    causal_amount(&run);
    let encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: FightStyle::Patchwerk,
    };
    literal_rotation::capture_fixture(run.app.world_mut(), run.actor, encounter).unwrap();
    for spell in [FIRE_BREATH, ETERNITY_SURGE] {
        start_native(&mut run, spell, Some(1));
        finish_channel(&mut run);
    }
    start_native(&mut run, DRAGONRAGE, None);
    assert!(run.app.world().get::<SpellHistory>(run.actor).is_some());
    for (&id, &remaining) in &sample(&run).remaining {
        assert!(
            remaining > 0.0,
            "native spell{id} must really be recovering"
        );
    }
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    run
}

fn first_tick(run: &mut Run) {
    for _ in 0..TICKS {
        if sample(run).ticks > 0 {
            assert!(run.app.world().get::<CastState>(run.actor).is_some());
            return;
        }
        run.app.update();
    }
    panic!("ordinary Disintegrate did not produce its first native tick");
}

fn assert_reduction(before: &Sample, after: &Sample, reduction: f64) {
    let elapsed = after.at - before.at;
    for &id in WATCHED {
        let causal = if id == DRAGONRAGE { 0.0 } else { reduction };
        let expected = (before.remaining[&id] - elapsed - causal).max(0.0);
        let actual = after.remaining[&id];
        assert!(
            (actual - expected).abs() <= 0.002,
            "native{id}: before{}, after{actual}, elapsed{elapsed}, Causality{causal}, expected{expected}",
            before.remaining[&id],
        );
    }
}

fn remove_causality(run: &mut Run) {
    crate::auras::remove_auras(
        run.app.world_mut(),
        run.actor,
        RemoveReason::Unlearned,
        |aura| aura.spell_id == CAUSALITY,
    );
    assert!(!crate::auras::has_aura(
        run.app.world(),
        run.actor,
        CAUSALITY
    ));
}

#[test]
#[ignore = "requires prepared1467 inputs; native TC Causality RED"]
fn native_causality_disintegrate_tick_zero_and_later_ticks_reduce_both_empowers_only() {
    let mut run = prepared();
    let seconds = causal_amount(&run);
    let before = sample(&run);
    start_native(&mut run, DISINTEGRATE, None);
    first_tick(&mut run);
    let first = sample(&run);
    assert_eq!(first.ticks, 1, "actual extra-initial-period tick");
    assert_reduction(&before, &first, seconds);
    finish_channel(&mut run);
    let after = sample(&run);
    assert!(after.ticks > first.ticks);
    assert_reduction(&before, &after, seconds * after.ticks as f64);
}

#[test]
#[ignore = "requires prepared1467 inputs; native TC Causality absence control"]
fn native_causality_without_passive_leaves_ordinary_recovery_unchanged() {
    let mut run = prepared();
    remove_causality(&mut run);
    let before = sample(&run);
    start_native(&mut run, DISINTEGRATE, None);
    finish_channel(&mut run);
    let after = sample(&run);
    assert!(after.ticks > 0);
    assert!(!crate::auras::has_aura(
        run.app.world(),
        run.actor,
        CAUSALITY
    ));
    assert_reduction(&before, &after, 0.0);
}

#[test]
#[ignore = "requires prepared1467 inputs; native TC Causality removal boundary"]
fn native_causality_removal_during_channel_stops_later_tick_reduction() {
    let mut run = prepared();
    let seconds = causal_amount(&run);
    let before = sample(&run);
    start_native(&mut run, DISINTEGRATE, None);
    first_tick(&mut run);
    let first = sample(&run);
    assert_reduction(&before, &first, seconds * first.ticks as f64);
    remove_causality(&mut run);
    finish_channel(&mut run);
    let after = sample(&run);
    assert!(after.ticks > first.ticks);
    assert!(!crate::auras::has_aura(
        run.app.world(),
        run.actor,
        CAUSALITY
    ));
    assert_reduction(&before, &after, seconds * first.ticks as f64);
}

#[test]
#[ignore = "requires prepared1467 inputs; TC Load snapshot and next native channel"]
fn native_causality_late_passive_restoration_only_enables_the_next_channel() {
    let mut run = prepared();
    let seconds = causal_amount(&run);
    remove_causality(&mut run);
    let before = sample(&run);
    start_native(&mut run, DISINTEGRATE, None);
    first_tick(&mut run);
    // Restore the already-selected passive through its native application path.
    crate::auras::apply_aura(
        run.app.world_mut(),
        &AuraApplication {
            caster: run.actor,
            target: run.actor,
            spell_id: CAUSALITY,
            effect_mask: u32::MAX,
            source: AuraSource::KnownSpell,
        },
    );
    assert_eq!(causal_amount(&run), seconds);
    finish_channel(&mut run);
    let unloaded = sample(&run);
    assert_reduction(&before, &unloaded, 0.0);
    start_native(&mut run, DISINTEGRATE, None);
    finish_channel(&mut run);
    let loaded = sample(&run);
    let new_ticks = loaded.ticks - unloaded.ticks;
    assert!(new_ticks > 0);
    assert_reduction(&unloaded, &loaded, seconds * new_ticks as f64);
}
