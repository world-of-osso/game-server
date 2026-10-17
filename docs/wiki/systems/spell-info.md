# Spell Info

## Native aura restrictions producer — 2026-10-16

The maintained `scripts/db2.py import` transaction now includes `SpellAuraRestrictions`, with all fifteen native columns in `spell_aura_restrictions` and the selected `db2_build` receipt. This retains caster/target state IDs, required/excluded spell references and aura types rather than silently discarding the provided CSV. Reimport replaces the prior rows through the normal schema transaction and validates required headers before mutation; non-DB2 tables remain outside replacement.

Native70391 fixtures include Victory Rush34428 caster-state10, Execute163201 target-state2, Massacre281000 target-state13, and170183's nonzero required/excluded aura-spell references. The real maintained CLI originally returned success while omitting all those rows (RED2pass/1fail,8.10s); atff8b9323 the three producer tests pass8.483s; current merged mastercbeea718 passes3/0/0 in10.792s. Maintained70391 CSV/schema check passes; workspace/all-target check passes66.132s/0warnings with landed protocol5405246, fmt and manual Python/SQL source/readability audit pass. Canonical publication remains lead-owned and pending; read-only checks confirm the table is absent in both slot and canonical databases. Test inputs now supply the active-build receipt required by the current CLI and use a genuinely missing required header for rollback coverage, not an allowed extra column. No canonical database was changed. The runtime metadata loader below now reads this producer's table; health-state production and admission remain separate. This is not Massacre/Execute gameplay certification. [Contract](../../specs/class-completeness.md); evidence `data/class-completeness6-2026-10-12/aura-restrictions-import-{goal,source,proof-ledger}.json`.

Server spell data model in `crates/server/src/spell_info/`. `SpellInfoPlugin` loads two read-only resources from `data/world.db` at startup: `SpellStore` (every spell) and `ClassSpells` (per-class spell lists). Source tables and build: [[world-data]] (DB2 12.1.0.69933). [[spell-casting]] resolves casts from these records.

## Native aura restrictions metadata — 2026-10-16

`SpellInfo.aura_restrictions` is a sparse boxed record of all twelve raw required/excluded caster/target state, spell and aura-type IDs. Startup's production `load_spell_store` loads only DifficultyID0 rows, using the existing named-spell lookup; absent rows remain `None` and orphan restrictions do not create spells. Missing table/columns propagate SQL errors: publish the maintained producer's table before deploying this reader, rather than silently starting with unrestricted metadata.

Native70391 loader fixtures retain Victory Rush34428/state10, Execute163201/state2, Massacre281000/state13,170183's four spell references, and nonzero required/excluded aura types. Genuine dropped-metadata/schema RED0pass/4fail precedes production9b39ca6f; GREEN4/0/0 in0.03s. Merged master7529db68 passes39/0/0 in11.45s (four new cases plus35 native loader/form/casting-restriction controls). The maintained importer populated only the owned slot DB in31.269s; all10675 rows and its build receipt equal native70391, with a persistent preimport backup. Canonical publication remains lead-owned; workspace/all-target check passes159.051s with0 warnings, fmt and manual changed-Rust readability pass. Protocol5405246 matches the tested master; no protocol behavior changed. The loader itself does not enforce restrictions. [Shared aura-spell admission](spell-casting.md#native-required-caster-aura-spell-admission--2026-10-16) consumes the required-caster and two excluded-spell fields separately at existing ordinary routes; [target type admission](spell-casting.md#native-target-aura-type-admission--2026-10-16) consumes the two target type fields. Target-required references, aura-state production/timing/aura262 bypass and caster types stay open. All CasterAuraType values in the inspected native build are zero; that column has no nonzero native fixture claim. [Contract](../../specs/class-completeness.md); source/proof receipts `data/class-completeness6-2026-10-12/aura-restrictions-load-{goal,source,proof-ledger}.json`.

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
| `aura_restrictions` | `spell_aura_restrictions` | Twelve raw required/excluded state, spell and aura-type IDs; metadata only, not admission. |
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
