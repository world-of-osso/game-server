# Fixed-output crafting orders

Server lifecycle in `crates/server/src/crafting_orders/`, using imported Retail CraftingOrder rows and the existing effect24 cast pipeline. [Implementation and provenance](../wiki/systems/crafting-orders.md).

## What it must do

- [ ] Import/export authentic2047-row layout05F8699D and select89 CraftingOrder→SkillLineAbility→effect24 rows without modifying the source database.
- [ ] Place public/guild/personal orders with exact authored reagent requirements; escrow customer contributions and commission atomically.
- [ ] Browse by type/profession; guild members and named personal recipients alone may claim their addressed orders. Customer need not know the profession; crafter must know the recipe.
- [ ] Fulfil through normal cast timing, known-spell, tools and reagent validation; mail result to customer and commission to crafter in the same transaction. Do not grant the result to the crafter's bags.
- [ ] Release claims; cancel unclaimed orders or expire active orders once, refunding held item identities and commission by mail. Interrupted/failed casts retain escrow and claim for retry/release.
- [ ] Preserve orders, claims and deliveries across redb reopen, and migrate existing two-sender mailboxes before introducing CraftingOrder system mail.

## How it works

- [Crafting orders](../wiki/systems/crafting-orders.md)
- [Existing craft pipeline](../wiki/systems/professions.md#crafting)

## Implementation inventory

- `scripts/export_db2_csv.py`, `scripts/crafting_orders_import.py`: pinned export and copy-only import.
- `crafting_orders/{catalog,rules,store,runtime,cast}.rs`: catalog, lifecycle, atomic storage, protocol and cast completion.
- `profession.rs`, `spell_cast/mod.rs`: order-context hooks into normal crafting and cancellation.
- `persistence_mail*.rs`: system sender and explicit mailbox ABI migration.
- `world_data.rs`, `main.rs`: catalog/plugin startup.

## Tests asserting this spec

- `scripts/tests/test_crafting_orders_import.py`
- `crates/server/src/crafting_orders/tests.rs` (shared real-world profession harness; authentic Glacial Cloak28208).
- `crates/server/src/crafting_orders/mail_compat_tests.rs`

## Known gaps (current cycle)

- [ ] Fees, Retail duration presets, claim lease durations and order quotas are not authored by this DB2. No guessed fees/presets/claim timers: this bounded implementation escrows commission only and uses an explicit positive duration supplied by the requester; this is not proven Retail duration policy.
- [ ] Only13 of89 output definitions are currently published in the supplied world DB. The89-row catalog remains visible; placement fails explicitly for missing output definitions.
- [ ] Native customer NPC service and both-skin UI proof belong to the integrating agents, not this server slice.

## Out of scope

Quality-dependent effect288, optional/quality reagents and recrafting are not fixed-output orders. Protocol belongs to crafting-orders2. No canonical data writes, production deployment or UDP5000 proof in this slice.
