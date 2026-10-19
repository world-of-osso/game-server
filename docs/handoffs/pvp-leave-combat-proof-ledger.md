# Battlefield leave combat gate

CPU-only; no runtime/deployment claim. Branch `pvp-leave-combat`, base `7a01c8f26`, protocol `aa6d925`; production `9be28be08`, merged master `c67f8c863` in `59d7998c9` before GREEN.

## Contract

[Specification](../specs/pvp-progression.md#ordinary-battleground-party-queue). TC a352b1fa `BattleGroundHandler.cpp431–439` refuses battlefield leave while `IsInCombat()` unless status is `STATUS_WAIT_LEAVE`. `pvp_leave.rs::return_destination` now reads existing CombatStatus and retained match phase; an in-combat participant cannot leave a non-Ended match. Validation happens before teleport or membership/ownership mutation. Ended matches and out-of-combat requests keep the existing return path. No penalty, reward, wire or timeout changes.

## Proof

Raw commands/revisions, source hash and logs: `game-engine/data/diagnostics/pvp-leave-combat-2026-10-18/`.

| Revision | Result | Boundary |
|---|---|---|
| `f85042be0` |3 behavioral REDs; separate control fixture failure | Concrete1018/Random32/arena leave emitted NewWorld and removed membership during real combat. Ended control lacked its interaction sender and failed before victory; NOT a fourth behavioral RED. `red.*.log`. |
| `002ef4e27` | RED3 FAIL +1 control PASS | Ended control uses normal interaction harness/source-node victory, then real hit and successful combat leave. `red2.*.log`. |
| `59d7998c9` |20 PASS +1 test-oracle FAIL | Arena refusal worked; its full copy correctly rejected an outside-entry capacity query with MaxPlayers. That query was not an ownership oracle. Preserved `green.*.log`; no production adjustment. |
| `6659606f2` | GREEN21 PASS/0 FAIL/ignored | Correct ownership oracle resolves actual reserved copy through InstanceManager::find_instance. New4 cases plus retained17 Deserter/bracket/dead-port cases, one invocation. `green2.*.log`. |

Active-match cases enter actual assigned copies via invite/accept/WorldPortAck, select opposing assigned teams, commit nonlethal damage through the normal damage boundary and assert existing CombatStatus true for both actors. Normal LeaveBattleground must emit no NewWorld and preserve accepted membership, phase and copy binding. `combat_stop` clears existing combat; the same normal leave then returns to the origin while peers remain in their copy. Arena uses its actual preparation clock to reach InProgress.

Ended control reaches a real Arathi source-node victory through normal object interactions/ticks, then commits a hit and leaves with active CombatStatus. No direct phase/CombatStatus assignment or new timer policy.

Ignored RED tests were pushed before production changes. Focused filter: `pvp_leave_combat_` (4 cases); RED used `--include-ignored`. Combined GREEN: `maps::tests::battleground_nodes::deserter::` (21 cases), using capacity-admitted `agent-run pvp` and server `scripts/build-host.py --root <slot> --test -p server --bin game-server <filter> -- --nocapture` with frozen sibling protocol/environment overrides.

Changed Rust formatted; manual changed-line readability clear. Lead owns check, fmt and independent final gate; pending. No full-suite claim.
