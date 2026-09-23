# Spell Info

Server spell data model in `crates/server/src/spell_info/`. `SpellInfoPlugin` loads two read-only resources from `data/world.db` at startup: `SpellStore` (every spell) and `ClassSpells` (per-class spell lists). Source tables and build: [[world-data]] (DB2 12.1.0.69933). Casts are still presentation-only ([[networking]] cast presentation); nothing resolves effects.

## SpellStore

- One `SpellInfo` per `spell_name` row (414,027), in a Vec sorted by ID; `get` binary-searches it.
- Only DifficultyID 0 rows are loaded. Rows for IDs missing from `spell_name` are skipped.
- Values are raw DB2 values. Flags, masks and enum IDs are not translated. Times are in ms.
- Index columns resolve to their rows: cast time, duration (and PvP duration), range, effect radius, procs-per-minute, and empower stages.

| Field | Table | Notes |
|---|---|---|
| `name`, `subtext` | `spell_name`, `spell` | |
| `misc` | `spell_misc` | Attributes 0-16 and the full `SchoolMask`. Range `_0` is hostile and `_1` is friendly. |
| `effects` | `spell_effect` | Every `EffectIndex`, sorted. Recklessness 1719 has 15. |
| `power_costs` | `spell_power` | Every `OrderIndex`, including aura-gated rows (`RequiredAuraSpellID`). |
| `aura_options` | `spell_aura_options` | `CumulativeAura` (max stacks), proc chance/charges/type mask, and PPM. |
| `categories` | `spell_categories` | Category, GCD category, dispel, mechanic, defense and prevention types. |
| `cooldowns` | `spell_cooldowns` | Recovery, category recovery, and start recovery (GCD). |
| others | levels, class options, shapeshift, interrupts, target restrictions, equipped items, empower | Boxed because they are sparse. |

## Power cost storage

- `power_type` is the raw DB2 Powers ID: 0 Mana, 1 Rage, 3 Energy, 6 RunicPower, 7 SoulShards, 9 HolyPower, and so on.
- Rage, Runic Power and Soul Shards are stored in tenths:
  - Slam 1464: `ManaCost` 200 = 20 Rage.
  - Death Strike 49998: 450 = 45 Runic Power.
  - Chaos Bolt 116858: 20 = 2 shards.
- The loader keeps these values unscaled.
- Percent costs stay in `power_cost_pct`. Fireball 133 has 1.0, meaning 1% of base mana.

## Shared SpellData conversion

`SpellInfo::to_spell_data` narrows a spell for shared `validate_cast`. Shared `SpellData` holds one school, one cost and three effects.

- **Cost:** the first cost with no required aura.
  - Amount = flat cost + percent × caster max pool. The server has no separate base mana.
  - A power type with no shared `ResourceType` rejects the cast (`CostError`), as does a percent cost when the caster has no pool.
- **Effects:** mapped by effect ID.
  - 2 is damage, 10 is heal, and 30 is energize (the power type is in `EffectMiscValue_0`).
  - 6 is apply aura, 38 is dispel, and 68 is interrupt. The interrupt lockout is the spell duration: Kick 1766 gets 6 s.
- **School:** the highest-priority bit of the mask.

## ClassSpells

- Covers classes 1-9 and 11. The rules are the same as `scripts/spell_coverage.py` (see `docs/spell-coverage.md`).
- **Learnable spells:**
  - Taken from `skill_line_ability` rows on the class skill line.
  - `ClassMask` must be 0 or include the class bit.
  - `AcquireMethod` must not be 3.
  - `SpellLevel` must be at most 90. `required_level` = `spell_levels.SpellLevel`.
- **Specs:** `chr_specialization` supplies role, primary stat, flags and mastery spells. The spell list comes from `specialization_spells`.
- **Power types:** from `chr_classes_x_power_types`.
- **Passive:** learnable and spec spells carry `passive` (`spell_misc.Attributes_0 & 0x40`).
- [[class-progression]] turns these lists into per-player known spells.

## Load cost

Measured on the real DB in the test harness:

- Load time is 3.9 s in release and about 4.2 s in dev (deps at opt-level 2).
- RSS grows by about 275 MiB. The live heap estimate is 259 MiB.
- Struct sizes: `SpellInfo` 360 B, `SpellEffect` 164 B.

Two choices keep the cost down:

- Effect Vecs are reserved to exact size. Growth slack had cost about 190 MiB of RSS.
- The effect scan runs in table order and is sorted afterwards. The index-ordered scan was about 3x slower.

## Sources

- [spell_info/mod.rs](../../../crates/server/src/spell_info/mod.rs) — types, store, plugin
- [spell_info/load.rs](../../../crates/server/src/spell_info/load.rs) — SQL loaders
- [spell_info/class_data.rs](../../../crates/server/src/spell_info/class_data.rs) — class lists
- [spell_info/convert.rs](../../../crates/server/src/spell_info/convert.rs) — shared conversion
- [spell_info/tests.rs](../../../crates/server/src/spell_info/tests.rs) — real-DB assertions
