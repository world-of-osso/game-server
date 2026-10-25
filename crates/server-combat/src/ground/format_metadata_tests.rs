//! Before/after runtime metadata receipts; product lookup/transform policy is untouched.
use super::*;
use std::collections::{BTreeMap, BTreeSet};

const AZEROTH_WDT: u32 = 775971;
const STOCKADE_WDT: u32 = 791060;
const NORTHSHIRE_SLOT: usize = 48 * 64 + 32;
const PLACEMENT_BYTES: usize = 32;

fn source_root() -> PathBuf {
    std::env::var_os(GROUND_DIR_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../game-engine/data")
        })
}

fn read_input(root: &Path, inputs: &mut BTreeSet<String>, name: String) -> Vec<u8> {
    inputs.insert(name.clone());
    std::fs::read(root.join(&name)).unwrap_or_else(|error| panic!("{name}: {error}"))
}

fn placement_bytes(placements: &[WmoPlacement]) -> Vec<u8> {
    let mut output = Vec::with_capacity(placements.len() * PLACEMENT_BYTES);
    for placement in placements {
        output.extend_from_slice(&placement.fdid.to_le_bytes());
        for value in placement
            .position
            .into_iter()
            .chain(placement.rotation)
            .chain([placement.scale])
        {
            output.extend_from_slice(&value.to_bits().to_le_bytes());
        }
    }
    output
}

fn chunk(tag: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut output = tag.iter().rev().copied().collect::<Vec<_>>();
    output.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    output.extend_from_slice(payload);
    output
}

#[test]
fn runtime_metadata_capture_real_stockade_and_northshire() {
    let root = source_root();
    let mut inputs = BTreeSet::new();
    let mut outputs = BTreeMap::new();
    let azeroth = read_input(&root, &mut inputs, format!("terrain/{AZEROTH_WDT}.wdt"));
    let tiles = parse_maid(&azeroth).unwrap();
    assert_eq!(tiles.len(), 4096);
    let mut maid = Vec::with_capacity(32768);
    for tile in &tiles {
        maid.extend_from_slice(&tile.root.to_le_bytes());
        maid.extend_from_slice(&tile.obj0.to_le_bytes());
    }
    assert_eq!(maid.len(), 32768);
    outputs.insert(format!("maid-{AZEROTH_WDT}.bin"), maid);

    let stockade = read_input(&root, &mut inputs, format!("terrain/{STOCKADE_WDT}.wdt"));
    let WdtGround::GlobalWmo(stockade_root) = wdt_ground(&stockade).unwrap() else {
        panic!("Stockade must be global WMO")
    };
    assert_eq!(stockade_root, 108631);
    let global = parse_modf_entry(find_chunk(&stockade, b"MODF").unwrap().unwrap()).unwrap();
    let global_bytes = placement_bytes(&[global]);
    assert_eq!(global_bytes.len(), PLACEMENT_BYTES);
    outputs.insert(format!("modf-{STOCKADE_WDT}.bin"), global_bytes);

    let obj0 = tiles[NORTHSHIRE_SLOT].obj0;
    assert_ne!(obj0, 0);
    let objects = read_input(&root, &mut inputs, format!("terrain/{obj0}.adt"));
    let records = find_chunk(&objects, b"MODF").unwrap().unwrap();
    let placements: Vec<_> = records
        .chunks_exact(MODF_ENTRY_SIZE)
        .filter_map(|entry| parse_modf_entry(entry).ok())
        .collect();
    assert!(!placements.is_empty());
    let placed_bytes = placement_bytes(&placements);
    assert_eq!(placed_bytes.len(), PLACEMENT_BYTES * placements.len());
    outputs.insert(format!("modf-{obj0}.bin"), placed_bytes);
    let roots: BTreeSet<_> = std::iter::once(stockade_root)
        .chain(placements.iter().map(|entry| entry.fdid))
        .collect();
    for fdid in roots {
        let wmo = read_input(&root, &mut inputs, format!("models/{fdid}.wmo"));
        let groups = group_file_data_ids(&wmo).unwrap();
        assert!(!groups.is_empty());
        if fdid == stockade_root {
            assert_eq!(groups.len(), 27);
        }
        let bytes: Vec<_> = groups
            .iter()
            .flat_map(|group| group.to_le_bytes())
            .collect();
        assert_eq!(bytes.len(), groups.len() * 4);
        outputs.insert(format!("gfid-{fdid}.bin"), bytes);
    }
    let sizes: BTreeMap<_, _> = outputs
        .iter()
        .map(|(name, bytes)| (name.clone(), bytes.len()))
        .collect();
    println!("runtime metadata sizes: {sizes:?}");
    if let Some(directory) = std::env::var_os("WOW_FORMATS_METADATA_OUTPUT") {
        let directory = PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        for (name, bytes) in outputs {
            std::fs::write(directory.join(name), bytes).unwrap();
        }
        std::fs::write(
            directory.join("input-files.txt"),
            inputs.into_iter().collect::<Vec<_>>().join("\n") + "\n",
        )
        .unwrap();
        std::fs::write(
            directory.join("output-sizes.json"),
            serde_json::to_vec_pretty(&sizes).unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn runtime_metadata_preserves_selected_field_and_refusal_boundaries() {
    let mut partial = vec![0; 58];
    assert_eq!(
        parse_modf_entry(&partial).err().unwrap(),
        "MODF entry names its WMO by path (flags 0x0)"
    );
    partial[..4].copy_from_slice(&108631u32.to_le_bytes());
    partial[56..58].copy_from_slice(&MODF_FLAG_FILEDATAID.to_le_bytes());
    let placement = parse_modf_entry(&partial).unwrap();
    assert_eq!(placement.fdid, 108631);
    assert_eq!(placement.scale, 1.0);
    partial[56..58].copy_from_slice(&(MODF_FLAG_FILEDATAID | MODF_FLAG_HAS_SCALE).to_le_bytes());
    assert!(parse_modf_entry(&partial).is_err());

    let mut mohd = vec![0; 8];
    mohd[4..8].copy_from_slice(&2u32.to_le_bytes());
    let mut wmo = chunk(b"MOHD", &mohd);
    assert_eq!(
        group_file_data_ids(&wmo).unwrap_err(),
        "WMO root has no GFID"
    );
    wmo.extend(chunk(b"GFID", &17u32.to_le_bytes()));
    assert_eq!(
        group_file_data_ids(&wmo).unwrap_err(),
        "read_u32 out of bounds at 0x4"
    );
    mohd[4..8].copy_from_slice(&1u32.to_le_bytes());
    let mut wmo = chunk(b"MOHD", &mohd);
    wmo.extend(chunk(b"GFID", &17u32.to_le_bytes()));
    wmo.extend_from_slice(b"TMOM\x10\0\0\0xx");
    assert_eq!(group_file_data_ids(&wmo).unwrap(), [17]);

    let mut maid = vec![0; (4096 - 1) * 32 + 8];
    let end = maid.len();
    maid[end - 8..end - 4].copy_from_slice(&123u32.to_le_bytes());
    maid[end - 4..].copy_from_slice(&456u32.to_le_bytes());
    let tiles = parse_maid(&chunk(b"MAID", &maid)).unwrap();
    assert_eq!(tiles.len(), 4096);
    assert_eq!((tiles[4095].root, tiles[4095].obj0), (123, 456));
    assert_eq!(parse_maid(&[]).err().unwrap(), "WDT has no MAID");
    assert_eq!(
        find_chunk(b"LIAF\x10\0\0\0xx", b"MAID").unwrap_err(),
        "FAIL chunk truncated"
    );
}
