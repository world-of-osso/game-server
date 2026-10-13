# Retail achievement criteria producer inventory

verified: 2026-10-13

Scope: all243 distinct Types in the pinned69933 Criteria.csv. Raw support is NOT a promise that every conditional row, actor, timer, world-state or current achievement tree is executable. Current structural all-faction player union: **5417 ordinary (3428 account/1989 character),1804 statistics; 99 enabled types**. Spec/producer contracts: [dungeon achievements](specs/dungeon-achievements.md).

## Current-tree wiring

- Plain Some8 requiring one of exactly one child (Amount0/1, TC1738–1750) reuses the existing All child evaluator; it does not discard that child. Native per-request Love225/Wave101 over controlled server units proves wrong/foreign/player/absent-target rejection, target68751/3236, all18 slime-selector account40475, idle and reopened counters. IDs are authenticated from pinned criteria; newer NPC definitions/spawn/model closure remain unclaimed. +40475 account/+1065 statistic; general multi-child non-difficulty OR, higher amounts and unknown leaves remain disabled.

- Plain empty All4 (TC CriteriaHandler1733–1737) is true; empty Some8 and SingleTrue2 remain false. Compiler now admits its seven cast rows61781–61787 for18905. Native triggered127117–127123 casts store all seven real goggles and grant the account achievement; refused depth, idle, redb reopen/relogin are covered. Private fixture adds exact existing Item/ItemSparse projections77533–77539 using content_policy.ITEM_COLUMNS; canonical content_item lacks these rows and was not changed. +1 structural account achievement, not production crafting/UI or data-closure certification.

- PlayerLevelEqual39 (TC CriteriaHandler1956–1959) lowers to existing inclusive minimum69 and maximum71 on the same level. Native Unit::Kill at levels1/2/3 proves exact-level orphan death counters16188/16402, lower/higher rejection, dead-hit replay safety and redb reopen. +24 supported raw orphan rows (one reach-level5 row, seven quest-count9 rows, sixteen death17 rows); unsupported title/internal21 remains disabled. No new current-root achievement/statistic and no orphan relinking; census unchanged.

- DieInInstance18 (TC Player1164, CriteriaHandler1481–1493): accepted native player death uses authored IsDungeon and the allocated copy's actual admission capacity, never current occupancy or a difficulty/map-name guess. Native Unit::Kill RED/GREEN proves party5/raid10/raid25 capacity and modifier isolation, continent/nonexistent-copy rejection, duplicate dead hits/transitions and reopened persistence. +3 statistics14784/14787/40748 structurally, no ordinary gain; delves content is not certified by the party/raid fixture.

- PlayerIsNotInParty35 (TC CriteriaHandler1940–1943) admits pinned zero-asset SingleTrue leaves only when native GroupRegistry and character identity establish an empty roster. Missing authority is not solo; offline roster members still make the character grouped. Native actual280339 self-target marker cast RED/GREEN earns41658/13026 only for known solo, rejects unknown registry/identity, depth refusal and a real joined party, and persists through idle/redb reopen. +2 character definitions13025/13026 structurally; the Nazmir quest's timed invader lifecycle and marker trigger are not certified by this cast fixture.

- PlayerIsInZone41 (TC CriteriaHandler1964–1973) compiles exact area membership through native AreaTable::zone_id: one immediate parent only for a flagged subzone, never recursive ancestry or ParentAreaID alone. Real terrain at Stormwind coordinates and native terminal pet-battle win credit19845, reject other-city/loser counters, preserve idle/reopened persistence. Catalog behavior proves top-level/flagged membership, absent area, unflagged-parent rejection and two-level rejection. +6 account achievements603/604/6030/6031/6584/6621 structurally; world-honor city kills, firework effects and complete city battle coverage remain unclaimed.

- PlayerMapInstanceType122 (TC CriteriaHandler2394–2397) compiles exact raw Map.InstanceType membership at catalog load; event-time reference map selects it. World0 is valid, missing/out-of-range types reject. Native arena/skirmish/rated2/3 and BG effect/death RED/GREEN proves arena50456/50457, BG50458 and actual queued BG death50460, cross-type rejection, idle/reopen. +4 statistics (383/1490/1491/14786); no ordinary gains. Instance-death type18 now has the allocated-capacity producer described above. Test-only death counter visibility allows the fixture to execute the actual death system, not synthesize criteria credit.

- Arena player predicates24/60 now read native PlayerPvpState→accepted/current-copy arena ledger at each event, not only a rated-result event payload. Team size uses native mode (2/3); ranked uses the actual RatedMatch presence, so skirmish and current unrated Shuffle cannot impersonate it. All gated producer types can use these leaves; structural/account-recomputed types remain excluded. Real-channel rated2/3 and skirmish2/3 RED/GREEN proves size-isolated5541/5542 killing blows,4962/4963 deaths and4959/4960 native kill rewards, with idle/reopen. +5 statistics (1110/1111/1492/1493/14785), no ordinary gains. Size5/unknown trees remain disabled.

- KillPlayer70 BG/arena path (TC Battleground1354–1383; Arena::HandleKillPlayer delegates to it): a committed native lethal damage event resolves the controlling player, accepted active match, killer and same-team recipients. Killer ignores reward distance; others use corpse position if dead and TC GetDistance (Object430–433), subtracting victim/player combat reach (Player1574–1577, default1.5×scale; corpse0), within native group reward distance74. Native effect/death RED/GREEN proves untimed orphan26004, boundary77/75.5, far/other-copy/unaccepted recipients, no nonlethal/dead/idle replay, self/world/waiting/ended rejection and reopen. Timed11491 and missing-aura20124 remain uncredited. +4 ordinary (1723/6973/13623/17852), structural only; no current-root grant/playability proof. Separate world honor path, lifetime honor113 and scoreboard honorable-kill availability remain unwired.

- LessThanTappers29 (TC CriteriaHandler.cpp1902–1905) compares the reference player's complete group roster, not physical attackers or nearby/online members. Native GroupRegistry supplies the event-time count (known solo0); unknown group authority or absent reference player cannot qualify. Authored Malygos616/10-player copy3 native defeat grants1869/criterion7174 for solo or roster8, rejects roster9 and25-player7175; offline roster members count, and no replay/redb reopen hold. +8 ordinary, no statistics; full Malygos mechanics/rendering unclaimed. The complete raw matrix below is reconciled with this census, not older batch counts.
- Further audit: ModifierTree81/89/90 are hard-false NYI in pinned TC CriteriaHandler2179–2180/2209–2211, despite native PvP pet battles; no enum-name qualification inference. Modifier84 candidate Monster Mash9600 names quest36860, absent from the local quest/questgiver catalog; no arbitrary accepted-quest backfill. Guild-reputation62 rows in this corpus have only guild actors, which remain excluded from player grants.

