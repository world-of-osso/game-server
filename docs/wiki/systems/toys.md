# Toy Box server foundation

Account-wide toy learning/favourites are separate from the legacy per-character mount/pet collection. Toy identity comes from locally extracted Retail Toy.db2, not item names or invented lists. [Contract](../../specs/toys.md).

## Authored script dispatch regression (2026-10-13)

[Coin ordering regression](../../toy-effects14-coverage.md#coin-dispatch-regression-recheck-2026-10-13) records the clean-master counterexample, introducing commit and retained source-specific fix.

## Prismatic Bauble source boundary (2026-10-13)

Exact223143 cosmetic admission has authoritative lifetime/recovery proof, not native rainbow certification. Source ribbon parsing and actual native delivery/lifecycle gaps remain separate. [Claim-specific coverage](../../toy-effects14-coverage.md#prismatic-bauble--source-admission-native-gaps-open-2026-10-13).

## Source facing boundary (2026-10-13)

Two source-facing roots use selected-unit180° arc validation and TC's strict exact3D target-radius exemption before spending. No CombatReach extension; ordinary start GCD is retained on completion refusal. [Sources and claim-specific proof](../../toy-effects14-coverage.md#source-toy-facing--bounded-gate-2026-10-13); [contract](../../specs/toys.md).

## Source area-group lifecycle (2026-10-13)

Cast start and completion consume the pinned Toy root manifest; actual terrain-area lookup and source ParentAreaID ancestry precede cooldown/effect spending. Player-owned restricted auras use ordinary interrupted removal on exit; within-group travel retains them and category history is unchanged. [Source, exact boundaries and proof](../../toy-effects14-coverage.md#source-requiredareasid-lifecycle-2026-10-13); [contract](../../specs/toys.md#source-area-group-closure--active2026-10-13).

## Current PvP faction and War Banner proof (2026-10-13)

`CurrentPvpFaction` carries the authoritative source condition value independently of faction reactions. Login derives the known racial value; accepted BG/arena entry assigns team after successful teleport; successful return restores it. Invitations and refused source transport entry do not assign the team prematurely. Client parser/codec consumes this scalar for36349/36348. Exact sources, coverage matrix, census and private asset boundaries live in [War Banner coverage](../../toy-effects14-coverage.md#current-pvp-faction-and-war-banners-2026-10-13); [match contract](../../specs/battleground-lifecycle.md#current-pvp-faction) owns lifecycle requirements.

## Source summon flag import (2026-10-12)

Canonical read-only inspection found3610 SummonProperties rows with only ID/Control/Faction/Title/Slot, despite the maintained DB2 schema already naming `Flags_0`/`Flags_1`. Local CASC FDID1345276/layoutA4CA5ECF and pinned TC DB2LoadInfo supply both words for3610 readable records; three encrypted records are explicitly omitted. `scripts/import_summon_property_flags.py` validates every existing source identity, transactionally adds the missing columns and populates flags without rewriting identities/unrelated data. Legacy3001 Title4294967295 equals the source signed32 -1 bit pattern; validation normalizes bits, not stored data.

Private visibility is a separate runtime step: TC `SpellEffects.cpp::EffectSummonType` uses OnlyVisibleToSummoner0x10 and OnlyVisibleToSummonerGroup0x10000 to set a private owner; `Object.cpp::CheckPrivateObjectOwnerVisibility` filters seers. Source3077 used by the four warning signs carries0x10, while the existing summon/interest path ignores it. Importing flags alone is not a visibility fix or a new working-toy claim. The owner-only runtime path reads `Flags_0`, attaches an unreplicated `PrivateObjectOwner` before interest registration, and caches that owner in the existing spatial interest index. Normal visibility transitions therefore omit private replicas for other player connections and remove cached ownership with the target. This does not mark other flag semantics implemented; group-private toy graphs retain an explicit preflight gap. Real two-peer UDP RED receives source-private73781 in both replicas while public66768 is visible to both. The first change missed both target-dirty visibility and the separate wild/default `zones::summon_creature` path; missing spawn traces falsified the assumed path. Wild spawns attach the same private ownership marker, and its Add observer updates/invalidates interest after the zone spawn's initial registration. Actual UDP GREEN with source-private Lilian88280 now has owner=true/nearby observer=false while public66768 remains true for both. Source owner model92597/Shirakess170196 also preserves four meshes,.6 scale/no pet frame and natural60s expiry in fresh Modern/Forever native processes (observed59405/59742ms after model readiness). Requested framed owner PNGs inspected; no two-rendered-client claim. Lead canonical command, once requested:

```text
python3 scripts/import_summon_property_flags.py --db2 /home/osso/.worktrees/logs/toy-effects14-continuation/runtime/data/summon-properties-source/1345276.db2 --db data/world.db
```

## Linked cooking traps (2026-10-12)

Source-limited `game_objects::cooking_traps` loads31442 plus addon35, spawns a separate unowned trap at its summoned focus and routes7353 through ordinary triggered spell selection. Environmental activation searches living same-map players within diameter/2 plus scaled combat reach; the spell independently selects10yd allies. Charges0 repeat at the source TC default4s. Parent expiry/removal despawns the linked trap before another refresh. The snapshotted versatility aura keeps its60s duration even after its GO caster disappears; rating-only snapshot retention fixes the observed unconditional casterless-removal failure without claiming other aura/source combinations.

[Exact seven identities, source capsule, RED/GREEN, census337 and representative both-skin proof](../../toy-effects14-coverage.md#linked-cooking-traps-2026-10-12). Leylight253080 missing source metadata and Akunda's Firesticks163211 multi-object chain remain separate; no generic trap or extra focus-field admission.

## Toy-effects14 ordinary caster visual kits (2026-10-11)

Largest remaining group is aura4(273 distinct toys). Complete source audit separates96 zero-shaped bound roots from27 ordinary-model native kit chains, then12 explicit caster-only cosmetic contracts. The unchanged complete-zero predicate admits those12 source-bound spells; it does not accept arbitrary dummy auras. Models/animations/sounds use existing native kit types2/5/6, with no procedural, positioner or conditional payload asserted. Real bag learning/UseToy, exact replicated lifetimes, cancellation and unchanged health/global models are tested. [Exact12 identities/source/gaps](../../toy-effects14-coverage.md#batch3-highest-count-aura4-complete-ordinary-model-caster-kits).

## Toy-effects14 zero-entry costumes (2026-10-11)

TC a352b1fa `HandleAuraTransform` supplies explicit mappings, not creature-entry joins, for Murloc42365/display21723, Gnomeregan75531/displays31654–31655 and Darkspear75532/displays31737–31738. The native player sex selects the two Pride displays even while another transform is active. Existing transform precedence and native-display restoration apply. Toy preflight accepts only those implemented zero-entry cases; other zero-entry costumes remain explicit refusals. [Source and proof](../../toy-effects14-coverage.md).

## Toy-effects14 item selection (2026-10-11)

Toy.db2 membership was absent from the general item-selection union, so 843 source-resolved toy items never reached `content_item`. Of these,156 already pass spell-graph preflight; that is not native-effect certification. General content rebuilding now retains source-resolved Toy members. The additive `scripts/add_toy_summon_templates.py` uses the same `content_policy.ITEM_COLUMNS` conversion, preserves existing runtime rows and still refuses missing required scenery sources. Unresolved Item/ItemSparse rows remain unresolved. No spell, script, payload or coordinate is manufactured.

Prepare only a disposable/approved copy: `python3 scripts/add_toy_summon_templates.py --db data/world.db.new`. This task does not promote canonical world.db. Real-ID tests cover source equality, idempotency, missing-metadata preservation, full item rebuild and Bones of Transformation64646/91760 learning/use/display32473/cancellation restoration. Proof and remaining mechanics: [coverage](../../toy-effects14-coverage.md).

## Toy-effects12 non-summon cosmetics (2026-10-11)

Non-summon runtime-item-eligible ranking starts with aura4=35, effect3=31, effect77=16, effect50=11 and aura56=8 (overlapping counts). Extend the existing shape-checked cosmetic audit, not a blanket dummy executor. Heartbreaker71909 has animation1684/3924 and heart attachment3014; Death's Door193333 has ghost-state model1249985; B.B.F. Fist408067 has animations398982/398986 and hand model978213; Abyss Caller430760 has seven authored water/ritual attachments plus animation kits. Source lifetimes are60000/1800000/86400000ms for the three auras; the horn has no aura duration, gameplay water or summoned actor. No timer is manufactured. All four have complete zero-payload dummy shapes, source visual chains, and no selected spell_script_names bindings in the actual TDB1210 archive.

The Heartbreaker previously deferred in toy-effects10 is now source-audited against its complete animation/attachment chain. Procedural Frosh Hat, zero-entry transforms, missing Black Ice/ring visuals, regional/seasonal scripts, returning balls and interaction objects remain unavailable. Wayfinder's Compass requires Type2 GenericID110576/positioner1937 rendering; it is not an ordinary model attachment. [Coverage and exact gaps](../../toy-effects12-coverage.md). Native rendering is not certified by server-preflight/CPU tests.

## Toy-effects10 cosmetic target and growth auras (2026-10-10)

Ranked prepared-copy runtime blockers: summon28=43, aura4=42, effect3=31. Missing summon templates do not establish interactions/control scripts. Three source-complete aura4 actions extend the unchanged complete zero-valued cosmetic predicate in `spell_cast/cosmetic.rs`; the root Aqir missile remains ordinary trigger32, not an audited dummy.

|Item / root|Audited aura|Source lifetime / target|Local visual evidence|
|---|---|---|---|
|Iron Belly Spirits88587/128327|128327|1000ms cast;60000ms aura / friendly21|visual27023, cast kit23617 and impact9579, animation829 and sound101465; no resource restoration|
|Darkshard Fragment131812/196783|196783|5000ms / caster1|visual51069, crystal model1249984 via kit61005/positioner36; cast392109 via61004|
|Aqir Egg Cluster175063/318453|318452|15000ms / target25|authored missile32 triggers318452/visual94332; kit123945 models1249925,1733424,165723 plus sound147889; no creature spawning/damage|

Local Retail12.1.0.69933 DB2 rows and the actual TDB1210 archive have no selected spell_script_names binding. Descriptions and full visual chains, not absence of bindings alone, establish these cosmetics. Heartbreaker/emote actions, procedural Frosh Hat, Wormhole menus, Gokk'lok's Shell/nonzero dummy, scouting maps and scaled Flame's Radiance stay blocked. No new importer/content changes: prepare toy-effects9 templates on a disposable copy with its existing importer. Behavioral tests exercise real bag learning, UseToy, target-vs-caster routing, missile-triggered aura, unchanged health/model, source lifetimes and retained refusals. Native rendering/audio/positioners are not certified by server proof.

Sources: server `data/diagnostics/toy-effects10-2026-10-10/{selected-source.json,visual-source.json,spell-script-names.sql,before.json}`; [contract](../../specs/toys.md), `crates/server/src/toys_tests.rs`. Prepared-copy census125→128/1166, added only88587/131812/175063 and removed none. Behavioral RED4 aura4 refusals; final GREEN4 including source1000ms cast/8yd missile delay, near-expiry existence and removal. First post-master5e858c8 gates passed349 spell_cast/47 toy tests. Before-push fetch then found source-changing master3953eb3 (class-spell batch15); merged featuref838340 passes360 spell_cast/47 toy tests and format/check warning-free. The second gate is required by that new spell-execution scope, not a repeat on unchanged code. Canonical data was also prepared externally during the task: initial scratch import added16 toy-effects9 templates; final read-only canonical census is128. This branch never wrote canonical world.db. Durable receipts and exact revisions: `/home/osso/.worktrees/handoff-toy-effects10.md` and the linked diagnostics.

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

Target87 is DEST_DEST, preserving the clicked/selected destination. Toy Box now enters the native cursor and carries that point in UseToy; missing/nonfinite/out-of-range points reject before casting instead of silently using the caster. Target47 still uses source forward radius. [Current transport](#toy-effects14-authentic-input-and-rendering-boundaries-2026-10-11).

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

UseToy checks ownership, raw metadata, runtime item eligibility and complete reachable spell preflight before dispatch. Registry-unhandled effects/auras and script-only auras fail explicitly. Aura56 requires a real creature template or an implemented source-defined zero-entry mapping; the three costume mappings above are supported, other special cases remain blocked. Summon28 requires available properties, supported control and a creature template. Area-trigger179 without a trigger binding/radius is lifecycle-only and unavailable as a toy action. Unsupported implicit targets and trigger depth/cycles are reported too.

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

### Toy-effects11 summon classification and scenery (2026-10-10)

[All43 runtime summon blockers](../../toy-effects11-coverage.md) are classified against read-only world.db and the actual local TDB1210 archive:32 missing templates,5 puppet-control3,3 vehicle-control4,3 specialized-title55. Every source display/model metadata join resolves; missing model-file residency is not inferred from metadata. Other effects/targets and interaction/visibility/scale contracts remain explicit. Existing guardians do not close Mylune's dummy/target72 chain. No new SummonProperties type is admitted without its required runtime.

Prepared census128→130/1166 adds Pilfered Sweeper147867 and Brewfest Chowdown Trophy169865, zero removals. Existing generic importer/summon runtime preserves authentic121762/76543 and183727/92396, wild factions35/0, source forward radii1/1.5yd, stationarity and300000ms expiry. Trophy triggers cosmetic caster aura302730: visual88525→kit115127→animation367705/loop82, not a stat buff or guardian. Exact source matrix, preparation command, per-toy retained contracts and proof boundaries live in the linked coverage page. Canonical DB unchanged by this task; parent owns preparation/promotion and native rendering proof.

## Toy-effects14 authentic input and rendering boundaries (2026-10-11)

The [coverage ledger](../../toy-effects14-coverage.md) distinguishes prepared320/1166 from complete effect certification. Source item selection and three TC costume mappings were followed by ordinary caster/area/enemy kits and static actors; unknown scripts, procedure/condition payloads, controls and interactions remain explicit gaps.

Native target87 placement now uses a real clicked point, with source range validation, Escape/right-click cancellation and no cast-at-feet replacement. Source-targeted toys carry the existing player sight witness in UseToy: the real Northshire bake refuses missing/forged rays and accepts an actual Gaze122120 witness. Snapshot metadata prevents caster/ground-only toys from depending on unrelated target eye data. Focused Toy Box typing must not execute AssistTarget/F; the native process regression reproduces and fixes the selected enemy becoming the player.

Stationary ownership no longer takes the combat-pet frame. Source display multipliers .35/.75/.5/1.5 survive summon creation and aura object-scale application/removal; native ModelData/CDI scale remains separate. Master NativeObjectScale for ordinary NPCs and the tick catalog-borrow fix are retained. All four scaled identities have both-skin actual journal/native mesh+scale proof. Sealamp and Spire additionally pass authored CDI state-kit start/scale/ground-origin and natural180-second actor/effect expiry in both skins; Banner/Prey native expiry and display-replacement process proof remain separate boundaries. [Coverage](../../toy-effects14-coverage.md#batch9-source-display-scale-for-stationary-scenery) links exact artifacts.

Both-skin private5911 PNG/JSON proofs cover costumes, wings, friendly-area powder, clicked Scarecrow and witnessed enemy Gaze. Ocean uses its authored precast/cast/impact sounds: isolated 3D source-stream mixes are nonzero in both skins; physical speaker output is not certified. Authentic BC5 texture2022306 exposed GPU dispatch bypassing the existing decoder; real-ID RED→GREEN fixes upload without replacement bytes. New artifacts live outside canonical checkout at `/home/osso/.worktrees/logs/toy-effects14-continuation/`; requested1280×720 PNGs are in `/syncthing/AgentShared/2026-10-11/toy-effects14/`. Private runtime only; no shared realm promotion.

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

## Blazing Diamond Pendant (2026-10-13)

Exact171352 admission uses source zero-payload shape, ordinary ground model937030 and existing area group4031 lifecycle; not generic dummy-aura support. Source/server/native evidence and wall-time limits: [Pendant coverage](../../toy-effects14-coverage.md#blazing-diamond-pendant--bounded-lifecycle-proof-2026-10-13). Dedicated expiry fixture keeps position unchanged and waits for natural aura/kit removal while recovery continues.

## Fountain of Knowledge (2026-10-13)

Exact91743 source cosmetic admission renders ordinary ground253570 with scale.5 for the source10s aura; no creature or gameplay effect. Natural removal preserves the15-minute item recovery. [Source, private asset provenance and bounded native proof](../../toy-effects14-coverage.md#fountain-of-knowledge--bounded-lifecycle-proof-2026-10-13).

## Accursed Tome of the Sargerei (2026-10-13)

Exact187146 source cosmetic admission uses ordinary1097945 at attachment17 and offsets(-.3,0,.3), scale1. Source aura lasts300s; natural removal retains one-hour item recovery without changing body/health/position. [Sources, native scopes and retained learning-update gap](../../toy-effects14-coverage.md#accursed-tome--bounded-lifecycle-proof-2026-10-13).
