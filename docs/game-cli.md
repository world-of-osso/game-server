# Headless economy CLI

`game-cli` keeps the existing token login flow, selects the account's first character, enters the world, and interacts with `--npc <server-entity-bits>`. This is NOT a creature template entry or spawn GUID. The character must already stand within interaction range of that NPC.

All actions accept `--server <ip:port>`, `--token <token>` (otherwise the saved token), `--npc <bits>`, and optional `--gossip-option <option-id>`. If the NPC offers gossip, the CLI prints its menu and fails unless the supplied option is offered. It waits for the requested role; every AH action then sends `OpenAuctionHouse` and waits for success before the operation.

## Actions

```text
ah open
ah search [--text TEXT] [--page 0] [--page-size 50]
          [--sort name|min-bid|buyout|time-left|quality|required-level]
          [--direction asc|desc] [--item-id ID] [--class-id RETAIL_CLASS_ID]
          [--quality QUALITY] [--min-level LEVEL] [--max-level LEVEL] [--usable-only]
ah browse [same arguments as ah search]
ah inventory
ah owned
ah bids
ah create --item-guid GUID [--count 1] --min-bid COPPER
          [--buyout COPPER] [--duration short|medium|long]
ah bid --auction-id ID --amount COPPER
ah buyout --auction-id ID
ah cancel --auction-id ID
merchant list
merchant buy --slot SLOT --item-id ID [--count 1]
```

Page indices are zero-based; page sizes are 1–100. Prices are integer copper. Search prints the server-returned query, total/returned rows, listing data, and `request_response_ms`, measured from sending the search request to receiving results (excludes authentication and opening). The server chooses the auction house from the auctioneer session, not a CLI faction override.

`ah search` remains the flat listing operation. `ah browse` sends `QueryAuctionBrowse` only after the auctioneer role and successful open; it waits exclusively for `AuctionBrowseResults`, never flat search results. Browse prints `request_response_ms`, the returned query, `total_results` (distinct matching items before paging), `returned` (items on this page), and each item's ID/name/quality/required level, global `total_quantity`, and `lowest_unit_price` in copper. Quantities and unit prices retain the server's 64-bit values. Both commands expose quality, level bounds, and usable-only filters; the server applies them.

Example (append the auth/server/NPC arguments described above):

```text
ah browse --text Linen --class-id 7 --quality 1 --min-level 1 --max-level 80 --usable-only --page-size 1 --page 0 --sort buyout --direction asc
```

Merchant list waits for the vendor role and its inventory. Buy waits for the initial bag snapshot and vendor list, validates slot/item against that list, sends the purchase, and waits for the resulting bag item-count increase. `--count` is purchases, not individual items: each yields the vendor's `stack_count`. A refreshed limited-stock list alone is not purchase success.

Rejections and interaction closures print explicit failures and exit nonzero. Startup/auth has a 10-second deadline; interaction/opening has a 10-second deadline, and each dispatched operation gets a new 10-second deadline. A timeout does NOT prove a mutation was rejected: inspect inventory/owned/bids before retrying. Mutations are never automatically retried.

## Isolated-server smoke recipe (operator-run)

Do this **before native engine economy testing**. Do not point mutation probes at a shared player database.

1. Start the existing server binary from the repository root with isolated mutable storage, UDP port, and admin socket:

   ```sh
   GAME_SERVER_DB_PATH="$PWD/data/cli-smoke/game.redb" \
   GAME_SERVER_BIND_IP=127.0.0.1 \
   GAME_SERVER_PORT=5098 \
   GAME_SERVER_ADMIN_SOCKET="$PWD/data/cli-smoke/admin.sock" \
   target/debug/game-server
   ```

   Create `data/cli-smoke/` first. `GAME_SERVER_BIND_IP` defaults to all IPv4 interfaces; fixtures explicitly bind loopback. Invalid IPs or ports stop startup rather than selecting another listener. Keep `data/world.db` unchanged. Set the same admin-socket environment for every admin command below. Keep the server running; do not restart the shared server.

2. Use `game-server-admin create-account` / `create-character` with their help syntax to create separate seller and buyer accounts, exactly one character each. Login via `game-cli login --server 127.0.0.1:5098 --username NAME --password PASSWORD`; retain each token separately (`--token` on later calls prevents saved-token ambiguity). Seed seller inventory with `game-server-admin grant-item SELLER 2589 4`; seed money with `set-gold CHARACTER 100000` for both accounts.

3. Query `game-server-admin npc-info 0 0 8719` for Auctioneer Fitch; use its returned entity bits and coordinates. Put both **offline** characters near it using `set-position` (see admin help). For each subsequent command append `--server 127.0.0.1:5098 --token ACCOUNT_TOKEN --npc FITCH_BITS`. Run seller `ah open`, `ah inventory`, then create a linen auction using the returned item GUID. Run seller `ah owned`; record auction ID. Buyer searches `--item-id 2589 --page-size 1 --page 0 --sort buyout`, bids, checks `ah bids`, and buys out. Create a second seller auction, then cancel it. Inspect each response rather than assuming a sent request succeeded.

4. For paging, create at least two matching auctions before searching pages 0 and 1 with `--page-size 1`; assert total >= 2, distinct listing IDs across pages, selected text/item/category filters and both sort directions. Record `request_response_ms`. Repeat with bot-populated inventory when the main economy slice is installed. A one-row fixture does not prove pagination.

5. Query vendor Godric Rothgar (`npc-info 0 0 1213`), move an offline test character near him, then run `merchant list` and `merchant buy --slot 0 --item-id 2379 --count 1` with his entity bits. Verify the bag-count confirmation. Negative probes: an invalid NPC, a distant NPC, an unoffered gossip option, insufficient-money bid/buy, and an unused UDP port. Each must fail nonzero with an explicit rejection or timeout. Only after these CLI probes pass, proceed to native engine testing.

## Targeted development proof

```sh
CARGO_TARGET_DIR=/tmp/game-client-economy-target cargo test -p client --bin game-cli economy_cli -- --nocapture
CARGO_TARGET_DIR=/tmp/game-client-economy-target cargo test -p client --lib economy:: -- --nocapture
```

Browse proofs cover shared search-argument parsing, role/open ordering, exact grouped request transport through HostClient, ignoring flat responses, grouped completion, rejection/timeout, and 64-bit global quantity/price reporting. Existing flat search and merchant proofs remain unchanged. These are client/parser/transport proofs, not live server economy acceptance. Live smoke needs the isolated server recipe above.
