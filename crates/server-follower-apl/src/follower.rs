//! Persistent follower APL: precombat once, with a fresh encounter clock for each pull.
use crate::literal_rotation::{self, PrecombatPoll, execution::Execution};
use bevy::prelude::*;
use server::followers::AplControlled;
use server::simc_apl::Action;
use std::collections::VecDeque;

/// Precombat setup bound, as in the parity fixture: 30s of 20Hz ticks.
const PRECOMBAT_TICKS: u32 = 600;

#[cfg(test)]
mod lifecycle_tests;

/// A spec's production APL driver over the shared literal `Driver`.
pub trait SpecRotation: std::any::Any + Send + Sync {
    /// Spell IDs bound to actions in this rotation's actual program.
    fn action_spells(&self) -> Vec<u32>;

    /// Evaluate maintained default-list buffs while out of combat.
    fn drive_upkeep(&mut self, _world: &mut World, _actor: Entity) -> Result<(), String> {
        Ok(())
    }

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
    Idle,
}

/// The follower's APL persists for the lifetime of the follower.
#[derive(Component)]
pub struct FollowerApl {
    rotation: Box<dyn SpecRotation>,
    plan: VecDeque<(Action, Execution)>,
    pending_plan_cast: Option<(u32, Option<u64>)>,
    phase: Phase,
    combat_started_at: f64,
    maintained_buffs: Option<std::collections::BTreeSet<u32>>,
}

/// The APL failed this pull; the error was logged once and casting stops until the next pull.
#[derive(Component)]
pub struct FollowerAplFailed;

/// The headless owner temporarily directs its existing APL at the next chain target.
#[derive(Component)]
pub struct DirectedOwnerTarget(pub Entity);

impl FollowerApl {
    pub fn new(rotation: Box<dyn SpecRotation>) -> Self {
        Self {
            rotation,
            plan: VecDeque::new(),
            pending_plan_cast: None,
            phase: Phase::Precombat { ticks: 0 },
            combat_started_at: 0.0,
            maintained_buffs: None,
        }
    }

    pub fn new_for_world(world: &World, rotation: Box<dyn SpecRotation>) -> Self {
        let mut apl = Self::new(rotation);
        apl.initialize_maintained_buffs(world);
        apl
    }

    fn initialize_maintained_buffs(&mut self, world: &World) {
        self.maintained_buffs = Some(
            self.rotation
                .action_spells()
                .into_iter()
                .filter(|&spell| {
                    is_maintained_buff(world, spell)
                        && !crate::party_buffs::PARTY_BUFFS.contains(&spell)
                })
                .collect(),
        );
    }

    /// The production rotation that owns this follower's live decisions.
    pub fn rotation(&self) -> &dyn std::any::Any {
        self.rotation.as_ref()
    }

    pub fn in_combat(&self) -> bool {
        matches!(self.phase, Phase::Combat)
    }

    pub fn in_precombat(&self) -> bool {
        matches!(self.phase, Phase::Precombat { .. })
    }

    pub fn tick(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        if self.maintained_buffs.is_none() {
            self.initialize_maintained_buffs(world);
        }
        if targets.is_empty() {
            self.idle();
        }
        if !targets.is_empty() && !matches!(self.phase, Phase::Combat) {
            self.plan.clear();
            self.pending_plan_cast = None;
            self.combat_started_at = crate::spell_cast::now(world);
            self.rotation.start_combat_at(self.combat_started_at);
            self.phase = Phase::Combat;
        }
        if self.in_precombat() {
            self.prepare_tick(world, actor, targets)?;
            if self.in_precombat() {
                return Ok(());
            }
        }
        if targets.is_empty() {
            return self.maintain_buffs(world, actor);
        }
        let (gcd, cast) = literal_rotation::read_busy(world, actor);
        let elapsed = crate::spell_cast::now(world) - self.combat_started_at;
        if !gcd && !cast && crate::lust_policy::try_lust(world, actor, targets, elapsed)? {
            return Ok(());
        }
        let rotation = &mut self.rotation;
        literal_rotation::drive_tick(world, actor, |world| rotation.drive(world, actor, targets))
    }

