//! Native 300s dummy qualification with explicit per-spec fixture controllers.
mod dummy_reference_tests;
mod engagement_tests;
mod equipment_input_tests;
mod firestarter_tests;
mod haste_tests;
mod inputs;
pub(crate) mod literal_rotation;
mod periodic_crit_tests;
mod pet_child_tests;
mod pet_melee_tests;
mod pet_stats_tests;
mod player_rotation;
mod power_haste_tests;
mod power_tests;
mod report;
pub(crate) mod single_spell;
mod windwalker_rotation;
use crate::simc_apl_native::{FixtureAdds, FixtureTarget};
use bevy::prelude::*;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

const TICKS: u32 = 6_000;
const ADD_WAVE_TICKS: u32 = 600;
const LAST_30S_START: u32 = TICKS - ADD_WAVE_TICKS;
const DUMMY_ENTRY: u32 = 31_146;
const FIGHT_DURATION: Duration = Duration::from_secs(300);
const ADD_LIFETIME: Duration = Duration::from_secs(30);
// SimC76f8118 raid_event.cpp2700–2701: equal cooldown/duration adds10ms.
const ADD_INTERVAL: Duration = Duration::from_millis(30_010);

struct Run {
    app: App,
    actor: Entity,
    targets: Vec<Entity>,
    damage_targets: Vec<Entity>,
    target_waves: Vec<Value>,
    profile: inputs::Profile,
    gaps: Vec<String>,
    player_rotation: Option<player_rotation::PlayerRotation>,
    windwalker_rotation: Option<windwalker_rotation::Rotation>,
}

fn spawn_dummy(app: &mut App, actor: Entity, index: usize) -> Result<Entity, String> {
    let target = crate::zones::summon_creature(
        app.world_mut(),
        actor,
        DUMMY_ENTRY,
        [1.5, index as f32, 0.0, std::f32::consts::PI],
    )
    .ok_or("native training dummy31146 could not spawn")?;
    if app
        .world()
        .get::<crate::creature_scripts::TrainingDummy>(target)
        .is_none()
    {
        return Err("native31146 lacks its actual training-dummy script".into());
    }
    // SIMC_FIXED_TIME_TARGET_HEALTH owns this fixture's current health. Source31146
    // has RegenHealth=1; its ordinary two-second heal must not alter phase gates.
    // Keep the template, script, flags and damage handling unchanged.
    app.world_mut()
        .entity_mut(target)
        .remove::<crate::spell_cast::powers::CreatureHealthRegenTimer>();
    Ok(target)
}

fn spawn_targets(app: &mut App, actor: Entity, count: usize) -> Result<Vec<Entity>, String> {
    (0..count)
        .map(|index| spawn_dummy(app, actor, index))
        .collect()
}

fn install_fixture_lifetimes(run: &mut Run, started_at: Duration) {
    for (index, &target) in run.targets.iter().enumerate() {
        let lifetime = if index == 0 {
            FIGHT_DURATION
        } else {
            ADD_LIFETIME
        };
        run.app
            .world_mut()
            .entity_mut(target)
            .insert(FixtureTarget {
                expires_at: started_at + lifetime,
                lifetime,
                is_boss: index == 0,
            });
    }
    if run.targets.len() == 3 {
        run.app
            .world_mut()
            .entity_mut(run.actor)
            .insert(FixtureAdds {
                next_spawn_at: started_at + ADD_INTERVAL,
                expires_at: Some(started_at + ADD_LIFETIME),
                lifetime: ADD_LIFETIME,
                interval: ADD_INTERVAL,
                count: 2,
            });
    }
}

fn retire_add_wave(run: &mut Run) -> Vec<Entity> {
    let retired = run.targets.split_off(1);
    for &unit in &retired {
        crate::zones::unsummon(run.app.world_mut(), unit);
    }
    run.app
        .world_mut()
        .get_mut::<FixtureAdds>(run.actor)
        .unwrap()
        .expires_at = None;
    retired
}

