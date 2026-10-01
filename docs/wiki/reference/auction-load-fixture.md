# Auction load fixture

Explicit disposable database producer for the CLI-first auction load boundary. It writes real `AuctionData` through `economy::store::commit_market`; it does not define production listing policy or prove network latency. Main owns invocation, server/CLI execution and subsequent engine ordering.

## Inputs and invocation

Run only from `/home/osso/.worktrees/game-server-bot-economy`, through Pyrun argv helpers. Coordinate `CARGO_TARGET_DIR` with main before compilation. Catalog must be JSON produced by `scripts/economy_catalog.py`; world input is opened read-only by the actual item-template loader. No copied bitcode layouts or SQLite templates.

All five environment inputs are required:

- `AH_FIXTURE_OWNER=owned-disposable-ah-load-fixture`: caller accepts sole ownership of a disposable fixture DB, including any partial file after failure.
- `AH_FIXTURE_DB`: absolute, **nonexisting** file in an existing owned directory; filename `game.redb` forbidden. Atomic `create_new` refuses existing files/symlinks without writing them. No default database path, overwrite or automatic deletion.
- `AH_FIXTURE_CATALOG`: absolute importer config JSON path.
- `AH_FIXTURE_WORLD`: absolute `world.db` path, read-only.
- `AH_FIXTURE_EXPIRY_START`: explicit Unix seconds, over one hour ahead of invocation. Same inputs yield identical offers; choose enough runway for main's CLI work.

Example Pyrun invocation (replace DB with a new caller-owned path and expiry with a future timestamp):

```python
repo = '/home/osso/.worktrees/game-server-bot-economy'
fixture_env = {
    'CARGO_TARGET_DIR': repo + '/target',  # only after main agrees
    'AH_FIXTURE_OWNER': 'owned-disposable-ah-load-fixture',
    'AH_FIXTURE_DB': '/tmp/owned-ah-proof/offers.redb',
    'AH_FIXTURE_CATALOG': repo + '/data/economy/config.json',
    'AH_FIXTURE_WORLD': repo + '/data/world.db',
    'AH_FIXTURE_EXPIRY_START': '1791417600',  # October 8, 2026 00:00 UTC
}
cli.cargo('test', '-p', 'server', '--bin', 'game-server',
          'auction_house::tests::load_fixture::produce_owned_300k_auction_fixture',
          '--', '--ignored', '--exact', '--nocapture').cwd(repo).env(fixture_env).run()
```

Existing-file refusal development test:

```text
cargo test -p server --bin game-server auction_house::tests::load_fixture::load_fixture_refuses_existing_database_without_changing_bytes -- --exact
```

## Defined boundary and proof

Exactly 300,000 live neutral-house offers, round-robin across the sorted union of actual catalog gatherables, selected recipe outputs and ingredients. Missing market/metadata/world templates fail rather than invent replacements. Catalog must have over 50 distinct eligible IDs; producer prints observed distinct count, not a fabricated target. Names, quality, required levels and vendor values come from importer metadata. Quantities cycle from one through each actual template's `max_stack`. Unit prices vary deterministically within actual catalog market bounds; stack totals use checked multiplication. Expirations stagger by one second from the required future start.

Seller character ID 1 is explicitly fixture-only economic ownership, not a login character. Before central market batch writes, producer reserves last-used counters: auction/item-instance 300000, character 1. It closes and reopens redb, asserts every decoded offer equals its expected real structure, verifies distinct IDs and last-used counters, and exercises real counter allocation. Those assertions consume IDs 300001/300001/2; subsequent real fixture auctions/items/characters begin at 300002/300002/3. No economy state, accounts, login characters or inventories are fabricated. Main may create actual fixture players afterward without collisions.

Real browse/search helpers prove populated, nonoverlapping first/second grouped pages and first/second ItemID offer pages after reopening. Output `AH_FIXTURE_PAGES` reports actual page/offer counts; `AH_FIXTURE` reports ownership, paths, offer/distinct counts, expiry range and next IDs. These are persistence/helper assertions, **not** the actual CLI request/response `<200ms` acceptance measurement. Fixture changes no house caps, production policies, layout or protocol.

## Saved execution proof (October 1, 2026)

`/home/osso/.cache/economy-cli-20260930/a54a0ad0/load-producer-final.json` records a passing producer run: 300000 offers, 311 real ItemIDs, grouped first/second pages 50/50, ItemID 4264 with 965 offers and second page 50. Next auction/item-instance/character IDs are 300002/300002/3. This supersedes a merely planned producer invocation, not production policy. Saved `load-fixture-tests.json` separately records two passing ownership/counter tests and the opt-in producer ignored in that invocation.

[[auction-house]] owns subsequent actual CLI timing: the live flat population is 300075, not exactly the producer's 300000. Those five sampled loopback workloads are request-response proof, not a universal concurrency SLA. Artifact run label is not an exact Git revision stamp.

## Sources

- [Producer and assertions](../../../crates/server/src/auction_house_tests/load_fixture.rs).
- [Central market persistence](../../../crates/server/src/economy/store.rs).
- [Real templates](../../../crates/server/src/item_templates.rs).

## See Also

- [[auction-house]] — actual CLI/network proof boundary.
- [[economy-catalog]] — real catalog generation and eligibility provenance.
