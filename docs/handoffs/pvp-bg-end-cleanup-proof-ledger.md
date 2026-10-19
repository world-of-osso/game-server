# Ordinary battleground completion life/combat cleanup

CPU-only; no live/deployment claim. Branch `pvp-bg-end-cleanup`, base `ecae421c2`, frozen protocol `f9a9175`. Production `89ba4fb3a`; master `72ee66aa4` merged in `6ba7deb02` before GREEN.

## Contract

[Specification](../specs/pvp-progression.md#ordinary-battleground-party-queue). TC a352b1fa `Battleground.cpp675–695` fully resurrects dead current players/clears corpses, or stops living players' combat, during EndBattleground. Existing accepted/current-copy completion roster now invokes `resurrect_player_fully` or `combat_stop` before unchanged achievement/Honor credits and result publication. No leave/teleport, new reward, penalty, wire or clock policy. Arena completion unchanged.

## Evidence

Exact commands/revisions, source hash and logs: `game-engine/data/diagnostics/pvp-bg-end-cleanup-2026-10-18/`.

| Revision | Result | Scope |
|---|---|---|
| `fa7f7217a` | Behavioral RED3 FAIL | Arathi objective victory: dead health0 instead of100; living CombatStatus true instead of false. WSG timeout draw: dead health0 instead of100. Exclusion/no-transfer assertions passed before those failing oracles. |
| `6ba7deb02` | GREEN3 PASS | Same three cases; dead Alive/full health/mana/no corpse/graveyard; living unhealed/unrefilled and combat stopped on first observed Ended state. |
| `6ba7deb02` | Three separate retained controls:1 PASS each | Full arena channel lifecycle/result/origin; Arathi objectives/source Honor/victory/origin; WSG timeout-draw source Honor. |

Six distinct passing tests across four invocations, not one6-test run. No prerequisite failures or broad local gate.

Fixtures drive real source-node interactions/resource ticks and the normal WSG timeout. Native lethal damage/DeathPlugin creates the participant corpse. Small real hits refresh living combat immediately before each terminal tick. Accepted players remain in their copy with no NewWorld. A dead nonparticipant in Stormwind remains dead; injured combatants in a separate manager-reserved copy remain untouched. That copy is an exclusion fixture, not a second queued match. No phase or CombatStatus assignment substitutes for the lifecycle.

Ignored RED pushed before production edits. New family: `bg_end_cleanup_` (RED adds `--include-ignored`). Retained exact filters:

- `maps::tests::four_arena_players_invite_ack_prepare_pick_shadow_sight_end_and_return_over_channel`
- `maps::tests::battleground_nodes::arathi_channel_objectives_scoreboard_victory_and_origin_return`
- `maps::tests::battleground_nodes::warsong_channel_zero_capture_timeout_gets_only_end_honor`

All commands use capacity-admitted `agent-run pvp` and server `scripts/build-host.py --root <slot> --test -p server --bin game-server <filter> -- --nocapture` with frozen sibling/environment overrides; exact controls add `--exact`.

Changed Rust formatted; manual readability clear. Lead owns check/fmt and independent gate/landing; pending.
