# Quest GO and spell-credit coverage

Verified: 2026-10-11. Source: canonical world.db (resolved immutable Skyborn snapshot), TDB1210 `tdb_quest_objectives`, TrinityCore a352b1fa `QuestDef.h`/`GameObject::Use`/`SpellEffects.cpp`, local CASC Lock1343608 (WoWDBDefs layoutDA418152). No world.db rows changed. Contract: [quests](specs/quests.md); implementation/input details: [game objects](wiki/systems/game-objects.md#quest-objects-2026-10-11).

## Per-Type census and before/after

**Counts measure mapped counter/event routes, not fully playable quests.** A mapped row matches QuestID, Type, ObjectID and required Amount in the existing four NPC/GO or six item slots. After GO counts additionally require a spawned goober with a display box. Availability, map hosting, objective flags/sequencing, script/condition execution and complete spell chains are not certified by these joins. Raw data contains61511 objectives; the runtime content policy selects 12343 quest templates, not every TDB quest.

| Type | Retail objective | Raw TDB rows | In runtime quest catalog | Before → after route rows |
|---:|---|---:|---:|---:|
| 0 | Monster | 37239 | 4550 | 4488 → 4488; authored effect90/134 delivery **0 → 895** |
| 1 | Item | 15282 | 6194 | 6181 → 6181; authored chest delivery **0 → 546** |
| 2 | GameObject | 2948 | 248 | **0 → 186** |
| 3 | TalkTo | 710 | 10 | 0 → 0 |
| 4 | Currency | 259 | 47 | 0 → 0 |
| 5 | LearnSpell | 108 | 40 | 0 → 0 |
| 6 | MinReputation | 230 | 129 | 0 → 0 |
| 7 | MaxReputation | 2 | 2 | 0 → 0 |
| 8 | Money | 117 | 55 | 0 → 0 (legacy turn-in costs are not this Type) |
| 9 | PlayerKills | 72 | 5 | 0 → 0 |
| 10 | AreaTrigger | 99 | 83 | 52 → 52 (existing involved-relation route) |
| 11 | WinPetBattleAgainstNPC | 239 | 62 | 0 → 0 |
| 12 | DefeatBattlePet | 137 | 0 | 0 → 0 |
| 13 | WinPVPPetBattles | 8 | 1 | 0 → 0 |
| 14 | CriteriaTree | 2083 | 6 | 0 → 0 |
| 15 | ProgressBar | 1092 | 0 | 0 → 0 |
| 16 | HaveCurrency | 33 | 0 | 0 → 0 |
| 17 | ObtainCurrency | 156 | 0 | 0 → 0 |
| 18 | IncreaseReputation | 0 | 0 | 0 → 0 |
| 19 | AreaTriggerEnter | 500 | 1 | 0 → 0 |
| 20 | AreaTriggerExit | 10 | 0 | 0 → 0 |
| 21 | KillWithLabel | 176 | 0 | 0 → 0 |
| 22 | Unk1127 | 11 | 0 | 0 → 0 |

Type2:246/248 catalog rows survive the legacy slot shape;186 have the implemented goober event route (110 quests). The remaining60 mapped rows need other GO types, scripts or absent spawn/display data;2700 raw rows belong outside the selected quest catalog. Native locks may further refuse an otherwise matched route when its alternatives require unsupported skills/spell keys.

Spell effects90/134 reach895 mapped Type0 rows across673 quests; chest loot item joins reach546 mapped Type1 rows across461 quests. These are source-route census joins, not673/461 live completions. Retail defines **no cast-spell objective Type**: Type5 is learning a spell; ordinary cast/item-use credit advances an authored monster or label objective. Effect16 completion events and316/317 label credit are still unimplemented. Existing item/Spray Water/SmartAI credit for26391 remains separate.

## Behavioral proof

| Feature | Real data / observable result | Proof level |
|---|---|---|
| GO use |11602: out-of-range/other-map refusal, ten distinct egg sacs, no repeated consumed credit, actual reward | Native server request test |
| Required key |8889: provided24337 permits first spire; destroying it prevents second spire credit | Native server request test |
| GO quest-item loot |9548: Clopper's Equipment181746 exposes23830, no inventory credit until taken, no reroll, actual reward | Current real-data server request test, including next-frame corpse-cleanup failure boundary |
| Chest metadata |Crate1586 references24985 for highlighting, not a goober acceptance gate | Native server request test |
| Personal spell credit |27955/spell94940: no creature death, capped progress, nearby party member not credited, actual reward | Real spell target-resolution test |
| Group spell credit |10566/spells37221/37320/37322/37323: matching counts, nearby/far/other-map party cases, actual reward | Real spell target-resolution test |
| Item cast |28175/item63139 → spell89040 → count and actual reward | Actual inventory UseItem request with real scaling/spell data and authored implicit-target conditions |
| Client rendering/input |Chest/goober render admission + Interact cursor; actual UDP UseGameObject in both skins | Native engine tests2/2 |
| Rendered GO use |26446: eight authored Fertile Dirt Mounds; Gnarl reward dialogs and turn-in,11750XP +10g50s | Modern and Forever native rendered proof |
| Rendered item spell |28175: provided63139 native bag right-click; Velastrasza reward dialogs and turn-in,11750XP +13g74s | Modern and Forever native rendered proof |
| Rendered chest loot |7848 accepted;179553 not mirrored in dungeon230. Alternative10058 cannot enter530;29412/29401 givers hidden | Blocked; no rendered item-loot turn-in claim |

Evidence: `game-engine/data/diagnostics/quest-objectives-2026-10-11/`; successful `live-go-{modern,forever}.log` and `live-spell-{modern,forever}.log` retain real reward messages. Twelve successful object/item/reward/turn-in PNGs and three diagnostic PNGs under `/syncthing/AgentShared/2026-10-11/quest-objectives/` were inspected in contact sheets no larger than1280px. Serverae312c3, engine392f6fdd6 produced four successful scenarios; subsequent fixture-only commits add roster diagnostics and wait for each bag-count increment. Private UDP5496 and `fb_*` accounts only; no objective/reward admin writes.

`quest_objectives_live.gd` now places the player three yards before the actual pick shape along the existing camera direction and searches its complete projected box (the viewport when large treant/dragon bounds cross the camera). Native UnitPicker rays and right-clicks still drive interaction. This removes synthetic steering as a fixture dependency; it does not establish a product camera defect or repair. A resumed quest no longer blindly sends Escape and potentially opens GameMenu. PackedStringArray counters are compared as Arrays.

Loot boundary: Core Fragment179553 hosted spawn43133 carries difficulty0 on dungeon230 and did not mirror after authentic7848 acceptance/travel. Hosted continent policy rejects530, so10058 is unsuitable. Vin54117 and Jaga54004 exist server-side but their addon89302 applies aura18 with MiscValue35. Mountain-Lord Rendan47393/display35579 exists as entity4294907172 (hosted spawn20359750, map0/difficulty0, no event/pool membership); addon82343 applies aura18 with MiscValue12. `auras/unit_state.rs::reconcile` treats every aura18 as globally Invisible and `interest::set_hidden` excludes it for every observer; no typed invisibility-detection route exists. This explains the unmirrored givers without removing authored auras, granting quest credit or changing world.db. Proper detection/quest visibility remains a product boundary, not a fixture selector issue. Missing giver texture receipts were authenticated against actual local Retail roots before publication; recovery results are under `live-asset-auth/`.

Build reproduction pins immutable shared-protocol44b2b0f, archived under the leased engine slot's target. Current protocolb452322 adds timed QuestEntrySnapshot fields absent from serverae312c3; direct builds against that newer protocol fail. Neither the shared checkout nor master was modified.

## Fixture refresh — 2026-10-14

Two master reds were fixture defects, not repaired gameplay. The current Retail template185915 has `Data1=0`, so it cannot supply the egg loot asserted by the former11049 test. The test now uses authored chest181746, giver17421, quest9548 and guaranteed item23830 (`tdb_gameobject_template.Data1=18498`, remapped content loot20018498), retaining the complete open→later autoloot→bag credit→no reroll→turn-in sequence. No SQLite rows or production loot rules changed. The item63139 test fixture also omitted `SpellTargetConditions`, required by effect89040's entry-area target selection; it now loads the same authored conditions as production rather than installing an empty substitute. Both original failures were reproduced before changing the fixture; both tests then passed. Current DB2 data follows `data/db2/active`12.1.5.70391; the census and older rendered receipts above remain historical, not a new census or live proof.

## Remaining boundaries

- Missing quest catalog coverage, slot overflow/amount differences and unsupported Types above are not fixed by this slice; they may still affect quest completion eligibility.
- GO condition/flag evaluation, professional/spell-key open casts, authored GO events/spells, visual activation/despawn and general chest group-loot rules remain incomplete. No full Retail GO parity claim.
- Hosted continents are0,1,2991; arbitrary continent530 quests cannot serve as live acceptance on this realm. Dungeon230 is a separate live boundary, not an assumed success.
- Lock companion data must be generated beside SQLite's **resolved** world.db before the server gate. Exporter3/3 covers array decoding, schema rejection, symlink placement and unchanged DB bytes. Missing companion/layout/lock definitions fail explicitly; no fallback.
