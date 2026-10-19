use super::{
    scene::Scene,
    trace::{Row, Trace},
};
use std::collections::BTreeMap;

const COLORS: [&str; 5] = ["#ffd166", "#06d6a0", "#55bfff", "#ef78bd", "#cdadff"];
const TICK_MS: i64 = 10000;

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn point(point: &[f32; 3]) -> String {
    format!("{},{}", point[0], point[2])
}

fn bounds(trace: &Trace, scene: &Scene) -> Result<[f32; 4], String> {
    let points = trace
        .rows
        .iter()
        .filter_map(|row| row.position)
        .chain(scene.polygons.iter().flatten().copied())
        .chain(scene.spawns.iter().map(|spawn| spawn.position));
    let mut rect = [
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    ];
    for position in points {
        rect[0] = rect[0].min(position[0]);
        rect[1] = rect[1].min(position[2]);
        rect[2] = rect[2].max(position[0]);
        rect[3] = rect[3].max(position[2]);
    }
    if rect.iter().any(|value| !value.is_finite()) {
        return Err("No observed map points".into());
    }
    Ok(rect)
}

pub fn render(trace: &Trace, scene: &Scene) -> Result<String, String> {
    let [left, top, right, bottom] = bounds(trace, scene)?;
    let width = (right - left).max(40.0) + 30.0;
    let height = (bottom - top).max(40.0) + 70.0;
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1920\" height=\"1080\" viewBox=\"{} {} {width} {height}\"><rect x=\"{}\" y=\"{}\" width=\"{width}\" height=\"{height}\" fill=\"#111923\"/>",
        left - 15.0,
        top - 35.0,
        left - 15.0,
        top - 35.0
    );
    svg.push_str(&heading(trace, left, top));
    for polygon in &scene.polygons {
        let points = polygon.iter().map(point).collect::<Vec<_>>().join(" ");
        svg.push_str(&format!("<polygon points=\"{points}\" fill=\"#344452\" stroke=\"#415565\" stroke-width=\"0.08\"/>"));
    }
    svg.push_str(&spawns(trace, scene));
    svg.push_str(&paths(trace));
    svg.push_str(&markers(trace));
    svg.push_str(&legend(trace, left, bottom));
    svg.push_str("</svg>\n");
    Ok(svg)
}

fn heading(trace: &Trace, left: f32, top: f32) -> String {
    let start = &trace.rows[0];
    let run = escape(start.run_id.as_deref().unwrap_or("unnamed"));
    let revision = escape(start.revision.as_deref().unwrap_or("unknown revision"));
    let seconds = (trace.end_ms - trace.start_ms) as f64 / 1000.0;
    format!(
        "<text x=\"{left}\" y=\"{}\" fill=\"white\" font-size=\"4\">Stockade · {run} · {revision} · {seconds:.1}s</text><text x=\"{left}\" y=\"{}\" fill=\"#bbbbbb\" font-size=\"2.5\">Client100ms samples; X/Z world yards; all nav levels flattened; wall-time ticks (not simulation)</text>",
        top - 25.0,
        top - 18.0
    )
}

fn spawns(trace: &Trace, scene: &Scene) -> String {
    let states: BTreeMap<u32, bool> = trace
        .rows
        .iter()
        .filter(|row| row.event == "spawn_state")
        .filter_map(|row| row.spawn_guid.zip(row.alive))
        .collect();
    scene.spawns.iter().map(|spawn| {
        let [x, _, y] = spawn.position;
        let color = COLORS[spawn.pack as usize % COLORS.len()];
        let (status, fill) = match states.get(&spawn.guid) {
            Some(true) => ("ALIVE", "#06d6a0"),
            Some(false) => ("DEAD", "#ff6666"),
            None => ("UNKNOWN", "#889099"),
        };
        format!("<circle cx=\"{x}\" cy=\"{y}\" r=\"0.8\" fill=\"{fill}\" stroke=\"{color}\" stroke-width=\"0.3\"><title>spawn {} entry {} pack {}; end-state {status}</title></circle>", spawn.guid,spawn.entry,spawn.pack)
    }).collect()
}

