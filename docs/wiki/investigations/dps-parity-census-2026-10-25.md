# All-DPS parity census — 2026-10-25

Measured revision: `210df12e3cddb9e99760798e0648e0173b30b2ce` (origin/master at checkout creation). Private SQLite backup of current canonical world.db; canonical data was not changed. No surrogate follower profiles or mechanics changes.

## Results

Percentages are native DPS / pinned SimC DPS ×100 over300s, seed70448, on one and three targets. Official followers use `--actor follower`; specs without an official follower use `--actor player` and the supplied AssistedCombat70448 input. A dash means no valid measurement, not zero DPS. Identical ST/3T failures are shown once.

| Spec | Actor | ST native/ref | 3T native/ref | Result |
|---|---|---:|---:|---|
| 62 Arcane | player | — | — | NATIVE_ASSISTED_COMBAT_GAP: spec62: rule 72080: equality comparator inclusivity is unverified |
| 63 Fire | follower | 17.44% | 16.23% | Measured |
| 64 Frost Mage | player | — | — | NATIVE_ASSISTED_COMBAT_GAP: spec64: AssistedCombatRule 4512: condition 11 has unverified numeric/predictive semantics |
| 70 Retribution | player | — | — | NATIVE_ASSISTED_COMBAT_GAP: spec70: AssistedCombatRule 6369: condition 12 has unverified numeric/predictive semantics |
| 71 Arms | player | — | — | NATIVE_ASSISTED_COMBAT_GAP: spec71: AssistedCombatRule 1187: condition 12 has unverified numeric/predictive semantics |
| 72 Fury | player | 40.68% | 75.58% | Measured |
| 102 Balance | — | — | — | No reference JSON in supplied diagnostics |
| 103 Feral | follower | 38.39% | 34.57% | Measured |
| 251 Frost DK | player | — | — | NATIVE_REFERENCE_TRAIT_CONFIG_REJECTED: node 76073 requires one of nodes [76072] fully ranked |
| 252 Unholy | player | — | — | reference profile lacks its class namespace |
| 253 Beast Mastery | follower | 27.36% | 22.81% | Measured |
| 254 Marksmanship | player | — | — | NATIVE_ASSISTED_COMBAT_GAP: spec254: AssistedCombatRule 21006: condition 11 has unverified numeric/predictive semantics |
| 255 Survival | player | — | — | NATIVE_PLAYER_CAST_REJECTED: spec255 spell883: Pet(NoPetAvailable) |
| 258 Shadow | player | — | — | NATIVE_ASSISTED_COMBAT_GAP: spec258: rule 95097: equality comparator inclusivity is unverified |
| 259 Assassination | player | — | — | NATIVE_ASSISTED_COMBAT_GAP: spec259: AssistedCombatRule 413: condition 12 has unverified numeric/predictive semantics |
| 260 Outlaw | player | — | — | NATIVE_ASSISTED_COMBAT_GAP: spec260: AssistedCombatRule 19535: condition 12 has unverified numeric/predictive semantics |
| 261 Subtlety | player | — | — | NATIVE_ASSISTED_COMBAT_GAP: spec261: AssistedCombatRule 20449: condition 12 has unverified numeric/predictive semantics |
| 262 Elemental | follower | 13.28% | 8.91% | Measured |
| 263 Enhancement | player | 4.31% | 3.59% | Measured |
| 265 Affliction | player | — | — | reference actor lacks declared native power SoulShards |
| 266 Demonology | follower | 6.52% | 3.93% | Measured |
| 267 Destruction | player | — | — | reference actor lacks declared native power SoulShards |
| 269 Windwalker | follower | 50.00% | 66.03% | Measured |
| 577 Havoc | player | — | — | reference profile lacks its class namespace |
| 1467 Devastation | follower | — | — | NATIVE_REFERENCE_TRAIT_CONFIG_REJECTED: node 98931 is not available: condition 25886 requires a spec in spec set 174 [1473] |
| 1473 Augmentation | — | — | — | No reference JSON in supplied diagnostics |
| 1480 Devourer | player | — | — | NATIVE_REFERENCE_TRAIT_CONFIG_REJECTED: node 108144 requires one of nodes [110172] fully ranked |

## Measurement inputs

- Worktree: `/home/osso/.worktrees/game-server-parity-all`.
- Private database: `data/world.db`; logs and JSON: `parity-output/<spec>.native.<1|3>.{log,json}` in that worktree.
- Exact manifests: engine `data/diagnostics/dps-parity-2026-10-21/all-spec-current-reference-manifests/`; effective traits: `<spec>.selected-traits-active.json` from the diagnostics root. No trait subsets were substituted.
- Expanded references: `all-spec-expanded-references/`. Existing cohort references come from the diagnostics root, using corrected `reference-fixed.three` reports rather than historical broken three-target reports.
- Player input: engine `data/diagnostics/dps-parity-2026-10-21/native-player-inputs/assisted-70448/`, supplied through `DPS_PARITY_ASSISTED_DB2`.

Native follower specs are63/66/73/103/105/253/257/262/266/269/1467. All other reference-bearing DPS specs were attempted as followers first, then as players; no missing profile was replaced. Fire and Feral were retried after adding the missing installed-data symlinks. Eight specs produced measured DPS,17 hit the exact setup/implementation boundaries above, and two have no supplied reference. Tank/healer references are outside this DPS census.

## Sources

- [DPS parity contract](../../specs/dps-parity.md).
- [Measurement runner](../../../scripts/dps_parity_measure.py).
- [Native input contract](../../../crates/server-simulation/src/dungeon_simulator/dps_parity/inputs.rs).

## See Also

- [Dungeon simulator](../systems/dungeon-simulator.md).
