//! Authored Lock.db2 alternatives for direct game-object interaction.
use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

const LOCK_LAYOUT: u32 = 0xDA418152;
const LOCK_KEY_ITEM: u8 = 1;
const LOCK_KEY_SKILL: u8 = 2;
const LOCK_KEY_SPELL: u8 = 3;
/// TC a352b1fa LockType: open, close, quick open/close, tinkering, kneeling, attacking.
/// Profession, aura-specific and spell-key alternatives require their own cast path.
const INTERACTION_LOCK_TYPES: [u32; 7] = [5, 8, 10, 11, 12, 13, 14];

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) struct LockRequirement {
    kind: u8,
    index: u32,
    skill: u16,
    action: u8,
}

impl LockRequirement {
    fn allows_interaction(&self, bag_count: &impl Fn(u32) -> u32) -> bool {
        if self.action > 1 {
            return false;
        }
        match self.kind {
            LOCK_KEY_ITEM => bag_count(self.index) > 0,
            LOCK_KEY_SKILL => self.skill == 0 && INTERACTION_LOCK_TYPES.contains(&self.index),
            _ => false,
        }
    }
}

pub(super) fn can_open(requirements: &[LockRequirement], bag_count: impl Fn(u32) -> u32) -> bool {
    requirements.is_empty()
        || requirements
            .iter()
            .any(|entry| entry.allows_interaction(&bag_count))
}

#[derive(Deserialize)]
pub(crate) struct LockDb {
    layout_hash: u32,
    locks: HashMap<u32, Vec<LockRequirement>>,
}

#[derive(Debug, PartialEq)]
pub(crate) enum LockCastResult {
    Allowed,
    InvalidTarget,
    LowSkill,
}

impl LockDb {
    /// TC CanOpenLock: alternatives are ordered; the first matching skill gate
    /// returns LOW_CASTLEVEL immediately, not a search for a weaker alternative.
    pub(crate) fn check_spell(
        &self,
        id: u32,
        spell_id: u32,
        lock_type: u32,
        skill: i32,
    ) -> rusqlite::Result<LockCastResult> {
        let requirements = self.requirements(id)?;
        for entry in &requirements {
            match entry.kind {
                LOCK_KEY_SKILL if entry.index == lock_type => {
                    return Ok(if skill >= i32::from(entry.skill) {
                        LockCastResult::Allowed
                    } else {
                        LockCastResult::LowSkill
                    });
                }
                LOCK_KEY_SPELL if entry.index == spell_id => return Ok(LockCastResult::Allowed),
                _ => {}
            }
        }
        Ok(
            if requirements
                .iter()
                .any(|entry| matches!(entry.kind, LOCK_KEY_ITEM | LOCK_KEY_SKILL | LOCK_KEY_SPELL))
            {
                LockCastResult::InvalidTarget
            } else {
                LockCastResult::Allowed
            },
        )
    }

    pub fn requirements(&self, id: u32) -> rusqlite::Result<Vec<LockRequirement>> {
        if id == 0 {
            return Ok(Vec::new());
        }
        self.locks.get(&id).cloned().ok_or_else(|| {
            rusqlite::Error::InvalidParameterName(format!(
                "GO Lock{id} is missing from game_object_locks.json"
            ))
        })
    }
}

pub(crate) fn load_locks(world_db_path: &str) -> rusqlite::Result<LockDb> {
    let path = Path::new(world_db_path).with_file_name("game_object_locks.json");
    let bytes = std::fs::read(&path).map_err(|error| {
        rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
            error.kind(),
            format!(
                "read {}: {error}; run scripts/import_game_object_locks.py",
                path.display()
            ),
        )))
    })?;
    let db: LockDb = serde_json::from_slice(&bytes)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    if db.layout_hash != LOCK_LAYOUT {
        return Err(rusqlite::Error::InvalidParameterName(format!(
            "{}: expected Lock layout DA418152",
            path.display()
        )));
    }
    Ok(db)
}
