//! FIRE_PLAYER_METEORITE_NATIVE_DATA_BRIDGE: actual reference Sunfury90 traits.
//! Native1260673 names Fireball, not SimC76f8118's Pyroblast predicate.
use shared::casting::CastState;
use shared::protocol::CombatLogKind;

use super::{Run, inputs, prepare_reference, read_reference};
use crate::auras::has_aura;
use crate::class_progression::KnownSpells;
use crate::followers::simulator::ParityActor;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use crate::spell_info::SpellStore;

const FIREBALL: u32 = 133;
const FIRE_BLAST: u32 = 108853;
const PYROBLAST: u32 = 11366;
const PYROCOSM: u32 = 1260673;
const GLORIOUS: u32 = 449394;
const MISSILE: u32 = 449560;
const IMPACT: u32 = 449569;
const SPHERE_TALENT: u32 = 448601;
const SPHERE_DRIVER: u32 = 449849;
const SPHERE_BUFF: u32 = 448604;
const GLORIOUS_MARKER: u32 = 451073;
const MAX_CAST_WAIT_FRAMES: usize = 400;

fn load_and_spawn_sunfury(player_actor: bool) -> Run {
    load_and_spawn_sunfury_targets(player_actor, 1)
}

fn load_and_spawn_sunfury_targets(player_actor: bool, target_count: usize) -> Run {
    let (profile, traits) = read_reference(63).expect("prepared exact reference Sunfury63 profile");
    assert_eq!(profile.level, "90");
    let identity = if player_actor {
        ParityActor::Player {
            race: inputs::read_reference_race(&profile.race).unwrap(),
        }
    } else {
        ParityActor::Follower
    };
    let run = prepare_reference(profile, &traits, target_count, identity)
        .expect("native reference admission");
    let known = run.app.world().get::<KnownSpells>(run.actor).unwrap();
    for spell in [FIREBALL, PYROBLAST, PYROCOSM, GLORIOUS] {
        assert!(
            known.contains(spell),
            "actual exported native trait/book must admit{spell}"
        );
    }
    assert!(has_aura(run.app.world(), run.actor, PYROCOSM));
    assert!(has_aura(run.app.world(), run.actor, GLORIOUS));
    run
}

fn native_missile_wait_frames(run: &Run) -> usize {
    let missile = run
        .app
        .world()
        .resource::<SpellStore>()
        .get(MISSILE)
        .unwrap();
    let misc = missile.misc.as_ref().unwrap();
    assert_ne!(
        misc.attributes[9] & 0x10,
        0,
        "native449560 speed is a delay in seconds"
    );
    let frame = match run.app.world().resource::<bevy::time::TimeUpdateStrategy>() {
        bevy::time::TimeUpdateStrategy::ManualDuration(frame) => frame.as_secs_f64(),
        _ => panic!("native qualification needs the fixture's deterministic clock"),
    };
    (f64::from(misc.speed) / frame).ceil() as usize + 2
}

fn wait_for_native_cast_window(run: &mut Run, spell_id: u32) {
    for _ in 0..MAX_CAST_WAIT_FRAMES {
        let world = run.app.world();
        let spell = world.resource::<SpellStore>().get(spell_id).unwrap();
        let gcd = world
            .get::<crate::spell_cast::SpellHistory>(run.actor)
            .is_some_and(|history| history.on_gcd(spell, crate::spell_cast::history_now(world)));
        if world.get::<CastState>(run.actor).is_none() && !gcd {
            return;
        }
        run.app.update();
    }
    panic!("native caster did not reach an ordinary cast window for{spell_id}");
}

fn cast_and_observe_hit(run: &mut Run, spell_id: u32) -> bool {
    wait_for_native_cast_window(run, spell_id);
    let first = run.app.world().resource::<ObservedCombatEvents>().0.len();
    let actor = run.actor;
    let target = run.targets[0];
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, spell_id, Some(target), false)
        .unwrap_or_else(|error| panic!("ordinary native{spell_id} admission failed: {error:?}"));
    for _ in 0..MAX_CAST_WAIT_FRAMES {
        run.app.update();
        let hit = run.app.world().resource::<ObservedCombatEvents>().0[first..]
            .iter()
            .find(|event| {
                event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(spell_id)
                    && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
            });
        if let Some(hit) = hit {
            return hit.kind == CombatLogKind::Damage && hit.amount > 0;
        }
    }
    panic!("ordinary native{spell_id} produced neither damage nor a miss outcome");
}

