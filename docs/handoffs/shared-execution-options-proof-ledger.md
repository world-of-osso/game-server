# Native literal execution options: proof ledger

## Contract and scope

[Shared APL spec](../specs/simc-apl.md#native-execution-option-boundary). Source-selected integer `empower_to` uses the existing native empowered-cast API. Replacement readiness ignores only the current-cast veto; dispatch validates through the existing native replacement API before cancelling. Native GCD, power, target, cooldown and caster checks remain authoritative. Items retain ordinary admission. No specialization predicates, Driver fork, actor normalization, stats, coordinates or canonical data were changed.

## Behavioral proof

| Boundary | Qualified RED | Qualified GREEN |
|---|---|---|
| Selected Fire Breath stage1 reaches actual target DoT | 3c035f55f: actual12000ms versus native stage1 metadata24000ms,30.81s | 7003defcf: actual duration equals native stage1 metadata |
| Existing Disintegrate can be replaced by Living Flame only after admission | bf0fd2c21: final replacement selection absent after actual GCD, hostile-target, insufficient-Essence and read-only controls pass | 7003defcf: controls pass, replacement starts, finishes and consumes native Mana |

Both GREEN cases ran together in35.57s, exit0, exactly2 tests, zero compiler warnings. Source: `7003defcf60ce888e866ff55ec70c82f50200cde`. Implementations are separate generic commits `a7c3e9abf` and `7003defcf`.

The earlier61218bfda run failed fixture startup GCD and is not RED. The first empower run missed its cone target before comparing duration;254766fc9 records the native release events and positions.3c035f55f uses the existing native auto-attack/pursuit path to face the target, without writing coordinates, and then reaches the duration assertion.

## Reproducible artifacts

Evidence root: `game-engine/data/diagnostics/dev-evoker-parity-2026-10-22/`.

- Completed state, command/environment and frozen input directory: `dev-execution-green-retry1-7003defcf-state.json`.
- Compile JSON: `dev-execution-green-retry1-7003defcf-compile.jsonl`;34.20s, exit0. Native core artifact was fresh from the preceding attempt; only the simulation target rebuilt.
- Runtime: `dev-execution-green-retry1-7003defcf-literal_execution_native.log`.
- Test binary: `server_simulation-c8dd41cff95cf6e8`, SHA256 `5dea4c2b51efd3575a4783e8364513f79d819d6cca03a833ed63c6ab7843e80e`. Binary and paired input hashes were unchanged across execution.
- The preceding `dev-execution-green-7003defcf` worker has no terminal/runtime proof. Its state records the missing completion and exited processes; it is not acceptance evidence.

Build selects `server-simulation --lib` through `agent-run`, one `-j2` job after capacity admission. Runtime uses `scripts/dps_parity_native.py` with filter `literal_execution_native_ --ignored --nocapture --test-threads=1` and the frozen native DB, profile/traits and ground paths recorded in the state file. The wrapper read-only binds the prepared DB.

## Not qualified by these cases

- Other empower ranks, non-integer option errors, or combined empower/replacement options. Unsupported combinations fail explicitly; no fallback is claimed.
- Shared Driver early-chain/cancel predicates, source-wide Devastation bindings or all89-row execution.
- The separate core `native_apl_prev_gcd_` case; its genuine RED is recorded in the shared spec, GREEN remains pending the parent core artifact/gate.
- Causality, haste or300s DPS parity. Their evidence remains separately scoped; this change does not establish the±10% goal.