    /// Advance the same production precombat with the targets of an upcoming pull.
    pub fn prepare_tick(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        if self.maintained_buffs.is_none() {
            self.initialize_maintained_buffs(world);
        }
        let Phase::Precombat { ticks } = &mut self.phase else {
            return Ok(());
        };
        *ticks += 1;
        if *ticks > PRECOMBAT_TICKS {
            return Err("precombat did not finish within 30s".into());
        }
        let (gcd, cast) = literal_rotation::read_busy(world, actor);
        if gcd || cast {
            return Ok(());
        }
        if world
            .get::<shared::components::CombatStatus>(actor)
            .is_some_and(|status| status.0)
        {
            self.plan.clear();
            self.pending_plan_cast = None;
            self.phase = Phase::Idle;
            return Ok(());
        }
        if self.advance_plan(world, actor)? {
            return Ok(());
        }
        match self.rotation.poll_precombat(world, actor, targets)? {
            PrecombatPoll::Waiting => Ok(()),
            PrecombatPoll::Selected { action, execution } => {
                self.execute_plan(world, actor, action, execution)
            }
            PrecombatPoll::Finished => {
                self.phase = Phase::Idle;
                Ok(())
            }
        }
    }

    fn idle(&mut self) {
        if matches!(self.phase, Phase::Combat) {
            self.phase = Phase::Idle;
        }
    }

    fn maintain_buffs(&mut self, world: &mut World, actor: Entity) -> Result<(), String> {
        let (gcd, cast) = literal_rotation::read_busy(world, actor);
        if gcd || cast {
            return Ok(());
        }
        if self.advance_plan(world, actor)? {
            return Ok(());
        }
        if crate::party_buffs::cast_lost_party_buff(world, actor)? {
            return Ok(());
        }
        let now = crate::spell_cast::now(world);
        let spells = self
            .maintained_buffs
            .as_ref()
            .expect("maintained buffs initialized before upkeep")
            .iter()
            .copied()
            .filter(|spell| {
                !world
                    .get::<server::auras::ActiveAuras>(actor)
                    .into_iter()
                    .flat_map(|auras| auras.iter())
                    .any(|aura| {
                        aura.caster == actor
                            && aura.spell_id == *spell
                            && aura.expires_at.is_none_or(|expiry| expiry > now + 1.0)
                    })
            })
            .collect::<std::collections::BTreeSet<_>>();
        if !spells.is_empty() {
            world.insert_resource(literal_rotation::BuffUpkeep { actor, spells });
            let decision = self.rotation.poll_precombat(world, actor, &[]);
            world.remove_resource::<literal_rotation::BuffUpkeep>();
            if let PrecombatPoll::Selected { action, execution } = decision? {
                return self.execute_plan(world, actor, action, execution);
            }
        }
        let spells = self.maintained_buffs.as_ref().unwrap().clone();
        world.insert_resource(literal_rotation::BuffUpkeep { actor, spells });
        let result = self.rotation.drive_upkeep(world, actor);
        world.remove_resource::<literal_rotation::BuffUpkeep>();
        result
    }

    fn advance_plan(&mut self, world: &mut World, actor: Entity) -> Result<bool, String> {
        if let Some((spell, previous)) = self.pending_plan_cast.take() {
            if successful_cast_sequence(world, actor, spell) == previous {
                self.plan.clear();
                return Ok(false);
            }
        }
        let Some((action, execution)) = self.plan.pop_front() else {
            return Ok(false);
        };
        if !literal_rotation::execution::composite_step_ready(world, actor, &execution)? {
            self.plan.clear();
            return Ok(false);
        }
        self.execute_plan_step(world, actor, action, execution)?;
        Ok(true)
    }

    fn execute_plan_step(
        &mut self,
        world: &mut World,
        actor: Entity,
        action: Action,
        execution: Execution,
    ) -> Result<(), String> {
        let marker = match &execution {
            Execution::Spell(spell, _, _) if !self.plan.is_empty() => {
                Some((*spell, successful_cast_sequence(world, actor, *spell)))
            }
            _ => None,
        };
        self.rotation.execute(world, actor, action, execution)?;
        self.pending_plan_cast = marker;
        Ok(())
    }

    /// Each composite step waits for the previous step's cast and GCD, as in the fixture.
    pub fn execute_plan(
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
        self.execute_plan_step(world, actor, action, step)
    }
}

fn successful_cast_sequence(world: &World, actor: Entity, spell: u32) -> Option<u64> {
    world
        .get::<server::spell_cast::SpellHistory>(actor)
        .and_then(|history| history.successful_cast_sequence(spell))
}

