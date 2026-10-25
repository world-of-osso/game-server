# Server crate boundaries

Verified: 2026-10-24. Stable catalog code has one implementation in `server-catalog`; gameplay modules and their tests now belong to the existing `server` library, with a thin executable.

## Spell data foundation (2026-10-25)

`server-core` owns canonical spell records, `SpellStore`, DB2/server-side loaders, trait records, positivity, `CatalogData`, and the immutable fixture cache. `server::spell_info` re-exports these exact types and retains the native immunity and server-side spell integration tests. Gameplay, aura execution and cast dispatch remain in `server`; there is no second catalog or cache. Both test-support features select the same copy-on-write representation. Data-only tests keep their module names in `server-core`, with the existing opt2/debug1 test profile.

This is a dependency foundation, not a completed hot-gameplay split. Baseline LLVM output associates 174,925 IR lines with spell data (including generics); that attribution is not a compile-time saving estimate.

## Current ownership

- `crates/server-catalog`: generated character/spell/XP tables, six generated achievement lookups and their immutable records, pure faction-template reactions, and DB2 source-build validation/reading. Its only dependency is the existing rusqlite version/connection type; no Bevy plugin or runtime initialization.
- `crates/server/src/lib.rs`: existing standalone modules plus the former binary's gameplay module declarations and App assembly. `configure_app(&mut App)` preserves runtime→gameplay→resource→system registration order. One `extern crate self as server` keeps existing standalone-library imports canonical without editing hot modules.
- `crates/server/src/main.rs`: creates App, calls `server::configure_app`, runs it. No gameplay source files move for this consolidation. `item_instance::enchants` imports the canonical catalog spell table instead of compiling a second source copy.
- `crates/server-simulation`: test-only consumer of the server library; owns `dungeon_simulator` and nested DPS parity, with the original module prefix retained. Its normal library is empty; dependencies enabling `server/test-support` are dev-only.
- `server::test_support`: canonical frozen graph/Space, healer-intake receipts/Capture, and LiteralParityFixture marker used by both gameplay observation hooks and simulator. Existing shared protocol, LOS and navigation crates retain their types and responsibilities.

Catalog extraction changes visibility, not values or algorithms: 24 `pub(crate)` declarations become public. Three generators now write to `crates/server-catalog/src/` and emit public constants. Generated numeric tables and faction/provenance logic otherwise remain unchanged. `level_scaling` remains in server because combat/cast tests consume its private fixture helpers.

Test opt2/debug1 overrides apply explicitly to server, server-catalog, server-simulation and server-combat-tests. No tests move in the catalog-only commit. Library consolidation moves the former `game-server` test target's ownership to `server` lib, retaining existing library tests too; test functions/module paths remain unchanged. The thin bin has zero tests. Select `--lib` / nextest `binary(server)`, not the obsolete `--bin game-server` test filter. This small leaf cut is a crate-wiring proof, not a claimed material parity-iteration speedup.

## Test workflow and pending proof

```text
python3 scripts/build-host.py --test -p server --lib --no-run
python3 scripts/build-host.py --test -p server-simulation --lib --no-run
python3 scripts/build-host.py --test -p server-simulation --lib dungeon_simulator
```

`scripts/dps_parity_native.py` accepts this checkout's executable `target/.../server_simulation-*` test artifacts; the real bubblewrap/source-view fixture exercises this artifact-selection boundary through the CLI. The batch runner consumes the Cargo-emitted executable path; its seed/test names remain unchanged.

A alone does **not** isolate parity edits from the large library test target. B moves those edits to the simulator crate; per-area public-API test crates remain the next step. No full-suite or paired rebuild-speed claim is made here: the exact unsplit primer safety-aborted below6GiB host availability; the interim a28 opt2/-j2 comment-edit receipt (114.96s wall/69CPU-s) is a different source revision, not paired attribution. Full before/after test-census reconciliation and parent full-suite acceptance remain required.

## Fixture feature and test ownership

The explicit `server/test-support` feature preserves the original unit-test RNG scopes, immutable catalog/cache representation, test-only Clone implementations and observation hooks when server is a normal dependency of a test harness. Its optional dependencies are existing blake3, tempfile and pinned game-engine-core. Ordinary server/workspace builds do not enable this feature through the simulator package's normal dependency graph. Incoming native factor capture also needs this boundary at every observation call and armor-observation field/initializer, not just its public resource constructor/export: enabling the module alone compiles a capture that observes nothing. Unit-only test modules remain `cfg(test)`. No alternate gameplay implementation or duplicate component/resource definitions exist.

