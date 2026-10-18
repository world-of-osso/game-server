# Guild lifecycle event log

Authoritative Mainline invite/join/promote/demote/remove/quit history, separate from bank logs and news. Sources: installed `Blizzard_Communities/GuildInfo.lua:158–195`, `GuildInfo.xml:385–459`; TrinityCore a352b1fa `Guild.h:209–216` and `SharedDefines.h:6626`. [Storage and producers](../wiki/systems/guild-lifecycle.md).

## What it must do

- [x] Record only successful actual invitations, joins, explicit step Promote/Demote requests, removals and quits; commit each event with its corresponding durable mutation. Direct SetMemberRank follows its separately qualified no-step-log [contract](guild-ranks.md#direct-member-rank-assignment). Refused/repeated-invalid actions and administrative setup create no historical rows.
- [x] Retain the latest100 actual events in insertion order, including tied timestamps, with event-time actor/member/rank names and Unix seconds. History and current membership/rank state survive database reopen together.
- [x] Query only the authenticated in-world actor's current guild. Members need no elevated rank; nonmembers receive NotInGuild and no history. Never accept a packet-supplied actor/guild ID.
- [x] Empty real history stays empty. Do not backfill from the present roster or substitute guild bank/news history.
- [x] Native Modern and Forever View Log renders the same source dialog, newest first, with real scroll and restart/relog proof.

## How it works

- [Lifecycle storage and producers](../wiki/systems/guild-lifecycle.md).
- [Existing bank log boundary](../wiki/systems/banks.md).

## Implementation inventory

| Files | Role |
|---|---|
| shared-protocol `src/protocol/guild_rank_messages.rs` | QueryEventLog and typed six-kind reply |
| `crates/server/src/guild_event_log.rs` | Additive bounded history, transaction append and member-scoped query |
| `crates/server/src/guild_ranks.rs`, `persistence.rs` | Authenticated route, rank labels and initialization |
| `persistence_character.rs`, `persistence_banks.rs`, `persistence_charters.rs` | Same-transaction lifecycle events, including real founder joins |
| `crates/server/src/guild_ranks_tests.rs` | Real action sequence, refusals, privacy and reopen acceptance |

## Tests asserting this spec

- `guild_ranks_tests.rs::guild_lifecycle_log_records_real_actions_only_and_survives_restart` — RED→GREEN: real six-kind mutations, refusals, event-time names and persisted reopen.
- `guild_event_log.rs::tests::guild_lifecycle_retains_hundred_tied_events_across_reopen_and_rollback` — concrete103-event cap/order and abort/reopen control.
- `persistence_charters_tests.rs::guild_charter_purchase_sign_rename_create_and_consumption_survive_database_reopen` — actual founder join labels/timestamps survive reopen.
- Actual loopback six-kind/order/typed-refusal transport1/1 and native both-skin actual UDP acceptance PASS at engine27661fa5e/server8038e839/protocol4fad8c7. Real member queries/refused promotion/post-quit privacy pass;100 physical rank changes retain100 real events, then a fresh server/client process preserves every event and timestamp with unchanged binary receipts. Evidence: `/home/osso/.worktrees/logs/guild-lifecycle-2026-10-15-run4/`; eleven inspected1920×1080 PNGs in `/syncthing/AgentShared/2026-10-15/guild-completion/*-lifecycle-*.png`.

## Known gaps (current cycle)

- [x] Backend targeted4/4 atc317745f/protocol4fad8c7: real six-action RED→GREEN, founder joins,103→100 tied-order retention, abort and durable reopen. Actual server-process/native restart also passes in both skins; see native acceptance above.
- [x] Native source dialog, source30px scrolling/overflow and UDP/both-skin restart acceptance.
- [ ] Refreshed-master integration gates and coordinated landing.

## Out of scope

News, perks and rewards are subsequent separately landed guild-completion slices. Broader roster/leadership gaps remain in the approved guild audit.
