//! Per-pull APL lifecycle for a class follower: its precombat list once, then the default list.
use crate::literal_rotation::{self, PrecombatPoll, execution::Execution};
use bevy::prelude::*;
use server::followers::{AplControlled, Follower};
use server::simc_apl::Action;
use std::collections::VecDeque;

/// Precombat setup bound, as in the parity fixture: 30s of 20Hz ticks.
const PRECOMBAT_TICKS: u32 = 600;

/// A spec's production APL driver over the shared literal `Driver`.
pub trait SpecRotation: Send + Sync {
    /// Next precombat decision. `Waiting` covers a row the adapter executed itself.
    fn poll_precombat(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<PrecombatPoll, String>;
    /// Execute one step of a decision returned by `poll_precombat`.
    fn execute(
        &mut self,
        world: &mut World,
        actor: Entity,
        action: Action,
        execution: Execution,
    ) -> Result<(), String>;
    fn start_combat_at(&mut self, started_at: f64);
    /// One default-list decision against the engaged enemies, primary target first.
    fn drive(&mut self, world: &mut World, actor: Entity, targets: &[Entity])
    -> Result<(), String>;
}

enum Phase {
    Precombat { ticks: u32 },
    Combat,
}

/// The follower's APL for the current pull; dropped when the pull ends.
#[derive(Component)]
pub struct FollowerApl {
    rotation: Box<dyn SpecRotation>,
    plan: VecDeque<(Action, Execution)>,
    phase: Phase,
}

/// The APL failed this pull; the error was logged once and casting stops until the next pull.
#[derive(Component)]
pub struct FollowerAplFailed;

impl FollowerApl {
    pub fn new(rotation: Box<dyn SpecRotation>) -> Self {
        Self {
            rotation,
            plan: VecDeque::new(),
            phase: Phase::Precombat { ticks: 0 },
        }
    }

    pub fn in_combat(&self) -> bool {
        matches!(self.phase, Phase::Combat)
    }

    pub fn tick(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        if let Phase::Precombat { ticks } = &mut self.phase {
            *ticks += 1;
            if *ticks > PRECOMBAT_TICKS {
                return Err("precombat did not finish within 30s".into());
            }
            let (gcd, cast) = literal_rotation::read_busy(world, actor);
            if gcd || cast {
                return Ok(());
            }
            if let Some((action, execution)) = self.plan.pop_front() {
                return self.rotation.execute(world, actor, action, execution);
            }
            match self.rotation.poll_precombat(world, actor, targets)? {
                PrecombatPoll::Waiting => return Ok(()),
                PrecombatPoll::Selected { action, execution } => {
                    return self.execute_plan(world, actor, action, execution);
                }
                PrecombatPoll::Finished => {
                    self.rotation.start_combat_at(crate::spell_cast::now(world));
                    self.phase = Phase::Combat;
                }
            }
        }
        let rotation = &mut self.rotation;
        literal_rotation::drive_tick(world, actor, |world| rotation.drive(world, actor, targets))
    }

    /// Each composite step waits for the previous step's cast and GCD, as in the fixture.
    fn execute_plan(
        &mut self,
        world: &mut World,
        actor: Entity,
        action: Action,
        execution: Execution,
    ) -> Result<(), String> {
        let steps = match execution {
            Execution::Composite(steps) => steps,
            execution => vec![execution],
        };
        self.plan = steps
            .into_iter()
            .map(|step| (action.clone(), step))
            .collect();
        let (action, step) = self
            .plan
            .pop_front()
            .ok_or("selected native composite plan is empty")?;
        self.rotation.execute(world, actor, action, step)
    }
}

/// Drive every APL-controlled follower after the follower brain positioned it.
pub fn drive_followers(world: &mut World) {
    let units: Vec<_> = world
        .query_filtered::<(Entity, &Follower), With<AplControlled>>()
        .iter(world)
        .map(|(unit, follower)| (unit, follower.profile.spec_id))
        .collect();
    for (unit, spec) in units {
        drive_follower(world, unit, spec);
    }
}

fn drive_follower(world: &mut World, unit: Entity, spec: u32) {
    let targets = if crate::spell_cast::targets::is_alive(world, unit) {
        ordered_targets(world, unit)
    } else {
        Vec::new()
    };
    if targets.is_empty() {
        world
            .entity_mut(unit)
            .remove::<(FollowerApl, FollowerAplFailed)>();
        return;
    }
    if world.get::<FollowerAplFailed>(unit).is_some() {
        return;
    }
    let apl = match world.entity_mut(unit).take::<FollowerApl>() {
        Some(apl) => Ok(apl),
        None => crate::registry::load(world, unit, spec).map(FollowerApl::new),
    };
    let result = apl.and_then(|mut apl| apl.tick(world, unit, &targets).map(|()| apl));
    match result {
        Ok(apl) => {
            world.entity_mut(unit).insert(apl);
        }
        Err(error) => {
            error!("Class follower {unit} spec{spec} APL stopped for this pull: {error}");
            world.entity_mut(unit).insert(FollowerAplFailed);
        }
    }
}

/// The follower's party, owner included; empty for an actor that is not a follower.
pub fn party_of(world: &mut World, actor: Entity) -> Vec<Entity> {
    match world.get::<Follower>(actor).map(|follower| follower.owner) {
        Some(owner) => server::followers::party_units(world, owner),
        None => Vec::new(),
    }
}

/// Engaged enemies with the brain's current combat target first.
fn ordered_targets(world: &mut World, unit: Entity) -> Vec<Entity> {
    let mut targets = server::followers::engaged_targets(world, unit);
    let primary = world
        .get::<crate::networking::CombatTarget>(unit)
        .and_then(|target| target.0);
    if let Some(index) = primary.and_then(|primary| targets.iter().position(|&t| t == primary)) {
        let primary = targets.remove(index);
        targets.insert(0, primary);
    }
    targets
}

pub struct FollowerAplPlugin;

impl Plugin for FollowerAplPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            drive_followers.after(server::followers::update_ai),
        );
    }
}