Source relocation keeps algorithms and numeric data unchanged: 46 tracked source/assets relocate after incorporating the landed haste and periodic-crit tests; four graph/receipt files belong to core support; the ignored corridor-input capture remains consumer-owned because it builds the simulator runtime. `../../data`, `../../content` and `../../target` paths retain their workspace location because both packages have the same manifest depth. Graph extraction-only modules/helpers and unrelated unit-only probes remain `cfg(test)` rather than pulling unit-test modules into a normal fixture-support dependency. Calendar/presence counters retain fixture-mode imports; their unit-test synchronization locks remain test-only.

Most case names retain `dungeon_simulator::...`. Six graph cases instead map to `test_support::graph::...` in server lib: four enabled route/graph cases and two ignored extraction cases. The ignored `dungeon_simulator::graph::corridor_inputs::freeze_native_corridor_inputs` case keeps its original name in the consumer; a graph re-export shares canonical types without reversing the runtime dependency. [Exact expected case/target mapping](../../handoffs/crate-split-test-mapping.json) records the exceptions and unchanged-name consumer move. Function bodies and ignore reasons are retained. The landed A suite at `8e7c7a685` supplies 123 exact subtree names: 117 consumer cases (30 enabled, 87 ignored) and six retained graph cases. B at `22aa67892` passes the simulator test-target compiler check and the ordinary `game-server` binary check without warnings. Incoming master `eec5b6bcf` adds four ignored single-spell cases to the consumer, making121 consumer cases (30 enabled/91 ignored) without changing original case identities. `14c74e6fa` also passes combined retained-core unit and simulator compilation without diagnostics. Actual post-split test inventories/full-suite reconciliation remain acceptance gates; compiler checks do not substitute for them.

## Next test-area pilot

`crates/server-combat-tests` follows the simulator's empty normal-library/dev-dependency pattern and owns 106 public-behavior melee cases. Canonical `collect_swings`/`collect_broadcast_swings` now reside in existing `spell_cast::test_harness`, shared with retained core tests; private outcome-formula tests stay in core. Fixture cache payloads remain private except the existing DB path. No new dependency versions or duplicate gameplay/resource types are introduced. [C handoff and measurement controls](../../handoffs/crate-split-area-tests.md), [exact case mapping](../../handoffs/crate-split-combat-test-mapping.json). At C code tip `acecb97d1`, the combined core-unit/simulator/combat-target compiler check and the production-no-feature `game-server` check pass without diagnostics. Full-suite/exact post-split census acceptance is pending.

## Generated achievement catalog cut (D1)

Six generated files (37,819 lines before relocation) now live under `server-catalog/src/achievement/`: conditions, mythic predicates, node counts, overlays, single modifiers and traditional-rank factions. `CompiledConditions`, `DifficultyRule` and `MythicRequirements` have one owner there; no ECS/resource dependencies move into catalog. Achievement credit, `MythicCompletion`, player/target authority and matching stay in server. The two matching methods become core free functions because inherent implementations cannot remain on externally owned records; predicates are unchanged.

All six generators still require an explicit output argument; point it at the corresponding new catalog file. Generated visibility now permits the core consumer. No lookup values, test bodies, names, ignored flags or should-panic flags change. The pure move commit is separate from wiring. Paired elapsed-time and full-suite acceptance remain separate proof; source-size reduction is not a speedup claim.

## Spell fixture qualification (D2)

`server-spell-tests` is an empty normal library with dev-only fixture dependencies and the existing opt2/debug1 test profile. Its first slice contains 29 spell-cast source files and 175 enabled cases; the exact `spell_cast::...` names and flags are preserved ([mapping](../../handoffs/crate-split-spell-test-mapping.json)). All 175 externally compile through canonical server at fixture tip `af8f7f5b6` before any source move, with no diagnostics. The 29 byte-identical moves form separate commit `e0aaa1840` before consumer wiring. Their their setup reads use `WorldData::spell_store()` to borrow the existing canonical cached `SpellStore`; cache fields remain private and no second resource/cache is introduced. Only existing `varied`, native-enemy-dodge harness and native-creature-world fixture helpers are public for this slice. The dodge helper's existing constant/apply dependencies use the same explicit test-support gate. Ordinary production does not compile the fixture harness.

External qualification exposed three additional `dummy7` charge-state reads after the cache/helper blockers were removed. The separately approved read-only `test_harness::charge_recovery` observer borrows canonical `SpellHistory` and returns consumed count plus oldest pending recharge; charge fields stay private. It retains the original missing-category failure boundary and checks the exact oldest recharge, not a weaker wire notification proxy.