fn spawn_add_wave(run: &mut Run, retired: &[Entity], started_at: Duration) -> Result<(), String> {
    let now = run.app.world().resource::<Time>().elapsed();
    let adds = *run.app.world().get::<FixtureAdds>(run.actor).unwrap();
    for index in 1..=adds.count {
        let unit = spawn_dummy(&mut run.app, run.actor, index)?;
        run.app.world_mut().entity_mut(unit).insert(FixtureTarget {
            expires_at: now + adds.lifetime,
            lifetime: adds.lifetime,
            is_boss: false,
        });
        run.targets.push(unit);
        run.damage_targets.push(unit);
    }
    {
        let mut state = run
            .app
            .world_mut()
            .get_mut::<FixtureAdds>(run.actor)
            .unwrap();
        state.expires_at = Some(now + adds.lifetime);
        state.next_spawn_at = now + adds.interval;
    }
    apply_fixed_time_target_health(run, now)?;
    engage_fixture_opponents(run);
    let retired_still_present = retired
        .iter()
        .filter(|&&unit| run.app.world().get_entity(unit).is_ok())
        .count();
    run.target_waves
        .push(json!({"at": (now - started_at).as_secs_f64(),
        "entities": run.targets.iter().map(|unit| unit.to_bits()).collect::<Vec<_>>(),
        "retired_still_present": retired_still_present}));
    Ok(())
}

fn read_reference(spec_id: u32) -> Result<(inputs::Profile, Vec<inputs::AllocatedTrait>), String> {
    let directory = inputs::directory()?;
    let profile: inputs::Profile =
        inputs::read_json(&directory.join(format!("{spec_id}.profile.json")))?;
    if profile.spec_id != spec_id {
        return Err("reference spec identity mismatch".into());
    }
    let traits =
        inputs::read_json(&directory.join(format!("{spec_id}.selected-traits-active.json")))?;
    Ok((profile, traits))
}

fn prepare(spec_id: u32, count: usize) -> Result<Run, String> {
    let (profile, traits) = read_reference(spec_id)?;
    let identity = inputs::read_actor(&profile)?;
    prepare_reference(profile, &traits, count, identity)
}

fn prepare_reference(
    profile: inputs::Profile,
    traits: &[inputs::AllocatedTrait],
    count: usize,
    identity: crate::followers::simulator::ParityActor,
) -> Result<Run, String> {
    let spec_id = profile.spec_id;
    let mut app = super::runtime::build_app();
    inputs::install_reference_catalogs(&mut app)?;
    let copy = super::runtime::create_copy(&mut app);
    let level = profile
        .level
        .parse()
        .map_err(|error| format!("reference level: {error}"))?;
    let actor = crate::followers::simulator::spawn_parity_attacker(
        app.world_mut(),
        spec_id,
        level,
        copy,
        identity,
    )?;
    inputs::equip(app.world_mut(), actor, &profile)?;
    inputs::install_traits(app.world_mut(), actor, traits)?;
    let targets = spawn_targets(&mut app, actor, count)?;
    let target_waves = vec![
        json!({"at": 0.0, "entities": targets.iter().map(|unit| unit.to_bits()).collect::<Vec<_>>()}),
    ];
    for _ in 0..4 {
        app.update();
    }
    let is_player = app
        .world()
        .get::<shared::components::Player>(actor)
        .is_some();
    let gaps = report::gaps(&profile, is_player);
    Ok(Run {
        app,
        actor,
        damage_targets: targets.clone(),
        targets,
        target_waves,
        profile,
        gaps,
        player_rotation: None,
        windwalker_rotation: None,
    })
}

fn fixture_player(run: &Run) -> Entity {
    let world = run.app.world();
    if world.get::<shared::components::Player>(run.actor).is_some() {
        run.actor
    } else {
        world
            .get::<crate::followers::Follower>(run.actor)
            .unwrap()
            .owner
    }
}

fn engage_fixture_opponents(run: &mut Run) {
    let owner = fixture_player(run);
    for &target in &run.targets {
        crate::combat_refs::set_in_combat_with(run.app.world_mut(), owner, target);
    }
}

fn engage(run: &mut Run) {
    let owner = fixture_player(run);
    let world = run.app.world_mut();
    // A player selects its target directly; a follower retains its quiet controller.
    world.entity_mut(owner).insert((
        crate::networking::CombatTarget(Some(run.targets[0])),
        crate::combat::AutoAttack::new(run.targets[0]),
    ));
    world
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .clear();
    engage_fixture_opponents(run);
}

