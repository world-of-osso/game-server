//! Full-population route controller and observable run results.
use super::{graph::Graph, runtime};
use bevy::prelude::*;
use serde::Serialize;
use shared::components::{CombatStatus, Health, Mana, Position};
use std::collections::BTreeMap;

const MAX_TICKS: usize = 72_000;
const STALL_SECONDS: f64 = 60.0;
const BOSS_ENTRIES: [u32; 3] = [46383, 46264, 46254];

#[derive(Component)]
pub(super) struct MoveIntent(pub Vec3);

#[derive(Serialize)]
pub(super) struct MemberSample {
    entity: u64,
    exists: bool,
    name: Option<String>,
    health: Option<f32>,
    max_health: Option<f32>,
    mana: Option<f32>,
    max_mana: Option<f32>,
    position: Option<[f32; 2]>,
}

#[derive(Serialize)]
pub(super) struct Sample {
    at: f64,
    members: Vec<MemberSample>,
}

#[derive(Serialize)]
pub(super) struct PullDecision {
    pub at: f64,
    pub pack: u32,
    pub decision: String,
}

#[derive(Serialize)]
pub(super) struct Report {
    pub seed: u64,
    pub status: String,
    pub failed_pack: Option<u32>,
    pub bosses_killed: Vec<u32>,
    pub simulated_seconds: f64,
    pub wall_seconds: f64,
    pub detail: String,
    pub samples: Vec<Sample>,
    pub pulls: Vec<PullDecision>,
    pub performance: serde_json::Value,
    pub terminal_units: serde_json::Value,
}

#[test]
fn spawned_normal_population_allows_existing_fight_tank_movement() {
    let _rng_scope = crate::random::seed_scope(34);
    let mut run = spawn_run(34, &mut None).unwrap();
    let target = run.actors[&20375718];
    let position = *run.app.world().get::<Position>(target).unwrap();
    let tank = run.party.tank;
    let members: Vec<_> = run.members().collect();
    for unit in members {
        run.app.world_mut().entity_mut(unit).insert(Position {
            x: position.x - 2.0,
            ..position
        });
    }
    {
        let mut data = run
            .app
            .world_mut()
            .resource_mut::<crate::world_data::WorldData>();
        let index = data
            .spawns
            .iter()
            .position(|spawn| spawn.guid == 20375718)
            .unwrap();
        data.spawns_by_map.insert(34, vec![index]);
    }
    run.app
        .world_mut()
        .entity_mut(tank)
        .insert(crate::networking::CombatTarget(Some(target)));
    crate::creature_aggro::add_threat(run.app.world_mut(), target, tank, 1.0);
    crate::followers::update_ai(run.app.world_mut());
    assert_eq!(
        crate::followers::simulator::inspect_tank_movement(run.app.world_mut(), tank),
        Ok(Some(true)),
        "native existing-fight admission must consume the spawned Normal population without missing source metadata"
    );
}

#[test]
fn stationary_full_party_still_stalls_without_any_progress() {
    let _rng_scope = crate::random::seed_scope(1);
    let mut run = spawn_run(1, &mut None).unwrap();
    for _ in 0..400 {
        run.app.update();
    }
    run.last_progress = run.now();
    assert!(!run.stalled());
    for _ in 0..1201 {
        run.app.update();
    }
    assert!(
        run.stalled(),
        "full resources must not manufacture progress"
    );
}

#[test]
fn native_mana_recovery_is_party_progress_without_damage_or_movement() {
    let _rng_scope = crate::random::seed_scope(1);
    let mut run = spawn_run(1, &mut None).unwrap();
    for _ in 0..400 {
        run.app.update();
    }
    run.last_progress = run.now();
    let healer = run.party.members[1];
    crate::spell_cast::creature_cast(run.app.world_mut(), healer, 8936, Some(healer), false)
        .unwrap();
    for _ in 0..1201 {
        run.app.update();
        assert!(
            !run.stalled(),
            "native OOC resource recovery must refresh party progress: {}",
            run.report.detail
        );
    }
}

