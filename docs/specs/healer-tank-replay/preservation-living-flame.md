# Preservation Living Flame triage — rejected

Native has neither an urgent nor an allied Living Flame filler rule. After higher-priority heals, the existing damage list remains Disintegrate then enemy Living Flame. The unsourced `target.health.pct<95` allied filler and `injured != 0` damage-return blocker are removed together; no new threshold is added.

A rule cast Living Flame on the lowest ally at or below 61.83% HP before routine healing (`c959b46664`, reverted). That threshold was the median target HP at 23 retail friendly Living Flame completions in five current-build pulls (`/home/osso/.worktrees/healer-merithra-wcl/living-heal-casts-hp.json`). A median at actual casts is not a trigger: it says nothing about the moments when Living Flame was ready but retail cast something else. Natively the rule fired about 25 times per minute against retail's 23 casts in five pulls. In the matched pull **INFERRED_WCL** `vhKYp3J2BcVdQ7fa` f2 p1, it displaced every group heal from 12 to 30 s and left mana at 88–100%. The DPS and healer died at 30.4 s in every seed. With the rule reverted, the first DPS death moves to 34.4–54.5 s (`/home/osso/.worktrees/healer-wcl-current-v15/` versus `-v16/`).

Any future threshold must show that cast-time HP separates from HP at ready-but-not-cast moments, as for Vampiric Blood in [250](250.md).

## Allied filler rejection

**INFERRED_WCL**, five pulls listed in the [priority audit](preservation-priority-audit.md): in [0,75s), 31 Living Flame casts include 10 allied and 21 enemy targets. Native qualified-items has 77 casts over five seeds, 76 allied and one enemy. Survivor control does not remove the excess: the shared first34.4s has retail2.4 casts per pull (4.19/min) versus native6.4 (11.16/min); native also casts15.29/min of healer-live time versus retail4.96/min. Removing Echo exposed more allied filler, and removing plain Reversion exposed still more. This does not establish another allied HP trigger; remove the unsupported policy instead.

The behavioral fixture holds routine healing cooldowns and Disintegrate with one injured tank and verifies an actual Living Flame targets the enemy. An injured ally must not cause idle time after higher-priority healing is unavailable.
