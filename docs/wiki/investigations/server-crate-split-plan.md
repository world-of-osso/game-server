# Server crate split plan

Verified: 2026-10-23. **Land one tiny catalog mechanics proof, then prioritize (A) thin bin/existing server lib, (B) separate simulator/parity crate, (C) area test crates.** This is a source-backed plan, not a measured crate-split speedup. Initial source analysis performed no gameplay changes or Cargo builds. An execution baseline was subsequently attempted under the expanded parent grant; its safety abort is recorded below.

## Evidence and reproducibility

Source: `origin/master` at `46c2bf608f81a5fa585bd63b32b2db6346e843da`. Analysis checkout was detached under `/var/tmp/osso` before creating `crate-split-plan`. [Full adjacency, SCC membership and per-module churn](server-crate-split-graph.md) are generated, not hand-selected examples.

```text
python3 scripts/server-crate-graph.py --root <checkout-at-46c2bf608> \
  --since 2026-10-09T20:51:09Z --until 2026-10-23T20:51:09Z \
  --output /var/tmp/osso/server-crate-graph.json \
  --markdown docs/wiki/investigations/server-crate-split-graph.md
python3 -m unittest discover -s scripts -p test_server_crate_graph.py
```

The interval is exactly 14 days ending at the source commit's committer timestamp. Churn uses `git log --no-merges --no-renames --since --until`, distinct commits per module across all history reachable from the pinned tip, not first-parent merge frequency. A commit touching five files in one module counts once. Current ownership follows `mod`/`#[path]` declarations, not filename prefixes; deleted/unreachable historical files are not assigned. Top-level `@main` is a synthetic entry-point node.

The script masks comments, normal/raw strings and chars, follows inline/external module declarations, and handles grouped `use crate::{...}`. Edges include **all explicit `crate::` imports/type/expression paths**, not imports alone; root entry-point `module::` calls are also recorded. JSON retains file/line witnesses. Two fixture tests cover masking, grouped paths, custom module ownership, cfg-test removal, cycles and deduplicated churn. No unresolved external modules remain at this revision.

Limit: lexical **potential** graph, not rustc expansion/name resolution. Non-test feature/platform cfg alternatives are unioned; only direct `#[cfg(test)]` subtrees are removed from the production view. `super::`/aliases, macro-expanded paths and external crates are not resolved. Thus this is the requested explicit-path dependency graph, not proof that an apparently empty node is extractable without checking its other names/dependencies. The test-inclusive view also unions `cfg(not(test))` alternatives. Inspect witnesses and real imports before each cut.

This snapshot has 166 declared modules, 1,413 file visits / 505,334 physical lines; 276,694 lines are in wholly test-only files. One 1,390-line fixture is reached twice; file-visit LOC is not unique disk LOC. Inline test bodies in production files are additional to the test-only-file total. This differs from the supplied older ~164/~493k/~267k census: source pin and module reachability, not removed tests. Simulator includes parity: 41 files / 10,889 lines; parity alone is 18 files / 4,060 lines. Simulator references 30 top-level modules; parity references 22. These counts do not include external crates or `super::` dependencies.

## Cycles determine the cut

Production potential graph: **1,410 edges, one 117-module SCC**. Test-inclusive graph: **1,963 edges, one 138-module SCC**. All remaining SCCs are singletons. Full member lists and every adjacency row are in the [generated appendix](server-crate-split-graph.md).

Concrete cycle witnesses:

| Edge | Source witness | Consequence |
|---|---|---|
| spell_cast → networking → spell_cast | `spell_cast/mod.rs:311`; `networking.rs:424` | Network-owned components/state are gameplay inputs, not just sockets. Moving the socket plugin alone does not make core independent. |
| followers → auras → followers | `followers/ai.rs:119`; `auras/mastery.rs:5` | Keep these together initially; no trait/service redesign required just to name a crate. |
| spell_info → catalog_data | Production spell-info graph | Small data-layer cut exists below the gameplay SCC. |
| spell_info → spell_cast/auras/test_data | `spell_info/*tests.rs` and inline tests | Catalog production is acyclic, but current catalog tests cannot accompany it unchanged as dependency-free unit tests. Move cross-system tests outward. |
| spell_cast → dungeon_simulator | `spell_cast/effects.rs:1351–1465`, test-only replay | Healer `Hit`/`HitKind`/`Capture` types currently live in the consumer. Put the shared receipt contract below both, or move replay adapter outward without changing numerical execution. |

