# DPS parity status — 2026-10-29

Snapshot of the 25 DPS specs with approved pinned profiles. Train data is ranked by the first-parent master commit, not report modification time. Each cell uses the newest successful archived literal-rotation report for that target count on master through `8ccdf3012e`; commit and train entry identify its source. Older commits than `2a49eee69c` are fallback measurements. Local diagnostic results are listed separately below and do not replace train measurements.

SimC `76f8118` references use 300 seconds, fixed time, 1,000 iterations and seed 70448; Devastation uses the approved MID1 profile. The 3T scenario uses the corrected add-event run, not the invalid Patchwerk files that duplicate single target. The expanded-reference runs omit Patchwerk and include the add event. Gap = `(native / SimC - 1) × 100`; **OUT** means either absolute gap exceeds 5%, or 3T data is missing. Native results use the seeds and actor identity recorded in their archived JSON; assisted combat is not a valid substitute for a literal rotation.

| Spec | 1T native | 1T SimC | 1T gap | 1T master / entry | 3T native | 3T SimC | 3T gap | 3T master / entry | Status |
|---|---:|---:|---:|---|---:|---:|---:|---|---|
| Arcane Mage (62) | 183,497 | 177,681 | +3.27% | `6cd8b489e1` / `ed6300d16b` | 327,637 | 330,648 | -0.91% | `6cd8b489e1` / `ed6300d16b` | in band |
| Fire Mage (63) | 199,463 | 197,191 | +1.15% | `ca8106b09b` / `7b4bbf84dd` | 371,316 | 371,059 | +0.07% | `ca8106b09b` / `7b4bbf84dd` | in band |
| Frost Mage (64) | 193,125 | 196,155 | -1.54% | `6cd8b489e1` / `ed6300d16b` | 294,105 | 298,610 | -1.51% | `6cd8b489e1` / `ed6300d16b` | in band |
| Retribution Paladin (70) | 176,355 | 178,061 | -0.96% | `2d63f547eb` / `9f3a7d0716` | 317,620 | 321,347 | -1.16% | `2d63f547eb` / `9f3a7d0716` | in band |
| Arms Warrior (71) | 190,551 | 188,596 | +1.04% | `6d6cff57dd` / `eabc2a7a15` | 339,239 | 328,902 | +3.14% | `6d6cff57dd` / `eabc2a7a15` | in band |
| Fury Warrior (72) | 202,915 | 202,543 | +0.18% | `6d6cff57dd` / `eabc2a7a15` | 266,179 | 272,806 | -2.43% | `6d6cff57dd` / `eabc2a7a15` | in band |
| Feral Druid (103) | 189,718 | 191,667 | -1.02% | `6d6cff57dd` / `eabc2a7a15` | 253,674 | 247,749 | +2.39% | `6d6cff57dd` / `eabc2a7a15` | in band |
| Frost Death Knight (251) | 205,152 | 206,579 | -0.69% | `670c613ad8` / `8a7fe346ac` | 379,687 | 387,833 | -2.10% | `670c613ad8` / `8a7fe346ac` | in band |
| Unholy Death Knight (252) | 193,495 | 198,328 | -2.44% | `670c613ad8` / `8a7fe346ac` | 325,551 | 330,005 | -1.35% | `670c613ad8` / `8a7fe346ac` | in band |
| Beast Mastery Hunter (253) | 193,501 | 202,388 | -4.39% | `76f22e8784` / `859a9c84a5` | 245,734 | 259,613 | -5.35% | `6cd8b489e1` / `ed6300d16b` | **OUT** |
| Marksmanship Hunter (254) | 187,129 | 192,745 | -2.91% | `76f22e8784` / `859a9c84a5` | 279,826 | 280,759 | -0.33% | `6cd8b489e1` / `ed6300d16b` | in band |
| Survival Hunter (255) | 197,750 | 198,514 | -0.38% | `76f22e8784` / `859a9c84a5` | 317,287 | 318,630 | -0.42% | `2d63f547eb` / `9f3a7d0716` | in band |
| Shadow Priest (258) | 183,126 | 183,245 | -0.07% | `6d6cff57dd` / `eabc2a7a15` | 325,492 | 339,984 | -4.26% | `6d6cff57dd` / `eabc2a7a15` | in band |
| Assassination Rogue (259) | 192,846 | 186,209 | +3.56% | `2d63f547eb` / `9f3a7d0716` | 261,472 | 265,841 | -1.64% | `2d63f547eb` / `9f3a7d0716` | in band |
| Outlaw Rogue (260) | 202,901 | 207,454 | -2.19% | `2d63f547eb` / `9f3a7d0716` | 332,866 | 348,127 | -4.38% | `2d63f547eb` / `9f3a7d0716` | in band |
| Subtlety Rogue (261) | 231,858 | 228,930 | +1.28% | `2d63f547eb` / `9f3a7d0716` | 381,842 | 391,502 | -2.47% | `2d63f547eb` / `9f3a7d0716` | in band |
| Elemental Shaman (262) | 205,443 | 207,736 | -1.10% | `6cd8b489e1` / `ed6300d16b` | 327,455 | 319,420 | +2.52% | `6cd8b489e1` / `ed6300d16b` | in band |
| Enhancement Shaman (263) | 189,495 | 193,093 | -1.86% | `2d63f547eb` / `9f3a7d0716` | 392,465 | 400,251 | -1.95% | `2d63f547eb` / `9f3a7d0716` | in band |
| Affliction Warlock (265) | 199,208 | 200,692 | -0.74% | `6cd8b489e1` / `ed6300d16b` | 278,395 | 275,122 | +1.19% | `6cd8b489e1` / `ed6300d16b` | in band |
| Demonology Warlock (266) | 194,209 | 196,790 | -1.31% | `f7fcf2fa6a` / `876c672701` | 408,986 | 420,736 | -2.79% | `f7fcf2fa6a` / `876c672701` | in band |
| Destruction Warlock (267) | 189,823 | 195,142 | -2.73% | `6cd8b489e1` / `ed6300d16b` | 253,427 | 256,778 | -1.31% | `6cd8b489e1` / `ed6300d16b` | in band |
| Windwalker Monk (269) | 203,434 | 206,444 | -1.46% | `66acee2046` / `a0bf50ef53` | 390,525 | 376,560 | +3.71% | `66acee2046` / `a0bf50ef53` | in band |
| Havoc Demon Hunter (577) | 197,900 | 214,737 | -7.84% | `670c613ad8` / `8a7fe346ac` | 311,985 | 334,847 | -6.83% | `670c613ad8` / `8a7fe346ac` | **OUT** |
| Devastation Evoker (1467) | 116,836 | 113,728 | +2.73% | `6d6cff57dd` / `eabc2a7a15` | 184,309 | 182,445 | +1.02% | `6d6cff57dd` / `eabc2a7a15` | in band |
| Devourer Demon Hunter (1480) | 206,035 | 214,110 | -3.77% | `670c613ad8` / `8a7fe346ac` | 329,934 | 341,884 | -3.50% | `670c613ad8` / `8a7fe346ac` | in band |

