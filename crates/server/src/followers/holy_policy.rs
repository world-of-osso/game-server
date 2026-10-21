//! HOLY_PRIEST_TRIAGE_V1: project decisions over source-valid shared triage inputs.
use super::healer_triage::{Decision, EMERGENCY_SECONDS, SpecVerdict, SpellSet, TriageContext};
use bevy::prelude::{Entity, Vec3};
use shared::components::Health;

use crate::auras::ActiveAuras;
use crate::spell_cast::{self, CastFailure};
use crate::spell_info::SpellStore;

const FLASH_HEAL: u32 = 2061;
const PRAYER_OF_MENDING: u32 = 33076;
const SANCTIFY: u32 = 34861;
const DIVINE_HYMN: u32 = 64843;
const APOTHEOSIS: u32 = 200183;
const SERENITY: u32 = 2050;
const GUARDIAN_SPIRIT: u32 = 47788;
const DESPERATE_PRAYER: u32 = 19236;
const FORTITUDE: u32 = 21562;
const FADE: u32 = 586;
const MASS_DISPEL: u32 = 32375;
const MAGIC_DISPEL_TYPE: u32 = 1;
const MASS_DISPEL_PATIENTS: usize = 3;
const SURGE: u32 = 114255;
const MENDING_WARD: u32 = 41635;
const OWNED: SpellSet = &[
    PRAYER_OF_MENDING,
    SANCTIFY,
    DIVINE_HYMN,
    APOTHEOSIS,
    GUARDIAN_SPIRIT,
    DESPERATE_PRAYER,
    FORTITUDE,
    FADE,
    MASS_DISPEL,
];
const POLICY_SPELLS: &[u32] = &[
    FLASH_HEAL,
    PRAYER_OF_MENDING,
    SANCTIFY,
    DIVINE_HYMN,
    APOTHEOSIS,
];
const GUARDIAN_HEALTH: f32 = 0.20;
const PERSONAL_HEALTH: f32 = 0.50;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct HolyDecision {
    pub spell: u32,
    pub target: Entity,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Patient {
    pub unit: Entity,
    pub health_fraction: f32,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Cluster {
    pub spell: u32,
    pub center: Entity,
    pub injured: usize,
}

struct PolicySnapshot {
    caster: Entity,
    patients: Vec<Patient>,
    candidates: Vec<HolyDecision>,
    clusters: Vec<Cluster>,
    surge: bool,
    mending: bool,
    depleted: bool,
    channel: bool,
}

pub(super) fn decide(ctx: &TriageContext<'_>) -> SpecVerdict {
    let decision = choose_guardian(ctx)
        .or_else(|| choose_personal_heal(ctx))
        .or_else(|| choose_emergency(ctx))
        .or_else(|| choose_fade(ctx))
        .or_else(|| choose_mass_dispel(ctx))
        .or_else(|| {
            select_action(&read_snapshot(ctx)).map(|action| Decision {
                spell_id: action.spell,
                target: action.target,
                urgent: false,
            })
        })
        .or_else(|| choose_fortitude(ctx));
    match decision {
        Some(decision) => SpecVerdict::Cast {
            decision,
            owned: OWNED,
        },
        None => SpecVerdict::Defer { owned: OWNED },
    }
}

fn ready(ctx: &TriageContext<'_>, spell_id: u32, target: Entity) -> bool {
    let Some(entry) = ctx.entries.iter().find(|entry| entry.spell_id == spell_id) else {
        return false;
    };
    ctx.ready(spell_id) && super::kit_ai::patient_is_eligible(ctx.world, ctx.caster, target, entry)
}

fn health_fraction(ctx: &TriageContext<'_>, target: Entity) -> Option<f32> {
    let health = ctx.world.get::<Health>(target)?;
    (health.current > 0.0 && health.max > 0.0).then_some(health.current / health.max)
}

fn own_aura(ctx: &TriageContext<'_>, target: Entity, spell: u32) -> bool {
    ctx.world.get::<ActiveAuras>(target).is_some_and(|auras| {
        auras
            .iter()
            .any(|aura| aura.spell_id == spell && aura.caster == ctx.caster)
    })
}

fn choose_guardian(ctx: &TriageContext<'_>) -> Option<Decision> {
    ctx.patients.iter().find_map(|patient| {
        let critical = health_fraction(ctx, patient.entity)? < GUARDIAN_HEALTH
            || patient.projected_death_seconds <= EMERGENCY_SECONDS;
        (critical && ready(ctx, GUARDIAN_SPIRIT, patient.entity)).then_some(Decision {
            spell_id: GUARDIAN_SPIRIT,
            target: patient.entity,
            urgent: true,
        })
    })
}

fn choose_personal_heal(ctx: &TriageContext<'_>) -> Option<Decision> {
    let injured = health_fraction(ctx, ctx.caster)? < PERSONAL_HEALTH;
    let incoming = ctx
        .patients
        .iter()
        .any(|patient| patient.entity == ctx.caster && patient.incoming_per_second > 0.0);
    (injured && incoming && ready(ctx, DESPERATE_PRAYER, ctx.caster)).then_some(Decision {
        spell_id: DESPERATE_PRAYER,
        target: ctx.caster,
        urgent: true,
    })
}

fn choose_emergency(ctx: &TriageContext<'_>) -> Option<Decision> {
    ctx.candidates
        .iter()
        .find(|candidate| {
            candidate.decision.urgent && !OWNED.contains(&candidate.decision.spell_id)
        })
        .map(|candidate| candidate.decision)
}

fn choose_fade(ctx: &TriageContext<'_>) -> Option<Decision> {
    let threatened = ctx
        .enemies
        .iter()
        .any(|(_, victim)| *victim == Some(ctx.caster));
    (threatened && ready(ctx, FADE, ctx.caster)).then_some(Decision {
        spell_id: FADE,
        target: ctx.caster,
        urgent: false,
    })
}

fn choose_mass_dispel(ctx: &TriageContext<'_>) -> Option<Decision> {
    if !ctx.ready(MASS_DISPEL) {
        return None;
    }
    let radius = super::kit_area::radius_for_spell(ctx.world, ctx.caster, MASS_DISPEL)?;
    ctx.patients
        .iter()
        .filter_map(|patient| {
            let count = count_magic_patients(ctx, patient.position?, radius);
            (count >= MASS_DISPEL_PATIENTS && ready(ctx, MASS_DISPEL, patient.entity))
                .then_some((count, patient.entity))
        })
        .max_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then_with(|| right.1.to_bits().cmp(&left.1.to_bits()))
        })
        .map(|(_, target)| Decision {
            spell_id: MASS_DISPEL,
            target,
            urgent: false,
        })
}

