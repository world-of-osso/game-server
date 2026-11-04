# Preservation Fire Breath pull timing

**INFERRED_WCL:** Permit the healer's damage-list Fire Breath 26.862 seconds after pull engagement. The engaged-enemy count and party health do not gate it. Emergency mana recovery and Rewind take priority. This is a policy approximation, not a retail priority-list claim.

In five current-build reports, 31 completed Fire Breath casts hit a median of seven and nearest-rank p25 of five distinct enemy instances. Direct 357209 damage was matched to each 357208 empower release within one second; canceled casts and misses are excluded. First observed party engagement to first empower start was 7.226, 24.414, 26.862, 28.161 and 66.426 seconds, median 26.862. Reports and fight/pull: `vhKYp3J2BcVdQ7fa` 2/1, `caf6pzLP3nNr7CMv` 3/1, `1gKbWZAyzT7JqBn6` 4/1, `6wC7gFdD9avRAWVL` 17/1 and `b64JANPjvFap98Kf` 11/1. Earliest hostile damage serves as observed engagement. Target counts are damage receipts, not exact pre-cast cone occupancy. Evidence: `/home/osso/.worktrees/healer-merithra-wcl/fire-breath-counts.json`.

## Rejected party-health gate

An earlier gate required the lowest living party member to have at least 51.857% HP: the p10 of the lowest living HP at 32 retail empower starts. A value at actual casts is not a trigger. Over the same five pulls, lowest living HP at starts (median 74.6%) matched HP at 250 ms samples where a charge was ready but Fire Breath was not started (n=1,103, median 78.0%; AUC 0.51). Starts per ready-second were 0.152 / 0.146 / 0.127 in the 50–65 / 65–80 / 80–95% bands and 0.082 at 95% or more. Below 50% there were 2 starts in 38 ready-seconds, too few to support a gate. Readiness assumes two charges (Legacy of the Lifebinder), sequential 30 s recharge and full charges at pull start. Evidence: `/home/osso/.worktrees/healer-merithra-wcl/audit_ready_not_cast.py`.

## Rejected engaged-enemy clause

An earlier gate also permitted Fire Breath at pull start when at least five enemies were engaged (the p25 of distinct enemies hit per cast). Over the same five pulls and ready windows, the audit counted distinct enemy instances hit by party damage in the preceding 3 s:

- In the first 26.862 s, at least five enemies were present for 119 of 124 ready-seconds. Only 3 starts occurred in that time (0.025 per ready-second).
- After 26.862 s, there were 29 starts in 150 ready-seconds (0.193 per ready-second).

The enemy count did not predict starts, so the clause is removed. The time clause separates ready-but-not-cast moments by a factor of about eight. The 26.862 s boundary itself is the median first-cast delay; 32 starts do not resolve where the step falls. In the matched replay the enemy clause made the healer spend both charges 3–4 s after engagement. Retail `vhKYp3J2BcVdQ7fa` 2/1 started at 4.9 s and then at 27.0 s. Evidence: `/home/osso/.worktrees/healer-table-recheck-evidence/fire_breath_enemy_time_audit.py`.
