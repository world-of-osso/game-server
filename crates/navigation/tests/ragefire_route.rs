//! Offline route receipt for private native movement; no actor relocation.
//! Endpoints come from observed native state/authored content, not new spawn data.
use glam::Vec3;
use navigation::NavigationMap;
use std::path::PathBuf;

fn read_point(variable: &str) -> Vec3 {
    let value = std::env::var(variable).expect(variable);
    let coordinates: Vec<f32> = value.split(',').map(|part| part.parse().unwrap()).collect();
    assert_eq!(coordinates.len(), 3, "{variable}: expected Bevy x,y,z");
    let point = Vec3::from_slice(&coordinates);
    assert!(point.is_finite());
    point
}

#[test]
#[ignore = "requires RAGEFIRE_NAV_ROOT, RAGEFIRE_ROUTE_FROM, RAGEFIRE_ROUTE_TO and RAGEFIRE_ROUTE_OUTPUT"]
fn ragefire_baked_route_probe() {
    let root = PathBuf::from(std::env::var_os("RAGEFIRE_NAV_ROOT").unwrap());
    let mut map = NavigationMap::open(&root.join("389")).unwrap();
    let from = read_point("RAGEFIRE_ROUTE_FROM");
    let to = read_point("RAGEFIRE_ROUTE_TO");
    let points = map
        .complete_path(from, to)
        .expect("complete baked route, never a direct shortcut");
    assert!(points.len() >= 2);
    assert!(points.first().unwrap().distance(from) < 3.0);
    assert!(points.last().unwrap().distance(to) < 3.0);
    let receipt: String = points
        .iter()
        .map(|point| format!("{}\t{}\t{}\n", point.x, point.y, point.z))
        .collect();
    let output = PathBuf::from(std::env::var_os("RAGEFIRE_ROUTE_OUTPUT").unwrap());
    std::fs::write(output, receipt).unwrap();
    println!(
        "{} source-mesh corners from {from:?} to {to:?}",
        points.len()
    );
}
