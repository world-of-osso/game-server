# Toy Box server foundation

Account-wide toy learning/favourites are separate from the legacy per-character mount/pet collection. Toy identity comes from locally extracted Retail Toy.db2, not item names or invented lists. [Contract](../../specs/toys.md).

## Catalog preparation

Extract FDID988200 with `casc-local` from the local WoW install. Store the DB2 under the engine's persistent data tree. Then run:

```text
python3 scripts/export-toys.py <988200.db2> <engine>/data/db2/12.1.0.69933/Toy.csv --exporter <engine>/scripts/export_db2_csv.py
python3 scripts/import-toys.py <Toy.csv> data/world.db --report <joins.json>
```

The wrapper registers WoWDBDefs layout821B5D20 with the engine's maintained WDC5 decoder; unknown encrypted records are dropped by that decoder. The importer atomically replaces only the `toy` reference table. It preserves every decrypted identity and writes unresolved joins to the requested report. Runtime joins Item/ItemSparse and the first on-use ItemXItemEffect→ItemEffect ordered by LegacySlotIndex/ID, matching item-use catalog selection. Missing runtime content_item definitions remain unavailable rather than receiving substitute templates. Startup fails explicitly if the reference table has not been prepared.

## Transmitted toy objects and viewer-local models (2026-10-10)

Effect50 had a ritual-only handler: nonritual toys could report success and spawn nothing. `game_objects::summoned` now loads authentic spell-referenced TDB1210 templates/DB2 display bounds (not the Wrath-derived content templates: Stormwind banner194274 uses Retail display10483, not8623) at world-data startup and handles non-scripted generic decoration, chair and mailbox objects. Other template types/scripts and absent content remain explicit preflight failures. Existing ritual channel ownership is unchanged. Template scale, chair seats, destination/facing and map copy are preserved; positive spell duration expires in whole seconds, while nonpositive duration does not set a despawn timer (TC a352b1fa EffectTransmitted).

Target87 is DEST_DEST, not a request to snap to terrain. It preserves the requested or earlier selected destination. TC CheckDst initializes a missing destination to the caster. UseToy currently carries no destination, so standalone87 resolves to the caster; SpellCastIntent ground positions are honored by ordinary spells.

Aura233 is TC HandleUnused/client-side-only appearance, not aura56's global transform. Server stores/replicates the authentic aura and removes it normally. Native client viewer-local humanoid presentation still needs implementation; no rendered parity claimed. Source: pinned TC a352b1fa SpellAuraDefines.h320/SpellAuraEffects.cpp305; concrete local DB2 spells289277 and384585. No synthetic IDs or database writes. Required missing object imports and census receipts live in the toy-effects4 handoff.

## Account state and transport

`account_toys` stores a bitcode map `item_id -> favourite` keyed by account. Learning on normal UseItem writes both ownership and the consumed inventory record in one redb transaction. A duplicate returns an error without consumption. Favourite updates reread the durable account row inside their write transaction.

ToyCollectionUpdate on CollectionChannel carries the full catalog and learned/favourite flags. Initial world entry and each learning/favourite change publish it; updates broadcast to every in-world connection authenticated to that account. Other accounts remain independent. SetToyFavourite and UseToy are client-to-server requests on the same channel. ToyResult identifies Learn/Favourite/Use and carries an optional error; learning also retains the normal InventoryError refusal path.

## Activation boundary

UseToy checks ownership, raw metadata, runtime item eligibility and complete reachable spell preflight before dispatch. Registry-unhandled effects/auras and script-only auras fail explicitly. Aura56 requires a real nonzero creature template; zero-entry special costumes are not implemented. Summon28 requires available properties, supported control and a creature template. Area-trigger179 without a trigger binding/radius is lifecycle-only and unavailable as a toy action. Unsupported implicit targets and trigger depth/cycles are reported too.

### Data-backed toy effects (2026-10-10)

The original 300 summon blockers were primarily reference-data absence, not a stale effect registry: effect28 was already registered. Import authentic local `SummonProperties.csv` through `scripts/db2.py`; a private test database with the table raised the unchanged classifier from38 to43 candidates. Creature templates are still required; do not invent newer Retail creatures from spell names. Preflight now reports property/creature IDs and unsupported control/title pairs separately from effect28.

Wild control0/title0 summons keep their template faction. Ally/guardian/companion summons inherit their caster's faction. Title5 companions join FollowOwner and are immune to PCs/NPCs; positive slots replace previous occupants. Control3 puppets, control4/5 vehicles and specialized titles are not generic temporary creatures. Green Balloon69895 references missing creature53453: the companion test uses existing template25305 as an explicitly labelled mechanism fixture, not a replacement catalog definition.

