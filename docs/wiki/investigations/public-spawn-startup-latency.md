# Public creature spawn startup latency

Verified 2026-10-20 (captures 2026-10-21 UTC). On the full unfiltered private world DB, parallel ground-file validation and lazy per-tile WMO collision remove generic work from the public-creature readiness path. This is a local release comparison, not a reproduction or resolution of Azure's reported 44-second post-bind delay.

## Boundary and provenance

Private loopback UDP5761, `agent-run server_perf`, fresh redb per run, private SQLite backup; no clients, shared runtime changes, ptrace, or protocol changes. World SHA256 `1e5d35f7f64a534d11545e3ef2e89a08b0b679ce2ebfb7e52bee3adf1f6059d4`; protocol `03cebdb971894aa1fa69b2e4ba7e7fae2702d515`. Capacity admission precedes every build/run.

Both comparison artifacts are container-built optimized release with the same local-only `debug=1, strip=false` override for symbolized `perf`. Before: instrumentation-only server `70fabab94`, binary SHA256 `d8db8e745d980559f35607874ae852462b1a98dfc0a8de697dda38ca0f1c2539`. After: `dd958b92b`, SHA256 `add32c817656cb2e9e464349d48fe0e989341102895e3bf47af5f11b328e2eb5`. Both share baseline `3e1add814`; later master integration does not change the measured ground/zones files.

Process launch is sampled immediately before `Popen`, after artifact/input hashing. UDP bind is observed in `/proc/net/udp` and corroborated by Lightyear's socket-bound log; earlier `Server listening` is only the queued Start request. The new ready marker runs after all queued creature commands apply, not at the last per-map preparation log. It is **creature population readiness**, not first Tick/admin/gameobject/LOS readiness.

## Measurements

Seconds; primary pair ran before→after, cross-check reversed order. No OS cache flush. Host/cache noise is retained, not removed by selecting only faster runs.

| Phase | Primary before | Primary after | Reverse before | Reverse after |
|---|---:|---:|---:|---:|
| Process → actual UDP bind | 19.055 | 19.884 | 26.029 | 18.709 |
| Ground validation (233 spawned maps) | 4.003 | 1.223 | 5.738 | 1.732 |
| Other post-bind work before preparation | 0.036 | 0.040 | 0.073 | 0.039 |
| Prepare 46 hosted public maps | 2.650 | 1.089 | 4.957 | 1.067 |
| Apply queued creature commands | 0.962 | 1.021 | 2.054 | 1.480 |
| UDP → all public creature commands applied | **7.652** | **3.373** | **12.823** | **4.318** |
| Process → public creature commands applied | 26.707 | 23.257 | 38.853 | 23.027 |

Primary post-bind reduction: 4.279s / 56%. Process-to-bind is deliberately unchanged by this fix and did not improve in that pair. Reverse-before encountered materially higher ambient CPU/I/O pressure; do not interpret its larger difference as an isolated code speedup. Earlier exploratory runs ranged from 8.49 to 11.45s bind→last-map-log; those omit deferred application and are not mixed into the table.

Primary pre-bind log intervals (before/after): initial scripts + WorldData 2.099/1.976s; remaining world/reference catalogs through queued listener 4.515/4.441s; toys/pets/quests/conditions 2.696/2.625s; spell metadata 2.814/3.053s; new-character kits 3.580/4.439s; item templates 0.639/1.055s; follower/persistence startup and actual bind 2.712/2.296s. These are sequential observed log intervals, not individually instrumented function-exclusive costs.

## Profile and fix

`perf record -e cycles:u -F 99 --call-graph dwarf,8192` covers startup; reverse runs use `--clockid mono` to align exact phase slices with driver monotonic timestamps. Earlier default-clock slices are approximate and labelled accordingly. No lost samples in retained recordings.

