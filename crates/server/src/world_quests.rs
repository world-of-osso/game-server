//! World of Osso deterministic rotation policy, not a reconstruction of Retail activation.
use bevy::prelude::*;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Definition {
    pub quest_id: u32,
    pub zone_id: u32,
    /// Authentic ManagedWorldState.UpTimeSecs. Never substitute a realm default.
    pub duration: u32,
}

#[derive(Resource, Debug, Clone)]
pub(crate) struct Rotation {
    pub epoch: i64,
    pub active_per_zone: usize,
    pub descending: bool,
    pub gap_seconds: u32,
}

impl Default for Rotation {
    fn default() -> Self {
        Self {
            epoch: 1_767_225_600, // 2026-01-01 00:00 UTC, realm policy.
            active_per_zone: 1,
            descending: false,
            gap_seconds: 86_400,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Window {
    pub quest_id: u32,
    pub starts_at: i64,
    pub expires_at: i64,
}

impl Rotation {
    pub(crate) fn active(&self, definitions: &[Definition], now: i64) -> Vec<Window> {
        if now < self.epoch {
            return Vec::new();
        }
        let mut zones: BTreeMap<u32, Vec<Definition>> = BTreeMap::new();
        for &definition in definitions {
            zones
                .entry(definition.zone_id)
                .or_default()
                .push(definition);
        }
        zones
            .into_values()
            .flat_map(|rows| self.zone_windows(rows, now))
            .collect()
    }

    fn zone_windows(&self, mut rows: Vec<Definition>, now: i64) -> Vec<Window> {
        rows.sort_by_key(|row| row.quest_id);
        if self.descending {
            rows.reverse();
        }
        let lanes = self.active_per_zone.min(rows.len());
        (0..lanes)
            .filter_map(|lane| self.lane_window(rows.iter().skip(lane).step_by(lanes), now))
            .collect()
    }

    fn lane_window<'a>(
        &self,
        rows: impl Iterator<Item = &'a Definition> + Clone,
        now: i64,
    ) -> Option<Window> {
        let cycle: i64 = rows
            .clone()
            .map(|q| i64::from(q.duration) + i64::from(self.gap_seconds))
            .sum();
        let elapsed = now - self.epoch;
        let mut start = self.epoch + elapsed / cycle * cycle;
        for quest in rows {
            let end = start + i64::from(quest.duration);
            if start <= now && now < end {
                return Some(Window {
                    quest_id: quest.quest_id,
                    starts_at: start,
                    expires_at: end,
                });
            }
            start = end + i64::from(self.gap_seconds);
        }
        None
    }

    pub(crate) fn read_env() -> Result<Self, String> {
        let mut policy = Self::default();
        if let Ok(value) = std::env::var("WORLD_QUEST_EPOCH") {
            policy.epoch = value
                .parse()
                .map_err(|_| "WORLD_QUEST_EPOCH must be Unix seconds")?;
        }
        if let Ok(value) = std::env::var("WORLD_QUEST_ACTIVE_PER_ZONE") {
            policy.active_per_zone = value
                .parse()
                .map_err(|_| "WORLD_QUEST_ACTIVE_PER_ZONE must be positive")?;
        }
        if policy.active_per_zone == 0 {
            return Err("WORLD_QUEST_ACTIVE_PER_ZONE must be positive".into());
        }
        if let Ok(value) = std::env::var("WORLD_QUEST_GAP_SECONDS") {
            policy.gap_seconds = value
                .parse()
                .map_err(|_| "WORLD_QUEST_GAP_SECONDS must be a nonnegative u32")?;
        }
        if let Ok(value) = std::env::var("WORLD_QUEST_ORDER") {
            policy.descending = match value.as_str() {
                "ascending" => false,
                "descending" => true,
                _ => return Err("WORLD_QUEST_ORDER must be ascending or descending".into()),
            };
        }
        Ok(policy)
    }
}

pub(crate) fn load_definitions(conn: &rusqlite::Connection) -> rusqlite::Result<Vec<Definition>> {
    let installed: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='content_world_quest')",
        [], |row| row.get(0),
    )?;
    if !installed {
        return Ok(Vec::new());
    }
    let mut statement = conn.prepare(
        "SELECT quest_id, zone_id, duration_seconds FROM content_world_quest ORDER BY quest_id",
    )?;
    let definitions = statement.query_map([], |row| {
        let definition = Definition {
            quest_id: row.get(0)?,
            zone_id: row.get(1)?,
            duration: row.get(2)?,
        };
        if definition.duration == 0 {
            return Err(rusqlite::Error::InvalidQuery);
        }
        Ok(definition)
    })?;
    definitions.collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn definitions() -> Vec<Definition> {
        vec![
            Definition {
                quest_id: 52847,
                zone_id: 9734,
                duration: 604800,
            },
            Definition {
                quest_id: 52848,
                zone_id: 9734,
                duration: 604800,
            },
            Definition {
                quest_id: 54895,
                zone_id: 10288,
                duration: 604800,
            },
            Definition {
                quest_id: 54896,
                zone_id: 10288,
                duration: 604800,
            },
        ]
    }

