//! Native reference90 Firecaster; TC a352b1fa spell_mage.cpp762–779.
//! Only direct133/11366 here; generic periodic crit/321712 remains separate.
use shared::casting::CastState;
use shared::components::{Health, Player};
use shared::protocol::{CombatLogEvent, CombatLogKind};

use super::{Run, inputs, prepare_reference, read_reference};
use crate::auras::ActiveAuras;
use crate::class_progression::KnownSpells;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use crate::spell_info::SpellStore;

const FIRESTARTER: u32 = 205026;
const FIREBALL: u32 = 133;
const PYROBLAST: u32 = 11366;
const OBSERVATION_FRAMES: usize = 400;
const MAX_MISS_ATTEMPTS: usize = 16;
const MAX_OUTCOME_CASTS: usize = 64;

fn load_native90_player() -> Run {
    let (profile, traits) = read_reference(63).expect("prepared native Fire reference");
    assert_eq!(profile.level, "90");
    let identity = inputs::read_actor(&profile).expect("native reference actor/race");
    assert!(
        matches!(
            identity,
            crate::followers::simulator::ParityActor::Player { .. }
        ),
        "run with DPS_PARITY_ACTOR=player; never convert a follower into a player"
    );
    let run = prepare_reference(profile, &traits, 1, identity).expect("native90 player admission");
    assert!(run.app.world().get::<Player>(run.actor).is_some());
    let known = run.app.world().get::<KnownSpells>(run.actor).unwrap();
    for spell in [FIRESTARTER, FIREBALL, PYROBLAST] {
        assert!(
            known.contains(spell),
            "actual native90 book must admit{spell}"
        );
    }
    assert_eq!(native_threshold(&run), 90.0);
    let crit = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .crit_pct;
    assert!(
        crit > 0.0 && crit < 100.0,
        "ordinary native crit must permit both outcomes"
    );
    run
}

fn native_threshold(run: &Run) -> f32 {
    run.app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == FIRESTARTER)
        .expect("actual selected Firestarter aura, never granted")
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount
}

fn wait_for_cast_window(run: &mut Run, spell_id: u32) {
    for _ in 0..OBSERVATION_FRAMES {
        let world = run.app.world();
        let spell = world.resource::<SpellStore>().get(spell_id).unwrap();
        let on_gcd = world
            .get::<crate::spell_cast::SpellHistory>(run.actor)
            .is_some_and(|history| history.on_gcd(spell, crate::spell_cast::history_now(world)));
        if world.get::<CastState>(run.actor).is_none() && !on_gcd {
            return;
        }
        run.app.update();
    }
    panic!("native{spell_id} did not reach an ordinary cast window");
}

fn cast_at_health(run: &mut Run, spell_id: u32, health_pct: f32) -> Option<bool> {
    wait_for_cast_window(run, spell_id);
    let target = run.targets[0];
    let mut health = run.app.world_mut().get_mut::<Health>(target).unwrap();
    // Control only this condition's recipient health, not the spell/actor/clock.
    health.current = health.max * health_pct / 100.0;
    let start = run.app.world().resource::<ObservedCombatEvents>().0.len();
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        spell_id,
        Some(target),
        false,
    )
    .unwrap_or_else(|error| panic!("ordinary native{spell_id} refused: {error:?}"));
    observe_cast_crit(run, spell_id, health_pct, start)
}

fn find_cast_outcome(run: &Run, spell_id: u32, start: usize) -> Option<&CombatLogEvent> {
    run.app.world().resource::<ObservedCombatEvents>().0[start..]
        .iter()
        .find(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(spell_id)
                && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
        })
}

fn observe_cast_crit(run: &mut Run, spell_id: u32, health_pct: f32, start: usize) -> Option<bool> {
    for _ in 0..OBSERVATION_FRAMES {
        run.app.update();
        let Some(event) = find_cast_outcome(run, spell_id, start) else {
            continue;
        };
        if event.kind != CombatLogKind::Damage {
            return None;
        }
        assert!(event.amount > 0);
        eprintln!(
            "FIRESTARTER_NATIVE spell={spell_id} recipient_pct={health_pct} crit={} damage={}",
            event.crit, event.amount
        );
        return Some(event.crit);
    }
    panic!("native{spell_id} produced neither damage nor a miss outcome");
}

fn landed_critical_result(run: &mut Run, spell_id: u32, health_pct: f32) -> bool {
    for _ in 0..MAX_MISS_ATTEMPTS {
        if let Some(crit) = cast_at_health(run, spell_id, health_pct) {
            return crit;
        }
    }
    panic!("native{spell_id} never landed; no crit-policy proof from misses");
}

fn assert_above_and_at_native_threshold(spell_id: u32) {
    let _seed = crate::random::seed_scope(205026);
    let mut run = load_native90_player();
    for health_pct in [100.0, native_threshold(&run)] {
        for _ in 0..2 {
            assert!(
                landed_critical_result(&mut run, spell_id, health_pct),
                "native{spell_id} with selected205026 must crit at{health_pct}% HP"
            );
        }
    }
}

#[test]
#[ignore = "actual native reference90 book/traits, no crit chance or aura grants"]
fn firestarter_native90_fireball_crits_above_and_at_native_threshold() {
    assert_above_and_at_native_threshold(FIREBALL);
}

#[test]
#[ignore = "actual native reference90 book/traits, no crit chance or aura grants"]
fn firestarter_native90_pyroblast_crits_above_and_at_native_threshold() {
    assert_above_and_at_native_threshold(PYROBLAST);
}

fn observe_both_crit_outcomes(run: &mut Run, spell_id: u32, health_pct: f32) -> bool {
    let mut saw_crit = false;
    let mut saw_noncrit = false;
    for _ in 0..MAX_OUTCOME_CASTS {
        let crit = landed_critical_result(run, spell_id, health_pct);
        saw_crit |= crit;
        saw_noncrit |= !crit;
        if saw_crit && saw_noncrit {
            return true;
        }
    }
    false
}

#[test]
#[ignore = "native below-threshold control; ordinary chance retained, not forced noncritical"]
fn firestarter_native90_below_threshold_retains_ordinary_direct_crit() {
    let _seed = crate::random::seed_scope(205026);
    for spell_id in [FIREBALL, PYROBLAST] {
        let mut run = load_native90_player();
        let below = native_threshold(&run) - 1.0;
        assert!(
            observe_both_crit_outcomes(&mut run, spell_id, below),
            "native{spell_id} below Firestarter must preserve both ordinary crit outcomes"
        );
    }
}
