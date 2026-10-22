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
| Campaign authored route order, cold process |27.024s wall /20.50s sampled CPU /614624KiB HWM|pending|safe baseline PASS|
| Full native Stockade clear |aborted after919.72s sampled CPU|pending|no passing baseline receipt|
| Nethekurse native aggro, Normal + Heroic |48.535s wall /41.25s sampled CPU /1457520KiB HWM|pending|safe baseline PASS|
| Full gate-filter union |reported40–64min|pending|not accepted|

The copied executable's machine-readable nextest census selects2748 enabled tests in the game-server binary. Its total census contains5397 cases, including mismatches and pre-existing ignored cases. `baseline-union-census.json` and `union-filter.txt` retain exact selection semantics: `(fu913 OR sm894) minus their existing opening-admission skip`, union `rd912`. Other workspace binaries still require their own census and acceptance;2748 is not presented as the entire workspace count.

During the baseline Stockade test, `/proc` reported peakRSS626396KiB but currentRSS95996KiB plus520608KiB swap. Shared-host reclaim therefore confounds wall comparisons; RSS alone does not describe its resident-plus-swapped footprint. A profile speedup must be measured, not inferred from the sample.

That first run was stopped after917.28s user+2.44s system CPU when its embedded `CARGO_MANIFEST_DIR` was found to point checkpoint/preparation output at the original checkout's target directory. The partial run is diagnostic evidence, not a passing before receipt. The corrected baseline uses a private `bwrap` mount mapping the original checkout path onto the private checkout, read-only host root, and explicit private `DUNGEON_SIMULATOR_OUTPUT`. A mount-identity/write probe confirmed its target output exists only in the private checkout. The bounded matching cohort is authored-route initialization plus Normal/Heroic Nethekurse aggro; the optimized full Stockade test remains additional absolute-budget coverage. Safe baseline **2/2PASS**,75.771s timer wall,62.376182s exact child CPU (56.933442user+5.442740system),1452960KiB child `ru_maxrss`; wrapper wall75.946s. Per-case `/proc` CPU is a sampled lower bound, and HWM may differ slightly from the exit-time resource metric. Maximum sampled swap is2760KiB for route setup and668KiB for Nethekurse.

Both safe sides use a private SQLite online backup created before the safe baseline executed:886865920bytes, SHA256`09dac63b0337dee505bd704ad7766abe2bc3cf6a71dfbf3acd84de5690ef12f0`, mode0444, under the private target. Other asset directories remain read-only source links. This freezes canonical imports without mutating source data. The optimized build is admitted separately after the safe baseline finishes; initial compile remains pending.

## Repeated dependency compilation

Lead gates `az914b`, `sa920`, `ra922` and `to917` repeatedly begin with the same four roots: `libc`, `libm`, `libloading`, `libsqlite3-sys`, followed by their dependency cascade. Installed housekeeping had a deterministic artifact-grouping defect, not evidence that every third-party feature changed:

- `artifact_stem()` stripped leading `lib` from every filename. Thus `libc-HASH.d` became group `c-HASH`, whereas `liblibc-HASH.rlib` became `libc-HASH`.
- The first group's third-party guard looked for nonexistent `c-HASH.d`, so an aged real dep-info file was deleted. Depending on directory ordering, the library group was then deleted too, or survived until the fingerprint repair could no longer find its dep-info. Either route invalidated an otherwise reusable dependency and its transitive consumers.
- The lead deployed the correction in `/home/osso/.worktrees/logs/prune-target-deps.py`: strip the library prefix only for `.rlib`, `.rmeta` and `.so`, never `.d` or executable names. Already missing dependencies need one rebuilding pass; the fix prevents subsequent housekeeping invalidation.

`logs/test_prune_target_deps.py` exercises actual aged filesystem artifacts/fingerprints for all four `lib*` roots in both file-creation orders, retains vendored libraries, and still removes expired workspace libraries. The lead ran the same standalone unittest against a copy differing only by legacy prefix stripping: **8 failing subcases before,3 test methods OK after**. This is filesystem behavior proof, not a source-string or grouping-shape assertion. Installed housekeeping remains lead-owned; this branch does not replace it.

Alternative checks: `git diff 31ce2d6db..dd5702f88` is empty for `Cargo.lock` and root/server/navigation/client/integration-test manifests between the same petgate's az914b and sa920 heads. Both use `--workspace` without feature flags; they do not alternate `-p server --bin` with workspace selection. `agent-run` sets TMPDIR and scheduling priority, not RUSTFLAGS. Captured libc fingerprints have `rustflags=[]`, unchanged feature sets and profile/config keys; test/check variants coexist under distinct stable unit hashes. These observations do not support lock/feature/flag churn as the repeated four-root cause.

`gate-ft923-land.log` is different: its paths name `game-server-pygate` and it starts with proc-macro2/unicode-ident/cfg-if plus broad check-only artifacts. Prior warm state for that tree is unproven; do not label its entire cold-looking check as the four-root cascade. Ordinary first test/check builds and actual manifest/feature changes remain legitimate compilation.

Evidence: `pruner-artifact-classification.json`, `same-petgate-dependency-config.diff` (empty), `gate-libc-fingerprints.json`, the installed pruner and adjacent standalone unittest. Logs count108–112 total compile events including workspace/path crates, not108–112 registry crates.

## Gate invocation and remaining slow cases

Pending measured execution and memory comparison. Keep the existing six-thread libtest invocation available: its process-wide fixtures are shared, unlike nextest's per-test processes. Nextest supplies isolated per-case timing here; process isolation is not assumed faster for thousands of cold-data cases. No timeout, retry or ignored-test policy is changed to manufacture a passing budget.

The reported audit identifies campaign12, Nethekurse6, Omor3, and Azure combat/Kokia/Vyletongue2 each above60s. These are historical counts, not post-change measurements. A final receipt must enumerate the actual remaining slow names, durations and sampled causes, and compare test identities/statuses before accepting the budget.

## Sources

- [Cargo test/dependency profiles](../../../Cargo.toml).
- [Simulator manual clock and loaders](../../../crates/server/src/dungeon_simulator/runtime.rs).
- [Simulator catalog ownership](../../../crates/server/src/dungeon_simulator/catalogs.rs).
- [Canonical fixture ownership](../../../crates/server/src/test_data.rs).
- Evidence: `baseline-binary.json`, `baseline-union-census.json`, `baseline-safe.jsonl`, `baseline-safe.time`, `baseline-safe-processes.json`, `frozen-world.json`, `baseline-full-stockade.perf` and `baseline-full-stockade-hotspots.txt` in the directory above.
- Lead-owned housekeeping: `/home/osso/.worktrees/logs/{prune-target-deps.py,disk-housekeeping.sh,test_prune_target_deps.py}`; the four gate scripts and their recorded heads.

## See Also

- [Slow native test fixtures](slow-native-test-fixtures.md) — existing caches, isolation proofs and retained-memory tradeoffs.
- [Dungeon simulator](../systems/dungeon-simulator.md) — unchanged native steps, seed qualification and manual clocks.
