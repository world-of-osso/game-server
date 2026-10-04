# Ground asset provisioning

Verified 2026-10-04. `ground_files.py` and `extract_ground.py` walk the same WDT/ADT/WMO dependencies as server `ground.rs`. [Contract](../../specs/ground-assets.md).

## Selection and local extraction

Default selection remains maps with `content_creature` spawns. Repeat `--map ID` to select registered maps without spawns; `--registered` selects every registered map. Explicit unknown maps fail before extraction. Map-row `db2_row_source.build` selects the CASC product: pinned `1.60.1.70205` uses `wow_classic_beta`; base Retail rows use `wow`. Unknown overlay builds fail explicitly rather than silently extracting Retail.

The closure is WDT, each MAID root/obj0, FILEDATAID MODF WMO roots and the MOHD primary group count of GFID entries. Additional GFID LOD entries are not ground dependencies. Missing parents leave their children undiscovered. Extraction repeats discovery after each pass, groups requests by product, and records failures in a TSV. Unlisted Forever files extracted as `.dat` are validated as chunk files before adopting server `.wdt`/`.adt`/`.wmo` names. `--casc-local` selects the local resolver executable; no CDN is used.

Example (scratch database containing the source-decoded Map overlay):

```text
python3 scripts/extract_ground.py --world-db <scratch-world.db> --cache <engine-data> \
  --engine <engine-checkout> --casc-local <asset-resolver>/target/debug/casc-local \
  --failed <engine-data>/zephras-ground-failed.tsv --map 2991
```

## Zephras acceptance and limits

The local cache holds 203 discovered dependencies, 32,421,875 bytes. Forty-two discovered files remain unavailable from local CASC: 24 root ADTs, nine obj0 ADTs and nine WMO roots/groups. The failure TSV lives at canonical engine `data/zephras-ground-failed.tsv`. WMO 7749471 was recovered from the existing Forever local probe, then its primary group 7749486 extracted; its other GFID entries are LODs, not missing primary groups. No live world database was changed; the overlay is in engine `data/zephras-ground-world.db`.

Real Forever fixtures exposed two parser issues:

- Independently rounded MCNK corners left sub-milliyard gaps in raw chunk bounds, including root 7199999's exact tile center. `terrain_height::find_chunk` now identifies authored chunk-grid cells instead. Existing 21 terrain tests pass, including the real Northshire swimming regression.
- Forever WMO groups use MPY2 (u16 flags/material) instead of MOPY (u8 flags/material). Shared collision now reads both, preserving the widened collision-only material sentinel. Real floor queries yield 788/2921/2279/7378 hits for sample roots 333477/7704156/7704158/7749471 respectively. Shared-protocol commit `98a3c9d` is required; server/helper proof uses that sibling source. The attempted `-p shared` test command cannot test a dependency outside the server workspace; no shared standalone-suite pass is claimed.

Three opt-in desktop fixtures pass: center support/minimum bounds, source-authored liquid type 1251 at height 756.5897, and all four sample WMO roots' primary groups/floors. The Rust-generated `/tmp/zephras-ground-heights.json` enumerates all 72 nominal WoW centers: 14 supported, 34 authored terrain holes, 24 missing roots. Hole/missing heights remain null. Three supported centers lie below authored MFBO minimum 500; `above_min_height` flags these. No starting coordinate is selected.

Run acceptance with `GAME_SERVER_GROUND_DIR` pointing at the cache:

```text
python3 scripts/build-host.py --build-host desktop --test -p server --bin game-server \
  zephras_ -- --ignored --nocapture
```

`--bin` belongs after `--test` as a Cargo selector, not before it as an adapter runtime option. Supply `DEPOT_SIBLING_SHARED_PROTOCOL` and `BUILD_HOST_SCRIPTS` when the sibling checkouts are not adjacent.

## LOS proof

Full `--map 2991 --all --no-eyes` stops on missing root 7199844; full-map coverage is blocked by local assets, not a successful bake. An explicitly bounded 48-available-root bake completed on desktop, using existing cached model/skin inputs and listfile. It produced 1,350,977 triangles (901,340 terrain, 257,762 WMO, 191,875 doodad), 52,222,400 bytes. Remaining report entries include unavailable models/skins and missing obj0/WMO files: 115 recorded problems, 66 deduplicated lines. This is partial geometry, not complete LOS closure or eye-table acceptance.

Owned output is `<engine-data>/los/2991/`; desktop scratch output is `/home/osso-test/game-engine-trial/zephras-ground/los/2991/`. Other maps and global eye tables were preserved. Runtime `LosMap` loads the bake and blocks the vertical interior probe `(2934.333,1334.333,900)`→`(2934.333,1334.333,500)` at Z 746.51. An exact-center vertical ray was clear before the ground lookup fix: baked mesh vertices retain independent f32 rounding; sub-milliyard mesh cracks remain unverified. The ground-query fix does not change baked vertices.

## Sources

- [ground_files.py](../../../scripts/ground_files.py), [extract_ground.py](../../../scripts/extract_ground.py), [selection/extraction fixtures](../../../scripts/tests/test_ground_selection.py)
- [terrain_height.rs](../../../crates/server/src/terrain_height.rs), [real Zephras fixtures](../../../crates/server/src/zephras_ground_tests.rs)
- [LOS baker](../../../crates/los/src/bin/los-bake.rs), [LOS contract](../../specs/line-of-sight.md)
- Runtime evidence: `/tmp/zephras-ground-extract.log`, `/tmp/zephras-ground-extract-recovered-root.log`, `/tmp/zephras-ground-green.log`, `/tmp/zephras-terrain-regressions.log`, `/tmp/zephras-los-partial-bake.log`, `/tmp/zephras-los-runtime-interior-probe.log`; persistent bake report `<engine-data>/los/2991/report.txt`.

## See Also

- [[skyborne]] — map registration and unchanged race start
- [Client/server ground rule](../../../../game-engine/docs/wiki/systems/player-ground.md)
