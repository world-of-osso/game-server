# Preservation priority audit

## Opening-window cast comparison

**INFERRED_WCL:** report/fight/pull order below is `1gKbWZAyzT7JqBn6` 4/1, `6wC7gFdD9avRAWVL` 17/1, `b64JANPjvFap98Kf` 11/1, `caf6pzLP3nNr7CMv` 3/1, `vhKYp3J2BcVdQ7fa` 2/1. Count healer-source `cast` records in each exact pull-relative interval [0,75 s), not 75 s after the first cast. Echo aura applications from Temporal Anomaly are not Echo casts; Merithra's Blessing is counted separately from plain Reversion.

Native baseline is `745294a872`, five seeds of matched vhKY 2/1, in `/home/osso/.worktrees/healer-wcl-current-v18/set-eb/`. Native deaths shorten casting opportunity; these counts do not establish new health thresholds or an optimal ordering.

| Priority / spell | Retail counts, five pulls | Retail mean | Native counts, seeds 1–5 | Native mean |
|---|---|---:|---|---:|
| urgent Emerald Communion | 0/0/0/0/0 | 0.0 | 0/0/0/0/0 | 0.0 |
| urgent Rewind | 0/0/0/0/0 | 0.0 | 1/1/1/1/1 | 1.0 |
| hybrid Fire Breath | 5/5/2/3/5 | 4.0 | 3/2/2/2/2 | 2.2 |
| default 1 Temporal Anomaly | 6/5/3/6/6 | 5.2 | 6/3/2/3/3 | 3.4 |
| default 2 Echo | 0/1/0/0/0 | 0.2 | 28/16/8/12/9 | 14.6 |
| default 3 Dream Breath | 6/5/2/4/5 | 4.4 | 4/3/3/3/3 | 3.2 |
| default 4 Spiritbloom | 0/0/0/0/0 | 0.0 | 0/0/0/0/0 | 0.0 |
| default 5 Verdant Embrace | 4/3/3/2/4 | 3.2 | 6/3/2/2/2 | 3.0 |
| default 6 Merithra's Blessing | 3/4/3/4/6 | 4.0 | 4/5/3/4/2 | 3.6 |
| default 7 Emerald Blossom | 20/18/10/12/17 | 15.4 | 0/0/0/1/2 | 0.6 |
| default 8 plain Reversion | 0/0/0/1/0 | 0.2 | 8/4/4/2/3 | 4.2 |
| default 9 Living Flame | 4/7/4/8/8 | 6.2 | 9/3/3/1/3 | 3.8 |

## Sources and decisions

SimC `76f8118` `apl_evoker.cpp:160–162` has an empty Preservation default, so it does not source this healing order. The table supports removing unsupported cast-producing rules; it does not support introducing new conditions.

- Echo's foreground rule precedes Blossom and consumes Essence. It is grossly overcast relative to retail. Remove it; keep Temporal Anomaly's native Echo applications.
- Plain Reversion's maintenance rule is also grossly overcast and precedes Living Flame. Remove it separately; preserve the [Merithra spend rule](preservation-merithra-policy.md).
- Urgent Rewind is used once in every native seed but never in the measured retail openings. Remove that unsupported urgent rule separately.
- Leave Spiritbloom and Communion unchanged: neither is cast by this native baseline.
- Preserve Blossom and Living Flame's existing conditions; improve access only by removing unsupported priorities. [Fire Breath](preservation-fire-breath.md) and [Merithra](preservation-merithra-policy.md) have separate measured policies.

Raw retail casts: `/home/osso/.worktrees/healer-merithra-wcl/*-pull1-all-casts.jsonl`. Native casts: healer-source `CastSuccess` events in the baseline traces. Compare each subsequent change using the same intake and five seeds, and retain death outcomes rather than filtering them out.
