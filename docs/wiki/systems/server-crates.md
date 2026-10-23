# Server crate boundaries

Verified: 2026-10-23. Stable catalog code has one implementation in `server-catalog`; gameplay and test-target consolidation follows separately.

## Current ownership

- `crates/server-catalog`: generated character/spell/XP tables, pure faction-template reactions, and DB2 source-build validation/reading. Its only dependency is the existing rusqlite version/connection type; no Bevy plugin or runtime initialization.
- `crates/server`: existing standalone library and runtime binary. The binary currently owns gameplay modules and reuses catalog modules through imports. `item_instance::enchants` imports the same spell-scaling table rather than compiling a second source copy.
- Existing shared protocol, LOS and navigation crates retain their types and responsibilities.

Catalog extraction changes visibility, not values or algorithms: 24 `pub(crate)` declarations become public. Three generators now write to `crates/server-catalog/src/` and emit public constants. Generated numeric tables and faction/provenance logic otherwise remain unchanged. `level_scaling` remains in server because combat/cast tests consume its private fixture helpers.

Test opt2/debug1 overrides apply explicitly to both server packages. No tests move in the catalog-only commit; existing server consumer tests remain authoritative. This small leaf cut is a crate-wiring proof, not a claimed material parity-iteration speedup.

## Sources

- `crates/server-catalog/{Cargo.toml,src/lib.rs}` — canonical module/dependency ownership.
- `Cargo.toml`, `crates/server/Cargo.toml`, `crates/server/src/main.rs`, `crates/server/src/item_instance/enchants.rs` — profile and consumer wiring.
- `scripts/gen_character_stat_tables.py`, `scripts/gen_spell_scaling_table.py`, `scripts/gen_xp_tables.py` — reproducible generated-table destinations and visibility.

## See Also

- [[dungeon-simulator]] — headless gameplay qualification.
- [[server-test-performance]] — measured profile/runtime tradeoffs; extraction alone is not a benchmark result.