fn has_owner_meteorite_damage(run: &Run) -> bool {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .any(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id == Some(IMPACT)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
}

fn assert_fireball_produces_meteorite(player_actor: bool) {
    let _seed = crate::random::seed_scope(1260673);
    let mut run = load_and_spawn_sunfury(player_actor);
    let flight_frames = native_missile_wait_frames(&run);
    let mut positive_fireballs = 0;
    for _ in 0..64 {
        positive_fireballs += usize::from(cast_and_observe_hit(&mut run, FIREBALL));
        for _ in 0..flight_frames {
            run.app.update();
        }
        if has_owner_meteorite_damage(&run) {
            assert!(positive_fireballs > 0);
            return;
        }
    }
    assert!(
        positive_fireballs > 0,
        "native Fireball must actually land before testing its proc"
    );
    panic!(
        "{positive_fireballs} actual native Fireballs never produced owner449569; selected1260673 effect1=20%, no proc chance, buff or learned spell was injected"
    );
}

fn owner_impact_counts(run: &Run) -> Vec<usize> {
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    run.targets
        .iter()
        .map(|target| {
            events
                .iter()
                .filter(|event| {
                    event.source == Some(run.actor.to_bits())
                        && event.target == Some(target.to_bits())
                        && event.spell_id == Some(IMPACT)
                        && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
                })
                .count()
        })
        .collect()
}

fn launch_native_destination_missile(run: &mut Run) -> f64 {
    let target = run
        .app
        .world()
        .get::<shared::components::Position>(run.targets[0])
        .unwrap();
    let position = bevy::prelude::Vec3::new(target.x, target.y, target.z);
    let launched_at = crate::spell_cast::now(run.app.world());
    // Isolate the encoded missile→impact edge, not natural Pyrocosm admission.
    // No learned spell, aura, chance or coefficient is injected.
    crate::spell_cast::cast_triggered_at_destination(
        run.app.world_mut(),
        run.actor,
        MISSILE,
        position,
        1,
    );
    launched_at
}

fn assert_native_destination_missile_lands_once(target_count: usize) {
    let _seed = crate::random::seed_scope(449560);
    let mut run = load_and_spawn_sunfury_targets(true, target_count);
    let delay = f64::from(
        run.app
            .world()
            .resource::<SpellStore>()
            .get(MISSILE)
            .unwrap()
            .misc
            .as_ref()
            .unwrap()
            .speed,
    );
    let launched_at = launch_native_destination_missile(&mut run);
    assert_eq!(owner_impact_counts(&run), vec![0; target_count]);
    for _ in 0..native_missile_wait_frames(&run) {
        run.app.update();
        let elapsed = crate::spell_cast::now(run.app.world()) - launched_at;
        if elapsed + f64::EPSILON < delay {
            assert_eq!(
                owner_impact_counts(&run),
                vec![0; target_count],
                "native449560 must not land before its structured delay; elapsed={elapsed}",
            );
        }
    }
    assert_eq!(
        owner_impact_counts(&run),
        vec![1; target_count],
        "one destination missile must dispatch one child, not one AoE child per selected victim",
    );
    assert!(has_owner_meteorite_damage(&run));
}

fn sphere_stacks(run: &Run) -> u8 {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .and_then(|auras| auras.iter().find(|aura| aura.spell_id == SPHERE_BUFF))
        .map_or(0, |aura| aura.stacks)
}

fn advance_native_sphere_tick(run: &mut Run) {
    let spell = run
        .app
        .world()
        .resource::<SpellStore>()
        .get(SPHERE_DRIVER)
        .unwrap();
    let period = spell
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .aura_period_ms;
    assert_eq!(period, 8000, "approved native OOC driver period");
    let frame = match run.app.world().resource::<bevy::time::TimeUpdateStrategy>() {
        bevy::time::TimeUpdateStrategy::ManualDuration(frame) => frame.as_secs_f64(),
        _ => panic!("OOC source fixture requires its deterministic clock"),
    };
    let frames = ((f64::from(period) / 1000.0) / frame).ceil() as usize + 2;
    for _ in 0..frames {
        run.app.update();
    }
}

fn advance_native_missile_flight(run: &mut Run) {
    for _ in 0..native_missile_wait_frames(run) {
        run.app.update();
    }
}

fn read_fire_blast_start(
    run: &mut Run,
) -> Result<Option<bevy::prelude::Entity>, crate::spell_cast::CastFailure> {
    let spell = run
        .app
        .world()
        .resource::<SpellStore>()
        .get(FIRE_BLAST)
        .unwrap()
        .clone();
    crate::spell_cast::read_native_cast_start(
        run.app.world_mut(),
        run.actor,
        &spell,
        run.targets[0],
    )
}

fn exhaust_native_fire_blast(run: &mut Run) {
    for _ in 0..MAX_CAST_WAIT_FRAMES {
        match read_fire_blast_start(run) {
            Err(crate::spell_cast::CastFailure::NoCharges) => return,
            Ok(_) => assert!(cast_and_observe_hit(run, FIRE_BLAST)),
            Err(error) => panic!("ordinary Fire Blast before exhaustion: {error:?}"),
        }
    }
    panic!("ordinary Fire Blast never exhausted its native charges");
}

fn observe_native_fire_blast_recovery(target_count: usize, launch_missile: bool) -> f64 {
    let _seed = crate::random::seed_scope(1260673);
    let mut run = load_and_spawn_sunfury_targets(true, target_count);
    exhaust_native_fire_blast(&mut run);
    let exhausted_at = crate::spell_cast::history_now(run.app.world());
    if launch_missile {
        launch_native_destination_missile(&mut run);
    }
    for _ in 0..MAX_CAST_WAIT_FRAMES {
        match read_fire_blast_start(&mut run) {
            Ok(_) => return crate::spell_cast::history_now(run.app.world()) - exhausted_at,
            Err(crate::spell_cast::CastFailure::NoCharges) => run.app.update(),
            Err(error) => panic!("native charge recovery readiness: {error:?}"),
        }
    }
    panic!("native Fire Blast charge did not recover within the observation window");
}

fn assert_one_native_recovery_adjustment(target_count: usize) {
    let baseline = observe_native_fire_blast_recovery(target_count, false);
    let with_missile = observe_native_fire_blast_recovery(target_count, true);
    let reduction = baseline - with_missile;
    assert!(
        (reduction - 0.5).abs() <= 0.051,
        "native1260673e3=500ms once per landing: baseline={baseline}, impact={with_missile}, reduction={reduction}"
    );
}

#[test]
#[ignore = "ordinary native charge expenditure/readiness; no granted charges or timer mutation"]
fn fire_player_meteorite_native90_landing_reduces_actual_charge_recovery_st() {
    assert_one_native_recovery_adjustment(1);
}

#[test]
#[ignore = "ordinary native charge expenditure/readiness; one reduction, not per AoE victim"]
fn fire_player_meteorite_native90_landing_reduces_actual_charge_recovery_3t_once() {
    assert_one_native_recovery_adjustment(3);
}

#[test]
#[ignore = "diagnose native follower placement/Travel before fixture-only correction"]
fn fire_follower_meteorite_native90_idle_follow_keeps_native_cast_range() {
    let _seed = crate::random::seed_scope(1260673);
    let mut run = load_and_spawn_sunfury(false);
    let owner = run
        .app
        .world()
        .get::<crate::followers::Follower>(run.actor)
        .unwrap()
        .owner;
    for frame in 0..=200 {
        if frame % 40 == 0 {
            let world = run.app.world();
            println!(
                "FOLLOW_TRACE frame={frame} follower={:?} owner={:?} target={:?} travel={}",
                world.get::<shared::components::Position>(run.actor),
                world.get::<shared::components::Position>(owner),
                world.get::<shared::components::Position>(run.targets[0]),
                super::super::movement::inspect_travel(world, run.actor)
            );
        }
        if frame < 200 {
            run.app.update();
        }
    }
    let spell = run
        .app
        .world()
        .resource::<SpellStore>()
        .get(FIREBALL)
        .unwrap()
        .clone();
    let readiness = crate::spell_cast::read_native_cast_start(
        run.app.world_mut(),
        run.actor,
        &spell,
        run.targets[0],
    );
    assert!(
        readiness.is_ok(),
        "quiet native follower lost ordinary cast range: {readiness:?}"
    );
}

#[test]
#[ignore = "native OOC Sphere producer; no granted initial spheres or marker"]
fn fire_player_meteorite_native90_ooc_sphere_arms_one_fire_blast_storm() {
    let _seed = crate::random::seed_scope(449849);
    let mut run = load_and_spawn_sunfury(true);
    assert!(has_aura(run.app.world(), run.actor, SPHERE_TALENT));
    assert_eq!(sphere_stacks(&run), 0, "no fabricated initial sphere grant");
    assert!(!has_aura(run.app.world(), run.actor, GLORIOUS_MARKER));
    advance_native_sphere_tick(&mut run);
    assert_eq!(
        sphere_stacks(&run),
        1,
        "one sphere from the native OOC tick"
    );
    assert!(has_aura(run.app.world(), run.actor, GLORIOUS_MARKER));
    assert!(cast_and_observe_hit(&mut run, FIRE_BLAST));
    advance_native_missile_flight(&mut run);
    assert_eq!(owner_impact_counts(&run), vec![4]);
    assert!(!has_aura(run.app.world(), run.actor, GLORIOUS_MARKER));
    assert!(cast_and_observe_hit(&mut run, FIRE_BLAST));
    advance_native_missile_flight(&mut run);
    assert_eq!(
        owner_impact_counts(&run),
        vec![4],
        "marker is consumed once"
    );
}

#[test]
#[ignore = "native OOC Sphere acquisition cap; no initial5 or combat PRD bridge"]
fn fire_player_meteorite_native90_ooc_spheres_stop_at_native_cap() {
    let _seed = crate::random::seed_scope(449849);
    let mut run = load_and_spawn_sunfury(true);
    let cap = run
        .app
        .world()
        .resource::<SpellStore>()
        .get(SPHERE_BUFF)
        .unwrap()
        .aura_options
        .as_ref()
        .unwrap()
        .cumulative_aura;
    assert_eq!(cap, 3, "approved current native Sphere cap");
    assert_eq!(sphere_stacks(&run), 0);
    for tick in 1..=cap + 1 {
        advance_native_sphere_tick(&mut run);
        assert_eq!(u32::from(sphere_stacks(&run)), tick.min(cap));
    }
}

#[test]
#[ignore = "native Meteorite missile-edge isolation; not natural producer proof"]
fn fire_player_meteorite_native90_destination_missile_st_uses_native_delay() {
    assert_native_destination_missile_lands_once(1);
}

#[test]
#[ignore = "native Meteorite missile-edge isolation; not natural producer proof"]
fn fire_player_meteorite_native90_destination_missile_3t_lands_once_per_target() {
    assert_native_destination_missile_lands_once(3);
}

#[test]
#[ignore = "native player Meteorite source gate"]
fn fire_player_meteorite_native90_fireball_pyrocosm_produces_owner_damage() {
    assert_fireball_produces_meteorite(true);
}

#[test]
#[ignore = "native player Meteorite source gate"]
fn fire_follower_meteorite_native90_fireball_pyrocosm_produces_owner_damage() {
    assert_fireball_produces_meteorite(false);
}

#[test]
#[ignore = "native player Meteorite source gate"]
fn fire_player_meteorite_native90_pyroblast_without_fireball_does_not_trigger_pyrocosm() {
    let _seed = crate::random::seed_scope(1260673);
    let mut run = load_and_spawn_sunfury(true);
    let first = run.app.world().resource::<ObservedCombatEvents>().0.len();
    let flight_frames = native_missile_wait_frames(&run);
    let mut positive_pyroblasts = 0;
    for _ in 0..16 {
        positive_pyroblasts += usize::from(cast_and_observe_hit(&mut run, PYROBLAST));
        for _ in 0..flight_frames {
            run.app.update();
        }
    }
    assert!(positive_pyroblasts > 0);
    let events = &run.app.world().resource::<ObservedCombatEvents>().0[first..];
    assert!(
        !events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && matches!(event.spell_id, Some(FIREBALL | FIRE_BLAST))
                && event.kind == CombatLogKind::Damage),
        "the negative control must not have another native Meteorite producer"
    );
    assert!(
        !has_owner_meteorite_damage(&run),
        "native70448 Pyrocosm says Fireball damage; do not copy SimC's Pyroblast predicate"
    );
}
