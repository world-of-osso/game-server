# Experience

`ExperiencePlugin` (`crates/server/src/experience.rs`) grants kill and quest XP and levels characters up. The rules are Retail as TrinityCore master implements them: `Trinity::XP` in `Formulas.h`, `KillRewarder`, `Quest::XPValue` and `Player::GiveXP` / `GiveLevel`. `shared::xp` (Wrath `player_xp_for_level` and invented group bonus) is not used for XP; `rest.rs` still uses its `RestedXp`.

## Data (12.1.0.69933)

`scripts/gen_xp_tables.py` generates `xp_data.rs` from:

- gtXp, `data/gametables/xp.txt` (local CASC FDID 1391661). The table gives `Total` (XP to the next level), `PerKill` and `Divisor` per level. Level 1 needs 250, level 10 needs 10,590 and level 89 needs 592,980. Row 90 is the sentinel 99999999, so the cap is 90 (`spell_info::MAX_LEVEL`).
- QuestXP DB2 from wago.tools, `game-engine/data/db2/12.1.0.69933/QuestXP.csv`: `Difficulty_0..9` for quest levels 1–130.

## Kill XP

- `base_gain` (BaseGain):
  - Same-level or higher mob: player `PerKill` × (1 + 0.05 × min(diff, 4)).
  - Lower mob: mob `PerKill` × (1 − diff / ZeroDifference) × divisor ratio.
  - Grey mob (level ≤ `GetGrayLevel`): 0. Grey level is 0 below 7, `(L−7) − (count of multiples of 5 in 15..=L − 1)` below 35, and `L−10` from 35.
- `kill_gain` (Gain):
  - Critters (`type` 8) and `flags_extra & 0x40` give 0.
  - Retail's ÷10 for creatures of an older expansion (`GetExpansionForLevel`) is omitted: without ContentTuning scaling every creature is older than a level 30+ player, so levelling would stall. A level 35 same-level kill gives the full 190.
  - Elites (`rank` 1 or 2) give ×2. There are no dungeon maps, so the 2.75× dungeon rate never applies.
  - Multiplied by `ExperienceModifier`.
- Level 10 examples: a level 10 mob gives 65, level 8 gives 39, level 5 gives 11 and level 4 gives 0. A level 10 elite gives 130.
- `kill_rewards` (KillRewarder):
  - Solo: the killer gets the gain if alive.
  - Grouped (`GroupRegistry`):
    - Counted members: the killer plus alive members within 74 yd.
    - XP is computed at the highest counted level for which the mob is not grey.
    - Each alive member at or below that level gets `xp × group_rate × level / sum_level`. `group_rate` is 1.0 for 1–2 members, 1.166 for 3, 1.3 for 4 and 1.4 for 5+.
    - If a higher member sees the mob as grey, each share is halved and gets +1.
  - Two level 10 players killing a level 10 mob get 32 each.
- `award_kill_xp` (PostUpdate) reads `Added<DeathState>` NPCs with `LastDamager`, `UnitLevel` and the `WorldData` template.

## Quest XP

- `quest_xp` (XPValue):
  - Base: `QuestXP[quest level][RewardXPDifficulty]`.
  - Multiplied by `diffFactor = clamp(2 × (quest level − player level) + 12, 1, 10) / 10`.
  - Retail's ÷9 at player level 80+ for old-expansion quests is omitted: all quest content is AzerothCore Wrath, so it would cut every quest past 80. A level 85 quest at 85 (difficulty 5) gives the full 12,450.
  - Rounded by `RoundXPValue`.
- Imported quests have a fixed `QuestLevel` instead of Retail ContentTuning, where −1 means the player's level. Quests far below the player therefore keep the reduction.
- A Threat Within 783 (level 1, difficulty 3) gives 100 at level 1, 20 at level 6 and 10 at level 10.
- `choose_reward` queues the XP after the quest is marked rewarded.
- At the cap, quest XP is dropped. Retail pays `RewardMoneyMaxLevel` instead, which is not implemented.

## Levelling

- `XpAwards` queue: `apply_xp_awards` (PostUpdate, after kill XP) runs GiveXP.
  - Dead or ghost players (`PlayerDeathState`) and capped players gain nothing.
  - XP carries over across levels. `give_xp(1, 0, 1000)` gives level 3 with 95 XP.
- On a level-up:
  - `CharacterData.level` is saved.
  - `Level` and `UnitLevel` are set.
  - Class progression sends spell and spec deltas and the trait budget ([[class-progression]]).
  - Stats are recomputed ([[character-stats]]).
  - `RefillAfterLevelUp` sets health and mana to the new maxima after `update_character_stats`. Retail `GiveLevel` refills only health and mana (`SetToMaxOnLevelUp`).
- redb `character_xp`: `character_id` → u32 XP into the current level. The row is removed with the character.

## Messages (ExperienceChannel, owner only)

- `PlayerXpUpdate { xp, next_level_xp, rested_xp }`: on enter world (`Added<InWorld>`), after each applied gain and on any `Changed<Level>` (admin set-level included). `next_level_xp` is 0 at the cap.
- `LogXpGain { victim, original, amount, group_bonus, reason }` for each applied gain (Retail `LogXPGain`): Kill carries the NPC entity bits and the group rate; Quest has no victim and rate 1.0. Rested XP is not applied, so `original == amount`.
- `QuestGiverQuestComplete.xp`: the turn-in XP, 0 at the cap.
- `send_xp_updates` runs after `apply_xp_awards` in PostUpdate, before replication send.

## Gaps

- `rested_xp` in `PlayerXpUpdate` comes from `PlayerRestState`, but the pool never accumulates or is consumed.
- Rested bonus (`RestMgr::GetRestBonusFor`) is not applied. Rested XP is not persisted.
- `SPELL_AURA_MOD_XP_PCT`, quest XP auras, pet XP, BG/raid rates and XP-to-money at the cap are not implemented.

## Sources

- [experience.rs](../../../crates/server/src/experience.rs), [experience_tests.rs](../../../crates/server/src/experience_tests.rs)
- TrinityCore master `src/server/game/Miscellaneous/Formulas.h`, `Entities/Player/KillRewarder.cpp`, `Entities/Player/Player.cpp` (GiveXP, GiveLevel, RewardQuest), `Quests/QuestDef.cpp` (XPValue, RoundXPValue), `Entities/Creature/Creature.cpp` (IsElite, NO_XP)

## See Also

- [[quests]] — turn-in pipeline
- [[class-progression]] — level-change deltas
- [[character-stats]] — per-level stats
