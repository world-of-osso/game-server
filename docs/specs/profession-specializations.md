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
- Shared protocol `profession_messages.rs`: specialization query/commit, node/tree metadata, currency identities and errors.

## Tests asserting this spec

- `crates/server/src/profession_trait_tests.rs`: source Dragon Isles Leatherworking tree350, achievement15876, wallet9, rooted spending, granted thresholds and irreversible purchases.

## Known gaps (current cycle)

- [ ] Request/runtime persistence/acquisition integration and live proof remain open.
- [ ] Crafting-quality numeric effects remain gated by the missing authentic operation evaluator; this does not block sourced recipe unlocks.

## Out of scope

None of the user-requested feature goal is excluded. Unknown source semantics remain explicit gaps.