/// Only positive self/ally aura actions without a cooldown are maintained.
fn is_maintained_buff(world: &World, spell_id: u32) -> bool {
    let store = world.resource::<server::spell_info::SpellStore>();
    let Some(spell) = store.get(spell_id) else {
        return false;
    };
    if spell
        .cooldowns
        .as_ref()
        .is_some_and(|cooldown| cooldown.recovery_ms != 0 || cooldown.category_recovery_ms != 0)
        || spell
            .categories
            .as_ref()
            .is_some_and(|category| category.max_charges != 0)
        || matches!(
            server::spell_cast::targets::explicit_requirement(spell.effects.iter()),
            Some(server::spell_cast::targets::Relation::Enemy)
        )
    {
        return false;
    }
    has_positive_aura(store, spell_id, &mut std::collections::BTreeSet::new())
}

fn has_positive_aura(
    store: &server::spell_info::SpellStore,
    spell_id: u32,
    visited: &mut std::collections::BTreeSet<u32>,
) -> bool {
    if !visited.insert(spell_id) {
        return false;
    }
    let Some(spell) = store.get(spell_id) else {
        return false;
    };
    let aura_mask = server::auras::aura_effect_mask(spell);
    let positive = aura_mask & !server::spell_info::positivity::negative_effects(store, spell);
    let duration = spell
        .misc
        .as_ref()
        .and_then(|misc| misc.duration)
        .is_some_and(|duration| duration.duration_ms != 0);
    (positive != 0 && duration)
        || spell.effects.iter().any(|effect| {
            effect.trigger_spell != 0 && has_positive_aura(store, effect.trigger_spell, visited)
        })
}

/// Drive every APL-controlled follower after the follower brain positioned it.
pub fn drive_followers(world: &mut World) {
    let units = server::system_states::entities_with::<With<AplControlled>>(world);
    for unit in units {
        if let Some(follower) = server::followers::acting_follower(world, unit) {
            drive_follower(world, unit, follower.profile.spec_id);
        }
    }
}

fn drive_follower(world: &mut World, unit: Entity, spec: u32) {
    if !crate::spell_cast::targets::is_alive(world, unit) {
        if let Some(mut apl) = world.get_mut::<FollowerApl>(unit) {
            apl.idle();
        }
        return;
    }
    let targets = if let Some(target) = world.get::<DirectedOwnerTarget>(unit) {
        vec![target.0]
    } else {
        ordered_targets(world, unit)
    };
    if targets.is_empty() {
        world
            .entity_mut(unit)
            .remove::<server::followers::retreat::RetreatSetup>();
        if let Some(mut apl) = world.get_mut::<FollowerApl>(unit) {
            apl.idle();
        }
        if let Some(target) = injured_recovery_target(world, unit) {
            if world.get::<FollowerAplFailed>(unit).is_some() {
                return;
            }
            match heal_recovery_target(world, unit, spec, target) {
                Ok(()) => {}
                Err(error) if server::simc_apl::is_not_ready(&error) => {
                    debug!(?unit, "follower recovery healing waits one tick: {error}");
                }
                Err(error) => {
                    error!(?unit, "follower recovery healing failed: {error}");
                    world.entity_mut(unit).insert(FollowerAplFailed);
                }
            }
        } else {
            world.entity_mut(unit).remove::<FollowerAplFailed>();
        }
    }
    if world.get::<FollowerAplFailed>(unit).is_some() {
        return;
    }
    let apl = match world.entity_mut(unit).take::<FollowerApl>() {
        Some(apl) => Ok(apl),
        None => crate::registry::load(world, unit, spec)
            .map(|rotation| FollowerApl::new_for_world(world, rotation)),
    };
    match apl {
        Ok(mut apl) => {
            let result = apl.tick(world, unit, &targets);
            world.entity_mut(unit).insert(apl);
            match result {
                Ok(()) => {}
                Err(error) if server::simc_apl::is_not_ready(&error) => {
                    debug!(?unit, "follower APL waits one tick: {error}");
                }
                Err(error) => {
                    error!("Class follower {unit} spec{spec} APL stopped for this pull: {error}");
                    world.entity_mut(unit).insert(FollowerAplFailed);
                }
            }
        }
        Err(error) => {
            error!("Class follower {unit} spec{spec} APL failed to load: {error}");
            world.entity_mut(unit).insert(FollowerAplFailed);
        }
    }
}

