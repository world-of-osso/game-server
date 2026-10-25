//! TC a352b1fa SpellInfo.cpp532–553 and DB2Stores.cpp2296–2458.
//! Only referenced curves are loaded; native70448 uses Curve.Type0 and2.
use std::collections::{BTreeSet, HashMap};

use rusqlite::{Connection, types::Type};

#[derive(Debug, Clone, Copy)]
enum Operation {
    None,
    Set,
    Multiply,
}

#[derive(Debug)]
struct EffectPoints {
    index: u8,
    operation: Operation,
    curve_id: u32,
}

#[derive(Debug, Clone, Copy)]
enum CurveKind {
    Linear,
    Bezier,
}

#[derive(Debug)]
struct Curve {
    kind: CurveKind,
    points: Vec<(f32, f32)>,
}

#[derive(Debug, Default)]
pub(super) struct TraitEffectPoints {
    definitions: HashMap<u32, Vec<EffectPoints>>,
    curves: HashMap<u32, Curve>,
}

impl TraitEffectPoints {
    pub(super) fn load(conn: &Connection) -> rusqlite::Result<Self> {
        let definitions = load_definitions(conn)?;
        let curve_ids: BTreeSet<_> = definitions
            .values()
            .flatten()
            .map(|points| points.curve_id)
            .collect();
        let curves = curve_ids
            .into_iter()
            .map(|id| load_curve(conn, id).map(|curve| (id, curve)))
            .collect::<rusqlite::Result<_>>()?;
        Ok(Self {
            definitions,
            curves,
        })
    }

    pub(super) fn apply(&self, definition: u32, rank: u32, index: u8, value: f32) -> f32 {
        let points = self
            .definitions
            .get(&definition)
            .and_then(|effects| effects.iter().find(|effect| effect.index == index));
        let Some(points) = points else {
            return value;
        };
        let curve_value = self.curves[&points.curve_id].evaluate(rank as f32);
        match points.operation {
            Operation::None => value,
            Operation::Set => curve_value,
            Operation::Multiply => value * curve_value,
        }
    }
}

fn invalid_data(message: String) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, Type::Integer, message.into())
}

fn load_definitions(conn: &Connection) -> rusqlite::Result<HashMap<u32, Vec<EffectPoints>>> {
    let mut statement = conn.prepare(
        "SELECT TraitDefinitionID,EffectIndex,OperationType,CurveID
         FROM trait_definition_effect_points ORDER BY ID",
    )?;
    let rows = statement.query_map([], |row| {
        let operation = match row.get::<_, i32>(2)? {
            -1 => Operation::None,
            0 => Operation::Set,
            1 => Operation::Multiply,
            value => {
                return Err(invalid_data(format!(
                    "unknown trait points operation {value}"
                )));
            }
        };
        Ok((
            row.get::<_, u32>(0)?,
            EffectPoints {
                index: row.get(1)?,
                operation,
                curve_id: row.get(3)?,
            },
        ))
    })?;
    let mut definitions: HashMap<u32, Vec<EffectPoints>> = HashMap::new();
    for row in rows {
        let (id, points) = row?;
        definitions.entry(id).or_default().push(points);
    }
    Ok(definitions)
}

fn load_curve(conn: &Connection, id: u32) -> rusqlite::Result<Curve> {
    let kind = conn.query_row("SELECT Type FROM curve WHERE ID=?1", [id], |row| match row
        .get::<_, u32>(0)?
    {
        0 => Ok(CurveKind::Linear),
        2 => Ok(CurveKind::Bezier),
        value => Err(invalid_data(format!(
            "trait curve {id} has unsupported Type {value}"
        ))),
    })?;
    let points = load_curve_points(conn, id)?;
    if points.is_empty() {
        return Err(invalid_data(format!(
            "trait curve {id} has no CurvePoint rows"
        )));
    }
    Ok(Curve { kind, points })
}

fn load_curve_points(conn: &Connection, id: u32) -> rusqlite::Result<Vec<(f32, f32)>> {
    let mut statement = conn
        .prepare("SELECT Pos_0,Pos_1 FROM curve_point WHERE CurveID=?1 ORDER BY OrderIndex,ID")?;
    statement
        .query_map([id], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect()
}

impl Curve {
    fn evaluate(&self, x: f32) -> f32 {
        if self.points.len() == 1 {
            return self.points[0].1;
        }
        match self.kind {
            CurveKind::Linear => self.interpolate_segment(x, false),
            CurveKind::Bezier if self.points.len() == 2 => self.interpolate_segment(x, true),
            CurveKind::Bezier => self.interpolate_bezier(x),
        }
    }

    fn interpolate_segment(&self, x: f32, cosine: bool) -> f32 {
        let right = self.points.iter().take_while(|point| point.0 <= x).count();
        if right == 0 {
            return self.points[0].1;
        }
        if right == self.points.len() {
            return self.points[right - 1].1;
        }
        let (x0, y0) = self.points[right - 1];
        let (x1, y1) = self.points[right];
        if x1 == x0 {
            return y1;
        }
        let mu = (x - x0) / (x1 - x0);
        if cosine {
            (y1 - y0) * (1.0 - (mu * std::f32::consts::PI).cos()) * 0.5 + y0
        } else {
            mu * (y1 - y0) + y0
        }
    }

    fn interpolate_bezier(&self, x: f32) -> f32 {
        let first = self.points[0];
        let last = *self.points.last().unwrap();
        if first.0 == last.0 {
            return if self.points.len() <= 4 {
                self.points[1].1
            } else {
                last.1
            };
        }
        let mu = (x - first.0) / (last.0 - first.0);
        let inverse = 1.0 - mu;
        match self.points.as_slice() {
            [(_, y0), (_, y1), (_, y2)] => {
                inverse * inverse * y0 + inverse * 2.0 * mu * y1 + mu * mu * y2
            }
            [(_, y0), (_, y1), (_, y2), (_, y3)] => {
                inverse * inverse * inverse * y0
                    + 3.0 * mu * inverse * inverse * y1
                    + 3.0 * mu * mu * inverse * y2
                    + mu * mu * mu * y3
            }
            _ => self.interpolate_de_casteljau(mu),
        }
    }

    fn interpolate_de_casteljau(&self, mu: f32) -> f32 {
        let mut values: Vec<_> = self.points.iter().map(|point| point.1).collect();
        for width in (1..values.len()).rev() {
            for index in 0..width {
                let value = values[index] + mu * (values[index + 1] - values[index]);
                values[index] = value;
            }
        }
        values[0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_trait_curve_82523_uses_tc_bezier_not_control_point_lookup() {
        let conn = Connection::open_with_flags(
            crate::test_data::world_db_path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        // Native CurvePoint239069–239071: (0,0), (0,99), (99,99).
        let curve = load_curve(&conn, 82523).unwrap();
        assert_eq!(curve.evaluate(0.0), 0.0);
        assert!((curve.evaluate(1.0) - 1.989_899).abs() < 0.000_01);
        assert!((curve.evaluate(2.0) - 3.959_596).abs() < 0.000_01);
        assert_eq!(curve.evaluate(99.0), 99.0);
    }
}
