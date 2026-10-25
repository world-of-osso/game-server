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
| Targeted native `cargo test` through the build helper, all three packages, `--lib -j 2`, six filters, `--include-ignored --nocapture --test-threads=1` | `2d0bc79c8bac07770e0e5545f26fac52a794d1c5` | Exit 0; 36 passed, none failed/ignored. Cold compile 51m41s; total 3,181.43s; no compiler warnings | Subsequent changes are documentation only |
| `cargo fmt --all --check` | `2d0bc79c8bac07770e0e5545f26fac52a794d1c5` | Exit 0 | Subsequent changes are documentation only |

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

No full 300-second Feral ST/3T run, numerical parity, new native mechanic, deployment or push is claimed. The literal development branch still requires explicit parent landing/consumption notices for `bf1102` and `fp1104`.