Visual-only support uses the audited IDs in [cosmetic.rs](../../../crates/server/src/spell_cast/cosmetic.rs), plus runtime checks: a nonzero DifficultyID-0 SpellXSpellVisual and exclusively zero-valued effect3/effect77/aura4, no misc payload, trigger, item or mechanic. Descriptions promise only attachments/sounds/visuals: Touch of the Naaru119144, Fruit Basket119083, Seafarer's Slidewhistle126931 and Heart of a Champion169768 have concrete loopback use tests. SpellGo already broadcasts caster, spell and hit targets; aura4 uses existing ActiveAuras/UnitAuras apply, duration and cancellation. No new spell-specific gameplay script is installed. Visual presence cannot prove that periodic dummy ticks do nothing; the data-backed226 bridge and retained script gaps are described below.

Counterexamples to blanket acceptance: Gadgetzan18986/23453 teleports and scouting-map150743/360598 unlocks flight points despite single zero-valued dummy effects plus visuals. Both stay unavailable. Metadata-drift tests remove a visual binding or append another effect to audited176594 and assert refusal without cooldown/SpellGo.

Retail type meanings (TC a352b1fa SpellAuraDefines/SpellAuraEffects):293 override abilities,321 no actions,428 linked summons,37 effect immunity,105 feather fall,258 override summoned object,260 screen effect,61 model scale,56 transform. They are not interchangeable cosmetic auras; puppet/vehicle controls remain blocked; generic61/105/37/293 and data-qualified linked summons are described below. Aura56 with an existing template was already handled; zero-entry scripted costumes and missing templates stay blocked. Time-Lost Figurine32782 now explicitly proves ModelDisplay changes to a creature23423 display and cancellation removes it to restore the native race/sex model. Aura260 is implemented as ClientVisual: TC's registry labels it `HandleNoImmediateEffect` with no server code required. Nonzero ScreenEffect IDs use standard aura replication/cancellation. Shadescale170187/304370 proves authentic screen1720 state, but the item stays unavailable because content_item is absent; no runtime item is synthesized.

References: TrinityCore a352b1fa `Spell::EffectSummonType` companion immunity/category dispatch; `AuraEffect::HandleAuraDummy` and `Spell::EffectScriptEffect` retain script hooks. Local Retail12.1.0.69933 Spell descriptions, SpellEffect and SpellXSpellVisual provide the per-ID audit, preserved in the toyfx evidence directory.

Supported casts use the existing start_cast pipeline. A per-cast spell clone applies ItemEffect recovery/category overrides; negative values retain SpellCooldowns. SpellCooldownUpdate reports the actual category and durations, including item recovery. ToyResult success means **cast started**, not completed; SpellGo/CastFailed remain the later outcome. Existing character SpellHistory lifetime is unchanged; server-restart cooldown persistence is not added.

### Generic scale, fall, effect immunity and override sets (2026-10-10)

Aura61 sums per-stack percentage points into replicated UnitScale (player minimum0.1, NPC minimum0.01); unit visuals restore native scale on removal. Aura105 replicates FeatherFall, caps downward prediction/server gravity at7yd/s, caps client-reported descent per server tick, and negates landing damage. Removing the last aura restarts fall accounting at current height. TC a352b1fa Unit::RecalculateObjectScale, HandleAuraFeatherFall and MovementUtil.cpp:30 ground these rules.

Aura37 uses MiscValue as a SpellEffects ID, checked during target filtering, per-target effect dispatch and direct aura application, including friendly summon offers. Spell/effect NoImmunity bypass it (TC Unit.cpp:7934, SpellInfo.cpp:3555/3804, DBCEnums.h:2418). Existing UnitAuras and Immune combat logs provide client presentation.

Aura293 reads ordered10-slot OverrideSpellData (local FDID1002186/layout297CBDC8); existing AuraView.overrides carries SpellSet alongside aura332 pairs. Active set spells receive temporary cast permission without modifying KnownSpells or action persistence. Retail OverrideActionBar.lua:42/Setup displays six slots; the native main bar uses that order, preserving zero/empty slots. Dedicated override-bar chrome and the abilities' individual scripts are separate from this generic source mechanism.

