# Retail achievement criteria producer inventory

verified: 2026-10-12

Scope: all243 distinct Types in the pinned69933 Criteria.csv. Raw support is NOT a promise that every conditional row, actor, timer, world-state or current achievement tree is executable. Current structural all-faction player union: **5015 ordinary (3288 account/1727 character),1485 statistics; 94 enabled types**. Spec/producer contracts: [dungeon achievements](specs/dungeon-achievements.md).

## Current-tree wiring

- ModifierTree DungeonDifficulty68 (TC CriteriaHandler2090–2093, operators1723–1751): 2539 criteria whose only condition is an All/SingleTrue, All/SingleFalse or Some(≤1) difficulty tree are compiled (`achievement_difficulty_rules.rs`) and checked against the player's current copy Difficulty (continents0) at event time; +460 ordinary/+1226 statistics. Raw counts below include these rows under their own criteria type.
- Types196/221/253 are earned native XP transitions, not login/admin facts. Thirteen exact current221 class leaves now admit7382/7383/7384/19489; native Paladin+Mage transitions earn7382 and survive reopened account login.
- Types13/54/55/102–106 and the new unconditional rank/area/level counters do not gain an Achievement merely because a raw CriteriaTree references them. Census follows actual Achievement roots; orphan raw rows are retained for non-achievement consumers, never relinked.
- Damage Control227 is still blocked by battleground start/fail lifetime conditions and zone/target predicates, not missing raw damage/healing counters. Unknown conditions cannot be stripped.
- Both-skin transport proof currently covers actual paid2084/2076/2078 and native target comparison at the exact earlier binary SHAs in the handoff. Current-build refresh and integration gate remain pending.
- Independent subscription verifier failed OAuth refresh at this checkpoint; no paid fallback used. Whole goal remains open.

## Complete raw-type matrix

Counts: supported raw / raw; supported current-root rows / all current-root rows. Orphans are supported raw rows outside all Achievement roots. A declared producer may have zero eligible rows. Primary names use independently refetched TC a352b1fa DBCEnums.h (SHA256 a00f6c7d8c771e91cb0dfad37b87ade5f9f9e74f960ccad47426029bd8f7f4e1). The enum ends at its standalone closing line, not the inline `{FactionID};` comment at243. Types245–272 are therefore not mislabeled absent. Missing primary names stay unknown instead of being invented. Source/qualification gaps are OPEN, not completion claims.

