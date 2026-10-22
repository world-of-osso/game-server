//! FOLLOWER_HEALER_TRIAGE_POLICY: project decisions over native spell execution.
use bevy::prelude::*;
use shared::casting::{CastState, CastType};
use shared::components::{Health, Mana};
use shared::protocol::{CombatLogEvent, CombatLogKind};
use std::collections::VecDeque;

use super::kit::{Entry, Slot, Target};
use super::{Follower, ai, kit_ai};
use crate::maps::InMap;
use crate::spell_cast::{self, CostPool};
use crate::spell_info::{SpellInfo, SpellStore};

pub(super) const EMERGENCY_SECONDS: f32 = 3.0;
const DAMAGE_WINDOW_SECONDS: f64 = 10.0;
const MIN_RATE_WINDOW_SECONDS: f64 = 1.0;
const TANK_WEIGHT: f32 = 1.25;
const EMERGENCY_HEALTH: f32 = 0.25;
const LOW_MANA: f32 = 0.30;

pub(super) type SpellSet = &'static [u32];

#[derive(Clone, Copy, Debug)]
pub(super) struct Decision {
    pub spell_id: u32,
    pub target: Entity,
    pub urgent: bool,
}

pub(super) enum SpecVerdict {
    Cast { decision: Decision, owned: SpellSet },
    Defer { owned: SpellSet },
}

#[derive(Clone, Debug)]
pub(super) struct Patient {
    pub entity: Entity,
    pub missing_health: f32,
    pub incoming_per_second: f32,
    pub periodic_remaining: f32,
    pub score: f32,
    pub emergency: bool,
    pub projected_death_seconds: f32,
    pub position: Option<Vec3>,
}

#[derive(Clone, Debug)]
pub(super) struct Candidate {
    pub decision: Decision,
    pub effective_healing: f32,
    pub mana_cost: f32,
    pub cast_seconds: f32,
    pub score: f32,
}

pub(super) struct TriageContext<'a> {
    pub world: &'a World,
    pub caster: Entity,
    pub follower: &'a Follower,
    pub entries: &'a [Entry],
    pub patients: Vec<Patient>,
    pub candidates: Vec<Candidate>,
    pub mana_fraction: Option<f32>,
    pub upcoming: Vec<super::healer_context::UpcomingDamage>,
    pub clusters: Vec<super::healer_context::InjuryCluster>,
    pub enemies: Vec<(Entity, Option<Entity>)>,
}

impl TriageContext<'_> {
    /// Learned native cooldown/charges/school and costs; ignores only transient GCD.
    pub fn ready(&self, spell_id: u32) -> bool {
        self.entries.iter().any(|entry| entry.spell_id == spell_id)
            && self
                .world
                .resource::<SpellStore>()
                .get(spell_id)
                .is_some_and(|spell| ready(self.world, self.caster, spell))
    }

    pub fn best_candidate(&self, owned: SpellSet) -> Option<Decision> {
        self.candidates
            .iter()
            .find(|candidate| owned.contains(&candidate.decision.spell_id))
            .map(|candidate| candidate.decision)
    }
}

pub(super) struct Outcome {
    pub acted: bool,
    pub owned: SpellSet,
    pub safe_to_dps: bool,
}

#[derive(Component)]
struct IncomingDamage {
    map: InMap,
    started: f64,
    samples: VecDeque<(f64, f32)>,
}

pub(super) fn record_damage(world: &mut World, event: &CombatLogEvent) {
    if !matches!(
        event.kind,
        CombatLogKind::Damage | CombatLogKind::Environmental(_)
    ) {
        return;
    }
    let Some(target) = event.target.and_then(Entity::try_from_bits) else {
        return;
    };
    let Some(map) = world.get::<InMap>(target).copied() else {
        return;
    };
    let amount = effective_health_loss(world, target, event);
    if amount <= 0.0 {
        return;
    }
    let now = crate::auras::aura_now(world);
    let same_copy = world
        .get::<IncomingDamage>(target)
        .is_some_and(|history| history.map == map);
    if !same_copy {
        world.entity_mut(target).insert(IncomingDamage {
            map,
            started: now,
            samples: VecDeque::new(),
        });
    }
    let mut history = world.get_mut::<IncomingDamage>(target).unwrap();
    while history
        .samples
        .front()
        .is_some_and(|(at, _)| *at < now - DAMAGE_WINDOW_SECONDS)
    {
        history.samples.pop_front();
    }
    if history.samples.is_empty() {
        history.started = now;
    }
    history.samples.push_back((now, amount));
}

