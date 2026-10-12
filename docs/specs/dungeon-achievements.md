# Dungeon progress and achievements

Protocol/server phases 1–2: ordered dungeon objectives, DB2-driven character achievements and bounded achievement-window data. Source: `crates/server/src/achievement.rs`, `achievement_catalog.rs`, `maps/`, and `scripted_ai/instance_script.rs`. [Implementation and provenance](../wiki/systems/dungeon-achievements.md).

## What it must do

- [x] Read DungeonEncounter map/difficulty/order/bit rows, including negative order indices; isolate defeated encounters per copy.
- [x] Successful scripted deaths record encounter progress; duplicate DONE does not credit it.
- [x] Persist scheduled instance boss/encounter masks with existing instance locks; normal-copy checkpoints survive restart without acquiring scheduled reset restrictions and clear on manual reset.
- [x] Grant Stockade achievement 633 after all three encounters, using its real Hogger criterion; deliver existing toast and snapshot to that copy's players only.
- [x] Evaluate unconditional kill-creature (0), reach-level (5), complete-quest (27), earned-achievement dependency (8), successful spell casts (29), current base skill ranks (7), spell-target hits (28), and defeat-dungeon-encounter (165) criteria from DB2.
- [x] Credit quest completion only after successful reward persistence; incomplete/rejected turn-ins never credit. Repeat reward events remain boolean, not accumulated counts.
- [x] Recover quest criteria from the character's persisted rewarded quest IDs on login; dependency criteria read persisted earned IDs and cascade newly earned meta-achievements to a fixed point. Reconnect sends snapshots without replaying earned toasts.
- [x] Match Achievement.Faction (-1 both, 0 Horde, 1 Alliance) to the player's authored faction template; exclude opposing/neutral faction-only rows from grants, catalog/criteria pages and live progress.
- [x] Separate account-wide grants with Retail ACHIEVEMENT_FLAGS_ACCOUNT (0x20000), not SHOW_CRITERIA_MEMBERS (0x10000). Aggregate durable character contributions within the owning account; migrate historical progress without changing other character rows. Never persist account totals into character counters.
- [x] Retain account earned IDs and earliest known dates across reconnect, restart and character deletion. Account criteria pages and snapshots use account aggregates; character rows use their own counters. Unrelated accounts remain isolated.
- [x] Zero-amount Single leaves still require a real positive credit and present coherent0/1→1/1 progress, never granting from a stored zero counter.
- [x] Evaluate Single/All/Any and direct-child count operators, preserving numeric progress in redb.
- [x] Grant Retail achievements at the previous 10/20/40/60/70/80 thresholds, using actual Retail IDs rather than placeholder 1–6; migrate persisted placeholders once.

### Phase 2

- [x] Import pinned local Achievement_Category FDID 1324299, category parents/order, and achievement category/icon metadata atomically with the phase-1 tables.
- [x] Send ordered encounter names and defeated state on instance entry, reconnect and each successful encounter defeat, scoped to the copy; preserve raw flags and unknown optionality.
- [x] Clear dungeon objective rows on leaving an instance, even when DungeonEncounter also contains world-boss rows for the continent.
- [x] Serve categories, category achievements and criterion lines on request with bounded pages; never push the catalog on login.
- [x] Include character earned state, known completion dates and numeric criterion progress in category responses; preserve unknown historical dates.

### Native Summary

- [ ] Serve full faction-eligible non-statistic, non-guild catalog counts and earned points, independent of client-loaded pages; distinguish All, account-flag and character-flag scopes.
- [ ] Include category totals with descendant contributions and the newest three earned rows, known dates before unknown dates. Preserve authoritative account aggregates and character isolation.
- [ ] Expose authentic Achievement.Flags with every row for native account/character and Statistics presentation.
- [ ] Resolve individual achievement IDs to authoritative own-character/account state for chat/toast navigation; reject opposing-faction rows without inventing completion data.

## How it works

- [Dungeon achievements](../wiki/systems/dungeon-achievements.md)
- [Phase-2 client hookup and payload bounds](../dungeon-achievements-client.md)
- [Boss scripts](../wiki/systems/boss-scripts.md)

## Implementation inventory

