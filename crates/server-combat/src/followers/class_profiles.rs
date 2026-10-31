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
    pub race_source: String,
    pub talent_entries: Option<Vec<(u32, u32)>>,
    pub rejection: Option<String>,
}

impl ClassProfile {
    pub(super) fn group_role(&self) -> Result<GroupRole, String> {
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

    pub(super) fn talent_source(&self) -> Result<crate::trait_config::TalentSource, String> {
        match self.race_source.as_str() {
            "RACE_FROM_SIMC_PROFILE" => Ok(crate::trait_config::TalentSource::SimcProfile),
            "RACE_FROM_PARITY_PROFILE" => Ok(crate::trait_config::TalentSource::ParityProfile),
            source => Err(format!("unknown follower profile source {source}")),
        }
    }
}

pub fn read_profiles() -> Result<Vec<ClassProfile>, String> {
    serde_json::from_str(include_str!(
        "../../../../scripts/reference/class-follower-profiles.json"
    ))
    .map_err(|error| format!("read class follower profiles: {error}"))
}

/// Reject profiles that cannot build a native spellbook and trait configuration.
pub(super) fn validate_profiles(
    profiles: &[ClassProfile],
    classes: &crate::spell_info::ClassSpells,
    spells: &crate::spell_info::SpellStore,
    level: u8,
) -> Vec<ClassProfile> {
    profiles
        .iter()
        .filter_map(|profile| {
            let result = profile.rejection.as_ref().map_or_else(
                || {
                    let entries = profile
                        .talent_entries
                        .as_ref()
                        .ok_or("decoded talents are missing".to_owned())?;
                    let source = profile.talent_source()?;
                    crate::race_class::validate_race_class(profile.race, profile.class_id)?;
                    let (known, _, _) = crate::class_progression::evaluate_follower_progression(
                        classes,
                        profile.class_id,
                        profile.race,
                        profile.spec_id,
                        level,
                        source,
                        entries,
                    )?;
                    super::class_spawn::attack_spell(classes, spells, profile, &known).map(|_| ())
                },
                |error| Err(error.clone()),
            );
            match result {
                Ok(()) => Some(profile.clone()),
                Err(error) => {
                    bevy::log::warn!("follower spec {} rejected: {error}", profile.spec_id);
                    None
                }
            }
        })
        .collect()
}

pub fn select_profiles(
    profiles: &[ClassProfile],
    role: GroupRole,
    player_spec: u32,
    seed: u64,
) -> Result<Vec<ClassProfile>, String> {
    let candidates = profiles
        .iter()
        .filter(|profile| profile.rejection.is_none() && profile.talent_entries.is_some())
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
    fn class_profiles_validate_native_talents_before_selection() {
        let db = rusqlite::Connection::open_with_flags(
            crate::test_data::world_db_path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let (spells, classes) = crate::spell_info::load_spell_data(&db).unwrap();
        let profiles = validate_profiles(&read_profiles().unwrap(), &classes, &spells, 90);
        for role in [GroupRole::Tank, GroupRole::Healer, GroupRole::Dps] {
            let count = profiles
                .iter()
                .filter(|profile| profile.group_role().unwrap() == role)
                .count();
            println!("{role:?}: {count} qualified profiles");
            assert!(
                count >= 2,
                "{role:?}: only {count} profiles passed server validation"
            );
        }
    }

    #[test]
    fn class_follower_roster_passes_native_validation() {
        let db = rusqlite::Connection::open_with_flags(
            crate::test_data::world_db_path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let (spells, classes) = crate::spell_info::load_spell_data(&db).unwrap();
        let roster = read_profiles().unwrap();
        let validated = validate_profiles(&roster, &classes, &spells, 90);
        let accepted: BTreeSet<_> = validated.iter().map(|profile| profile.spec_id).collect();
        let rejected: Vec<_> = roster
            .iter()
            .filter(|profile| !accepted.contains(&profile.spec_id))
            .map(|profile| (profile.spec_id, &profile.rejection))
            .collect();
        assert!(
            rejected.is_empty(),
            "rejected follower profiles: {rejected:?}"
        );
    }

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
            assert!(party.iter().all(|p| {
                p.race != 0
                    && p.class_id != 0
                    && !p.talents.is_empty()
                    && p.rejection.is_none()
                    && p.talent_entries
                        .as_ref()
                        .is_some_and(|entries| !entries.is_empty())
            }));
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
