//! Persistent follower APL: precombat once, with a fresh encounter clock for each pull.
use crate::literal_rotation::{self, PrecombatPoll, execution::Execution};
use bevy::prelude::*;
use server::followers::AplControlled;
use server::simc_apl::Action;
use std::collections::VecDeque;

/// Precombat setup bound, as in the parity fixture: 30s of 20Hz ticks.
const PRECOMBAT_TICKS: u32 = 600;

/// A spec's production APL driver over the shared literal `Driver`.
pub trait SpecRotation: Send + Sync {
    /// Spell IDs bound to actions in this rotation's actual program.
    fn action_spells(&self) -> Vec<u32>;

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

/// The follower's APL persists across pulls until death or a precombat buff expires.
#[derive(Component)]
pub struct FollowerApl {
    rotation: Box<dyn SpecRotation>,
    plan: VecDeque<(Action, Execution)>,
    phase: Phase,
    combat_started_at: f64,
    precombat_buffs: std::collections::BTreeMap<u32, f64>,
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
            phase: Phase::Precombat { ticks: 0 },
            combat_started_at: 0.0,
            precombat_buffs: Default::default(),
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
        if matches!(self.phase, Phase::Idle) {
            self.combat_started_at = crate::spell_cast::now(world);
            self.rotation.start_combat_at(self.combat_started_at);
            self.phase = Phase::Combat;
        }
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
                    let actions = self.rotation.action_spells();
                    self.precombat_buffs = world
                        .get::<server::auras::ActiveAuras>(actor)
                        .into_iter()
                        .flat_map(|auras| auras.iter())
                        .filter(|aura| {
                            aura.caster == actor
                                && !aura.passive
                                && !aura.harmful
                                && actions.contains(&aura.spell_id)
                                && is_maintained_buff(world, aura.spell_id)
                        })
                        .filter_map(|aura| aura.expires_at.map(|at| (aura.spell_id, at)))
                        .collect();
                    self.combat_started_at = crate::spell_cast::now(world);
                    self.rotation.start_combat_at(self.combat_started_at);
                    self.phase = Phase::Combat;
                }
            }
        }
        let (gcd, cast) = literal_rotation::read_busy(world, actor);
        let elapsed = crate::spell_cast::now(world) - self.combat_started_at;
        if !gcd && !cast && crate::lust_policy::try_lust(world, actor, targets, elapsed)? {
            return Ok(());
        }
        let rotation = &mut self.rotation;
        literal_rotation::drive_tick(world, actor, |world| rotation.drive(world, actor, targets))
    }

    fn idle(&mut self) {
        if matches!(self.phase, Phase::Combat) {
            self.phase = Phase::Idle;
        }
    }

    fn precombat_buff_expired(&mut self, world: &World, actor: Entity) -> bool {
        let now = crate::spell_cast::now(world);
        self.precombat_buffs.iter_mut().any(|(spell, deadline)| {
            let current = world
                .get::<server::auras::ActiveAuras>(actor)
                .into_iter()
                .flat_map(|auras| auras.iter())
                .find(|aura| aura.caster == actor && aura.spell_id == *spell);
            match current {
                Some(aura) => {
                    if let Some(at) = aura.expires_at {
                        *deadline = at;
                    }
                    false
                }
                None => now >= *deadline,
            }
        })
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
        self.rotation.execute(world, actor, action, step)
    }
}

/// Cooldown-limited precombat effects are combat actions, not maintained buffs.
fn is_maintained_buff(world: &World, spell_id: u32) -> bool {
    world
        .resource::<server::spell_info::SpellStore>()
        .get(spell_id)
        .is_some_and(|spell| {
            spell.cooldowns.as_ref().is_none_or(|cooldown| {
                cooldown.recovery_ms == 0 && cooldown.category_recovery_ms == 0
            }) && spell
                .categories
                .as_ref()
                .is_none_or(|category| category.max_charges == 0)
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
        world
            .entity_mut(unit)
            .remove::<(FollowerApl, FollowerAplFailed)>();
        return;
    }
    let targets = if let Some(target) = world.get::<DirectedOwnerTarget>(unit) {
        vec![target.0]
    } else {
        ordered_targets(world, unit)
    };
    if targets.is_empty() {
        if let Some(mut apl) = world.get_mut::<FollowerApl>(unit) {
            apl.idle();
        }
        if let Some(target) = injured_recovery_target(world, unit) {
            if world.get::<FollowerAplFailed>(unit).is_some() {
                return;
            }
            if let Err(error) = heal_recovery_target(world, unit, spec, target) {
                error!(?unit, "follower recovery healing failed: {error}");
                world.entity_mut(unit).insert(FollowerAplFailed);
            }
        } else {
            world.entity_mut(unit).remove::<FollowerAplFailed>();
        }
        return;
    }
    if let Some(mut apl) = world.entity_mut(unit).take::<FollowerApl>() {
        if !apl.precombat_buff_expired(world, unit) {
            world.entity_mut(unit).insert(apl);
        }
    }
    if world.get::<FollowerAplFailed>(unit).is_some() {
        return;
    }
    let apl = match world.entity_mut(unit).take::<FollowerApl>() {
        Some(apl) => Ok(apl),
        None => crate::registry::load(world, unit, spec).map(FollowerApl::new),
    };
    let result = apl.and_then(|mut apl| match apl.tick(world, unit, &targets) {
        Ok(()) => Ok(apl),
        Err(error) if error == server::simc_apl_native::DUNGEON_MOVEMENT_DESTINATION_PENDING => {
            debug!(
                ?unit,
                "follower APL waits for its pending navigation destination"
            );
            Ok(apl)
        }
        Err(error) => Err(error),
    });
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
    let mut candidates: Vec<Entity> = server::system_states::with_query::<
        (
            Entity,
            &server::maps::InMap,
            &shared::components::Npc,
            &shared::components::Health,
        ),
        (),
        _,
    >(world, |world, units| {
        units
            .iter(world)
            .filter(|(_, candidate_map, npc, health)| {
                **candidate_map == map
                    && health.current > 0.0
                    && health.current < health.max
                    && objectives.contains_key(&(map.map_id, npc.template_id))
            })
            .map(|(entity, _, _, _)| entity)
            .collect()
    });
    candidates.sort_unstable();
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
