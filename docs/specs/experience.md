# Experience

Kill and quest experience and level-ups, Retail 12.1.0.69933 rules (TrinityCore master `Formulas.h`, `KillRewarder`, `Quest::XPValue`, `Player::GiveXP`/`GiveLevel`). Server: `crates/server/src/experience.rs`, generated `xp_data.rs` (`scripts/gen_xp_tables.py`), redb `character_xp`. See [experience system](../wiki/systems/experience.md) for the mechanism.

## What it must do

- [x] XP to the next level comes from gtXp `Total`; the level cap is 90.
- [x] Killing an NPC gives kill XP from gtXp `PerKill`: +5% per level above (max 4), scaled down for lower mobs by the zero-difference window, 0 for grey mobs, critters and `NO_XP` creatures; elites ×2; `ExperienceModifier`; ÷10 for creatures of an expansion below the player's level expansion.
- [x] Group kills split XP among the killer and alive group members within 74 yards by level share with the 1.0/1.166/1.3/1.4 group bonus; half XP (+1) when the mob is grey for a higher member; members above the highest non-grey level get none.
- [x] Turning in a quest gives `QuestXP[quest level][RewardXPDifficulty]` × `diffFactor`/10 (low-level reduction), ÷9 at level 80+ for old-expansion content, rounded by `RoundXPValue`.
- [x] Dead players and capped players gain no XP.
- [x] XP carries over across level-ups; a level-up saves the level, sets `Level`/`UnitLevel` (known-spell, spec, trait budget and stat updates follow) and refills health and mana.
- [x] XP into the current level persists per character.

## Tests asserting this spec

- `crates/server/src/experience_tests.rs` — gtXp thresholds, level 10 same-level kill 65, grey mob 0, elite, critter, expansion ÷10, solo/group/grey-split rewards, QuestXP 783 = 100 and reductions, carry-over and cap, level-up persistence and refill, dead player.
- `crates/server/src/class_progression_tests.rs` `kill_xp_crossing_level_10_levels_up_and_sends_learned_spells` — real kill → level 10 → Holy spec and level 10 spells sent.
- `crates/server/src/quest_tests.rs` — A Threat Within turn-in stores 100 XP.

## Known gaps

- [ ] No XP protocol: current/next XP, rested XP, XP gain log (Retail `LogXPGain`), quest-complete XP.
- [ ] Rested bonus not applied; XP auras, pet XP, BG/raid rates, XP→money at the cap not implemented.
- [ ] Quest level is the imported fixed `QuestLevel`, not Retail ContentTuning scaling.