`spell_info` is **not just DB2 records**: `SpellInfo` contains per-cast state, `Entity`, charged modifiers and crate-private fields. `catalog_data` aliases `T` in production but wraps `Arc<T>` under `cfg(test)`. A library dependency is not built with `cfg(test)` merely because its consumer is a test: preserve that fixture representation explicitly before moving catalog consumers. These are real boundary work, not blanket search/replace tasks.

## Churn: stable versus hot

Full per-module 14-day commit counts, LOC and `pub(crate)` occurrence counts are in the appendix. Selected boundaries:

| Module | Commits | File-visit LOC | Interpretation |
|---|---:|---:|---|
| character_stats_data / spell_scaling_data / xp_data / faction_reaction | 0 each | 129 / 130 / 150 / 115 | Stable, zero outgoing explicit crate edges; generated constants/pure rows. |
| db2_source_build / level_scaling | 2 / 6 | 27 / 621 | Small provenance loader / ExpectedStat math and tests, no explicit crate dependencies. |
| catalog_data / spell_info / world_data | 1 / 37 / 61 | 100 / 3,776 / 1,471 | Progressively less leaf-like; world_data is inside the gameplay SCC. |
| spell_cast / auras / followers | 1,246 / 445 / 666 | 69,792 / 22,877 / 36,222 | Concurrent hot work; do not combine their semantic edits with moves. |
| dungeon_simulator / scripted_ai | 192 / 875 | 10,889 / 55,404 | Simulator is smaller but not stable; scripted AI belongs with gameplay initially. |

Existing `server` **library already exists**, exporting character_data, item_instance, pet_battle, simc_apl and mythic/raid stores. Existing `shared` protocol and LOS/navigation crates remain dependencies, not duplicated into new crates. The `server` package contains both library and huge binary targets; adding `lib.rs` alone is not the split.

## Proposed layering and landable sequence

Arrows below mean dependency direction (consumer → dependency):

```text
game-server thin bin ─────────→ server (existing lib) ──→ server-catalog
server-simulation (tests) ────→ server (existing lib) ──→ shared / existing LOS+navigation
per-area API test crates ─────→ server (existing lib) / server-catalog
```

Keep one authoritative definition of every Bevy component/resource/message and system-set label. Do not re-declare equivalent types in different crates: Bevy type identity and scheduling dependencies must stay identical. Plugin registration and tick order stay unchanged; runtime transport setup remains in the executable. Initially the `server` lib still depends on Lightyear/shared through the current SCC; transport-free simulation means not installing socket plugins, not necessarily eliminating those dependency packages immediately.

| Cut | Moves and cycle breaking | Visibility / Bevy boundary | Estimated touched lines, excluding logic changes |
|---|---|---|---|
| **1. Catalog foundation** | Move only character_stats_data, spell_scaling_data, xp_data, faction_reaction, db2_source_build into `server-catalog`. No observed outgoing crate edges, no SCC to break. Update generators' output paths/visibility. Leave level_scaling untouched: external combat/cast tests consume its private test fixtures and parity consumes cfg(test)-only observation helpers. | 24 crate-private occurrences across these modules become public where used externally; existing public reaction types stay public. No Bevy dependency or new plugin. item_instance/enchants.rs must import the canonical catalog instead of its current #[path]-included spell table. Re-export module names in bin initially as canonical imports, not duplicate implementations. | 551 moved LOC, ~50–120 wiring/generator/manifest lines. No tests move; existing consumer tests/counts remain unchanged. |
| **A. Thin bin / existing server lib — immediate next priority** | Move all main.rs module declarations into the existing lib.rs, retaining its current exported modules. **Do not move the source tree** or first break the 117-module SCC. Move App registration helpers behind library bootstrap functions; main.rs only creates/builds/runs App. Preserve current unit tests in library initially. Audit existing `server::` paths from bin modules becoming same-crate imports; use canonical crate paths/self-alias only as required. | Internal pub(crate) remains crate-private. Expose only assembly/bootstrap boundaries now. SCC has 2,231 pub(crate) occurrences: inventory upper bound, **not** blanket public API. All Bevy identities and schedule order remain unchanged. | ~166 declarations plus ~180 App-assembly LOC relocate; ~300–600 affected declaration/bootstrap/import lines. No mass module-file renames. Reconcile old game-server test names with new server-lib names. |
| **B. Simulator/parity crate — first major edit-isolation win** | Move dungeon_simulator tree; share receipt types below gameplay and simulation to break spell_cast→simulator replay edge. Move reusable test_data/cache and spell_cast::test_harness helpers to explicit test-support modules, not cfg(test)-only library internals. Keep private per-system tests in gameplay until cut 5. | Expose the currently used 30-module surface deliberately (not all implementation internals). Preserve Combat/SpellCast/Aura/Follower/etc plugins, manual 50ms tick, single-threaded schedules, exact observations. Simulator-dependent follower systems must not remain a gameplay→simulation edge. | ~10.9k moved LOC plus relevant harness files; ~200–600 export/support/receipt lines. Preserve env!(CARGO_MANIFEST_DIR) asset/output paths with explicit workspace-root handling. |
| **C. Per-area public-API test crates — one pilot, then expand** | Pilot one area (e.g. spell_cast) and measure before expanding to auras, followers, scripted_ai, toys and persistence/protocol, sharing explicit fixture helpers. Retain genuine private unit tests beside each implementation. Do not create one replacement 277k-line test crate. | Prefer plugin/App/World/resource operations already used in tests. Do not expose every private function merely to relocate a test; tests of implementation-private rules remain unit tests. Existing library tests/other workspace tests remain accounted for. | Upper bound ~277k test-only-file LOC, plus inline tests selectively; no credible exact moved total before per-test API audit. Rough wiring ~1–3k lines spread over later small steps, not one commit. |

