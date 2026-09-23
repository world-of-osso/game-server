# Character Stats

`StatsPlugin` (`crates/server/src/stats.rs`) computes player stats in the retail 12.x model. Players spawn with default `UnitStats` and the health persisted in redb. `update_character_stats` then computes their stats on the first frame, and again whenever `Level`, `ActiveSpec`, `EquippedItems` or `StatModifiers` changes. It writes:

- the effective `UnitStats` and `CombatRatings`;
- `Health.max` and `Mana.max`, keeping each current value at the same fraction of the new max;
- the server-only `CharacterStats` component, read by the [[spell-casting]] effects and costs.

NPCs have no `CharacterStats`, so their spell power, attack power and crit are 0.

## Sources

| Data | Source | Status |
|---|---|---|
| Class base Str/Agi/Sta/Int per level 1–90 | SimC `engine/dbc/sc_extra_data.inc` `__gt_class_stats_by_level` | No DB2 or game table holds these; SimC measures them in game. See TODO gaps below. |
| Racial modifiers | SimC `__gt_race_stats`, mapped to ChrRaces IDs | Dracthyr, Earthen and Haranir are 0 in SimC and are not in the table. |
| Health per stamina | CASC `gametables/hppersta.txt` (FDID 1391642) | From the local 12.x install. |
| Base mana | CASC `gametables/basemp.txt` (FDID 1391664) | From the local 12.x install. |
| Rating per 1% (crit, haste, versatility) and per mastery point | CASC `gametables/combatratings.txt` (FDID 1391669): columns Crit - Spell, Haste - Spell, Mastery, Versatility - Damage Done | Melee, ranged and spell columns are equal. |
| Spec main stat | `chr_specialization.PrimaryStatPriority`: 0–1 Int, 2–3 Agi, 4–5 Str | |
| Mastery coefficient | Effect 0 `EffectBonusCoefficient` of the spec's `MasterySpellID_0`, for example Holy 183997 = 1.5 | |
| AP conversion | `chr_classes.AttackPowerPerStrength` / `AttackPowerPerAgility` | |

- SimC checkout: `simulationcraft/simc` commit `303610892fe2cd9c71a75eae89007c8d320788ff` (2026-04-05), client data 12.0.1.66709 (`client_data_version.inc`).
- The three CASC tables are byte-for-byte the values in SimC `sc_scale_data.inc` for build 12.0.1.66709: crit at level 80 is 12.158 and at 90 is 46.
- `octhpperstamina`, `octbasehpbyclass`, `octbasempbyclass` and `chancetospellcrit` are in the listfile but not in the local root.
- Retail has no base health. SimC `dbc_t::health_base` returns 0.
- `scripts/gen_character_stat_tables.py` generates `crates/server/src/character_stats_data.rs`. It reads a SimC checkout and `data/gametables/` (extracted with `casc-local`).

## Formulas

1. **Primary** = class base + racial modifier + gear + `primary_flat`, then × (1 + `primary_pct`/100).
   - Gear Str/Agi/Int count only when they match the spec's main stat. Gear stamina always counts.
2. **Max health** = floor(Sta) × hppersta(level) × (1 + `max_health_pct`/100), floored.
3. **Mana:** max mana = base mana = basemp(class, level). Intellect does not add mana.
   - Only classes with a non-zero basemp column get a `Mana` component: Paladin, Priest, Shaman, Mage, Warlock, Monk, Druid and Evoker.
4. **Spell power** = Int. **Attack power** = Str × AttackPowerPerStrength + Agi × AttackPowerPerAgility.
5. **Crit %** = 5 + DR(crit rating / rating-per-1%) + `crit_pct`.
6. **Haste %** = DR(haste rating / rating-per-1%) + `haste_pct`.
7. **Versatility %** = DR(versatility rating / rating-per-1%) + `versatility_pct`.
8. **Mastery %** = (8 + DR(mastery rating / rating-per-point) + `mastery_points`) × mastery coefficient.
9. **Armor** = gear armor rating.

DR is the server's `secondary_dr`, curve 21024 from SimC `item_scaling.inc` (12.0.1.66709), applied as in `player_t::apply_combat_rating_dr`. It is the identity up to 30%. Rating conversion and DR exist only in `stats.rs`; the server does not use `shared::formulas` rating functions.

