# Profession specializations

Source-grounded profession configurations are separate from class talent purchases but use the shared trait validator. [Contract](../../specs/profession-specializations.md); [source import and unresolved operation fields](../reference/profession-reference-import.md). Private native spending/request proof is recorded below; earned acquisition and full operation evaluation remain distinct gates.

## Representation and spending

`SpecializationState.trees` stores a BTreeMap of tree IDs to purchased `{node_id,entry_id,ranks}` records. Purchased ranks never include source-granted ranks. `TraitTree` supplies ordered entries, costs, groups, conditions and edges. The profession layer adds earned quests/achievements, CurrencyTypes wallet budgets and tokens already spent in sibling trees; class level budgets are not profession knowledge.

A commit validates the requested profession and tree, rejects omission/decrease of existing ranks, applies purchased entries, then resolves granted perks until stable. Per-path Tiered entries follow `TraitNodeXTraitNodeEntry._Index`: unlock entry before knowledge entry. Source currency costs are validated together. Only the increase in CurrencyTypes-backed spend is removed from the wallet; derived unlock tokens are represented by source conditions/purchases, not minted into wallet balances. Nonzero `TraitDefinition.SpellID` values become known spells without casting the recipe or changing profession rank.

`profession_specializations` is a separate redb table; existing PlayerProfessions encoding stays unchanged. Specialization state, wallet JSON and learned recipe spells are written in one transaction, before live components are replaced. Loaded purchases are checked against the current source graph, not silently discarded. Save/reopen/abort tests assert the three observable state parts.

## Source acquisition and owner snapshots

Quest metadata classifies TDB reward currency slots by their profession TraitCurrency mapping. Only quests with those source rewards require the specialization catalog/currency/profession state. A valid turn-in writes knowledge in the same transaction as quest completion, items, gold and appearances; ordinary quests keep their existing path. Missing source/state refuses turn-in before items are taken. The live wallet is reloaded afterward, and an already-viewed specialization receives a fresh owner snapshot.

Queries/commits are received only from authenticated InWorld connections and their PlayerLink. Node snapshots include purchased/granted/max ranks, costs, ProfTrait linkage and a tentative-next-purchase error. Perks expose a path node only when their Granted conditions uniquely reference a known ProfTrait path; unresolved ownership stays0, never every path in the tree. Authored Type0 visual edges are sent but do not gate availability. CurrencyTypes IDs and TraitCurrency IDs are both exposed; they are not interchangeable. Retail background art is selected by the client's source `Professions.GetProfessionSpecializationBackgroundAtlas`, not invented from tree IDs.

## Sourced descriptions and perk flags — verified 2026-10-11

`ProfessionSpecPerk.is_major_bonus` reads ProfTraitPerkNode Flags bit2 independently of `unlocks_subpath` bit1 (Retail ProfessionSpecConstantsDocumentation.lua). Root tree350 tests distinguish major node28535 from subpath node28533.

Descriptions join TraitDefinition→ProfessionTrait→ProfessionTraitXEffect→ProfessionEffect→ProfessionEffectType.EnumID. Numbered effect tokens use the imported zero-based effect slot; amounts are imported float32 values, names are the localized type text, not crafting calculations. Definition36464/root knowledge36327 resolves to “Improve at making leather armor, gaining +1 Skill per point in this Specialization.” Source spell-name references resolve through SpellName. Identical bindings are deduplicated; missing, ambiguous or unsupported variables produce explicit unavailable text rather than guessed values or raw placeholders. Definition129480 has multiple distinct effects in slot0 but its authored text uses a spell-name reference, so no effect is arbitrarily chosen.

The ignored `seed_private_crafting_joint_fixture` acceptance setup requires the exact private diagnostic redb/world paths and refuses an existing redb. It primes accounts, professions, achievement15876 and knowledge2025; this is not earned-acquisition evidence. The private world receipt records Clerk Gretal's fixture relocation to open-world map0 and the unchanged Northshire mailbox. Private Modern/Forever native logs and PNGs independently show staged spend/Undo, authoritative root rank0→1/30, knowledge25→24 and resolved +1Skill tooltip. This does not prove actual quest-earned acquisition or numeric quality crafting. [Integrated proof and evidence](crafting-orders.md#integrated-private-proof--2026-10-11).

## Explicit boundaries

Numeric quality/concentration and unresolved profession operation bindings stay gaps. Learning a quality recipe is not permission to craft it. Server cast validation must refuse unsupported effect288/297 before consuming reagents; the order backend owns those cast hooks. Recrafting requires source item lineage/modification evaluation, not a new fixed CREATE_ITEM output.

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