#[test]
fn scripted_boss_ready_single_pull_does_not_require_a_numeric_forecast() {
    let _rng_scope = crate::random::seed_scope(3);
    let mut run = spawn_run(3, &mut None).unwrap();
    for _ in 0..400 {
        run.app.update();
    }
    let tank = run.party.tank;
    let boss = std::collections::BTreeSet::from([20375766]);
    let decision = crate::followers::simulator::pull(run.app.world_mut(), tank, &boss);
    assert!(
        matches!(&decision, Ok((true, _))),
        "SCRIPTED_BOSS_PULL_WHEN_READY: {decision:?}"
    );
    let multiple = std::collections::BTreeSet::from([20375766, 20375765]);
    assert!(
        !crate::followers::simulator::pull(run.app.world_mut(), tank, &multiple)
            .unwrap()
            .0,
        "boss admission must not chain an add"
    );
    let healer = run.party.members[1];
    crate::combat::kill_unit(run.app.world_mut(), run.party.owner, healer);
    assert!(
        !crate::followers::simulator::pull(run.app.world_mut(), tank, &boss)
            .unwrap()
            .0,
        "dead admitted member is a real wait, not ready"
    );
}

#[test]
fn cleared_ready_party_pulls_next_single_pack_without_measurements() {
    let _rng_scope = crate::random::seed_scope(3);
    let mut run = spawn_run(3, &mut None).unwrap();
    for _ in 0..400 {
        run.app.update();
    }
    let tank = run.party.tank;
    let first = run.actors[&20375759];
    crate::combat::deal_direct_damage(run.app.world_mut(), tank, first, 100.0);
    crate::combat::kill_unit(run.app.world_mut(), tank, first);
    run.app.update();
    let next = std::collections::BTreeSet::from([20375765]);
    let decision = crate::followers::simulator::pull(run.app.world_mut(), tank, &next);
    assert!(
        matches!(&decision, Ok((true, _))),
        "CLEARED_STATE_ALWAYS_PULLS_NEXT_PACK: {decision:?}"
    );
}

#[test]
#[ignore = "opening footprint regression; explicit RED before generic admission diagnosis"]
fn opening_admission_does_not_recruit_unprojected_source_packs() {
    let _rng_scope = crate::random::seed_scope(7);
    let mut run = spawn_run(7, &mut None).unwrap();
    for _ in 0..600 {
        run.control().unwrap();
        run.app.update();
        let Some(opening) = run.report.pulls.first() else {
            continue;
        };
        let admitted = &run
            .graph
            .packs
            .iter()
            .find(|pack| pack.id == opening.pack)
            .unwrap()
            .spawns;
        let unexpected: Vec<_> = run
            .actors
            .iter()
            .filter(|(guid, actor)| {
                !admitted.iter().any(|spawn| spawn.guid == **guid)
                    && run
                        .app
                        .world()
                        .get::<CombatStatus>(**actor)
                        .is_some_and(|status| status.0)
            })
            .map(|(guid, actor)| {
                super::diagnostics::inspect_unit(run.app.world(), Some(*guid), *actor)
            })
            .collect();
        assert!(
            unexpected.is_empty(),
            "single-pack bootstrap recruited unprojected sources at {}s; owner={:?}; tank={:?}; unexpected={unexpected:?}",
            run.now(),
            run.app.world().get::<Position>(run.party.owner),
            run.app.world().get::<Position>(run.party.tank)
        );
    }
}

#[test]
fn sampling_retains_a_member_after_native_dead_summon_despawn() {
    let mut run = spawn_run(2, &mut None).unwrap();
    let member = run.party.members[1];
    crate::combat::kill_unit(run.app.world_mut(), run.party.owner, member);
    for _ in 0..1202 {
        run.app.update();
    }
    assert!(
        run.app.world().get_entity(member).is_err(),
        "native corpse decay must actually despawn the dead follower"
    );
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run.sample())).is_ok(),
        "a native dead summon is reportable, not a batch-aborting panic"
    );
    let sample = run.report.samples.last().unwrap();
    assert_eq!(sample.members.len(), 5);
    let absent = sample
        .members
        .iter()
        .find(|state| state.entity == member.to_bits())
        .unwrap();
    assert!(!absent.exists);
    assert!(absent.health.is_none());
    assert!(absent.position.is_none());
}

struct Run {
    app: App,
    graph: Graph,
    party: crate::followers::simulator::Party,
    actors: BTreeMap<u32, Entity>,
    report: Report,
    current_pack: Option<u32>,
    last_progress: f64,
    event_cursor: usize,
    last_owner_position: Vec3,
}

pub(super) fn run(seed: u64) -> Result<Report, String> {
    run_cached(seed, &mut None)
}

