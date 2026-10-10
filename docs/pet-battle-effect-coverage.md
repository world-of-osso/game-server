# Pet battle effect-property coverage

Verified2026-10-10; source local-CASC12.1.0.69933; code2f99296.

**12/104 property handlers** (2008/2829 rows reference those IDs). Recursive admission accepts657/1404 ability records and930/2829 effect rows. Admission is static gating, **not behavioral verification of every accepted ability**; empty/passive records are included.23 targeted tests pass. Unsupported IDs fail explicitly.

| Property ID | Rows | Implemented | Test |
|---:|---:|---|---|
| 24 | 945 | Partial;434 rows admitted in complete graphs | Bite110 type/crit; Poison Fang152; Deep Breath169 |
| 26 | 244 | Partial;120 rows admitted in complete graphs | Burrow159; Metabolic Boost194 refresh/expiry |
| 50 | 194 | Partial;76 rows admitted in complete graphs | Curse of Doom218 delayed impact |
| 49 | 138 | Partial;16 rows admitted in complete graphs | Burrow159 removes Underground340 |
| 23 | 115 | Partial;78 rows admitted in complete graphs | Healing Wave123; healing modifiers/cooldown |
| 222 | 90 | Partial;70 rows admitted in complete graphs | Burn113 variance; Burrow159 second turn |
| 31 | 86 | Partial;33 rows admitted in complete graphs | Deep Breath169 multi-turn state sequence |
| 54 | 77 | Partial;55 rows admitted in complete graphs | Poison Fang152 five ticks/missed chain; Aquatic modifiers |
| 158 | 69 | No (explicit rejection) | — |
| 22 | 61 | Partial;20 rows admitted in complete graphs | Time Stop600; Curse218 (dummy rounds) |
| 116 | 57 | No (explicit rejection) | — |
| 52 | 44 | Partial;20 rows admitted in complete graphs | Time Stop600 skip/Resilient; Grasp249 root |
| 28 | 29 | No (explicit rejection) | — |
| 85 | 29 | No (explicit rejection) | — |
| 79 | 28 | No (explicit rejection) | — |
| 177 | 28 | No (explicit rejection) | — |
| 135 | 27 | No (explicit rejection) | — |
| 32 | 25 | No (explicit rejection) | — |
| 80 | 24 | No (explicit rejection) | — |
| 178 | 24 | No (explicit rejection) | — |
| 29 | 23 | No (explicit rejection) | — |
| 62 | 23 | No (explicit rejection) | — |
| 384 | 23 | No (explicit rejection) | — |
| 141 | 22 | No (explicit rejection) | — |
| 27 | 21 | No (explicit rejection) | — |
| 136 | 19 | No (explicit rejection) | — |
| 145 | 18 | No (explicit rejection) | — |
| 373 | 17 | No (explicit rejection) | — |
| 139 | 15 | No (explicit rejection) | — |
| 157 | 14 | No (explicit rejection) | — |
| 111 | 13 | No (explicit rejection) | — |
| 179 | 13 | No (explicit rejection) | — |
| 229 | 13 | No (explicit rejection) | — |
| 246 | 12 | No (explicit rejection) | — |
| 68 | 11 | No (explicit rejection) | — |
| 104 | 11 | No (explicit rejection) | — |
| 53 | 10 | Partial;5 rows admitted in complete graphs | Wish273 heal50%; Humanoid Recovery726 heal4% |
| 159 | 10 | No (explicit rejection) | — |
| 0 | 9 | No (explicit rejection) | — |
| 107 | 9 | No (explicit rejection) | — |
| 131 | 9 | No (explicit rejection) | — |
| 156 | 9 | No (explicit rejection) | — |
| 363 | 9 | No (explicit rejection) | — |
| 150 | 8 | No (explicit rejection) | — |
| 76 | 7 | No (explicit rejection) | — |
| 96 | 7 | No (explicit rejection) | — |
| 100 | 6 | No (explicit rejection) | — |
| 168 | 6 | No (explicit rejection) | — |
| 170 | 6 | No (explicit rejection) | — |
| 226 | 6 | No (explicit rejection) | — |
| 25 | 5 | No (explicit rejection) | — |
| 121 | 5 | No (explicit rejection) | — |
| 122 | 5 | No (explicit rejection) | — |
| 44 | 4 | No (explicit rejection) | — |
| 59 | 4 | No (explicit rejection) | — |
| 63 | 4 | Partial;3 rows admitted in complete graphs | Wish273 heals swapped active pet |
| 112 | 4 | No (explicit rejection) | — |
| 169 | 4 | No (explicit rejection) | — |
| 172 | 4 | No (explicit rejection) | — |
| 372 | 4 | No (explicit rejection) | — |
| 385 | 4 | No (explicit rejection) | — |
| 66 | 3 | No (explicit rejection) | — |
| 128 | 3 | No (explicit rejection) | — |
| 129 | 3 | No (explicit rejection) | — |
| 230 | 3 | No (explicit rejection) | — |
| 248 | 3 | No (explicit rejection) | — |
| 370 | 3 | No (explicit rejection) | — |
| 77 | 2 | No (explicit rejection) | — |
| 117 | 2 | No (explicit rejection) | — |
| 123 | 2 | No (explicit rejection) | — |
| 124 | 2 | No (explicit rejection) | — |
| 138 | 2 | No (explicit rejection) | — |
| 149 | 2 | No (explicit rejection) | — |
| 194 | 2 | No (explicit rejection) | — |
| 197 | 2 | No (explicit rejection) | — |
| 223 | 2 | No (explicit rejection) | — |
| 249 | 2 | No (explicit rejection) | — |
| 254 | 2 | No (explicit rejection) | — |
| 255 | 2 | No (explicit rejection) | — |
| 256 | 2 | No (explicit rejection) | — |
| 360 | 2 | No (explicit rejection) | — |
| 45 | 1 | No (explicit rejection) | — |
| 61 | 1 | No (explicit rejection) | — |
| 65 | 1 | No (explicit rejection) | — |
| 67 | 1 | No (explicit rejection) | — |
| 72 | 1 | No (explicit rejection) | — |
| 74 | 1 | No (explicit rejection) | — |
| 75 | 1 | No (explicit rejection) | — |
| 78 | 1 | No (explicit rejection) | — |
| 86 | 1 | No (explicit rejection) | — |
| 97 | 1 | No (explicit rejection) | — |
| 137 | 1 | No (explicit rejection) | — |
| 140 | 1 | No (explicit rejection) | — |
| 144 | 1 | No (explicit rejection) | — |
| 165 | 1 | No (explicit rejection) | — |
| 171 | 1 | No (explicit rejection) | — |
| 201 | 1 | No (explicit rejection) | — |
| 204 | 1 | No (explicit rejection) | — |
| 358 | 1 | No (explicit rejection) | — |
| 359 | 1 | No (explicit rejection) | — |
| 362 | 1 | No (explicit rejection) | — |
| 374 | 1 | No (explicit rejection) | — |
| 376 | 1 | No (explicit rejection) | — |
| 380 | 1 | No (explicit rejection) | — |