Every actual file relocation uses `git mv`; keep rename-only content separate from visibility/path wiring commits, with no numerical/gameplay changes. Step A relocates declarations, not files. Larger spell_info/catalog_data extraction is deferred until after A/B/C; the initial leaf proof must not delay the substantive payoff. Announce exact source paths before starting so hot branches can avoid overlaps; land one gated branch at a time, refresh origin/master only between accepted steps. Do not retain alternative implementations or copy modules via #[path] across crate roots. Record moved tests as old target/name → new target/name, enabled/ignored counts and exclusions. A count-only match cannot prove preservation: compare the mapped name census as well. Parent's full-suite gate owns acceptance; no step is declared landed just because its focused tests pass.

## Expected rebuild benefit (not benchmark results)

| Common edit | After catalog-only step | After gameplay + simulator/test isolation |
|---|---|---|
| Test-only body/assertion | Almost unchanged: still inside giant bin test target. | Public-API integration target alone rebuilds/relinks; private unit test edit still rebuilds its owning gameplay test target. Biggest improvement requires C, not just A's library target. |
| Simulator/parity implementation/test | Almost unchanged. | Rebuild simulator/test target, reuse gameplay/catalog artifacts. Main intended win: no re-typechecking/optimizing all unrelated gameplay unit tests. |
| spell/aura gameplay | Still giant target. | Rebuild gameplay and dependent simulator/runtime/test artifacts; **not** cheap. Smaller final test crates remove unrelated test codegen, but one large core is still the next bottleneck. |
| Data loader implementation | Only extracted loader is isolated; its consumers can still become dirty. | Rebuild catalog and downstream dependents. Rust/Cargo invalidation does not promise implementation-only ABI caching; API/generic changes propagate. Loader extraction primarily lets catalogs remain warm during non-loader edits. |

Bevy generic system registration, World/query access and iterators monomorphize where instantiated; some final-crate optimization/codegen remains. Public generics/inline functions carry bodies downstream, so moving LOC is not equivalent to moving LLVM cost. Cross-crate inlining/LTO/profile settings alter both runtime and compile cost; retain current opt2/debug1 behavior and explicitly apply it to new workspace packages. Linking a large executable and memory-pressure stalls do not vanish. New workspace crates inherit package-profile rules differently; do not accidentally benchmark opt0 against opt2 or test different feature graphs.

**Step 1 estimate:** 551 LOC is ~0.11% of the current file-visit corpus, mostly constants/pure non-generic logic. Budget **0–1% compile-CPU saving**, plausibly below wall-time noise (0–26 CPU-s relative to supplied 1,800–2,600 CPU-s builds); this is a heuristic, not measured attribution. Constant arrays can still be instantiated/optimized in consumers. A 15–50 minute build will not become usable from this cut alone. Its value is establishing a safe crate boundary and import pattern before the substantive gameplay/test separation. Do not spend weeks extracting tiny leaves before moving the large knot.

No credible minute estimate for the final split without rustc phase attribution and the paired measurement. Agent 14179 owns incremental/profile/job-count experiments; do not duplicate them. Keep current gameplay runtime performance requirements from [[server-test-performance]].

### Per-step edit-time forecast

