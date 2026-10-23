# Ground assets

Provision local client-cache geometry for registered server maps, including maps without creature spawns. Sources: `scripts/ground_files.py`, `scripts/extract_ground.py`, server ground/terrain parsers and shared WMO collision. [Implementation and proof](../wiki/systems/ground-assets.md).

## What it must do

- [x] Preserve default spawned-map selection; accept repeated explicit registered-map IDs and all registered maps; reject unknown explicit maps.
- [x] Select the local CASC product from Map-row build provenance, including the pinned Forever overlay, without hardcoded map IDs.
- [x] Discover WDT MAID root/obj0, MODF WMO roots and GFID primary groups recursively; place unlisted chunk files under server-readable cache names.
- [x] Discovery registers and opens root ADTs, WMO groups and vehicle M2s without reading unused payload bytes. Preserve ordered present/missing paths, map selections, duplicate handling and open failures; parse WDT/obj0/WMO-root contents as before.
- [x] Accept real Forever MVER-18 terrain and sample WMO primary groups; return finite supported terrain centers, minimum bounds and the authored liquid surface; preserve terrain holes.
- [x] Export all 72 Zephras center records with explicit nulls for unavailable/unsupported terrain; do not invent a starting position.

## How it works

- [Provisioning, parser corrections and partial bake](../wiki/systems/ground-assets.md)
- [LOS contract](line-of-sight.md)

## Implementation inventory

- `scripts/ground_files.py`, `scripts/extract_ground.py` — map selection, build/product resolution and local dependency walk.
- `crates/server/src/terrain_height.rs`, `ground.rs` — authored-grid sampling and lazy map support.
- Sibling shared-protocol `src/ground/wmo.rs` — MOPY/MPY2 collision faces.
- `crates/server/src/zephras_ground_tests.rs` — real assets, height-grid export and floor/liquid acceptance.

## Tests asserting this spec

- `scripts/tests/test_ground_selection.py` — concrete SQLite/process fixtures, including full synthetic local extraction closure.
- `scripts/tests/test_vehicle_ground_files.py` — vehicle CLI closure and real process-I/O bound over48MiB of sparse root/group/model payloads, retaining parsed dependencies and missing-group identity.
- `crates/server/src/vehicle_tests.rs` — unchanged real-world deploy-list and missing-model assertions.
- `crates/server/src/zephras_ground_tests.rs` — three opt-in real Forever fixtures.
- `crates/server/src/terrain_height_tests.rs` — existing terrain/interpolation/holes and real Northshire water regressions.

## Known gaps (current cycle)

- [ ] Complete ground closure and all non-hole center heights require the 42 discovered files unavailable from local CASC, including 24 root tiles.
- [ ] Full 72-tile LOS geometry remains blocked by missing root/model/placement assets; the current 48-tile bake is partial. Runtime loading/hit proof covers one interior ray, not geometry parity.
- [ ] Baked mesh exact-center rounding cracks and standalone shared-protocol regression-suite execution remain unverified.

## Out of scope

Starting-coordinate selection, content spawns/quests, visual asset closure, global eye-table regeneration and live database publication.