fn target_snapshot(run: &Run) -> Vec<Value> {
    let world = run.app.world();
    run.targets
        .iter()
        .map(|&target| {
            let native = *world.get::<shared::components::UnitLevel>(target).unwrap();
            let viewer = world.get::<shared::components::Level>(run.actor).unwrap().0;
            let effective = shared::level_scaling::level_for_viewer(
                native,
                world.get::<shared::level_scaling::LevelScaling>(target),
                viewer,
            );
            json!({"entry": DUMMY_ENTRY, "native_level": native.0, "effective_level": effective,
            "health": world.get::<shared::components::Health>(target),
            "armor": world.get::<crate::level_scaling::CreatureArmor>(target).map(|armor| armor.0)})
        })
        .collect()
}

/// Main uses the global fight deadline; sc_enemy.cpp1053–1101 timed adds
/// project their own expiration. Native training-dummy hit handling stays inert.
fn apply_fixed_time_target_health(run: &mut Run, at: Duration) -> Result<(), String> {
    for &target in &run.targets {
        let metadata = *run
            .app
            .world()
            .get::<FixtureTarget>(target)
            .ok_or("native fixture target lifetime missing")?;
        if metadata.lifetime.is_zero() {
            return Err("native fixture target has zero lifetime".into());
        }
        let remaining = metadata.remaining(at).as_secs_f64() / metadata.lifetime.as_secs_f64();
        let mut health = run
            .app
            .world_mut()
            .get_mut::<shared::components::Health>(target)
            .ok_or_else(|| {
                format!("SIMC_FIXED_TIME_TARGET_HEALTH target{target:?} lacks native Health")
            })?;
        health.current = (f64::from(health.max) * remaining) as f32;
    }
    Ok(())
}

fn next_fixture_boundary(run: &Run, frame_end: Duration) -> Duration {
    let Some(adds) = run.app.world().get::<FixtureAdds>(run.actor) else {
        return frame_end;
    };
    let boundary = adds
        .expires_at
        .map_or(adds.next_spawn_at, |at| at.min(adds.next_spawn_at));
    frame_end.min(boundary)
}

fn advance_fixture_step(run: &mut Run, boundary: Duration) -> Result<(), String> {
    let now = run.app.world().resource::<Time>().elapsed();
    let delta = boundary
        .checked_sub(now)
        .filter(|delta| !delta.is_zero())
        .ok_or("native fixture clock did not advance")?;
    apply_fixed_time_target_health(run, boundary)?;
    run.app
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(delta));
    run.app.update();
    if run.app.world().resource::<Time>().elapsed() != boundary {
        return Err("native fixture clock differs from requested boundary".into());
    }
    Ok(())
}

fn apply_fixture_add_events(
    run: &mut Run,
    retired: &mut Vec<Entity>,
    started_at: Duration,
) -> Result<(), String> {
    let Some(adds) = run.app.world().get::<FixtureAdds>(run.actor).copied() else {
        return Ok(());
    };
    let now = run.app.world().resource::<Time>().elapsed();
    if adds.expires_at.is_some_and(|at| at <= now) {
        *retired = retire_add_wave(run);
    }
    if adds.next_spawn_at <= now {
        spawn_add_wave(run, retired, started_at)?;
    }
    Ok(())
}

fn advance_fixture_frame(
    run: &mut Run,
    frame_end: Duration,
    started_at: Duration,
    retired: &mut Vec<Entity>,
    observe: &mut impl FnMut(&Run),
) -> Result<(), String> {
    while run.app.world().resource::<Time>().elapsed() < frame_end {
        let boundary = next_fixture_boundary(run, frame_end);
        advance_fixture_step(run, boundary)?;
        apply_fixture_add_events(run, retired, started_at)?;
        if run.app.world().resource::<Time>().elapsed() < started_at + FIGHT_DURATION {
            drive_player(run)?;
        }
        observe(run);
    }
    Ok(())
}

fn advance_combat(run: &mut Run) -> Result<(f64, BTreeMap<u32, f64>, Vec<Value>), String> {
    advance_combat_observed(run, |_| {})
}

