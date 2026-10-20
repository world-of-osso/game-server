# Guild Last Online recency

Persist actual observed character logout authority, not guessed historical dates. TC a352b1fa Guild.cpp603–606/640/681–685 records, loads and derives inactivity from real logout. Retail GuildRoster.lua115–120 and CommunitiesMemberList.lua1285–1297 consume elapsed recency in existing fields.

## What it must do

- [x] A real in-world disconnect records observed logout time durably; reopen and repeated queries preserve it.
- [x] Unknown history remains absent. Never backfill from query/startup/migration time or RestData timestamps (admin rest commands also write those).
- [x] Offline same-guild snapshots carry optional timestamp authority; online snapshots suppress stale offline time. Other-guild members remain private.
- [x] Distinct genuine logout events retain distinct timestamps. No new guild-only mirror of a second authoritative logout field.

## How it works

- [Guild lifecycle](../wiki/systems/guild-lifecycle.md).

## Implementation inventory

- `crates/server/src/guild_last_online_tests.rs`: actual production Connected-removal/save/despawn, closed DB/new app, authorized GuildChannel query.
- `crates/server/src/networking.rs`: existing disconnect boundary, not arbitrary character saves or rest/admin mutation.

## Tests asserting this spec

- `guild_last_online_real_disconnect_reopen_preserves_distinct_times_and_unknown_history` (ignored RED pushed first).

## Known gaps (current cycle)

- [ ] Actual authority RED observed after real disconnect/save/despawn and closed DB/new-app reopen. Implementation committed; focused GREEN, coordinated P/S/E landing and both-skin native disconnect/readback/reconnect proof pending.

## Out of scope

Sorting, new layout, mobile/BNet/remote chat, inactivity dethroning, professions, leadership/self-policy changes, fabricated history and exact process-crash logout recovery.
