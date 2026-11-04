# Preservation Merithra triage

When the Merithra's Blessing override 1256579 is active, the healer may use the upgraded Reversion before Echo and Dream Breath if the lowest-health living party member is at or below 65.17229577976941% health. Emergency mana recovery and Rewind remain higher priority. This policy is **INFERRED_WCL**, not a retail priority-list claim.

The threshold is the median of the latest pre-cast target HP snapshots for 35 Merithra casts in current-build reports `vhKYp3J2BcVdQ7fa` fight 2 pull 1, `b64JANPjvFap98Kf` 11/1, `caf6pzLP3nNr7CMv` 3/1, `1gKbWZAyzT7JqBn6` 4/1 and `6wC7gFdD9avRAWVL` 17/1. Snapshots precede the cast by 1–612 ms (median 118 ms); target HP at the exact cast is not observed. These are cast-target values, not measurements of the lowest party member. The policy applies the median to the native lowest-health target as an inferred decision. Per-cast evidence: `/home/osso/.worktrees/healer-merithra-wcl/cast-target-hp.json`.

## Status: unsupported trigger kept for spend rate

The 65.17229577976941% cutoff is not a supported trigger. Over the five reports above, the lowest living party HP at 39 casts (median 67.3%) was compared with HP at 250 ms samples in which 1256579 was active and not spent (n=1,582, median 79.0%; AUC 0.674). Casts per ready-second rise steadily as health falls: 0.016 at 95% or more, 0.064 at 80–95%, 0.108 at 65–80%, 0.168 at 50–65% and 0.229 below 50%. 54% of casts occurred above the cutoff. This is a graded hazard, not a step.

The rule is kept only because it is the sole native path that reproduces retail's spend rate. Retail spent 38 of 39 procs before expiry, with 6 casts in the first 75 s of `vhKYp3J2BcVdQ7fa` 2/1. Without the rule, the override is reachable only through the routine Reversion slot. Five matched 75 s replays then cast it 0/0/1/1/0 times, against 7/7/10/11/4 with the rule. A timing rule sourced from proc-gain-to-spend delays will replace it. Evidence: `/home/osso/.worktrees/healer-merithra-wcl/audit_ready_not_cast.py`, `/home/osso/.worktrees/healer-wcl-current-v17-merithra/` and `/home/osso/.worktrees/healer-wcl-current-v17-fire-breath/`.
