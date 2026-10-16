//! World of Osso deterministic rotation policy, not a reconstruction of Retail activation.
use bevy::prelude::*;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Definition {
    pub quest_id: u32,
    /// Signed QuestSortID: positive AreaTable zone, negative quest category.
    pub zone_id: i32,
    /// Authentic DB2 duration or explicitly imported realm policy duration.
    pub duration: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Area {
    pub map_id: u32,
    pub ui_map_id: u32,
    /// Source XYZ; acceptance retains its existing XY footprint policy.
    pub vertices: Vec<(f32, f32, f32)>,
}

impl Area {
    pub(crate) fn contains(&self, map_id: u32, x: f32, y: f32) -> bool {
        if self.map_id != map_id || self.vertices.len() < 3 {
            return false;
        }
        let mut inside = false;
        for (&(ax, ay, _), &(bx, by, _)) in self
            .vertices
            .iter()
            .zip(self.vertices.iter().cycle().skip(1))
        {
            let cross = (x - ax) * (by - ay) - (y - ay) * (bx - ax);
            if cross == 0.0
                && x >= ax.min(bx)
                && x <= ax.max(bx)
                && y >= ay.min(by)
                && y <= ay.max(by)
            {
                return true;
            }
            if (ay > y) != (by > y) && x < (bx - ax) * (y - ay) / (by - ay) + ax {
                inside = !inside;
            }
        }
        inside
    }
}

fn require_source_z_column(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    let has_source_z: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM pragma_table_info('content_world_quest_area') WHERE name='z')",
        [],
        |row| row.get(0),
    )?;
    if has_source_z {
        return Ok(());
    }
    Err(rusqlite::Error::InvalidColumnName(
        "content_world_quest_area.z is missing; rebuild world-quest content with \
         scripts/import_world_quests.py --source-db <world.db> --output-db <new-world.db> \
         --poi-db2-dir <installed-build-csv-directory> --duration-receipt <receipt.json>; \
         publish the verified world-quest tables before starting the server"
            .into(),
    ))
}

pub(crate) fn load_areas(
    conn: &rusqlite::Connection,
) -> rusqlite::Result<std::collections::HashMap<u32, Vec<Area>>> {
    let installed: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='content_world_quest_area')", [], |r| r.get(0))?;
    let mut result = std::collections::HashMap::new();
    if !installed {
        return Ok(result);
    }
    require_source_z_column(conn)?;
    let mut statement = conn.prepare("SELECT quest_id,blob_id,map_id,ui_map_id,x,y,z FROM content_world_quest_area ORDER BY quest_id,blob_id,point_index")?;
    let rows = statement.query_map([], |r| {
        Ok((
            r.get::<_, u32>(0)?,
            r.get::<_, u32>(1)?,
            r.get::<_, u32>(2)?,
            r.get::<_, u32>(3)?,
            r.get::<_, f32>(4)?,
            r.get::<_, f32>(5)?,
            r.get::<_, f32>(6)?,
        ))
    })?;
    let mut blobs: BTreeMap<(u32, u32), Area> = BTreeMap::new();
    for row in rows {
        let (quest, blob, map_id, ui_map_id, x, y, z) = row?;
        blobs
            .entry((quest, blob))
            .or_insert_with(|| Area {
                map_id,
                ui_map_id,
                vertices: Vec::new(),
            })
            .vertices
            .push((x, y, z));
    }
    for ((quest, _), area) in blobs {
        result.entry(quest).or_insert_with(Vec::new).push(area);
    }
    Ok(result)
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
        let mut zones: BTreeMap<i32, Vec<Definition>> = BTreeMap::new();
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
        let defaults = Self::default();
        let active_per_zone =
            read_setting("WORLD_QUEST_ACTIVE_PER_ZONE", defaults.active_per_zone)?;
        if active_per_zone == 0 {
            return Err("WORLD_QUEST_ACTIVE_PER_ZONE must be positive".into());
        }
        let order = read_setting("WORLD_QUEST_ORDER", "ascending".to_owned())?;
        let descending = match order.as_str() {
            "ascending" => false,
            "descending" => true,
            _ => return Err("WORLD_QUEST_ORDER must be ascending or descending".into()),
        };
        Ok(Self {
            epoch: read_setting("WORLD_QUEST_EPOCH", defaults.epoch)?,
            active_per_zone,
            descending,
            gap_seconds: read_setting("WORLD_QUEST_GAP_SECONDS", defaults.gap_seconds)?,
        })
    }
}

fn read_setting<T: std::str::FromStr>(name: &str, default: T) -> Result<T, String> {
    match std::env::var(name) {
        Ok(value) => value
            .parse()
            .map_err(|_| format!("invalid {name}: {value}")),
        Err(std::env::VarError::NotPresent) => Ok(default),
        Err(std::env::VarError::NotUnicode(_)) => Err(format!("{name} must be UTF-8")),
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
#[path = "world_quest_objective_tests.rs"]
mod objective_tests;

#[cfg(test)]
#[path = "world_quest_expiry_tests.rs"]
mod expiry_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_quest_rotation_env_configuration() {
        const CHILD_MODE: &str = "WORLD_QUEST_TEST_CONFIG_MODE";
        if let Ok(mode) = std::env::var(CHILD_MODE) {
            let result = Rotation::read_env();
            match mode.as_str() {
                "defaults" => {
                    let policy = result.unwrap();
                    assert_eq!(
                        (
                            policy.epoch,
                            policy.active_per_zone,
                            policy.descending,
                            policy.gap_seconds
                        ),
                        (1_767_225_600, 1, false, 86400)
                    );
                }
                "configured" => {
                    let policy = result.unwrap();
                    assert_eq!(
                        (
                            policy.epoch,
                            policy.active_per_zone,
                            policy.descending,
                            policy.gap_seconds
                        ),
                        (1000, 2, true, 0)
                    );
                }
                "invalid" => assert!(result.is_err()),
                _ => panic!("unknown configuration test mode"),
            }
            return;
        }
        let cases: &[(&str, &[(&str, &str)])] = &[
            ("defaults", &[]),
            (
                "configured",
                &[
                    ("WORLD_QUEST_EPOCH", "1000"),
                    ("WORLD_QUEST_ACTIVE_PER_ZONE", "2"),
                    ("WORLD_QUEST_ORDER", "descending"),
                    ("WORLD_QUEST_GAP_SECONDS", "0"),
                ],
            ),
            ("invalid", &[("WORLD_QUEST_ACTIVE_PER_ZONE", "0")]),
            ("invalid", &[("WORLD_QUEST_ORDER", "random")]),
            ("invalid", &[("WORLD_QUEST_EPOCH", "not-a-time")]),
            ("invalid", &[("WORLD_QUEST_GAP_SECONDS", "-1")]),
        ];
        for (mode, variables) in cases {
            let mut command = std::process::Command::new(std::env::current_exe().unwrap());
            command.args([
                "--exact",
                "world_quests::tests::world_quest_rotation_env_configuration",
                "--nocapture",
            ]);
            for name in [
                "WORLD_QUEST_EPOCH",
                "WORLD_QUEST_ACTIVE_PER_ZONE",
                "WORLD_QUEST_ORDER",
                "WORLD_QUEST_GAP_SECONDS",
            ] {
                command.env_remove(name);
            }
            let output = command
                .env(CHILD_MODE, mode)
                .envs(variables.iter().copied())
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "configuration child {mode}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

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
