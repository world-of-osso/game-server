use super::*;
use std::collections::BTreeMap;

#[path = "mfbo_receipt_cases.rs"]
mod cases;
#[path = "mfbo_receipt_codec.rs"]
mod codec;
#[path = "mfbo_receipt_queries.rs"]
mod queries;

struct RootSpec {
    fdid: u32,
    texture_fdid: u32,
    tile: (u32, u32),
}
const ROOTS: [RootSpec; 3] = [
    RootSpec {
        fdid: 779167,
        texture_fdid: 779170,
        tile: (38, 40),
    },
    RootSpec {
        fdid: 778027,
        texture_fdid: 778030,
        tile: (32, 48),
    },
    RootSpec {
        fdid: 776382,
        texture_fdid: 776385,
        tile: (23, 39),
    },
];
struct Config {
    output: Option<PathBuf>,
    reference: Option<PathBuf>,
}
struct Fixture<'a> {
    name: &'a str,
    fdid: u32,
    data_dir: &'a Path,
    root: &'a [u8],
    texture: &'a [u8],
    tile: (u32, u32),
}
struct Captured {
    server: Result<Vec<u8>, String>,
    client: Result<Vec<u8>, String>,
    queries: usize,
}

fn require_path(name: &str) -> PathBuf {
    std::env::var_os(name)
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("{name} required"))
}

fn create_empty_output() -> PathBuf {
    let output = require_path("WOW_FORMATS_MFBO_OUTPUT");
    std::fs::create_dir_all(&output).unwrap();
    assert!(
        std::fs::read_dir(&output).unwrap().next().is_none(),
        "never overwrite frozen receipts"
    );
    output
}

fn prepare_receipt_io() -> Config {
    let mode = std::env::var("WOW_FORMATS_MFBO_MODE");
    if matches!(mode, Err(std::env::VarError::NotPresent)) {
        assert!(std::env::var_os("WOW_FORMATS_MFBO_OUTPUT").is_none());
        assert!(std::env::var_os("WOW_FORMATS_MFBO_REFERENCE").is_none());
        return Config {
            output: None,
            reference: None,
        };
    }
    let output = create_empty_output();
    let reference = match mode.as_deref() {
        Ok("capture") => {
            assert!(std::env::var_os("WOW_FORMATS_MFBO_REFERENCE").is_none());
            None
        }
        Ok("replay") => Some(require_path("WOW_FORMATS_MFBO_REFERENCE")),
        other => panic!("invalid WOW_FORMATS_MFBO_MODE: {other:?}"),
    };
    if let Some(reference) = &reference {
        assert_ne!(
            std::fs::canonicalize(reference).unwrap(),
            std::fs::canonicalize(&output).unwrap()
        );
    }
    Config {
        output: Some(output),
        reference,
    }
}

fn data_root() -> PathBuf {
    std::env::var_os("GAME_SERVER_GROUND_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../game-engine/data")
        })
}

fn read_required(path: &Path) -> Vec<u8> {
    std::fs::read(path)
        .unwrap_or_else(|error| panic!("required fixture {}: {error}", path.display()))
}

fn write_receipt(config: &Config, name: &str, bytes: &[u8]) {
    if let Some(output) = &config.output {
        std::fs::write(output.join(name), bytes).unwrap();
    }
}

fn capture_queries(
    config: &Config,
    fixture: &Fixture<'_>,
    terrain: TerrainHeightMap,
    min: Option<[i16; 9]>,
) -> usize {
    let filename = format!("query-input-{}.bin", fixture.name);
    let bytes = match &config.reference {
        Some(reference) => read_required(&reference.join(&filename)),
        None => queries::encode(&queries::generate(fixture.root)),
    };
    let points = queries::decode(&bytes);
    let outcomes = queries::outcomes(terrain, min, fixture.tile, &points);
    write_receipt(config, &filename, &bytes);
    write_receipt(
        config,
        &format!("min-height-{}.bin", fixture.name),
        &outcomes,
    );
    points.len()
}

fn capture_server(config: &Config, fixture: &Fixture<'_>) -> (Result<Vec<u8>, String>, usize) {
    match load_terrain(fixture.data_dir, fixture.fdid) {
        Ok((terrain, min)) => {
            let count = capture_queries(config, fixture, terrain, min);
            let raw = find_chunk(fixture.root, b"MFBO").unwrap();
            write_receipt(
                config,
                &format!("raw-server-{}.bin", fixture.name),
                &codec::selected_payload(raw),
            );
            (Ok(codec::server_record(fixture.fdid, min)), count)
        }
        Err(error) => (Err(error), 0),
    }
}

fn capture_client(config: &Config, fixture: &Fixture<'_>) -> Result<Vec<u8>, String> {
    // The unchanged full client parser consumes its existing texture-shadow companion.
    let parsed = wow_adt::adt::load_adt_for_tile_parsed(
        fixture.root,
        fixture.tile.0,
        fixture.tile.1,
        Some(fixture.texture),
    )?;
    let raw = wow_chunk::ChunkIter::new(fixture.root)
        .filter_map(|chunk| {
            let (tag, payload) = chunk.unwrap();
            (tag == b"OBFM").then_some(payload)
        })
        .last();
    write_receipt(
        config,
        &format!("raw-client-{}.bin", fixture.name),
        &codec::selected_payload(raw),
    );
    Ok(codec::client_record(fixture.fdid, parsed.flight_bounds))
}

fn capture_fixture(config: &Config, fixture: &Fixture<'_>) -> Captured {
    let (server, queries) = capture_server(config, fixture);
    let client = capture_client(config, fixture);
    Captured {
        server,
        client,
        queries,
    }
}

