# Character stats

Players get retail 12.x stats on the server. See [character stats](../wiki/systems/character-stats.md).

## What it must do

- [x] Compute base Str/Agi/Sta/Int from class, race and level 1–90.
- [x] Recompute on world entry and on level, spec, gear or aura-modifier change.
- [x] Set max health from stamina × gtHpPerSta and max mana from gtBaseMP, keeping each current value at the same fraction of the new max.
- [x] Expose spell power, attack power, crit, haste, mastery, versatility and armor for spell effects.
- [x] Expose base mana, which `PowerCostPct` mana costs use.
- [x] Count gear Str/Agi/Int only when they match the spec's main stat.
- [x] Fold in aura `StatModifiers` after base and gear.
- [x] Rescale existing characters' persisted health on login without changing the redb format.
- [x] Deliberate deviation from TrinityCore: its `Unit::SetMaxHealth` only clamps the current value and its login sets `min(saved, max)` (Player.cpp:18736); here every max change keeps the percentage, at login and in session. Retail keeps the percentage for in-session max-health changes (TrinityCore too, for MOD_INCREASE_HEALTH_PERCENT), and characters saved with the old 100-health placeholder rely on it. A level change still refills (Player::GiveLevel).

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
- [ ] Spec/class passives, dodge/parry/block from stats, and `octclasscombatratingscalar` are not modeled.
- [ ] `CharacterStats` is not replicated.
