# Disenchant

Spell13262/effect99 consumes an owned bag item and opens personal material loot. See [implementation](../wiki/systems/disenchant.md).

## What it must do
- [x] Validate owned Retail bag GUID, eligible ItemDisenchantLoot metadata, and learned Disenchant at cast start and completion.
- [x] Compare authentic SkillRequired generically; current166 rows all require0. No invented skill thresholds or synthetic low-skill test.
- [x] Destroy an eligible uncommon unbound BoE, roll its TDB1210.26091 entry, and expose materials to LootFrame/manual or automatic taking.
- [x] Persist disposal appearance collection and item removal in one inventory transaction, including unbound items; survive redb reopen.
- [x] Refuse a grey item with CantBeDisenchanted, and a caster without Enchanting with NotKnown; retain the item.

## How it works
- [Disenchant pipeline and provenance](../wiki/systems/disenchant.md)
- [Appearance receipts](../wiki/investigations/appearance-acquisition.md)

## Implementation inventory
- `scripts/import-disenchant.py`, `db2.py`, `db2_schema.sql`: local Retail metadata import.
- `crates/server/src/spell_cast/disenchant.rs`: eligibility, completion disposal and materials.
- `spell_cast/mod.rs`, `effects.rs`, `targets.rs`, `spell_info/mod.rs`: item-GUID cast snapshots and effect99.
- `loot.rs`, `loot/tables.rs`: item loot sources, TDB roll and shared loot window.

## Tests asserting this spec
- `crates/server/src/spell_cast/disenchant_tests.rs`: real727/25, refusal, completion ownership, automatic loot and reopen.

## Known gaps (current cycle)
No current gameplay test gap; native rendered/client integration and full-suite proof are not claimed.

## Out of scope
- Scrap, source-aware wardrobe state, item bonus-list quality/level overrides and ItemCurrencyCost-only eligibility; existing inventory model does not provide these inputs.
- Durable unclaimed loot across server restart; item loot uses the existing runtime loot window lifecycle.
- Service restart/deployment, UDP5000, merges.