## Reference and measurement provenance

References are under `game-engine/data/diagnostics/dps-parity-2026-10-21/`: `<spec>.reference.single.normalized.json` when present, otherwise `all-spec-expanded-references/<spec>.reference.single.normalized.json`. For 3T use `<spec>.reference-fixed.three.normalized.json` when present, otherwise `all-spec-expanded-references/<spec>.reference.three.normalized.json`. The corresponding `.proof.json` records the add-event command. Train entry files are `/home/osso/.worktrees/train/results/<entry>.json`; archived reports are `/home/osso/.worktrees/train/logs/<entry>.reports/<spec>.native.<targets>.json`.

**Missing reference and measurement — Balance Druid (102):** This additional DPS spec is outside the 25-profile comparison: `all-spec-current-reference-manifests/102.profile.json` records no shipped current-tier reference and no native harness profile; its MID1 candidate was not selected. It has no valid 1T or 3T comparison.

## Party-buff composition remeasure (2026-11-02)

References changed from `optimal_raid=0` to the composition rule in [DPS parity](dps-parity.md): each actor gains only its own class raid buff, in both SimC and native. Old = branch `party-buffs-parity` without the native cast line, against the pinned references above. New = with the cast line, against `game-engine/data/diagnostics/party-buffs-composition-2026-11-02/dps/`. Both use one `sim`-profile build of the same tree, canonical `world.db`, the latest train inputs per spec (copied to `.../native/{baseline,new}/`) and seeds 70448, 1–9. Mage references are unchanged because the APL already casts Arcane Intellect, and the Priest reference is unchanged because Fortitude adds no damage.

