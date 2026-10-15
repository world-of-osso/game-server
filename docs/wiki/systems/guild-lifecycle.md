# Guild lifecycle history

Server-authoritative guild lifecycle events are distinct from [bank logs](banks.md). The [contract](../../specs/guild-event-log.md) tracks the ongoing slice; query and atomic producer wiring exist; backend acceptance4/4 passes, native/UDP acceptance remains pending.

## Storage and query

`guild_event_log.rs` reads additive guild-keyed JSON histories without changing existing guild/rank bitcode records. Startup initializes the table; corrupt stored history fails explicitly rather than becoming an empty log. An absent guild history is genuinely empty.

`guild_ranks.rs` routes `QueryEventLog` through its authenticated in-world connection, using that actor's persisted membership to select history. Replies include current server time, oldest-to-newest events and a typed refusal for nonmembers. The request cannot select another guild. No data is reconstructed from administrative membership setup.

## Producer boundary

Pending invitation creation/acceptance, rank promotion/demotion and remove/quit append before their existing redb transaction commits. Charter registration appends actual founder joins with the supplied creation timestamp in its membership/charter-consumption transaction. Administrative set-membership does not reconstruct history. Names are captured from real character records; rank labels are captured before later renames can rewrite their meaning.

The ledger retains the latest100 entries in insertion order, including tied timestamps. Callers retain ownership of commit/abort; appending never opens or commits another transaction. Mainline reverses the returned order for display. Behavioral RED reproduced all six successful actions leaving an empty log; targeted4/4 GREEN atc317745f/protocol4fad8c7 covers all six real mutations, refused actions, no administrative backfill, event-time labels after rank renames, actual founder joins at the creation timestamp,103→100 tied-order retention and transaction abort/durable reopen. Actual network/private server-process restart and native UI acceptance remain pending.

## Sources

- [Guild event-log contract](../../specs/guild-event-log.md) — source attribution and acceptance ledger.
- `crates/server/src/guild_event_log.rs`, `guild_ranks.rs`, `persistence.rs` — current query/table boundaries.

## See Also

- [Banks](banks.md) — separate per-tab item/money history and guild authority.