fn count_magic_patients(ctx: &TriageContext<'_>, center: Vec3, radius: f32) -> usize {
    ctx.patients
        .iter()
        .filter(|patient| {
            patient
                .position
                .is_some_and(|at| center.distance(at) <= radius)
                && crate::auras::has_dispellable_aura(
                    ctx.world,
                    ctx.caster,
                    patient.entity,
                    &[MAGIC_DISPEL_TYPE],
                )
        })
        .count()
}

fn choose_fortitude(ctx: &TriageContext<'_>) -> Option<Decision> {
    let calm = ctx
        .patients
        .iter()
        .all(|patient| patient.score == 0.0 && !ctx.preparing(patient.entity));
    let missing = ctx
        .patients
        .iter()
        .any(|patient| !crate::auras::has_aura(ctx.world, patient.entity, FORTITUDE));
    (calm && missing && ready(ctx, FORTITUDE, ctx.caster)).then_some(Decision {
        spell_id: FORTITUDE,
        target: ctx.caster,
        urgent: false,
    })
}

fn read_snapshot(ctx: &TriageContext<'_>) -> PolicySnapshot {
    let patients = ctx
        .patients
        .iter()
        .filter_map(|patient| {
            Some(Patient {
                unit: patient.entity,
                health_fraction: health_fraction(ctx, patient.entity)?,
            })
        })
        .collect();
    let candidates = POLICY_SPELLS
        .iter()
        .flat_map(|&spell| read_candidates(ctx, spell))
        .collect();
    let clusters = ctx
        .clusters
        .iter()
        .map(|cluster| Cluster {
            spell: cluster.spell_id,
            center: cluster.anchor,
            injured: cluster.members.len(),
        })
        .collect();
    PolicySnapshot {
        caster: ctx.caster,
        patients,
        candidates,
        clusters,
        surge: own_aura(ctx, ctx.caster, SURGE),
        mending: ctx
            .patients
            .iter()
            .any(|patient| own_aura(ctx, patient.entity, MENDING_WARD)),
        depleted: words_depleted(ctx),
        channel: !ctx.patients.iter().any(|patient| patient.emergency)
            && !ctx.preparing(ctx.caster),
    }
}

