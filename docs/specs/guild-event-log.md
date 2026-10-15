# Guild lifecycle event log

Authoritative Mainline invite/join/promote/demote/remove/quit history, separate from bank logs and news. Sources: installed `Blizzard_Communities/GuildInfo.lua:158–195`, `GuildInfo.xml:385–459`; TrinityCore a352b1fa `Guild.h:209–216` and `SharedDefines.h:6626`. [Storage and producers](../wiki/systems/guild-lifecycle.md).

## What it must do

- [ ] Record only successful actual invitations, joins, promotions, demotions, removals and quits; commit each event with its corresponding durable mutation. Refused/repeated-invalid actions and administrative setup create no historical rows.
- [ ] Retain the latest100 actual events in insertion order, including tied timestamps, with event-time actor/member/rank names and Unix seconds. History and current membership/rank state survive database reopen together.
- [ ] Query only the authenticated in-world actor's current guild. Members need no elevated rank; nonmembers receive NotInGuild and no history. Never accept a packet-supplied actor/guild ID.
- [ ] Empty real history stays empty. Do not backfill from the present roster or substitute guild bank/news history.
- [ ] Native Modern and Forever View Log renders the same source dialog, newest first, with real scroll and restart/relog proof.

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

- `guild_ranks_tests.rs::guild_lifecycle_log_records_real_actions_only_and_survives_restart` — pending RED/GREEN.
- `guild_event_log.rs::tests::guild_lifecycle_retains_hundred_tied_events_across_reopen_and_rollback` — concrete103-event cap/order and abort/reopen control.
- `persistence_charters_tests.rs::guild_charter_purchase_sign_rename_create_and_consumption_survive_database_reopen` — actual founder join labels/timestamps survive reopen.
- Native/loopback query controls remain to be added.

## Known gaps (current cycle)

- [ ] Atomic lifecycle producers, bounded writes and real sequence/restart acceptance.
- [ ] Native source dialog and UDP/both-skin acceptance.

## Out of scope

News, perks and rewards are subsequent separately landed guild-completion slices. Broader roster/leadership gaps remain in the approved guild audit.