fn paths(trace: &Trace) -> String {
    let mut actors: BTreeMap<&str, Vec<&Row>> = BTreeMap::new();
    for row in trace.rows.iter().filter(|row| row.event == "pose") {
        actors
            .entry(row.actor.as_deref().unwrap())
            .or_default()
            .push(row);
    }
    actors.values().enumerate().map(|(index, rows)| {
        let color = COLORS[index % COLORS.len()];
        let points = rows.iter().map(|row| point(&row.position.unwrap())).collect::<Vec<_>>().join(" ");
        let name = escape(rows[0].name.as_deref().unwrap_or("bot"));
        let mut output = format!("<polyline points=\"{points}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"0.35\"><title>{name}</title></polyline>");
        output.push_str(&time_ticks(rows, trace.start_ms, color));
        output
    }).collect()
}

fn time_ticks(rows: &[&Row], start: i64, color: &str) -> String {
    let mut next = 0;
    let mut output = String::new();
    for row in rows {
        let elapsed = row.at_ms - start;
        if elapsed < next {
            continue;
        }
        let [x, _, y] = row.position.unwrap();
        output.push_str(&format!("<circle cx=\"{x}\" cy=\"{y}\" r=\"0.45\" fill=\"{color}\"/><text x=\"{}\" y=\"{}\" fill=\"{color}\" font-size=\"1.8\">{}s</text>", x+0.8,y-0.8,elapsed/1000));
        next = (elapsed / TICK_MS + 1) * TICK_MS;
    }
    output
}

fn markers(trace: &Trace) -> String {
    trace.rows.iter().filter(|row| matches!(row.event.as_str(), "combat_start"|"combat_end"|"death"|"wipe"|"overlap_pull"))
        .filter_map(|row| trace.position_at(row).map(|position| (row,position)))
        .map(|(row,[x,_,y])| {
            let color = if row.event == "combat_start" { "#06d6a0" } else { "#ff6666" };
            let label = escape(&row.event);
            format!("<path d=\"M{},{} l2,2 m-2,0 l2,-2\" stroke=\"{color}\" stroke-width=\"0.5\"><title>{label} at {}ms</title></path><text x=\"{}\" y=\"{}\" fill=\"{color}\" font-size=\"1.8\">{label}</text>",x-1.0,y-1.0,row.at_ms-trace.start_ms,x+1.5,y)
        }).collect()
}

fn legend(trace: &Trace, left: f32, bottom: f32) -> String {
    let mut names = BTreeMap::new();
    for row in trace.rows.iter().filter(|row| row.event == "pose") {
        names.insert(
            row.actor.as_deref().unwrap(),
            row.name.as_deref().unwrap_or("bot"),
        );
    }
    let mut output = String::new();
    for (index, name) in names.values().enumerate() {
        let color = COLORS[index % COLORS.len()];
        output.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" fill=\"{color}\" font-size=\"2.5\">{}</text>",
            left,
            bottom + 7.0 + index as f32 * 3.5,
            escape(name)
        ));
    }
    output.push_str(&format!("<text x=\"{}\" y=\"{}\" fill=\"#bbbbbb\" font-size=\"2\">Dots: source spawns; ring: formation pack. X: combat/death marker.</text>",left,bottom+29.0));
    output.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"2\"><tspan fill=\"#06d6a0\">ALIVE (observed)</tspan><tspan fill=\"#ff6666\"> · DEAD (observed)</tspan><tspan fill=\"#889099\"> · UNKNOWN (no end-state census)</tspan></text>", left, bottom+32.0));
    if let Some(gaps) = &trace.rows[0].capture_gaps {
        output.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" fill=\"#bbbbbb\" font-size=\"1.5\">Gaps: {}</text>",
            left,
            bottom + 35.0,
            escape(&gaps.join("; "))
        ));
    }
    output
}
