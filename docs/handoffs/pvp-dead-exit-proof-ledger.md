# Dead battlefield participant exit

CPU-only; no runtime/deployment claim. Branch `pvp-dead-exit`, base `9a8fd3951`, frozen protocol `f9a9175`. Production `67b6a18dd`; master `ff4595fa3` merged in `14b4710f3` before GREEN.

[Specification](../specs/pvp-progression.md#ordinary-battleground-party-queue). TC a352b1fa `Battleground.cpp846–849`, inside `RemovePlayerAtLeave`, fully resurrects a dead participant and clears its corpse before exit. The normal return path now calls existing `resurrect_player_fully` only for a dead participant, after existing eligibility/combat validation and before teleport. Living injured players are not refilled. No penalty, graveyard-selection, wave, reward, wire or combat-gate changes.

## Evidence

Exact commands, revisions, cached-source hash and raw logs: `game-engine/data/diagnostics/pvp-dead-exit-2026-10-18/`.

| Revision | Result | Boundary |
|---|---|---|
| `44aee50e7` | Behavioral RED:3 FAIL,1 control PASS | Concrete1018/Random32 exit health0 instead of100; surviving arena exit mana30 instead of200 before return WorldPortAck. Living injured exit already passed. `red.*.log`. |
| `14b4710f3` | GREEN25 PASS/0 FAIL/ignored | New4 plus retained21 Deserter/bracket/dead-entry/combat-leave cases in one invocation. `green.*.log`. |

Tests enter actual assigned copies through invite/accept/WorldPortAck, commit real opposing-player lethal damage, and let DeathPlugin create the corpse and stop combat. The surviving match remains InProgress. Ordinary LeaveBattleground must start exactly one origin return, clear membership, and expose Alive/full health/full mana/no corpse/graveyard both before and after return acknowledgment; peers remain in their copy. Living control retains40/100 health and30/200 mana throughout exit. Retained combat-leave cases prove that gate remains unchanged.

Ignored RED tests were pushed before production edits. Focused filter: `pvp_dead_exit_` (4 cases); RED adds `--include-ignored`. Combined GREEN: `maps::tests::battleground_nodes::deserter::` (25 cases). Run through capacity-admitted `agent-run pvp` and server `scripts/build-host.py --root <slot> --test -p server --bin game-server <filter> -- --nocapture`, with the frozen sibling protocol/environment overrides in the receipt.

Changed Rust formatted; manual changed-line readability clear. Lead owns independent check, fmt, gate and landing; pending. No full-suite claim.
