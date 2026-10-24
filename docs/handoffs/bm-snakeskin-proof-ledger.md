# Native Snakeskin execution proof

## Behavioral receipts

| Source | Active native468695 | Actual removed passive | Result |
|---|---|---|---|
| `8e04d880a` RED |122 Auto/43 paid Cobra/0 triggered free |122/43/0 |1FAIL/1PASS33.20s |
| `b77be90e9` GREEN |122 Auto/29 paid Cobra/13 triggered free |122/43/0 |2PASS26.23s |

Both cases use the actual Austin native kit, pinned253 gear/traits, native15% chance, seed70448 and300s clock. No chance forcing, stats/resource assignment or rotation rewrite. Positive case requires real triggered Cobra launches at the successful shot’s native victim, non-guaranteed proc frequency and retained paid Cobra. Removing actual468695 suppresses every free launch. These counts establish proc correctness, **not a DPS improvement**; paid casts decline, and later cadence/resource causes remain open.

GREEN compiled once in29m31s. Compiler JSON selected `server-3270869ea8fa4b0c`, source `b77be90e9c34d4441316ff837a3a3bb37f294334`, ELF SHA256 `cba65c9bb8df7078218615fef7408e148dd37154cb545de4bd73a8a6733be414`, private world SHA256 `cd7230ac1d62bf7d7ea8fef5fbb82faa9f36b61ec78108e591f1204c614a8ed6`. Native current-runtime copy passed normal class-summon closure with0 additions; canonical DB was never written.

Diagnostics under `game-engine/data/diagnostics/dps-parity-fix-2026-10-21/`:

- `snakeskin-red-8e04d880a-{compile.log,native.log,executable.json}`
- `snakeskin-green-b77be90e9-{compile.log,native.log,executable.json}`
- `snakeskin-inputs/manifest.json`, `proof-ledger.jsonl`

Both native runs use `agent-run dps-parity-fix`, existing read-only startup-source wrapper, one process/thread, filter `dps_parity_snakeskin_native_ --ignored --nocapture --test-threads=1`. Compiler/setup errors are not RED.

## Source boundary

Named policy `BM_SNAKESKIN_QUIVER_SIMC_76F8118` follows `sc_hunter.cpp3988–4007/5041–5047`: roll on successful Auto Shot execute and launch free background Cobra. Native468695 supplies chance; native193455 supplies damage and missile behavior. Pinned TC a352b1fa scripts have no Snakeskin binding; this is not called a TC port. Existing `auto_repeat::on_cast` already receives the real victim after native timer setup, so no target guessing or script-argument threading is needed. [Contract](../specs/spell-dummy-scripts.md#native-snakeskin-quiver-execution).

## Subsequent split B integration

Parent requested current `origin/master` integration and `cargo check --locked -p server -p server-simulation --tests`. Merge `31788f21e` moves the fixtures under `crates/server-simulation/src/dungeon_simulator/dps_parity/`, preserving all11 Pack/resource/Snakeskin case names. Only the existing test-support triggered-cast observer is exposed; private core formulas remain private. Rage reads public NativeFacts, and the Snakeskin precondition reads its actual active aura amount.

First two-package check at31788 found one private `end_all_combat` call in the moved Rage fixture, not a gameplay failure. Commit2391f9a7a uses the existing public native `combat::combat_stop` instead; no private API is widened. `cargo check --locked -p server -p server-simulation --tests` then **PASS in2m21s** at2391f9a7a, with no warnings (`crate-split-b-check-2391f9a7a.log` under the same diagnostics root).

The b77 native GREEN above is explicitly **pre-split**. A later changed-boundary requalification at5b7cb789f uses the same newly built `server_simulation-c8dd41cff95cf6e8` as the owner-cooldown RED cases: **Snakeskin2GREEN37.63s**, active120Auto/23paid/16free versus removed120Auto/43paid/0free. Receipt `owner-cooldown-red-5b7cb789f-snakeskin-postsplit.log` under the diagnostics root. This qualifies relocated observer/public fixture paths at runtime without a second build; it remains neither a DPS-gain nor a full-suite claim. Future native builds select `server_simulation-*`, not the old server-lib ELF. Parent owns broad integration gate.