| Spec | T | Old native / SimC | Old gap | New native / SimC | New gap |
|---|---|---:|---:|---:|---:|
| 62 | 1T | 175,182 / 177,681 | -1.41% | 180,788 / 177,681 | +1.75% |
| 62 | 3T | 323,393 / 330,648 | -2.19% | 325,782 / 330,648 | -1.47% |
| 63 | 1T | 208,117 / 197,191 | +5.54% | 203,649 / 197,191 | +3.28% |
| 63 | 3T | 390,279 / 371,059 | +5.18% | 395,159 / 371,059 | +6.49% |
| 64 | 1T | 192,292 / 196,155 | -1.97% | 192,692 / 196,155 | -1.77% |
| 64 | 3T | 294,671 / 298,610 | -1.32% | 293,854 / 298,610 | -1.59% |
| 71 | 1T | 181,627 / 188,596 | -3.70% | 183,230 / 197,315 | -7.14% |
| 71 | 3T | 339,796 / 328,902 | +3.31% | 353,806 / 344,060 | +2.83% |
| 72 | 1T | 202,715 / 202,543 | +0.09% | 207,777 / 211,803 | -1.90% |
| 72 | 3T | 268,274 / 272,806 | -1.66% | 284,430 / 284,888 | -0.16% |
| 103 | 1T | 182,532 / 191,667 | -4.77% | 193,745 / 202,645 | -4.39% |
| 103 | 3T | 228,918 / 247,749 | -7.60% | 244,750 / 261,956 | -6.57% |
| 258 | 1T | 164,620 / 183,245 | -10.16% | 165,262 / 183,245 | -9.81% |
| 258 | 3T | 323,607 / 339,984 | -4.82% | 322,091 / 339,984 | -5.26% |
| 262 | 1T | 231,301 / 207,736 | +11.34% | 234,286 / 211,367 | +10.84% |
| 262 | 3T | 365,319 / 319,420 | +14.37% | 363,890 / 324,957 | +11.98% |
| 263 | 1T | 179,234 / 193,093 | -7.18% | 181,108 / 200,137 | -9.51% |
| 263 | 3T | 363,119 / 400,251 | -9.28% | 383,819 / 414,337 | -7.37% |
| 1467 | 1T | 116,142 / 113,728 | +2.12% | 115,612 / 113,711 | +1.67% |
| 1467 | 3T | 184,204 / 182,445 | +0.96% | 185,810 / 182,175 | +2.00% |

Old gaps for 258, 262 and 263 are already outside ±5% and differ from the table above on the same reference. That table is older train data; this run's inputs or the code since then differ, and the cause is not attributed. 71 1T moves out of band with this change: the reference gains 4.6% with Battle Shout, native gains 0.9%, while 71 3T and 72 gain 2.5–6.0%. It is open for investigation and was not tuned.

## First evidence for out-of-band results

- **Beast Mastery Hunter (253):** 3T 245,734 vs 259,613 (-5.35%; train `ed6300d16b`). In the local 3T sample, Stomp (201754) contributes 27,717 vs 33,208 reference DPS.
- **Havoc Demon Hunter (577):** 1T 197,900 vs 214,737 (-7.84%; train `8a7fe346ac`); 3T 311,985 vs 334,847 (-6.83%; same entry). In the local 1T sample, Burning Blades (453177) contributes 32,795 vs 37,569 reference DPS; in 3T, Abyssal Gaze empowered tick (1287949) contributes 15,437 vs 19,842.

These action comparisons are first evidence for investigation, not confirmed root causes. Different seeds and action attribution can affect the differences.

## Local remeasurements

- Beast Mastery Hunter (253) 1T at `759e5e2abd`, seeds `70448,1,2`: 193,642 vs 202,388 (-4.32%). Report: `/home/osso/.worktrees/game-server-dps-3t/parity-status-inputs/253.native.1.json`.
- Beast Mastery Hunter (253) 3T at `759e5e2abd`, seeds `70448,1,2`: 246,206 vs 259,613 (-5.16%). Report: `/home/osso/.worktrees/game-server-dps-3t/parity-status-inputs/253.native.3.json`.
- Havoc Demon Hunter (577) 1T at `759e5e2abd`, seeds `70448,1,2`: 195,915 vs 214,737 (-8.77%). Report: `/home/osso/.worktrees/game-server-dps-3t/parity-status-inputs/577.native.1.json`.
- Havoc Demon Hunter (577) 3T at `759e5e2abd`, seeds `70448,1,2`: 308,207 vs 334,847 (-7.96%). Report: `/home/osso/.worktrees/game-server-dps-3t/parity-status-inputs/577.native.3.json`.
