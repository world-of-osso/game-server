//! TC ConditionMgr.cpp EvalSingleValue/EvalValue/EvalRelOp and
//! IsMeetingWorldStateExpression, over world-state values fixed at load.
use rusqlite::{Connection, OptionalExtension};
use std::collections::HashMap;
use std::path::Path;

// TC DBCEnums.h WorldStateExpressionValueType.
const CONSTANT: u8 = 1;
const WORLD_STATE: u8 = 2;
const FUNCTION: u8 = 3;

/// Native WorldStateExpression bytecode by ID.
#[derive(Default)]
pub(crate) struct WorldStateExpressions(HashMap<u32, String>);

#[derive(serde::Deserialize)]
struct Row {
    #[serde(rename = "ID")]
    id: u32,
    #[serde(rename = "Expression")]
    expression: String,
}

impl WorldStateExpressions {
    pub(crate) fn read(db2: &Path) -> Result<Self, String> {
        let path = db2.join("WorldStateExpression.csv");
        let mut reader = csv::Reader::from_path(&path)
            .map_err(|error| format!("open {}: {error}", path.display()))?;
        reader
            .deserialize::<Row>()
            .map(|row| row.map(|row| (row.id, row.expression)))
            .collect::<Result<_, _>>()
            .map(Self)
            .map_err(|error| format!("read {}: {error}", path.display()))
    }

    /// Whether expression `id` holds. TC WorldStateMgr::GetValue/LoadFromDB: a world
    /// state without a map-scoped template reads its realm value, the template's
    /// DefaultValue or else 0; this server sets no realm world state at runtime.
    /// Map-scoped states and time/random functions are not load-time values.
    pub(crate) fn evaluate(&self, conn: &Connection, id: u32) -> Result<bool, String> {
        let expression = self
            .0
            .get(&id)
            .ok_or_else(|| format!("WorldStateExpression {id} missing"))?;
        let bytes = decode_hex(expression)
            .ok_or_else(|| format!("WorldStateExpression {id}: malformed bytecode"))?;
        evaluate(&bytes, &|state| realm_value(conn, state))
            .map_err(|reason| format!("WorldStateExpression {id}: {reason}"))
    }
}

fn realm_value(conn: &Connection, state: i32) -> Result<i32, String> {
    let template = conn
        .query_row(
            "SELECT DefaultValue,MapIDs FROM tdb_world_state WHERE ID=?1",
            [state],
            |row| Ok((row.get::<_, i32>(0)?, row.get::<_, Option<String>>(1)?)),
        )
        .optional()
        .map_err(|error| format!("read world state {state}: {error}"))?;
    match template {
        None => Ok(0),
        Some((default, maps)) if maps.as_deref().is_none_or(str::is_empty) => Ok(default),
        Some(_) => Err(format!("world state {state} is map-scoped")),
    }
}

fn decode_hex(text: &str) -> Option<Vec<u8>> {
    let text = text.as_bytes();
    if text.len() % 2 != 0 {
        return None;
    }
    text.chunks(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok())
        .collect()
}

struct Bytecode<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Bytecode<'_> {
    fn byte(&mut self) -> Result<u8, String> {
        let byte = *self.bytes.get(self.at).ok_or("truncated bytecode")?;
        self.at += 1;
        Ok(byte)
    }

    fn int(&mut self) -> Result<i32, String> {
        let bytes = self
            .bytes
            .get(self.at..self.at + 4)
            .ok_or("truncated bytecode")?;
        self.at += 4;
        Ok(i32::from_le_bytes(bytes.try_into().expect("four bytes")))
    }
}

type Values<'a> = dyn Fn(i32) -> Result<i32, String> + 'a;

/// IsMeetingWorldStateExpression: enabled flag, then relations joined by
/// And1/Or2/Xor3 until None0 or the end of the bytecode.
fn evaluate(bytes: &[u8], values: &Values) -> Result<bool, String> {
    let mut code = Bytecode { bytes, at: 0 };
    if bytes.is_empty() || code.byte()? == 0 {
        return Ok(false);
    }
    let mut result = relation(&mut code, values)?;
    loop {
        let logic = code.byte()?;
        if logic == 0 {
            break;
        }
        let second = relation(&mut code, values)?;
        result = match logic {
            1 => result && second,
            2 => result || second,
            3 => result != second,
            _ => result,
        };
        if code.at >= bytes.len() {
            break;
        }
    }
    Ok(result)
}

/// EvalRelOp: Equal1, NotEqual2, Less3, LessOrEqual4, Greater5, GreaterOrEqual6;
/// None0 tests the left value against zero.
fn relation(code: &mut Bytecode, values: &Values) -> Result<bool, String> {
    let left = value(code, values)?;
    let comparison = code.byte()?;
    if comparison == 0 {
        return Ok(left != 0);
    }
    let right = value(code, values)?;
    Ok(match comparison {
        1 => left == right,
        2 => left != right,
        3 => left < right,
        4 => left <= right,
        5 => left > right,
        6 => left >= right,
        _ => false,
    })
}

/// EvalValue: Sum1, Substraction2, Multiplication3, Division4, Remainder5;
/// a zero divisor yields 0.
fn value(code: &mut Bytecode, values: &Values) -> Result<i32, String> {
    let left = single_value(code, values)?;
    let operator = code.byte()?;
    if operator == 0 {
        return Ok(left);
    }
    let right = single_value(code, values)?;
    Ok(match operator {
        1 => left.wrapping_add(right),
        2 => left.wrapping_sub(right),
        3 => left.wrapping_mul(right),
        4 if right == 0 => 0,
        4 => left.wrapping_div(right),
        5 if right == 0 => 0,
        5 => left.wrapping_rem(right),
        _ => left,
    })
}

fn single_value(code: &mut Bytecode, values: &Values) -> Result<i32, String> {
    match code.byte()? {
        CONSTANT => code.int(),
        WORLD_STATE => values(code.int()?),
        FUNCTION => Err(format!("function {} is not a load-time value", code.int()?)),
        kind => Err(format!("unknown value type {kind}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read() -> (WorldStateExpressions, Connection) {
        let path = crate::test_data::world_db_path();
        let conn =
            Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let db2 = path.parent().unwrap().join("db2/active");
        (WorldStateExpressions::read(&db2).unwrap(), conn)
    }

    #[test]
    fn world_state_expressions_read_realm_values_like_trinitycore() {
        let (expressions, conn) = read();
        // 38025: untemplated world state21851 == 1; nothing sets it, so it reads 0.
        assert_eq!(expressions.evaluate(&conn, 38025), Ok(false));
        // World state17043 is a realm template with DefaultValue10.
        assert_eq!(expressions.evaluate(&conn, 28225), Ok(false), ">= 20");
        assert_eq!(expressions.evaluate(&conn, 29288), Ok(true), "< 20");
    }

    #[test]
    fn world_state_expressions_without_a_load_time_value_are_explicit() {
        let (expressions, conn) = read();
        // 721 reads world state602, scoped to maps 30 and 2197.
        let mapped = expressions.evaluate(&conn, 721).unwrap_err();
        assert!(mapped.contains("map-scoped"), "{mapped}");
        // 61 reads the random function.
        let function = expressions.evaluate(&conn, 61).unwrap_err();
        assert!(function.contains("function 1"), "{function}");
    }
}