## Proof levels

- Data/stat formulas: real CSV→SQLite; species39 and duplicate-state species4597 numeric fixtures.
- Turn/effect/passive behavior: CPU deterministic fixtures with real ability IDs; synthetic fixed stats isolate formulas. No live/captured Retail numeric comparison.
- Unsupported:92 property IDs; object/team damage, weather, target overrides, conditional/opcode branching, proc events1..4/9/12, special/max-health states.
- Elemental passive: preserved state47; weather cannot execute, so immunity is not behaviorally proved.
- Family passives covered: Beast, Flying, Critter, Magic, Mechanical, Undead, Humanoid, Aquatic, Dragonkin.
- Runtime SQLite producer pending: pets-server-fix currently has species only. No runtime CSV fallback.
- Undefined species metadata:35 raw state rows preserved in Catalog.unresolved_states, not fabricated.

## Source boundaries

Current local family ability236..245 descriptions define family passives and displayed strong50%/weak33%. Weak factor currently.67 from the displayed33%; precise Retail game-table factor and damage/control rounding require captured/client game-table proof. Stats divisor20/100 and health offset100 come from pinned TrinityCore a352b1fa; float32 precision is preserved. Retail tooltip aliases define power.05 and explicit variance spread. Blizzard5.3 accuracy article defines critical5%/1.5x and basevariance+/-10%; legacy LegionCore has conflicting2x crit and is not followed. Opcode references are evidence of dispatch shape, not universal Retail parity.

## Reproduction

Server basec2774c5 and shared master75fe6a4 are aligned. Shared current3facd18 is incompatible because Disenchant added target_item_guid; declared git pin8c16175 predates Toy Box messages. Use isolated unchanged shared checkout75fe6a4.

```text
PET_BATTLE_DB2=/home/osso/Projects/world-of-osso/game-engine/data/db2/12.1.0.69933
agent-run petbattle cargo --config 'patch."https://github.com/world-of-osso/shared-protocol.git".shared.path="/home/osso/.worktrees/shared-protocol-petbattle"' test --locked -p server pet_battle -- --nocapture
```

Logs: game-engine/data/diagnostics/petbattle-engine-20261010/{turn-red,lifecycle-red,resilient-red,final-aligned-targeted,cargo-check,fmt-check}.log.