- WinArena32 / ParticipateInArena33 (TC Arena.cpp241–250): successful once-only native rated settlement credits every online in-world participant and map-specific winners, after the result's applied marker. Arena team-size24/ranked60 now read the current accepted native arena as described above. Native 2v2/3v3 real-channel RED/GREEN covers winner/loser participation, map/team-size isolation, no queue/preparation/skirmish credit, idle replay and redb reopen. No native5v5 or inferred slot-to-team-size rating39 mapping. Census +World Wide Winner699 / +40 statistics, structural only.

- DoEmote54 TargetCreatureId4 (TC ChatHandler703–753; CriteriaHandler1782–1785): each EmoteIntent snapshots the selected server entity; native receipt resolves an NPC on the sender's map copy and publishes its template entry with that request's TextEmote ID. All4/SingleTrue target-entry leaves are compiled only for type54; other producers have no target snapshot and remain disabled. Love225 has no animation (pinned69933 EmotesText EmoteID0). Native RED/GREEN covers two different targets queued together, wrong emote, targetless/player/forged/stale/foreign-map targets, world-entry eligibility, idle replay and redb reopen. Census +5 account achievements:1206/2557/5548/5865/16729; structural only, not spawn/model completeness. Other target modifiers remain unsupported.

- ModifierTree DungeonDifficulty68 (TC CriteriaHandler2090–2093) and LegacyDungeonDifficulty20 (1860–1865, Difficulty.db2 OldEnumValue), operators1723–1751: 3085 criteria whose only condition is an All/SingleTrue, All/SingleFalse or Some(≤1) tree of these leaves are compiled (now `achievement_conditions.rs`) and checked against the player's current copy Difficulty (continents0) at event time; 68 added +460 ordinary/+1226 statistics, 20 added +129/+222.
- Player-only ModifierTree leaves (TC CriteriaHandler: PlayerHasAura8 1798, PlayerIsInArea17 1842, PlayerRace25 1886, PlayerClass26 1890, PlayerMap32 1922, PlayerLevel≥69 2094, PlayerLevel≤71 2102): 1253 criteria whose only condition is All4 of SingleTrue leaves of these types are compiled (now `achievement_conditions.rs`) and checked against the reference player at the update; a leaf naming an unknown spell/area/map, a non-playable race, class outside1–13 or a min level above the cap can never hold and leaves the row disabled. Aura batch +113 ordinary/+1 statistic; area/map/race/level batch +77/+43 (battleground killing-blow statistics, Dynamic Flight Power, dungeon unlocks). Types8/81/156/185/186 and single-modifier family rows (221) are excluded. Structural only: most aura rows need auras with no native producer (WoW Remix424143 gates 80 achievements, Heroic World Tier1238465, Polymorphic Translation456122, vehicle/quest auras).
- Mixed trees (`scripts/import_achievement_conditions.py`, one compiler for both families): All4 roots whose children are player-fact SingleTrue leaves, difficulty SingleTrue/SingleFalse leaves or a nested Some(≤1) of difficulty SingleTrue leaves; difficulty parts combine as (∩ allowed) − (∪ excluded). 4491 criteria compiled; +41 ordinary (e.g. heroic dungeon achievements gated by a quest aura). Ungated types (8/81/156/185/186) are now also excluded from difficulty-only trees: type8 rows are evaluated structurally and could never honor the difficulty.
- PlayerMeetsCondition2 (TC CriteriaHandler1768, ConditionMgr2903–2959): reuse the native spellcaster identity evaluator for class/race masks only; flags other than ClientExecutable, any non-identity field, missing conditions and TDB source34 rules reject the entire row. Compiler4515→4717; +8 ordinary, including actual Human/Orc XP-cap Double Agent7380. This is not a general PlayerCondition evaluator; levels/content tuning, quests, auras and nested ModifierTreeID remain unsupported.
- HasTimeEventPassed289 (TC CriteriaHandler3578–3611): SingleTrue leaves on IDs TC does not timestamp default to the current time and always hold, so the compiler drops them (+2 ordinary). Timestamped IDs and season windows (A passed AND NOT B passed, where TC's unknown B always passed) stay disabled because TC can never satisfy them.
- Types196/221/253 are earned native XP transitions, not login/admin facts. Thirteen exact current221 class leaves now admit7382/7383/7384/19489; native Paladin+Mage transitions earn7382 and survive reopened account login.
- Types13/54/55/102–106 and the new unconditional rank/area/level counters do not gain an Achievement merely because a raw CriteriaTree references them. Census follows actual Achievement roots; orphan raw rows are retained for non-achievement consumers, never relinked.
- Damage Control227 is still blocked by battleground start/fail lifetime conditions and zone/target predicates, not missing raw damage/healing counters. Unknown conditions cannot be stripped.
- Both-skin commerce/peer proof remains at the exact criteria17 revisions in the handoff. Later batches have revision-scoped native/CPU gates there, not a new rendered-client claim.
- Independent subscription verifier failed OAuth refresh at this checkpoint; no paid fallback used. Whole goal remains open.

## Complete raw-type matrix

Counts: supported raw / raw; supported current-root rows / all current-root rows. Orphans are supported raw rows outside all Achievement roots. A declared producer may have zero eligible rows. Primary names use independently refetched TC a352b1fa DBCEnums.h (SHA256 a00f6c7d8c771e91cb0dfad37b87ade5f9f9e74f960ccad47426029bd8f7f4e1). The enum ends at its standalone closing line, not the inline `{FactionID};` comment at243. Types245–272 are therefore not mislabeled absent. Missing primary names stay unknown instead of being invented. Source/qualification gaps are OPEN, not completion claims.

| Type | Pinned source name | Raw | Current roots | Orphans | State / producer evidence |
|---:|---|---:|---:|---:|---|
| 0 | KillCreature | 7377/10154 | 3475/4428 | 3902 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 1 | WinBattleground | 27/212 | 25/160 | 2 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 3 | CompleteAnyResearchProject | 0/12 | 0/12 | 0 | Producer gap: no archaeology/research runtime; enum names do not authorize synthetic progress. |
| 4 | FindResearchObject | 0/12 | 0/0 | 0 | Producer gap: no archaeology/research runtime; enum names do not authorize synthetic progress. |
| 5 | ReachLevel | 65/270 | 17/196 | 48 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 6 | ExhaustAnyResearchSite | 0/4 | 0/3 | 0 | Producer gap: no archaeology/research runtime; enum names do not authorize synthetic progress. |
| 7 | SkillRaised | 190/375 | 190/363 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 8 | EarnAchievement | 5618/5959 | 5146/5414 | 472 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 9 | CompleteQuestsCount | 8/13 | 1/1 | 7 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 10 | CompleteAnyDailyQuestPerDay | 0/2 | 0/1 | 0 | Condition gap: every raw row has an unsupported qualifier; native candidate does not authorize dropping conditions. |
| 11 | CompleteQuestsInZone | 149/641 | 59/59 | 90 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 12 | CurrencyGained | 127/337 | 64/204 | 63 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 13 | DamageDealt | 1/18 | 0/15 | 1 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 14 | CompleteDailyQuest | 13/34 | 13/16 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 15 | ParticipateInBattleground | 31/84 | 25/67 | 6 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 16 | DieOnMap | 24/24 | 0/0 | 24 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 17 | DieAnywhere | 22/44 | 5/11 | 17 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 18 | DieInInstance | 10/12 | 8/9 | 2 | Native accepted death with authored IsDungeon and allocated-copy capacity; difficulty/map-type qualifiers remain conjunctive. Party5/raid10/25 and persistence proved; delve content unclaimed. |
| 19 | RunInstance | 0/3 | 0/3 | 0 | Source/qualification gap: requires exact producer boundary or progress rule not established by current native criterion implementation; no enum-name inference. |
| 20 | KilledByCreature | 167/168 | 0/1 | 167 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 21 | CompleteInternalCriteria | 0/376 | 0/68 | 0 | Source/producer gap: deprecated/internal criterion or title grant; no designer/title ownership producer. |
| 22 | CompleteAnyChallengeMode | 0/19 | 0/9 | 0 | Source/qualification gap: requires exact producer boundary or progress rule not established by current native criterion implementation; no enum-name inference. |
| 23 | KilledByPlayer | 1/1 | 1/1 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 24 | MaxDistFallenWithoutDying | 0/3 | 0/3 | 0 | Condition gap: every raw row has an unsupported qualifier; native candidate does not authorize dropping conditions. |
| 26 | DieFromEnviromentalDamage | 4/6 | 4/6 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 27 | CompleteQuest | 12739/14115 | 9505/9889 | 3234 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 28 | BeSpellTarget | 855/1839 | 564/1290 | 291 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 29 | CastSpell | 2507/2972 | 1478/1657 | 1029 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 30 | TrackedWorldStateUIModified | 16/87 | 16/78 | 0 | Native: every battleground objective credit (TC BattlegroundScore::UpdatePvpStat:78-80) records +1 for the connected character, completed in the match map; only unconditional rows whose PvpStat is a native column. BG start/fail-event rows stay disabled. Native completion-map gate leaves 199/200 (Instance_ID489) unearned on WSG2106. See spec. |
| 31 | PVPKillInArea | 0/51 | 0/45 | 0 | Producer/state gap: required honor eligibility, team/rated-BG rating or honor/prestige/solo-shuffle progression is unwired; native rated-arena results do not supply those facts. |
| 32 | WinArena | 57/82 | 56/62 | 1 | Native committed rated winners: exact match map; team-size24(2/3) and ranked60 qualified.5v5/unknown qualifiers remain disabled. |
| 33 | ParticipateInArena | 55/87 | 55/61 | 0 | Native committed rated participants, including losers: exact match map; team-size24(2/3) and ranked60 qualified.5v5/unknown qualifiers remain disabled. |
| 34 | LearnOrKnowSpell | 530/578 | 441/467 | 89 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 35 | EarnHonorableKill | 0/62 | 0/52 | 0 | Producer/state gap: required honor eligibility, team/rated-BG rating or honor/prestige/solo-shuffle progression is unwired; native rated-arena results do not supply those facts. |
| 36 | AcquireItem | 1650/1759 | 1123/1132 | 527 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 37 | WinAnyRankedArena | 4/60 | 1/34 | 3 | Native rated2v2/3v3 committed settlement: authenticated online winners only; pre-result rating minimum1 (TC Arena.cpp249), one accumulated win. Real-channel rated wins/skirmish/losers/no-replay/redb-reopen proved. Timers and unsupported qualifiers stay disabled. |
| 38 | EarnTeamArenaRating | 0/6 | 0/3 | 0 | Producer/state gap: required honor eligibility, team/rated-BG rating or honor/prestige/solo-shuffle progression is unwired; native rated-arena results do not supply those facts. |
| 39 | EarnPersonalArenaRating | 0/88 | 0/68 | 0 | Native rated settlement exists, but source/data mapping unresolved: pinned TC ArenaTeam.cpp448 passes team size2/3/5 while local Retail criteria447/451/452 use slot-like assets1/0/2. No guessed remap; still unwired. No native rated5v5. |
| 40 | AchieveSkillStep | 12/36 | 12/12 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 41 | UseItem | 1309/1404 | 1215/1231 | 94 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 42 | LootItem | 388/579 | 292/292 | 96 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 43 | RevealWorldMapOverlay | 1728/1932 | 1724/1779 | 4 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 44 | EarnTitle | 0/14 | 0/14 | 0 | Source/producer gap: deprecated/internal criterion or title grant; no designer/title ownership producer. |
| 45 | BankSlotsPurchased | 0/2 | 0/1 | 0 | Producer gap: native banks buy tabs, not legacy bank bag slots. |
| 46 | ReputationGained | 272/387 | 233/342 | 39 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 47 | TotalExaltedFactions | 1/2 | 0/0 | 1 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 48 | GotHaircut | 1/2 | 1/1 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 49 | EquipItemInSlot | 0/433 | 0/113 | 0 | Source/data mismatch: TC Player.cpp11638/11766 sends slot as miscValue1 and itemID as miscValue2, but ModifierTree3/14/15 read miscValue1 as itemID. Local rows use those qualifiers. No invented argument remap or dropped conditions. |
| 50 | RollNeed | 0/2 | 0/1 | 0 | Condition gap: every raw row has an unsupported qualifier; native candidate does not authorize dropping conditions. |
| 51 | RollGreed | 0/2 | 0/1 | 0 | Condition gap: every raw row has an unsupported qualifier; native candidate does not authorize dropping conditions. |
| 52 | DeliverKillingBlowToClass | 0/13 | 0/13 | 0 | Producer/state gap: required honor eligibility, team/rated-BG rating or honor/prestige/solo-shuffle progression is unwired; native rated-arena results do not supply those facts. |
| 53 | DeliverKillingBlowToRace | 0/149 | 0/148 | 0 | Producer/state gap: required honor eligibility, team/rated-BG rating or honor/prestige/solo-shuffle progression is unwired; native rated-arena results do not supply those facts. |
| 54 | DoEmote | 195/398 | 181/254 | 14 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 55 | HealingDone | 1/16 | 0/15 | 1 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 56 | DeliveredKillingBlow | 35/80 | 20/63 | 15 | Native: authenticated in-world killer on actual lethal health commits, plus genuine HunterPet owner credit (TC Unit::Kill); dead replay/self raw KillUnit idle. See spec. |
| 57 | EquipItem | 162/163 | 11/11 | 151 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 58 | CompleteQuestsInSort | 0/3 | 0/0 | 0 | Source/qualification gap: requires exact producer boundary or progress rule not established by current native criterion implementation; no enum-name inference. |
| 59 | MoneyEarnedFromSales | 1/1 | 1/1 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 60 | MoneySpentOnRespecs | 0/1 | 0/0 | 0 | Producer gap: current trait commits do not execute legacy paid Player::ResetTalents; no respec fee/reset settlement. |
| 61 | TotalRespecs | 0/1 | 0/0 | 0 | Producer gap: current trait commits do not execute legacy paid Player::ResetTalents; no respec fee/reset settlement. |
| 62 | MoneyEarnedFromQuesting | 1/1 | 1/1 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 63 | MoneySpentOnTaxis | 1/2 | 1/1 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 64 | KilledAllUnitsInSpawnRegion | 0/75 | 0/0 | 0 | Source/qualification gap: requires exact producer boundary or progress rule not established by current native criterion implementation; no enum-name inference. |
| 65 | MoneySpentAtBarberShop | 1/1 | 1/1 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 66 | MoneySpentOnPostage | 1/1 | 1/1 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 67 | MoneyLootedFromCreatures | 1/2 | 1/2 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 68 | UseGameobject | 526/578 | 381/383 | 145 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 69 | GainAura | 542/760 | 93/225 | 449 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 70 | KillPlayer | 42/180 | 25/102 | 17 | Native BG/arena lethal kill rewards: accepted active match, killer and same-team reward-distance/corpse recipients. Separate world honor eligibility is unwired; no lifetime/scoreboard honor inference. Qualifiers stay conjunctive. |
| 71 | CompleteChallengeMode | 0/244 | 0/120 | 0 | Source/qualification gap: requires exact producer boundary or progress rule not established by current native criterion implementation; no enum-name inference. |
| 72 | CatchFishInFishingHole | 0/38 | 0/36 | 0 | Producer gap: no fishing pool/catch settlement (toy transmitted-object placeholders are not fishing). |
| 73 | PlayerTriggerGameEvent | 230/804 | 74/587 | 156 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 74 | Login | 12/462 | 11/372 | 1 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 75 | LearnSpellFromSkillLine | 12/16 | 2/2 | 10 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 76 | WinDuel | 5/27 | 3/16 | 2 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 77 | LoseDuel | 1/1 | 1/1 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 78 | KillAnyCreature | 3/112 | 1/21 | 2 | Native: TC KillRewarder:185 via shared TapRewardRecipients (map copy, range, group, corpse eligibility). See spec. |
| 79 | CreatedItemsByCastingSpellWithLimit | 0/5 | 0/5 | 0 | Source/qualification gap: requires exact producer boundary or progress rule not established by current native criterion implementation; no enum-name inference. |
| 80 | MoneyEarnedFromAuctions | 1/2 | 1/1 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 81 | BattlePetAchievementPointsEarned | 1/1 | 1/1 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 82 | ItemsPostedAtAuction | 1/1 | 1/1 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 83 | HighestAuctionBid | 1/1 | 1/1 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 84 | AuctionsWon | 1/1 | 1/1 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 85 | HighestAuctionSale | 1/1 | 1/1 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 86 | MostMoneyOwned | 1/1 | 1/1 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 87 | TotalReveredFactions | 1/1 | 0/0 | 1 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 88 | TotalHonoredFactions | 1/1 | 0/0 | 1 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 89 | TotalFactionsEncountered | 0/1 | 0/0 | 0 | State gap: CharacterReputation.tracked is player selection, not Retail Visible flags; initialized435 rows are not encountered435. |
| 90 | LootAnyItem | 0/5 | 0/2 | 0 | Condition gap: every raw row has an unsupported qualifier; native candidate does not authorize dropping conditions. |
| 91 | ObtainAnyItem | 0/25 | 0/3 | 0 | Condition gap: every raw row has an unsupported qualifier; native candidate does not authorize dropping conditions. |
| 92 | AnyoneTriggerGameEventScenario | 0/3642 | 0/12 | 0 | Source/qualification gap: requires exact producer boundary or progress rule not established by current native criterion implementation; no enum-name inference. |
| 93 | RollAnyNeed | 0/1 | 0/0 | 0 | Condition gap: every raw row has an unsupported qualifier; native candidate does not authorize dropping conditions. |
| 94 | RollAnyGreed | 0/1 | 0/0 | 0 | Condition gap: every raw row has an unsupported qualifier; native candidate does not authorize dropping conditions. |
| 95 | ReleasedSpirit | 0/1 | 0/0 | 0 | Source semantics gap: native candidate event/state exists, but pinned evaluator supplies no executable progress rule for this raw type; do not infer count/reset/asset semantics. |
| 96 | AccountKnownPet | 132/136 | 126/127 | 6 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 97 | DefeatDungeonEncounterWhileElegibleForLoot | 0/382 | 0/0 | 0 | Source/qualification gap: requires exact producer boundary or progress rule not established by current native criterion implementation; no enum-name inference. |
| 101 | HighestDamageDone | 0/1 | 0/0 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 102 | HighestDamageTaken | 1/1 | 0/0 | 1 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 103 | TotalDamageTaken | 1/1 | 0/0 | 1 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 104 | HighestHealCast | 1/1 | 0/0 | 1 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 105 | TotalHealReceived | 1/1 | 0/0 | 1 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 106 | HighestHealReceived | 1/1 | 0/0 | 1 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 107 | AbandonAnyQuest | 1/1 | 1/1 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 108 | BuyTaxi | 1/1 | 1/1 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 109 | GetLootByType | 1/78 | 1/7 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 110 | LandTargetedSpellOnTarget | 2/159 | 1/121 | 1 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 112 | LearnTradeskillSkillLine | 19/20 | 18/19 | 1 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 113 | HonorableKills | 0/4 | 0/1 | 0 | Producer/state gap: required honor eligibility, team/rated-BG rating or honor/prestige/solo-shuffle progression is unwired; native rated-arena results do not supply those facts. |
| 114 | AcceptSummon | 1/1 | 1/1 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 115 | EarnAchievementPoints | 0/8 | 0/1 | 0 | Condition gap: every raw row has an unsupported qualifier; native candidate does not authorize dropping conditions. |
| 118 | CompletedLFGDungeon | 0/7 | 0/0 | 0 | Producer gap: lfg.rs stores role checks/match_found only, not an LFG/LFR instance completion, stranger roster or kick/abandon settlement. |
| 119 | CompletedLFGDungeonWithStrangers | 0/2 | 0/1 | 0 | Producer gap: lfg.rs stores role checks/match_found only, not an LFG/LFR instance completion, stranger roster or kick/abandon settlement. |
| 120 | KickInitiatorInLFGDungeon | 0/1 | 0/0 | 0 | Producer gap: lfg.rs stores role checks/match_found only, not an LFG/LFR instance completion, stranger roster or kick/abandon settlement. |
| 121 | KickVoterInLFGDungeon | 0/1 | 0/0 | 0 | Producer gap: lfg.rs stores role checks/match_found only, not an LFG/LFR instance completion, stranger roster or kick/abandon settlement. |
| 122 | KickTargetInLFGDungeon | 0/1 | 0/0 | 0 | Producer gap: lfg.rs stores role checks/match_found only, not an LFG/LFR instance completion, stranger roster or kick/abandon settlement. |
| 123 | AbandonedLFGDungeon | 0/1 | 0/0 | 0 | Producer gap: lfg.rs stores role checks/match_found only, not an LFG/LFR instance completion, stranger roster or kick/abandon settlement. |
| 124 | MoneySpentOnGuildRepair | 0/1 | 0/1 | 0 | Actor gap: player/account achievements cannot impersonate Guild progress; no Guild criteria aggregation. |
| 125 | GuildAttainedLevel | 0/1 | 0/1 | 0 | Actor gap: player/account achievements cannot impersonate Guild progress; no Guild criteria aggregation. |
| 126 | CreatedItemsByCastingSpell | 0/15 | 0/10 | 0 | Condition gap: every raw row has an unsupported qualifier; native candidate does not authorize dropping conditions. |
| 127 | FishInAnyPool | 0/4 | 0/3 | 0 | Producer gap: no fishing pool/catch settlement (toy transmitted-object placeholders are not fishing). |
| 128 | GuildBankTabsPurchased | 0/1 | 0/1 | 0 | Actor gap: player/account achievements cannot impersonate Guild progress; no Guild criteria aggregation. |
| 129 | EarnGuildAchievementPoints | 0/1 | 0/1 | 0 | Actor gap: player/account achievements cannot impersonate Guild progress; no Guild criteria aggregation. |
| 130 | WinAnyBattleground | 17/75 | 0/35 | 17 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 131 | ParticipateInAnyBattleground | 0/11 | 0/1 | 0 | Source semantics gap: pinned enum marks NYI and TC UpdateCriteria has no executable progress case (default Not implemented at877). No inferred participation/start/completion policy. |
| 132 | EarnBattlegroundRating | 0/36 | 0/22 | 0 | Producer/state gap: required honor eligibility, team/rated-BG rating or honor/prestige/solo-shuffle progression is unwired; native rated-arena results do not supply those facts. |
| 133 | GuildTabardCreated | 0/1 | 0/1 | 0 | Actor gap: player/account achievements cannot impersonate Guild progress; no Guild criteria aggregation. |
| 134 | CompleteQuestsCountForGuild | 0/1 | 0/1 | 0 | Actor gap: player/account achievements cannot impersonate Guild progress; no Guild criteria aggregation. |
| 135 | HonorableKillsForGuild | 0/1 | 0/1 | 0 | Actor gap: player/account achievements cannot impersonate Guild progress; no Guild criteria aggregation. |
| 136 | KillAnyCreatureForGuild | 0/1 | 0/1 | 0 | Actor gap: player/account achievements cannot impersonate Guild progress; no Guild criteria aggregation. |
| 137 | GroupedTankLeftEarlyInLFGDungeon | 0/1 | 0/0 | 0 | Producer gap: lfg.rs stores role checks/match_found only, not an LFG/LFR instance completion, stranger roster or kick/abandon settlement. |
| 138 | CompleteGuildChallenge | 0/6 | 0/5 | 0 | Actor gap: player/account achievements cannot impersonate Guild progress; no Guild criteria aggregation. |
| 139 | CompleteAnyGuildChallenge | 0/1 | 0/1 | 0 | Actor gap: player/account achievements cannot impersonate Guild progress; no Guild criteria aggregation. |
| 140 | MarkedAFKInBattleground | 0/1 | 0/0 | 0 | Producer gap: no battleground AFK-report/removal authority. |
| 141 | RemovedAFKInBattleground | 0/1 | 0/0 | 0 | Producer gap: no battleground AFK-report/removal authority. |
| 142 | StartAnyBattleground | 0/1 | 0/0 | 0 | Source semantics gap: pinned enum marks NYI and TC UpdateCriteria has no executable progress case (default Not implemented at877). No inferred participation/start/completion policy. |
| 143 | CompleteAnyBattleground | 0/6 | 0/0 | 0 | Source semantics gap: pinned enum marks NYI and TC UpdateCriteria has no executable progress case (default Not implemented at877). No inferred participation/start/completion policy. |
| 144 | MarkedSomeoneAFKInBattleground | 0/1 | 0/0 | 0 | Producer gap: no battleground AFK-report/removal authority. |
| 145 | CompletedLFRDungeon | 0/2 | 0/0 | 0 | Producer gap: lfg.rs stores role checks/match_found only, not an LFG/LFR instance completion, stranger roster or kick/abandon settlement. |
| 146 | AbandonedLFRDungeon | 0/1 | 0/0 | 0 | Producer gap: lfg.rs stores role checks/match_found only, not an LFG/LFR instance completion, stranger roster or kick/abandon settlement. |
| 147 | KickInitiatorInLFRDungeon | 0/1 | 0/0 | 0 | Producer gap: lfg.rs stores role checks/match_found only, not an LFG/LFR instance completion, stranger roster or kick/abandon settlement. |
| 148 | KickVoterInLFRDungeon | 0/1 | 0/0 | 0 | Producer gap: lfg.rs stores role checks/match_found only, not an LFG/LFR instance completion, stranger roster or kick/abandon settlement. |
| 149 | KickTargetInLFRDungeon | 0/1 | 0/0 | 0 | Producer gap: lfg.rs stores role checks/match_found only, not an LFG/LFR instance completion, stranger roster or kick/abandon settlement. |
| 150 | GroupedTankLeftEarlyInLFRDungeon | 0/1 | 0/0 | 0 | Producer gap: lfg.rs stores role checks/match_found only, not an LFG/LFR instance completion, stranger roster or kick/abandon settlement. |
| 151 | CompleteAnyScenario | 6/8 | 1/2 | 5 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 152 | CompleteScenario | 933/1070 | 78/139 | 855 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 153 | EnterAreaTriggerWithActionSet | 0/33 | 0/8 | 0 | Source/qualification gap: requires exact producer boundary or progress rule not established by current native criterion implementation; no enum-name inference. |
| 155 | LearnedNewPet | 127/127 | 122/122 | 5 | Native: battle_pets::learn_companion_from_item and wild capture credit species after the new-GUID journal commit (TC BattlePetMgr::AddPet:483); compiled AND4→BattlePetSpecies91 leaves only. Local item sources exist for species1928/1929 only; Coiled Isle wild species lack battle abilities. See spec. |
| 156 | UniquePetsOwned | 1/1 | 1/1 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 157 | AccountObtainPetThroughBattle | 2/510 | 1/459 | 1 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 158 | WinPetBattle | 77/2034 | 74/1540 | 3 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 159 | LosePetBattle | 0/2 | 0/0 | 0 | Source semantics gap: native candidate event/state exists, but pinned evaluator supplies no executable progress rule for this raw type; do not infer count/reset/asset semantics. |
| 160 | BattlePetReachLevel | 6/31 | 6/27 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 161 | PlayerObtainPetThroughBattle | 0/79 | 0/68 | 0 | Source semantics gap: native candidate event/state exists, but pinned evaluator supplies no executable progress rule for this raw type; do not infer count/reset/asset semantics. |
| 162 | ActivelyEarnPetLevel | 20/24 | 0/0 | 20 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 163 | EnterArea | 148/174 | 3/4 | 145 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 164 | LeaveArea | 4/5 | 0/0 | 4 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 165 | DefeatDungeonEncounter | 2393/4358 | 1450/2757 | 943 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 167 | PlaceGarrisonBuilding | 0/35 | 0/0 | 0 | Producer gap: no Garrison building/mission/research actor or lifecycle in current server. |
| 168 | ActivateAnyGarrisonBuilding | 0/3 | 0/2 | 0 | Producer gap: no Garrison building/mission/research actor or lifecycle in current server. |
| 169 | ActivateGarrisonBuilding | 0/77 | 0/44 | 0 | Producer gap: no Garrison building/mission/research actor or lifecycle in current server. |
| 170 | UpgradeGarrison | 0/10 | 0/2 | 0 | Producer gap: no Garrison building/mission/research actor or lifecycle in current server. |
| 171 | StartAnyGarrisonMissionWithFollowerType | 0/5 | 0/0 | 0 | Producer gap: no Garrison building/mission/research actor or lifecycle in current server. |
| 172 | StartGarrisonMission | 0/35 | 0/0 | 0 | Producer gap: no Garrison building/mission/research actor or lifecycle in current server. |
| 173 | SucceedAnyGarrisonMissionWithFollowerType | 0/40 | 0/16 | 0 | Producer gap: no Garrison building/mission/research actor or lifecycle in current server. |
| 174 | SucceedGarrisonMission | 0/373 | 0/107 | 0 | Producer gap: no Garrison building/mission/research actor or lifecycle in current server. |
| 175 | RecruitAnyGarrisonFollower | 0/22 | 0/10 | 0 | Producer gap: no Garrison building/mission/research actor or lifecycle in current server. |
| 176 | RecruitGarrisonFollower | 0/176 | 0/49 | 0 | Producer gap: no Garrison building/mission/research actor or lifecycle in current server. |
| 178 | LearnAnyGarrisonBlueprint | 0/1 | 0/1 | 0 | Producer gap: no Garrison building/mission/research actor or lifecycle in current server. |
| 182 | CollectGarrisonShipment | 0/95 | 0/28 | 0 | Producer gap: no Garrison building/mission/research actor or lifecycle in current server. |
| 183 | ItemLevelChangedForGarrisonFollower | 0/10 | 0/6 | 0 | Producer gap: no Garrison building/mission/research actor or lifecycle in current server. |
| 184 | LevelChangedForGarrisonFollower | 0/7 | 0/6 | 0 | Producer gap: no Garrison building/mission/research actor or lifecycle in current server. |
| 185 | LearnToy | 39/59 | 37/39 | 2 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 186 | LearnAnyToy | 1/1 | 1/1 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 187 | QualityUpgradedForGarrisonFollower | 0/9 | 0/8 | 0 | Producer gap: no Garrison building/mission/research actor or lifecycle in current server. |
| 189 | LearnAnyHeirloom | 0/1 | 0/1 | 0 | Producer/state gap: no owned Heirloom journal/acquisition authority. |
| 190 | EarnArtifactXP | 0/1 | 0/1 | 0 | Producer/state gap: no artifact/relic/Azerite/soulbind progression lifecycle. |
| 191 | AnyArtifactPowerRankPurchased | 0/31 | 0/1 | 0 | Producer/state gap: no artifact/relic/Azerite/soulbind progression lifecycle. |
| 192 | LearnTransmog | 0/68 | 0/34 | 0 | Data/state gap: transmog collection stores ItemAppearance, not source ItemModifiedAppearance; no authored set/group membership catalog in world.db. |
| 193 | LearnAnyTransmog | 0/1 | 0/0 | 0 | Data/state gap: transmog collection stores ItemAppearance, not source ItemModifiedAppearance; no authored set/group membership catalog in world.db. |
| 194 | HonorLevelIncrease | 0/2 | 0/0 | 0 | Producer/state gap: required honor eligibility, team/rated-BG rating or honor/prestige/solo-shuffle progression is unwired; native rated-arena results do not supply those facts. |
| 195 | PrestigeLevelIncrease | 0/30 | 0/25 | 0 | Producer/state gap: required honor eligibility, team/rated-BG rating or honor/prestige/solo-shuffle progression is unwired; native rated-arena results do not supply those facts. |
| 196 | ActivelyReachLevel | 5/30 | 0/3 | 5 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 197 | CompleteResearchAnyGarrisonTalent | 0/26 | 0/26 | 0 | Producer gap: no Garrison building/mission/research actor or lifecycle in current server. |
| 198 | CompleteResearchGarrisonTalent | 0/116 | 0/72 | 0 | Producer gap: no Garrison building/mission/research actor or lifecycle in current server. |
| 199 | LearnAnyTransmogInSlot | 0/13 | 0/13 | 0 | Data/state gap: transmog collection stores ItemAppearance, not source ItemModifiedAppearance; no authored set/group membership catalog in world.db. |
| 200 | RecruitAnyGarrisonTroop | 0/13 | 0/13 | 0 | Producer gap: no Garrison building/mission/research actor or lifecycle in current server. |
| 202 | StartResearchGarrisonTalent | 0/36 | 0/8 | 0 | Producer gap: no Garrison building/mission/research actor or lifecycle in current server. |
| 203 | CompleteAnyWorldQuest | 0/179 | 0/12 | 0 | Source gap: native world-quest reward exists; pinned TC UpdateCriteria switch marks this type unimplemented. Ordinary quest credit is not WorldQuest criteria proof. |
| 204 | EarnLicense | 0/6 | 0/6 | 0 | Source/qualification gap: requires exact producer boundary or progress rule not established by current native criterion implementation; no enum-name inference. |
| 205 | CollectTransmogSetFromGroup | 0/90 | 0/79 | 0 | Data/state gap: transmog collection stores ItemAppearance, not source ItemModifiedAppearance; no authored set/group membership catalog in world.db. |
| 206 | ParagonLevelIncreaseWithFaction | 0/49 | 0/46 | 0 | State/producer gap: CharacterReputation stores traditional signed standing; no paragon thresholds, reward-quest state or tier-completion producer. |
| 207 | PlayerHasEarnedHonor | 0/17 | 0/13 | 0 | Producer/state gap: required honor eligibility, team/rated-BG rating or honor/prestige/solo-shuffle progression is unwired; native rated-arena results do not supply those facts. |
| 208 | KillCreatureScenario | 0/15 | 0/0 | 0 | Source/qualification gap: requires exact producer boundary or progress rule not established by current native criterion implementation; no enum-name inference. |
| 211 | ChooseRelicTalent | 0/2 | 0/1 | 0 | Producer/state gap: no artifact/relic/Azerite/soulbind progression lifecycle. |
| 212 | EarnExpansionLevel | 0/2 | 0/1 | 0 | Producer/state gap: no artifact/relic/Azerite/soulbind progression lifecycle. |
| 213 | AccountHonorLevelReached | 0/21 | 0/21 | 0 | Producer/state gap: required honor eligibility, team/rated-BG rating or honor/prestige/solo-shuffle progression is unwired; native rated-arena results do not supply those facts. |
| 214 | EarnArtifactXPForAzeriteItem | 0/1 | 0/1 | 0 | Producer/state gap: no artifact/relic/Azerite/soulbind progression lifecycle. |
| 215 | AzeriteLevelReached | 0/3 | 0/0 | 0 | Producer/state gap: no artifact/relic/Azerite/soulbind progression lifecycle. |
| 216 | MythicPlusCompleted | 64/813 | 63/781 | 1 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 218 | CompleteAnyReplayQuest | 0/1 | 0/0 | 0 | Condition gap: every raw row has an unsupported qualifier; native candidate does not authorize dropping conditions. |
| 219 | BuyItemsFromVendors | 2/5 | 1/1 | 1 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 220 | SellItemsToVendors | 1/1 | 1/1 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 221 | ReachMaxLevel | 16/16 | 15/15 | 1 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 222 | MemorizeSpell | 0/1 | 0/0 | 0 | Source/qualification gap: requires exact producer boundary or progress rule not established by current native criterion implementation; no enum-name inference. |
| 225 | EnterTopLevelArea | 120/123 | 5/5 | 115 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 228 | SocketAnySoulbindConduit | 0/6 | 0/0 | 0 | Producer/state gap: no artifact/relic/Azerite/soulbind progression lifecycle. |
| 229 | ObtainAnyItemWithCurrencyValue | 0/19 | 0/0 | 0 | Source/qualification gap: requires exact producer boundary or progress rule not established by current native criterion implementation; no enum-name inference. |
| 230 | MythicPlusRatingAttained | 0/16 | 0/16 | 0 | Condition gap: every raw row has an unsupported qualifier; native candidate does not authorize dropping conditions. |
| 231 | SpentTalentPoint | 0/2 | 0/0 | 0 | Source semantics gap: native candidate event/state exists, but pinned evaluator supplies no executable progress rule for this raw type; do not infer count/reset/asset semantics. |
| 234 | MythicPlusDisplaySeasonEnded | 0/24 | 0/19 | 0 | Condition gap: every raw row has an unsupported qualifier; native candidate does not authorize dropping conditions. |
| 239 | WinRatedSoloShuffleRound | 0/20 | 0/18 | 0 | Source/state gap: native Shuffle is explicitly unrated; pinned TC UpdateCriteria has no executable239 progress case (default Not implemented). No inferred rated credit. |
| 240 | ParticipateInRatedSoloShuffleRound | 0/2 | 0/0 | 0 | Source/state gap: native Shuffle is explicitly unrated; pinned TC UpdateCriteria has no executable240 progress case (default Not implemented). No inferred rated credit. |
| 243 | ReputationAmountGained | 0/352 | 0/0 | 0 | Source semantics gap: native candidate event/state exists, but pinned evaluator supplies no executable progress rule for this raw type; do not infer count/reset/asset semantics. |
| 244 | Newer ID — no pinned enum name | 0/40 | 0/0 | 0 | Source gap: newer raw ID absent from pinned primary enum/progress rules; no guessed producer or Asset interpretation. |
| 245 | FulfillAnyCraftingOrder | 2/67 | 1/16 | 1 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 246 | FulfillCraftingOrderType | 4/9 | 3/3 | 1 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 249 | PerksProgramMonthComplete | 0/1 | 0/1 | 0 | Producer gap: adventure_guide.rs has activity points but no sourced monthly reward/interval-completion binding. |
| 250 | CompleteTrackingQuest | 0/14 | 0/0 | 0 | Source/qualification gap: pinned enum marks NYI; do not infer criteria from an ordinary quest reward or discard qualifiers. |
| 252 | Newer ID — no pinned enum name | 0/54 | 0/3 | 0 | Source gap: newer raw ID absent from pinned primary enum/progress rules; no guessed producer or Asset interpretation. |
| 253 | GainLevels | 6/12 | 1/1 | 5 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 254 | Newer ID — no pinned enum name | 0/10 | 0/9 | 0 | Source gap: newer raw ID absent from pinned primary enum/progress rules; no guessed producer or Asset interpretation. |
| 255 | Newer ID — no pinned enum name | 0/50 | 0/16 | 0 | Source gap: newer raw ID absent from pinned primary enum/progress rules; no guessed producer or Asset interpretation. |
| 256 | Newer ID — no pinned enum name | 0/16 | 0/16 | 0 | Source gap: newer raw ID absent from pinned primary enum/progress rules; no guessed producer or Asset interpretation. |
| 257 | CompleteQuestsCountOnAccount | 0/2 | 0/1 | 0 | Source/state gap: pinned enum marks NYI; distinct/repeat/account quest-count policy is not supplied by a type9 per-character maximum. |
| 260 | BankTabPurchased | 3/3 | 2/2 | 1 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 261 | ReachRenownLevel | 0/42 | 0/36 | 0 | Producer gap: native renown progress now exists but only admin setup writes it. TC Player7165 requires a positive RenownRepGain currency mutation and current capped quantity, keyed by CurrencyID; admin/major-faction IDs cannot substitute. |
| 262 | LearnTaxiNode | 45/45 | 41/41 | 4 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 263 | Newer ID — no pinned enum name | 0/4 | 0/0 | 0 | Source gap: newer raw ID absent from pinned primary enum/progress rules; no guessed producer or Asset interpretation. |
| 266 | Newer ID — no pinned enum name | 0/12 | 0/0 | 0 | Source gap: newer raw ID absent from pinned primary enum/progress rules; no guessed producer or Asset interpretation. |
| 267 | Newer ID — no pinned enum name | 0/1 | 0/0 | 0 | Source gap: newer raw ID absent from pinned primary enum/progress rules; no guessed producer or Asset interpretation. |
| 269 | Newer ID — no pinned enum name | 0/3 | 0/3 | 0 | Source gap: newer raw ID absent from pinned primary enum/progress rules; no guessed producer or Asset interpretation. |
| 270 | PlaceDecor | 0/10 | 0/0 | 0 | Source semantics gap: native housing placement exists, but pinned enum is NYI and evaluator has no counter/asset/reset rule. No guessed criteria credit. |
| 271 | RemoveDecor | 0/4 | 0/0 | 0 | Source semantics gap: native housing removal exists, but pinned enum is NYI and evaluator has no counter/asset/reset rule. No guessed criteria credit. |
| 272 | CollectUniqueDecor | 0/1 | 0/1 | 0 | Source gap: housing storage exists, but unique DecorID/lifetime/current and Asset2 criteria semantics are not sourced. |
| 273 | Newer ID — no pinned enum name | 0/34 | 0/22 | 0 | Source gap: newer raw ID absent from pinned primary enum/progress rules; no guessed producer or Asset interpretation. |
| 274 | Newer ID — no pinned enum name | 0/24 | 0/21 | 0 | Source gap: newer raw ID absent from pinned primary enum/progress rules; no guessed producer or Asset interpretation. |
| 275 | Newer ID — no pinned enum name | 0/1 | 0/0 | 0 | Source gap: newer raw ID absent from pinned primary enum/progress rules; no guessed producer or Asset interpretation. |
| 276 | Newer ID — no pinned enum name | 0/121 | 0/120 | 0 | Source gap: newer raw ID absent from pinned primary enum/progress rules; no guessed producer or Asset interpretation. |
| 277 | Newer ID — no pinned enum name | 0/1 | 0/0 | 0 | Source gap: newer raw ID absent from pinned primary enum/progress rules; no guessed producer or Asset interpretation. |
| 278 | Newer ID — no pinned enum name | 0/2 | 0/0 | 0 | Source gap: newer raw ID absent from pinned primary enum/progress rules; no guessed producer or Asset interpretation. |
| 279 | Newer ID — no pinned enum name | 0/18 | 0/17 | 0 | Source gap: newer raw ID absent from pinned primary enum/progress rules; no guessed producer or Asset interpretation. |
| 282 | Newer ID — no pinned enum name | 0/3 | 0/0 | 0 | Source gap: newer raw ID absent from pinned primary enum/progress rules; no guessed producer or Asset interpretation. |
| 283 | Newer ID — no pinned enum name | 0/1 | 0/0 | 0 | Source gap: newer raw ID absent from pinned primary enum/progress rules; no guessed producer or Asset interpretation. |

## Proof ledger for this continuation

| Scope | Revision | Proof |
|---|---|---|
| Earned native levels196/221/253 | f2deef96 | level-criteria-red-native → level-criteria-green-native,1/1 |
| Current max-level class leaves/account7382 | 10e84176; fixture6f268356 | level-class-tree-red → level-class-tree-relogin-green,1/1 |
| Rank47/87/88 | ea331b3a | rank-totals-red → rank-totals-green,1/1; authentic45068 reward settlement, exclusions/loss/idle/redb |
| Area163/164 | 68771f07 | area163-164-red → area163-164-green,2/2; real2444 WDT/ADT movement, ancestry/negative leave/redb |
| Raw inventory | 17e6aaeb | raw-inventory-red → raw-inventory-green,1/1; supported orphan reference cannot become achievement coverage |

Logs and census JSON: `/home/osso/.worktrees/logs/achievements-criteria-2026-10-12/`. Pinned input hashes are in each census JSON and generated source. No data/cache or canonical writes authorized.
