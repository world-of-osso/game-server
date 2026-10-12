# Profession specializations

Source-grounded profession configurations are separate from class talent purchases but use the shared trait validator. [Contract](../../specs/profession-specializations.md); [source import and unresolved operation fields](../reference/profession-reference-import.md). Native/request acceptance remains open until the current proof ledger says otherwise.

## Representation and spending

`SpecializationState.trees` stores a BTreeMap of tree IDs to purchased `{node_id,entry_id,ranks}` records. Purchased ranks never include source-granted ranks. `TraitTree` supplies ordered entries, costs, groups, conditions and edges. The profession layer adds earned quests/achievements, CurrencyTypes wallet budgets and tokens already spent in sibling trees; class level budgets are not profession knowledge.

A commit validates the requested profession and tree, rejects omission/decrease of existing ranks, applies purchased entries, then resolves granted perks until stable. Per-path Tiered entries follow `TraitNodeXTraitNodeEntry._Index`: unlock entry before knowledge entry. Source currency costs are validated together. Only the increase in CurrencyTypes-backed spend is removed from the wallet; derived unlock tokens are represented by source conditions/purchases, not minted into wallet balances. Nonzero `TraitDefinition.SpellID` values become known spells without casting the recipe or changing profession rank.

`profession_specializations` is a separate redb table; existing PlayerProfessions encoding stays unchanged. Specialization state, wallet JSON and learned recipe spells are written in one transaction, before live components are replaced. Loaded purchases are checked against the current source graph, not silently discarded. Save/reopen/abort tests assert the three observable state parts.

## Source acquisition and owner snapshots

Quest metadata classifies TDB reward currency slots by their profession TraitCurrency mapping. Only quests with those source rewards require the specialization catalog/currency/profession state. A valid turn-in writes knowledge in the same transaction as quest completion, items, gold and appearances; ordinary quests keep their existing path. Missing source/state refuses turn-in before items are taken. The live wallet is reloaded afterward, and an already-viewed specialization receives a fresh owner snapshot.

Queries/commits are received only from authenticated InWorld connections and their PlayerLink. Node snapshots include purchased/granted/max ranks, costs, ProfTrait linkage and a tentative-next-purchase error. Perks expose a path node only when their Granted conditions uniquely reference a known ProfTrait path; unresolved ownership stays0, never every path in the tree. Authored Type0 visual edges are sent but do not gate availability. CurrencyTypes IDs and TraitCurrency IDs are both exposed; they are not interchangeable. Retail background art is selected by the client's source `Professions.GetProfessionSpecializationBackgroundAtlas`, not invented from tree IDs.

## Explicit boundaries

Numeric quality/concentration and unresolved profession effect/description bindings stay gaps. Learning a quality recipe is not permission to craft it. Server cast validation must refuse unsupported effect288/297 before consuming reagents; the order backend owns those cast hooks. Recrafting requires source item lineage/modification evaluation, not a new fixed CREATE_ITEM output.

The canonical world database is never mutated by this work. Production loading requires the source import into the lead's explicitly selected database; the [reference command](../reference/profession-reference-import.md#sources-and-staging-interface) is the handoff. Private fixtures may stage the references on an isolated copy.

## Sources

- `crates/server/src/profession/specializations/` — source catalog, decisions, atomic store, owner view and runtime.
- `crates/server/src/trait_config/professions.rs` — earned profession progression and validator integration.
- `crates/server/src/quest_data.rs`, `quest_requests.rs` — source classification and reward transaction boundary.
- Local Retail `Blizzard_ProfessionsSpecializations.lua` — PurchaseRank/Apply/Undo and currency behavior.
- TrinityCore a352b1fa `src/server/game/Spells/TraitMgr.cpp` — currency source evaluation, grants and config validation; retained under the canonical crafting diagnostic directory.

## See Also

- [Class talents](talents.md) — unchanged level-derived class progression.
- [Professions](professions.md) — fixed-output crafting pipeline.
