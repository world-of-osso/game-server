use rusqlite::Connection;
use server_combat::floor_resolver::{AnchorRow, AuthoredAnchor, Request};
use std::collections::{HashMap, HashSet};

#[cfg(test)]
#[path = "anchors_tests.rs"]
mod tests;

const RADIUS: f32 = 15.0;
type Cell = (u32, i32, i32);

pub struct AnchorIndex(HashMap<Cell, Vec<AuthoredAnchor>>);

fn cell(map: u32, x: f32, y: f32) -> Cell {
    (
        map,
        (x / RADIUS).floor() as i32,
        (y / RADIUS).floor() as i32,
    )
}

impl AnchorIndex {
    pub fn read(db: &Connection, requests: &[Request]) -> rusqlite::Result<Self> {
        let maps: HashSet<_> = requests.iter().map(|request| request.map).collect();
        let mut index = Self(HashMap::new());
        for table in ["tdb_creature", "content_creature", "tdb_gameobject"] {
            let condition = if table == "content_creature" {
                " WHERE instr(upper(coalesce(Comment,'')), 'LOG_DERIVED') = 0 AND instr(upper(coalesce(Comment,'')), 'JOURNAL') = 0"
            } else {
                ""
            };
            let sql = format!(
                "SELECT guid, map, position_x, position_y, position_z FROM {table}{condition}"
            );
            let mut query = db.prepare(&sql)?;
            let rows = query.query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, u32>(1)?,
                    row.get::<_, f32>(2)?,
                    row.get::<_, f32>(3)?,
                    row.get::<_, f32>(4)?,
                ))
            })?;
            for row in rows {
                let (guid, map, x, y, z) = row?;
                if maps.contains(&map) {
                    index
                        .0
                        .entry(cell(map, x, y))
                        .or_default()
                        .push(AuthoredAnchor {
                            row: AnchorRow { table, guid },
                            x,
                            y,
                            z,
                        });
                }
            }
        }
        Ok(index)
    }

    pub fn near(&self, request: &Request) -> Vec<AuthoredAnchor> {
        let (_, cx, cy) = cell(request.map, request.x, request.y);
        let mut anchors = Vec::new();
        for x in cx - 1..=cx + 1 {
            for y in cy - 1..=cy + 1 {
                if let Some(rows) = self.0.get(&(request.map, x, y)) {
                    anchors.extend(
                        rows.iter()
                            .filter(|anchor| {
                                (anchor.x - request.x).hypot(anchor.y - request.y) <= RADIUS
                            })
                            .cloned(),
                    );
                }
            }
        }
        anchors
    }
}