Ground validation was the dominant **post-bind** phase. It serially walked 233 maps, reading WDT/ADT `_obj0` and WMO-root metadata and checking group-file existence. Its reverse-before slice contains 141 user-cycle samples over 5.64s, with `FileCheck::tile`, `read`, `open64`, MODF parsing and allocation in the retained report; CPU-only samples do not measure blocked I/O time. Four scoped read-only workers now join before any public spawning. Sorted chunk/join order preserves identical refusal/known-gap diagnostics. Bevy remains single-threaded; no per-map ECS concurrency was introduced.

Spawn preparation also used `GroundMap::area_at` for regional level tuning, which loaded collision for 773 tiles solely to read terrain areas in the primary baseline. Per-tile WMO collision is now loaded synchronously only when a collision query needs it, before returning the same floor result; terrain area/liquid/MFBO queries retain their existing data. Global-WMO open policy, population, RNG/spawn order, levels, first-fixed-step work and navigation policy remain unchanged. First cold collision queries can still block; this is not asynchronous terrain publication or a loaded-server 50ms guarantee.

Pre-bind user CPU remains SQLite-heavy (`sqlite3VdbeExec`, mutex/row decoding in the symbolized whole/pre-bind profiles). No exact slow SQL statement or N+1 query was established; no SQL/gameplay change was made speculatively.

## Behavioral and runtime proof

The real FIFO map test is RED when one map's read prevents another's read from starting, then GREEN with scoped workers and exact sorted parse refusals. The real auction-house area test is RED while `_obj0` collision I/O blocks an area-only request; GREEN returns authored area5148 without collision bytes, then restores authentic collision and verifies identical first/cached floors. Fixture corrections: retained floor tolerance0.05 and private LOS link preserve the real M2 floor; neither changes production behavior. Initial Bevy-task attempt stayed serial because this server intentionally lacks its multi-threaded feature; it was replaced, not enabled globally.

Final merged-source focused gate: **30 PASS / 0 FAIL**, covering both FIFO boundaries, ordinary/global-WMO floors, gravity/falls, MFBO metadata/refusal order, and regional-level tuning/startup ordering. Native server check and workspace fmt check pass without warnings. No broad suite was run.

Every measured run produced the same per-map counts: **187687 admitted creatures across46 public maps**; the full DB has233456 rows including non-public instance populations. Existing83 explicit level-context refusals and1519 model-combat-reach diagnostics are unchanged, not fixed or silently omitted. Both primary steady idle windows and reverse steady windows report approximately20.0 ticks/s; primary after steady work mean2.2–2.3ms, max3.8–5.6ms. First stepping windows still have >50ms outliers (primary after333.7ms). This does **not** establish sakuin's20tps under player/combat load.

## Evidence

Durable root: `/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/spawn-latency-2026-10-20/`.

- `measure_startup.py`, `paired-{before,after}.jsonl`, `reverse-{before,after}.jsonl`, matching receipts and `*.perf.data`/phase reports: process, bind, phases, complete map counts, ambient pressure and frame windows.
- `red.log`, `red-area-corrected.log`, `green-scoped.log`, `final-targeted.log`, `final-check.log`, `final-fmt.log`: revision-scoped behavioral/build proof; final code gate at integration `8dfaf314260d6a43c6b38492ea90f646097c3e55`.
- Release build logs and frozen binaries: reproducible input/artifact hashes; no canonical `data/` asset changes committed.

## Sources

- [Ground implementation](../../../crates/server/src/ground.rs)
- [Spawn/application marker](../../../crates/server/src/zones.rs)
- [Filesystem behavior tests](../../../crates/server/src/ground/startup_latency_tests.rs)
- Original Azure finding: engine `data/diagnostics/follower-kits-2026-10-18/azure-follower205-integration-8ea8af5a3/findings.md`.

## See Also

- [[zone-architecture]] — public-map startup and lazy collision policy
- [[tick-model]] — simulation cadence is separate from startup readiness