fn effective_health_loss(world: &World, target: Entity, event: &CombatLogEvent) -> f32 {
    let amount = event.amount.saturating_sub(event.overflow.max(0)).max(0) as f32;
    let Some(source) = event.source.and_then(Entity::try_from_bits) else {
        return amount;
    };
    let source = crate::level_scaling::UnitScaling::of(world, source);
    let target = crate::level_scaling::UnitScaling::of(world, target);
    crate::level_scaling::health_loss(&source, &target, amount)
}

fn incoming_rate(world: &World, target: Entity, now: f64) -> f32 {
    let Some(history) = world.get::<IncomingDamage>(target) else {
        return 0.0;
    };
    if world.get::<InMap>(target) != Some(&history.map) {
        return 0.0;
    }
    let amount: f32 = history
        .samples
        .iter()
        .filter(|(at, _)| *at >= now - DAMAGE_WINDOW_SECONDS)
        .map(|(_, amount)| amount)
        .sum();
    let seconds = (now - history.started).clamp(MIN_RATE_WINDOW_SECONDS, DAMAGE_WINDOW_SECONDS);
    amount / seconds as f32
}

pub(super) fn read_patient(world: &World, target: Entity) -> Option<Patient> {
    let health = world.get::<Health>(target)?;
    if health.current <= 0.0 || health.max <= 0.0 {
        return None;
    }
    let now = crate::auras::aura_now(world);
    let incoming = incoming_rate(world, target, now);
    let (periodic, absorb) = world
        .get::<crate::auras::ActiveAuras>(target)
        .map_or((0.0, 0.0), |auras| {
            crate::auras::forecast::healing_coverage(auras, now, f64::from(EMERGENCY_SECONDS))
        });
    let missing = (health.max - health.current).max(0.0);
    let uncovered = (missing + incoming * EMERGENCY_SECONDS
        - periodic
        - absorb.min(incoming * EMERGENCY_SECONDS))
    .max(0.0);
    let death = if incoming > 0.0 {
        (health.current + absorb + periodic) / incoming
    } else {
        f32::INFINITY
    };
    let weight = if kit_ai::is_tank(world, target) {
        TANK_WEIGHT
    } else {
        1.0
    };
    Some(Patient {
        entity: target,
        missing_health: missing,
        incoming_per_second: incoming,
        periodic_remaining: periodic,
        score: uncovered / health.max * weight,
        emergency: death <= EMERGENCY_SECONDS || health.current / health.max <= EMERGENCY_HEALTH,
        projected_death_seconds: death,
        position: super::healer_context::position(world, target),
    })
}

pub(super) fn manages(entry: &Entry) -> bool {
    matches!(entry.slot, Slot::Heal | Slot::Hot)
        || (entry.slot == Slot::Defensive
            && matches!(entry.conditions.target, Target::Party | Target::Tank))
}

fn ready(world: &World, caster: Entity, spell: &SpellInfo) -> bool {
    spell_cast::read_healer_recovery(world, caster, spell).is_ok()
        && spell_cast::spell_costs(world, caster, spell).is_ok()
}

fn read_candidates(ctx: &TriageContext<'_>) -> Vec<Candidate> {
    let mut candidates: Vec<_> = ctx
        .entries
        .iter()
        .filter(|entry| manages(entry))
        .flat_map(|entry| {
            ctx.patients
                .iter()
                .filter_map(move |patient| read_candidate(ctx, entry, patient))
        })
        .collect();
    let conserve = ctx.mana_fraction.is_some_and(|mana| mana <= LOW_MANA)
        || ctx
            .patients
            .iter()
            .all(|patient| patient.incoming_per_second == 0.0);
    candidates.sort_by(|left, right| compare_candidates(right, left, conserve));
    candidates
}