| Type | Pinned source name | Raw | Current roots | Orphans | State / producer evidence |
|---:|---|---:|---:|---:|---|
| 0 | KillCreature | 6564/10154 | 2725/4428 | 3839 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 1 | WinBattleground | 25/212 | 25/160 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 3 | CompleteAnyResearchProject | 0/12 | 0/12 | 0 | Producer gap: no archaeology/research runtime; enum names do not authorize synthetic progress. |
| 4 | FindResearchObject | 0/12 | 0/0 | 0 | Producer gap: no archaeology/research runtime; enum names do not authorize synthetic progress. |
| 5 | ReachLevel | 13/270 | 1/196 | 12 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 6 | ExhaustAnyResearchSite | 0/4 | 0/3 | 0 | Producer gap: no archaeology/research runtime; enum names do not authorize synthetic progress. |
| 7 | SkillRaised | 190/375 | 190/363 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 8 | EarnAchievement | 5618/5959 | 5146/5414 | 472 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 9 | CompleteQuestsCount | 1/13 | 1/1 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 10 | CompleteAnyDailyQuestPerDay | 0/2 | 0/1 | 0 | Condition gap: every raw row has an unsupported qualifier; native candidate does not authorize dropping conditions. |
| 11 | CompleteQuestsInZone | 149/641 | 59/59 | 90 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 12 | CurrencyGained | 127/337 | 64/204 | 63 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 13 | DamageDealt | 1/18 | 0/15 | 1 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 14 | CompleteDailyQuest | 1/34 | 1/16 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 15 | ParticipateInBattleground | 31/84 | 25/67 | 6 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 16 | DieOnMap | 24/24 | 0/0 | 24 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 17 | DieAnywhere | 1/44 | 1/11 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 18 | DieInInstance | 0/12 | 0/9 | 0 | Condition gap: every raw row has an unsupported qualifier; native candidate does not authorize dropping conditions. |
| 19 | RunInstance | 0/3 | 0/3 | 0 | Source/qualification gap: requires exact producer boundary or progress rule not established by current native criterion implementation; no enum-name inference. |
| 20 | KilledByCreature | 59/168 | 0/1 | 59 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 21 | CompleteInternalCriteria | 0/376 | 0/68 | 0 | Source/producer gap: deprecated/internal criterion or title grant; no designer/title ownership producer. |
| 22 | CompleteAnyChallengeMode | 0/19 | 0/9 | 0 | Source/qualification gap: requires exact producer boundary or progress rule not established by current native criterion implementation; no enum-name inference. |
| 23 | KilledByPlayer | 1/1 | 1/1 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 24 | MaxDistFallenWithoutDying | 0/3 | 0/3 | 0 | Condition gap: every raw row has an unsupported qualifier; native candidate does not authorize dropping conditions. |
| 26 | DieFromEnviromentalDamage | 3/6 | 3/6 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 27 | CompleteQuest | 12627/14115 | 9393/9889 | 3234 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 28 | BeSpellTarget | 715/1839 | 458/1290 | 257 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 29 | CastSpell | 2397/2972 | 1391/1657 | 1006 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 30 | TrackedWorldStateUIModified | 15/87 | 15/78 | 0 | Native: every battleground objective credit (TC BattlegroundScore::UpdatePvpStat:78-80) records +1 for the connected character, completed in the match map; only unconditional rows whose PvpStat is a native column. BG start/fail-event rows stay disabled. Native completion-map gate leaves 199/200 (Instance_ID489) unearned on WSG2106. See spec. |
| 31 | PVPKillInArea | 0/51 | 0/45 | 0 | Producer/state gap: no sourced honor eligibility, rated arena/BG result or honor/prestige/solo-shuffle progression; pvp_objective.rs initializes honor/rating facts to None. |
| 32 | WinArena | 0/82 | 0/62 | 0 | Producer/state gap: no sourced honor eligibility, rated arena/BG result or honor/prestige/solo-shuffle progression; pvp_objective.rs initializes honor/rating facts to None. |
| 33 | ParticipateInArena | 0/87 | 0/61 | 0 | Producer/state gap: no sourced honor eligibility, rated arena/BG result or honor/prestige/solo-shuffle progression; pvp_objective.rs initializes honor/rating facts to None. |
| 34 | LearnOrKnowSpell | 530/578 | 441/467 | 89 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 35 | EarnHonorableKill | 0/62 | 0/52 | 0 | Producer/state gap: no sourced honor eligibility, rated arena/BG result or honor/prestige/solo-shuffle progression; pvp_objective.rs initializes honor/rating facts to None. |
| 36 | AcquireItem | 1650/1759 | 1123/1132 | 527 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 37 | WinAnyRankedArena | 0/60 | 0/34 | 0 | Producer/state gap: no sourced honor eligibility, rated arena/BG result or honor/prestige/solo-shuffle progression; pvp_objective.rs initializes honor/rating facts to None. |
| 38 | EarnTeamArenaRating | 0/6 | 0/3 | 0 | Producer/state gap: no sourced honor eligibility, rated arena/BG result or honor/prestige/solo-shuffle progression; pvp_objective.rs initializes honor/rating facts to None. |
| 39 | EarnPersonalArenaRating | 0/88 | 0/68 | 0 | Producer/state gap: no sourced honor eligibility, rated arena/BG result or honor/prestige/solo-shuffle progression; pvp_objective.rs initializes honor/rating facts to None. |
| 40 | AchieveSkillStep | 12/36 | 12/12 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 41 | UseItem | 1292/1404 | 1213/1231 | 79 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 42 | LootItem | 388/579 | 292/292 | 96 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 43 | RevealWorldMapOverlay | 1640/1932 | 1636/1779 | 4 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 44 | EarnTitle | 0/14 | 0/14 | 0 | Source/producer gap: deprecated/internal criterion or title grant; no designer/title ownership producer. |
| 45 | BankSlotsPurchased | 0/2 | 0/1 | 0 | Producer gap: native banks buy tabs, not legacy bank bag slots. |
| 46 | ReputationGained | 259/387 | 222/342 | 37 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 47 | TotalExaltedFactions | 1/2 | 0/0 | 1 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 48 | GotHaircut | 1/2 | 1/1 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 49 | EquipItemInSlot | 0/433 | 0/113 | 0 | Condition gap: every raw row has an unsupported qualifier; native candidate does not authorize dropping conditions. |
| 50 | RollNeed | 0/2 | 0/1 | 0 | Condition gap: every raw row has an unsupported qualifier; native candidate does not authorize dropping conditions. |
| 51 | RollGreed | 0/2 | 0/1 | 0 | Condition gap: every raw row has an unsupported qualifier; native candidate does not authorize dropping conditions. |
| 52 | DeliverKillingBlowToClass | 0/13 | 0/13 | 0 | Producer/state gap: no sourced honor eligibility, rated arena/BG result or honor/prestige/solo-shuffle progression; pvp_objective.rs initializes honor/rating facts to None. |
| 53 | DeliverKillingBlowToRace | 0/149 | 0/148 | 0 | Producer/state gap: no sourced honor eligibility, rated arena/BG result or honor/prestige/solo-shuffle progression; pvp_objective.rs initializes honor/rating facts to None. |
| 54 | DoEmote | 1/398 | 0/254 | 1 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 55 | HealingDone | 1/16 | 0/15 | 1 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 56 | DeliveredKillingBlow | 1/80 | 1/63 | 0 | Native: authenticated in-world killer on actual lethal health commits, plus genuine HunterPet owner credit (TC Unit::Kill); dead replay/self raw KillUnit idle. See spec. |
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
| 68 | UseGameobject | 498/578 | 354/383 | 144 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 69 | GainAura | 495/760 | 53/225 | 442 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 70 | KillPlayer | 0/180 | 0/102 | 0 | Producer/state gap: no sourced honor eligibility, rated arena/BG result or honor/prestige/solo-shuffle progression; pvp_objective.rs initializes honor/rating facts to None. |
| 71 | CompleteChallengeMode | 0/244 | 0/120 | 0 | Source/qualification gap: requires exact producer boundary or progress rule not established by current native criterion implementation; no enum-name inference. |
| 72 | CatchFishInFishingHole | 0/38 | 0/36 | 0 | Producer gap: no fishing pool/catch settlement (toy transmitted-object placeholders are not fishing). |
| 73 | PlayerTriggerGameEvent | 125/804 | 41/587 | 84 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 74 | Login | 1/462 | 1/372 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 75 | LearnSpellFromSkillLine | 12/16 | 2/2 | 10 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 76 | WinDuel | 3/27 | 1/16 | 2 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 77 | LoseDuel | 1/1 | 1/1 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 78 | KillAnyCreature | 2/112 | 1/21 | 1 | Native: TC KillRewarder:185 via shared TapRewardRecipients (map copy, range, group, corpse eligibility). See spec. |
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
| 113 | HonorableKills | 0/4 | 0/1 | 0 | Producer/state gap: no sourced honor eligibility, rated arena/BG result or honor/prestige/solo-shuffle progression; pvp_objective.rs initializes honor/rating facts to None. |
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
| 130 | WinAnyBattleground | 16/75 | 0/35 | 16 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 131 | ParticipateInAnyBattleground | 0/11 | 0/1 | 0 | Source/qualification gap: requires exact producer boundary or progress rule not established by current native criterion implementation; no enum-name inference. |
| 132 | EarnBattlegroundRating | 0/36 | 0/22 | 0 | Producer/state gap: no sourced honor eligibility, rated arena/BG result or honor/prestige/solo-shuffle progression; pvp_objective.rs initializes honor/rating facts to None. |
| 133 | GuildTabardCreated | 0/1 | 0/1 | 0 | Actor gap: player/account achievements cannot impersonate Guild progress; no Guild criteria aggregation. |
| 134 | CompleteQuestsCountForGuild | 0/1 | 0/1 | 0 | Actor gap: player/account achievements cannot impersonate Guild progress; no Guild criteria aggregation. |
| 135 | HonorableKillsForGuild | 0/1 | 0/1 | 0 | Actor gap: player/account achievements cannot impersonate Guild progress; no Guild criteria aggregation. |
| 136 | KillAnyCreatureForGuild | 0/1 | 0/1 | 0 | Actor gap: player/account achievements cannot impersonate Guild progress; no Guild criteria aggregation. |
| 137 | GroupedTankLeftEarlyInLFGDungeon | 0/1 | 0/0 | 0 | Producer gap: lfg.rs stores role checks/match_found only, not an LFG/LFR instance completion, stranger roster or kick/abandon settlement. |
| 138 | CompleteGuildChallenge | 0/6 | 0/5 | 0 | Actor gap: player/account achievements cannot impersonate Guild progress; no Guild criteria aggregation. |
| 139 | CompleteAnyGuildChallenge | 0/1 | 0/1 | 0 | Actor gap: player/account achievements cannot impersonate Guild progress; no Guild criteria aggregation. |
| 140 | MarkedAFKInBattleground | 0/1 | 0/0 | 0 | Producer gap: no battleground AFK-report/removal authority. |
| 141 | RemovedAFKInBattleground | 0/1 | 0/0 | 0 | Producer gap: no battleground AFK-report/removal authority. |
| 142 | StartAnyBattleground | 0/1 | 0/0 | 0 | Source/qualification gap: requires exact producer boundary or progress rule not established by current native criterion implementation; no enum-name inference. |
| 143 | CompleteAnyBattleground | 0/6 | 0/0 | 0 | Source/qualification gap: requires exact producer boundary or progress rule not established by current native criterion implementation; no enum-name inference. |
| 144 | MarkedSomeoneAFKInBattleground | 0/1 | 0/0 | 0 | Producer gap: no battleground AFK-report/removal authority. |
| 145 | CompletedLFRDungeon | 0/2 | 0/0 | 0 | Producer gap: lfg.rs stores role checks/match_found only, not an LFG/LFR instance completion, stranger roster or kick/abandon settlement. |
| 146 | AbandonedLFRDungeon | 0/1 | 0/0 | 0 | Producer gap: lfg.rs stores role checks/match_found only, not an LFG/LFR instance completion, stranger roster or kick/abandon settlement. |
| 147 | KickInitiatorInLFRDungeon | 0/1 | 0/0 | 0 | Producer gap: lfg.rs stores role checks/match_found only, not an LFG/LFR instance completion, stranger roster or kick/abandon settlement. |
| 148 | KickVoterInLFRDungeon | 0/1 | 0/0 | 0 | Producer gap: lfg.rs stores role checks/match_found only, not an LFG/LFR instance completion, stranger roster or kick/abandon settlement. |
| 149 | KickTargetInLFRDungeon | 0/1 | 0/0 | 0 | Producer gap: lfg.rs stores role checks/match_found only, not an LFG/LFR instance completion, stranger roster or kick/abandon settlement. |
| 150 | GroupedTankLeftEarlyInLFRDungeon | 0/1 | 0/0 | 0 | Producer gap: lfg.rs stores role checks/match_found only, not an LFG/LFR instance completion, stranger roster or kick/abandon settlement. |
| 151 | CompleteAnyScenario | 6/8 | 1/2 | 5 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 152 | CompleteScenario | 865/1070 | 70/139 | 795 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 153 | EnterAreaTriggerWithActionSet | 0/33 | 0/8 | 0 | Source/qualification gap: requires exact producer boundary or progress rule not established by current native criterion implementation; no enum-name inference. |
| 155 | LearnedNewPet | 127/127 | 122/122 | 5 | Native: battle_pets::learn_companion_from_item and wild capture credit species after the new-GUID journal commit (TC BattlePetMgr::AddPet:483); compiled AND4→BattlePetSpecies91 leaves only. Local item sources exist for species1928/1929 only; Coiled Isle wild species lack battle abilities. See spec. |
| 156 | UniquePetsOwned | 1/1 | 1/1 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 157 | AccountObtainPetThroughBattle | 1/510 | 1/459 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 158 | WinPetBattle | 1/2034 | 1/1540 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 159 | LosePetBattle | 0/2 | 0/0 | 0 | Source semantics gap: native candidate event/state exists, but pinned evaluator supplies no executable progress rule for this raw type; do not infer count/reset/asset semantics. |
| 160 | BattlePetReachLevel | 6/31 | 6/27 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 161 | PlayerObtainPetThroughBattle | 0/79 | 0/68 | 0 | Source semantics gap: native candidate event/state exists, but pinned evaluator supplies no executable progress rule for this raw type; do not infer count/reset/asset semantics. |
| 162 | ActivelyEarnPetLevel | 6/24 | 0/0 | 6 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 163 | EnterArea | 145/174 | 0/4 | 145 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 164 | LeaveArea | 4/5 | 0/0 | 4 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 165 | DefeatDungeonEncounter | 2363/4358 | 1429/2757 | 934 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
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
| 194 | HonorLevelIncrease | 0/2 | 0/0 | 0 | Producer/state gap: no sourced honor eligibility, rated arena/BG result or honor/prestige/solo-shuffle progression; pvp_objective.rs initializes honor/rating facts to None. |
| 195 | PrestigeLevelIncrease | 0/30 | 0/25 | 0 | Producer/state gap: no sourced honor eligibility, rated arena/BG result or honor/prestige/solo-shuffle progression; pvp_objective.rs initializes honor/rating facts to None. |
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
| 207 | PlayerHasEarnedHonor | 0/17 | 0/13 | 0 | Producer/state gap: no sourced honor eligibility, rated arena/BG result or honor/prestige/solo-shuffle progression; pvp_objective.rs initializes honor/rating facts to None. |
| 208 | KillCreatureScenario | 0/15 | 0/0 | 0 | Source/qualification gap: requires exact producer boundary or progress rule not established by current native criterion implementation; no enum-name inference. |
| 211 | ChooseRelicTalent | 0/2 | 0/1 | 0 | Producer/state gap: no artifact/relic/Azerite/soulbind progression lifecycle. |
| 212 | EarnExpansionLevel | 0/2 | 0/1 | 0 | Producer/state gap: no artifact/relic/Azerite/soulbind progression lifecycle. |
| 213 | AccountHonorLevelReached | 0/21 | 0/21 | 0 | Producer/state gap: no sourced honor eligibility, rated arena/BG result or honor/prestige/solo-shuffle progression; pvp_objective.rs initializes honor/rating facts to None. |
| 214 | EarnArtifactXPForAzeriteItem | 0/1 | 0/1 | 0 | Producer/state gap: no artifact/relic/Azerite/soulbind progression lifecycle. |
| 215 | AzeriteLevelReached | 0/3 | 0/0 | 0 | Producer/state gap: no artifact/relic/Azerite/soulbind progression lifecycle. |
| 216 | MythicPlusCompleted | 64/813 | 63/781 | 1 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 218 | CompleteAnyReplayQuest | 0/1 | 0/0 | 0 | Condition gap: every raw row has an unsupported qualifier; native candidate does not authorize dropping conditions. |
| 219 | BuyItemsFromVendors | 2/5 | 1/1 | 1 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 220 | SellItemsToVendors | 1/1 | 1/1 | 0 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 221 | ReachMaxLevel | 14/16 | 13/15 | 1 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 222 | MemorizeSpell | 0/1 | 0/0 | 0 | Source/qualification gap: requires exact producer boundary or progress rule not established by current native criterion implementation; no enum-name inference. |
| 225 | EnterTopLevelArea | 120/123 | 5/5 | 115 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 228 | SocketAnySoulbindConduit | 0/6 | 0/0 | 0 | Producer/state gap: no artifact/relic/Azerite/soulbind progression lifecycle. |
| 229 | ObtainAnyItemWithCurrencyValue | 0/19 | 0/0 | 0 | Source/qualification gap: requires exact producer boundary or progress rule not established by current native criterion implementation; no enum-name inference. |
| 230 | MythicPlusRatingAttained | 0/16 | 0/16 | 0 | Condition gap: every raw row has an unsupported qualifier; native candidate does not authorize dropping conditions. |
| 231 | SpentTalentPoint | 0/2 | 0/0 | 0 | Source semantics gap: native candidate event/state exists, but pinned evaluator supplies no executable progress rule for this raw type; do not infer count/reset/asset semantics. |
| 234 | MythicPlusDisplaySeasonEnded | 0/24 | 0/19 | 0 | Condition gap: every raw row has an unsupported qualifier; native candidate does not authorize dropping conditions. |
| 239 | WinRatedSoloShuffleRound | 0/20 | 0/18 | 0 | Producer/state gap: no sourced honor eligibility, rated arena/BG result or honor/prestige/solo-shuffle progression; pvp_objective.rs initializes honor/rating facts to None. |
| 240 | ParticipateInRatedSoloShuffleRound | 0/2 | 0/0 | 0 | Producer/state gap: no sourced honor eligibility, rated arena/BG result or honor/prestige/solo-shuffle progression; pvp_objective.rs initializes honor/rating facts to None. |
| 243 | ReputationAmountGained | 0/352 | 0/0 | 0 | Source semantics gap: native candidate event/state exists, but pinned evaluator supplies no executable progress rule for this raw type; do not infer count/reset/asset semantics. |
| 244 | Newer ID — no pinned enum name | 0/40 | 0/0 | 0 | Source gap: newer raw ID absent from pinned primary enum/progress rules; no guessed producer or Asset interpretation. |
| 245 | FulfillAnyCraftingOrder | 2/67 | 1/16 | 1 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 246 | FulfillCraftingOrderType | 4/9 | 3/3 | 1 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 249 | PerksProgramMonthComplete | 0/1 | 0/1 | 0 | Producer gap: adventure_guide.rs has activity points but no sourced monthly reward/interval-completion binding. |
| 250 | CompleteTrackingQuest | 0/14 | 0/0 | 0 | Source/qualification gap: pinned enum marks NYI; do not infer criteria from an ordinary quest reward or discard qualifiers. |
| 252 | Newer ID — no pinned enum name | 0/54 | 0/3 | 0 | Source gap: newer raw ID absent from pinned primary enum/progress rules; no guessed producer or Asset interpretation. |
| 253 | GainLevels | 4/12 | 0/1 | 4 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 254 | Newer ID — no pinned enum name | 0/10 | 0/9 | 0 | Source gap: newer raw ID absent from pinned primary enum/progress rules; no guessed producer or Asset interpretation. |
| 255 | Newer ID — no pinned enum name | 0/50 | 0/16 | 0 | Source gap: newer raw ID absent from pinned primary enum/progress rules; no guessed producer or Asset interpretation. |
| 256 | Newer ID — no pinned enum name | 0/16 | 0/16 | 0 | Source gap: newer raw ID absent from pinned primary enum/progress rules; no guessed producer or Asset interpretation. |
| 257 | CompleteQuestsCountOnAccount | 0/2 | 0/1 | 0 | Source/state gap: pinned enum marks NYI; distinct/repeat/account quest-count policy is not supplied by a type9 per-character maximum. |
| 260 | BankTabPurchased | 3/3 | 2/2 | 1 | Native criterion events wired in achievement_catalog.rs/achievement.rs; exact boundary and limits in linked spec. Conditional/unsupported rows stay disabled. |
| 261 | ReachRenownLevel | 0/42 | 0/36 | 0 | Producer/state gap: no native Renown amount/level authority; traditional reputation cannot stand in for renown currencies. |
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
