# Recurring Loop Elimination

The server performance pass is replacing broad recurring scans with state-, event-, or cadence-bounded work while preserving gameplay and protocol behavior. Each loop is handled as an independent tests-first change.

## Status

| Loop | Previous cadence/work | Target | Status |
|---|---|---|---|
| Auction expiration | Scan every auction at 20 Hz | Scan at most once per second | Completed in `e02ce0a` |
| Terrain streaming | Recompute each connection's 3×3 neighborhood at 20 Hz | Evaluate on initial delivery or tile transition | Pending |
| Friends online snapshots | Rebuild all online/friend snapshots every app frame | Rebuild only dirty affected owners | Pending |
| Interest management | Scan every visible entity against every player every 500 ms | Movement/visibility revisions plus spatial buckets | Pending |
| Lightyear replication | Scan every replicated root/component per client every 50 ms | Dirty entity/component queue | Pending |

## Auction Expiration

`AuctionHousePlugin` still schedules expiration in `FixedUpdate`, but a local run condition now executes `expire_auctions` immediately once and then at most once per accumulated second. Sub-second fixed ticks do not borrow the auction state or scan its entries.

Expiration semantics are unchanged: expired auctions are removed from persistence, completed sales deliver their existing proceeds/items, and unsold items return through auction mail. A deadline heap was intentionally not added because the requested one-second cadence is the smaller complete solution.

### Behavioral proof

The test `scheduled_auction_expiration_runs_at_most_once_per_second` exercises the actual `FixedUpdate` registration:

1. The first due auction expires immediately and produces seller mail.
2. A second due auction remains through five 50 ms ticks.
3. The second auction expires when accumulated time reaches one second and produces the second mail entry.

The pre-fix test failed when the second auction expired during the sub-second ticks. The focused GREEN test passed after `e02ce0a`.

## Sources

- [auction plugin](../../../crates/server/src/auction_house.rs) — one-second expiration run condition
- [auction handlers](../../../crates/server/src/auction_house/handlers.rs) — unchanged expiration side effects
- [auction lifecycle tests](../../../crates/server/src/auction_house_tests/lifecycle.rs) — scheduler-boundary RED/GREEN coverage
- `game-server` commit `e02ce0a` — auction cadence implementation and proof

## See Also

- [[tick-model]] — server fixed-update cadence and lower-frequency gates
- [[persistence]] — auction, inventory, and mail storage
- [[semantic-noop-replication]] — earlier elimination of downstream no-op replication work
