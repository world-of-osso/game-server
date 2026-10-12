# Profession specializations

Profession knowledge, irreversible specialization spending and learned recipes come from Retail ProfTrait/TraitTree data, not character-level talent grants. [Source corpus](profession-reference-import.md).

## What it must do

- [ ] Load the profession's explicitly selected variant trees, preserving visual edges separately from availability edges.
- [ ] Use CurrencyTypes-backed knowledge and authored quest/achievement/node-entry unlock currency sources; never grant knowledge from character level.
- [ ] Validate absolute purchases against node/entry membership, maxima, availability and all currency costs; reject refunds/omissions of committed ranks.
- [ ] Apply source-granted perks after purchases and validate the resulting configuration, including unlock currencies shared across a profession's trees.
- [ ] Persist spending and currency changes atomically; expose authoritative knowledge balances, authored metadata and lock reasons.
- [ ] Learn sourced recipe definition spells on commit without casting them; record unresolved numeric profession effects rather than inventing crafting bonuses.
- [ ] Prove acquisition/spend/effects and persistence on private UDP5871 in both skins.

## How it works

- [Existing trait validation](../wiki/systems/talents.md).
- [Additional source receipt](../wiki/reference/profession-reference-import.md).

## Implementation inventory

- `crates/server/src/spell_info/trait_data.rs`: source trees/conditions/costs and explicit tree-ID loading.
- `crates/server/src/trait_config.rs`, `trait_config/professions.rs`: shared trait validation with profession-earned progression.
- `crates/server/src/profession/specializations/{data,state,store,view,runtime}.rs`: source catalog, pure spending/recipe decisions, atomic persistence, owner snapshots and authenticated requests.
- `crates/server/src/quest_data.rs`, `quest_requests.rs`: source-classified knowledge rewards, written with quest/item/gold rewards in one transaction.
- `crates/server/src/world_data.rs`, `persistence.rs`, `profession.rs`: mandatory source loading, store initialization and plugin wiring.
- Shared protocol `profession_messages.rs`: specialization query/commit, node/tree metadata, currency identities and errors.

## Tests asserting this spec

- `crates/server/src/profession_trait_tests.rs`: source Dragon Isles Leatherworking tree350, achievement15876, wallet9, rooted spending, granted thresholds and irreversible purchases.
- `crates/server/src/profession/specializations/tests.rs`: real-source views, delta-only spending, recipe375110 acquisition without a craft, quest70191/knowledge2025Qty1, atomic three-part persistence/reopen/abort.

## Known gaps (current cycle)

- [ ] Runtime integration tests, actual quest/request boundary proof, and native both-skin proof remain open.
- [ ] Source-known profession effect/description variables without executable bindings remain explicit gaps; nonzero trait definition spells are learned, not cast, and quality operations remain disabled.
- [ ] Crafting-quality numeric effects remain gated by the missing authentic operation evaluator; this does not block sourced recipe unlocks.

## Out of scope

None of the user-requested feature goal is excluded. Unknown source semantics remain explicit gaps.
