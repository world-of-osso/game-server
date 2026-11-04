# Preservation Living Flame triage — rejected

Native has no urgent Living Flame rule; Living Flame keeps its place after routine healing (`living_flame,if=target.health.pct<95`). Do not add a threshold until retail data shows one.

A rule cast Living Flame on the lowest ally at or below 61.83% HP before routine healing (`c959b46664`, reverted). That threshold was the median target HP at 23 retail friendly Living Flame completions in five current-build pulls (`/home/osso/.worktrees/healer-merithra-wcl/living-heal-casts-hp.json`). A median at actual casts is not a trigger: it says nothing about the moments when Living Flame was ready but retail cast something else. Natively the rule fired about 25 times per minute against retail's 23 casts in five pulls. In the matched pull **INFERRED_WCL** `vhKYp3J2BcVdQ7fa` f2 p1, it displaced every group heal from 12 to 30 s and left mana at 88–100%. The DPS and healer died at 30.4 s in every seed. With the rule reverted, the first DPS death moves to 34.4–54.5 s (`/home/osso/.worktrees/healer-wcl-current-v15/` versus `-v16/`).

Any future threshold must show that cast-time HP separates from HP at ready-but-not-cast moments, as for Vampiric Blood in [250](250.md).