pub(super) fn run_cached(
    seed: u64,
    catalogs: &mut Option<super::catalogs::Catalogs>,
) -> Result<Report, String> {
    let _rng_scope = crate::random::seed_scope(seed);
    let started = std::time::Instant::now();
    let mut run = spawn_run(seed, catalogs)?;
    let setup = started.elapsed();
    let mut timings = [std::time::Duration::ZERO; 3];
    let mut ticks = 0;
    for tick in 0..MAX_TICKS {
        let observe = std::time::Instant::now();
        if tick % 10 == 0 {
            run.sample();
        }
        if run.finished() {
            break;
        }
        timings[0] += observe.elapsed();
        let control = std::time::Instant::now();
        if let Err(error) = run.control() {
            run.report.status = "blocked".into();
            run.report.detail = error;
            break;
        }
        timings[1] += control.elapsed();
        let update = std::time::Instant::now();
        run.app.update();
        timings[2] += update.elapsed();
        ticks += 1;
        if tick % 1000 == 0 {
            run.write_progress(ticks, started, setup, &timings)?;
        }
        if run.stalled() {
            break;
        }
    }
    run.sample();
    run.report.performance = run.performance(ticks, started, setup, &timings);
    run.report.performance["tank_movement_admission"] = serde_json::json!(
        crate::followers::simulator::inspect_tank_movement(run.app.world_mut(), run.party.tank)
    );
    run.report.performance["combat_events"] = serde_json::to_value(
        &run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0,
    )
    .map_err(|error| format!("serialize native combat trace: {error}"))?;
    run.report.terminal_units = run.inspect_units();
    run.report.simulated_seconds = run.now();
    run.report.wall_seconds = started.elapsed().as_secs_f64();
    run.report.failed_pack = (run.report.status != "cleared")
        .then_some(run.current_pack)
        .flatten();
    if run.report.status == "running" {
        run.report.status = "timeout".into();
        run.report.detail = "one-hour simulated bound reached".into();
    }
    *catalogs = Some(super::catalogs::Catalogs::take(run.app.world_mut()));
    Ok(run.report)
}

fn spawn_run(seed: u64, catalogs: &mut Option<super::catalogs::Catalogs>) -> Result<Run, String> {
    let graph = super::graph::read_graph()?;
    let mut app = runtime::build_cached_app(catalogs);
    let copy = runtime::create_copy(&mut app);
    let party = crate::followers::simulator::spawn_party(
        app.world_mut(),
        copy,
        Position {
            x: graph.entrance[0],
            y: 0.0,
            z: graph.entrance[1],
        },
    )?;
    let actors = runtime::spawn_population(&mut app, copy);
    let missing: Vec<_> = graph
        .packs
        .iter()
        .flat_map(|pack| &pack.spawns)
        .filter(|spawn| !actors.contains_key(&spawn.guid))
        .map(|spawn| spawn.guid)
        .collect();
    if !missing.is_empty() {
        return Err(format!(
            "frozen graph contains sources absent from production Normal population: {missing:?}"
        ));
    }
    let last_owner_position = Vec3::new(graph.entrance[0], 0.0, graph.entrance[1]);
    Ok(Run {
        app,
        graph,
        party,
        actors,
        current_pack: None,
        last_progress: 0.0,
        event_cursor: 0,
        last_owner_position,
        report: Report {
            seed,
            status: "running".into(),
            failed_pack: None,
            bosses_killed: Vec::new(),
            simulated_seconds: 0.0,
            wall_seconds: 0.0,
            detail: String::new(),
            samples: Vec::new(),
            pulls: Vec::new(),
            performance: serde_json::Value::Null,
            terminal_units: serde_json::Value::Null,
        },
    })
}

impl Run {
    fn performance(
        &self,
        ticks: usize,
        started: std::time::Instant,
        setup: std::time::Duration,
        timings: &[std::time::Duration; 3],
    ) -> serde_json::Value {
        let elapsed = started.elapsed();
        let steady = elapsed.saturating_sub(setup).as_secs_f64();
        serde_json::json!({"ticks": ticks, "sim_seconds": self.now(),
            "setup_wall_seconds": setup.as_secs_f64(), "wall_seconds": elapsed.as_secs_f64(),
            "sim_per_wall": self.now() / elapsed.as_secs_f64(),
            "steady_sim_per_wall": self.now() / steady,
            "observation_wall_seconds": timings[0].as_secs_f64(),
            "control_wall_seconds": timings[1].as_secs_f64(),
            "update_wall_seconds": timings[2].as_secs_f64(),
            "allocated_entity_slots": self.app.world().entities().len(),
            "live_entities": self.app.world().entities().count_spawned(),
            "source_map_actors": self.actors.len(),
            "observed_events": self.app.world().resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0.len(),
            "samples": self.report.samples.len(), "memory": super::diagnostics::read_process_memory()})
    }

