# Spell Info

## Native aura restrictions producer — 2026-10-16

The maintained `scripts/db2.py import` transaction now includes `SpellAuraRestrictions`, with all fifteen native columns in `spell_aura_restrictions` and the selected `db2_build` receipt. This retains caster/target state IDs, required/excluded spell references and aura types rather than silently discarding the provided CSV. Reimport replaces the prior rows through the normal schema transaction and validates required headers before mutation; non-DB2 tables remain outside replacement.

Native70391 fixtures include Victory Rush34428 caster-state10, Execute163201 target-state2, Massacre281000 target-state13, and170183's nonzero required/excluded aura-spell references. The real maintained CLI originally returned success while omitting all those rows (RED2pass/1fail,8.10s); targeted GREEN/publication checks pending. Test inputs now supply the active-build receipt required by the current CLI and use a genuinely missing required header for rollback coverage, not an allowed extra column. No canonical database was changed. Runtime `SpellInfo` loading/state admission is still separate and does not consume this new table yet; this is not Massacre/Execute gameplay certification. [Contract](../../specs/class-completeness.md); evidence `data/class-completeness6-2026-10-12/aura-restrictions-import-{goal,source,proof-ledger}.json`.

Server spell data model in `crates/server/src/spell_info/`. `SpellInfoPlugin` loads two read-only resources from `data/world.db` at startup: `SpellStore` (every spell) and `ClassSpells` (per-class spell lists). Source tables and build: [[world-data]] (DB2 12.1.0.69933). [[spell-casting]] resolves casts from these records.

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
| `categories` | `spell_categories` + `spell_category` | Category, GCD category, dispel, mechanic, defense and prevention types; `MaxCharges`/`ChargeRecoveryTime` of the charge category. |
| `cooldowns` | `spell_cooldowns` | Recovery, category recovery, and start recovery (GCD). |
| `labels` | `spell_label` | `LabelID`s, ascending; matched by label spell modifiers ([[auras]]). |
| others | levels, class options, shapeshift, interrupts, target restrictions, equipped items, empower | Boxed because they are sparse. |

## Power cost storage

- `power_type` is the raw DB2 Powers ID: 0 Mana, 1 Rage, 3 Energy, 6 RunicPower, 7 SoulShards, 9 HolyPower, and so on.
- Rage, Runic Power and Soul Shards are stored in tenths:
  - Slam 1464: `ManaCost` 200 = 20 Rage.
  - Death Strike 49998: 450 = 45 Runic Power.
  - Chaos Bolt 116858: 20 = 2 shards.
- The loader keeps these values unscaled.
- Percent costs stay in `power_cost_pct`. Fireball 133 has 1.0, meaning 1% of base mana.

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
- **Trait tree:** `ClassInfo.traits` is the class talent tree from `spell_info/trait_data.rs` ([[talents]]). Unknown node, edge or condition types fail the load.
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
- [spell_info/trait_data.rs](../../../crates/server/src/spell_info/trait_data.rs) — class trait trees
- [spell_info/tests.rs](../../../crates/server/src/spell_info/tests.rs) — real-DB assertions
