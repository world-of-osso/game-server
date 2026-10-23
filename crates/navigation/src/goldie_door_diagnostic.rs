//! Trace the candidate door model into actual authored static placement inputs.
use super::*;
use los::bake::adt::{self, ModelRef, Placement};
use std::collections::BTreeMap;
use std::io::{BufWriter, Write};

const BREWERY_DOOR_MODEL: u32 = 5768795;
const NAVIGATION_DOOR_MODEL: u32 = 5334391;

fn resolve_fdid(baker: &los::bake::Baker, model: &ModelRef) -> u32 {
    match model {
        ModelRef::Fdid(fdid) => *fdid,
        ModelRef::Path(path) => baker
            .path_fdid(path)
            .expect("authored model path must resolve"),
    }
}

fn trace_selected_door_models(
    data: &Path,
    baker: &los::bake::Baker,
    placements: BTreeMap<u32, Placement>,
    trace: &mut impl Write,
) -> (usize, usize, usize) {
    let mut definitions = 0;
    let mut brewery = 0;
    let mut navigation = 0;
    for (uid, placement) in placements {
        let fdid = resolve_fdid(baker, &placement.model);
        let path = data.join(format!("models/{fdid}.wmo"));
        let bytes = std::fs::read(&path).unwrap();
        let root = los::bake::wmo::WmoRoot::parse(&bytes).unwrap();
        for doodad in root.doodads(placement.doodad_set) {
            definitions += 1;
            brewery += usize::from(doodad.fdid == BREWERY_DOOR_MODEL);
            navigation += usize::from(doodad.fdid == NAVIGATION_DOOR_MODEL);
            if [BREWERY_DOOR_MODEL, NAVIGATION_DOOR_MODEL].contains(&doodad.fdid) {
                writeln!(trace, "selected_door parent_uid={uid} root={fdid} set={} parent={placement:?} child_fdid={} file_position={:?} file_rotation={:?} child_scale={}", placement.doodad_set, doodad.fdid, doodad.position, doodad.rotation, doodad.scale).unwrap();
            }
        }
        writeln!(
            trace,
            "selected_root uid={uid} fdid={fdid} set={}",
            placement.doodad_set
        )
        .unwrap();
    }
    (definitions, brewery, navigation)
}

#[test]
#[ignore = "requires local Goldie5319046 assets and FOLLOWER_GOREK_DIAGNOSTICS"]
fn goldie_444271_static_model_source_probe() {
    const WDT: u32 = 5319046;
    let data = PathBuf::from(std::env::var_os("FOLLOWER_COMPONENT_ASSETS").unwrap());
    let output = PathBuf::from(std::env::var_os("FOLLOWER_GOREK_DIAGNOSTICS").unwrap());
    let mut trace = BufWriter::new(
        std::fs::File::create(output.join("goldie-door-static-source.txt")).unwrap(),
    );
    let baker = los::bake::Baker::for_navigation(&data);
    let bytes = std::fs::read(data.join(format!("terrain/{WDT}.wdt"))).unwrap();
    assert!(
        adt::global_wmo(&bytes).unwrap().is_none(),
        "Goldie fixture uses authored ADT placements"
    );
    let tiles = baker.map_tiles(WDT).unwrap();
    let mut direct = BTreeMap::new();
    let mut wmos = BTreeMap::new();
    let mut source_files = HashSet::new();
    for tile in tiles {
        if tile.obj0 == 0 || !source_files.insert(tile.obj0) {
            continue;
        }
        let path = data.join(format!("terrain/{}.adt", tile.obj0));
        let (doodads, models) = adt::placements(&std::fs::read(&path).unwrap()).unwrap();
        for placement in doodads {
            direct.insert(placement.unique_id, placement);
        }
        for placement in models {
            wmos.insert(placement.unique_id, placement);
        }
        writeln!(trace, "obj0={}", path.display()).unwrap();
    }
    let mut direct_brewery = 0;
    let mut direct_navigation = 0;
    for (uid, placement) in &direct {
        let fdid = resolve_fdid(&baker, &placement.model);
        direct_brewery += usize::from(fdid == BREWERY_DOOR_MODEL);
        direct_navigation += usize::from(fdid == NAVIGATION_DOOR_MODEL);
        if [BREWERY_DOOR_MODEL, NAVIGATION_DOOR_MODEL].contains(&fdid) {
            writeln!(
                trace,
                "direct_door uid={uid} fdid={fdid} placement={placement:?}"
            )
            .unwrap();
        }
    }
    let wmo_count = wmos.len();
    let (definitions, selected_brewery, selected_navigation) =
        trace_selected_door_models(&data, &baker, wmos, &mut trace);
    writeln!(trace, "wdt={WDT} obj0_files={} direct_placements={} wmo_placements={wmo_count} selected_definitions={definitions} brewery_model={BREWERY_DOOR_MODEL} direct_brewery={direct_brewery} selected_brewery={selected_brewery} navigation_model={NAVIGATION_DOOR_MODEL} direct_navigation={direct_navigation} selected_navigation={selected_navigation}", source_files.len(), direct.len()).unwrap();
    trace.flush().unwrap();
    assert!(
        wmo_count > 0 && definitions > 0,
        "static source coverage must be nonempty"
    );
    eprintln!(
        "Goldie candidate model: direct={direct_brewery},selected={selected_brewery}; control direct={direct_navigation},selected={selected_navigation}"
    );
}