fn advance_combat_observed(
    run: &mut Run,
    mut observe: impl FnMut(&Run),
) -> Result<(f64, BTreeMap<u32, f64>, Vec<Value>), String> {
    let start_time = run
        .app
        .world()
        .resource::<Time<Virtual>>()
        .elapsed_secs_f64();
    let started_at = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(run, started_at);
    if run
        .app
        .world()
        .get::<shared::components::Player>(run.actor)
        .is_some()
    {
        run.player_rotation = Some(player_rotation::PlayerRotation::load(
            run.profile.spec_id,
            start_time,
        )?);
    } else if run.profile.spec_id == 269 {
        let encounter = crate::simc_apl_native::Encounter::Fixture {
            started_at: start_time,
            duration: f64::from(TICKS) / 20.0,
            style: crate::simc_apl_native::FightStyle::Patchwerk,
        };
        run.windwalker_rotation = Some(windwalker_rotation::Rotation::load(
            run.app.world_mut(),
            run.actor,
            encounter,
        )?);
    }
    let mut buffs = BTreeMap::new();
    let mut samples = vec![report::engagement(run.app.world(), run.actor, &run.targets)];
    observe(run);
    drive_player(run)?;
    let frame = FIGHT_DURATION / TICKS;
    let mut retired = Vec::new();
    for tick in 1..=TICKS {
        advance_fixture_frame(
            run,
            started_at + frame * tick,
            started_at,
            &mut retired,
            &mut observe,
        )?;
        report::sample_buffs(run.app.world(), run.actor, &mut buffs);
        if matches!(tick, 1 | 20 | LAST_30S_START | TICKS) {
            samples.push(report::engagement(run.app.world(), run.actor, &run.targets));
        }
    }
    let seconds = run
        .app
        .world()
        .resource::<Time<Virtual>>()
        .elapsed_secs_f64()
        - start_time;
    let buff_uptimes = buffs
        .into_iter()
        .map(|(spell, ticks)| (spell, f64::from(ticks) * 100.0 / f64::from(TICKS)))
        .collect();
    Ok((seconds, buff_uptimes, samples))
}

fn drive_player(run: &mut Run) -> Result<(), String> {
    if let Some(rotation) = &mut run.player_rotation {
        rotation.drive(run.app.world_mut(), run.actor, run.targets[0])?;
    }
    if let Some(rotation) = &mut run.windwalker_rotation {
        rotation.drive(run.app.world_mut(), run.actor, &run.targets)?;
    }
    Ok(())
}

fn measure(spec_id: u32, count: usize) -> Result<Value, String> {
    let _seed_scope = crate::random::seed_scope(70_448);
    let mut run = prepare(spec_id, count)?;
    let before = report::stats(run.app.world(), run.actor);
    let targets = target_snapshot(&run);
    engage(&mut run);
    let started = Instant::now();
    let (seconds, buff_uptimes, engagement_samples) = advance_combat(&mut run)?;
    let combat = report::combat(run.app.world(), &run.damage_targets, seconds);
    Ok(
        json!({"spec_id": spec_id, "target_count": count, "simulated_seconds": seconds,
        "actor_kind": if run.player_rotation.is_some() { "player" } else { "follower" },
        "driver": if run.windwalker_rotation.is_some() { "LITERAL_SIMC_WINDWALKER_76f8118" } else if run.player_rotation.is_some() { "NATIVE_ASSISTED_COMBAT_70448" } else { "NATIVE_FOLLOWER_KIT" },
        "windwalker_rotation": run.windwalker_rotation.as_ref().map(|rotation| rotation.report()),
        "assisted_decisions": run.player_rotation.as_ref().map_or(0, |rotation| rotation.decisions),
        "wall_seconds": started.elapsed().as_secs_f64(), "seed": 70448, "combat": combat,
        "target_health_policy": {"name": "SIMC_FIXED_TIME_TARGET_HEALTH", "clock": "per_target_lifetime",
            "duration_seconds": FIGHT_DURATION.as_secs_f64(), "add_lifetime_seconds": ADD_LIFETIME.as_secs_f64(),
            "add_interval_seconds": ADD_INTERVAL.as_secs_f64(), "initial_pct": 100, "death_pct": 0},
        "stats_before": before, "stats_after": report::stats(run.app.world(), run.actor),
        "targets_before": targets, "targets_after": target_snapshot(&run),
        "target_waves": run.target_waves, "pet": report::pet(run.app.world(), run.actor, &run.damage_targets),
        "buff_uptime_pct": buff_uptimes, "engagement_samples": engagement_samples, "gaps": run.gaps,
        "reference_race": run.profile.race, "server_build": 70448, "simc_build": 70391}),
    )
}

