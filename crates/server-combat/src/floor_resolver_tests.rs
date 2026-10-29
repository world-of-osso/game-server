use super::*;

#[test]
fn floor_resolver_missing_geometry_is_not_a_single_floor() {
    let dir = tempfile::tempdir().unwrap();
    let source =
        Path::new("/home/osso/Projects/world-of-osso/game-engine/data/terrain/4289128.wdt");
    std::fs::create_dir(dir.path().join("terrain")).unwrap();
    std::fs::copy(source, dir.path().join("terrain/4289128.wdt")).unwrap();
    let mut resolver = FloorResolver::new(dir.path(), dir.path(), HashMap::from([(2519, 4289128)]));
    let request = Request {
        file: String::new(),
        run_start: String::new(),
        guid: String::new(),
        map: 2519,
        x: -250.0,
        y: -550.0,
        ui_map_id: None,
        neighbor_hints: Vec::new(),
    };
    assert!(matches!(
        resolver.resolve(&request),
        Resolution::Gap {
            gap: Gap::MissingGeometry { .. }
        }
    ));
}

#[test]
#[ignore = "requires local Neltharus geometry and navigation assets"]
fn floor_resolver_neltharus_real_data() {
    let geometry = Path::new("/home/osso/Projects/world-of-osso/game-engine/data");
    let nav = Path::new("/home/osso/Projects/world-of-osso/game-server/data/ground/navmesh");
    let db = rusqlite::Connection::open_with_flags(
        "/home/osso/Projects/world-of-osso/game-server/data/world.db",
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let wdt: u32 = db
        .query_row("SELECT WdtFileDataID FROM map WHERE ID=2519", [], |row| {
            row.get(0)
        })
        .unwrap();
    let mut resolver = FloorResolver::new(geometry, nav, HashMap::from([(2519, wdt)]));
    let request = Request {
        file: String::new(),
        run_start: String::new(),
        guid: String::new(),
        map: 2519,
        x: -250.0,
        y: -550.0,
        ui_map_id: Some(1),
        neighbor_hints: Vec::new(),
    };
    let Resolution::Gap {
        gap: Gap::NamedGap { candidates },
    } = resolver.resolve(&request)
    else {
        panic!("stacked floors must remain ambiguous without hints");
    };
    assert!(candidates.iter().any(|floor| (floor.z - 0.0).abs() < 1.0));
    assert!(candidates.iter().any(|floor| (floor.z - 133.6).abs() < 1.0));
    for (x, y, expected) in [
        (-250.0, -550.0, 0.0),
        (-256.0, -539.0, 58.765),
        (-268.0, -543.0, 123.003),
        (-250.0, -550.0, 133.6),
    ] {
        let request = Request {
            x,
            y,
            ..request.clone()
        };
        assert!(matches!(
            resolver.resolve(&request),
            Resolution::Gap {
                gap: Gap::NamedGap { .. }
            }
        ));
        let result =
            resolver.resolve_with_hints(&request, &[(x, y, expected, "unit".to_owned())], &[]);
        assert!(
            matches!(result, Resolution::Resolved { z, method: "neighbor_path", .. } if (z - expected).abs() < 1.0),
            "{result:?}"
        );
    }
    let later_request = Request {
        x: -248.0,
        y: -603.0,
        neighbor_hints: vec![Hint {
            guid: "later".to_owned(),
            x: -244.0,
            y: -603.0,
        }],
        ..request.clone()
    };
    assert!(
        matches!(resolver.resolve(&later_request), Resolution::Resolved { z, method: "neighbor_path", hint_ids, .. } if z.abs() < 1.0 && hint_ids == ["later"])
    );
    let ruby_wdt: u32 = db
        .query_row("SELECT WdtFileDataID FROM map WHERE ID=2521", [], |row| {
            row.get(0)
        })
        .unwrap();
    let mut ruby = FloorResolver::new(geometry, nav, HashMap::from([(2521, ruby_wdt)]));
    let request = Request {
        map: 2521,
        x: 1786.9,
        y: 27.945,
        ..request
    };
    let anchors = [
        AuthoredAnchor {
            row: AnchorRow {
                table: "tdb_creature",
                guid: 9001671,
            },
            x: 1778.87,
            y: 25.1024,
            z: 312.687,
        },
        AuthoredAnchor {
            row: AnchorRow {
                table: "tdb_creature",
                guid: 9001749,
            },
            x: 1781.53,
            y: 18.532,
            z: 313.228,
        },
    ];
    let result = ruby.resolve_with_hints(&request, &[], &anchors);
    assert!(
        matches!(result, Resolution::Resolved { z, method: "authored_neighbor_anchor", .. } if (312.0..314.0).contains(&z)),
        "{result:?}"
    );
    let mut conflicting = anchors.to_vec();
    conflicting[1].z = 128.0;
    assert!(matches!(
        ruby.resolve_with_hints(&request, &[], &conflicting),
        Resolution::Gap {
            gap: Gap::NamedGap { .. }
        }
    ));
}
