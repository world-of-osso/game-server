# Late Deserter battleground port refusal — 2026-10-18

CPU-only normal-message-channel proof. Production `7f5d6f2b7c16463bb95fe112050d560631afd361`, based on server `f0f9b31d6` and protocol `a2b07a6`. This documentation-only follow-up does not change the tested Rust source. Lead gate/check/fmt/landing remain separate.

## Source and change

TC a352b1fa `WorldSession::HandleBattleFieldPortOpcode`, BattleGroundHandler.cpp347–358, converts a non-arena accept with `IsDeserter()` to a decline and emits the existing refusal. Player.h2594 defines the predicate as `HasAura(26013)`.

The server reuses landed `validate_no_deserter`. A late violation makes the effective accept false, traverses the unchanged decline/removal and reservation-release path, then returns its normal Deserter error. Match ownership and already-accepted checks remain in the existing response helper. Arena accepts bypass this BG-only check. No new penalty, timer, duration/reset, reward, match minimum, UI or wire format.

## Behavioral proof

Native SpellStore26013 is applied through the real aura API **after** normal invitations exist. Support acceptance and WorldPortAck first create a real reserved map copy; no invitation, phase or roster is fabricated.

| Case | Result |
|---|---|
| Waiting direct1018, one support entered | Refusal, no NewWorld, origin retained, invitation removed and real copy reservation released; clear aura→normal requeue/accept into the same copy |
| InProgress direct1018,19 supports entered naturally | Same refusal/release/readmission; existing peers remain in their copy |
| InProgress original Random32→Arathi2107 | Same refusal/release/readmission, existing configured-map/minimum rules preserved |
| Native skirmish arena invite + late26013 | Acceptance and NewWorld remain allowed |
| Five retained admission cases | Solo/party ×1018/32 refusals and clear-aura admission; native DungeonDeserter71041 does not refuse BG admission |

Observed RED at `9f35c1d63`: three BG cases still accepted the port with errorNone; arena control passed. GREEN at7f5d6f2b7: **9/9** in one `bg_deserter_` invocation, zero ignored. This includes four port cases plus five retained queue-admission cases.

Two additional exact normal-entry controls passed independently: ordinary four-party Arathi20 entry, and Random32 party entry retaining its selected-map source minimum. Total11 distinct passing tests across three invocations, not one11-test run. No source change after these proofs.

## Gate filters and artifacts

- `bg_deserter_`
- `maps::tests::battleground_nodes::arathi_four_normal_five_player_parties_enter_one_twenty_player_match`
- `maps::tests::battleground_nodes::random_queue_parties_enter_selected_map_keep_original32_and_source_start_minimum`

Tests use the approved helper with `--test -p server --bin game-server <filter> -- --nocapture`; the two retained controls also used `--exact`. Workspace format check passed at7f5; manual changed-line Rust readability audit found no new issues. Lead owns the final gate including package check and fmt; no duplicate check was started after that instruction.

Exact commands/revisions/stdout/stderr and proof ledger: game-engine `data/diagnostics/pvp-deserter-port-2026-10-18/`. Native aura/source binding and earlier queue-admission proof: `data/diagnostics/pvp-deserter-admission-2026-10-18/` (landed ds573). No live server, UDP client, rendering, deployment or broader PvP parity is claimed.