    fn inspect_units(&self) -> serde_json::Value {
        let units: Vec<_> = self
            .actors
            .iter()
            .map(|(guid, actor)| (Some(*guid), *actor))
            .chain(self.members().map(|actor| (None, actor)))
            .map(|(guid, actor)| super::diagnostics::inspect_unit(self.app.world(), guid, actor))
            .collect();
        serde_json::json!(units)
    }

    fn write_progress(
        &mut self,
        ticks: usize,
        started: std::time::Instant,
        setup: std::time::Duration,
        timings: &[std::time::Duration; 3],
    ) -> Result<(), String> {
        let mut performance = self.performance(ticks, started, setup, timings);
        performance["tank_movement_admission"] =
            serde_json::json!(crate::followers::simulator::inspect_tank_movement(
                self.app.world_mut(),
                self.party.tank
            ));
        println!(
            "PROGRESS seed={} ticks={} sim_s={:.2} ratio={} steady_ratio={} entities={} events={}",
            self.report.seed,
            ticks,
            self.now(),
            performance["sim_per_wall"],
            performance["steady_sim_per_wall"],
            performance["live_entities"],
            performance["observed_events"]
        );
        let directory = super::runtime::output_directory();
        std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
        let path = directory.join(format!("progress-{}.json", self.report.seed));
        let checkpoint = serde_json::json!({"seed": self.report.seed, "current_pack": self.current_pack,
            "performance": performance, "last_sample": self.report.samples.last(), "units": self.inspect_units()});
        std::fs::write(&path, serde_json::to_vec_pretty(&checkpoint).unwrap())
            .map_err(|error| format!("write {}: {error}", path.display()))
    }

    fn now(&self) -> f64 {
        self.app
            .world()
            .resource::<Time<Virtual>>()
            .elapsed_secs_f64()
    }

