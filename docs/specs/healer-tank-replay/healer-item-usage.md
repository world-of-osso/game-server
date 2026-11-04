# Healer on-use item qualification

## Rule and scope

**INFERRED_WCL: kept for use rate; timing unsupported.** Healer `use_items` considers only items whose measured combat-ready share is at most about 30%. This is a per-item qualification, not a per-class whitelist or a claim that retail casts immediately when ready. Existing urgent heals precede item use. Do not add a health, GCD or fixed-delay condition from this measurement.

Native rule: `use_items,slots=trinket1:trinket2,if=in_combat&this_trinket.healer_rate_qualified`. `NativeFacts.in_combat` reads the actor's actual `CombatStatus`; missing status means false. Per-candidate item context resolves `this_trinket`. A single qualification slice in `literal_rotation/equipment.rs` supplies the item fact. Unknown items are not qualified. DPS/tank sourced item actions do not use this restriction.

| Equipped item | On-use spell | DB2 cooldown | Healer qualification | Reason |
|---|---:|---:|---|---|
| Soulcoiler Ritual Vessel 270162 | 1291894 | 120 s | Yes | 22.2% combat-ready time; omission loses all uses |
| Hex Lord's Dooming Idol 270169 | 1295885 | 30 s | No | 67.6% combat-ready time; on-CD use would greatly exceed retail |

Other trinkets in the seven matched builds (250214, 270164, 270167, 270171, 281215) have passive item effects, not on-use effects. The Idol remains unused until a sourced use condition exists. Retire or revise each qualification when source-defined timing or a better matched retail measurement replaces it.

## Measurement

Whole current-build keys, after the first observed healer-source cast. Opening cooldown state is unknown and excluded. Soulcoiler allied-player deaths reduce a running cooldown by 30 seconds, per DB2 1291894; the calculation accounts for those events. The Idol has no such reduction. No reconstructed use precedes readiness. Combat time is the union of WCL dungeonPull intervals; wall time includes travel. Death-to-resurrection downtime is not separately excluded, so ready fractions are upper bounds for living usability. Names identify distinct players; zero-use reports do not prove an item was equipped.

| Item | Uses / distinct players | Subsequent intervals | Combat delay p10 / median / p90 | Wall delay p10 / median / p90 | Combat / wall ready share |
|---|---|---:|---|---|---|
| Soulcoiler | 110 / 9 | 101 | 0.79 / 21.44 / 83.20 s | 1.32 / 21.60 / 95.16 s | 22.2% / 23.3% |
| Idol | 70 / 4 | 66 | 5.55 / 33.26 / 170.45 s | 5.55 / 38.66 / 197.83 s | 67.6% / 70.0% |

Only 21.8% of subsequent Soulcoiler uses and 9.1% of Idol uses occur within five seconds of combat-time readiness. Soulcoiler's observed rate is 0.410 uses per wall minute, 0.428 per combat minute across the nine users' whole keys; Idol's is 0.585 / 0.651. These are aggregate rates, not the reciprocal of a median interval. First-use timing in a 75-second replay is not a steady-state cooldown rate.

Soulcoiler report/fight IDs: `qLJr1K6gD3xWNfcw/1`, `whDVj67G9gxC3WbF/1`, `6ctdTRZJxGF34Kk7/24`, `X8VcqT9g1ZjMvJy2/13`, `pYrTNkVFBGH7n8mK/6`, `Fk4Aph7aKHByqPC6/18`, `dA72xfKtyPmTJVLh/12`, `MFjm9rV3Dz2kvtX6/1`, `7LGMHTcz38DYwXhj/50`. A tenth sampled player (`LFvC3kbYqfcZJ7tR/18`) has no observed Soulcoiler use and is excluded, not counted as an equipped non-user.

Idol report/fight IDs: `tDxrFAQVRyJPngc1/1`, `2nrLfRQ18aVcZMDN/32`, `fN1hwGzr9ZgcQWAL/5`, `k7Lhr3DadQxtGjPq/8`. Reports are dated October 16, 2026 or later. Raw casts, deaths, metadata, selected players and scripts: `/home/osso/.worktrees/healer-item-usage/` (`measure_usage.py`, `summary.json`, `hex-summary.json`).

## Matched Preservation gap

In `vhKYp3J2BcVdQ7fa` fight 2 pull 1, healer item 270162 is item level 341. Its cast at 45.678 s applies 1307578 barriers to the party at 47.223–49.333 s, about 432–434k per non-tank member. In 45–55 s, those barriers absorb 1,099,914 damage on actors 2/3/5, versus 99,514 Temporal Anomaly and 35,778 Reversion absorbs. Native `v18/no-reversion` has no item use or Soulcoil Barrier application in any seed; one seed's hunter dies at 53.55 s and healer at 54.45 s. This identifies missing item use, not a healing-coefficient error. An on-ready first use is not a reconstruction of that 45.678-second cast.

## DPS/tank source audit

All 31 tracked DPS/tank profile APLs already contain sourced item actions: 16 contain generic `use_items`, 15 use explicit `use_item` entries. Their conditions and item eligibility remain unchanged. The audit checks the actual `actions` arrays under `crates/server-follower-apl/profiles`, not healer source comments; retaining source lines alone is not a fresh runtime parity test.