fn read_candidates(ctx: &TriageContext<'_>, spell: u32) -> Vec<HolyDecision> {
    let targets: Vec<_> = if matches!(spell, APOTHEOSIS | DIVINE_HYMN) {
        vec![ctx.caster]
    } else {
        ctx.patients.iter().map(|patient| patient.entity).collect()
    };
    targets
        .into_iter()
        .filter(|&target| ready(ctx, spell, target))
        .map(|target| HolyDecision { spell, target })
        .collect()
}

fn words_depleted(ctx: &TriageContext<'_>) -> bool {
    let store = ctx.world.resource::<SpellStore>();
    let words: Vec<_> = ctx
        .entries
        .iter()
        .filter(|entry| matches!(entry.spell_id, SERENITY | SANCTIFY))
        .filter_map(|entry| store.get(entry.spell_id))
        .collect();
    !words.is_empty()
        && words.iter().all(|spell| {
            matches!(
                spell_cast::read_healer_recovery(ctx.world, ctx.caster, spell),
                Err(CastFailure::NoCharges)
            )
        })
}

// Named project thresholds, never sourced spell mechanics.
const SURGE_HEALTH: f32 = 0.90;
const MENDING_HEALTH: f32 = 0.95;
const HYMN_HEALTH: f32 = 0.55;
const APOTHEOSIS_HEALTH: f32 = 0.65;
const GROUP_PRESSURE_COUNT: usize = 3;

fn select_action(context: &PolicySnapshot) -> Option<HolyDecision> {
    [
        choose_surge(context),
        choose_sanctify(context),
        choose_apotheosis(context),
        choose_hymn(context),
        choose_mending(context),
    ]
    .into_iter()
    .flatten()
    .next()
}

fn candidate(context: &PolicySnapshot, spell: u32, target: Entity) -> Option<HolyDecision> {
    context
        .candidates
        .iter()
        .copied()
        .find(|candidate| candidate.spell == spell && candidate.target == target)
}

fn injured_candidate(context: &PolicySnapshot, spell: u32, threshold: f32) -> Option<HolyDecision> {
    context
        .patients
        .iter()
        .filter(|patient| patient.health_fraction < threshold)
        .find_map(|patient| candidate(context, spell, patient.unit))
}

fn group_pressure(context: &PolicySnapshot, threshold: f32) -> bool {
    context
        .patients
        .iter()
        .filter(|patient| patient.health_fraction < threshold)
        .count()
        >= GROUP_PRESSURE_COUNT
}

fn choose_surge(context: &PolicySnapshot) -> Option<HolyDecision> {
    context
        .surge
        .then(|| injured_candidate(context, FLASH_HEAL, SURGE_HEALTH))
        .flatten()
}

fn choose_sanctify(context: &PolicySnapshot) -> Option<HolyDecision> {
    context
        .clusters
        .iter()
        .filter(|cluster| cluster.spell == SANCTIFY && cluster.injured >= GROUP_PRESSURE_COUNT)
        .filter_map(|cluster| {
            candidate(context, SANCTIFY, cluster.center).map(|action| (cluster.injured, action))
        })
        .max_by_key(|(injured, _)| *injured)
        .map(|(_, action)| action)
}

fn choose_hymn(context: &PolicySnapshot) -> Option<HolyDecision> {
    if context.channel && group_pressure(context, HYMN_HEALTH) {
        candidate(context, DIVINE_HYMN, context.caster)
    } else {
        None
    }
}

