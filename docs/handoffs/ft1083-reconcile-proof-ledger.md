# ft1083 reconciliation

## Scope

Branch `ft1083-reconcile` in `/home/osso/.worktrees/game-server-ft1083-reconcile` starts at `7701fc1b3e99e9c598a77ddec6502cce042eab55` and merges exact master `4996e5967d80fa82b92d6590a59bd005f50d0965`. No push. Parent owns integration. Frozen form-display-power candidate `e4b269947` is untouched.

## Conflict decisions

| Boundary | Resolution |
|---|---|
| Shared literal execution | Re-export the public core knowledge resolver for existing fixture callers; remove the duplicate local wrapper; preserve master's Prepared/resource/row/composite implementation |
| Resolver qualification callers | Keep master's typed optional helper imports; retain both public core APIs |
| Moved launch test | Keep master's `launch_combat_tests` registration |
| Relocated form consumer | Expose world-level `attack_timing` for master's `server-spell-tests` consumer; no timing behavior change |
| Documentation | Preserve audited provenance/refusal, authored form clocks, master fixture paths and Ret/shared additions; remove the duplicate census index entry |

No resolver, provenance, form-clock or master capability is intentionally removed. The maintained worktree helper links canonical native data; no assets or target caches were copied. At setup, form metadata contained all seven nonzero authored `CombatRoundTime` rows and `tdb_spell_proc` contained 1,291 IDs. This is not the historical private `54e9` database fixture.

## Proof ledger

Receipts: `game-engine/data/diagnostics/feral-parity-2026-10-23/ft1083-reconcile/`.

| Command/scope | Revision | Result | Later invalidation |
|---|---|---|---|
| Requested combined native check: `-p server -p server-simulation -p server-spell-tests --tests -j 2` | Merge `ef720fd8b0d85d989501c0a50c5bfbfd2ec2f2c5` | Ran once after capacity admission; exit 101, 499.14s. Five caller diagnostics refer to the same private `execution::resolve_action` import | Visibility fixed in `2d0bc79c8`; retain the failed receipt, do not call it a passing final check |
| Targeted native `cargo test` through the build helper, all three packages, `--lib -j 2`, six filters, `--include-ignored --nocapture --test-threads=1` | `2d0bc79c8bac07770e0e5545f26fac52a794d1c5` | Exit 0; 36 passed, none failed/ignored. Cold compile 51m41s; total 3,181.43s; no compiler warnings | Valid through ledger-only `3fec675b9`; later master merge requires a new integration gate |
| `cargo fmt --all --check` | `2d0bc79c8bac07770e0e5545f26fac52a794d1c5` | Exit 0 | Valid through ledger-only `3fec675b9`; later master merge requires new proof |

The visibility correction is a `pub(crate)` re-export of the same core resolver, not a restored wrapper. The requested broad check was not repeated. The targeted command compiled and linked all three library test targets, including the formerly failing Ret/WDP callers. This is not a new passing `--tests` check covering every target kind. See `check.proof.json`, `tests.proof.json`, retained stdout/stderr, and `fmt.final.proof.json`.

## Capability coverage

| Behavior | Current native proof |
|---|---|
| Known/selected/parent action resolution, passive endpoint refusal, starter-kit admission | 16 core cases passed |
| Unsourced Savage Fury refusal; explicit Infected Wounds and generated Frantic Momentum preservation | 3 core cases passed |
| Cat/Bear authored clocks, unshifted equipment cadence, all authored forms and actor kinds | 4 core cases passed |
| Shared literal selected-target execution/readiness/precombat/expiry, readonly pools, retarget autoattack and WW policy; unselected Feral talent refusal | 10 simulation cases passed |
| Relocated world-level attack-timing consumer and owner-only melee vulnerability | 3 spell-test cases passed |

All 33 historical target cases now have current proof, plus three relocated consumer controls. No fixture failure or unsupported-fact refusal occurred in this run. Manual readability review of the reconciliation changes found no new violations; the duplicate resolver wrapper was removed.

## Limits

No full 300-second Feral ST/3T run, numerical parity, new native mechanic, deployment or push is claimed. `bf1102` subsequently landed and is included through the exact master merge below. Literal native-pool qualification still requires the parent-owned `fp1104` landing notice.

## fr1108 loader gate repair

Parent reported that `fr1108` did not land: both successful-load aura-restriction tests fail with `no such table: tdb_spell_proc`, including an isolated rerun. The failed full gate is not a ready/green claim; other failures in that log remain outside this assigned repair.

Merged exact `a804452eb5be0c2665e33d83a525a23e7f34678e` as `44dc0b1abc1646b60b36ec1f4d3e94a3635d1df4`; only two wiki conflicts required resolution, preserving both sides. `origin/master` had advanced at inspection; the requested exact revision was used. Frozen `e4b269947` is untouched.

The four restriction-loader cases reproduced at `44dc0b1ab`: **2 failed / 2 passed**, exit 101, 2,420.73s including cold single-package compilation (tests 0.03s). Both failures are the same missing TC source table; malformed/absent restriction-table controls passed. Receipts: `game-engine/data/diagnostics/feral-parity-2026-10-23/fr1108-loader/red.{proof.json,stdout.log,stderr.log}`.

Root cause: `open_spell_database` applied only the DB2 schema, but production `load_spell_store` now also requires audited TC proc-source presence. The DB2 importer must not own TC tables. Repair the in-memory fixture constructor with an explicitly empty `tdb_spell_proc(SpellId PRIMARY KEY)` table; no proc rows are invented. Other direct loader fixtures were inspected: immunity tests attach the native database, where SQLite resolves the TC table; teleport fixtures load their separate position adapter rather than `load_spell_store`. Neither requires this fix.

A new missing-proc-source control drops that table and requires the production loader to return its source-table error. Runtime loading remains strict: no fallback, schema migration, database publication or gameplay change. At the separate fix commit `f900c7d08b0a2c823efd60df80f7471f7a1d13e4`, the targeted helper command (`--test -p server --lib -j 2 spell_info::aura_restrictions_tests:: -- --nocapture --test-threads=1`) passed **5/5**, exit 0, 677.18s including compilation (tests 0.04s), with no compiler warnings. Both original failing contracts and all three source/schema rejection controls passed. `cargo fmt --all --check` passed at the same revision. Receipts: `fr1108-loader/green.{proof.json,stdout.log,stderr.log}` and `fmt.proof.json` under the diagnostics root above. Manual readability review found no new violations in the fixture constructor and rejection control. Only documentation changed after this proof; no broad gate was repeated locally. Parent owns the re-gate and push.
