//! Source-backed specializations available for normal-dungeon auto-fill.
use serde::Deserialize;
use shared::group::GroupRole;

use super::spec_selection::{QualifiedSpec, select_specs};

#[derive(Clone, Debug, Deserialize)]
pub struct ClassProfile {
    pub spec_id: u32,
    pub class_id: u8,
    pub role: u8,
    pub race: u8,
    pub talents: String,
}

impl ClassProfile {
    fn group_role(&self) -> Result<GroupRole, String> {
        match self.role {
            0 => Ok(GroupRole::Tank),
            1 => Ok(GroupRole::Healer),
            2 => Ok(GroupRole::Dps),
            role => Err(format!(
                "invalid follower spec {} role {role}",
                self.spec_id
            )),
        }
    }
}

pub fn read_profiles() -> Result<Vec<ClassProfile>, String> {
    serde_json::from_str(include_str!(
        "../../../../scripts/reference/class-follower-profiles.json"
    ))
    .map_err(|error| format!("read class follower profiles: {error}"))
}

pub fn select_profiles(
    profiles: &[ClassProfile],
    role: GroupRole,
    player_spec: u32,
    seed: u64,
) -> Result<Vec<ClassProfile>, String> {
    let candidates = profiles
        .iter()
        .map(|profile| {
            Ok(QualifiedSpec {
                spec_id: profile.spec_id,
                role: profile.group_role()?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let selected = select_specs(&candidates, role, player_spec, seed)?;
    selected
        .into_iter()
        .map(|selected| {
            profiles
                .iter()
                .find(|profile| profile.spec_id == selected.spec_id)
                .cloned()
                .ok_or_else(|| format!("selected follower spec {} disappeared", selected.spec_id))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn class_profiles_fill_each_missing_role_from_source_profiles() {
        let profiles = read_profiles().unwrap();
        for role in [GroupRole::Tank, GroupRole::Healer, GroupRole::Dps] {
            let party = select_profiles(&profiles, role, 70, 70448).unwrap();
            assert_eq!(party.len(), 4);
            assert!(!party.iter().any(|profile| profile.spec_id == 70));
            assert_eq!(
                party
                    .iter()
                    .map(|p| p.spec_id)
                    .collect::<BTreeSet<_>>()
                    .len(),
                4
            );
            assert!(
                party
                    .iter()
                    .all(|p| p.race != 0 && p.class_id != 0 && !p.talents.is_empty())
            );
            for (expected, total) in [
                (GroupRole::Tank, 1),
                (GroupRole::Healer, 1),
                (GroupRole::Dps, 3),
            ] {
                assert_eq!(
                    party
                        .iter()
                        .filter(|p| p.group_role().unwrap() == expected)
                        .count(),
                    total - usize::from(role == expected)
                );
            }
            assert_eq!(
                party.iter().map(|p| p.spec_id).collect::<Vec<_>>(),
                select_profiles(&profiles, role, 70, 70448)
                    .unwrap()
                    .iter()
                    .map(|p| p.spec_id)
                    .collect::<Vec<_>>()
            );
        }
    }
}
