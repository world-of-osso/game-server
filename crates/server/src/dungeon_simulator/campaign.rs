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
    name: String,
    health: f32,
    max_health: f32,
    mana: Option<f32>,
    max_mana: Option<f32>,
    position: [f32; 2],
}

#[derive(Serialize)]
pub(super) struct Sample {
    at: f64,
    members: Vec<MemberSample>,
}

#[derive(Serialize)]
pub(super) struct PullDecision {
    at: f64,
    pack: u32,
    decision: String,
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
}

struct Run {
    app: App,
    graph: Graph,
    party: crate::followers::simulator::Party,
    actors: BTreeMap<u32, Entity>,
    report: Report,
    current_pack: Option<u32>,
    last_progress: f64,
    last_health: f32,
}

pub(super) fn run(seed: u64) -> Result<Report, String> {
    let _rng_scope = crate::random::seed_scope(seed);
    let started = std::time::Instant::now();
    let mut run = spawn_run(seed)?;
    for tick in 0..MAX_TICKS {
        if tick % 10 == 0 {
            run.sample();
        }
        if run.finished() {
            break;
        }
        if let Err(error) = run.control() {
            run.report.status = "blocked".into();
            run.report.detail = error;
            break;
        }
        run.app.update();
        if run.stalled() {
            break;
        }
    }
    run.sample();
    run.report.simulated_seconds = run.now();
    run.report.wall_seconds = started.elapsed().as_secs_f64();
    run.report.failed_pack = (run.report.status != "cleared")
        .then_some(run.current_pack)
        .flatten();
    if run.report.status == "running" {
        run.report.status = "timeout".into();
        run.report.detail = "one-hour simulated bound reached".into();
    }
    Ok(run.report)
}

fn spawn_run(seed: u64) -> Result<Run, String> {
    let graph = super::graph::read_graph()?;
    let mut app = runtime::build_app();
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
    let mut actors = BTreeMap::new();
    for pack in &graph.packs {
        let units = runtime::spawn_pack(&mut app, copy, pack, seed ^ u64::from(pack.id));
        actors.extend(
            pack.spawns
                .iter()
                .zip(units)
                .map(|(source, actor)| (source.guid, actor)),
        );
    }
    Ok(Run {
        app,
        graph,
        party,
        actors,
        current_pack: None,
        last_progress: 0.0,
        last_health: 0.0,
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
        },
    })
}

impl Run {
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
                let health = world
                    .get::<Health>(entity)
                    .expect("admitted bot retains health");
                let mana = world.get::<Mana>(entity);
                let position = world.get::<Position>(entity).unwrap();
                let name = world
                    .get::<shared::components::Npc>(entity)
                    .map(|npc| npc.name.clone())
                    .or_else(|| {
                        world
                            .get::<shared::components::Player>(entity)
                            .map(|player| player.name.clone())
                    })
                    .unwrap();
                MemberSample {
                    entity: entity.to_bits(),
                    name,
                    health: health.current,
                    max_health: health.max,
                    mana: mana.map(|mana| mana.current),
                    max_mana: mana.map(|mana| mana.max),
                    position: [position.x, position.z],
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
            self.last_progress = self.now();
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
            Ok(()) => {
                self.last_progress = self.now();
                Ok(())
            }
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
        let health: f32 = self
            .actors
            .values()
            .filter_map(|actor| self.app.world().get::<Health>(*actor))
            .map(|hp| hp.current)
            .sum();
        if health != self.last_health {
            self.last_health = health;
            self.last_progress = self.now();
        }
        if self.now() - self.last_progress < STALL_SECONDS {
            return false;
        }
        self.report.status = "stalled".into();
        self.report.detail = self.report.pulls.last().map_or_else(
            || "no damage or admitted movement for 60s".into(),
            |pull| pull.decision.clone(),
        );
        true
    }
}