Prepare OverrideSpellData with the existing scripts/db2.py local extract/import workflow, now aware of its inline10-element array. The private census database contains3050 authentic records, none encrypted; canonical world.db was not modified. On missing records/spells the toy remains unavailable, not a successful empty override. Missing content_item still blocks Whispers of Rai'Vosh113542: its authentic167273 proves the fall mechanism, not UseToy. Puppet46709 similarly isolates authentic aura65451/37 for a real meeting-stone85 rejection/removal test; its unsupported puppet behavior remains blocked; aura321 itself is now handled below.

Prior census:71/1166 preflight candidates versus historical58. This branch adds3 (18660,90427,119211);10 other gains came from already-landed master teleport/animation support. Supported reachable aura-toy sets:61=15,105=19,37=20,293=30; sets overlap and are not total usable toys. Source/proof receipts: private toyfx2 diagnostics and /home/osso/.worktrees/handoff-toy-effects2.md. See [client presentation](../../../../game-engine/docs/wiki/systems/toy-box.md).

### Periodic dummy, no-actions and linked summons (2026-10-10)

Census **71→75/1166** with the prepared private database; no previous candidate loses support. New items: Convergent Prism193033 and Gnoll/Market/Expedition Tent193476/194059/194060. This is server-preflight availability, not a complete toy-script or rendered-client claim. Without the linked template import the code yields72 candidates.

Aura226:36 toy roots,8 with explicit DB2 triggers. Seven parasol variants trigger a falling aura that consumes another aura; the falling predicate is absent from DB2, so they remain explicit conditional-script gaps. The remaining data bridge is Gin-Ji Knife Set90175:131568 ticks every330ms and triggers131569 at the channel's original target. Its final corpse-chopping dummy131571 and destination-random72 still block the actual toy. Other28 roots have no trigger and require scripts; no periodic dummy is accepted as a cosmetic no-op. TC a352b1fa PeriodicTick leaves226 to scripts; the explicit trigger bridge is derived from these Retail DB2 records, not a claim that TC automatically executes226 triggers.

Aura321 uses SpellCategories.PreventionType bit4 (NO_ACTIONS), **not aura MiscValue**: real toy auras contain0 or85. Application interrupts a matching current cast; eligible categories remain cast-blocked until the last321 leaves. Existing CastBlock dispatch and textual failure detail carry the rejection without a protocol ABI change. TC a352b1fa HandleAuraModNoActions and SharedDefines.h ground the4 bit. All29 type blockers are removed; other toy effects/puppets/items may still block use.

Aura428 applies its trigger from the aura target once, never again on refresh. Removal despawns one owned matching creature per SUMMON effect, preserving other owners. SummonProperties/data gates remain shared with ordinary summons; four vehicle-controlled toys remain blocked. Tents require target pair18/79: the caster destination followed by a backward offset using the authentic DB2 radius7=2yd (TC SelectImplicitDestDestTargets). Other destination-back forms remain unsupported.20/24 linked-aura type blockers are removed; underlying scripts/targets/items still qualify each toy.

Preparation: all24 linked creatures were absent from content_creature_template but existed in local TDB data with models/difficulty rows. `retail_content.TOY_LINKED_SUMMON_ENTRIES` retains them during normal content preparation. For an existing database, run `python3 scripts/add_toy_summon_templates.py --db <private-copy>` before swapping the prepared database through the normal deployment workflow. The additive importer inserted24 templates and36 model rows, exactly matching the existing Retail conversion; no synthetic templates, item definitions or values. Canonical world.db was not changed.

Aura258 remains explicitly unavailable for all17 toys. Its bindings replace gameobject35591 with DB2 MiscValue1's bobber entry, not a creature summon (TC EffectTransmitted). All33 replacement identities are absent from runtime content_gameobject_template; only18 have TDB source rows. Transmitted-gameobject/fishing support is absent, so no data-complete case exists in this dataset. Classifier reasons name the original/replacement entries; durable JSON lists every item/binding and source availability.

Proof/data receipts: `/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/toyfx3-2026-10-10/`; [linked lifecycle](../../../crates/server/src/auras/linked_summon.rs), [importer](../../../scripts/add_toy_summon_templates.py). Concrete loopback tests cover193033 use/cancel; isolated46709/37710 no-actions masks, interruption, overlapping/removal;193476 real cast/model/2yd placement, rotation, refresh, owner isolation and cancellation/death cleanup. The226 fixture preserves90175's real trigger/period but gives child131569 Fireball133's real damage effect **only in the fixture**, because its unimplemented terminal dummy has no observable outcome and triggered casts emit no SpellGo. It proves tick scheduling/targeting/removal, not Gin-Ji toy completion.142528 proves explicit bobber refusal. No client changes, merge, live realm or deployment claim.

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