#[test]
#[ignore = "opt-in real profile gear, allocated traits and300s native combat; requires private DB/evidence path"]
fn dps_parity_native_bis_dummy_records_300s() {
    let spec: u32 = std::env::var("DPS_PARITY_SPEC")
        .expect("DPS_PARITY_SPEC")
        .parse()
        .expect("numeric specialization");
    let count: usize = std::env::var("DPS_PARITY_TARGETS")
        .expect("DPS_PARITY_TARGETS")
        .parse()
        .expect("numeric target count");
    assert!(matches!(count, 1 | 3));
    let result = measure(spec, count);
    let output = match &result {
        Ok(report) => report.clone(),
        Err(error) => json!({"spec_id": spec, "target_count": count, "measurement_error": error}),
    };
    let file = inputs::directory()
        .unwrap()
        .join(format!("{spec}.native.{count}.json"));
    std::fs::write(&file, serde_json::to_vec_pretty(&output).unwrap()).unwrap();
    let report = result.unwrap_or_else(|error| panic!("native spec{spec}: {error}"));
    assert!((report["simulated_seconds"].as_f64().unwrap() - 300.0).abs() < 0.001);
    assert!(
        report["combat"]["total_damage"].as_u64().unwrap() > 0,
        "actual production kit must deal damage; full report {}",
        file.display()
    );
    if count == 3 {
        let waves = report["target_waves"].as_array().unwrap();
        assert_eq!(
            waves.len(),
            10,
            "two adds must retire and respawn every30s across300s"
        );
        let main = waves[0]["entities"][0].as_u64().unwrap();
        let mut adds = std::collections::BTreeSet::new();
        for (index, wave) in waves.iter().enumerate() {
            assert!(
                (wave["at"].as_f64().unwrap() - index as f64 * ADD_INTERVAL.as_secs_f64()).abs()
                    < 0.0001
            );
            if index > 0 {
                assert_eq!(
                    wave["retired_still_present"], 0,
                    "old wave must actually despawn"
                );
            }
            assert_eq!(
                wave["entities"][0].as_u64().unwrap(),
                main,
                "main dummy must persist"
            );
            for add in wave["entities"].as_array().unwrap().iter().skip(1) {
                assert!(
                    adds.insert(add.as_u64().unwrap()),
                    "retired add identity must not be reused"
                );
            }
        }
        assert_eq!(adds.len(), 20);
    }
    if matches!(spec, 253 | 266) {
        assert_eq!(
            report["pet"]["present"], true,
            "native pet-class execution requires its authored pet"
        );
        assert!(
            !report["pet"]["casts"].as_array().unwrap().is_empty(),
            "real pet must execute its learned casts"
        );
        assert!(
            report["pet"]["damage"].as_i64().unwrap() > 0,
            "real pet must damage the dummy"
        );
    }
    let samples = report["engagement_samples"].as_array().unwrap();
    if report["actor_kind"] == "player" {
        assert_eq!(samples[0]["actor_is_player"], true);
        assert!(
            samples[0]["owner"].is_null(),
            "player has no synthetic controller"
        );
    } else {
        assert_eq!(
            samples[0]["actor_is_player"], false,
            "measured follower stays an NPC"
        );
        assert_eq!(
            samples[0]["owner_is_player"], true,
            "owner uses native player initialization"
        );
        let owner = samples[0]["owner"]["entity"].as_u64().unwrap();
        let owner_damage: u64 = report["combat"]["actions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|action| action["actor"].as_u64() == Some(owner))
            .map(|action| action["damage"].as_u64().unwrap())
            .sum();
        assert_eq!(
            owner_damage, 0,
            "out-of-range controller must not contribute damage"
        );
    }
    let end_damage = samples.last().unwrap()["damage_to_targets"]
        .as_i64()
        .unwrap();
    let last_30s_damage = end_damage
        - samples[samples.len() - 2]["damage_to_targets"]
            .as_i64()
            .unwrap();
    assert!(
        last_30s_damage > 0,
        "fixture must sustain actual damage in the final30s, not merely land first-tick hits"
    );
    eprintln!(
        "NATIVE_DPS spec{spec} targets{count}: {}",
        report["combat"]["dps"]
    );
}

#[test]
#[ignore = "opt-in actual300s per-wave health/TTD; requires private parity inputs"]
fn dps_parity_fixture_add_health_and_ttd_follow_each_summon_lifetime() {
    use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
    use server::simc_apl::{Action, ActionFacts};
    let _seed_scope = crate::random::seed_scope(70_448);
    let (profile, traits) = read_reference(66).unwrap();
    let mut run = prepare_reference(
        profile,
        &traits,
        3,
        crate::followers::simulator::ParityActor::Follower,
    )
    .unwrap();
    engage(&mut run);
    let started_at = crate::spell_cast::now(run.app.world());
    let encounter = Encounter::Fixture {
        started_at,
        duration: 300.0,
        style: FightStyle::Patchwerk,
    };
    let bindings = Bindings::default();
    let mut seen = [[false; 2]; 10];
    let mut gaps = [false; 9];
    let mut errors = Vec::new();
    let (seconds, _, _) = advance_combat_observed(&mut run, |run| {
        let elapsed = crate::spell_cast::now(run.app.world()) - started_at;
        let millis = (elapsed * 1000.0).round() as u32;
        let wave = (millis / 30_010) as usize;
        if wave >= seen.len() {
            return;
        }
        let age = millis % 30_010;
        if age >= 30_000 {
            gaps[wave] = true;
            if run.targets.len() != 1 {
                errors.push(format!(
                    "{elapsed}s: expected absent adds, got{} targets",
                    run.targets.len()
                ));
            }
            return;
        }
        let sample_index = if age >= 27_000 {
            1 //10% health: both Execute thresholds have been crossed in this wave.
        } else if age >= 15_000 {
            0 //50% health: same wave before Execute eligibility.
        } else {
            return;
        };
        if seen[wave][sample_index] {
            return;
        }
        seen[wave][sample_index] = true;
        if run.targets.len() != 3 {
            errors.push(format!(
                "wave{wave}: expected3 targets, got{}",
                run.targets.len()
            ));
        }
        let facts = NativeFacts::new(
            run.app.world(),
            run.actor,
            &bindings,
            encounter,
            run.targets.len(),
        );
        for (index, &target) in run.targets.iter().enumerate() {
            // sc_enemy.cpp1053–1101: timed adds use their own expiration, not300s.
            let expected_ttd = if index == 0 {
                300.0 - elapsed
            } else {
                f64::from(30_000 - age) / 1000.0
            };
            let lifetime = if index == 0 { 300.0 } else { 30.0 };
            let expected_pct = expected_ttd / lifetime * 100.0;
            let health = run
                .app
                .world()
                .get::<shared::components::Health>(target)
                .unwrap();
            let native_pct = f64::from(health.current) * 100.0 / f64::from(health.max);
            let action = Action {
                name: "auto_attack".into(),
                target: Some(target.to_bits()),
                options: Default::default(),
            };
            let readings = [
                ("native health", Ok(native_pct), expected_pct),
                (
                    "target.health.pct",
                    facts.value_for(&action, "target.health.pct"),
                    expected_pct,
                ),
                (
                    "target.time_to_die",
                    facts.value_for(&action, "target.time_to_die"),
                    expected_ttd,
                ),
                // player.hpp964 and raid_event.cpp77: main boss, timed ENEMY_ADDs not bosses.
                (
                    "target.is_boss",
                    facts.value_for(&action, "target.is_boss"),
                    if index == 0 { 1.0 } else { 0.0 },
                ),
            ];
            for (label, actual, expected) in readings {
                match actual {
                    Ok(value) if (value - expected).abs() < 0.001 => {}
                    other => errors.push(format!(
                        "wave{wave}/sample{sample_index}/target{index} {label}: {other:?}, expected{expected}"
                    )),
                }
            }
        }
    })
    .unwrap();
    assert!((seconds - 300.0).abs() < 0.001);
    assert!(
        seen.into_iter().flatten().all(|seen| seen),
        "all10 waves must be observed before and during their Execute phase"
    );
    assert!(errors.is_empty(), "{}", errors.join("\n"));
    assert!(
        gaps.into_iter().all(|seen| seen),
        "all9 actual10ms absent windows must be observed"
    );
}

#[test]
#[ignore = "opt-in actual300s native add lifetimes; requires private parity inputs"]
fn dps_parity_fixture_add_start_schedule_matches_simc_10ms_cooldown_clamp() {
    let _seed_scope = crate::random::seed_scope(70_448);
    let (profile, traits) = read_reference(66).unwrap();
    let mut run = prepare_reference(
        profile,
        &traits,
        3,
        crate::followers::simulator::ParityActor::Follower,
    )
    .unwrap();
    engage(&mut run);
    let (seconds, _, _) = advance_combat(&mut run).unwrap();
    assert!((seconds - 300.0).abs() < 0.001);
    assert_eq!(run.target_waves.len(), 10);
    // Pinned raid_event.cpp2700–2701: cooldown<=duration becomes duration+0.01s.
    for (index, wave) in run.target_waves.iter().enumerate() {
        let expected = index as f64 * 30.01;
        let actual = wave["at"].as_f64().unwrap();
        assert!(
            (actual - expected).abs() < 0.0001,
            "wave{index} started at{actual}s, source requires{expected}s"
        );
    }
}

#[test]
#[ignore = "opt-in real300s dummy clock and add replacement; requires private parity inputs"]
fn dps_parity_fixture_fixed_time_main_health_follows_global_clock() {
    let _seed_scope = crate::random::seed_scope(70_448);
    let (profile, traits) = read_reference(66).unwrap();
    let mut run = prepare_reference(
        profile,
        &traits,
        3,
        crate::followers::simulator::ParityActor::Follower,
    )
    .unwrap();
    engage(&mut run);
    let (seconds, _, samples) = advance_combat(&mut run).unwrap();
    assert!((seconds - 300.0).abs() < 0.001);
    let expected = [
        100.0,
        100.0 * 5999.0 / 6000.0,
        100.0 * 5980.0 / 6000.0,
        10.0,
        0.0,
    ];
    assert_eq!(samples.len(), expected.len());
    for (sample, expected_pct) in samples.iter().zip(expected) {
        // Adds' independent phases are checked twice in every wave above.
        for target in sample["targets"].as_array().unwrap().iter().take(1) {
            let health = target["state"]["health"].as_array().unwrap();
            let observed = health[0].as_f64().unwrap() * 100.0 / health[1].as_f64().unwrap();
            assert!(
                (observed - expected_pct).abs() < 0.001,
                "SIMC_FIXED_TIME_TARGET_HEALTH expected{expected_pct}% but observed{observed}%: {target}"
            );
        }
    }
    assert_eq!(run.target_waves.len(), 10);
}

#[test]
#[ignore = "opt-in exact-BiS player authored decision or explicit source gap; requires private parity inputs"]
fn dps_parity_fixture_player_uses_authored_decision_or_records_rule_gap() {
    match measure(71, 1) {
        Ok(report) => {
            assert_eq!(report["actor_kind"], "player");
            assert_eq!(report["driver"], "NATIVE_ASSISTED_COMBAT_70448");
            assert!(report["assisted_decisions"].as_u64().unwrap() > 0);
        }
        Err(error) => {
            assert!(error.starts_with("NATIVE_ASSISTED_COMBAT_GAP:"), "{error}");
            let lowercase = error.to_lowercase();
            assert!(
                lowercase.contains("rule") && lowercase.contains("unverified"),
                "{error}"
            );
            eprintln!("AUTHORED_PLAYER_RULE_GAP: {error}");
        }
    }
}

#[test]
#[ignore = "opt-in native Arms71 player with pinned level90 gear/traits; requires private parity inputs"]
fn dps_parity_native_player_profile_spawns_with_pinned_gear() {
    let run = prepare(71, 1).expect("pinned native Arms71 player profile must spawn");
    let world = run.app.world();
    let player = world
        .get::<shared::components::Player>(run.actor)
        .expect("measured actor must be a real player");
    assert_eq!(player.class, 1);
    assert!(world.get::<shared::components::Npc>(run.actor).is_none());
    assert!(world.get::<crate::followers::Follower>(run.actor).is_none());
    assert_eq!(
        world.get::<shared::components::Level>(run.actor).unwrap().0,
        90
    );
    let expected_items: Vec<_> = run
        .profile
        .equipment
        .iter()
        .map(|item| {
            (
                inputs::slot(&item.slot).unwrap(),
                shared::item_data::ItemDefinitionSource::Retail,
                item.item_id,
            )
        })
        .collect();
    assert_eq!(
        world
            .get::<crate::stats::EquippedItems>(run.actor)
            .unwrap()
            .items,
        expected_items
    );
    let expected_modifiers: Vec<_> = run
        .profile
        .equipment
        .iter()
        .map(|item| (inputs::slot(&item.slot).unwrap(), item.modifiers()))
        .collect();
    assert_eq!(
        world
            .get::<crate::stats::EquippedItemModifiers>(run.actor)
            .unwrap()
            .items,
        expected_modifiers
    );
    let stats = world
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap();
    assert!(
        stats.main_hand.min > 1.0,
        "native BiS weapon stats must be applied"
    );
    assert!(stats.primary.strength > 0.0);
}
