# Shared WoW Format Readers

Verified: 2026-10-18. Runtime ground metadata, `crates/los` and vehicle seat lookup consume the public `world-of-osso/wow-formats` crates, pinned by git revision. The client's [shared format documentation](../../../../game-engine/docs/wiki/systems/wow-formats.md) describes crate ownership and the extraction provenance.

## Server boundary

`los` always enables raw `wow-adt`/`wow-chunk` records for its runtime liquid parser; M2/WMO remain gated by its existing `bake` feature. All use `default-features = false`; M2 adds only `animation` (first-frame eye/head posing) and `materials` (opaque-batch selection). Vehicles add only `attachments` for static seat offsets. No particle, ribbon, light, camera, event, texture-composition, WMO presentation, or ADT mesh-helper features are enabled.

Local `bake/{m2,wmo,adt}.rs` retain server policies and public adapter names: base opaque batches, dropped triangle counts, doodad-set selection, first-frame matrices, coordinate conversion and exact terrain fan ordering. Binary record decoding is shared. `bake/chunks.rs` delegates iteration and byte reads to `wow-chunk`; its established first-chunk selection remains local. `vehicle.rs` retains the seat-enum mapping and coordinate conversion, not a second M2 reader.

Raw format metadata preserves distinctions needed by existing consumers: authored skin indexStart high bits versus the client's cumulative start; all MAID slots versus MAIN-active slots; file-axis terrain origins/heights/holes without render-only companion payloads; authored M2 face normals; and static attachment positions without activation-track evaluation.

No assets are downloaded, copied or committed. No decompiled or leaked client source was inspected. Runtime state and deployment are outside this extraction.

## Runtime ground metadata

`ground.rs` delegates MPHD/MAID, MODF transform and MOHD/GFID decoding to projected shared views. Mandatory chunks, first-match selection, FDID-only refusal, per-placement skipping, file loading, transforms and shared runtime WMO collision remain server policy. `find_chunk` and integer/vector decoding are no longer duplicate local implementations. MFBO flight-plane interpretation, terrain interpolation and MH2O are unchanged; existing flight-plane reads use the shared byte helper.

Projected views preserve the old runtime field boundary rather than adopting unrelated rendering validation: a58-byte FDID MODF without scale,8-byte MOHD prefix and MAID without unused trailing companion fields remain sufficient. Scale-bearing MODF still requires its scale bytes; missing/short GFID and path MODF still fail. Unvisited malformed render tails remain unvisited. Before/after both pass5 runtime/metadata tests; all11 metadata bins33288bytes and fresh4LOS+4NAV bins7196988bytes match byte-for-byte with identical inventories/sizes/SHA-256,113 input hashes unchanged. See the [runtime metadata proof ledger](../../handoffs/wow-formats-ground-metadata.md).

## Runtime terrain heights

`terrain_height.rs` reads fixed MCNK header fields and MCVT floats through shared projections. Root collection still validates the whole file before MH2O and individual MCNK parsing; first-MCVT selection, exact errors, low-hole widening, file-order liquid association, seam rounding, coordinates and four-triangle interpolation stay local. MH2O is unchanged. Raw header parsing does not require optional rendering payloads.

The [height proof ledger](../../handoffs/wow-formats-runtime-heights.md) specifies frozen old-reader XY bits, full decoded-height/query bytes, malformed-input outcomes and the required Northshire/Jasperlode corpus. Before/after:23 height/harness tests and4 real ground controls pass;35/35 frozen receipt/bake files19,169,465B match exactly,115/115 real input hashes unchanged. Each required ADT has256 chunks and135428 frozen queries;17 synthetic cases preserve failure/selection boundaries. Server check/fmt pass; independent lead integration remains pending.

## MH2O raw records

The client and server share12B chunk,24B instance and16B attribute projections. `los::liquid` uses those records and shared float reads, preserving its all-offset prepass, byte-per-row bitmap, strided LVF1/3 heights and inferred LiquidObject format. The client retains packed masks, sequential vertex arrays and material-deferred object bytes. No mode flag or reconciliation was added; samplers, error labels/precedence, runtime MCNK source-order association and navigation MCNK-index selection remain local.

The [MH2O ledger](../../handoffs/wow-formats-mh2o-records.md) freezes each consumer independently on three required real roots and25 synthetic payloads, plus water-bound and terrain-query outputs. Shared library2a91b224 exposes the existing raw record readers; default250/no-default170 tests and check/fmt pass. Before/after180/180 receipt/bake files28,034,481B match exactly,116/116 source inputs unchanged. Server height23, MH2O2, scoped swim10/real ground4/ocean1/navigation-bake1 controls pass; runtime server tree stays320 package/version pairs with no added crates/features. Engine water20/terrain45/ground18 and all checks/fmt pass. Independent lead integration remains pending.

## Reference proof

Reference captured with the pre-migration readers at rewritten server revision `ac36572bb`: existing LOS tests **16 passed / 0 failed / 4 ignored**. Stockade map34 reference `nav-bake` produced **4 LOS + 4 navigation tiles**, **7,196,988 bytes**; navigation collection contained39,791 triangles. SHA-256 inventory and an input superset of104 local asset files are recorded under `/home/osso/.worktrees/logs/wow_formats-phase2-*`. The after-reader gate must preserve those bytes and existing test outcomes; timing-bearing `report.txt` is not a binary-bake contract.

## Sources

- [LOS dependencies/features](../../../crates/los/Cargo.toml)
- [Vehicle dependencies](../../../crates/server/Cargo.toml)
- [Bake adapters](../../../crates/los/src/bake/mod.rs)
- [Shared format crate README](../../../../wow-formats/README.md)

## See Also

- [Navigation](navigation.md) — existing geometry selection and bake/runtime contracts
