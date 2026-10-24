# Server crate-split measurements

Verified: 2026-10-24. B's 31-case simulator batch and C's 106-case melee batch pass on both sides, with exact inventories and unchanged frozen executable/source/input hashes. Comment-rebuild and native measurements below are **shared-host observations, not controlled causal speedup claims**. Concurrent load and reclaim differed; neither pair qualifies the whole-workspace 900s/~8GiB/no-swap goal.

## Measurement boundary

Both pairs retain opt2/debug1, debug assertions, overflow checks, incremental compilation and two Cargo jobs. A warm rebuild appends one comment to a byte-identical representative test source; primer/admission durations are excluded. No semantic-edit timing claim is made. Native batches use one fresh process/one test thread, the same sealed read-only inputs and nice10 agent-run wrapper. Existing ignored cases are explicitly excluded, not enabled or newly ignored. One owned paired heavy phase runs at a time; other parent-approved gates remain concurrent.

Before compilation, actual MemAvailable must be at least14GiB and capacity admission must return ADD. Builds abort below6GiB available or300GiB free; aborted attempts remain evidence, not successful measurements. Dependency diagnostics are archived separately from first-party diagnostics: 1,371 registry warnings were retained, zero project warnings. The original B-before primer aborted; the second succeeded but its initial reporting-only assertion rejected registry warnings. A pinned reporting-only runner resumed that completed primer without rebuilding it. No all-dependency-warning-free claim follows.

## B: simulator consumer

Source pair `90baf702e97baa7d3787a9d676dff1ebaa2c90e7` → `315627b57c10bb63452f0ff87faa31783ee19390`; the landed B merge is `d675bf8fb`. Its unrelated incoming toy changes are intentionally outside this measured pair. Representative edit: `dungeon_simulator/dps_parity/power_tests.rs`, core `server` lib → `server-simulation` lib.

| Metric | Before | After |
|---|---:|---:|
| Comment rebuild wall |148.921s|24.646s|
| Comment rebuild child CPU |98.823s|21.579s|
| Native31-case wall |324.257s|255.371s|
| Native31-case child CPU |279.593s|240.259s|
| Sampled native aggregate peak RSS |1,728,480KiB|1,504,692KiB|
| Sampled native peak swap |1,205,436KiB|707,972KiB|

Both native sides pass31/31 with no failures/ignored cases; the logical simulator domain's102 ignored cases remain excluded. Both comment rebuilds retain256/256 new-session object identities from prior incremental sessions (inode+size+mtime), a lower bound on object reuse, **not a query-reuse percentage**.

Native host CPU averages59.30%→56.91%; agent CPU17.10→17.08cores. Full-memory PSI avg10 peaks40.04%→34.03%, avg60 peaks17.33%→8.81%. Compiler-phase median loadavg differs53.90→36.77, with median full-memory PSI avg104.25%→2.02%. Observed wall/CPU reductions cannot be assigned solely to extraction. Both native sides swap; no no-swap qualification exists.

## C: melee behavior consumer

Source pair `d675bf8fbc1261cfcb629058fff59f4ddd0249a8` → `5bb9a992e316ced7c0f37386c7c0da47280e1d10`. Representative edit: `combat_npc_cadence_tests.rs`, core `server` lib → `server-combat-tests` lib. Twelve source files retain106enabled case names. Private-formula tests remain in core, using canonical shared fixture collectors rather than copied resources or gameplay.

| Metric | Before | After |
|---|---:|---:|
| Comment rebuild wall |105.801s|6.509s|
| Comment rebuild child CPU |58.426s|12.178s|
| Native106-case wall |154.205s|106.261s|
| Native106-case child CPU |145.184s|98.372s|
| Sampled native aggregate peak RSS |894,708KiB|866,600KiB|
| Sampled native peak swap |686,432KiB|0KiB|

Both native batches pass the exact106/106, with zero failed/ignored cases. Frozen executable/source/world/input hashes remain unchanged. Before/after comment rebuilds retain256/256 and108/108 new-session object identities respectively; these are lower-bound object-reuse receipts, not query-reuse percentages.

**Concurrent load differed materially: no causal runtime speedup claim.** Host CPU averages74.61%→43.76%; agent CPU22.61→13.45cores. Full-memory PSI avg10 peaks6.75%→3.63%, avg60 peaks3.99%→2.17%. Compiler-phase median loadavg24.40→28.32 and full-memory PSI avg101.00%→0.18% also differ. Observed native wall−31.09%/CPU−32.24% and comment wall105.80→6.51s describe these runs only. Zero sampled swap in C-after alone does not qualify the whole-workspace budget.

The RSS table reports sampled aggregate process RSS, not `ru_maxrss`: C child high-water RSS is864,988→838,644KiB. Swap is recorded in KiB;686,432KiB is approximately670.34MiB, not686MiB.

## Evidence and remaining scope

Evidence root: `/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/server-crate-split-2026-10-23/`.

- `paired-joined-ledger.json`, `paired-results.md`, `paired-runtime/{B-before,B-after,C-before,C-after}/`: exact selected cases, timer/resource results, concurrent-load telemetry and input/executable identity checks.
- `paired-B-before-attempt2/opt2-j2/`, `paired-B-after/opt2-j2/`, `paired-C-{before,after}/opt2-j2/`: Cargo argv/profile metadata, source restoration, warnings, timings and incremental object evidence.
- `{B,C}-paired-compile-comparison.json`: shared-host compile-load/PSI summaries; `{B-before,B-after,C-before,C-after}-seal-proof.json`: SHA256/profile/inventory proof.
- Pinned reporting runner SHA256`375abfd21ffb6ea0719a008ad27de402967f0f1f9608a9b07865ac06da5e8c23`; original runner and failed/aborted attempts retained separately.

All four native phases are complete; no rerun is required to restate these observations. Whole-workspace execution-budget proof, semantic edits and D2's spell-fixture runtime/matched-rebuild qualification remain separate open work. Full suite acceptance and exact ownership reconciliation are not replaced by these bounded batches.

## Sources

- [Server crate boundaries](../systems/server-crates.md) — canonical catalog/core/simulator/combat ownership and fixture support.
- [Combat extraction handoff](../../handoffs/crate-split-area-tests.md) and [exact case mapping](../../handoffs/crate-split-combat-test-mapping.json) — C's unchanged case identities and retained private boundaries.
- Diagnostic receipts named above — raw process, compiler, hash and case-inventory evidence.

## See Also

- [[server-test-performance]] — whole-workspace budget, distinct historical profile measurements and outstanding proof.
- [[dungeon-simulator]] — native simulator behavior and immutable fixture inputs.
