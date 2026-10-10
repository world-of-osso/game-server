# Toy Box server foundation

Account-wide toy learning/favourites are separate from the legacy per-character mount/pet collection. Toy identity comes from locally extracted Retail Toy.db2, not item names or invented lists. [Contract](../../specs/toys.md).

## Catalog preparation

Extract FDID988200 with `casc-local` from the local WoW install. Store the DB2 under the engine's persistent data tree. Then run:

```text
python3 scripts/export-toys.py <988200.db2> <engine>/data/db2/12.1.0.69933/Toy.csv --exporter <engine>/scripts/export_db2_csv.py
python3 scripts/import-toys.py <Toy.csv> data/world.db --report <joins.json>
```

The wrapper registers WoWDBDefs layout821B5D20 with the engine's maintained WDC5 decoder; unknown encrypted records are dropped by that decoder. The importer atomically replaces only the `toy` reference table. It preserves every decrypted identity and writes unresolved joins to the requested report. Runtime joins Item/ItemSparse and the first on-use ItemXItemEffect→ItemEffect ordered by LegacySlotIndex/ID, matching item-use catalog selection. Missing runtime content_item definitions remain unavailable rather than receiving substitute templates. Startup fails explicitly if the reference table has not been prepared.

## Account state and transport

`account_toys` stores a bitcode map `item_id -> favourite` keyed by account. Learning on normal UseItem writes both ownership and the consumed inventory record in one redb transaction. A duplicate returns an error without consumption. Favourite updates reread the durable account row inside their write transaction.

ToyCollectionUpdate on CollectionChannel carries the full catalog and learned/favourite flags. Initial world entry and each learning/favourite change publish it; updates broadcast to every in-world connection authenticated to that account. Other accounts remain independent. SetToyFavourite and UseToy are client-to-server requests on the same channel. ToyResult identifies Learn/Favourite/Use and carries an optional error; learning also retains the normal InventoryError refusal path.

## Activation boundary

UseToy checks ownership, raw metadata, runtime item eligibility and complete reachable spell preflight before dispatch. Registry-unhandled effects/auras and script-only auras fail explicitly. Aura56 requires a real nonzero creature template; zero-entry special costumes are not implemented. Summon28 requires available properties, supported control and a creature template. Area-trigger179 without a trigger binding/radius is lifecycle-only and unavailable as a toy action. Unsupported implicit targets and trigger depth/cycles are reported too.

### Data-backed toy effects (2026-10-10)

The original 300 summon blockers were primarily reference-data absence, not a stale effect registry: effect28 was already registered. Import authentic local `SummonProperties.csv` through `scripts/db2.py`; a private test database with the table raised the unchanged classifier from38 to43 candidates. Creature templates are still required; do not invent newer Retail creatures from spell names. Preflight now reports property/creature IDs and unsupported control/title pairs separately from effect28.

Wild control0/title0 summons keep their template faction. Ally/guardian/companion summons inherit their caster's faction. Title5 companions join FollowOwner and are immune to PCs/NPCs; positive slots replace previous occupants. Control3 puppets, control4/5 vehicles and specialized titles are not generic temporary creatures. Green Balloon69895 references missing creature53453: the companion test uses existing template25305 as an explicitly labelled mechanism fixture, not a replacement catalog definition.

Visual-only support uses the audited IDs in [cosmetic.rs](../../../crates/server/src/spell_cast/cosmetic.rs), plus runtime checks: a nonzero DifficultyID-0 SpellXSpellVisual and exclusively zero-valued effect3/effect77/aura4, no misc payload, trigger, item or mechanic. Descriptions promise only attachments/sounds/visuals: Touch of the Naaru119144, Fruit Basket119083, Seafarer's Slidewhistle126931 and Heart of a Champion169768 have concrete loopback use tests. SpellGo already broadcasts caster, spell and hit targets; aura4 uses existing ActiveAuras/UnitAuras apply, duration and cancellation. No new spell-specific gameplay script is installed. Aura226 remains unsupported: visual presence cannot prove that periodic script ticks do nothing.

Counterexamples to blanket acceptance: Gadgetzan18986/23453 teleports and scouting-map150743/360598 unlocks flight points despite single zero-valued dummy effects plus visuals. Both stay unavailable. Metadata-drift tests remove a visual binding or append another effect to audited176594 and assert refusal without cooldown/SpellGo.

Retail type meanings (TC a352b1fa SpellAuraDefines/SpellAuraEffects):293 override abilities,321 no actions,428 linked summons,37 effect immunity,105 feather fall,258 override summoned object,260 screen effect,61 model scale,56 transform. They are not interchangeable cosmetic auras; override/control/linked-summon mechanisms remain blocked until their actual state is implemented. Aura56 with an existing template was already handled; zero-entry scripted costumes and missing templates stay blocked. Time-Lost Figurine32782 now explicitly proves ModelDisplay changes to a creature23423 display and cancellation removes it to restore the native race/sex model. Aura260 is implemented as ClientVisual: TC's registry labels it `HandleNoImmediateEffect` with no server code required. Nonzero ScreenEffect IDs use standard aura replication/cancellation. Shadescale170187/304370 proves authentic screen1720 state, but the item stays unavailable because content_item is absent; no runtime item is synthesized.

References: TrinityCore a352b1fa `Spell::EffectSummonType` companion immunity/category dispatch; `AuraEffect::HandleAuraDummy` and `Spell::EffectScriptEffect` retain script hooks. Local Retail12.1.0.69933 Spell descriptions, SpellEffect and SpellXSpellVisual provide the per-ID audit, preserved in the toyfx evidence directory.

Supported casts use the existing start_cast pipeline. A per-cast spell clone applies ItemEffect recovery/category overrides; negative values retain SpellCooldowns. SpellCooldownUpdate reports the actual category and durations, including item recovery. ToyResult success means **cast started**, not completed; SpellGo/CastFailed remain the later outcome. Existing character SpellHistory lifetime is unchanged; server-restart cooldown persistence is not added.

## Proof and coverage

Pinned local export:1166 decrypted toys. Concrete behavioural fixtures cover Orb of Deception1973 learning/consumption; Time-Lost Figurine32782 spell41301, durable account favourite, second-character relog/reopen, same-account broadcast and cooldown rejection; Eagger Basket188698 spell61810 uses ItemEffect category1203/10s instead of spell category44, rejects repeat and works after expiry; Flag of Ownership38578 rejects unsupported effect3 without starting cooldown. Census output records every root effect/aura and unavailable type including triggered spell chains. These are CPU/loopback proofs, not rendered client or live Retail parity.

## Sources

- [Toy contract](../../specs/toys.md) — requirements and Retail/TrinityCore references.
- [Catalog](../../../crates/server/src/toy_catalog.rs) — joins and unsupported-type preflight.
- [Requests](../../../crates/server/src/toys.rs) — account replication and activation.
- [Persistence](../../../crates/server/src/persistence_toys.rs) — atomic learning.
- [Behavioural fixtures](../../../crates/server/src/toys_tests.rs) — redb/loopback/cooldown proofs.

## See Also

- [[persistence]] — account tables and frozen character ABI.
- [[inventory]] — physical UseItem and inventory delta publication.
- [[spell-casting]] — cast history, effects and cooldown transport.
- [[auras]] — transform and script-only aura limitations.
