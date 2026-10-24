# hs1063 reconciliation gate

Scope: merge `72252f831` with `1466a4ca7`, preserving native haste/Causality/empower/replacement behavior and master's literal execution APIs. No numerical Devastation parity claim, movement repair, push or deployment.

## Integration

`Prepared::setup_pending`, `permits_row`, `execution_in`, composite preparation and self-target selection contexts remain intact. Replacement reads retain real cast/GCD state; composites/items use ordinary admission. Empower/replacement variants participate in native harmful-precombat classification. Test registries and docs are combined. `power_haste_tests.rs` is identical to `1466a4ca7`.

## Proof matrix

| Scope | Result | Artifact under `/var/tmp/osso/` |
| --- | --- | --- |
| `cargo check -p server -p server-simulation -p server-spell-tests --tests -j2` | PASS, 2m04s, no warnings; run once | `hs1063r-check.log` |
| `cargo fmt --all` | PASS; no Rust formatting delta | Pi tool output; Cargo.lock package ordering normalized |
| Native Causality4, composed haste1, empower/replacement2, Dev trait1 | 8 PASS, 81.98s; original frozen Dev input receipt/hash verified | `hs1063r-dev-valid.log` |
| Server haste/history controls15, simulation Energy/Rage unit controls3, Feral trait1 | 19 PASS; 29.52s / 15.97s / 18.47s | `hs1063r-tests.log`, `hs1063r-feral-valid.log` |
| Real-profile Focus/Mana/Energy cap-wait fixtures | 3 FAIL at unchanged-position assertion, 20.49s; master-control classification pending | `hs1063r-power-valid.log` |

Initial three-package Cargo test build completed in55m38s without warnings. Its runtime invocation omitted required profile environment variables:12 simulation cases stopped in setup, not behavior. Only those invalid cases were subsequently executed with valid inputs through the existing read-only startup-source wrapper, using the already compiled binary; successful controls and Cargo were not rerun. The spell-test executable, skipped after simulation failure, selected0 matching tests and supplies no runtime proof (`hs1063r-spell-tests.log`). Overall qualified selection:27 passes,3 failures; gate is not GREEN.

## Resource fixture failure boundary

Inputs: `game-engine/data/diagnostics/dps-parity-fix-2026-10-21/power-haste-packet-inputs/`, with its manifest-verified `world.current-private.class-closure.db` (SHA-256 `dd1de231e32c26291182dff050a8de5a5132908723ed21454eb128e45e4b05ae`). The assertions fail after removing `FollowOwner`, setting `Idle`, then waiting for native resource cap:

| Profile | Position X before | Position X after |
| --- | --- | --- |
| BM253 | 12.896032 | 13.312033 |
| Mana66 | 2.142405 | 2.570886 |
| WW269 | 2.0000052 | 16.0 |

No movement workaround applied. Lead requested exactly these three cases once on plain `origin/master` (`13d2f9633`) with the same inputs after committing this merge. Record that control before deciding whether the integration caused these failures.
