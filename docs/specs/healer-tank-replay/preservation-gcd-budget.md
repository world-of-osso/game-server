# Preservation whole-action budget

**INFERRED_WCL:** exact pull-relative windows [0,34s) and [0,75s), five retail pulls in the [priority audit](preservation-priority-audit.md), versus five native seeds at `b4f86d9988` (`v18/damage-lf`). Counts below are means. Dead native time is not silently treated as idle.

## Foreground inventory

Automatic VE/LF/Dream/Fire children and Twin Flame are not foreground casts. Stasis recalls can also use ordinary parent IDs: five instantaneous Temporal Anomalies, three Blossoms and two Merithras after370562 removal are excluded. This corrects the initial raw-cast table, which counted those10 recalls as foreground. The five Dream355941 recalls were already excluded.

| Spell | Classification | Retail34 | Native34 | Retail75 | Native75 |
|---|---|---:|---:|---:|---:|
| Emerald Blossom 355913 | GCD metadata | 7.0 | 7.8 | 14.6 | 14.4 |
| Dream Breath 355936 | GCD metadata | 2.0 | 3.0 | 4.4 | 4.4 |
| Disintegrate 356995 | GCD metadata | 1.0 | 1.2 | 2.8 | 4.2 |
| Time Dilation 357170 | off-GCD | 0.4 | 0.0 | 0.8 | 0.0 |
| Fire Breath 357208 | GCD metadata | 2.2 | 2.0 | 4.0 | 3.0 |
| Deep Breath 357210 | GCD metadata | 0.2 | 0.0 | 0.2 | 0.0 |
| Wing Buffet 357214 | GCD metadata | 0.2 | 0.0 | 0.2 | 0.0 |
| Hover 358267 | off-GCD | 0.8 | 0.0 | 2.0 | 0.0 |
| Glide 358733 | off-GCD | 0.8 | 0.0 | 1.6 | 0.0 |
| Naturalize 360823 | GCD metadata | 1.2 | 0.0 | 3.2 | 0.0 |
| Verdant Embrace 360995 | GCD metadata | 1.6 | 2.0 | 3.2 | 5.0 |
| Living Flame 361469 | GCD metadata | 2.2 | 6.0 | 6.2 | 16.0 |
| Azure Strike 362969 | GCD metadata | 0.0 | 0.0 | 0.4 | 0.0 |
| Obsidian Scales 363916 | off-GCD | 0.4 | 0.0 | 0.8 | 0.0 |
| Blessing of the Bronze 364342 | GCD metadata | 0.0 | 0.0 | 0.2 | 0.0 |
| Echo 364343 | GCD metadata | 0.0 | 0.0 | 0.2 | 0.0 |
| Reversion 366155 | GCD metadata | 0.2 | 0.0 | 0.2 | 0.0 |
| Tail Swipe 368970 | GCD metadata | 0.4 | 0.0 | 0.4 | 0.0 |
| Stasis 370537 | off-GCD | 0.8 | 0.0 | 0.8 | 0.0 |
| Tip the Scales 370553 | off-GCD | 1.0 | 0.0 | 1.0 | 0.0 |
| Stasis 370564 | GCD metadata | 0.2 | 0.0 | 0.6 | 0.0 |
| Rescue 370665 | GCD metadata | 0.4 | 0.0 | 0.8 | 0.0 |
| Temporal Anomaly 373861 | GCD metadata | 1.8 | 3.0 | 4.2 | 5.4 |
| Zephyr 374227 | GCD metadata | 0.2 | 0.0 | 0.4 | 0.0 |
| Time Spiral 374968 | GCD metadata | 0.2 | 0.0 | 0.2 | 0.0 |
| Fury of the Aspects 390386 | off-GCD | 0.6 | 0.0 | 0.6 | 0.0 |
| Oppressing Roar 406971 | GCD metadata | 0.8 | 0.0 | 0.8 | 0.0 |
| Light's Potential 1236616 | unresolved, no GCD | 0.2 | 0.0 | 0.2 | 0.0 |
| Merithra's Blessing 1256581 | GCD metadata | 1.6 | 3.6 | 3.6 | 6.2 |
| Soulcoiler Ritual Vessel 1291894 | channel, no GCD | 0.0 | 1.0 | 0.2 | 1.0 |

