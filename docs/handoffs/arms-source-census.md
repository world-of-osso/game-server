# Arms71 DPS parity source census

Status: source inventory and targeted Execute regression only; native parity is **not qualified**.

## Authority and reference

- Native DB2/TC outrank SimC. Clean-room sources only.
- Rotation: literal SimC `76f8118cf3f7f41dad20708e8328e1f4ab2deebc`, `engine/class_modules/apl/apl_warrior.cpp:153–329`, through shared `server::simc_apl` and native ActionFacts, fixture encounters only. No production rotation replacement.
- Profile: `profiles/MID2/MID2_Warrior_Arms.simc`, SHA-256 `847080503d43467467dc6102416f61397f9f2a9cbb2160da614b636a7e088afc`.
- Pinned SimC executable SHA-256 `6415c116d22e191d436eea7e5e4485fb49ac47c265fa8706f4e7b43b4dbabba4`.
- Fresh read-only SQLite backup of canonical engine `data/server-world/world.db`: SHA-256 `51455391e7d4962589cc66d239bf77b2dbf3f4372ec8daee80c092f1e7989d9b`. Old v5/Ret snapshots are historical provenance, not current inputs.

Exact argv, raw/normalized reports, profile and selected traits live under engine `data/diagnostics/arms-parity-2026-10-23/`. Reproduced 1,000 iterations, 300 fixed seconds, seed70448, target level93, no optimal raid buffs, local item sources:

|Scenario|SimC DPS|Acceptable native DPS|Native status|
|---|---:|---:|---|
|ST / Patchwerk|188596.272|169736.645–207455.899|Not measured|
|3T / main + two recurring adds (timing finding below)|328901.678|296011.511–361791.846|Not measured|

## Shared v5 timing finding

Arms/Ret70/WW269/Dev1467 receipts all use `desired_targets=1` with two adds, first0/cooldown30/duration30 and zero variance. Native fixtures atomically replace both adds every30.000s. However, pinned `raid_event.cpp:2700–2701` clamps cooldown≤duration to duration+0.01s. The retained one-iteration `71.three-uptime-audit.*` log proves starts0/30.010/60.020 through270.090; each add is active299.91/300s (99.97%). This is a quantified common timing mismatch, not an Arms-specific scenario difference; no reference has been silently replaced.

Fixture boss classification must also match the reference: `sim.cpp:2783–2793` creates `desired_targets=3` as TANK_DUMMY plus ENEMY actors (all boss=true under `player.hpp:964`). This Arms reference instead uses ENEMY_ADD (`raid_event.cpp:77,269`), so only the main target is a boss. Identical native31146 templates cannot encode that distinction; explicit fixture metadata must not mutate their native creature rank.

## Coverage

`arms-source-census-v1.json` and `arms-spell-matrix-v1.md` contain 170 spell IDs from76 selected-trait rows, specialization grants,52 source-checked action/operand binding IDs, reference damage channels and authored child closure. Ambiguous NPC spell-name matches are excluded from the matrix; binding runtime behavior remains unqualified. `arms-apl-condition-ledger-v1.json` retains all148 source rows; `arms-aura-census-v1.json` inventories67 aura types. Each spell retains DB2 effects, all other difficulty rows, costs, cooldowns, aura options, PPM, descriptions, TDB signed bindings/procs, TC declarations and SimC channel shares. All runtime cells remain unmeasured; source presence never substitutes for a cast/effect test.

|Channel|ST share|3T share|Current source finding|
|---|---:|---:|---|
|Mortal Strike, including Unhinged child|19.7634%|9.5953%|Native direct damage/wound handler exists; Unhinged producer unqualified|
|Execute260798|14.0028%|13.7987%|Native parent forwards child/refunds; TC optional-Rage damage multiplier absent from resource snapshot; behavioral RED queued|
|Slayer's Strike445579|10.2663%|7.7597%|Native damage data exists; structured producer child binding absent|
|Bladestorm50622|8.1966%|13.1306%|Native periodic trigger data exists; full channel/Unhinged behavior unqualified|
|Reap the Storm446005|6.6025%|16.2322%|Native damage data exists; structured conditional producer absent|

Deep Wounds262115 additionally contributes7.7695% ST/6.0267%3T. Its selected1261060 producer has authored Aura42→262115 and native proc mask16; generic eligibility/actual application still requires native measurement.

## Named-policy audit

`arms-named-policy-candidates-v1.json` records exact SimC passages and fresh native data for444767/444775/386628. None has other-difficulty effect rows, PPM rows, TDB proc overrides or root script binding. Entire pinned TC `a352b1fa` server source has no root-ID/name matches; the ordinary12294 Mortal Strike wound script does not implement Unhinged. Root EffectTriggerSpell never links these producers to their named damage children.

Candidate policies, not implementations:

- `ARMS_SLAYER_STRIKE_SIMC_POLICY`: `sc_warrior.cpp:1640–1704`, Arms effect2 chance and increasing-attempt pseudo-random roll; native ProcChance101/mask4112/500ms ICD do not encode the dispatch or attempt distribution.
- `ARMS_REAP_STORM_SIMC_POLICY`: `sc_warrior.cpp:1843–1868,3708–3724,4344–4364`, Cleave target threshold and Imminent Demise/Sudden Death paths. Native description corroborates conditional chance, not authority for the named policy.
- `ARMS_UNHINGED_SIMC_POLICY`: `sc_warrior.cpp:3053–3068,3252–3256,3304–3338`, background Mortal Strike and odd Bladestorm ticks including initial-tick convention. Native effects supply2-strike cadence/100% effectiveness, but no structured child dispatch.

TC Execute `spell_warrior.cpp:747–835` supplies native raw base/optional costs and multiplier `1 + (DamageRageSpent - BaseRageCost) / OptionalRageCost`. Sudden Death's explicitly-applied cost modifier can substitute its talent amount×10 for damage scaling while actual paid Rage remains zero. This is TC behavior, not a SimC policy.

## Remaining proof

- Existing native-kit ST/3T baseline and gap attribution, including any explicit AssistedCombat-rule rejection rather than a fabricated zero-DPS value.
- Shared literal-driver integration after WW GREEN; no evaluator/ActionFacts fork.
- Behavioral RED→GREEN per accepted fix, ordered by measured channel share and preferring generic causes.
- Final matched-input ST/3T reports, focused test receipts, pushed tip and lead-owned full integration gate.