fn summary(captured: &Captured) -> serde_json::Value {
    serde_json::json!({"server_parsed":captured.server.is_ok(),"client_parsed":captured.client.is_ok(),"queries":captured.queries,
        "server_record_bytes":captured.server.as_ref().ok().map(Vec::len),"client_record_bytes":captured.client.as_ref().ok().map(Vec::len),
        "server_error":captured.server.as_ref().err(),"client_error":captured.client.as_ref().err()})
}

fn capture_real(config: &Config, data: &Path) -> BTreeMap<u32, serde_json::Value> {
    let mut server = codec::header(b"MFSR", ROOTS.len());
    let mut client = codec::header(b"MFCR", ROOTS.len());
    let mut rows = BTreeMap::new();
    for root in &ROOTS {
        let bytes = read_required(&data.join(format!("terrain/{}.adt", root.fdid)));
        let texture = read_required(&data.join(format!("terrain/{}.adt", root.texture_fdid)));
        let name = root.fdid.to_string();
        let fixture = Fixture {
            name: &name,
            fdid: root.fdid,
            data_dir: data,
            root: &bytes,
            texture: &texture,
            tile: root.tile,
        };
        let captured = capture_fixture(config, &fixture);
        let s = captured.server.as_ref().unwrap();
        let c = captured.client.as_ref().unwrap();
        assert_eq!(s[4], 1, "required server MFBO");
        assert_eq!(c[4], 1, "required client MFBO");
        server.extend_from_slice(s);
        client.extend_from_slice(c);
        println!(
            "MFBO {}: server{}B/client{}B, {} frozen queries",
            root.fdid,
            s.len(),
            c.len(),
            captured.queries
        );
        rows.insert(root.fdid, summary(&captured));
    }
    assert_eq!(server.len(), 81);
    assert_eq!(client.len(), 135);
    write_receipt(config, "server-real.bin", &server);
    write_receipt(config, "client-real.bin", &client);
    rows
}

fn capture_cases(config: &Config, data: &Path) -> BTreeMap<String, serde_json::Value> {
    let base = &ROOTS[0];
    let root = read_required(&data.join(format!("terrain/{}.adt", base.fdid)));
    let texture = read_required(&data.join(format!("terrain/{}.adt", base.texture_fdid)));
    let scratch = tempfile::tempdir().unwrap();
    std::fs::create_dir(scratch.path().join("terrain")).unwrap();
    let mut rows = BTreeMap::new();
    for case in cases::definitions(&root) {
        let name = format!("case-{}", case.name);
        let filename = format!("input-{name}.adt");
        let bytes = match &config.reference {
            Some(reference) => read_required(&reference.join(&filename)),
            None => case.bytes,
        };
        write_receipt(config, &filename, &bytes);
        std::fs::write(scratch.path().join("terrain/1.adt"), &bytes).unwrap();
        let fixture = Fixture {
            name: &name,
            fdid: 1,
            data_dir: scratch.path(),
            root: &bytes,
            texture: &texture,
            tile: base.tile,
        };
        let captured = capture_fixture(config, &fixture);
        write_receipt(
            config,
            &format!("server-{name}.bin"),
            &codec::outcome(b"MFSR", &captured.server),
        );
        write_receipt(
            config,
            &format!("client-{name}.bin"),
            &codec::outcome(b"MFCR", &captured.client),
        );
        rows.insert(name, summary(&captured));
    }
    assert_eq!(rows.len(), 51);
    rows
}

#[test]
fn mfbo_consumers_keep_frozen_signed_records_queries_selection_and_error_order() {
    let config = prepare_receipt_io();
    let data = data_root();
    let real = capture_real(&config, &data);
    let cases = capture_cases(&config, &data);
    let real_count = real.len();
    let case_count = cases.len();
    let counts = serde_json::json!({"real":real,"cases":cases,"server_real_bytes":81,"client_real_bytes":135});
    write_receipt(
        &config,
        "counts.json",
        &serde_json::to_vec_pretty(&counts).unwrap(),
    );
    println!("MFBO receipt: {real_count} required roots, {case_count} synthetic cases");
}

#[test]
fn duplicate_mfbo_keeps_runtime_first_and_client_last_with_own_failure_precedence() {
    let data = data_root();
    let spec = &ROOTS[0];
    let root = read_required(&data.join(format!("terrain/{}.adt", spec.fdid)));
    let texture = read_required(&data.join(format!("terrain/{}.adt", spec.texture_fdid)));
    let scratch = tempfile::tempdir().unwrap();
    std::fs::create_dir(scratch.path().join("terrain")).unwrap();
    let definitions = cases::definitions(&root);
    for (name, server_ok, client_ok) in [
        ("duplicate-distinct", true, true),
        ("duplicate-first-short", false, true),
        ("duplicate-last-short", true, false),
    ] {
        let case = definitions.iter().find(|case| case.name == name).unwrap();
        std::fs::write(scratch.path().join("terrain/1.adt"), &case.bytes).unwrap();
        let server = load_terrain(scratch.path(), 1);
        let client = wow_adt::adt::load_adt_for_tile_parsed(
            &case.bytes,
            spec.tile.0,
            spec.tile.1,
            Some(&texture),
        );
        assert_eq!(server.is_ok(), server_ok, "{name}");
        assert_eq!(client.is_ok(), client_ok, "{name}");
        if name == "duplicate-distinct" {
            assert_eq!(
                server.unwrap().1.unwrap(),
                [
                    -1000, -1000, -1237, -1000, -1000, -1237, -1000, -1000, -1237
                ]
            );
            assert_eq!(
                client.unwrap().flight_bounds.unwrap().min_heights,
                [
                    -1500, -1501, -1502, -1503, -1504, -1505, -1506, -1507, -1508
                ]
            );
        }
    }
}
