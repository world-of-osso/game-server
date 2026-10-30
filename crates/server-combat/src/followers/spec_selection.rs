//! Seeded role fill over qualified player-specialization profiles.
use rand::{SeedableRng, rngs::StdRng, seq::SliceRandom};
use shared::group::GroupRole;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QualifiedSpec {
    pub spec_id: u32,
    pub role: GroupRole,
}

/// Select each missing role uniformly without replacement. The seed and sorted
/// candidate set make the result reproducible, independent of input order.
pub fn select_specs(
    available: &[QualifiedSpec],
    player_role: GroupRole,
    player_spec: u32,
    seed: u64,
) -> Result<Vec<QualifiedSpec>, String> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut selected: Vec<QualifiedSpec> = Vec::with_capacity(4);
    for (role, total) in [
        (GroupRole::Tank, 1),
        (GroupRole::Healer, 1),
        (GroupRole::Dps, 3),
    ] {
        let missing = total - usize::from(player_role == role);
        for _ in 0..missing {
            let mut options: Vec<_> = available
                .iter()
                .copied()
                .filter(|candidate| {
                    candidate.role == role
                        && candidate.spec_id != player_spec
                        && !selected
                            .iter()
                            .any(|other| other.spec_id == candidate.spec_id)
                })
                .collect();
            options.sort_unstable_by_key(|candidate| candidate.spec_id);
            options.dedup_by_key(|candidate| candidate.spec_id);
            let chosen = options
                .choose(&mut rng)
                .copied()
                .ok_or_else(|| format!("no qualified follower spec remains for role {role:?}"))?;
            selected.push(chosen);
        }
    }
    Ok(selected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn class_follower_seeded_fill_uses_qualified_unique_specs_and_missing_roles() {
        let specs = [
            QualifiedSpec {
                spec_id: 66,
                role: GroupRole::Tank,
            },
            QualifiedSpec {
                spec_id: 73,
                role: GroupRole::Tank,
            },
            QualifiedSpec {
                spec_id: 65,
                role: GroupRole::Healer,
            },
            QualifiedSpec {
                spec_id: 257,
                role: GroupRole::Healer,
            },
            QualifiedSpec {
                spec_id: 70,
                role: GroupRole::Dps,
            },
            QualifiedSpec {
                spec_id: 62,
                role: GroupRole::Dps,
            },
            QualifiedSpec {
                spec_id: 63,
                role: GroupRole::Dps,
            },
            QualifiedSpec {
                spec_id: 64,
                role: GroupRole::Dps,
            },
        ];
        let party = select_specs(&specs, GroupRole::Dps, 70, 123).unwrap();
        assert_eq!(party.len(), 4);
        assert_eq!(
            party.iter().filter(|s| s.role == GroupRole::Tank).count(),
            1
        );
        assert_eq!(
            party.iter().filter(|s| s.role == GroupRole::Healer).count(),
            1
        );
        assert_eq!(party.iter().filter(|s| s.role == GroupRole::Dps).count(), 2);
        assert_eq!(
            party
                .iter()
                .map(|s| s.spec_id)
                .collect::<BTreeSet<_>>()
                .len(),
            4
        );
        assert!(!party.iter().any(|s| s.spec_id == 70));
        let reversed: Vec<_> = specs.into_iter().rev().collect();
        assert_eq!(
            party,
            select_specs(&reversed, GroupRole::Dps, 70, 123).unwrap()
        );
        let variants: BTreeSet<_> = (0..32)
            .map(|seed| select_specs(&specs, GroupRole::Dps, 70, seed).unwrap()[0].spec_id)
            .collect();
        assert_eq!(variants, BTreeSet::from([66, 73]));
        assert!(select_specs(&specs[..3], GroupRole::Dps, 70, 123).is_err());
    }
}