fn injured_recovery_target(world: &mut World, unit: Entity) -> Option<Entity> {
    let follower = server::followers::acting_follower(world, unit)?;
    if follower.profile.role != shared::group::GroupRole::Healer {
        return None;
    }
    healing_party(world, unit)
        .into_iter()
        .filter_map(|member| {
            let health = world.get::<shared::components::Health>(member)?;
            (health.current > 0.0 && health.current < health.max)
                .then_some((member, health.current / health.max))
        })
        .min_by(|left, right| left.1.total_cmp(&right.1))
        .map(|(member, _)| member)
}

fn heal_recovery_target(
    world: &mut World,
    healer: Entity,
    spec: u32,
    target: Entity,
) -> Result<(), String> {
    use server::spell_cast::CastFailure;
    let spell = match spec {
        65 => 82326,    // Holy Light
        105 => 8936,    // Regrowth
        256 => 2061,    // Flash Heal
        257 => 2060,    // Heal
        264 => 77472,   // Healing Wave
        270 => 116670,  // Vivify
        1468 => 361469, // Living Flame
        _ => return Err(format!("no recovery heal for healer spec {spec}")),
    };
    match server::spell_cast::creature_cast(world, healer, spell, Some(target), false) {
        Ok(())
        | Err(
            CastFailure::InProgress
            | CastFailure::OnGlobalCooldown
            | CastFailure::OnCooldown
            | CastFailure::Moving
            | CastFailure::OutOfRange
            | CastFailure::LineOfSight
            | CastFailure::CrowdControlled(_)
            | CastFailure::NotEnoughPower(_),
        ) => Ok(()),
        Err(error) => Err(format!("spell {spell}: {error:?}")),
    }
}

/// The follower's party, owner included; empty for an actor that is not a follower.
pub fn party_of(world: &mut World, actor: Entity) -> Vec<Entity> {
    match server::followers::acting_follower(world, actor).map(|follower| follower.owner) {
        Some(owner) => server::followers::party_units(world, owner),
        None => Vec::new(),
    }
}

/// Party members and nearby, data-backed healing encounter objectives.
pub fn healing_party(world: &mut World, actor: Entity) -> Vec<Entity> {
    let mut party = party_of(world, actor);
    let Some(follower) = server::followers::acting_follower(world, actor) else {
        return party;
    };
    if follower.profile.role != shared::group::GroupRole::Healer {
        return party;
    }
    // Each range comes from the native direct-heal spell used by this spec's APL.
    let spell_id = match follower.profile.spec_id {
        65 => 82326,
        105 => 8936,
        256 => 17,
        257 => 2060,
        264 => 77472,
        270 => 116670,
        1468 => 361469,
        _ => return party,
    };
    let Some(range) = world
        .get_resource::<server::spell_info::SpellStore>()
        .and_then(|store| store.get(spell_id))
        .map(|spell| server::spell_cast::targets::spell_range(spell, false).1)
    else {
        return party;
    };
    let Some(map) = world.get::<server::maps::InMap>(actor).copied() else {
        return party;
    };
    let Some(origin) = server::spell_cast::targets::position(world, actor) else {
        return party;
    };
    let objectives = world
        .get_resource::<server::maps::MapStore>()
        .map(|store| store.encounter_heal_objectives.clone())
        .unwrap_or_default();
    let candidates: Vec<Entity> = world
        .query::<(
            Entity,
            &server::maps::InMap,
            &shared::components::Npc,
            &shared::components::Health,
        )>()
        .iter(world)
        .filter(|(_, candidate_map, npc, health)| {
            **candidate_map == map
                && health.current > 0.0
                && health.current < health.max
                && objectives.contains_key(&(map.map_id, npc.template_id))
        })
        .map(|(entity, _, _, _)| entity)
        .collect();
    for target in candidates {
        let in_range = server::spell_cast::targets::position(world, target)
            .is_some_and(|position| position.distance(origin) <= range);
        let friendly = server::spell_cast::targets::relation_matches(
            world,
            actor,
            target,
            server::spell_cast::targets::Relation::Ally,
        );
        if in_range
            && friendly
            && server::scripted_ai::heal_objective::accepts_healing(world, target, actor)
        {
            party.push(target);
        }
    }
    party
}

/// The first tank in `party`.
pub fn party_tank(world: &World, party: &[Entity]) -> Option<Entity> {
    party
        .iter()
        .copied()
        .find(|&unit| server::followers::is_tank(world, unit))
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
        crate::action_metadata::install_provider(app);
        app.add_systems(
            FixedUpdate,
            drive_followers
                .after(server::followers::update_ai)
                .after(server::auras::AuraTickSet),
        );
    }
}
