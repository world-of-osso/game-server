//! HOLY_PRIEST_TRIAGE_V1: project decisions over source-valid shared triage inputs.
use bevy::prelude::Entity;

const FLASH_HEAL: u32 = 2061;
const PRAYER_OF_MENDING: u32 = 33076;
const SANCTIFY: u32 = 34861;
const DIVINE_HYMN: u32 = 64843;
const APOTHEOSIS: u32 = 200183;

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

/// Temporary local boundary until the shared TriageContext lands. Candidates
/// must already pass learned-spell, cost, range, LOS and cooldown admission.
pub(super) trait HolyTriageView {
    fn caster(&self) -> Entity;
    fn patients(&self) -> &[Patient];
    fn candidates(&self) -> &[HolyDecision];
    fn clusters(&self) -> &[Cluster];
    fn surge_active(&self) -> bool;
    fn owned_mending_active(&self) -> bool;
    fn words_depleted(&self) -> bool;
    fn safe_to_channel(&self) -> bool;
}

pub(super) fn decide(_context: &impl HolyTriageView) -> Option<HolyDecision> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TriageFixture {
        caster: Entity,
        patients: Vec<Patient>,
        candidates: Vec<HolyDecision>,
        clusters: Vec<Cluster>,
        surge: bool,
        mending: bool,
        depleted: bool,
        channel: bool,
    }

    impl TriageFixture {
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

    impl HolyTriageView for TriageFixture {
        fn caster(&self) -> Entity {
            self.caster
        }
        fn patients(&self) -> &[Patient] {
            &self.patients
        }
        fn candidates(&self) -> &[HolyDecision] {
            &self.candidates
        }
        fn clusters(&self) -> &[Cluster] {
            &self.clusters
        }
        fn surge_active(&self) -> bool {
            self.surge
        }
        fn owned_mending_active(&self) -> bool {
            self.mending
        }
        fn words_depleted(&self) -> bool {
            self.depleted
        }
        fn safe_to_channel(&self) -> bool {
            self.channel
        }
    }

    #[test]
    fn holy_policy_spends_surge_on_ranked_injured_patient() {
        let mut ctx = TriageFixture::injured_party();
        ctx.surge = true;
        let patient = ctx.patients[0].unit;
        let expected = ctx.allow(FLASH_HEAL, patient);
        ctx.allow(PRAYER_OF_MENDING, patient);
        assert_eq!(decide(&ctx), Some(expected));
    }

    #[test]
    fn holy_policy_maintains_missing_mending_not_root_aura() {
        let mut ctx = TriageFixture::injured_party();
        let patient = ctx.patients[0].unit;
        let expected = ctx.allow(PRAYER_OF_MENDING, patient);
        assert_eq!(decide(&ctx), Some(expected));
        ctx.mending = true;
        assert_eq!(
            decide(&ctx),
            None,
            "owned41635 on any bounced holder suppresses overwrite"
        );
    }

    #[test]
    fn holy_policy_sanctify_requires_a_native_radius_cluster() {
        let mut ctx = TriageFixture::injured_party();
        let patient = ctx.patients[0].unit;
        let expected = ctx.allow(SANCTIFY, patient);
        assert_eq!(
            decide(&ctx),
            None,
            "party-wide injury is not a local cluster"
        );
        ctx.clusters.push(Cluster {
            spell: SANCTIFY,
            center: patient,
            injured: 3,
        });
        assert_eq!(decide(&ctx), Some(expected));
    }

    #[test]
    fn holy_policy_apotheosis_requires_pressure_and_depleted_words() {
        let mut ctx = TriageFixture::injured_party();
        let expected = ctx.allow(APOTHEOSIS, ctx.caster);
        assert_eq!(decide(&ctx), None);
        ctx.depleted = true;
        assert_eq!(decide(&ctx), Some(expected));
        for patient in &mut ctx.patients {
            patient.health_fraction = 0.95;
        }
        assert_eq!(
            decide(&ctx),
            None,
            "do not spend an available throughput cooldown on healthy party"
        );
    }

    #[test]
    fn holy_policy_hymn_requires_sustained_injury_and_safe_channel() {
        let mut ctx = TriageFixture::injured_party();
        for patient in &mut ctx.patients {
            patient.health_fraction = 0.45;
        }
        let expected = ctx.allow(DIVINE_HYMN, ctx.caster);
        assert_eq!(decide(&ctx), Some(expected));
        ctx.channel = false;
        assert_eq!(
            decide(&ctx),
            None,
            "unsafe channel cannot delay emergency healing"
        );
    }

    #[test]
    fn holy_policy_never_invents_an_unavailable_spell_or_target() {
        let mut ctx = TriageFixture::injured_party();
        ctx.surge = true;
        ctx.depleted = true;
        ctx.clusters.push(Cluster {
            spell: SANCTIFY,
            center: ctx.patients[0].unit,
            injured: 4,
        });
        assert_eq!(decide(&ctx), None);
    }
}
