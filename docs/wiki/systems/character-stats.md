# Character Stats

`StatsPlugin` (`crates/server/src/stats.rs`) computes player stats and the approved [NPC follower bridge](../investigations/follower-data.md#passive-derived-stat-refresh--2026-10-15) in the retail 12.x model. Players spawn with default `UnitStats` and the health persisted in redb; followers initialize the same stat fields from their labelled class/spec and retain race0. `update_character_stats` computes both on the first frame and again when their level, spec/profile, equipment, known spells or `StatModifiers` changes. It writes:

- the effective `UnitStats` and `CombatRatings`, and `DerivedStats` (`CharacterStats::derived`: spell power, distinct melee/ranged attack power, the crit/haste/mastery/versatility percentages and leech/avoidance/speed);
- `Health.max` and `Mana.max`, keeping each current value at the same fraction of the new max;
- the server-only `CharacterStats` component, read by the [[spell-casting]] effects and costs.

Ordinary NPCs without `CharacterStats` still have no class-derived spell power, attack power or crit. Approved followers carry `CharacterStats` and the stat fields required by this same production query, while remaining NPCs rather than persisted players. Their absent `SheetOwner` keeps owner-only sheet fields hidden; health/mana replication remains ordinary.

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
| AP conversion | `chr_classes.AttackPowerPerStrength` / `AttackPowerPerAgility` / `RangedAttackPowerPerAgility` | |
| Gear stats, armor, weapon damage | world.db `content_item` + `item_sparse`, DB2 `rand_prop_points`, `item_armor_{quality,shield,total}`, `armor_location`, `item_damage_{one,two}_hand{,_caster}`, `item_squish_era`, `curve_point` (12.1.0.69933); CASC `gametables/{combatratingsmultbyilvl,staminamultbyilvl,itemsocketcostperlevel}.txt` (FDIDs 1391670, 1980632, 1391643) | See [Gear](#gear). |

- SimC checkout: `simulationcraft/simc` commit `303610892fe2cd9c71a75eae89007c8d320788ff` (2026-04-05), client data 12.0.1.66709 (`client_data_version.inc`).
- The three CASC tables are byte-for-byte the values in SimC `sc_scale_data.inc` for build 12.0.1.66709: crit at level 80 is 12.158 and at 90 is 46.
- `octhpperstamina`, `octbasehpbyclass`, `octbasempbyclass` and `chancetospellcrit` are in the listfile but not in the local root.
- Retail has no base health. SimC `dbc_t::health_base` returns 0.
- `scripts/gen_character_stat_tables.py` generates `crates/server/src/character_stats_data.rs`. It reads a SimC checkout and `data/gametables/` (extracted with `casc-local`).

## Formulas

1. **Primary** = class base + racial modifier + gear + `primary_flat`, then × (1 + `primary_pct`/100).
   - Gear Str/Agi/Int count only when they match the spec's main stat. Gear stamina always counts.
2. **Max health** = floor(Sta) × hppersta(level) × (1 + `max_health_pct`/100) + `max_health_flat`, floored.
3. **Mana:** max mana = base mana = basemp(class, level). Intellect does not add mana.
   - Only classes with a non-zero basemp column get a `Mana` component: Paladin, Priest, Shaman, Mage, Warlock, Monk, Druid and Evoker.
4. **Spell power** = Int when the main stat is Intellect, else 0 (TrinityCore Unit::SpellBaseDamageBonusDone). **Healing power** (`CharacterStats.healing_power`, used by heals, periodic heals and absorbs) = Int for every mana class (Unit::SpellBaseHealingBonusDone). Aura 366 OVERRIDE_SPELL_POWER_BY_AP_PCT sets both to AP × pct (Retribution, Protection, Feral/Guardian, Enhancement, Windwalker/Brewmaster passives). The main stat is the active spec's, else `chr_classes.PrimaryStatPriority`. **Melee AP** = (Str × AttackPowerPerStrength + Agi × AttackPowerPerAgility) × composed aura166 multiplier. **Ranged AP** = (level + max(Agi, 0)) × RangedAttackPowerPerAgility × composed aura167 multiplier (TC a352b1fa StatSystem.cpp354–414). Aura366 reads melee AP; native ranged-defense spell AP coefficients and hunter pet inheritance read RAP.
5. **Crit %** = 5 + DR(crit rating / rating-per-1%) + `crit_pct`.
6. **Haste %** = DR(haste rating / rating-per-1%) + `haste_pct`.
7. **Versatility %** = DR(versatility rating / rating-per-1%) + `versatility_pct`.
8. **Mastery %** = (8 + DR(mastery rating / rating-per-point) + `mastery_points`) × mastery coefficient.
9. **Armor** = gear armor rating.

DR is curve 21024 from SimC `item_scaling.inc` (12.0.1.66709), applied as in `player_t::apply_combat_rating_dr`. It is the identity up to 30%. Tertiary ratings (gear `ITEM_MOD_CR_SPEED` 61, `CR_LIFESTEAL` 62, `CR_AVOIDANCE` 63) convert with gtCombatRatings Speed/Lifesteal/Avoidance and diminish on curve 21025 (`DIMINISHING_RETURN_TERTIARY_CR_CURVE`, identity up to 10%); no content item carries one today. Rating conversion and DR exist only in `stats.rs`; the server does not use `shared::formulas` rating functions.

Rating units follow SimC `rating_t::init`: the game-table value is the rating needed for 1%, or for 1 mastery point.

## Gear

`item_stats.rs` computes each equipped item's stats; `ItemScalingTables` loads the DB2 tables at startup and the three item-level game tables are generated into `character_stats_data.rs`. Formulas are SimC `sc_item_data.cpp` (`scaled_stat`, `random_suffix_type`, `item_combat_rating_type`, `armor_value`, `weapon_dmg_min/max`) and `sc_const_data.cpp` `dbc_t::weapon_dps`; TrinityCore `Item::GetItemStatValue`, `ItemTemplate::GetArmor`/`GetDamage` read the same tables.

1. **Item level:** ItemSparse `ItemLevel`; items with `ItemSquishEraID` below 2 take the Midnight squish curve 92181 (1→1, 10→10, 342→70, …), rounded (SimC `item_t::download_item_data`). Black Duskwood Staff 21 → 12.
2. **Budget:** RandPropPoints float column for the level: Epic (quality 4-5), Superior (3, 7) or Good (rest); column 0 two-handers/head/chest/legs, 1 shoulders/waist/feet/hands/trinket, 2 neck/finger/cloak/wrist, 3 one-handers/off-hands.
3. **Stat** = floor(StatPercentEditor × budget × 0.0001 − round_half_even(StatPercentageOfSocket × socket cost) + 0.5), with ratings × gtCombatRatingsMultByILvl and stamina × gtStaminaMultByILvl for the slot column (armor, weapon, trinket, jewelry), multiplied before rounding.
   - Modeled `ITEM_MOD`s: 3 Agi, 4 Str, 5 Int, 7 Sta, 71–74 hybrids (added to each listed primary; the spec main-stat filter keeps one), 32 crit, 36 haste, 40 versatility, 49 mastery.
4. **Armor:** shields floor(ItemArmorShield[quality] + 0.5); cloth–plate floor(ItemArmorTotal[material] × ItemArmorQuality[quality] × ArmorLocation[slot][material] + 0.5), robe uses the chest row.
5. **Weapon:** DPS from ItemDamageOneHand/TwoHand (Caster variants with `ITEM_FLAG2_CASTER_WEAPON` 0x200; bows/guns/crossbows two-hand, wands one-hand caster); min = floor(DPS × speed × (1 − DmgVariance/2)), max = floor(DPS × speed × (1 + DmgVariance/2) + 0.5).

The main-hand result is `CharacterStats.main_hand` (unarmed 1–2 at 2.0 s, TrinityCore `BASE_MINDAMAGE`/`BASE_MAXDAMAGE`); an off-hand weapon is `CharacterStats.off_hand`. Auto-attacks swing both ([[player-damage]]). Spell weapon hits (effects 58, 121, 31) roll it and add AP / 6 × speed; 121 uses the normalized speed (dagger 1.7, one-hand 2.4, two-hand 3.3, ranged 2.8).

`EquippedItems.items` holds `(EquipmentSlot, ItemDefinitionSource, item_id)` of physically equipped items. Inventory publishes it on world entry and deltas. `EquippedItemModifiers.items` separately carries slot-bound instance properties; changing only those properties also triggers the production recompute.

### Instance properties — 2026-10-21

The additive redb `item_modifiers` table stores bonus lists, explicit item-level overrides, permanent enchants, gem item IDs and crafted stat choices by GUID. Existing `item_instances` soulbound bytes and inventory/wire encodings are unchanged. `InventoryRecord.item_modifiers` hydrates only owned GUIDs; the inventory model preserves those properties and projects them alongside equipped catalog identities. Startup creates the new table for existing databases.

The [native producer](../../../scripts/item_instance_data.py) reads the existing read-only 70448 exports into a supplied world database. Canonical publication remains lead-owned. The ordinary `scripts/db2.py import` invokes the same producer inside its existing transaction so native instance tables refresh with the core build. `ItemScalingPlugin` loads the new native bonus/enchantment resources; missing tables fail with the producer instruction.

| Resolution | Current coverage |
|---|---|
| ItemBonus | Types1/2/3/6/25/49/52: level delta, allocation, quality, sockets, crafted-stat replacement, Midnight config and crafting level. Type23 retains ItemEffect IDs; the SQLite resolver joins base/bonus effect IDs to real spell/trigger/cooldown fields. Native on-equip auras execute; scripted proc completeness remains open. |
| Explicit level / crafted choices | Existing RandPropPoints/weapon formula; no repeated legacy squish. Placeholders24/25 are replaced in order, never split into invented allocations. |
| Enchants / gems | Native type5 stats use the existing gtSpellScaling data and native level cap; GemProperties resolves the enchant. Physical-resistance type4/school0 contributes truncated, minimum-one armor (native8163 gives27 armor plus41 Strength/Agility); other resistance schools remain explicit gaps. SourceTC a352b1fa Player.cpp13509–13531. Type3 equip spells use the production aura path. Native8039 parent1236742 has authored BP0 but a nonzero item-scaling coefficient; LINKED_2 forwards the resolved amount to child1241710. |
| Inventory / stats | GUID persistence, owned projection, initial/delta delivery and modifier-only StatsPlugin refresh. Owned GUID hydration/projection passed after their genuine REDs, alongside modifier-only Stats and legacy source controls. |

Proof:18 library regressions pass (including three missing-effect RED→GREEN cases); the actual native chest271468/enchant7987 modifier-only boundary failed687 versus737 before consumer wiring, then passed alongside the unchanged level20 Arcane gear control. Two subsequent native GUID-hydration/projection cases gave genuine RED before implementation, then passed in the24-case library/main integration run. This is not DPS parity or a deployment claim; [the parity contract](../../specs/dps-parity.md) tracks the remaining harness and measurements.

Equipment qualification at447552507: native base268209, enchant8039, bonus13771 and item-level100→200 refresh all pass, including removal of linked children. LINKED/LINKED_2 follow TrinityCore a352b1fa `SpellAuraEffects.cpp5299–5335`: zero preserves child BP0, nonzero overrides it. The older test rejecting nonzero children contradicts that source; its replacement asserts native471492→471491 BP10 and392221→392223 BP8 plus owned removal. This does not qualify scripted procs or300s DPS.

Sources: [instance resolver](../../../crates/server/src/item_instance.rs), [bonus/enchant loaders](../../../crates/server/src/item_instance/load.rs), [GUID store](../../../crates/server/src/item_instance/store.rs), [inventory model](../../../crates/server/src/inventory_model.rs), [stat consumer](../../../crates/server/src/stats.rs). Pinned behavior reference: SimC76f8118 `sc_item_data.cpp` and `item.cpp1740`.

**Checked against Retail tooltips** (Wowhead tooltip API, 2026-09-24):

| Item | Level | Tooltip | Server |
|---|---|---|---|
| Notched Shortsword 727 | 5 | 1–2, 2.60, +1 Agi, +1 Sta | same |
| Black Duskwood Staff 937 | 21 → 12 | 6–9, 3.60 | same |
| Vendetta 776 | shown at 83 | 13–23, 1.80, +13 Agi, +13 crit | same at 83 |
| Forlorn Breastplate of War 35588 | shown at 34 | 26 armor, +8 Str/Int, +12 Sta, +15 crit | same at 34; 15 → 4/6/8, 18 armor unscaled |

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
- `max_health_pct`, `max_health_flat`
- `dodge_pct`, `block_pct`

MOD_INCREASE_HEALTH 34 changes current/max health by the same flat delta on aura change, rather than rescaling current health. Aura 49 adds nondiminishing player dodge points. [Defensives audit](../../defensives-audit.md) records catalog amounts, lifecycle tests and remaining gaps.

The aura runtime ([[auras]]) inserts or updates it, and `Changed<StatModifiers>` triggers a recompute. The pure entry point is `compute_character_stats(&StatInputs, Option<&StatModifiers>)`.

## Existing characters

The spawned `Health` comes from redb. The first recompute keeps the persisted ratio: a character saved at 50/100 enters as 23 250/46 500 at level 80, and full stays full. The rescaled value is truncated to whole points (999/1000 → 1108/1110), as TrinityCore's integer health. The periodic save writes the new max back, and the redb format is unchanged.

## Gaps

### Gear

- **Scaled item levels:** Wowhead shows some old items at a level other than ItemSparse `ItemLevel` (Vendetta 20 at 83, Riverpaw Leather Vest 6 at 80, Forlorn Breastplate 35 at 34). The source of that scaling (drop context bonus lists, ContentTuning, `PlayerLevelToItemLevelCurveID`) is not identified; the server uses the static, squished level. Explicit instance bonus lists use the qualified types [above](#instance-properties--2026-10-21); automatic bonus-tree/drop-context selection remains absent.
- **Not fully modeled:** scripted equipment procs, the effects of leech, avoidance and speed (only their sheet percentages), bonus armor, ranged swings. Explicit gem/enchant stats use the qualified instance path [above](#instance-properties--2026-10-21). Auto-attacks swing `main_hand` and `off_hand` ([[player-damage]]).
- **Admin grant:** `grant-equipment` writes equipment without the inventory change queue, so an online player's `EquippedItems` updates at the next inventory change or relog.

### Other
- **Not modeled:**
  - authentic NPC follower gear/stat-budget parity (the approved bridge applies sourced class/spec passive modifiers, not invented NPC budgets);
  - `octclasscombatratingscalar`;
  - multiplicative haste stacking;
  - additional dodge/parry/block aura types beyond handled 49/51.
- **Replication:** `CharacterStats` is server-only. `Health` and `Mana` replicate to every observer; `UnitStats`, `CombatRatings` and `DerivedStats` only to the owning connection (`sheet_visibility.rs`: `SheetOwner(connection)`, set on `PlayerLink` insert, is a replicon visibility filter scoped to those three components). They feed the client's character sheet and its spell tooltips (spell/attack power scaled effect points).
- **Shared formulas:** shared-protocol `formulas/rating_data.rs` interleaves a level column into every sixth entry and ends at level 80, and `rating_to_percent` divides by `raw × 100`, which is 100× too small.
  - The server no longer calls any of them.
  - They are scheduled for deletion in shared-protocol.

## Sources

- [stats.rs](../../../crates/server/src/stats.rs) — computation, components, system
- [stats_tests.rs](../../../crates/server/src/stats_tests.rs) — concrete-number and real enter-world/admin set-level tests
- [item_stats.rs](../../../crates/server/src/item_stats.rs) — item stats, armor, weapon damage, scaling tables
- [item_stats_tests.rs](../../../crates/server/src/item_stats_tests.rs) — real-item tooltip tests
- [character_stats_data.rs](../../../crates/server/src/character_stats_data.rs) — generated tables
- [gen_character_stat_tables.py](../../../scripts/gen_character_stat_tables.py) — generator

## See Also

- [[spell-casting]] — reads spell power, attack power, crit and base mana
- [[class-progression]] — `ActiveSpec`, `Level`, admin set-level
- [[combat]] — auto-attack systems
- [[player-damage]] — how AP, SP, crit, haste, versatility and weapons become damage
