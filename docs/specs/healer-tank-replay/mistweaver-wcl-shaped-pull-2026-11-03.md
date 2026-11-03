# Mistweaver WCL-shaped pull (2026-11-03)

The healer replay accepts an interval profile via `HEALER_ENEMY_PROFILE`. The fixture `altar_fangs_wcl_pull1.json` is **INFERRED_WCL** from Altar of Fangs report `pPFDbkfQK7MACtmY`, fight 41, pull 1: each of 19 NPC-instance intervals runs from first to last observed player damage, not from known spawn to death. The observed engaged count matched the inferred profile at every change in all five runs. Targets use training-dummy placement. The incoming damage is synthetic SimC tank and party-pulse pressure, not the WCL encounter's intake.

Five seeds (1–5), 75 seconds each, ran the same binary, with level 90 Mistweaver and the frozen world snapshot used by the earlier recheck. No party member died. The native range is a 95% Student-t interval across seed-level shares, not a WCL population interval. The healer in the profile source pull is a Preservation Evoker; the Mistweaver comparison uses report `zCXNH4Gf1MPwvcZg` fight 10 pull 1 actor 10, a similar-size Altar pull. It has 17 distinct damaged NPC instances versus 19 in the profile, inferred peak 17 versus 15, and at least six enemies for 80.1% versus 59.3% of the sampled time. This is not a matched timeline. WCL healing shares are effective healing plus absorbed amounts; cast shares include cast-success events, including movement and utility, but exclude begin-cast events. Native cast shares use the replay's healer cast-success counts. Different pulls, duration, intake, actor, positioning, and overheal limit comparability.

| Measure | Native mean (95% CI) | WCL actor |
|---|---:|---:|
| Spinning Crane Kick 101546, cast share | 55.66% (50.43–60.89%) | 23.17% (19 / 82 casts) |
| Rising Sun Kick 107428, cast share | 10.63% (9.97–11.28%) | 20.73% (17 / 82 casts) |
| Way of the Crane 389325, effective healing share | 35.18% (29.70–40.66%) | 18.33% |
| Way of the Crane 389328, effective healing share | 8.85% (6.56–11.15%) | 11.20% |

These are observations, not a reason to tune action thresholds: one synthetic WCL-shaped pull does not match full-dungeon conditions. WCL pull event extracts and native reports, input and binary hashes: `/home/osso/.worktrees/healer-multi-enemy-evidence-v2/`. Binary code was built at `a5fb64b5633952c68c863e9ee95f06d7b541df42`; the subsequent change only adds this report.