| Step | One-line parity edit | One-line spell/aura edit |
|---|---|---|
| Foundation | Near current 15–50min range under comparable host conditions; expected 0–1% CPU saving. | Near current cost; data moved, hot code unchanged. |
| A: lib + thin bin | Still near current cost: simulator/parity remains inside the giant lib test target. Architectural prerequisite, not the promised speedup. | Still expensive lib recompilation; additional dependent thin-bin/test linking possible. No quantified saving. |
| B: separate simulator | Intended to fall to the ~10.9k-line simulator/test rebuild plus remaining Bevy monomorphization/linking; **minutes unknown until paired warm measurement**, not a 10.9/505 linear extrapolation. | Gameplay library and dependent simulator still rebuild. Normal non-test library excludes ~277k test-only-file lines, but remaining core LLVM cost is unknown. |
| C: area test crates | Similar to B unless shared helper surface changes; unrelated test crates must remain fresh. | Core plus requested area/consumer rebuilds. Public-API test-only edits rebuild just their area. Pilot must report duplicated Bevy codegen/link cost and suite-wide build overhead. |

Absolute per-step minute predictions beyond the known baseline would be fabricated. Each acceptance packet replaces these qualitative bounds with same-machine, same-source paired measurements for **both edit classes**; report cold/warm dependency state and no-op controls separately.

## Measurement and acceptance packet for execution

Scope changed after initial analysis: implementation approved by parent. Build grant is `CAPACITY_ID=lead_split_14933`, explicit `-j 4`, one build at a time, `TMPDIR=/var/tmp/osso`, ADD/HOLD/SHED admission through `/home/osso/.worktrees/logs/capacity.sh`; all builds/tests through engine `scripts/agent/agent-run`. Execution checkout: `/home/osso/.worktrees/game-server-crate-split`, branch `crate-split-step-N` from fresh origin/master. Push feature branches only; parent gates full suite and lands.

Before cut 1, warm the **same unsplit execution tip** and measure one appended-comment parity-source edit rebuild. After each cut, repeat the identical edit with fixed machine/jobs/profile/incremental/features/target selection; record wall/user/system CPU, revision, exact command, edited file/line, dirty Cargo targets and test census. Establish a fresh unsplit pair if intervening master changes are incorporated; do not credit master changes or cold dependency builds to extraction. No timings in the planning packet. Agent 14179's runner appends one comment, not a semantic assertion/gameplay change: label results accordingly; measured frontend/incremental reuse is not a promise for arbitrary code edits. Parent specifies opt2/-j2 for paired measurements, -j4 for ordinary step builds. Initial read-only investigation did not consume optional cargo-check permission.

### First execution baseline attempt: resource abort, not a timing result

The fixed-source opt2/-j2 primer on `46c2bf608` was admitted with 15.3GiB host memory available, then safety-aborted when host availability fell to 6,234,888KiB (5.95GiB), below the runner's mandatory 6GiB floor. Observed primer telemetry spans 1,865.805s (~31m06s), **including cold dependencies**; the timed child did not finish, so this is neither a successful compile duration nor a warm edit measurement. Peak sampled aggregate RSS+swap was 6,837,420KiB (~6.52GiB); separate noncoincident maxima were RSS4,694,692KiB and swap3,248,124KiB. Disk remained above the 300GiB stop floor.

Evidence: sibling engine `data/diagnostics/server-crate-split-2026-10-23/unsplit-46c2bf608-parity/opt2-j2/{abort.json,primer-memory.json,primer-host.jsonl,primer-command.json,primer.stderr,run.log}`. No primer timing/summary, test executable or warm measure exists. Source bytes were restored and no owned build processes remained; no automatic retry was started. Dependencies are now warm, but paired before/after attribution remains blocked on a successful baseline. Parent controls the retry/implementation ordering; do not treat this resource abort as evidence that any split has passed.

## Sources

- [Generated graph/churn appendix](server-crate-split-graph.md), `scripts/server-crate-graph.py`, `scripts/test_server_crate_graph.py` — reproducible source evidence.
- `crates/server/src/main.rs`, `crates/server/src/lib.rs`, `crates/server/Cargo.toml`, root `Cargo.toml` at the pinned revision — target ownership and profile boundaries.
- `crates/server/src/spell_info/mod.rs`, `catalog_data.rs`, `spell_cast/effects.rs`, `dungeon_simulator/runtime.rs` — concrete state/test/plugin boundaries.
- [[server-test-performance]] — existing runtime/compile evidence; this plan does not repeat those measurements.

## See Also

- [[dungeon-simulator]] — current transport-free qualification and parity behavior.
- [[slow-native-test-fixtures]] — immutable input/cache contracts that extraction must preserve.
