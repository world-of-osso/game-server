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
| GO quest-item loot |11049: opening exposes32506, no inventory credit until taken, no reroll, actual reward | Native server request test, including next-frame corpse-cleanup failure boundary |
| Chest metadata |Crate1586 references24985 for highlighting, not a goober acceptance gate | Native server request test |
| Personal spell credit |27955/spell94940: no creature death, capped progress, nearby party member not credited, actual reward | Real spell target-resolution test |
| Group spell credit |10566/spells37221/37320/37322/37323: matching counts, nearby/far/other-map party cases, actual reward | Real spell target-resolution test |
| Item cast |28175/item63139 → spell89040 → count and actual reward | Actual inventory UseItem request with real scaling/spell data |
| Client rendering/input |Chest/goober render admission + Interact cursor; actual UDP UseGameObject in both skins | Native engine tests2/2; rendered proof pending |

Evidence: `game-engine/data/diagnostics/quest-objectives-2026-10-11/` (RED/GREEN logs and `census.json`). No screenshot or live-turn-in claim until the corresponding rendered fixture succeeds. Live scenarios currently authored:26446/eight Fertile Dirt Mounds,7848/Core Fragment,28175/Alexstrasza's Tear; private UDP5496, disposable `fb_questobj_*`, setup level/travel only. Initial28029 scenario could not locate its rendered giver and remains an unused alternative. `game-engine/godot/tests/quest_objectives_live.gd` uses ray-picked real mouse clicks, actual loot/bag controls and quest reward dialogs. Both skins remain pending. Modern rendered startup and quest acceptance succeeded, but object lookup initially searched WorldUnits instead of direct client children (fixture corrected). The subsequent run stops at real right-drag steering: three presses produce measured camera sensitivity0.0. No successful object interaction, loot delivery, item cast or rendered turn-in is claimed. GPU concurrency is no longer the blocker. Logs: `go-modern-live-nearest.log`, `go-modern-live-parent.log`; inspected failure screenshot: `/syncthing/AgentShared/2026-10-11/quest-objectives/fail-find-unit.png`.

## Remaining boundaries

- Missing quest catalog coverage, slot overflow/amount differences and unsupported Types above are not fixed by this slice; they may still affect quest completion eligibility.
- GO condition/flag evaluation, professional/spell-key open casts, authored GO events/spells, visual activation/despawn and general chest group-loot rules remain incomplete. No full Retail GO parity claim.
- Hosted continents are0,1,2991; arbitrary continent530 quests cannot serve as live acceptance on this realm. Dungeon230 is a separate live boundary, not an assumed success.
- Lock companion data must be generated beside SQLite's **resolved** world.db before the server gate. Exporter3/3 covers array decoding, schema rejection, symlink placement and unchanged DB bytes. Missing companion/layout/lock definitions fail explicitly; no fallback.
