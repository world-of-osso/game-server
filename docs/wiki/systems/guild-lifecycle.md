# Guild lifecycle history

Server-authoritative guild lifecycle events are distinct from [bank logs](banks.md). The [contract](../../specs/guild-event-log.md) tracks the ongoing slice; query plumbing exists, producers and acceptance remain pending.

## Storage and query

`guild_event_log.rs` reads additive guild-keyed JSON histories without changing existing guild/rank bitcode records. Startup initializes the table; corrupt stored history fails explicitly rather than becoming an empty log. An absent guild history is genuinely empty.

`guild_ranks.rs` routes `QueryEventLog` through its authenticated in-world connection, using that actor's persisted membership to select history. Replies include current server time, oldest-to-newest events and a typed refusal for nonmembers. The request cannot select another guild. No data is reconstructed from administrative membership setup.

## Producer boundary (pending)

The six real mutations must append within their existing redb transactions, not after commit. Mainline reverses insertion order for display; pinned TC caps retention at100. Rank/actor/member labels must be captured when the event occurs, so later renames/leaves cannot rewrite history. These are the unverified contract, not completed producer behavior.

## Sources

- [Guild event-log contract](../../specs/guild-event-log.md) — source attribution and acceptance ledger.
- `crates/server/src/guild_event_log.rs`, `guild_ranks.rs`, `persistence.rs` — current query/table boundaries.

## See Also

- [Banks](banks.md) — separate per-tab item/money history and guild authority.
