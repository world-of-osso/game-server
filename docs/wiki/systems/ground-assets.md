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

Post-sync map-2991 extraction at server `c5198367` finds **256 cached ground dependencies, zero missing**, including all 72 root/obj0 pairs. This supersedes the earlier 203-present/42-missing inventory. Current map-2991 failures were cleared from engine `data/zephras-ground-failed.tsv` after preserving its prior contents. WMO GFID LODs remain excluded from primary collision dependencies. Refreshed files were supplied to owned desktop `/home/osso-test/game-engine-trial/zephras-ground`; no live world/player database changed. [Revision-scoped refresh ledger](../../../target/zephras-content-proof/ledger.md).

Real Forever fixtures exposed two parser issues:

- Independently rounded MCNK corners left sub-milliyard gaps in raw chunk bounds, including root 7199999's exact tile center. `terrain_height::find_chunk` now identifies authored chunk-grid cells instead. Existing 21 terrain tests pass, including the real Northshire swimming regression.
- Forever WMO groups use MPY2 (u16 flags/material) instead of MOPY (u8 flags/material). Shared collision now reads both, preserving the widened collision-only material sentinel. Real floor queries yield 788/2921/2279/7378 hits for sample roots 333477/7704156/7704158/7749471 respectively. Shared-protocol commit `98a3c9d` is required; server/helper proof uses that sibling source. The attempted `-p shared` test command cannot test a dependency outside the server workspace; no shared standalone-suite pass is claimed.

Four opt-in desktop fixtures pass at `c5198367`: center support/minimum bounds, source-authored liquid type 1251 at height 756.5897, four sample WMO roots' primary groups/floors and approved race starts. Refreshed Rust-generated `/tmp/zephras-ground-heights.json` enumerates all 72 nominal WoW centers: **22 supported, 50 authored terrain holes, zero missing roots**. Hole heights remain null; `above_min_height` retains below-MFBO flags. Existing Python `Terrain.position` agrees with all 72 native records (supported heights within 0.01 yards), `target/forever-resync/terrain-vs-native.json`. No new starting coordinate is selected.

Run acceptance with `GAME_SERVER_GROUND_DIR` pointing at the cache:

```text
python3 scripts/build-host.py --build-host desktop --test -p server --bin game-server \
  zephras_ -- --ignored --nocapture
```

`--bin` belongs after `--test` as a Cargo selector, not before it as an adapter runtime option. Supply `DEPOT_SIBLING_SHARED_PROTOCOL` and `BUILD_HOST_SCRIPTS` when the sibling checkouts are not adjacent.

## LOS proof

Post-sync desktop `--map 2991 --all --no-eyes` exits 0: **72 tiles, 2,171,579 triangles, 84,995,712 bytes, zero missing/unreadable files**. The unchanged one-yard thickness policy deliberately drops 142 thin doodad model types; these are not extraction failures. `target/forever-resync/los-all.log` and `los-report.json` preserve exit/report evidence. This supersedes the blocked full-map and 48-tile partial bake (1,350,977 triangles, 115 recorded/66 unique problems). Eye tables were intentionally not rebuilt; no eye-table acceptance follows.

Owned output is `<engine-data>/los/2991/`; desktop scratch output is `/home/osso-test/game-engine-trial/zephras-ground/los/2991/`. Other maps and global eye tables were preserved. Runtime `LosMap` loads the bake and blocks the vertical interior probe `(2934.333,1334.333,900)`→`(2934.333,1334.333,500)` at Z 746.51. An exact-center vertical ray was clear before the ground lookup fix: baked mesh vertices retain independent f32 rounding; sub-milliyard mesh cracks remain unverified. The ground-query fix does not change baked vertices.

## Before-only metadata tooling qualification

The [ground-files qualification ledger](../../handoffs/wow-formats-ground-files-qualification.md) records actual ordered walks plus normal/missing CLI outputs and failures on frozen real/synthetic inputs. It makes no decoder replacement or deploy/extraction claim. Existing permissive chunk/record/group handling is script policy, not permission to substitute stricter runtime projections. A raw-field reuse boundary and any cross-language entry point require separate approval.

## Sources

- [ground_files.py](../../../scripts/ground_files.py), [extract_ground.py](../../../scripts/extract_ground.py), [selection/extraction fixtures](../../../scripts/tests/test_ground_selection.py)
- [terrain_height.rs](../../../crates/server/src/terrain_height.rs), [real Zephras fixtures](../../../crates/server/src/zephras_ground_tests.rs)
- [LOS baker](../../../crates/los/src/bin/los-bake.rs), [LOS contract](../../specs/line-of-sight.md)
- Runtime evidence: `/tmp/zephras-ground-extract.log`, `/tmp/zephras-ground-extract-recovered-root.log`, `/tmp/zephras-ground-green.log`, `/tmp/zephras-terrain-regressions.log`, `/tmp/zephras-los-partial-bake.log`, `/tmp/zephras-los-runtime-interior-probe.log`; persistent bake report `<engine-data>/los/2991/report.txt`.

## See Also

- [[skyborne]] — map registration and unchanged race start
- [Client/server ground rule](../../../../game-engine/docs/wiki/systems/player-ground.md)