The retained private `hit::tests` also used the NPC-dodge test module's setup helper. `spawn_native_dodge_target` now has one unchanged implementation in canonical `test_harness`; both retained formula tests and consumer cases use it. Its fixture spell IDs/imports move with the helper. No reverse server→consumer dependency exists.

No `execute_cast` seam or gameplay-internal visibility widening is included. Final wired-consumer/core compilation, actual post-build test inventories, runtime qualification and matched comment-rebuild measurements remain required before acceptance. Source size and check elapsed time do not establish edit-build payoff.

## Public spell remainder (D3)

D3 appends11byte-identical public spell test files to the existing consumer, retaining54case identities (51enabled/3existing ignored/0should-panic). The three proc-recipient ignore reasons remain unchanged. No test body, fixture seam, new package/version, profile or gameplay API changes. The existing rusqlite0.35/bundled workspace dependency is wired dev-only for the unchanged summon-property fixture; proc-recipient fixtures import canonical server::random at the consumer root. Pure moves `007c3d144` precede declaration-only wiring; [exact mapping](../../handoffs/crate-split-spell-d3-test-mapping.json) records core→consumer ownership. Consumer inventory becomes229cases (226enabled/3ignored). This1,543-line cut is housekeeping, not a claimed material whole-core speedup. Exact post-build inventory, one combined core/consumer compiler check, parent full gate and paired measurement remain acceptance proof.

## Independent domain crates

`server-pet-battle` owns the deterministic catalog/rules and235unchanged tests (230enabled/five existing ignored). Server imports its types directly; no server dependency or compatibility implementation remains.

`server-map-rules` owns map identity/difficulty/clock records, metadata/static-area catalogs, locks, admission/reset rules and graveyard choice. [32 unchanged tests](../../handoffs/crate-split-map-test-mapping.md) move with them. Core retains native transfer/PvP/housing/script integration tests and the original catalog-cache decorator. Fixture helpers have one feature-gated lower owner.

`server-follower-policy` owns scalar pull-budget inputs/decisions and [five unchanged tests](../../handoffs/crate-split-follower-policy-test-mapping.md), with no dependencies. Native measurement, readiness, AI and spell execution remain in server. The existing test-only forecast observer is shared through a dev-only `test-support` feature.

## Extraction candidates

Completed prerequisites: map rules and follower budget policy. Next: shared cycle-breaking identity/on-use types, then larger domains. The initial53d0e1ce1 graph found pet235, APL49, item-instance20, race/class11 and level-scaling10tests without gameplay imports; counts alone do not determine priority.

[Measured heavy-module walls](../investigations/server-test-performance.md) locate the larger boundaries:

| Domain | Earlier wall | Blocking cycle / next cut |
|---|---:|---|
| Scripted AI |517s|Damage/cast hooks↔scripts; lower hook records and a host interface. Native encounter tests still need real spells/combat.|
| SpellCast |499s|Aura↔cast↔damage/proc execution; lower immutable records, not the native assertions.|
| Toys |280s|Box/catalog↔spell classification/cast; lower on-use descriptors and keep a native adapter.|
| Followers |205s|AI↔spells, owner identity and pull-history callbacks; lower scalar budget/identity, then observation boundaries.|
| Maps |95s|Identity/rules now lower; transfer↔combat/cast/PvP/housing orchestration remains core-owned.|

These are historical module walls, not additive gate savings. Heavy toy/follower/encounter tests execute real spells and auras; traits alone do not make that coverage safe to skip. Shared types must have one lower owner, never a re-export of the whole core.


## Sources

- `crates/server-catalog/{Cargo.toml,src/lib.rs}`, `crates/server-simulation/{Cargo.toml,src/lib.rs}` — package/dependency ownership.
- `crates/server/src/test_support/` and core observation call sites — shared fixture type identity and graph/receipt behavior.
- `Cargo.toml`, `crates/server/Cargo.toml`, `crates/server/src/{main,lib}.rs`, `crates/server/src/item_instance/enchants.rs` — profile and consumer wiring.
- `scripts/gen_character_stat_tables.py`, `scripts/gen_spell_scaling_table.py`, `scripts/gen_xp_tables.py` — reproducible generated-table destinations and visibility.

## See Also

- [[dungeon-simulator]] — headless gameplay qualification.
- [[server-test-performance]] — measured profile/runtime tradeoffs; extraction alone is not a benchmark result.
- [[server-crate-split-measurements]] — completed B/C paired batches and comment rebuilds, sealed inputs, shared-host no-causal-claim limits.
