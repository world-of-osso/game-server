pub mod scene;
mod svg;
mod trace;

pub fn render_trace(input: &str, scene: &scene::Scene) -> Result<String, String> {
    svg::render(&trace::Trace::parse(input)?, scene)
}
