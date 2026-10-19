use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Row {
    pub event: String,
    pub at_ms: i64,
    pub actor: Option<String>,
    pub name: Option<String>,
    pub position: Option<[f32; 3]>,
    pub facing: Option<f32>,
    pub run_id: Option<String>,
    pub revision: Option<String>,
    pub map_id: Option<u32>,
    pub capture_gaps: Option<Vec<String>>,
}

pub struct Trace {
    pub rows: Vec<Row>,
    pub start_ms: i64,
    pub end_ms: i64,
}

impl Trace {
    pub fn parse(input: &str) -> Result<Self, String> {
        let rows: Vec<Row> = input
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line).map_err(|error| error.to_string()))
            .collect::<Result<_, _>>()?;
        let first = rows.first().ok_or("Trace is empty")?;
        let last = rows.last().unwrap();
        if first.event != "run_start" || first.map_id != Some(34) || last.event != "run_end" {
            return Err("Trace must start with map34 run_start and finish with run_end".into());
        }
        let start_ms = first.at_ms;
        let end_ms = last.at_ms;
        if end_ms < start_ms {
            return Err("Trace duration is negative".into());
        }
        for row in &rows {
            validate_pose(row)?;
        }
        Ok(Self {
            rows,
            start_ms,
            end_ms,
        })
    }

    pub fn position_at(&self, event: &Row) -> Option<[f32; 3]> {
        event.position.or_else(|| {
            self.rows
                .iter()
                .rev()
                .find(|row| {
                    row.event == "pose" && row.actor == event.actor && row.at_ms <= event.at_ms
                })
                .and_then(|row| row.position)
        })
    }
}

fn validate_pose(row: &Row) -> Result<(), String> {
    if row
        .position
        .is_some_and(|point| !point.iter().all(|value| value.is_finite()))
        || row.facing.is_some_and(|value| !value.is_finite())
    {
        return Err("Trace contains non-finite pose".into());
    }
    if row.event == "pose" && (row.actor.is_none() || row.position.is_none()) {
        return Err("Pose needs observed actor and position".into());
    }
    Ok(())
}
