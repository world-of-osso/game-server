# Shared WoW Format Readers

Verified: 2026-10-18. `crates/los` and vehicle seat lookup consume the public `world-of-osso/wow-formats` crates, pinned by git revision. The client's [shared format documentation](../../../../game-engine/docs/wiki/systems/wow-formats.md) describes crate ownership and the extraction provenance.

## Server boundary

`los` enables the shared crates only for its existing `bake` feature. All use `default-features = false`; M2 adds only `animation` (first-frame eye/head posing) and `materials` (opaque-batch selection). Vehicles add only `attachments` for static seat offsets. No particle, ribbon, light, camera, event, texture-composition, WMO presentation, or ADT mesh-helper features are enabled.

Local `bake/{m2,wmo,adt}.rs` retain server policies and public adapter names: base opaque batches, dropped triangle counts, doodad-set selection, first-frame matrices, coordinate conversion and exact terrain fan ordering. Binary record decoding is shared. `bake/chunks.rs` delegates iteration and byte reads to `wow-chunk`; its established first-chunk selection remains local. `vehicle.rs` retains the seat-enum mapping and coordinate conversion, not a second M2 reader.

Raw format metadata preserves distinctions needed by existing consumers: authored skin indexStart high bits versus the client's cumulative start; all MAID slots versus MAIN-active slots; file-axis terrain origins/heights/holes without render-only companion payloads; authored M2 face normals; and static attachment positions without activation-track evaluation.

No assets are downloaded, copied or committed. No decompiled or leaked client source was inspected. Runtime state and deployment are outside this extraction.

## Reference proof

Reference captured with the pre-migration readers at rewritten server revision `ac36572bb`: existing LOS tests **16 passed / 0 failed / 4 ignored**. Stockade map34 reference `nav-bake` produced **4 LOS + 4 navigation tiles**, **7,196,988 bytes**; navigation collection contained39,791 triangles. SHA-256 inventory and an input superset of104 local asset files are recorded under `/home/osso/.worktrees/logs/wow_formats-phase2-*`. The after-reader gate must preserve those bytes and existing test outcomes; timing-bearing `report.txt` is not a binary-bake contract.

## Sources

- [LOS dependencies/features](../../../crates/los/Cargo.toml)
- [Vehicle dependencies](../../../crates/server/Cargo.toml)
- [Bake adapters](../../../crates/los/src/bake/mod.rs)
- [Shared format crate README](../../../../wow-formats/README.md)

## See Also

- [Navigation](navigation.md) — existing geometry selection and bake/runtime contracts
