# Preservation Living Flame triage

**INFERRED_WCL:** After emergency mana recovery, Rewind and upgraded Reversion, the healer casts Living Flame on the lowest-health living party member at or below 61.8264916% HP, before routine healing and Fire Breath. This is a policy approximation, not a claim about retail's priority order.

In 23 friendly-target `361469` cast-completion events from five current-build pulls, the median latest pre-completion target HP was 61.8264916%. Snapshot age was 0–492 ms, and completion HP is not necessarily the HP when a cast began. Immediately before completion, 17/23 had Essence Burst 369299 and 10/23 had Leaping Flames 370901; neither is required by the policy. Reports/fight/pull: `vhKYp3J2BcVdQ7fa` 2/1, `caf6pzLP3nNr7CMv` 3/1, `1gKbWZAyzT7JqBn6` 4/1, `6wC7gFdD9avRAWVL` 17/1 and `b64JANPjvFap98Kf` 11/1. Per-cast evidence: `/home/osso/.worktrees/healer-merithra-wcl/living-heal-casts-hp.json`.

The 170,958 effective 361509 tank receipt at 6.559 s in `vhKYp3J2BcVdQ7fa` has no corresponding in-pull 361469 cast; its source remains unresolved and is not evidence for this direct-cast threshold.
