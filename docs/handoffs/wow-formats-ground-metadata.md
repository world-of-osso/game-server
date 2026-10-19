# Runtime ground metadata extraction

Scope: `ground.rs` MPHD/MAID/MODF/MOHD/GFID metadata and its chunk/byte readers only. FDID-only refusal, per-entry skipping, loading, transforms, runtime collision, MFBO interpretation, terrain height interpolation and MH2O are unchanged. BLP is excluded. Shared views live in public wow-formats `bb989f276669e5f900e97e985542cc440dcc00a8` with renderer features disabled.

## Baseline

Server baseline `0b193f9b2` adds only the test receipt harness to master `6c8929589`; production readers are unchanged. Protocol is exact, clean `aa6d925275a2a7c23fcd9a79fb7f903d879fe196`. Old shared revision is `c06b8e20c`.

Five baseline tests passed: complete real metadata capture, selected-field/refusal boundaries, real Stockade support/fall points, Northshire terrain support, and Stockade cache/missing-cache controls. The same tests run after extraction.

The canonical projection retains source order and f32 bits: MAID root/obj0 pairs (8bytes/slot,4096slots), accepted MODF FDID/position/rotation/scale (32bytes/placement), MOHD-count GFID IDs (4bytes/group). It is a proof artifact, not a new runtime cache or protocol.

| Baseline output | Count | Bytes |
| --- | ---: | ---: |
| `maid-775971.bin` | 4096 slots | 32768 |
| `modf-778028.bin` | 7 Northshire placements | 224 |
| `modf-791060.bin` | 1 Stockade placement | 32 |
| `gfid-107074.bin` | 13 groups | 52 |
| `gfid-107202.bin` | 1 group | 4 |
| `gfid-108104.bin` | 1 group | 4 |
| `gfid-108106.bin` | 1 group | 4 |
| `gfid-108121.bin` | 1 group | 4 |
| `gfid-108631.bin` | 27 Stockade groups | 108 |
| `gfid-110750.bin` | 10 groups | 40 |
| `gfid-111538.bin` | 12 groups | 48 |

Metadata total: **11 files /33288bytes**. Fresh Stockade reference: **4LOS +4NAV files /7196988bytes**,39,791 collected navigation triangles. Timing-bearing reports are excluded. Full reference inventory with filenames, byte lengths and SHA-256, plus113 input-file hashes: `/home/osso/.worktrees/logs/wow_formats-phase3-reference-receipt.json`. Reference directories: `wow_formats-phase3-reference-metadata/` and `wow_formats-phase3-reference-bake/` under the same logs directory.

## After proof

Tested consumer revision: `9dee3c63e0edcfbbad8300aa7294aeed095ed26f`, including master `8d88013ed`; protocol remained exact and clean `aa6d925`. Both before and after passed the same **5 tests /0 failures /0 ignored** and the complete five-name multiset matches. Initial compile caught an existing STV test helper using the removed local integer reader; its binding now imports the shared reader under `cfg(test)`, without restoring a duplicate implementation.

**All19 binary outputs are byte-for-byte identical**, with identical output inventories, counts, sizes and SHA-256: **11 metadata files /33288bytes** and **4LOS +4NAV files /7196988bytes**. All**113** metadata/Stockade-bake input-file sizes and hashes remain unchanged. The metadata input-file list and output-size JSON also match exactly. No approximate float comparison substitutes for binary equality.

After receipt and complete size/hash inventories: `/home/osso/.worktrees/logs/wow_formats-phase3-after-receipt.json`. After artifacts: `wow_formats-phase3-after-metadata/`, `wow_formats-phase3-after-bake/`; successful logs: `wow_formats-phase3-after-ground-2.log`, `wow_formats-phase3-after-bake.log`. Server and engine Godot workspace format checks pass. Independent landing gates remain lead-owned.

## Exact verification commands

Every command uses `agent-run wow_formats` and `CAPACITY_OWNER=wow_formats`; capacity admission precedes builds. Set `GAME_SERVER_GROUND_DIR=/home/osso/Projects/world-of-osso/game-engine/data` and an explicit `WOW_FORMATS_METADATA_OUTPUT=<reference-or-after-directory>` for the metadata process test:

```text
cargo test --locked -p server -- ground::format_metadata_tests:: ground::tests::stockade_fall_points_are_outside_the_wmo_and_its_floors_hold_beside_them ground::tests::northshire_ground_under_the_blackrock_spy_is_the_terrain ground::tests::cached_stockade_has_every_ground_file_and_a_missing_cache_lacks_the_wdt --test-threads=1 --nocapture
cargo run --locked -p navigation --features bake --bin nav-bake -- --data /home/osso/Projects/world-of-osso/game-engine/data --world-db data/world.db --out <reference-or-after-bake-directory> --map 34
```

Reference logs: `/home/osso/.worktrees/logs/wow_formats-phase3-baseline-ground.log`, `wow_formats-phase3-reference-bake.log`. After proof must compare complete output inventories and bytes, counts/sizes/SHA-256 and113 frozen input hashes, not approximate float results. The tests preserve58-byte unscaled FDID MODF,8-byte MOHD metadata, MAID with unused fields absent, mandatory complete GFID, FDID-only refusal and first-match/visited-chunk errors.

Shared library tests: three missing-API RED builds became seven GREEN behavioral tests; full default242 and no-default162 tests pass, plus check/fmt, no warnings. Existing complete renderer parsers retain their previous stricter field boundaries; projected runtime views do not change those APIs.
