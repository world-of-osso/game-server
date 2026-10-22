# Server test performance

Target (2026-10-22): the union of the lead's `fu913`, `sm894` and `rd912` server gate filters must finish in at most900s execution wall with approximately8GiB peak RSS. Test cases, existing ignores, gameplay steps and assertions are preserved. Compilation and capacity-queue time are reported separately. **Acceptance remains open until the complete union has a passing measured receipt.**

Controlled same-source route initialization is **3.045× faster in CPU and4.046× in wall time** with the server override; all5437 discovered case identities and ignore statuses are unchanged. Full native Stockade passes in73.885s. Profile-only full union **does not meet the target**:2277.819s combined wall (37m58s),6347.253s CPU,9.67GiB child peakRSS and15.25GiB sampled peak swap. All2815 enabled/81 ignored cases are accounted for; four failures are named below. Workspace check and format pass. Test-only immutable-cache/Arc work follows; the profile change alone is not completion.

## Baseline and cause

The reported debug gate took40–64minutes and reached approximately7.9GB RSS plus7.9GB swap. Historical gate logs are `/home/osso/.worktrees/logs/gate-{fu913,sm909,ct911,rd912}-srv.log`; exact filter sources are `lead-gate-{fu913,sm894,rd912}.sh` in that directory.

The default dependency override already optimizes external Bevy and the workspace LOS/navigation crates at level2. It does not optimize the server test crate. Generic Bevy/ECS, iterator and collection operations instantiated in that crate still execute unoptimized code. A25s49Hz CPU sample of the copied fu913 executable's full Stockade test contains1,261 samples and shows iterator steps, sparse-set lookup, `UnsafeWorldCell::get_entity`, pointer alignment/UB-precondition helpers and the server's `SpellStore::get` closure among the hot symbols. This is a compiled-code cost, not a real-time simulator clock: the simulator already advances manual50ms ticks.

`[profile.test.package.server]` now selects `opt-level=2` and limited `debug=1` information. Ordinary dev/release settings are unchanged. The test profile retains its inherited debug assertions and overflow checks; no test or gameplay policy changes. This also optimizes generic Bevy code instantiated by server tests, which a dependency-only override cannot cover.

Existing [native fixture caches](slow-native-test-fixtures.md) already reuse parsed canonical worlds/spells and prepared encounter definitions. Simulator batches move immutable catalogs between fresh worlds. Additional broad caches are not introduced by this profile change: prior receipts show retained-memory costs, and prepared-copy inputs must never be replaced by canonical definitions.

## Receipt ledger

Evidence directory: `game-engine/data/diagnostics/server-test-perf-2026-10-22/` in the canonical engine checkout. Admission uses `CAPACITY_ID=server-test-perf`; baseline nextest executes serially. `/usr/bin/time` is absent, so a fresh Python timing process records wall, child user/system CPU and `getrusage(RUSAGE_CHILDREN).ru_maxrss` without including compilation.

The lead authorized copying (not hardlinking) `game-server-verify2/target/debug/deps/game_server-71c0ac62686babf4` into the private checkout. SHA256: `299c2212aa8b255da715b61031df14ea7051f18dfebec040a4e39cb3863455a4`. The recorded fu913 gate head is `fd7e76756`; the selected simulator/Nethekurse sources have no diff to the checkout base `40812331c`. This is a historical executable, not an assertion that every linked input equals the new build:49 underlying server source files differ despite matching selected fixture files. Historical comparisons below are not isolated opt-level effects; the separate same-source control is. New compilation pins protocol `2abc7fa844f6e71d93203ce5421ae853e884854d` in an independent detached checkout.

| Test / scope | Before | After | Status |
|---|---:|---:|---|
| Campaign authored route order, cold process |27.024s wall /20.50s sampled CPU /614624KiB HWM|8.789s /7.72s /587552KiB|both PASS; historical source confounded|
| Full native Stockade clear |aborted after919.72s sampled CPU|73.885s /73.16s /697692KiB|after PASS; no passing before receipt|
| Nethekurse native aggro, Normal + Heroic |48.535s wall /41.25s sampled CPU /1457520KiB HWM|15.620s /12.15s /1421772KiB|both PASS; historical source confounded|
| Full gate-filter union |reported40–64min|2277.819s combined /6347.253CPU /10135336KiB childRSS|2815 executed,4failures; budget NOT met|