fn read_candidate(ctx: &TriageContext<'_>, entry: &Entry, patient: &Patient) -> Option<Candidate> {
    let preparing = entry.slot == Slot::Hot && ctx.preparing(patient.entity);
    if (patient.score <= 0.0 && !preparing)
        || !kit_ai::patient_is_eligible(ctx.world, ctx.caster, patient.entity, entry)
    {
        return None;
    }
    let enough_injured = entry.conditions.min_injured == 0
        || ctx
            .injury_cluster(entry.spell_id, patient.entity)
            .is_some_and(|cluster| cluster.members.len() >= entry.conditions.min_injured);
    if !enough_injured {
        return None;
    }
    let spell = ctx.world.resource::<SpellStore>().get(entry.spell_id)?;
    if !ctx.ready(entry.spell_id) || (patient.missing_health == 0.0 && normal_direct_heal(spell)) {
        return None;
    }
    let healing = estimate_healing(ctx.world, ctx.caster, spell);
    if healing == 0.0 && !patient.emergency {
        return None;
    }
    let costs = spell_cast::spell_costs(ctx.world, ctx.caster, spell).ok()?;
    let mana = costs
        .iter()
        .filter(|(pool, _)| *pool == CostPool::Power(shared::components::PowerType::Mana))
        .map(|(_, amount)| amount.max(0.0))
        .sum();
    let need = if preparing {
        healing
    } else {
        patient.missing_health + patient.incoming_per_second * EMERGENCY_SECONDS
            - patient.periodic_remaining
    };
    Some(Candidate {
        decision: Decision {
            spell_id: entry.spell_id,
            target: patient.entity,
            urgent: patient.emergency,
        },
        effective_healing: healing.min(need.max(0.0)),
        mana_cost: mana,
        cast_seconds: spell_cast::cast_time_ms(ctx.world, ctx.caster, spell).max(0) as f32 / 1000.0,
        score: patient.score,
    })
}

fn estimate_healing(world: &World, caster: Entity, spell: &SpellInfo) -> f32 {
    let duration = spell_cast::duration_ms(world, caster, spell).max(0) as f32;
    spell
        .effects
        .iter()
        .map(|effect| {
            let amount = spell_cast::effects::scaled_amount(world, caster, spell, effect).max(0.0);
            if effect.effect == 10 {
                amount
            } else if crate::auras::is_aura_effect(effect)
                && effect.aura == crate::auras::handlers::PERIODIC_HEAL
                && effect.aura_period_ms > 0
            {
                amount * (duration / effect.aura_period_ms as f32).floor()
            } else {
                0.0
            }
        })
        .sum()
}

fn compare_candidates(left: &Candidate, right: &Candidate, conserve: bool) -> std::cmp::Ordering {
    left.decision
        .urgent
        .cmp(&right.decision.urgent)
        .then_with(|| {
            if left.decision.urgent {
                right
                    .cast_seconds
                    .total_cmp(&left.cast_seconds)
                    .then_with(|| left.effective_healing.total_cmp(&right.effective_healing))
            } else if conserve {
                (left.mana_cost == 0.0)
                    .cmp(&(right.mana_cost == 0.0))
                    .then_with(|| {
                        (left.effective_healing / left.mana_cost.max(f32::MIN_POSITIVE)).total_cmp(
                            &(right.effective_healing / right.mana_cost.max(f32::MIN_POSITIVE)),
                        )
                    })
            } else {
                left.effective_healing.total_cmp(&right.effective_healing)
            }
        })
        .then_with(|| left.score.total_cmp(&right.score))
        .then_with(|| {
            right
                .decision
                .target
                .to_bits()
                .cmp(&left.decision.target.to_bits())
        })
        .then_with(|| right.decision.spell_id.cmp(&left.decision.spell_id))
}

