#![cfg(feature = "bake")]

#[test]
#[ignore = "requires published map 2773 assets"]
fn dense_retail_tile_rejects_polygon_neighbor_overflow() {
    let root = std::path::Path::new("/home/osso/Projects/world-of-osso/game-engine/data");
    let mut baker = los::bake::Baker::for_navigation(root);
    let tiles = baker.map_tiles(5_839_271).unwrap();
    let (geometry, _) = baker.collect_tile(&tiles, (36, 28)).unwrap().unwrap();
    assert!(baker.problems.is_empty(), "{:?}", baker.problems);
    let water = baker.collect_water(tiles[(28 * 64 + 36) as usize]).unwrap();
    let error = navigation::bake::bake_collected_on_grid(&geometry, &water, (36, 28), 4)
        .expect_err("oversized polygon indices must not be encoded as tile borders");
    assert!(
        error.contains(navigation::bake::POLYGON_CAPACITY_ERROR),
        "{error}"
    );
}
