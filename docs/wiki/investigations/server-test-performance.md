# Server test performance

Target (2026-10-22): the union of the lead's `fu913`, `sm894` and `rd912` server gate filters must finish in at most900s execution wall with approximately8GiB peak RSS. Test cases, existing ignores, gameplay steps and assertions are preserved. Compilation and capacity-queue time are reported separately. **Acceptance remains open until the complete union has a passing measured receipt.**

## Baseline and cause

The reported debug gate took40–64minutes and reached approximately7.9GB RSS plus7.9GB swap. Historical gate logs are `/home/osso/.worktrees/logs/gate-{fu913,sm909,ct911,rd912}-srv.log`; exact filter sources are `lead-gate-{fu913,sm894,rd912}.sh` in that directory.

The default dependency override already optimizes external Bevy and the workspace LOS/navigation crates at level2. It does not optimize the server test crate. Generic Bevy/ECS, iterator and collection operations instantiated in that crate still execute unoptimized code. A25s49Hz CPU sample of the copied fu913 executable's full Stockade test contains1,261 samples and shows iterator steps, sparse-set lookup, `UnsafeWorldCell::get_entity`, pointer alignment/UB-precondition helpers and the server's `SpellStore::get` closure among the hot symbols. This is a compiled-code cost, not a real-time simulator clock: the simulator already advances manual50ms ticks.

`[profile.test.package.server]` now selects `opt-level=2` and limited `debug=1` information. Ordinary dev/release settings are unchanged. The test profile retains its inherited debug assertions and overflow checks; no test or gameplay policy changes. This also optimizes generic Bevy code instantiated by server tests, which a dependency-only override cannot cover.

Existing [native fixture caches](slow-native-test-fixtures.md) already reuse parsed canonical worlds/spells and prepared encounter definitions. Simulator batches move immutable catalogs between fresh worlds. Additional broad caches are not introduced by this profile change: prior receipts show retained-memory costs, and prepared-copy inputs must never be replaced by canonical definitions.

## Receipt ledger

Evidence directory: `game-engine/data/diagnostics/server-test-perf-2026-10-22/` in the canonical engine checkout. Admission uses `CAPACITY_ID=server-test-perf`; baseline nextest executes serially. `/usr/bin/time` is absent, so a fresh Python timing process records wall, child user/system CPU and `getrusage(RUSAGE_CHILDREN).ru_maxrss` without including compilation.

The lead authorized copying (not hardlinking) `game-server-verify2/target/debug/deps/game_server-71c0ac62686babf4` into the private checkout. SHA256: `299c2212aa8b255da715b61031df14ea7051f18dfebec040a4e39cb3863455a4`. The recorded fu913 gate head is `fd7e76756`; the selected simulator/Nethekurse sources have no diff to the checkout base `40812331c`. This is a historical executable, not an assertion that every linked input equals the new build. New compilation pins protocol `2abc7fa844f6e71d93203ce5421ae853e884854d` in an independent detached checkout.

| Test / scope | Before | After | Status |
|---|---:|---:|---|
| Campaign authored route order, cold process |29.101s|pending|baseline PASS|
| Full native Stockade clear |pending|pending|baseline running|
| Nethekurse native aggro, Normal + Heroic |pending|pending|baseline queued|
| Full gate-filter union |reported40–64min|pending|not accepted|

The copied executable's machine-readable nextest census selects2748 enabled tests in the game-server binary. Its total census contains5397 cases, including mismatches and pre-existing ignored cases. `baseline-union-census.json` and `union-filter.txt` retain exact selection semantics: `(fu913 OR sm894) minus their existing opening-admission skip`, union `rd912`. Other workspace binaries still require their own census and acceptance;2748 is not presented as the entire workspace count.

During the baseline Stockade test, `/proc` reported peakRSS626396KiB but currentRSS95996KiB plus520608KiB swap. Shared-host reclaim therefore confounds wall comparisons; RSS alone does not describe its resident-plus-swapped footprint. A profile speedup must be measured, not inferred from the sample.

## Gate invocation and remaining slow cases

Pending measured execution and memory comparison. Keep the existing six-thread libtest invocation available: its process-wide fixtures are shared, unlike nextest's per-test processes. Nextest supplies isolated per-case timing here; process isolation is not assumed faster for thousands of cold-data cases. No timeout, retry or ignored-test policy is changed to manufacture a passing budget.

The reported audit identifies campaign12, Nethekurse6, Omor3, and Azure combat/Kokia/Vyletongue2 each above60s. These are historical counts, not post-change measurements. A final receipt must enumerate the actual remaining slow names, durations and sampled causes, and compare test identities/statuses before accepting the budget.

## Sources

- [Cargo test/dependency profiles](../../../Cargo.toml).
- [Simulator manual clock and loaders](../../../crates/server/src/dungeon_simulator/runtime.rs).
- [Simulator catalog ownership](../../../crates/server/src/dungeon_simulator/catalogs.rs).
- [Canonical fixture ownership](../../../crates/server/src/test_data.rs).
- Evidence: `baseline-binary.json`, `baseline-union-census.json`, `baseline-copied.jsonl`, `baseline-full-stockade.perf` and `baseline-full-stockade-hotspots.txt` in the directory above.

## See Also

- [Slow native test fixtures](slow-native-test-fixtures.md) — existing caches, isolation proofs and retained-memory tradeoffs.
- [Dungeon simulator](../systems/dungeon-simulator.md) — unchanged native steps, seed qualification and manual clocks.
