# Fixed-output crafting orders

Verified source/private integration2026-10-11. [Contract and current gaps](../../specs/crafting-orders.md); no production deployment claim.

## Catalog import

CraftingOrder FDID4529122, Retail12.1.0.69933 layout05F8699D,2047 rows. Raw SHA256 `1667dd0855db6d5c7790a452e7bf0114bdc577b531e1e419e933247d8766b0ce`. Exact WoWDBDefs fields: noninline ID, SkillLineAbilityID, CraftingOrderHouseID, unknown Field_10_0_2_46479_002. Unknown field is not interpreted as policy. Joined89 default-difficulty effect24 rows;13 outputs currently have content_item definitions. Test CSV is the exact2047-row export, not a synthetic recipe list.

From this branch, export then create a NEW database copy (output paths must not exist):

```text
python3 scripts/export_db2_csv.py CraftingOrder /home/osso/Projects/world-of-osso/game-engine/data/diagnostics/crafting-2026-10-11/db2/4529122.db2 <new-dir>/CraftingOrder.csv
python3 scripts/crafting_orders_import.py --source-db <source-world.db> --output-db <new-world.db> --csv <new-dir>/CraftingOrder.csv
```

Importer opens source read-only and uses SQLite backup before adding the table. Lead must explicitly choose the resulting world DB for integration; no canonical writes. Without the imported table, startup logs feature unavailability and placement returns an explicit catalog error.

## Durable lifecycle

`crafting_orders_v1` stores ID, fixed recipe/output, customer/guild/recipient/crafter identities, exact requirements/contributions, escrowed item instances, commission, expiry and status. Public/guild/personal wire kinds map to stored0/1/2; open/claimed/completed/cancelled map0/1/2/3. Names and current guild membership resolve server-side; personal name matching is case-insensitive. Claimed orders cannot be customer-cancelled; assigned crafter can release. Expiry refunds either open or claimed orders.

Order/inventory/mail changes commit in one redb write transaction, comparing the previously read order and inventories under the write lock. Split escrow stacks keep instance metadata and source identity. Refunds retain held GUIDs; attachments are partitioned by the existing mail attachment limit. CraftingOrder system mail cannot be returned to a nonexistent player.

Fulfil inserts a transient cast context and calls `cast_for_player`. `check_cast_items` checks the crafter's remaining contributions and authored tools, but does not require space in crafter bags for the customer result. `take_reagents` defers order consumption until the effect24 completion transaction; that transaction consumes crafter contributions and atomically mails output and commission. Normal crafting stays unchanged. Stop/movement/death/completion failure clears only transient context, retaining durable claim/escrow.

Appending the third bitcode MailSender variant changes enum packing. `character_mail` is therefore decoded with its exact legacy two-variant layout and atomically moved into `character_mail_v2` before regular mail loading; existing AuctionHouse/player mail fields and attachments are preserved. No decode fallback.

## Proof

RED importer stubs failed expected counts; RED lifecycle stubs rejected placement; old-mail reopen RED failed with `invalid packing`, demonstrating migration necessity. Post-master server e4603dd6/protocol86dcd21:17 affected crafting tests (including10 order tests),10 existing mail regressions and2 Python importer tests pass; `cargo check --locked -p server` and `cargo fmt --all -- --check` pass without warnings. Targeted GREEN/check/fmt ledger: `/home/osso/.worktrees/handoff-crafting-orders-srv.md`. Persistent artifacts: canonical engine `data/diagnostics/crafting-orders-srv-2026-10-11/`. Later integrating native proof follows.

## Integrated private proof — 2026-10-11

Customer `Query{skill_line:0,mine:true}` now returns source recipes and owned rows across profession tiers while retaining kind/identity filters; native customer callers require no learned profession. A wire RED reproduced the empty catalog caused by exact-zero filtering; the corrected test covers catalog, placement gate, owned query and cancellation. Tier-filter tests use authored Classic Tailoring2540 rather than parent197, with parent197 explicitly empty.

One private UDP5871 instance hosted Modern/Forever knowledge staging/Undo/Apply (rank0→1, wallet25→24) and public Glacial Cloak28208→22658 orders, four fully supplied source slots, 10000-copper escrow, actual effect24 completion, customer attachment receipt and worker commission receipt. Accounts/professions/knowledge/achievement were acceptance-fixture grants, not acquisition proof. Clerk Gretal215258/display117678/menu30243 was privately relocated to Northshire map0; mailbox202578 retained its real location. Its local-CASC authored profile and full164-asset closure were staged only in a private cache:28 material textures freshly extracted,79 missing receipt identities added after85 duplicate hashes matched existing receipts. No unrelated model substitution or canonical cache/world replacement.

Crafter Public browse removes completed rows; native completion is status plus row removal and actual mail, not a retained Completed detail row. Evidence logs/PNGs: canonical engine `data/diagnostics/crafting-ui-2026-10-11/live/`; server/source/asset ledgers: `data/diagnostics/crafting-2026-10-11/`. Owned server stopped after proof; UDP5000 excluded throughout. Final code121866d4/protocol2a48bdc:19 crafting +6 specialization tests,4 profession importer +2 order importer tests,1730 full protocol tests, and both repositories' check/fmt pass. The ignored explicit private seeder is not counted as a passing regression test.

## Sources

- [Server code](../../../crates/server/src/crafting_orders/mod.rs), [tests](../../../crates/server/src/crafting_orders/tests.rs), [importer](../../../scripts/crafting_orders_import.py).
- Local WoWDBDefs `game-engine/data/diagnostics/crafting-2026-10-11/definitions/CraftingOrder.dbd`: exact05F8699D layout.
- Retail `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/Blizzard_Professions/Blizzard_ProfessionsCrafterOrderView.lua`: ClaimOrder, ReleaseOrder, FulfillOrder and own-reagent allocation/confirmation.
- Existing [[professions]] documents TrinityCore a352b1fa normal CheckItems/TakeReagents/CREATE_ITEM rules.

## See Also

- [[professions]] — normal craft validation/progression.
- [[mail]] — existing server mail retention/collection; order duration policy is separate.
