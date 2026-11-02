# Seven-healer party-pulse recheck (2026-11-02)

The hostile-hit party-pulse fixture was run for 120 seconds per seed, seeds 1–10, with level-90 healers and a Protection Paladin tank. All 70 runs used the first-combat boundary, the same executable and the same frozen world snapshot. Healer talents are the +17 WCL population-mode builds; healer gear is each spec's tracked top-character profile, while other party members use Journal gear. The one-enemy synthetic trace is not the WCL dungeon intake.

Measurement source was master `7cd700375a9f1733f341dbe4719f8019bbd5b5ba` when the binary was built. Binary SHA-256: `35df1f28379112de15a39d363a99b35d2f21d30d56b14b47ab67504929b88aac`. Frozen snapshot v3 SHA-256: `8cead1741348f4cfc39409e49cf3f79dbbe9efcfccb37344234c23ed05449703`. Tank trace SHA-256: `d6ddd24ceb2690194d70efc78829b140c64b2346c59b8e63ba6ef1975b9d7a79`; party trace SHA-256: `6d95d3bc47067ef413cf91d546eba9f50687301eb04096c2b750b8054f588ef9`. Per-seed reports and metadata: `/home/osso/.worktrees/healer-table-recheck-evidence/runs/`; aggregate: `/home/osso/.worktrees/healer-table-recheck-evidence/summary.json`.

Master advanced to `f992bbad53fb59d90a3f6cb5bae5314c9b89422a` while the batch ran; the table measures the earlier binary, not a rebuilt `f992bbad53` binary. Intervening changes include shared attack-stop timer handling, so equivalence on that later binary is not established.

Each native value is the mean of effective HPS plus consumed absorbs per second. Native ratio divides by the same-binary Holy Priest mean. WCL ratio is the mean of the +17 Altar of Fangs and Den of Nalorakk spec-median/Holy-Priest-median ratios, from `game-engine/data/diagnostics/follower-class-rebaseline-2026-10-31/healer-ratio-vs-wcl-6e4e8b69.json`; its ranking populations may be capped. Difference is `(native ratio / WCL ratio - 1) × 100`. No run had a party death.

| Healer | Spec | Native HPS + absorbs/s | Native / Holy | WCL / Holy | Difference | Within 10% |
|---|---:|---:|---:|---:|---:|---|
| Holy Priest | 257 | 34,130.66 | 1.0000 | 1.0000 | 0.00% | denominator |
| Discipline Priest | 256 | 32,005.10 | 0.9377 | 0.9145 | +2.54% | yes |
| Restoration Shaman | 264 | 31,047.17 | 0.9097 | 0.8949 | +1.65% | yes |
| Holy Paladin | 65 | 32,630.43 | 0.9560 | 0.9359 | +2.15% | yes |
| Preservation Evoker | 1468 | 31,510.41 | 0.9232 | 0.8662 | +6.58% | yes |
| Mistweaver Monk | 270 | 34,114.57 | 0.9995 | 0.8872 | +12.66% | no |
| Restoration Druid | 105 | 33,709.91 | 0.9877 | 0.9083 | +8.74% | yes |

## Mistweaver spell shares

Shares are the percentage of summed effective healing plus consumed absorbs, aggregated over all ten native seeds. The WCL comparison uses actor-attributed heal and absorbed amounts for `zCXNH4Gf1MPwvcZg` fight 10 actor 10, Altar of Fangs +17, not the population-median ratio above. Zero means no recorded amount; values are rounded to two decimals. The complete union of recorded spell IDs is shown.

| Spell ID | Native % | WCL actor % |
|---:|---:|---:|
| 115175 | 41.48 | 1.56 |
| 388024 | 19.12 | 18.80 |
| 389325 | 0.00 | 15.00 |
| 191894 | 7.63 | 12.49 |
| 406220 | 8.90 | 3.41 |
| 389328 | 0.00 | 8.20 |
| 388025 | 2.75 | 7.47 |
| 443039 | 6.47 | 2.55 |
| 343819 | 5.13 | 4.16 |
| 1260617 | 0.24 | 4.93 |
| 399491 | 3.32 | 3.38 |
| 1260681 | 0.54 | 3.17 |
| 1263727 | 0.00 | 3.14 |
| 443106 | 2.69 | 1.12 |
| 116849 | 0.00 | 1.90 |
| 124682 | 1.00 | 1.73 |
| 119611 | 0.74 | 1.37 |
| 414143 | 0.00 | 1.19 |
| 115310 | 0.00 | 1.14 |
| 443113 | 0.00 | 1.09 |
| 428439 | 0.00 | 0.85 |
| 1287665 | 0.00 | 0.32 |
| 1277446 | 0.00 | 0.25 |
| 1263002 | 0.00 | 0.23 |
| 132463 | 0.00 | 0.20 |
| 443028 | 0.00 | 0.13 |
| 1287955 | 0.00 | 0.11 |
| 443614 | 0.00 | 0.08 |
| 274062 | 0.00 | 0.01 |

Soothing Mist 115175 accounts for the largest discrepancy: native casts total 61 across ten 120-second runs, versus 52 casts in this 1,704-second WCL dungeon. The current native policy channels it when its target is below 90% and higher-priority actions are not selected. The WCL actor also uses Way of the Crane 389325/389328, which the one-enemy native policy does not trigger because it reserves Spinning Crane Kick for at least two enemies. Different target counts, incoming damage and overheal affect these shares. These observations do not establish an incorrect coefficient or a retail channel-interruption rule, so no mechanic was changed to force the ratio into band. The Soothing Mist policy/channel behavior and its impact under matched intake remain unqualified; see [Mistweaver notes](270.md).