    fn members(&self) -> impl Iterator<Item = Entity> + '_ {
        self.party.members.iter().copied().chain([self.party.owner])
    }

    fn sample(&mut self) {
        let members = self
            .members()
            .map(|entity| {
                let world = self.app.world();
                let health = world.get::<Health>(entity);
                let mana = world.get::<Mana>(entity);
                let position = world.get::<Position>(entity);
                let name = world
                    .get::<shared::components::Npc>(entity)
                    .map(|npc| npc.name.clone())
                    .or_else(|| {
                        world
                            .get::<shared::components::Player>(entity)
                            .map(|player| player.name.clone())
                    });
                MemberSample {
                    entity: entity.to_bits(),
                    exists: world.get_entity(entity).is_ok(),
                    name,
                    health: health.map(|health| health.current),
                    max_health: health.map(|health| health.max),
                    mana: mana.map(|mana| mana.current),
                    max_mana: mana.map(|mana| mana.max),
                    position: position.map(|position| [position.x, position.z]),
                }
            })
            .collect();
        self.report.samples.push(Sample {
            at: self.now(),
            members,
        });
    }

    fn finished(&mut self) -> bool {
        let world = self.app.world();
        self.report.bosses_killed = BOSS_ENTRIES
            .into_iter()
            .filter(|entry| {
                self.graph
                    .packs
                    .iter()
                    .flat_map(|pack| &pack.spawns)
                    .any(|spawn| {
                        spawn.entry == *entry
                            && world
                                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                                .0
                                .iter()
                                .any(|event| {
                                    event.kind == shared::protocol::CombatLogKind::Death
                                        && event.target == Some(self.actors[&spawn.guid].to_bits())
                                })
                    })
            })
            .collect();
        if self.report.bosses_killed.len() == BOSS_ENTRIES.len() {
            self.report.status = "cleared".into();
            self.report.detail = "all three source boss actors died".into();
            return true;
        }
        if self
            .members()
            .all(|member| !crate::spell_cast::targets::is_alive(world, member))
        {
            self.report.status = "wiped".into();
            self.report.detail = "all five bots died".into();
            return true;
        }
        false
    }

    fn control(&mut self) -> Result<(), String> {
        if self.members().any(|member| {
            self.app
                .world()
                .get::<CombatStatus>(member)
                .is_some_and(|status| status.0)
        }) {
            self.app
                .world_mut()
                .entity_mut(self.party.owner)
                .remove::<MoveIntent>();
            return Ok(());
        }
        let Some((pack_id, target)) = self.next_pack() else {
            return Err(
                "no living hostile source pack remains, but boss deaths are incomplete".into(),
            );
        };
        self.current_pack = Some(pack_id);
        let target_position = *self.app.world().get::<Position>(target).unwrap();
        let tank_position = *self.app.world().get::<Position>(self.party.tank).unwrap();
        let (_, range) =
            crate::followers::simulator::opening_spell(self.app.world(), self.party.tank);
        if (target_position.x - tank_position.x).hypot(target_position.z - tank_position.z) > range
        {
            self.app
                .world_mut()
                .entity_mut(self.party.owner)
                .insert(MoveIntent(Vec3::new(
                    target_position.x,
                    0.0,
                    target_position.z,
                )));
            return Ok(());
        }
        self.app
            .world_mut()
            .entity_mut(self.party.owner)
            .remove::<MoveIntent>();
        let guids = self
            .graph
            .packs
            .iter()
            .find(|pack| pack.id == pack_id)
            .unwrap()
            .spawns
            .iter()
            .map(|spawn| spawn.guid)
            .collect();
        let (allowed, decision) = match crate::followers::simulator::pull(
            self.app.world_mut(),
            self.party.tank,
            &guids,
        ) {
            Ok(decision) => decision,
            Err(error) => {
                self.record_pull(pack_id, format!("Unavailable: {error}"));
                return Ok(());
            }
        };
        self.record_pull(pack_id, decision);
        if !allowed {
            return Ok(());
        }
        self.app
            .world_mut()
            .entity_mut(self.party.owner)
            .insert(crate::networking::CombatTarget(Some(target)));
        match crate::followers::simulator::cast_opening(
            self.app.world_mut(),
            self.party.tank,
            target,
        ) {
            Ok(()) => Ok(()),
            Err(
                crate::spell_cast::CastFailure::OnGlobalCooldown
                | crate::spell_cast::CastFailure::OnCooldown
                | crate::spell_cast::CastFailure::NoCharges
                | crate::spell_cast::CastFailure::InProgress,
            ) => Ok(()),
            Err(error) => Err(format!(
                "native tank opening failed on pack{pack_id}: {error:?}"
            )),
        }
    }

    fn next_pack(&self) -> Option<(u32, Entity)> {
        let world = self.app.world();
        let from = world.get::<Position>(self.party.tank)?;
        self.graph
            .packs
            .iter()
            .filter_map(|pack| {
                let target = pack
                    .spawns
                    .iter()
                    .map(|spawn| self.actors[&spawn.guid])
                    .find(|actor| {
                        crate::spell_cast::targets::is_alive(world, *actor)
                            && world
                                .get::<crate::creature_aggro::AggroRange>(*actor)
                                .is_some()
                            && crate::spell_cast::targets::is_hostile(
                                world,
                                self.party.tank,
                                *actor,
                            )
                    })?;
                let distance = (pack.position[0] - from.x).hypot(pack.position[1] - from.z);
                Some((pack.id, target, distance))
            })
            .min_by(|a, b| a.2.total_cmp(&b.2).then(a.0.cmp(&b.0)))
            .map(|(id, target, _)| (id, target))
    }

    fn record_pull(&mut self, pack: u32, decision: String) {
        if self
            .report
            .pulls
            .last()
            .is_some_and(|last| last.pack == pack && last.decision == decision)
        {
            return;
        }
        self.report.pulls.push(PullDecision {
            at: self.now(),
            pack,
            decision,
        });
    }

    fn stalled(&mut self) -> bool {
        let world = self.app.world();
        let events = &world
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        let members: Vec<_> = self.members().collect();
        let damage = events[self.event_cursor..].iter().any(|event| {
            event.kind == shared::protocol::CombatLogKind::Damage
                && event.amount > event.overflow.max(0)
                && event
                    .source
                    .and_then(Entity::try_from_bits)
                    .is_some_and(|source| {
                        members.contains(&crate::spell_cast::combat_event_owner(world, source))
                    })
        });
        self.event_cursor = events.len();
        let position = world.get::<Position>(self.party.owner).unwrap();
        let position = Vec3::new(position.x, 0.0, position.z);
        if damage || position != self.last_owner_position {
            self.last_progress = self.now();
        }
        self.last_owner_position = position;
        if self.now() - self.last_progress < STALL_SECONDS {
            return false;
        }
        self.report.status = "stalled".into();
        self.report.detail = self.report.pulls.last().map_or_else(
            || "no party damage or actual leader movement for 60s".into(),
            |pull| {
                format!(
                    "no party damage or actual leader movement for 60s; last pull: {}",
                    pull.decision
                )
            },
        );
        true
    }
}