fn choose_apotheosis(context: &PolicySnapshot) -> Option<HolyDecision> {
    if context.depleted && group_pressure(context, APOTHEOSIS_HEALTH) {
        candidate(context, APOTHEOSIS, context.caster)
    } else {
        None
    }
}

fn choose_mending(context: &PolicySnapshot) -> Option<HolyDecision> {
    if context.mending {
        None
    } else {
        injured_candidate(context, PRAYER_OF_MENDING, MENDING_HEALTH)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl PolicySnapshot {
        fn injured_party() -> Self {
            let caster = Entity::from_bits(1);
            let patients = (2..=5)
                .map(|id| Patient {
                    unit: Entity::from_bits(id),
                    health_fraction: 0.6,
                })
                .collect();
            Self {
                caster,
                patients,
                candidates: Vec::new(),
                clusters: Vec::new(),
                surge: false,
                mending: false,
                depleted: false,
                channel: true,
            }
        }

        fn allow(&mut self, spell: u32, target: Entity) -> HolyDecision {
            let decision = HolyDecision { spell, target };
            self.candidates.push(decision);
            decision
        }
    }

    #[test]
    fn holy_policy_spends_surge_on_ranked_injured_patient() {
        let mut ctx = PolicySnapshot::injured_party();
        ctx.surge = true;
        let patient = ctx.patients[0].unit;
        let expected = ctx.allow(FLASH_HEAL, patient);
        ctx.allow(PRAYER_OF_MENDING, patient);
        assert_eq!(select_action(&ctx), Some(expected));
    }

    #[test]
    fn holy_policy_maintains_missing_mending_not_root_aura() {
        let mut ctx = PolicySnapshot::injured_party();
        let patient = ctx.patients[0].unit;
        let expected = ctx.allow(PRAYER_OF_MENDING, patient);
        assert_eq!(select_action(&ctx), Some(expected));
        ctx.mending = true;
        assert_eq!(
            select_action(&ctx),
            None,
            "owned41635 on any bounced holder suppresses overwrite"
        );
    }

    #[test]
    fn holy_policy_sanctify_requires_a_native_radius_cluster() {
        let mut ctx = PolicySnapshot::injured_party();
        let patient = ctx.patients[0].unit;
        let expected = ctx.allow(SANCTIFY, patient);
        assert_eq!(
            select_action(&ctx),
            None,
            "party-wide injury is not a local cluster"
        );
        ctx.clusters.push(Cluster {
            spell: SANCTIFY,
            center: patient,
            injured: 3,
        });
        assert_eq!(select_action(&ctx), Some(expected));
    }

    #[test]
    fn holy_policy_apotheosis_requires_pressure_and_depleted_words() {
        let mut ctx = PolicySnapshot::injured_party();
        let expected = ctx.allow(APOTHEOSIS, ctx.caster);
        assert_eq!(select_action(&ctx), None);
        ctx.depleted = true;
        assert_eq!(select_action(&ctx), Some(expected));
        for patient in &mut ctx.patients {
            patient.health_fraction = 0.95;
        }
        assert_eq!(
            select_action(&ctx),
            None,
            "do not spend an available throughput cooldown on healthy party"
        );
    }

    #[test]
    fn holy_policy_hymn_requires_sustained_injury_and_safe_channel() {
        let mut ctx = PolicySnapshot::injured_party();
        for patient in &mut ctx.patients {
            patient.health_fraction = 0.45;
        }
        let expected = ctx.allow(DIVINE_HYMN, ctx.caster);
        assert_eq!(select_action(&ctx), Some(expected));
        ctx.channel = false;
        assert_eq!(
            select_action(&ctx),
            None,
            "unsafe channel cannot delay emergency healing"
        );
    }

    #[test]
    fn holy_policy_never_invents_an_unavailable_spell_or_target() {
        let mut ctx = PolicySnapshot::injured_party();
        ctx.surge = true;
        ctx.depleted = true;
        ctx.clusters.push(Cluster {
            spell: SANCTIFY,
            center: ctx.patients[0].unit,
            injured: 4,
        });
        assert_eq!(select_action(&ctx), None);
    }
}