The copied executable's machine-readable nextest census selects2748 enabled tests in the game-server binary. Its total census contains5397 cases, including mismatches and pre-existing ignored cases. `baseline-union-census.json` and `union-filter.txt` retain exact selection semantics: `(fu913 OR sm894) minus their existing opening-admission skip`, union `rd912`. Other workspace binaries still require their own census and acceptance;2748 is not presented as the entire workspace count.

During the baseline Stockade test, `/proc` reported peakRSS626396KiB but currentRSS95996KiB plus520608KiB swap. Shared-host reclaim therefore confounds wall comparisons; RSS alone does not describe its resident-plus-swapped footprint. A profile speedup must be measured, not inferred from the sample.

That first run was stopped after917.28s user+2.44s system CPU when its embedded `CARGO_MANIFEST_DIR` was found to point checkpoint/preparation output at the original checkout's target directory. The partial run is diagnostic evidence, not a passing before receipt. The corrected baseline uses a private `bwrap` mount mapping the original checkout path onto the private checkout, read-only host root, and explicit private `DUNGEON_SIMULATOR_OUTPUT`. A mount-identity/write probe confirmed its target output exists only in the private checkout. The bounded matching cohort is authored-route initialization plus Normal/Heroic Nethekurse aggro; the optimized full Stockade test remains additional absolute-budget coverage. Safe baseline **2/2PASS**,75.771s timer wall,62.376182s exact child CPU (56.933442user+5.442740system),1452960KiB child `ru_maxrss`; wrapper wall75.946s. Per-case `/proc` CPU is a sampled lower bound, and HWM may differ slightly from the exit-time resource metric. Maximum sampled swap is2760KiB for route setup and668KiB for Nethekurse.

Both safe sides use a private SQLite online backup created before the safe baseline executed:886865920bytes, SHA256`09dac63b0337dee505bd704ad7766abe2bc3cf6a71dfbf3acd84de5690ef12f0`, mode0444, under the private target. Other asset directories remain read-only source links. This freezes canonical imports without mutating source data. The current opt2 three-case run at550c4b453 passes3/3:100.394074s timer wall,94.934341s exact child CPU,1418396KiB `ru_maxrss`,zero sampled swap. A server-only warm rebuild takes8m19s with zero third-party compilation; an earlier cold attempt was stopped at the mandatory300GiB disk floor, not reported as a passing build. Cargo's overall `test` banner still says unoptimized; the actual server rustc argv records `opt-level=2`, `debuginfo=1`, `debug-assertions=on`.

### Same-source opt-level control

Both executables use550c4b453, protocol2abc7fa, the same09dac63b frozen database, `debug=1`, `CARGO_INCREMENTAL=0` and nextest process isolation. Only the cheap authored-route test executes, once per profile; no full Stockade opt0 rerun. CLI profile override selectsopt0 without editing source. Compiler artifact metadata retains debug assertions and overflow checks. Full discovery matches all5437 case names/kinds/ignore statuses between the binaries.

| Metric, identical one-case cohort | Opt0 | Opt2 |
|---|---:|---:|
| Test-body wall |31.085s|7.654s|
| Exact timer wall |31.133876s|7.695305s|
| Exact child user+system CPU |23.252883s|7.635607s|
| Child peak RSS |619324KiB|579512KiB|
| Sampled swap |0|0|

Both pass. These give the controlled3.045× CPU/4.046× wall result, not a claimed full-suite speedup. The opt2 snapshot SHA256 is `041973351cb9195b6c86683462a1081b67b6599cbc4cf2643487cc173b0c1263`. The opt0 build takes7m but also repairs already-pruned vendor/Lightyear dependencies, so its compilation duration is not an isolated optimization comparison. `controlled-profiles.log`, both `controlled-opt*.time`/`-processes.json`/`-census.json` files and the opt0 compiler artifact preserve raw proof.

## Repeated dependency compilation

Lead gates `az914b`, `sa920`, `ra922` and `to917` repeatedly begin with the same four roots: `libc`, `libm`, `libloading`, `libsqlite3-sys`, followed by their dependency cascade. Installed housekeeping had a deterministic artifact-grouping defect, not evidence that every third-party feature changed:

- `artifact_stem()` stripped leading `lib` from every filename. Thus `libc-HASH.d` became group `c-HASH`, whereas `liblibc-HASH.rlib` became `libc-HASH`.
- The first group's third-party guard looked for nonexistent `c-HASH.d`, so an aged real dep-info file was deleted. Depending on directory ordering, the library group was then deleted too, or survived until the fingerprint repair could no longer find its dep-info. Either route invalidated an otherwise reusable dependency and its transitive consumers.
- The lead deployed the correction in `/home/osso/.worktrees/logs/prune-target-deps.py`: strip the library prefix only for `.rlib`, `.rmeta` and `.so`, never `.d` or executable names. Already missing dependencies need one rebuilding pass; the fix prevents subsequent housekeeping invalidation.

`logs/test_prune_target_deps.py` exercises actual aged filesystem artifacts/fingerprints for all four `lib*` roots in both file-creation orders, retains vendored libraries, and still removes expired workspace libraries. The lead ran the same standalone unittest against a copy differing only by legacy prefix stripping: **8 failing subcases before,3 test methods OK after**. This is filesystem behavior proof, not a source-string or grouping-shape assertion. Installed housekeeping remains lead-owned; this branch does not replace it.

Alternative checks: `git diff 31ce2d6db..dd5702f88` is empty for `Cargo.lock` and root/server/navigation/client/integration-test manifests between the same petgate's az914b and sa920 heads. Both use `--workspace` without feature flags; they do not alternate `-p server --bin` with workspace selection. `agent-run` sets TMPDIR and scheduling priority, not RUSTFLAGS. Captured libc fingerprints have `rustflags=[]`, unchanged feature sets and profile/config keys; test/check variants coexist under distinct stable unit hashes. These observations do not support lock/feature/flag churn as the repeated four-root cause.

`gate-ft923-land.log` is different: its paths name `game-server-pygate` and it starts with proc-macro2/unicode-ident/cfg-if plus broad check-only artifacts. Prior warm state for that tree is unproven; do not label its entire cold-looking check as the four-root cascade. Ordinary first test/check builds and actual manifest/feature changes remain legitimate compilation.

Evidence: `pruner-artifact-classification.json`, `same-petgate-dependency-config.diff` (empty), `gate-libc-fingerprints.json`, the installed pruner and adjacent standalone unittest. Logs count108–112 total compile events including workspace/path crates, not108–112 registry crates.

## Integrated union input correction

Atad56be4e8 (merged masterdb57e5364), workspace discovery selects2815 enabled/81 already-ignored cases. The server artifact isopt2/debug1 with debug assertions and overflow checks. Build12m03s is recorded separately. The first union uses four shared-process libtest threads but a faulty frozen-input harness: mode0444 propagates into fixture `fs::copy` destinations that must be writable, and the flat snapshot directory breaks loaders resolving sibling `game_object_locks.json`/DB2 paths from the database realpath. Resulting fixture/cache-poison cascades invalidate the run; assertions and production code were not changed.

The process was stopped after1470.72s elapsed,3006.49user+904.70system CPU;1751 passes,216 failures and79 ignores had been observed shortly beforehand. Summed-tree peakRSS8368468KiB and peak swap11519200KiB are separate maxima. These are diagnostic partial observations, not a full-suite pass, not a valid budget measurement, and not evidence that the profile alone meets900s. Raw files are retained under `union-invalid-input/`.

Correction preserves the source directory structure in a private `data/server-world/world.db` view with canonical sibling assets, uses mode0644 so private fixture copies remain writable, and checks the source snapshot's SHA before/after rather than inheritable write protection. A fresh snapshot has SHA256`a2e815a2cbffc58366151808cf423ab8eedc2df8f9ef86324d470be341ee1fe5`. No `FOLLOWER_KIT_DB` override. The corrected preflight passes6/6 on the same compiled gameplay: population-copy9.709s, Tyrant native lifecycle85.881s, gathering sibling-input39.582s, isolated Holy Smite6.671s/Fists10.532s, and serial qualified frontal16.326s. The snapshot hash is unchanged before/after. Holy/Fists failures do not reproduce in isolation; they are not presented as established profile regressions. Corrected full union qualification remains pending.

The frontal case runs separately and serially; its exact16.358595s timer wall/11.053987s child CPU and1246000KiB peakRSS receipt is included in the union's combined wall/CPU/coverage instead of rerunning or dropping it. It uses lead-qualified `GAME_SERVER_GROUND_DIR=/home/osso/.worktrees/bot-harness-2026-10-15/azure-tank-fixture-assets` and `AZURE_TANK_BAKE_RECEIPT_SHA256=1780fed3e150cdcd3ba3ccc42da7b2305205044421652563308a1a8502569b8a`. The original private bake receipt's14 output hashes match its assets; canonical runtime tile hashes also match, but its two report.txt files differ, so those report files are not described as exact copies. No fake receipt or weakened assertion is used.

