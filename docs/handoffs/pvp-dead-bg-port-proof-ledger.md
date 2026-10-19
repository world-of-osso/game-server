# Dead ordinary battleground invitation port

CPU-only; no live/deployment claim. Branch `pvp-dead-bg-port`, base `ac36572bb`; production `e3d847bc5`. Fetched master before final GREEN; already up to date.

## Contract

[Specification](../specs/pvp-progression.md#ordinary-battleground-party-queue). TC a352b1fa `BattleGroundHandler.cpp380–384` calls `ResurrectPlayer(1.0f)`/`SpawnCorpseBones` for dead accepted entrants, not only arenas. `pvp_entry.rs` incorrectly restricted its existing full-resurrection call to arenas. Removing that restriction reuses `resurrect_player_fully`, which restores health/mana and clears corpse/graveyard state before teleport. Living entrants are not refilled.

No death-penalty, graveyard-selection, resurrection-wave, reward or wire changes.

## Evidence

Raw commands, source hash, revision bindings and logs live in `game-engine/data/diagnostics/pvp-dead-bg-port-2026-10-18/`.

| Revision | Result | Proof |
|---|---|---|
| `6993e482d` | Harness prerequisite failure, NOT RED | Internal invitation snapshot timer was None;3 tests failed before damage. Living control passed. Preserved `red.*.log`. |
| `3ce601a4e` | Behavioral RED:2 FAIL,2 PASS | Concrete1018 and Random32 accept emitted NewWorld but health remained0 instead of100 before WorldPortAck after actual post-invitation lethal damage/death/corpse creation. Dead-arena and injured living-BG controls passed. `red2.*.log`. |
| `e3d847bc5` | GREEN:17 PASS,0 FAIL/ignored | New4 cases plus retained13 Deserter/bracket cases in one invocation. `green.*.log`. |
| `e3d847bc5` | Workspace fmt0; manual changed-line Rust readability clear | Lead owns final independent check/fmt/gate. |

Tests commit actual lethal damage through `combat::deal_direct_damage`; DeathPlugin processes the normal transition and creates a corpse in the origin map. Acceptance uses normal invitation messages. Before WorldPortAck, tests assert exactly one NewWorld to the assigned map, Alive/full health/full mana, no corpse/graveyard. They assert those states again after acknowledgment. Dead arena keeps full resurrection; living BG starts at40/100 health and30/200 mana and retains both values through port/ack.

Ignored RED tests were pushed before production changes; RED used filter `bg_dead_port_ -- --include-ignored --nocapture`. Final combined filter is `maps::tests::battleground_nodes::deserter::` (17 cases); focused filter `bg_dead_port_` selects4.

Commands use `agent-run pvp` plus server `scripts/build-host.py --root <slot> --test -p server --bin game-server <filter> -- --nocapture`, with assigned sibling protocol and BUILD_HOST_SCRIPTS overrides. No full-suite claim. Independent Lead gate/landing pending.