Foreground actions with GCD metadata total **23.6 vs28.6** at34s and **51.0 vs58.6** at75s. Off-GCD mobility/defensives are action counts, not occupied GCD slots. Soulcoiler has no GCD but occupies its two-second channel. Light's Potential is retained as an unresolved no-GCD record, not assigned foreground GCD cost. Casting-source GCD metadata alone does not prove an automatic child consumed a GCD.

## Busy and empty time

Native state is sampled every50ms from the existing cast state and native GCD clock. A temporary read-only probe produced exactly the baseline's healer cast sequences and death times in all five seeds; its source was restored before committing. Mutually exclusive native mean seconds:

| Window | Cast/channel active | GCD only | Alive and neither | Dead |
|---|---:|---:|---:|---:|
| 34s | 17.27 | 16.51 | 0.22 | 0.00 |
| 75s | 36.19 | 29.53 | 0.51 | 8.77 |

Retail has no logged per-frame GCD clock. Reconstruction uses observed hard-cast begin/end pairs, empower start/end pairs, actual Disintegrate debuff lifetimes, and Soulcoiler's two-second channel. General GCD is inferred from nearby noninstant Temporal Anomaly cast spans (DB2 cast and GCD both1.5s); empower finish GCD is one third of that. Ancient Flame375583's40% LF cast/GCD reduction is accounted for. Missing LF start events are tested both as GCD-only and with inferred cast occupancy. The ranges below also vary inferred GCD duration by±10%. These are sensitivity estimates, not observed idle clocks or confidence intervals.

| Retail sample | Empty/unoccupied seconds at34s | At75s |
|---|---:|---:|
| Five-pull mean, central with inferred LF cast time | 10.80 | 23.02 |
| Five-pull sensitivity range | 9.84–12.22 | 20.73–26.08 |
| Matched vhKY, central with inferred LF cast time | 5.61 | 13.06 |
| Matched vhKY sensitivity range | 4.80–7.22 | 10.58–16.15 |

Unoccupied time may be movement, decisions, latency or unobserved interrupted casting; do not call it all voluntary waiting. Hover and Glide do not themselves consume GCDs. Resource/cast timing uncertainty does not authorize a synthetic idle delay.

## What occupies retail's time instead of LF

Azure Strike is only0.4 casts per75s and cannot explain the gap. Native already casts more Disintegrate, Verdant Embrace and Merithra; Blossom is close (14.4 native versus14.8 true retail foreground). Missing utility consumes7.8 retail GCD-metadata actions per75s, chiefly Naturalize3.2 plus dispel-independent CC/defensives/Rescue/Stasis release. There is also substantially more unoccupied retail time: matched vhKY has about13s in75s versus0.15–0.90s native while alive. Those differences are sufficient to explain extra native filler without a new LF HP gate or arbitrary wait policy. Naturalize is owned by a separate cross-healer dispel feature; see [the exact removed debuffs](1468.md#retail-naturalize-removals-separate-dispel-feature).

Stasis is a material missing mechanic, separate from GCD spending: all five openings recall three stored heals near0/.3/.6s after the ready aura ends. Four activations are logged inside75s; vhKY starts with an active partial store. Three manual release casts are logged; 1gKb expires its ready aura at36.899s, while vhKY's27.639s removal has no370564 cast record. The actual recalls, not button counts alone, establish missing healing.

Raw casts, dispels, Ancient Flame/ready aura events, Disintegrate lifetimes and reproducible retail model: `/home/osso/.worktrees/healer-gcd-budget/`. Native probe data: `healer-wcl-current-v18/gcd-budget-probe/`. Remaining spell/mechanic changes must be separately sourced; this budget does not justify another filler policy change.