## Relative vendor roots and the four-crate Lightyear cascade

The separate13796 verbose log reports four registry consumers Dirty because `lightyear_replication` was rebuilt. The root itself actually compiles at log line332, without its own Dirty report. `lightyear`, interpolation, frame-interpolation and prediction are registry crates; `lightyear_replication` is the checkout-local patched vendor crate, not a git dependency.

Its real dep-info first input is **`vendor/lightyear_replication/src/lib.rs`**. The old guard searched for`/vendor/`, which misses a leading relative `vendor/`; a read-only call on13796's actual artifact returnedFalse. A matching aged-filesystem regression deletes its real-shaped `.d`/library/fingerprint group:1 failing method among4 before the second repair. The lead deployed a normalized path-component check; all4 methods now pass0.009s. Its installed SHA256 is `2cbee2f3a37cc515dce4615f224cd181450fa97403c101467ad9347edbc90518`.

Both13796's and this task's real first inputs for Recast, Replication and Replicon are relative `vendor/.../src/lib.rs`; all six artifacts now classify as retained. No Recast exception is needed. The Replication manifest explicitly sets `build=false`, vendor/protocol directories are not symlinks, and vendor Git is clean with its last code commit inAugust. The target is not pinned in `target-keep.txt`: global housekeeping can select it while idle. The exact historical deletion invocation is not recorded, but the misclassification, production-shaped deletion and downstream rebuild pattern are directly demonstrated. There is no evidence that a build-script rerun or moving Shared symlink caused this occurrence.

This task's subsequent same-source opt0 build also heals precisely the three vendor roots and four registry consumers after the idle period, independently showing why a supposedly warm compile can take minutes. Existing missing artifacts need that one healing build; the second fix prevents future age-only vendor eviction. Evidence: `df-lightyear-fingerprints.json`, `df-relative-vendor-prune-classification.json`, `relative-vendor-fixed-classification.json`, `relative-vendor-{red,green}.log`, the preserved pre-relative-fix pruner and13796's `dailcry-ongoing-native-vv-670739696.log`.

## Completed profile-only union

Code lineagead56be4e8 (masterdb57e5364); laterf2ed87fac changes documentation only. Protocol2abc7fa, corrected private snapshota2e815; no `FOLLOWER_KIT_DB`. Four shared-process libtest threads plus the separately qualified serial frontal case. Discovery and execution inventories agree exactly:2815 enabled cases (2811pass/4fail),81 existing ignored, zero missing/extra. Source snapshot hash remains unchanged. Workspace check passes4m23s and format check passes; no warnings reported. Workspace prebuild12m24s is separate from execution; it heals previously pruned workspace artifacts, while third-party/vendor dependencies remain reusable.

| Metric | Measured |
|---|---:|
| Batch execution wall |2261.460143s|
| Serial frontal wall |16.358595s|
| Combined union wall |2277.818739s|
| Combined exact child CPU |6347.252680s|
| Child peakRSS |10135336KiB (9.67GiB)|
| Conservative sampled tree peakRSS |10060028KiB (9.59GiB)|
| Sampled tree peak swap |15994032KiB (15.25GiB)|

The memory maxima occur at different samples; do not sum them into a claimed simultaneous peak. RSS-only acceptance is not enough when the process swaps this much. Neither900s nor the approximately8GiB/no-swap follow-up target is met. Per-module sums measure overlapping per-test **wall**, not CPU: scriptedAI503cases/3228.701s; spell_cast1181/2774.509s; followers454/1108.823s (plus the separate frontal16.326s); combat195/482.941s; maps256/395.418s. These sums overlap and do not add to elapsed wall. Per-module CPU cannot be recovered honestly from one shared-process total without additional profiling. Full module ledger is `union-module-wall.json`.

