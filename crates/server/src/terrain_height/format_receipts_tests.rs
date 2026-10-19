use super::*;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[path = "receipt_cases.rs"]
mod cases;
#[path = "receipt_codec.rs"]
mod codec;

const CORPUS: [u32; 2] = [778027, 778232];

#[derive(Clone, Copy)]
enum Mode {
    Inspect,
    Capture,
    Replay,
}

struct Config {
    mode: Mode,
    output: Option<PathBuf>,
    reference: Option<PathBuf>,
}

fn require_path(name: &str) -> PathBuf {
    std::env::var_os(name)
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("{name} is required"))
}

fn read_config() -> Config {
    let mode = match std::env::var("WOW_FORMATS_HEIGHT_MODE").as_deref() {
        Err(std::env::VarError::NotPresent) => Mode::Inspect,
        Ok("capture") => Mode::Capture,
        Ok("replay") => Mode::Replay,
        other => panic!("invalid WOW_FORMATS_HEIGHT_MODE: {other:?}"),
    };
    if matches!(mode, Mode::Inspect) {
        assert!(std::env::var_os("WOW_FORMATS_HEIGHT_OUTPUT").is_none());
        assert!(std::env::var_os("WOW_FORMATS_HEIGHT_REFERENCE").is_none());
        return Config {
            mode,
            output: None,
            reference: None,
        };
    }
    let output = require_path("WOW_FORMATS_HEIGHT_OUTPUT");
    std::fs::create_dir_all(&output).unwrap();
    assert!(
        std::fs::read_dir(&output).unwrap().next().is_none(),
        "receipt output must be empty; never overwrite a frozen reference"
    );
    let reference = if matches!(mode, Mode::Replay) {
        let reference = require_path("WOW_FORMATS_HEIGHT_REFERENCE");
        assert_ne!(
            std::fs::canonicalize(&reference).unwrap(),
            std::fs::canonicalize(&output).unwrap()
        );
        Some(reference)
    } else {
        assert!(std::env::var_os("WOW_FORMATS_HEIGHT_REFERENCE").is_none());
        None
    };
    Config {
        mode,
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
        .unwrap_or_else(|error| panic!("required receipt input {}: {error}", path.display()))
}

fn write_receipt(config: &Config, name: &str, bytes: &[u8]) {
    if let Some(output) = &config.output {
        let path = output.join(name);
        std::fs::write(&path, bytes).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    }
}

fn frozen_queries(config: &Config, fdid: u32, map: &TerrainHeightMap) -> (Vec<[f32; 2]>, Vec<u8>) {
    let name = format!("query-input-{fdid}.bin");
    if matches!(config.mode, Mode::Replay) {
        let bytes = read_required(&config.reference.as_ref().unwrap().join(name));
        return (codec::decode_query_points(&bytes), bytes);
    }
    let points = codec::generate_query_points(map);
    let bytes = codec::encode_query_points(&points);
    (points, bytes)
}

fn capture_map(config: &Config, root: &Path, fdid: u32) -> serde_json::Value {
    let input = read_required(&root.join(format!("terrain/{fdid}.adt")));
    let map = parse_adt_heights(&input).unwrap();
    assert!(!map.chunks.is_empty());
    if fdid == 778027 {
        assert_eq!(map.chunks.len(), 256);
    }
    let heights = codec::height_snapshot(&map);
    let (points, frozen) = frozen_queries(config, fdid, &map);
    assert!(!points.is_empty());
    let queries = codec::query_snapshot(&map, &points);
    write_receipt(config, &format!("heights-{fdid}.bin"), &heights);
    write_receipt(config, &format!("query-input-{fdid}.bin"), &frozen);
    write_receipt(config, &format!("queries-{fdid}.bin"), &queries);
    serde_json::json!({"chunks": map.chunks.len(), "height_bytes": heights.len(), "queries": points.len(), "query_bytes": queries.len(), "frozen_query_bytes": frozen.len()})
}

fn append_blob(bytes: &mut Vec<u8>, blob: &[u8]) {
    bytes.extend_from_slice(&u32::try_from(blob.len()).unwrap().to_le_bytes());
    bytes.extend_from_slice(blob);
}

fn parse_case(case: &cases::Case, input: &[u8]) -> (u8, Vec<u8>, serde_json::Value) {
    match (case.expected_error.as_deref(), parse_adt_heights(input)) {
        (Some(expected), Err(actual)) => {
            assert_eq!(actual, expected, "{}", case.name);
            (
                0,
                actual.as_bytes().to_vec(),
                serde_json::json!({"status": "error", "message": actual}),
            )
        }
        (None, Ok(map)) => {
            let bytes = codec::height_snapshot(&map);
            let summary = serde_json::json!({"status": "parsed", "chunks": map.chunks.len(), "bytes": bytes.len()});
            (1, bytes, summary)
        }
        (Some(expected), Ok(_)) => panic!("{} unexpectedly parsed, wanted {expected}", case.name),
        (None, Err(error)) => panic!("{} unexpectedly failed: {error}", case.name),
    }
}

fn capture_cases(config: &Config) -> (usize, Vec<u8>, BTreeMap<&'static str, serde_json::Value>) {
    let cases = cases::definitions();
    let mut bytes = codec::negative_header(cases.len());
    let mut summaries = BTreeMap::new();
    for case in &cases {
        let name = format!("case-input-{}.adt", case.name);
        let input = match &config.reference {
            Some(reference) => read_required(&reference.join(&name)),
            None => case.bytes.clone(),
        };
        let (tag, result, summary) = parse_case(case, &input);
        append_blob(&mut bytes, case.name.as_bytes());
        bytes.push(tag);
        append_blob(&mut bytes, &result);
        summaries.insert(case.name, summary);
        write_receipt(config, &name, &input);
    }
    (cases.len(), bytes, summaries)
}

#[test]
fn runtime_height_bit_receipts_use_fixed_corpus_and_frozen_query_bits() {
    let config = read_config();
    let root = data_root();
    let maps: BTreeMap<_, _> = CORPUS
        .into_iter()
        .map(|fdid| (fdid, capture_map(&config, &root, fdid)))
        .collect();
    let (case_count, negative, outcomes) = capture_cases(&config);
    write_receipt(&config, "negative-outcomes.bin", &negative);
    write_receipt(
        &config,
        "negative-outcomes.json",
        &serde_json::to_vec_pretty(&outcomes).unwrap(),
    );
    let summary = serde_json::json!({"maps": maps, "negative_cases": case_count, "negative_bytes": negative.len()});
    write_receipt(
        &config,
        "output-counts.json",
        &serde_json::to_vec_pretty(&summary).unwrap(),
    );
    let files = CORPUS.map(|id| format!("terrain/{id}.adt")).join("\n") + "\n";
    write_receipt(&config, "input-files.txt", files.as_bytes());
    println!("runtime height receipt: {summary}");
}

#[test]
fn frozen_query_codec_retains_coordinate_bits_including_negative_zero() {
    let points = [[-0.0f32, 0.0f32], [-8558.0, -490.0], [12345.25, -543.5]];
    let bytes = codec::encode_query_points(&points);
    let decoded = codec::decode_query_points(&bytes);
    assert_eq!(
        points.map(|point| point.map(f32::to_bits)).to_vec(),
        decoded
            .into_iter()
            .map(|point| point.map(f32::to_bits))
            .collect::<Vec<_>>()
    );
}
