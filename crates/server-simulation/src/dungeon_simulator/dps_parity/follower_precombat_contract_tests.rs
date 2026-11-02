//! Production follower path for converted precombat adapters across pulls.
use super::follower_dps_diagnostic::prepare_class_follower;
use super::*;
use crate::simc_apl_native::Encounter;
use server::simc_apl::Action;
use server_follower_apl::literal_rotation::{PrecombatPoll, execution::Execution};
use server_follower_apl::{FollowerApl, FollowerAplFailed, SpecRotation};
use shared::protocol::CombatLogKind;
use std::sync::{Arc, Mutex};

const LEVEL: u8 = 90;
const PRECOMBAT_TICKS: usize = 600;
const PULL_TICKS: usize = 240;
const GAP_TICKS: usize = 60;
const COMPARED_CASTS: usize = 6;

#[derive(Clone, Debug, PartialEq)]
enum Note {
    Poll {
        targets: usize,
        outcome: &'static str,
    },
    Execute(String),
    Pull,
    Drive {
        targets: usize,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Unit {
    Actor,
    Pull(usize),
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Cast {
    tick: usize,
    spell: u32,
    target: Unit,
}

/// The pre-contract entry point: one precombat step executing inside the adapter.
trait LegacyPrecombat: SpecRotation {
    fn legacy_step(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<bool, String>;
}

macro_rules! legacy_precombat {
    ($rotation:ty) => {
        impl LegacyPrecombat for $rotation {
            fn legacy_step(
                &mut self,
                world: &mut World,
                actor: Entity,
                targets: &[Entity],
            ) -> Result<bool, String> {
                self.precombat_step(world, actor, targets)
            }
        }
    };
}

legacy_precombat!(server_follower_apl::elemental_rotation::Rotation);
legacy_precombat!(server_follower_apl::windwalker_rotation::Rotation);
legacy_precombat!(server_follower_apl::devourer_rotation::Rotation);

/// The spec rotation inside the follower's `FollowerApl`, recording each trait call.
struct Recorded<R> {
    rotation: R,
    notes: Arc<Mutex<Vec<Note>>>,
    legacy: bool,
}

impl<R> Recorded<R> {
    fn note(&self, note: Note) {
        self.notes.lock().unwrap().push(note);
    }
}

impl<R: LegacyPrecombat> SpecRotation for Recorded<R> {
    fn action_spells(&self) -> Vec<u32> {
        self.rotation.action_spells()
    }

    fn poll_precombat(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<PrecombatPoll, String> {
        let poll = if !self.legacy {
            self.rotation.poll_precombat(world, actor, targets)?
        } else if self.rotation.legacy_step(world, actor, targets)? {
            PrecombatPoll::Waiting
        } else {
            PrecombatPoll::Finished
        };
        let outcome = match poll {
            PrecombatPoll::Waiting => "waiting",
            PrecombatPoll::Selected { .. } => "selected",
            PrecombatPoll::Finished => "finished",
        };
        self.note(Note::Poll {
            targets: targets.len(),
            outcome,
        });
        Ok(poll)
    }

    fn execute(
        &mut self,
        world: &mut World,
        actor: Entity,
        action: Action,
        execution: Execution,
    ) -> Result<(), String> {
        self.note(Note::Execute(action.name.clone()));
        self.rotation.execute(world, actor, action, execution)
    }

    fn start_combat_at(&mut self, started_at: f64) {
        self.note(Note::Pull);
        self.rotation.start_combat_at(started_at);
    }

    fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        self.note(Note::Drive {
            targets: targets.len(),
        });
        self.rotation.drive(world, actor, targets)
    }
}

struct Outcome {
    notes: Vec<Note>,
    casts: Vec<Cast>,
    pulls: [usize; 2],
    /// Owners of the recorded rotation still alive: this test and the follower's `FollowerApl`.
    owners: usize,
}

struct Scenario {
    app: App,
    spec: u32,
    actor: Entity,
    owner: Entity,
    pulls: Vec<Entity>,
    notes: Arc<Mutex<Vec<Note>>>,
    casts: Vec<Cast>,
    seen: usize,
    tick: usize,
}

impl Scenario {
    fn advance(&mut self, ticks: usize) {
        for _ in 0..ticks {
            self.app.update();
            self.tick += 1;
            assert!(
                self.app
                    .world()
                    .get::<FollowerAplFailed>(self.actor)
                    .is_none(),
                "spec{} follower APL failed at tick {}",
                self.spec,
                self.tick
            );
            self.read_casts();
        }
    }

    fn read_casts(&mut self) {
        let events = &self
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        for event in &events[self.seen..] {
            if event.source != Some(self.actor.to_bits())
                || event.kind != CombatLogKind::CastSuccess
            {
                continue;
            }
            let Some(spell) = event.spell_id else {
                continue;
            };
            let target = match event.target {
                Some(target) if target == self.actor.to_bits() => Unit::Actor,
                Some(target) => self
                    .pulls
                    .iter()
                    .position(|pull| pull.to_bits() == target)
                    .map_or(Unit::Other, Unit::Pull),
                None => Unit::Other,
            };
            self.casts.push(Cast {
                tick: self.tick,
                spell,
                target,
            });
        }
        self.seen = events.len();
    }

    fn finish_precombat(&mut self) {
        for _ in 0..PRECOMBAT_TICKS {
            self.advance(1);
            let finished = self.notes.lock().unwrap().iter().any(|note| {
                matches!(
                    note,
                    Note::Poll {
                        outcome: "finished",
                        ..
                    }
                )
            });
            if finished {
                return;
            }
        }
        panic!("spec{} precombat did not finish out of combat", self.spec);
    }

    /// The owner starts the pull, as `follower_dummy_uses_class_apl_to_cast` does.
    fn pull(&mut self, target: Entity) -> usize {
        self.pulls.push(target);
        self.app.world_mut().entity_mut(self.owner).insert((
            crate::networking::CombatTarget(Some(target)),
            crate::combat::AutoAttack::new(target),
        ));
        crate::combat_refs::set_in_combat_with(self.app.world_mut(), self.owner, target);
        let started = self.tick;
        self.advance(PULL_TICKS);
        crate::combat::kill_unit(self.app.world_mut(), self.owner, target);
        self.app
            .world_mut()
            .entity_mut(self.owner)
            .remove::<(crate::networking::CombatTarget, crate::combat::AutoAttack)>();
        self.advance(GAP_TICKS);
        started
    }
}

/// One out-of-combat period and two pulls driven only by the production follower system.
fn run_follower<R: LegacyPrecombat + 'static>(
    spec: u32,
    load: fn(&mut World, Entity) -> Result<R, String>,
    legacy: bool,
) -> Outcome {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("snapshot path");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, first, _) = prepare_class_follower(app, copy, spec, LEVEL);
    assert!(
        app.world()
            .get::<crate::followers::AplControlled>(actor)
            .is_some()
    );
    let owner = app
        .world()
        .get::<crate::followers::Follower>(actor)
        .unwrap()
        .owner;
    let notes = Arc::new(Mutex::new(Vec::new()));
    let rotation = load(app.world_mut(), actor).unwrap();
    app.world_mut()
        .entity_mut(actor)
        .insert(FollowerApl::new(Box::new(Recorded {
            rotation,
            notes: notes.clone(),
            legacy,
        })));
    let mut scenario = Scenario {
        app,
        spec,
        actor,
        owner,
        pulls: Vec::new(),
        notes,
        casts: Vec::new(),
        seen: 0,
        tick: 0,
    };
    scenario.finish_precombat();
    scenario.advance(GAP_TICKS);
    let first = scenario.pull(first);
    // The second pull's dummy spawns after the first, so cleave cannot engage it early.
    let second = spawn_level_dummy(&mut scenario.app, actor, LEVEL, 1).unwrap();
    let pulls = [first, scenario.pull(second)];
    assert!(
        !scenario
            .app
            .world()
            .get::<FollowerApl>(actor)
            .expect("the follower keeps its APL after both pulls")
            .in_combat()
    );
    let notes = scenario.notes.lock().unwrap().clone();
    Outcome {
        notes,
        casts: scenario.casts,
        pulls,
        owners: Arc::strong_count(&scenario.notes),
    }
}

fn pull_casts(outcome: &Outcome, index: usize) -> Vec<Cast> {
    let start = outcome.pulls[index];
    outcome
        .casts
        .iter()
        .copied()
        .filter(|cast| cast.tick > start && cast.tick <= start + PULL_TICKS)
        .take(COMPARED_CASTS)
        .collect()
}

fn assert_follower_contract<R: LegacyPrecombat + 'static>(
    spec: u32,
    load: fn(&mut World, Entity) -> Result<R, String>,
) {
    let follower = run_follower(spec, load, false);
    let legacy = run_follower(spec, load, true);
    assert_eq!(
        follower.owners, 2,
        "spec{spec}: the follower must keep the loaded rotation across pulls"
    );
    let pulls: Vec<_> = follower
        .notes
        .iter()
        .enumerate()
        .filter(|(_, note)| **note == Note::Pull)
        .map(|(index, _)| index)
        .collect();
    assert_eq!(pulls.len(), 2, "spec{spec}: {:?}", follower.notes);
    let (precombat, combat) = follower.notes.split_at(pulls[0]);
    assert!(
        precombat
            .iter()
            .any(|note| matches!(note, Note::Execute(_))),
        "spec{spec}: precombat must execute a selected row: {precombat:?}"
    );
    assert!(
        precombat.iter().all(|note| match note {
            Note::Poll { targets, .. } => *targets == 0,
            _ => true,
        }),
        "spec{spec}: precombat polls must be out of combat"
    );
    assert!(
        combat
            .iter()
            .all(|note| matches!(note, Note::Pull | Note::Drive { .. })),
        "spec{spec}: no precombat poll or row may run after the first pull: {:?}",
        combat
            .iter()
            .filter(|note| !matches!(note, Note::Pull | Note::Drive { .. }))
            .collect::<Vec<_>>()
    );
    let second = pulls[1] - pulls[0];
    for drives in [&combat[..second], &combat[second..]] {
        assert!(
            drives
                .iter()
                .any(|note| matches!(note, Note::Drive { targets } if *targets > 0)),
            "spec{spec}: each pull must drive the persisted rotation"
        );
    }
    assert!(
        !legacy
            .notes
            .iter()
            .any(|note| matches!(note, Note::Execute(_))),
        "spec{spec}: the legacy control executes inside its precombat step"
    );
    let precombat_casts = |outcome: &Outcome| -> Vec<Cast> {
        outcome
            .casts
            .iter()
            .copied()
            .filter(|cast| cast.tick <= outcome.pulls[0])
            .collect()
    };
    assert_eq!(
        precombat_casts(&follower),
        precombat_casts(&legacy),
        "spec{spec}: precombat casts"
    );
    for index in 0..2 {
        let casts = pull_casts(&follower, index);
        assert_eq!(
            casts.len(),
            COMPARED_CASTS,
            "spec{spec} pull{index}: {:?}",
            follower.casts
        );
        assert!(
            casts.iter().any(|cast| cast.target == Unit::Pull(index)),
            "spec{spec} pull{index} must cast at its pull: {casts:?}"
        );
        assert_eq!(
            casts,
            pull_casts(&legacy, index),
            "spec{spec} pull{index}: first combat decisions"
        );
    }
}

fn dungeon(world: &World) -> Encounter {
    Encounter::dungeon(crate::spell_cast::now(world))
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn elemental_follower_runs_precombat_contract_across_two_pulls() {
    assert_follower_contract(262, |world, actor| {
        let profile = server_follower_apl::profiles::profile(262)?;
        let encounter = dungeon(world);
        server_follower_apl::elemental_rotation::Rotation::load(world, actor, encounter, &profile)
    });
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn windwalker_follower_runs_precombat_contract_across_two_pulls() {
    assert_follower_contract(269, |world, actor| {
        let encounter = dungeon(world);
        server_follower_apl::windwalker_rotation::Rotation::load(world, actor, encounter)
    });
}

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn devourer_follower_runs_precombat_contract_across_two_pulls() {
    assert_follower_contract(1480, |world, actor| {
        let encounter = dungeon(world);
        server_follower_apl::devourer_rotation::Rotation::load(world, actor, encounter)
    });
}
