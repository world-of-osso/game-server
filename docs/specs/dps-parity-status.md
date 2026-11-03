# DPS parity status

## Arms Warrior remeasure — 2026-11-03

The native follower used the frozen campaign database and ten seeds `70448,1–9` for 300 seconds on candidate `7131ba15ea`. SimC `76f8118` references match the landed-master table below.

| Targets | Native DPS | SimC DPS | Ratio | Flag |
|---|---:|---:|---:|---|
| 1T | 194,113 | 197,315 | 0.9838 | in |
| 3T | 349,783 | 344,060 | 1.0166 | in |

Both point estimates are within the ±5% band. The earlier geometry candidate `4530502682` measured 194,113 and 351,396 DPS on the same seeds; the revised results are close but not identical. The revisions also have different master bases, so this comparison does not isolate the geometry change. All twenty per-seed fact-error ledgers are empty. Evidence: `game-engine/data/diagnostics/parity-71-2026-11-03b/fix-7131ba15/`.

## Beast Mastery remeasure — 2026-11-03

The native follower used the frozen campaign database and ten seeds `70448,1–9` for 300 seconds. The SimC `76f8118` references and confidence-interval method match the landed-master table below.

| Targets | Native revision | Native DPS | SimC DPS | Ratio ± 95% CI | Flag |
|---|---|---:|---:|---:|---|
| 1T | `40770835c7` | 195,531 | 202,388 | 0.9661 ± 0.0119 | in |
| 3T | `3cab680a64` | 256,422 | 259,613 | 0.9877 ± 0.0146 | in |

Both point estimates are in the ±5% band. The 1T interval crosses its lower boundary. The 3T measurement predates the Feral-only rebase in `40770835c7`; it was not repeated on that revision. The earlier 3T result of 0.8322 used an older revision, so its improvement cannot be attributed to one fix. The reported fact-error ledgers are empty; potion inventory and external Power Infusion remain unavailable. Evidence: `game-engine/data/diagnostics/parity-253-2026-11-03b/407708-1t/` and `tip-fixed/`.

## Assassination death-refund gap — 2026-11-03

Venomous Wounds (79134) does not yet refund energy when a Rupture target dies. SimC reports 79.8 energy per 300-second 3T fight. Spending that energy on Fan of Knives at its SimC average damage estimates 369 direct DPS (0.14% of reference DPS), excluding indirect effects. The retail refund formula remains unverified; the SimC implementation marks it uncertain. Evidence: `game-engine/data/diagnostics/parity-259-2026-11-03b/refund-evidence.md`.

## Elemental and Fire remeasure — 2026-11-03

On candidate `1cea56b867` (base `55bd63da49`), the fixture-engagement, Elemental MotE/Fusion and Vile Vial fixes were measured with the frozen campaign database and native follower seeds `70448,1–9`. The SimC `76f8118` references are unchanged. These results include other changes since `389ca621d0`; the difference from the earlier campaign cannot be attributed to one fix.

| Spec | Targets | Native DPS | SimC DPS | Ratio ± 95% CI | Flag |
|---|---:|---:|---:|---:|---|
| Elemental Shaman (262) | 1T | 209,528 | 211,367 | 0.9913 ± 0.0288 | in |
| Elemental Shaman (262) | 3T | 320,193 | 324,957 | 0.9853 ± 0.0154 | in |
| Fire Mage (63) | 1T | 191,988 | 197,191 | 0.9736 ± 0.0345 | in |
| Fire Mage (63) | 3T | 354,685 | 371,059 | 0.9559 ± 0.0251 | in |

All four point estimates are within [0.95, 1.05]; the Fire 3T interval crosses the lower boundary. Each run completed ten seeds with an empty fact-error ledger. The CI method is the same as the landed-master table below. Logs and per-seed reports: `game-engine/data/diagnostics/parity-262-new-2026-11-03/vial-fix/`.

## Marksmanship diagnostic — 2026-11-03

The fixture-engagement and Radiant Edge fixes were measured locally with the frozen campaign database and seeds `70448,1–9`. On `561f69a99c`, native 1T was 177,957 versus SimC 192,745 (0.9233 ± 0.0149); native 3T was 263,329 versus 280,759 (0.9379 ± 0.0203). Both point estimates remain outside the ±5% band. The earlier campaign at `389ca621d0` measured 175,234 and 218,752 respectively; the different master revisions prevent attribution of the full change to one fix. Applying `edb3688d1d` locally did not change any seed: this profile uses fixed-stat neck bonus 13708, not that commit's random-stat bonus. Evidence: `game-engine/data/diagnostics/parity-254-2026-11-03/`.

