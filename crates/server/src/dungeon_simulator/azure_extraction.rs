//! Explicit extraction only: qualified navigation and authored translocation policy.
use super::*;
use std::collections::BTreeSet;

#[test]
#[ignore = "explicit extraction from staged2515 navigation; runtime never opens nav"]
fn extract_azure_graph() {
    let data = crate::test_data::world_data();
    let mut packs = source_packs(data, 2515);
    packs.push(Pack {
        id: 7116,
        position: [-5364.33008, -1035.76001],
        spawns: Vec::new(),
    });
    let directory = std::path::Path::new(
        "/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/follower-dungeons-2026-10-21/staging/navmesh/2515",
    );
    let candidates = query_edges(data, &packs, directory);
    let edges = select_edges(&packs, candidates);
    let links = [
        (29001868, 29001870),
        (29002007, 29001886),
        (29002008, 29001886),
        (29002009, 29001886),
        (29001906, 29001915),
        (29001918, 29002013),
        (29002014, 7116),
        (29002015, 7116),
        (29002016, 7116),
    ];
    let pack_for = |guid| {
        packs
            .iter()
            .find(|pack| pack.id == guid || pack.spawns.iter().any(|spawn| spawn.guid == guid))
            .unwrap()
            .id
    };
    let teleports: Vec<_> = links
        .into_iter()
        .map(|(from, to)| Edge {
            from: pack_for(from),
            to: pack_for(to),
            yards: 0.0,
        })
        .collect();
    let opening = packs
        .iter()
        .filter(|pack| pack.spawns.iter().any(|spawn| spawn.entry == 196102))
        .min_by(|a, b| {
            let distance = |pack: &Pack| {
                Vec2::from_array(pack.position).distance_squared(Vec2::new(-5050.48, -1024.56))
            };
            distance(a).total_cmp(&distance(b))
        })
        .unwrap()
        .id;
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let factions = crate::faction::load_faction_templates(&db).unwrap();
    let bosses = [186644, 186739, 186737, 186738];
    let hostile = |pack: &Pack| {
        pack.spawns.iter().any(|spawn| {
            let template = data.template_for(spawn.entry, 1).unwrap();
            bosses.contains(&spawn.entry)
                || (factions.reaction(u32::from(template.faction), 1)
                    == crate::faction::Reaction::Hostile
                    && template.aggro_mode != crate::creature_aggro::AggroMode::Passive)
        })
    };
    let mut seen = BTreeSet::new();
    let mut route = Vec::new();
    // Boss order is explicit; each stage walks only its qualified component.
    for root in [
        opening,
        pack_for(29001870),
        pack_for(29001886),
        pack_for(29001915),
        pack_for(29002013),
        7116,
    ] {
        let outside: BTreeSet<_> = unreachable(&packs, &edges, root).into_iter().collect();
        let mut current = root;
        loop {
            let next = packs
                .iter()
                .filter(|pack| {
                    !outside.contains(&pack.id) && !seen.contains(&pack.id) && hostile(pack)
                })
                .min_by(|a, b| {
                    let key = |pack: &Pack| {
                        (
                            pack.spawns
                                .iter()
                                .any(|spawn| bosses.contains(&spawn.entry)),
                            Vec2::from_array(pack.position).distance_squared(Vec2::from_array(
                                packs.iter().find(|p| p.id == current).unwrap().position,
                            )),
                        )
                    };
                    let ka = key(a);
                    let kb = key(b);
                    ka.0.cmp(&kb.0)
                        .then(ka.1.total_cmp(&kb.1))
                        .then(a.id.cmp(&b.id))
                });
            let Some(next) = next else { break };
            seen.insert(next.id);
            route.push(RouteStep {
                pack: next.id,
                anchor: next.position,
            });
            current = next.id;
        }
    }
    assert_eq!(
        route.first().unwrap().pack,
        opening,
        "authored Conjured Lasher opening"
    );
    let mut combined = edges.clone();
    combined.extend(teleports.clone());
    let graph = Graph {
        map_id: 2515,
        entrance: [-5050.48, -1024.56],
        entrance_pack: opening,
        unreachable: unreachable(&packs, &combined, opening),
        packs,
        edges,
        teleports,
        route,
    };
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../content/dungeons/azure-vault-2515.json");
    std::fs::write(&path, serde_json::to_vec_pretty(&graph).unwrap()).unwrap();
    println!(
        "AZURE_GRAPH packs={} walk_edges={} teleports={} route={} unreachable={:?}",
        graph.packs.len(),
        graph.edges.len(),
        graph.teleports.len(),
        graph.route.len(),
        graph.unreachable
    );
}