- `scripts/import_achievements.py` — pinned local DB2 CSV export and atomic SQLite import.
- `scripts/db2_casc.py` — existing WDC5 reader, extended for decrypted sections and virtual string blocks.
- `crates/server/src/achievement_catalog.rs` — supported catalog, counters, tree evaluation and grants.
- `crates/server/src/achievement_window.rs` — bounded presentation catalog and criterion continuations.
- `crates/server/src/dungeon_progress.rs` — copy entry/reconnect and defeat snapshots.
- `crates/server/src/achievement.rs` — persistent character state, gameplay events and existing message delivery.
- `crates/server/src/maps/store.rs`, `instances.rs` — ordered encounter rows and copy masks.
- `crates/server/src/scripted_ai/instance_script.rs` — successful encounter hook.

## Tests asserting this spec

- `crates/server/src/achievement_catalog_tests.rs`
- `crates/server/src/achievement_tests.rs`
- `crates/server/src/achievement_window_tests.rs` — real catalog continuation and unknown-ID behavior.
- `crates/server/src/achievement_normal_lock_tests.rs`
- `crates/server/src/scripted_ai/tests/dungeonach.rs`
- `scripts/tests/test_db2_sections.py`
- `scripts/tests/test_achievement_import.py` — real local categories and Stockade metadata.

## Known gaps (current cycle)

Specified phase-2 acceptance cases pass; see the wiki proof ledger for exact revisions, commands and retained regression evidence. No verified optional DungeonEncounter flag is available: optionality stays unknown and raw flags are preserved. Weighted/sum/max operators from phase 1 still lack dedicated acceptance cases; no new evaluation or Retail-parity claim is made.

## Out of scope

- Client objective tracker / achievement window rendering; client stays read-only in phase 2.
- ModifierTree, timed, world-state and unsupported/inverse criteria; their achievements remain disabled rather than partially evaluated.
- Exploration43 uses pinned local WorldMapOverlay→AreaTable.AreaBit joins and authoritative living, non-taxi terrain exploration, persisted per character; overlay IDs are never treated as AreaIDs. Reputation46 uses signed Retail standing and the recoverable-delta migration documented below; never compare legacy unsigned63000 to Retail42000.
- Additional criteria types beyond 0/5/7/8/27/28/29/165. Target hits credit each distinct recipient once after defense/missile filtering, including triggered spells, not the caster. Skill rows require an authored local SkillLine; loaded and changed nonzero ranks set numeric progress, never use maximum rank or increment it. Cast rows require a local spell and no DB2 conditions; TDB1210 holiday404 criteria11118–11127 remain disabled.

### Continued source-backed coverage