The reference profile has bonus 13708 on its neck, chest and weapon. Native models three independent 2 RPPM drivers refreshing one single-strength `1307910` aura. SimC `76f8118` instead sums all three item amounts into that buff while its reported effective proc rate matches one driver: the debug trace shows +1,305 crit rating and −217 to each other rating versus a native captured proc of about +393/−63. Through Penetrating Shots, the extra crit rating also raises SimC's critical-damage multiplier. This is a **reference-model difference**, not a measured correction for the remaining DPS gap; it also affects other profiles with multiple bonus-13708 items.

DB2 has no `SpellAuraOptions` row for `1307910`; its recorded attributes do not establish an item-source stacking exception. TC `SpellAuras.cpp:1756–1769` rejects the same spell and caster stacking except for a custom enchant-proc case. Retail stacking and amounts remain **unknown**: 4,000 inspected current-build ranking rows yielded no player with two or more bonus-13708 items. Native remains unchanged.

DB2 gives each driver 2 RPPM, no haste-modifier rows and a 12-second buff; the SimC debug trace also reports no haste scaling. With independent drivers, bad-luck protection and 30% refresh carryover, a 1,500-run illustrative simulation predicted 79.45% uptime for three drivers and 38.22% for one. Native's ten-seed mean was 78.55%; SimC's 1,000-iteration report was 38.96%. The first native seed alone was 81.37%; the aggregate report retained that seed's uptime rather than averaging it. The reference and native rates therefore represent different models, not evidence that native's three-driver rate is wrong.

## Demon Hunter reach-4 candidate remeasure (2026-11-03)

Candidate `1f6ed648ec` uses the SimC-sourced 4.0 target combat reach and includes the ballistic landing and single-writer movement changes. These are local candidate measurements, not a landed-master result. Each row uses the native production literal follower, ten seeds `70448,1–9`, 300 seconds and the frozen world database. SimC references use 1,000 iterations. Per-seed reports and logs are in `game-engine/data/diagnostics/parity-577-2026-11-03/simc-reach4/`. The 577 1T aggregate JSON was overwritten by a one-seed probe; its ten seed files and ten-seed log remain intact. All 40 seed reports have empty fact-error ledgers.

| Spec | Targets | Native DPS | Raw SimC DPS | Raw ratio | Raw status |
|---|---:|---:|---:|---:|---|
| Havoc (577) | 1T | 199,917 | 214,737 | 0.9310 | **OUT** |
| Havoc (577) | 3T | 313,027 | 334,847 | 0.9348 | **OUT** |
| Devourer (1480) | 1T | 214,424 | 214,110 | 1.0015 | in |
| Devourer (1480) | 3T | 325,406 | 341,884 | 0.9518 | in |