Rating units follow SimC `rating_t::init`: the game-table value is the rating needed for 1%, or for 1 mastery point.

## SimC level 81–89 TODO gaps

SimC marks these class blocks TODO for the listed levels:

| Class | TODO levels |
|---|---|
| Warrior | 81–89 |
| Hunter | 81–89 |
| Rogue | 81–89 |
| Priest | 81–89 |
| Death Knight | 82–89 |
| Warlock | 81–89 |
| Demon Hunter | 81–89 |

- SimC still ships numbers for those rows. They follow the main-stat curve of the verified classes (Paladin, Shaman, Mage, Monk, Druid, Evoker), with off-stats at the class ratios in the block comments, for example Warrior "Agi = 0.69 of Primary".
- The server uses the SimC rows as-is and interpolates nothing. The TODO levels are unverified in game.

## Examples

**Level 80 Human Holy Paladin, no gear:**

| Stat | Value | Working |
|---|---|---|
| Str / Agi / Sta / Int | 159 / 56 / 2325 / 159 | Human modifiers are 0 |
| Health | 46 500 | 2325 × 20 |
| Mana | 50 000 | |
| Spell power | 159 | |
| Attack power | 159 | Str |
| Crit | 5% | |
| Haste | 0% | |
| Mastery | 12% | 8 × 1.5 |

**Level 10 Orc Warrior (Initial spec, no mastery), no gear:**

| Stat | Value | Working |
|---|---|---|
| Str / Agi / Sta / Int | 36 / 20 / 556 / 25 | 33/23/555/26 + Orc +3/−3/+1/−1 |
| Health | 1112 | 556 × 2 |
| Mana | none | |
| Attack power | 36 | |
| Crit | 5% | |

**Level 90 Paladin:** 460 crit rating gives 10% more crit, because 46 rating = 1%.

## Fold-in point for auras

`StatModifiers` is a Component with every field additive and a default of none:

- `primary_flat`, `primary_pct`
- `ratings_flat`
- `crit_pct`, `haste_pct`, `mastery_points`, `versatility_pct`
- `max_health_pct`

The aura runtime ([[auras]]) inserts or updates it, and `Changed<StatModifiers>` triggers a recompute. The pure entry point is `compute_character_stats(&StatInputs, Option<&StatModifiers>)`.

## Existing characters

The spawned `Health` comes from redb. The first recompute keeps the persisted ratio: a character saved at 50/100 enters as 23 250/46 500 at level 80, and full stays full. The periodic save writes the new max back, and the redb format is unchanged.

## Gaps

### Phase 4 (inventory) blockers

- **Gear:** the server has no item stat data. world.db has no ItemSparse or item-level budget tables, and equipment records carry only item_id, name and quality. `EquippedItems` stays the gear input, but nothing fills it in production.
- **Weapons:** there is no weapon data. Weapon hits keep the placeholder in [[spell-casting]].

### Other
- **Not modeled:**
  - spec and class passives (armor specializations, tank stamina, spec mana pools);
  - `octclasscombatratingscalar`;
  - multiplicative haste stacking;
  - dodge, parry and block from stats.
- **Replication:** `CharacterStats` is server-only. The client sees only `UnitStats`, `CombatRatings`, `Health` and `Mana`.
- **Shared formulas:** shared-protocol `formulas/rating_data.rs` interleaves a level column into every sixth entry and ends at level 80, and `rating_to_percent` divides by `raw × 100`, which is 100× too small.
  - The server no longer calls any of them.
  - They are scheduled for deletion in shared-protocol.

## Sources

- [stats.rs](../../../crates/server/src/stats.rs) — computation, components, system
- [stats_tests.rs](../../../crates/server/src/stats_tests.rs) — concrete-number and real enter-world/admin set-level tests
- [character_stats_data.rs](../../../crates/server/src/character_stats_data.rs) — generated tables
- [gen_character_stat_tables.py](../../../scripts/gen_character_stat_tables.py) — generator

## See Also

- [[spell-casting]] — reads spell power, attack power, crit and base mana
- [[class-progression]] — `ActiveSpec`, `Level`, admin set-level
- [[combat]] — melee formulas in shared