- Type43: pinned WorldMapOverlay areas, Retail AreaBit persistence and actual living/non-taxi terrain discovery; no overlay/area ID substitution.
- Type73: authoritative successful spell effect61 sends its authored MiscValue to the player caster. Targetless effects execute once; unit-target effects use the hit/immunity boundary. Timers, fail-event conditions, unknown modifiers and absent scripted producers remain unevaluated.
- Type39 rated arena remains unsupported: no rating producer exists. Never seed a rating to claim support.
- Type69: new authoritative aura applications credit the receiving player after successful installation; refreshes, immune/empty applications and dead-target rejection do not credit.
- Type110: each distinct unit in a resolved native spell hit credits the player caster for that spell, separate from the recipient's type28 (TC Spell::PreprocessSpellHit). Missile launch, denied target plans, depth rejection and idle ticks cannot credit; delayed spells wait for actual landing. Rows require an authored local spell and no unevaluated DB2 conditions.
- Type132 rated battleground rating has no native rating producer, matching the type39 gap. Type205 lacks authored local TransmogSet/TransmogSetGroup membership and a group-completion producer; collection IDs are not inferred.
- Types1/130: native objective terminal wins credit only accepted, authenticated winning participants still in the completed copy; draws, losses, leaving and repeated ticks do not credit. Rated rows remain unsupported.
- Type15: native battleground completion also credits each accepted, authenticated participant still in that copy, including losers and draws (TC Battleground.cpp:779). Leaving before completion and later ticks never credit; the durable counter is independent of wins. Current unconditioned rows add15 statistics, not ordinary achievements.
- Type261 ReachRenownLevel remains blocked by absent native renown amounts/levels; signed traditional standings are not renown levels.
- Type57: actual persisted Retail equipment, including initial equipped state, sets item booleans. Bags, rejected moves, display-only art and Forever70205 item IDs cannot impersonate an equipped Retail item; repeats do not add counts. EquipItemInSlot49 remains modifier-blocked in this pinned catalog, not falsely treated as any-item slot credit.
- Type67: actual committed copper from validated money slots and each member's successful group share, in u64 without narrowing the gold cap. Unopened/retaken slots, capped wallets and ordinary money changes do not credit. Existing poor-loot auto-conversion remains the separately documented loot-policy deviation.
- Type42: only actual loot-slot commits (including group-roll delivery) add the awarded item count. General acquisition36 remains separate; vendor/craft rewards and inventory moves cannot impersonate looting.
- Types152/151: a scenario run reaching its last authored step credits CompleteAnyScenario and CompleteScenario(ID) once to every player in that copy, with the copy's map as the completion map (TC Scenario.cpp:88-96). Later step events cannot re-complete it. Content boundary: MAP_SCENARIO maps such as Arena of Annihilation1031 are still rejected by InstanceManager admission, and admitted dungeon scenarios (Gnomeregan649) have no type152 achievement row, so no current playable map reaches a type152 achievement. All type151 rows need unsupported modifiers. Type92 (game event inside a MAP_SCENARIO map) has no admitted map and stays disabled.
- Quest types9/11/62/107: a committed reward adds its positive copper (62) and one completion in its QuestSortID zone (11), and sets the count of rewarded quests that TC `CanIncreaseRewardedQuestCounters` admits (9: no Dungeon Finder, daily or plain repeatable quests). Login recomputes 9 and the highest per-zone rewarded count (TC login case) without replaying money. A successful log removal adds one abandon (107); refused abandons and turn-ins credit nothing. Type257 (account quest count) is not implemented: no source defines whether it counts distinct account quests or per-character sums.
- Type260: a successful bank-tab purchase sets the purchased tab count of that bank (TC BankType Character0/Account2); login sets both current counts. Refused purchases change nothing. Type45 (legacy bank bag slots) has no producer: the bank is tab-only.
- Type40: the counter is the known skill line's maximum rank (TC GetPureMaxSkillValue) and a leaf completes at amount x 75 (TC CriteriaHandler.cpp:1278-1279). Riding762 rows (Fast and Furious and others) are structural only: riding is not a native skill line, so no gameplay reaches them.
- Type81: the counter is the summed points of earned achievements in the Pet Battles category tree (Retail Blizzard_PetCollection.lua:4,222 PET_ACHIEVEMENT_CATEGORY15117 with GetCategoryAchievementPoints including subcategories). Each grant pass recomputes it, so an earned points achievement's own points can complete the next threshold in the same pass.
- Taxi types262/108/63: a flight master interaction that newly learns its node credits LearnTaxiNode(node) once (TC TaxiHandler.cpp:146); a paid, started flight adds one flight (108) and its copper (63), refused flights credit nothing (TC Player.cpp:23289-23307). Khaz Algar/Midnight node rows are structural only: the hosted taxi network is Eastern Kingdoms.
- Duel types76/77: a duel ended by the loser's defeat credits WinDuel to the winner and LoseDuel to the loser (TC DuelComplete DUEL_WON). Leaving the duel boundary (DUEL_FLED) and cancellations credit nothing.
- Barber types48/65: a successful barber-shop application credits its positive copper (65) and the haircut boolean (48), after payment (TC CharacterHandler.cpp:1867-1879). Unaffordable requests credit nothing.
- Vendor types219/220/59: each committed vendor purchase credits BuyItemsFromVendors (219); each committed sale credits SellItemsToVendors (220) and its copper to MoneyEarnedFromSales (59) (TC Player.cpp:23902, 23949-23950). Refused purchases and buybacks credit nothing.
- Loot type109: each taken loot item also credits GetLootByType with its source's TC client loot type: creature corpse/group roll 1, disenchant 4, skinning 6, chest or gathering node 9 (TC Player::StoreLootItem, Player.cpp:27203). Untaken or refused slots credit nothing. Fishing (3) has no producer.
- Account Mount Journal ownership counts as known spells for types34/75/112 on every character (TC CollectionMgr::AddMount LearnSpells each owned mount at login and on learning). Mount items now teach through the journal, which enforces the item's authored race/class and level.
- Type68: native successful goober/chest use and Font-of-Power activation credit actual authored GameObject entry after reach, life, map, lock, quest/cooldown and qualified-font validation. Requests and denied uses cannot credit. Portal/flag/minigame sources not owned by these handlers remain unclaimed.
- Types160/162: actual XP-driven transitions of existing owned pets after native terminal reward persistence. Each crossed authored level receives one credit, matching the PetLevel asset; stale/repeated rounds, seeded initial levels and newly captured pets are not treated as raised levels. Missing CreatureID joins and unsupported modifiers do not credit. Current PvP battles award no XP.
- Type14: committed rewards of quests whose authenticated template has DAILY, using the repeatability captured during reward validation. Failed/incomplete/repeated-before-reset rewards do not count; login never infers historical daily counts from ordinary quest history.
- Types185/186: authenticated account Toy Box ownership, learned through actual UseItem and atomic item/ownership persistence. LearnToy assets are real Toy.ItemID values, not Toy.ID; no absent local toy join is enabled. Re-entry reconstructs known items/count without replay increments. Account aggregation reads the owning toy table, not stale character copies; favourites do not change ownership counts.
- Type156: unique owned species from the authenticated account PetJournal, not pet instances or cosmetic defaults. Account aggregation reads that journal in the same write transaction and excludes stale per-character copies of this global quantity; no old unsupported counter can substitute for ownership. Known-state journal changes update the character view; already earned IDs stay earned.
- Type157: native terminal trap rewards credit one AccountObtainPetThroughBattle only after the pet journal commit. Rejected/stale actions and repeated ticks cannot replay capture; arbitrary species/quality conditions remain disabled.
- Type96: account PetJournal ownership resolves BattlePetSpecies to its authentic CreatureID asset; cosmetic default known flags never credit and duplicate owned pets remain boolean1. Missing species/creature joins remain disabled.
- Type34/75 mount learning: an actual zero-trigger LEARN_SPELL item cast learns its authenticated ItemEffect ON_LEARN spells, preserving the consumed casting item's identity through the cast snapshot. Buying/owning a mount item alone never substitutes for KnownSpells. No fabricated collection flags or spell IDs.
- Type41: actual Retail item-use casts credit once after successful execution checks, including delayed casts; missing items, cooldown/combat refusals and cancelled casts do not credit. Companion learning credits only after journal/item persistence; custom item IDs never impersonate Retail IDs. TDB1210 orphan/holiday criteria11134–11140 remain disabled.
- Type21 CompleteInternalCriteria and44 EarnTitle are deprecated/internal criteria (pinned TC DBCEnums.h570/593), not quest counts or equipment. No internal designer/title producer exists;54 has no unconditional executable achievement gain and needs target/data conditions.
- Types75/112: actual KnownSpells and authenticated SkillLineAbility joins determine distinct known-spell counts per skill line, recomputed on change. First zero rows are not created (TC SetCriteriaProgress); this is not seeded mount/pet-collection inference. Account aggregation retains the existing maximum-of-character-facts policy.
- Types16/17/20: accepted native player death transitions accumulate DieOnMap(current authoritative map), DieAnywhere, and the actual last creature killer entry (TC Player.cpp1162-1164/Unit.cpp11586). Healthy/initial state, duplicate transitions, idle ticks and repeated damage to an already dead player do not credit. Type18 has no unconditional rows and remains modifier-blocked. New ordinary61917/criterion111356 remains a named gap: pinned TC explicitly makes type20 statistic-only; no newer completion semantics are invented.
- Types245/246: a successful native order settlement credits the crafter once after atomic output/commission-mail and order-status persistence, never on posting, claiming, cast admission, cancellation or stale retry. Type246 assets use Retail CraftingOrderType Public0/Guild1/Personal2 from ProfessionConstantsDocumentation.lua; NPC3 has no native producer. Pinned TC DBCEnums names245 FulfillAnyCraftingOrder and246 FulfillCraftingOrderType; pinned DB2 leaves supply count thresholds.
- Type70 needs the native honor-eligible reward boundary (TC Player.cpp6758), not any player death; no honor-eligibility producer exists. Type30 has no WorldStateUI change producer;37 has no rated-arena result producer;198/173 have no garrison talent/mission producers. These remain explicit gaps.
