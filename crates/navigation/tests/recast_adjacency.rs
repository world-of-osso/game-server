#![cfg(feature = "bake")]

#[test]
fn recast_links_a_mesh_with_more_than_u16_max_edges() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let recast = root.join("../../vendor/recastnavigation-sys/recastnavigation/Recast");
    let temp = tempfile::tempdir().unwrap();
    let executable = temp.path().join("recast-adjacency");
    let output = std::process::Command::new("c++")
        .args([
            "-std=c++17",
            "-O2",
            "-ffunction-sections",
            "-fdata-sections",
            "-Wl,--gc-sections",
        ])
        .arg("-I")
        .arg(recast.join("Include"))
        .arg(root.join("tests/recast_adjacency_fixture.cpp"))
        .arg(recast.join("Source/RecastAlloc.cpp"))
        .arg("-o")
        .arg(&executable)
        .output()
        .expect("compile Recast adjacency fixture");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = std::process::Command::new(executable)
        .output()
        .expect("run adjacency fixture");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    println!("{}", String::from_utf8_lossy(&output.stdout));
}
