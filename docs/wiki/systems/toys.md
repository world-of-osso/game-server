# Toy Box server foundation

Account-wide toy learning/favourites are separate from the legacy per-character mount/pet collection. Toy identity comes from locally extracted Retail Toy.db2, not item names or invented lists. [Contract](../../specs/toys.md).

## Toy-effects10 cosmetic target and growth auras (2026-10-10)

Ranked prepared-copy runtime blockers: summon28=43, aura4=42, effect3=31. Missing summon templates do not establish interactions/control scripts. Three source-complete aura4 actions extend the unchanged complete zero-valued cosmetic predicate in `spell_cast/cosmetic.rs`; the root Aqir missile remains ordinary trigger32, not an audited dummy.

|Item / root|Audited aura|Source lifetime / target|Local visual evidence|
|---|---|---|---|
|Iron Belly Spirits88587/128327|128327|60000ms / friendly21|visual27023, cast kit23617 and impact9579, animation829 and sound101465; no resource restoration|
|Darkshard Fragment131812/196783|196783|15000ms / caster1|visual51069, crystal model1249984 via kit61005/positioner36; cast392109 via61004|
|Aqir Egg Cluster175063/318453|318452|5000ms / target25|authored missile32 triggers318452/visual94332; kit123945 models1249925,1733424,165723 plus sound147889; no creature spawning/damage|

Local Retail12.1.0.69933 DB2 rows and the actual TDB1210 archive have no selected spell_script_names binding. Descriptions and full visual chains, not absence of bindings alone, establish these cosmetics. Heartbreaker/emote actions, procedural Frosh Hat, Wormhole menus, Gokk'lok's Shell/nonzero dummy, scouting maps and scaled Flame's Radiance stay blocked. No new importer/content changes: prepare toy-effects9 templates on a disposable copy with its existing importer. Behavioral tests exercise real bag learning, UseToy, target-vs-caster routing, missile-triggered aura, unchanged health/model, source lifetimes and retained refusals. Native rendering/audio/positioners are not certified by server proof.

Sources: server `data/diagnostics/toy-effects10-2026-10-10/{selected-source.json,visual-source.json,spell-script-names.sql,before.json}`; [contract](../../specs/toys.md), `crates/server/src/toys_tests.rs`. Baseline125/1166; final proof pending in handoff.

## Toy-effects9 stationary banners and sign (2026-10-10)

