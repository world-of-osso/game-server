# Preservation Merithra triage

When the Merithra's Blessing override 1256579 is active, the healer may use the upgraded Reversion before Echo and Dream Breath if the lowest-health living party member is at or below 65.17229577976941% health. Emergency mana recovery and Rewind remain higher priority. This policy is **INFERRED_WCL**, not a retail priority-list claim.

The threshold is the median of the latest pre-cast target HP snapshots for 35 Merithra casts in current-build reports `vhKYp3J2BcVdQ7fa` fight 2 pull 1, `b64JANPjvFap98Kf` 11/1, `caf6pzLP3nNr7CMv` 3/1, `1gKbWZAyzT7JqBn6` 4/1 and `6wC7gFdD9avRAWVL` 17/1. Snapshots precede the cast by 1–612 ms (median 118 ms); target HP at the exact cast is not observed. These are cast-target values, not measurements of the lowest party member. The policy applies the median to the native lowest-health target as an inferred decision. Per-cast evidence: `/home/osso/.worktrees/healer-merithra-wcl/cast-target-hp.json`.
