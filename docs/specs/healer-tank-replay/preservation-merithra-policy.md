# Preservation Merithra spend

**INFERRED_WCL:** While the Merithra's Blessing override 1256579 is active, the healer casts the upgraded Reversion (1256581) in the default list after Verdant Embrace and before Emerald Blossom. Party health is not a condition. Emergency mana recovery, Rewind, Temporal Anomaly, Echo, Dream Breath, Spiritbloom and Verdant Embrace take priority. This is a policy approximation, not a retail priority-list claim.

## Evidence

The source is a spend share. Ten current-build whole keys, each from a different Preservation player, give 1,089 proc episodes (applybuff to removebuff of 1256579 on the caster). 1,075 of these ended in a 1256581 cast, so retail spends 98.7% of procs. The 1256579 aura lasts 30 s and does not stack; a new proc refreshes it. The keys (report/fight) are: `qLJr1K6gD3xWNfcw` 1, `whDVj67G9gxC3WbF` 1, `6ctdTRZJxGF34Kk7` 24, `X8VcqT9g1ZjMvJy2` 13, `pYrTNkVFBGH7n8mK` 6, `Fk4Aph7aKHByqPC6` 18, `LFvC3kbYqfcZJ7tR` 18, `dA72xfKtyPmTJVLh` 12, `MFjm9rV3Dz2kvtX6` 1 and `7LGMHTcz38DYwXhj` 50. They are key levels 22–23, dated 2026-10-26 to 2026-11-03, across all five Season 2 dungeons. Events: `/home/osso/.worktrees/healer-merithra-wcl-more/`; analysis: `/home/osso/.worktrees/healer-table-recheck-evidence/merithra_spend_timing.py`.

Retail spends are neither prompt nor held until expiry:

- Gain-to-spend delay: p10 2.5 s, median 8.4 s, p90 19.0 s. 6.3% of spends occur within 2 s.
- Remaining duration at spend: p10 20.6 s, median 26.6 s.
- Spend hazard: 0.032/s in the first 2 s after gain, then about 0.09–0.18/s.

The slot is the lowest one that still spends retail's share. In five matched 75 s replays of `vhKYp3J2BcVdQ7fa` fight 2 pull 1, `reversion,if=merithra.active` was tested at several positions:

| Position | Spends / procs |
|---|---|
| Urgent list | 47/48 |
| After Echo | 41/43 |
| After Verdant Embrace | 39/40 |
| After Emerald Blossom | 12/17, with 41 refreshes and delays of 37–49 s |
| Removed HP rule | 39/40 |

Replays: `/home/osso/.worktrees/healer-merithra-pos-*/`.

**Open gap:** at every position that spends retail's share, the native delay is mostly 0.8–2 s, against a retail median of 8.4 s. No single priority slot reproduces the retail delay distribution.

## Rejected health threshold

An earlier rule cast the upgraded Reversion in the urgent list when the lowest-health target was at or below 65.17229577976941%. This value was the median target HP at 35 retail casts. A value at actual casts is not a trigger.

Over five current-build pulls, the audit compared the lowest living party HP at 39 casts with HP at 250 ms samples in which 1256579 was active and not spent:

- At casts: median 67.3%.
- While active and not spent: n=1,582, median 79.0%; AUC 0.674.

Casts per ready-second rise steadily as health falls: 0.016 at 95% or more, 0.064 at 80–95%, 0.108 at 65–80%, 0.168 at 50–65% and 0.229 below 50%. 54% of casts occurred above the cutoff. This is a graded hazard, not a step. Evidence: `/home/osso/.worktrees/healer-merithra-wcl/audit_ready_not_cast.py`.