Missing runtime items and effect28 templates, not absent generic summon execution, blocked this source-complete group. `scripts/add_toy_summon_templates.py` now adds15 Retail banner item definitions using `content_policy.ITEM_COLUMNS` and16 authentic TDB1210 templates/models (15 banners plus Artisan's Sign). Full content-policy rebuilding retains those selected item/template identities. No world.db mutation is part of this branch: run `python3 scripts/add_toy_summon_templates.py --db data/world.db.new` on an approved copy, then let the owner promote it separately.

|Item|Spell|Creature|Display|
|---|---|---|---|
|163986 Orgrimmar Hero's War Banner|280827|144025|88125|
|163987 Stormwind Champion's War Banner|280828|144026|88126|
|187834 Tormented Banner|360123|181826|102568|
|187957 Encrypted Banner|361085|182459|103372|
|187958 Shrouded Banner|361090|182461|103384|
|197719 Artisan's Sign|381262|193182|108105|
|203716 Thundering Banner|401253|201798|111503|
|208057 Smoldering Banner|417591|208477|113602|
|211424 Dreaming Banner|429364|214220|115934|
|218128 Draconic Banner|442240|220620|118153|
|232301 Tempered Banner|469612|233144|123650|
|232302 Prized Banner|469613|233145|123654|
|232303 Unbound Banner|469614|233146|123655|
|232305 Forged Champion's Banner|469616|233148|123656|
|232306 Prized Champion's Banner|469617|233149|123658|
|232307 Astral Champion's Banner|469618|233150|123659|

All selected spells have exactly one summon effect, Control0/Title0/Slot0, no creature AIName/ScriptName/npcflags/gossip/SmartAI action or spell_script_names binding. Complete reference tuples match the actual local `TDB_full_world_1210.26091_2026_09_09.sql`:16 templates,16 models,21 difficulty rows and0 addons/SmartAI rows. Retail descriptions promise only planting a banner or placing this sign. This does not justify importing Lilian's editable sign, drinks, seats, wormholes or firework opt-in actors. Flame's Radiance Banner238852 has model DisplayScale0.75 and stays deferred; no scaled-render proof is inferred.

Banner target47 preserves radius16=1yd for war banners and radius7=2yd for seasonal banners. Target138 uses the current destination; UseToy has no ground-pick field, so Artisan's Sign lands at the caster after its2000ms cast. Banners last60000ms, sign300000ms. Wild summons keep source faction35 for the two war banners and0 for the other selected scenery, not caster faction. War-banner ItemSparse faction/race masks remain intact.

After merging current origin/master615fb90 into the feature branch, once-only gates pass: spell_cast349/349, toy43/43, importer3/3, format/check with no warnings. The real `tdb_item_ids`→`build_items` rebuild on a disposable copy retains all15 banner items (`content-rebuild.json`). Imported-copy census109→125/1166, +16 and no removals; unchanged shared database109. Behavioral fixtures exercise actual bag learning/UseToy, faction restrictions, SpellGo, one authentic actor/model, destination, ownership/map, stationary position and expiry. Importer tests cover source-row preservation, repeated CLI idempotency and atomic rollback for missing source models/items. Evidence/proof ledger: engine `data/diagnostics/toy-effects9-2026-10-10/` and `/home/osso/.worktrees/handoff-toy-effects9.md`. Server-preflight/CPU proof is not rendered Retail-client parity. [Contract](../../specs/toys.md).

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

Measured server-preflight census75→95/1166:21 newly available object toys plus2 aura233 toys, minus3 former false positives (Little Wickerman70722, Steamworks Sausage Grill116757 and Jeweled Offering193032 still require spell-focus/chest behavior). Two previously admitted chair/mailbox toys now actually spawn; Moonkin chair70161 uses its available authentic TDB template.31 toy tests,19 ritual regressions,12 teleport regressions and7 summon regressions pass (some overlap); format/check pass without warnings. Exact receipts: engine `data/diagnostics/toyfx4-2026-10-10/`. No rendered-client parity claimed.

## Audited attachment and sound toys (2026-10-10)

The largest remaining blocker among toys with runtime item definitions was aura4 (53 candidates), but dummy metadata still does not establish a script's action. The visual-only audit now admits eight more authentic items: Spitzy156871, Worn Cloak165791, Filmless Camera221964, Lampyridae Lure228413, Valdekar's Special263198, Lil' Scoots' Pillow267456, Ken-Ken's Mask88580 and Mallard Mortar204818. Each has a nonzero local DifficultyID-0 SpellXSpellVisual binding, zero-valued dummy-only effects and no selected-spell binding in the local TDB1210 spell_script_names table. Local descriptions promise attachments or a targeted squeak, not damage, teleport, discovery or creature spawning. The existing complete-effect audit remains unchanged; no blanket aura4/effect3 support was added.

Actual bag learning and UseToy prove caster/target SpellGo, aura replication/cancellation and no global transform or damage. Worn Cloak preserves its Horde-only race restriction (Orc use/Human refusal). Ken-Ken's Mask lasts60000ms on the selected friendly player, not the caster. Mallard Mortar has a2000ms cast and a minimum range; the valid20yd target receives the spell without damage. Valdekar's Special3300ms and Lil' Scoots' Pillow11000ms expire normally. Spitzy and Scoots retain their authentic conditional visual bindings; client condition evaluation/asset presentation are not proven by server tests.

Black Ice133997/203533 and Eternal Black Diamond Ring134007/203820 have no local SpellXSpellVisual binding and remain blocked despite attachment descriptions. Negative actual-use tests retain refusal without SpellGo/cooldown. No database import is needed for the eight admitted items; unknown scripts, zero-entry costumes, other aura types/object interactions and viewer-local233 remain outside this audit.

Census95→103/1166 on unchanged reference data. Proof: four new behavioral tests pass; one broad toy run passed34/35, then correcting the Human-only fixture for Worn Cloak made all four new tests pass (35 distinct toy tests covered across receipts). Required `cargo test -p server --bin game-server spell_cast` passed193. Evidence: engine `data/diagnostics/toyfx5-2026-10-10/{before,after,source-records}.json` and final logs. This is server-preflight/CPU proof, not rendered Retail-client parity. [Contract](../../specs/toys.md).

## Toy-effects6 follow-up (2026-10-10)

Duck-Stuffed Duck Lovie194056/383065 is the additional aura4 admission: local description promises a nap, complete effects are zero-valued dummy-only, visual118994 supplies kit162763's duck attachment/AnimKit3404 and kit167706's cast sound. Ordinary bag learning, UseToy, aura replication and cancellation apply without invented healing/transform/gameplay sleep. Behavioral RED was an actual aura4 refusal; GREEN proves learning/use/SpellGo/aura/cancel and unchanged health. No other aura4/effect3 gameplay scripts were inferred from zero-valued metadata.

After merging origin/master2ab5e5e into the feature branch, spell_cast261/261 and toy36/36 pass. Census103→104/1166, only gain194056, no removals. Receipts: engine `data/diagnostics/toy-effects6-2026-10-10/{before,after,delta}.json`, `final-spell-cast.log`, `final-toys.log`. These are server-preflight candidates, not complete Retail visual parity.

[Client follow-up](../../../../game-engine/docs/wiki/systems/toy-box.md#toy-effects6-conditional-selection-and-summon-coverage-2026-10-10) implements Spitzy's single inverted aura condition and imports authentic ordinary train-wrecker appearance coverage. Both configured skins produced native Spitzy/train/Duck Lovie assets through actual Toy Box input. Valid-range mortar remains CPU-proven but live-unproved: final Modern reached19.82yd with hostile faction14 Invader42937 but selection was lost (`NoTarget`); final Forever's selected model left during post-selection repositioning. The final fixture removes that repositioning and learns before selection; this correction is not itself rendered acceptance.

Haunting Memento170950 has DB2period180000 but trigger0; no tick action can be derived. Scoots needs absent ModifierTree303980 and cloak Type17 export lacks procedural payload. Largest remaining groups are aura4=285, summon28=263 and effect3=157; custom scripts/control categories and missing content are not closed by this one admission. Aura233 stays skipped. No world.db mutation, shared-realm deployment or master merge.

## Toy-effects7 visual-only audit (2026-10-10)

The rerun starts at104/1166. Shared blocker incidence remains aura4=285, summon28=263, effect3=157, effect77=61 and effect50=52; those counts overlap. Restricting to actual runtime-item eligible entries gives summon28=46, aura4=45, effect3=31, effect77=18 and effect50=12. Guardian actors do not supply missing creature templates, puppet/vehicle control, click interactions or scripted event actions. For example, importing the wormhole creature alone would not implement its travel menu, and spawning a cushion would not implement sitting on it. Those groups remain blocked rather than receiving invented actions or broad imports.

Three complete zero-valued dummy-aura spells have source-backed visual-only actions: Akazamzarak's Spare Hat147838/239560, Barrier Generator153183/254513 and A Drake's Big Basket of Eggs204675/405634. The existing complete-effect audit remains strict; no generic aura4 admission. Local Retail12.1.0.69933 SpellXSpellVisual→SpellVisualEvent→SpellVisualKitEffect→ModelAttach rows establish hat/rabbit models876205/1244081 with AnimKit9950, barrier model1607602 with hand effects166009, and the basket's unconditional and six caster-conditioned visual choices. No selected-spell binding occurs in the local TDB1210 spell_script_names dump; that absence is not itself proof of cosmetics.

Hat casting takes2000ms and its aura lasts5000ms; no NPC rabbit is synthesized. Barrier is a10000ms cosmetic channel, not an absorb or collidable object. Basket retains the authentic405634 aura and all visual conditions, not a forced player/mount model change. Client mount-condition selection, rendered attachments and sounds are not certified by server admission. All three runtime item definitions and required spell rows already exist; no importer or database write is needed.

Census104→107/1166 (+3, no removals) on unchanged reference data. Behavioral RED3 refusals became GREEN3; post-origin/master toy39/39, format/check pass without warnings. The required one-shot spell_cast gate passed314/315: unchanged guardian sacrifice fixture `guardians_sacrifice_splits_spell_damage_and_stops_on_native_health_floor` failed its random-damage transfer expectation at guardian_tests.rs113. That unrelated failure is retained, not hidden by a rerun or changed toy semantics. Source receipt: canonical engine `data/diagnostics/toy-effects7-2026-10-10/selected-source.json`; full baseline census and ranked candidates are beside it. Haunt tick action, cloak Type17 payload, Scoots ModifierTree303980 and aura233 remain excluded. [Contract](../../specs/toys.md).

## Toy-effects8 stationary scenery preparation (2026-10-10)

Runtime-eligible effect28 is the largest remaining group (46 blocked toys, versus42 aura4). Two entries require only authentic stationary scenery: Silversage Incense88381/127771→65298 and Totem of Harmony88584/128180→65387. Local Retail DB2 gives ally control1/title0/slot0, destination87 and20000/60000ms durations. TDB1210 supplies incense displays41007/41008 and totem41838, with no npcflags, AIName, ScriptName, addon or selected smart-script/spell-script binding. Source model identities are preserved; no client light/burning effect or rendered parity is certified by CPU tests.

[Importer](../../../scripts/add_toy_summon_templates.py) now prepares these two alongside the existing linked summons, using the same TDB conversion as a full content rebuild. Run `python3 scripts/add_toy_summon_templates.py --db <disposable-or-approved-db-copy>`; it imports source models/addons and fails explicitly when the required scenery templates/models are missing. Canonical world.db is not modified by this work. Availability gains depend on applying this data preparation; an unchanged shared database stays at107 candidates. Tests learn/use both toys, check source display membership, ownership/map/faction, stationary position and duration-based despawn on a disposable imported copy. A separate imported-copy census retains scripted seating, drinks, wormholes and fireworks as unavailable.

Sources: local12.1.0.69933 Spell/SpellEffect/SpellMisc/SpellDuration/SummonProperties and world.db's authentic TDB1210 creature template/model/difficulty tables; archive `data/reference/tdb1210/TDB_full_world_1210.26091_2026_09_09.sql` spell_script_names. [Contract](../../specs/toys.md). Durable source/proof receipts: engine `data/diagnostics/toy-effects8-2026-10-10/`. Measured107→109/1166 on the imported copy (+88381,+88584,zero removals); unmodified shared database remains107. One post-origin/master gate passed325 spell_cast and41 toy tests; importer2/2 passes source-row preservation, repeat CLI idempotency and rollback on missing authentic models. Format/check pass without warnings. Selected template/model/difficulty/addon/smart-script rows match the actual local archive value-for-value; shared DB SHA256 is unchanged. No native/rendered verification or deployment performed.

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
