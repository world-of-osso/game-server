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

## First evidence for out-of-band results

- **Beast Mastery Hunter (253):** 3T 245,734 vs 259,613 (-5.35%; train `ed6300d16b`). In the local 3T sample, Stomp (201754) contributes 27,717 vs 33,208 reference DPS.
- **Havoc Demon Hunter (577):** 1T 197,900 vs 214,737 (-7.84%; train `8a7fe346ac`); 3T 311,985 vs 334,847 (-6.83%; same entry). In the local 1T sample, Burning Blades (453177) contributes 32,795 vs 37,569 reference DPS; in 3T, Abyssal Gaze empowered tick (1287949) contributes 15,437 vs 19,842.

These action comparisons are first evidence for investigation, not confirmed root causes. Different seeds and action attribution can affect the differences.

## Local remeasurements

- Beast Mastery Hunter (253) 1T at `759e5e2abd`, seeds `70448,1,2`: 193,642 vs 202,388 (-4.32%). Report: `/home/osso/.worktrees/game-server-dps-3t/parity-status-inputs/253.native.1.json`.
- Beast Mastery Hunter (253) 3T at `759e5e2abd`, seeds `70448,1,2`: 246,206 vs 259,613 (-5.16%). Report: `/home/osso/.worktrees/game-server-dps-3t/parity-status-inputs/253.native.3.json`.
- Havoc Demon Hunter (577) 1T at `759e5e2abd`, seeds `70448,1,2`: 195,915 vs 214,737 (-8.77%). Report: `/home/osso/.worktrees/game-server-dps-3t/parity-status-inputs/577.native.1.json`.
- Havoc Demon Hunter (577) 3T at `759e5e2abd`, seeds `70448,1,2`: 308,207 vs 334,847 (-7.96%). Report: `/home/osso/.worktrees/game-server-dps-3t/parity-status-inputs/577.native.3.json`.