pub(super) fn update(
    world: &mut World,
    caster: Entity,
    follower: &Follower,
    party: &[Entity],
    entries: &[Entry],
) -> Outcome {
    cancel_topped_cast(world, caster);
    let enemies = ai::engaged_enemies(world, follower, caster, party);
    let mut patients: Vec<_> = party
        .iter()
        .copied()
        .filter(|target| world.get::<InMap>(*target) == world.get::<InMap>(caster))
        .filter_map(|target| read_patient(world, target))
        .collect();
    patients.sort_by(|left, right| {
        right
            .emergency
            .cmp(&left.emergency)
            .then_with(|| right.score.total_cmp(&left.score))
            .then_with(|| left.entity.to_bits().cmp(&right.entity.to_bits()))
    });
    let upcoming = super::healer_context::read_upcoming(world, &enemies, &patients);
    let clusters = super::healer_context::read_clusters(world, caster, entries, &patients);
    let mut ctx = TriageContext {
        world,
        caster,
        follower,
        entries,
        patients,
        candidates: Vec::new(),
        upcoming,
        clusters,
        enemies,
        mana_fraction: world
            .get::<Mana>(caster)
            .filter(|mana| mana.max > 0.0)
            .map(|mana| mana.current / mana.max),
    };
    let safe = ctx.patients.iter().all(|patient| {
        patient.score == 0.0 && !patient.emergency && !ctx.preparing(patient.entity)
    });
    ctx.candidates = read_candidates(&ctx);
    let (decision, owned) = match super::healer_policy::decide(&ctx) {
        SpecVerdict::Cast { decision, owned } => (Some(decision), owned),
        SpecVerdict::Defer { owned } => (None, owned),
    };
    let fallback = ctx
        .candidates
        .iter()
        .filter(|candidate| !owned.contains(&candidate.decision.spell_id))
        .map(|candidate| candidate.decision)
        .collect::<Vec<_>>();
    let acted = decision
        .into_iter()
        .chain(fallback)
        .any(|decision| execute(world, caster, follower, decision));
    Outcome {
        acted: acted || world.get::<CastState>(caster).is_some(),
        owned,
        safe_to_dps: safe,
    }
}

fn normal_direct_heal(spell: &SpellInfo) -> bool {
    !spell_cast::is_channeled(spell) && spell.effects.iter().any(|effect| effect.effect == 10)
}

fn cancel_topped_cast(world: &mut World, caster: Entity) {
    let Some(cast) = world.get::<CastState>(caster) else {
        return;
    };
    if cast.cast_type != CastType::Normal {
        return;
    }
    let direct_heal = world
        .resource::<SpellStore>()
        .get(cast.spell_id)
        .is_some_and(normal_direct_heal);
    let topped = Entity::try_from_bits(cast.target)
        .and_then(|target| world.get::<Health>(target))
        .is_none_or(|health| health.current <= 0.0 || health.current >= health.max);
    if direct_heal && topped {
        spell_cast::cancel_cast(world, caster);
    }
}

fn execute(world: &mut World, caster: Entity, follower: &Follower, decision: Decision) -> bool {
    if let Some(cast) = world.get::<CastState>(caster) {
        if cast.spell_id == decision.spell_id && cast.target == decision.target.to_bits() {
            return true;
        }
        let friendly = Entity::try_from_bits(cast.target)
            .is_some_and(|target| !spell_cast::targets::is_hostile(world, caster, target));
        if friendly && !decision.urgent {
            return true;
        }
    }
    if decision.target != caster
        && !ai::pursue(
            world,
            caster,
            decision.target,
            decision.spell_id,
            follower.profile.melee,
        )
    {
        return false;
    }
    let result = if world.get::<CastState>(caster).is_some() {
        spell_cast::creature_cast_interrupting_self(
            world,
            caster,
            decision.spell_id,
            Some(decision.target),
        )
    } else {
        spell_cast::creature_cast(
            world,
            caster,
            decision.spell_id,
            Some(decision.target),
            false,
        )
    };
    match result {
        Ok(()) | Err(spell_cast::CastFailure::OnGlobalCooldown) => true,
        Err(error) => {
            debug!(
                ?caster,
                spell = decision.spell_id,
                ?error,
                "Healer policy cast unavailable"
            );
            false
        }
    }
}
