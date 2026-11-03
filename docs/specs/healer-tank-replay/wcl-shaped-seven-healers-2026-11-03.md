# Seven healers: WCL-shaped pull with synthetic intake

Historical methodology only: the 65, 105, 270 and 1468 source reports predate the 2026-10-16 build-70448 cutoff. Do not use their spell shares or rotations as current retail evidence.

Five seeds per spec replay the same 75-second interval profile and binary, with the synthetic SimC tank and party-pulse intake. The profile is INFERRED_WCL from `pPFDbkfQK7MACtmY` fight 41 pull 1, using first-to-last observed player damage to each NPC instance, not confirmed spawn/death times. These results are diagnostic only: incoming damage, target placement, healing targets, talents, gear and enemy timelines do not match the named WCL actors. **Do not classify class defects from these shares alone.**

WCL references below use the first 75 seconds of one similar-size Altar of Fangs pull, except 1468 uses the profile's source pull and its actual healer. Counts are distinct NPC instances hit during the complete pull. The peak and time with at least six enemies are inferred from damage windows; percentages are sampled over the first 75 seconds. The native profile peaks at 15 and has at least six for 59.3%. Different occupancy makes even these similar pulls imperfect comparisons.

| Spec | WCL report / fight / pull / actor | Target instances | Peak | ≥6 time |
|---:|---|---:|---:|---:|
| 65 | `pWzdNbRhtKyfmJQB` / 5 / 1 / 3 | 17 | 17 | 98.0% |
| 105 | `kD8MqXLV6CFYHN2p` / 3 / 1 / 108 | 20 | 20 | 95.9% |
| 256 | `R19Zb4BqjxKvgzGm` / 13 / 1 / 1163 | 22 | 22 | 76.9% |
| 257 | `BCX8v3VPd6pnyzM4` / 1 / 1 / 4 | 19 | 19 | 89.2% |
| 264 | `mHcDYRVyprndwjFC` / 2 / 1 / 46 | 17 | 17 | 74.2% |
| 270 | `zCXNH4Gf1MPwvcZg` / 10 / 1 / 10 | 17 | 17 | 92.1% |
| 1468 | `pPFDbkfQK7MACtmY` / 41 / 1 / 221 | 19 | 15 | 59.3% |

Shares use effective healing plus consumed absorbs. Native numbers are seed means with 95% Student-t intervals; WCL is one actor, not a population interval. No native party member died. The complete per-spell union and confidence intervals are in `/home/osso/.worktrees/healer-multi-enemy-evidence-v2/matched-pull-spell-shares.json`; the named event extracts and binary/input hashes are in the same directory.

| Spec | Spell | Native mean (95% CI) | WCL |
|---:|---:|---:|---:|
| 65 | 85222 | 26.53% (19.70–33.36%) | 1.38% |
| 105 | 774 | 18.87% (15.79–21.94%) | 8.15% |
| 256 | Atonement 81751 + 94472 | 17.18% (12.79–21.58%) | 63.09% |
| 257 | 33110 | 12.57% (11.11–14.04%) | 9.39% |
| 264 | 73921 | 33.48% (30.25–36.72%) | 14.56% |
| 270 | 389325 | 35.18% (29.70–40.66%) | 19.73% |
| 1468 | 355941 | 30.50% (23.29–37.71%) | 4.75% |

Native reports all Atonement as 94472. Snapshot DB2 gives 81751 the `SPELL_ATTR2_CANT_CRIT` flag and 94472 no such flag; in the WCL Discipline reference, all 2,469 81751 heals are noncritical and all 554 94472 heals are critical. The comparison sums both IDs. Native Atonement was 97.41% overheal across the five seeds, so effective shares strongly depend on the unmatched intake. The binary was built at `a5fb64b5633952c68c863e9ee95f06d7b541df42`; subsequent commits only changed documentation.
