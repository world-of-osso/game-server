#![cfg(feature = "bake")]

use los::{
    bake::{Baker, DEFAULT_THICKNESS_CUTOFF, adt::TileFiles},
    liquid::{self, ChunkLiquid},
};
use std::path::{Path, PathBuf};

#[path = "mh2o_receipts/cases.rs"]
mod cases;
#[path = "mh2o_receipts/codec.rs"]
mod codec;
#[path = "mh2o_receipts/queries.rs"]
mod queries;

const ROOTS: [u32; 3] = [778027, 778232, 776382];
struct Config {
    output: Option<PathBuf>,
    reference: Option<PathBuf>,
}

fn require_path(name: &str) -> PathBuf {
    std::env::var_os(name)
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("{name} required"))
}

fn read_config() -> Config {
    let mode = std::env::var("WOW_FORMATS_MH2O_MODE");
    if matches!(mode, Err(std::env::VarError::NotPresent)) {
        assert!(std::env::var_os("WOW_FORMATS_MH2O_OUTPUT").is_none());
        assert!(std::env::var_os("WOW_FORMATS_MH2O_REFERENCE").is_none());
        return Config {
            output: None,
            reference: None,
        };
    }
    let output = require_path("WOW_FORMATS_MH2O_OUTPUT");
    std::fs::create_dir_all(&output).unwrap();
    assert!(
        std::fs::read_dir(&output).unwrap().next().is_none(),
        "never overwrite frozen receipts"
    );
    let reference = match mode.as_deref() {
        Ok("capture") => {
            assert!(std::env::var_os("WOW_FORMATS_MH2O_REFERENCE").is_none());
            None
        }
        Ok("replay") => Some(require_path("WOW_FORMATS_MH2O_REFERENCE")),
        other => panic!("invalid WOW_FORMATS_MH2O_MODE: {other:?}"),
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

fn capture_queries(config: &Config, name: &str, chunks: &[ChunkLiquid]) -> usize {
    let filename = format!("query-input-{name}.bin");
    let input = match &config.reference {
        Some(reference) => read_required(&reference.join(&filename)),
        None => queries::encode(&queries::generate(chunks)),
    };
    let points = queries::decode(&input);
    write_receipt(config, &filename, &input);
    write_receipt(
        config,
        &format!("server-queries-{name}.bin"),
        &queries::outcomes(chunks, &points),
    );
    points.len()
}

fn capture_server(
    config: &Config,
    name: &str,
    result: &Result<Vec<ChunkLiquid>, String>,
) -> String {
    let bytes = match result {
        Ok(chunks) => {
            let points = capture_queries(config, name, chunks);
            println!(
                "server {name}: {} chunks, {} layers, {points} frozen queries",
                chunks.len(),
                chunks.iter().map(|c| c.layers.len()).sum::<usize>()
            );
            codec::server_layers(chunks)
        }
        Err(error) => codec::error(error),
    };
    write_receipt(config, &format!("server-layers-{name}.bin"), &bytes);
    format!(
        "{name}\tserver\t{}\t{}\n",
        u8::from(result.is_ok()),
        bytes.len()
    )
}

fn capture_client(
    config: &Config,
    name: &str,
    result: &Result<wow_adt::adt_tex::AdtWaterData, String>,
) -> String {
    let bytes = match result {
        Ok(data) => codec::client_layers(data),
        Err(error) => codec::error(error),
    };
    write_receipt(config, &format!("client-layers-{name}.bin"), &bytes);
    format!(
        "{name}\tclient\t{}\t{}\n",
        u8::from(result.is_ok()),
        bytes.len()
    )
}

fn capture_payload(config: &Config, name: &str, payload: &[u8], required: bool) -> String {
    let server = liquid::parse_mh2o(payload);
    let client = wow_adt::adt_tex::parse_mh2o(payload);
    if required {
        assert!(server.is_ok(), "required server {name}: {:?}", server.err());
        assert!(client.is_ok(), "required client {name}: {:?}", client.err());
    }
    if server.is_ok() || client.is_ok() {
        write_receipt(
            config,
            &format!("raw-{name}.bin"),
            &codec::raw_records(payload),
        );
    }
    let mut summary = capture_server(config, name, &server);
    summary.push_str(&capture_client(config, name, &client));
    summary
}

fn capture_root(config: &Config, data: &Path, fdid: u32) -> String {
    let bytes = read_required(&data.join(format!("terrain/{fdid}.adt")));
    let payload = wow_chunk::find_reversed_chunk(&bytes, b"MH2O")
        .unwrap()
        .expect("required MH2O");
    let name = fdid.to_string();
    let summary = capture_payload(config, &name, payload, true);
    let baker = Baker::new(data, DEFAULT_THICKNESS_CUTOFF);
    let bounds = baker
        .collect_water(TileFiles {
            root: fdid,
            obj0: 0,
        })
        .unwrap();
    assert!(!bounds.is_empty(), "required water bounds {fdid}");
    println!(
        "water bounds {fdid}: {} records, {} bytes",
        bounds.len(),
        12 + 24 * bounds.len()
    );
    write_receipt(
        config,
        &format!("nav-water-{name}.bin"),
        &codec::water_bounds(&bounds),
    );
    summary
}

#[test]
fn each_mh2o_consumer_preserves_its_own_frozen_layer_and_error_records() {
    let config = read_config();
    let root = data_root();
    let mut summary = ROOTS
        .into_iter()
        .map(|fdid| capture_root(&config, &root, fdid))
        .collect::<String>();
    let cases = cases::definitions();
    assert_eq!(cases.len(), 25);
    for case in cases {
        let name = format!("case-{}", case.name);
        let filename = format!("input-{name}.bin");
        let payload = match &config.reference {
            Some(reference) => read_required(&reference.join(&filename)),
            None => case.payload,
        };
        write_receipt(&config, &filename, &payload);
        summary.push_str(&capture_payload(&config, &name, &payload, false));
    }
    write_receipt(&config, "counts.txt", summary.as_bytes());
}

#[test]
fn current_bitmap_and_lvf1_layout_interpretations_are_deliberately_not_reconciled() {
    let case = cases::definitions()
        .into_iter()
        .find(|case| case.name == "lvf-1")
        .unwrap();
    let server = liquid::parse_mh2o(&case.payload).unwrap();
    let client = wow_adt::adt_tex::parse_mh2o(&case.payload).unwrap();
    let server = &server[0].layers[0];
    let client = &client.chunks[0].layers[0];
    assert_eq!(&server.exists[..2], &[0b0010_1101, 0b0000_0101]);
    assert_eq!(&client.exists[..2], &[0b0000_0101, 0b0000_0101]);
    assert_eq!(server.heights[1].to_bits(), 2.5f32.to_bits());
    assert_eq!(client.vertex_heights[1].to_bits(), 2.25f32.to_bits());
}
