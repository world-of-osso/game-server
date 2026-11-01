//! Cited Midnight Season 2 keystone timing policy.
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Cited<T> {
    pub value: T,
    pub citation: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DeathBand {
    pub min_level: u32,
    pub milliseconds: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ChallengePolicy {
    pub countdown_ms: Cited<u64>,
    pub death_bands: Cited<Vec<DeathBand>>,
    pub upgrade_percentages: Cited<[u64; 3]>,
}

impl ChallengePolicy {
    pub fn retail() -> Self {
        let policy: Self = serde_json::from_str(include_str!("mythic_plus_policy.json"))
            .expect("parse cited Mythic+ policy");
        assert!(!policy.countdown_ms.citation.is_empty());
        assert!(!policy.death_bands.citation.is_empty());
        assert!(!policy.upgrade_percentages.citation.is_empty());
        policy
    }

    pub fn death_penalty_ms(&self, level: u32) -> u64 {
        self.death_bands
            .value
            .iter()
            .rev()
            .find(|band| level >= band.min_level)
            .map_or(0, |band| band.milliseconds)
    }

    pub fn upgrade_levels(&self, elapsed_ms: u64, timer_ms: u64) -> u32 {
        self.upgrade_percentages
            .value
            .iter()
            .filter(|&&percentage| {
                u128::from(elapsed_ms) * 100 <= u128::from(timer_ms) * u128::from(percentage)
            })
            .count() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mythic_policy_death_bands_and_upgrade_thresholds() {
        let policy = ChallengePolicy::retail();
        for (level, penalty) in [(2, 0), (5, 0), (6, 5000), (11, 5000), (12, 15000)] {
            assert_eq!(policy.death_penalty_ms(level), penalty);
        }
        for (elapsed, upgrades) in [(600, 3), (601, 2), (800, 2), (801, 1), (1000, 1), (1001, 0)] {
            assert_eq!(policy.upgrade_levels(elapsed, 1000), upgrades);
        }
        assert_eq!(policy.countdown_ms.value, 10000);
    }
}