Against the diagnostic SimC control described in [Havoc DPS parity](577.md#simc-without-double-applications), Havoc is **in band**: 199,917 / 207,944 = 0.9614 for 1T and 313,027 / 327,183 = 0.9567 for 3T. The control removes the documented extra Exergy and Eternal Hunt applications and the bugs-mode Soulscar factor; it is not a Retail measurement. The raw SimC comparison remains out of band. Neither candidate 3T measurement includes the separately owned fixture-engagement fix; the candidate also excludes the separately owned Aqirbane fix. These results do not establish post-merge parity.

## L90 landed-master remeasure (2026-11-03)

Each native row uses a follower actor, ten seeds `70448,1–9`, 300 seconds and the frozen world database. SimC `76f8118` references use 1,000 iterations, the same fixed duration, seed 70448 and the actor-only raid-buff composition. The 3T reference uses timed add events. Devastation uses its approved MID1 profile. The exact commands, hashes, per-seed reports, logs and world snapshot are in `game-engine/data/diagnostics/dps-parity-landed-master-2026-11-03/` and the references in `game-engine/data/diagnostics/party-buffs-composition-2026-11-02/dps/`.

Ratio is native mean / SimC mean. The approximate 95% CI uses the delta method for independent means, native sample standard deviation, SimC reported standard error and the conservative t critical value for nine degrees of freedom. It is not a paired-seed CI. **OUT** means the point estimate is outside [0.95, 1.05], regardless of whether its CI crosses the boundary.

| Spec | Targets | Native DPS | SimC DPS | Ratio ± 95% CI | Flag | Driver at measurement | Master |
|---|---:|---:|---:|---:|---|---|---|
| Arcane Mage (62) | 1T | 175,731 | 177,681 | 0.9890 ± 0.0309 | in | old wrapper | `389ca621d0` |
| Arcane Mage (62) | 3T | 332,276 | 330,648 | 1.0049 ± 0.0349 | in | old wrapper | `389ca621d0` |
| Fire Mage (63) | 1T | 206,602 | 197,191 | 1.0477 ± 0.0271 | in | old wrapper | `389ca621d0` |
| Fire Mage (63) | 3T | 387,793 | 371,059 | 1.0451 ± 0.0274 | in | old wrapper | `389ca621d0` |
| Frost Mage (64) | 1T | 192,575 | 196,155 | 0.9817 ± 0.0181 | in | production | `288a949232` |
| Frost Mage (64) | 3T | 295,078 | 298,610 | 0.9882 ± 0.0127 | in | production | `288a949232` |
| Retribution Paladin (70) | 1T | 178,637 | 178,061 | 1.0032 ± 0.0252 | in | production | `389ca621d0` |
| Retribution Paladin (70) | 3T | 317,175 | 321,347 | 0.9870 ± 0.0136 | in | production | `389ca621d0` |
| Arms Warrior (71) | 1T | 185,933 | 197,315 | 0.9423 ± 0.0272 | **OUT** | old wrapper | `389ca621d0` |
| Arms Warrior (71) | 3T | 348,989 | 344,060 | 1.0143 ± 0.0211 | in | old wrapper | `389ca621d0` |
| Fury Warrior (72) | 1T | 211,831 | 211,803 | 1.0001 ± 0.0134 | in | production | `288a949232` |
| Fury Warrior (72) | 3T | 275,990 | 284,888 | 0.9688 ± 0.0244 | in | production | `288a949232` |
| Feral Druid (103) | 1T | 190,439 | 202,645 | 0.9398 ± 0.0120 | **OUT** | old wrapper | `389ca621d0` |
| Feral Druid (103) | 3T | 242,130 | 261,956 | 0.9243 ± 0.0144 | **OUT** | old wrapper | `389ca621d0` |
| Frost Death Knight (251) | 1T | 199,242 | 206,579 | 0.9645 ± 0.0165 | in | old wrapper | `389ca621d0` |
| Frost Death Knight (251) | 3T | 372,121 | 387,833 | 0.9595 ± 0.0190 | in | old wrapper | `389ca621d0` |
| Unholy Death Knight (252) | 1T | 189,078 | 198,328 | 0.9534 ± 0.0151 | in | old wrapper | `389ca621d0` |
| Unholy Death Knight (252) | 3T | 322,187 | 330,005 | 0.9763 ± 0.0255 | in | old wrapper | `389ca621d0` |
| Beast Mastery Hunter (253) | 1T | 168,299 | 202,388 | 0.8316 ± 0.0128 | **OUT** | production | `ce51d76614` |
| Beast Mastery Hunter (253) | 3T | 216,044 | 259,613 | 0.8322 ± 0.0108 | **OUT** | production | `ce51d76614` |
| Marksmanship Hunter (254) | 1T | 175,234 | 192,745 | 0.9091 ± 0.0146 | **OUT** | old wrapper | `389ca621d0` |
| Marksmanship Hunter (254) | 3T | 218,752 | 280,759 | 0.7791 ± 0.0513 | **OUT** | old wrapper | `389ca621d0` |
| Survival Hunter (255) | 1T | 190,161 | 198,514 | 0.9579 ± 0.0150 | in | old wrapper | `389ca621d0` |
| Survival Hunter (255) | 3T | 306,827 | 318,630 | 0.9630 ± 0.0132 | in | old wrapper | `389ca621d0` |
| Shadow Priest (258) | 1T | 162,835 | 183,245 | 0.8886 ± 0.0100 | **OUT** | production | `288a949232` |
| Shadow Priest (258) | 3T | 322,370 | 339,984 | 0.9482 ± 0.0154 | **OUT** | production | `288a949232` |
| Assassination Rogue (259) | 1T | 177,749 | 186,209 | 0.9546 ± 0.0129 | in | old wrapper | `389ca621d0` |
| Assassination Rogue (259) | 3T | 246,311 | 265,841 | 0.9265 ± 0.0241 | **OUT** | old wrapper | `389ca621d0` |
| Outlaw Rogue (260) | 1T | 189,588 | 207,454 | 0.9139 ± 0.0305 | **OUT** | old wrapper | `389ca621d0` |
| Outlaw Rogue (260) | 3T | 336,375 | 348,127 | 0.9662 ± 0.0328 | in | old wrapper | `389ca621d0` |
| Subtlety Rogue (261) | 1T | 231,850 | 228,930 | 1.0128 ± 0.0129 | in | old wrapper | `389ca621d0` |
| Subtlety Rogue (261) | 3T | 383,049 | 391,502 | 0.9784 ± 0.0134 | in | old wrapper | `389ca621d0` |
| Elemental Shaman (262) | 1T | 233,900 | 211,367 | 1.1066 ± 0.0275 | **OUT** | old wrapper | `389ca621d0` |
| Elemental Shaman (262) | 3T | 364,612 | 324,957 | 1.1220 ± 0.0212 | **OUT** | old wrapper | `389ca621d0` |
| Enhancement Shaman (263) | 1T | 180,502 | 200,137 | 0.9019 ± 0.0169 | **OUT** | old wrapper | `389ca621d0` |
| Enhancement Shaman (263) | 3T | 374,935 | 414,337 | 0.9049 ± 0.0199 | **OUT** | old wrapper | `389ca621d0` |
| Affliction Warlock (265) | 1T | 189,587 | 200,692 | 0.9447 ± 0.0183 | **OUT** | production | `288a949232` |
| Affliction Warlock (265) | 3T | 270,472 | 275,122 | 0.9831 ± 0.0262 | in | production | `288a949232` |
| Demonology Warlock (266) | 1T | 170,854 | 196,790 | 0.8682 ± 0.0172 | **OUT** | production | `ce51d76614` |
| Demonology Warlock (266) | 3T | 392,647 | 420,736 | 0.9332 ± 0.0167 | **OUT** | production | `ce51d76614` |
| Destruction Warlock (267) | 1T | 183,949 | 195,142 | 0.9426 ± 0.0155 | **OUT** | production | `288a949232` |
| Destruction Warlock (267) | 3T | 249,623 | 256,778 | 0.9721 ± 0.0147 | in | production | `288a949232` |
| Windwalker Monk (269) | 1T | 197,011 | 206,444 | 0.9543 ± 0.0126 | in | old wrapper | `389ca621d0` |
| Windwalker Monk (269) | 3T | 372,079 | 376,560 | 0.9881 ± 0.0297 | in | old wrapper | `389ca621d0` |
| Havoc Demon Hunter (577) | 1T | 188,598 | 214,737 | 0.8783 ± 0.0135 | **OUT** | production | `288a949232` |
| Havoc Demon Hunter (577) | 3T | 304,141 | 334,847 | 0.9083 ± 0.0116 | **OUT** | production | `288a949232` |
| Devastation Evoker (1467) | 1T | 114,680 | 113,711 | 1.0085 ± 0.0177 | in | old wrapper | `389ca621d0` |
| Devastation Evoker (1467) | 3T | 183,445 | 182,175 | 1.0070 ± 0.0166 | in | old wrapper | `389ca621d0` |
| Devourer Demon Hunter (1480) | 1T | 220,555 | 214,110 | 1.0301 ± 0.0277 | in | old wrapper | `389ca621d0` |
| Devourer Demon Hunter (1480) | 3T | 356,053 | 341,884 | 1.0414 ± 0.0268 | in | old wrapper | `389ca621d0` |

**3T scope:** Every 3T row is labeled with the measured master. These results predate the separate fixture-engagement fix reported by the team; they do not establish parity after that fix.

**Ledger:** Each valid row reports an empty fact-error ledger for all seeds. An invalid row is not a passing measurement. The base and port campaign logs include setup failures and the pet-admission assertion failures; see the evidence directory for details.


## Earlier train snapshot — 2026-10-29

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

## Protection Warrior tank intake — Wrecked reference gap, 2026-11-03

The 300-second player-actor tank-dummy replay with external healing and seeds 70448, 1–9 measured 9,352 ± 286 DTPS native (`d15bf3a59f`) versus 10,622 SimC (`76f8118`, ten measured iterations): native is 11.96% lower. The runs use the frozen campaign world DB `99887e4dfb392b7f2dd93812e29fa4198c9a24ffaa8233445968781bf0c6d9e6`; ± is a 95% Student-t interval. Receipts are in `game-engine/data/diagnostics/tank-ranking-2026-11-03/`.

In seed 70448, both sides receive 145 bleed ticks with a 25,000 base each. Baseline native pre-absorb bleed was 2,398,939; the SimC trace totals 2,708,448. Native's Wrecked 447513 applies −2% damage per stack to its caster, reaching ten stacks for 51 ticks. Reconstructing the captured native factors with only this term removed adds 161,034 pre-absorb bleed damage; this is not an after-absorb DTPS result. The DB2 70448 Spell.csv tooltip explicitly describes affected enemies dealing less damage to the caster; SimC `sc_warrior.cpp:1193–1197` parses only Wrecked's outgoing damage effect and does not implement its incoming damage reduction. Do not remove native Wrecked DR to match that reference.

A separate diagnostic replay zeroed only the Wrecked incoming aura effect; ten native seeds produced 9,983 ± 310 DTPS, 6.01% below the same SimC reference. The mean paired native difference was +632 ± 38 DTPS, explaining about half of the original 1,270-DTPS gap; the remaining difference is unresolved. This counterfactual was built on `ddc53717b6`, which advanced beyond the baseline; the intervening changes were not isolated in this experiment. It had zero deaths and fact errors. Diagnostic change and per-seed receipts are `73.counterfactual.txt` and `73.no-wrecked.<seed>.json`; no diagnostic code was retained.

## Tank outgoing DPS — composition replay, 2026-11-03

These are the latest available measurements, not one common build. Each native result uses 300-second replays with seeds 70448 and 1–9; ± is a 95% Student's t interval for the native mean. References use the pinned SimC `76f8118` composition inputs. The status describes the mean relative to ±5%, not proof that the whole interval is in band.

| Spec | Native DPS ± 95% CI | SimC DPS | Gap | Measured tree | Status |
|---|---:|---:|---:|---|---|
| Protection Paladin (66) | 138,609 ± 6,149 | 142,925 | −3.02% | `18bfe3b4c0` | Mean in band; interval crosses boundary |
| Protection Warrior (73) | 159,976 ± 3,292 | 157,626 | +1.49% | `c692f9a706` | Mean in band; Wrecked differs from SimC as below |
| Guardian Druid (104) | 101,070 ± 2,258 | 103,423 | −2.27% | `389ca621d0` | Mean in band |
| Blood Death Knight (250) | 137,000 ± 3,385 | 127,534 | +7.42% | `18bfe3b4c0` | In progress; fixes need remeasurement |
| Brewmaster Monk (268) | 134,189 ± 2,154 | 140,478 | −4.48% | `ef0cdd5bb4` | Mean in band; interval crosses boundary; Aflame missing |
| Vengeance Demon Hunter (581) | 126,575 ± 2,974 | 130,306 | −2.86% | `389ca621d0` | Mean in band; interval crosses boundary |

Receipts are under `game-engine/data/diagnostics/`: `tank-remeasure-after-18bfe3b4c0/{66,250}/`, `tank-wrecked-retail-fix-2026-11-03/`, `tank-remeasure-2026-11-03/` (104 and 581), and `tank-brewmaster-harmony-attr-2026-11-03/`. Blood's Bone Shield fix `6dba10334f` and Gift fix `456632351b` are not included in its measurement; the latter is unverified. Brewmaster's unimplemented Aflame field contributes 467 DPS (0.33%) in the reference; see [Brewmaster tank replay](healer-tank-replay/268.md).

## Protection Warrior Wrecked discrepancy

**VERIFIED_DB2 / INFERRED_WCL:** Wrecked's damage-taken effect has 1% base per stack after the tooltip's 0.1 divisor. Protection Warrior 1258163 effect 16 doubles that value through label 4007, giving 20% at 10 stacks. In WCL reports `WVLfZxkmRFcMjnv9/35`, `RWb3vtjQJnmC9rfG/8`, `ya4H3X78fx19gpJF/1`, and `jxLNVC2MRw8vtJ74/8`, nearby same-target noncritical Devastator hits at inferred 10 stacks had median ratios 1.211, 1.204, 1.218, and 1.216 relative to zero stacks. The observations are not independent and player buffs can differ. SimC `76f8118` `sc_warrior.cpp:1193–1197` instead uses unmodified base points, or 10% at 10 stacks. A resulting native/SimC gap is not grounds to remove the authored modifier.

The same four reports do not show an increased Devastator critical/noncritical ratio with Wrecked. Native excludes its critical-damage effect pending retail evidence that the critical ratio rises with Wrecked stacks; SimC also omits that effect.

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
