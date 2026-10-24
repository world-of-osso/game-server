# Character stats

Players get retail 12.x stats on the server. See [character stats](../wiki/systems/character-stats.md).

## What it must do

- [x] Compute base Str/Agi/Sta/Int from class, race and level 1–90.
- [x] Recompute on world entry and on level, spec, gear or aura-modifier change.
- [x] Set max health from stamina × gtHpPerSta plus aura health modifiers and max mana from gtBaseMP, keeping each current value at the same fraction of the new max except MOD_INCREASE_HEALTH 34: its fixed bonus is added/removed from both current and max health at aura change, with a living minimum of 1 on removal.
- [x] Expose spell power, attack power, crit, haste, mastery, versatility and armor for spell effects.
- [x] Expose base mana, which `PowerCostPct` mana costs use.
- [x] Count gear Str/Agi/Int only when they match the spec's main stat.
- [x] Fold in aura `StatModifiers` after base and gear.
- [x] Rescale existing characters' persisted health on login without changing the redb format.
- [x] Deliberate deviation from TrinityCore: its `Unit::SetMaxHealth` only clamps the current value and its login sets `min(saved, max)` (Player.cpp:18736); here ordinary max changes keep the percentage, at login and in session; MOD_INCREASE_HEALTH 34 instead follows TrinityCore's equal current/max bonus semantics. Retail keeps the percentage for in-session max-health changes (TrinityCore too, for MOD_INCREASE_HEALTH_PERCENT), and characters saved with the old 100-health placeholder rely on it. A level change still refills (Player::GiveLevel).

## Mastery base

Raw mastery points equal diminishing-return-adjusted mastery rating plus active aura318 points. The learned native mastery passive supplies its base once; no additional hardcoded base is granted. Removing/relearning that passive updates sheet mastery without retaining or duplicating its base. The active specialization's native coefficient converts raw points to displayed mastery.

## Mastery on approved followers

`MasteryAffectsPoints` effects must use the caster's live raw mastery points for players and approved NPC followers with class-derived stats. Refresh amounts when those points or active auras change, without restarting duration, stacks or periodic phase. Ordinary NPCs must not acquire player mastery scaling. Native70448 coefficients remain authoritative, including76671/76857 AP and168534 damage modifiers.

## How it works

- [Character stats](../wiki/systems/character-stats.md)

## Implementation inventory

- `crates/server/src/stats.rs`: `StatModifiers`, `CharacterStats`, `compute_character_stats`, `update_character_stats`.
- `crates/server/src/character_stats_data.rs`: generated tables.
- `scripts/gen_character_stat_tables.py`: the generator (SimC + CASC game tables).
- `crates/server/src/spell_info/class_data.rs`: `chr_classes` AP conversions.

## Tests asserting this spec

- `crates/server/src/stats_tests.rs`:
  - concrete numbers for a level 80 Human Holy Paladin, a level 10 Orc Warrior and level 90 ratings;
  - gear main-stat filtering;
  - modifier fold-in;
  - the real character-select path, with persisted-health rescale, spec mastery and admin set-level;
  - equal recompute does not change `Health`.
- `crates/server/src/spell_cast/tests.rs`: Fireball damage from computed stats; percent mana cost from base mana.

## Known gaps

- [ ] Phase 4 (inventory) blocker, gear stats: the server has no item stat data, so nothing produces `EquippedItems`.
- [ ] Phase 4 (inventory) blocker, weapon damage: there is no server weapon data.
- [ ] Spec/class passives, additional avoidance aura types beyond handled 49/51, and `octclasscombatratingscalar` are not modeled.
- [ ] `CharacterStats` is not replicated.