Four batch failures remain explicit: `followers::holy_priest_tests::holy_priest_native_two_surge_uses_keep_half_cost_then_spend_one` and `followers::kit_proc_fidelity_tests::dps_parity_fists_of_fury_simc_tick_cadence_preserves_five_native_damage_hits` pass individually but fail in both four-thread shared-process runs; shared-state/order cause is not yet identified. `game_objects::collision::tests::arena_live_imported_objects_require_unphased_source_until_real_phasing_exists` sees a missing hardcoded arena proof database; the lead restored it after this case executed, so an isolated recheck is required. A fourth late failure is `spell_cast::dummy8_defense_tests::dummy8_breath_burn_reduces_only_damage_to_the_owning_monk_and_expires`; it needs an isolated retained-case recheck before attribution. No assertion is removed or weakened, and no failure is silently retried into a green union.

A25s49Hz midrun sample (3989samples) attributes19.28%self CPU to SQLite VM execution and13.78% to `pthread_mutex_lock`; captured callchains connect the latter through rusqlite column readers to SQLite `columnMem`. The bundled SQLite source explicitly enters `pVm->db->mutex` there. This identifies per-connection SQLite lock-acquisition overhead, **not proven cross-thread lock blocking**, and not a shared registry lock. `SpellInfo::clone` contributes2.88%self. Repeated test-App decoding/row copies and retained full spell-store copies are the next scoped targets; production SQLite safety flags stay unchanged.

### Ten longest cases, current union

All ten pass. Durations are per-test wall under concurrency, not isolated CPU attribution.

| Exact test | Seconds | Subsystem exercised |
|---|---:|---|
| `spell_cast::dummy7_tests::dummy7_survival_instincts_halves_incoming_damage_and_cancellation_removes_child` |129.331|Native defensive aura, damage and cancellation lifecycle|
| `dungeon_simulator::full_stockade_clears_all_three_bosses` |82.841|Full native Stockade route/combat; complete timeline retained|
| `dungeon_simulator::healer_metrics_tests::healer_triage_benchmark_records_native_pull_end_mana_and_hot_uptime` |81.797|Native healer run and mana/HoT metrics|
| `scripted_ai::tests::data_bosses::kokia::boss_alerts_opted_lua_scripts_publish_real_initial_requests_only` |66.239|Multiple source-qualified Lua boss alert lanes|
| `spell_cast::readiness_lifetime_tests::native_readiness_real_sixty_second_owner_lifetime_survives_slow_frames` |62.111|Intentional60s REAL-time charge/readiness lifetime, not a simulator sleep bug|
| `spell_cast::summon_damage_tests::class_completeness_native_summon_damage_changes_real_guardian_melee` |53.301|Native guardian stats/melee damage|
| `scripted_ai::tests::data_bosses::lost_dwarves::encounter_coverage_lost_dwarves_all_actor_current_lanes_and_native_completions` |44.685|All three native boss actors and source clock lanes|
| `toys::tests::toyfx14_stationary_source_summons_learn_spawn_and_expire` |44.476|Native toy learning/summon/expiry|
| `scripted_ai::tests::data_bosses::herald_liquid_fire::encounter_coverage_herald_liquid_fire_heroic_fireball` |33.651|Heroic Herald source Fireball lane|
| `scripted_ai::tests::data_bosses::herald_liquid_fire::encounter_coverage_herald_liquid_fire_heroic_cone` |32.927|Heroic Herald source frontal lane|

Only the first five exceed60s. Apart from the intentional real-time readiness test and full scenario timelines, exact residual loader/clone versus simulation contributions need isolated measurements; no tick counts will be shortened based on wall time alone. The lead approved a bounded nextest RSS cohort (these ten plus cache-heavy/native-copy controls), explicitly **not a global largest-RSS census**. That ranking and a memory-weighted scheduling policy follow; process-wide cached RSS cannot be assigned to individual overlapping tests from this run.

## Gate invocation

Use the committed default `test` profile through ordinary `cargo test --locked --workspace --no-fail-fast -- <union filters> --skip opening_admission_does_not_recruit_unprojected_source_packs --test-threads 4`, with no kit override and a hash-frozen, writable-copy-compatible private source layout. The qualified frontal case must run separately/serially with its actual bake receipt; include its execution receipt in total coverage/time. Four threads are the **measured baseline, not an accepted final recommendation**: memory-bounded heavy-test scheduling still needs proof. Nextest supplies isolated timing/RSS here; cold per-test process isolation is not assumed faster for thousands of native fixtures. Exact full argv is preserved in `union.log`/`union-provenance.json`; filters remain the literal three-gate union. No timeout, retry, ignore or assertion policy changes to manufacture acceptance.

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