    #[test]
    fn world_quest_rotation_is_restart_and_input_order_deterministic() {
        let policy = Rotation {
            epoch: 1000,
            ..Default::default()
        };
        let rows = definitions();
        let mut reverse = rows.clone();
        reverse.reverse();
        let expected = vec![
            Window {
                quest_id: 52847,
                starts_at: 1000,
                expires_at: 605800,
            },
            Window {
                quest_id: 54895,
                starts_at: 1000,
                expires_at: 605800,
            },
        ];
        assert_eq!(policy.active(&rows, 1000), expected);
        assert_eq!(policy.active(&reverse, 1000), expected);
        assert_eq!(policy.clone().active(&rows, 500000), expected);
    }

    #[test]
    fn world_quest_rotation_has_half_open_authentic_windows_and_configurable_order() {
        let rows = definitions();
        let policy = Rotation {
            epoch: 1000,
            ..Default::default()
        };
        assert!(policy.active(&rows, 999).is_empty());
        assert_eq!(policy.active(&rows, 605799).len(), 2);
        assert!(policy.active(&rows, 605800).is_empty());
        assert!(policy.active(&rows, 692199).is_empty());
        let second = policy.active(&rows, 692200);
        assert_eq!(
            second.iter().map(|w| w.quest_id).collect::<Vec<_>>(),
            vec![52848, 54896]
        );
        assert!(second.iter().all(|w| w.expires_at - w.starts_at == 604800));
        assert_eq!(policy.active(&rows, 1383400)[0].quest_id, 52847);
        let backwards = Rotation {
            descending: true,
            ..policy.clone()
        };
        assert_eq!(backwards.active(&rows, 1000)[0].quest_id, 52848);
        let two = Rotation {
            active_per_zone: 2,
            ..policy
        };
        assert_eq!(two.active(&rows, 1000).len(), 4);
    }

    #[test]
    fn world_quest_rotation_uses_each_duration_without_overbooking_zone() {
        let policy = Rotation {
            epoch: 0,
            gap_seconds: 0,
            ..Default::default()
        };
        let rows = vec![
            Definition {
                quest_id: 10,
                zone_id: 1,
                duration: 30,
            },
            Definition {
                quest_id: 20,
                zone_id: 1,
                duration: 60,
            },
            Definition {
                quest_id: 30,
                zone_id: 1,
                duration: 15,
            },
        ];
        assert_eq!(policy.active(&rows, 29)[0].quest_id, 10);
        assert_eq!(policy.active(&rows, 30)[0].quest_id, 20);
        assert_eq!(policy.active(&rows, 89)[0].expires_at, 90);
        assert_eq!(policy.active(&rows, 90)[0].quest_id, 30);
        assert_eq!(policy.active(&rows, 105)[0].starts_at, 105);
        for now in 0..500 {
            assert_eq!(policy.active(&rows, now).len(), 1);
        }
    }
}
